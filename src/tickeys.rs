//! Tickeys audio engine — rodio + crossbeam bounded channel.
//!
//! Architecture:
//!   - One bounded crossbeam channel (capacity 64), process lifetime.
//!   - One audio worker thread owning the rodio OutputStream.
//!   - CGEventTap callback → try_send(Play(idx)) — non-blocking, no alloc.
//!   - Scheme reload → try_send(ReloadScheme(data)) — reuses worker.

use std::collections::{VecDeque, BTreeMap};
use std::fs::File;
use std::io::BufReader;
use std::sync::OnceLock;

use crossbeam::channel::{bounded, Receiver, Sender};
use rodio::{Decoder, OutputStream, OutputStreamHandle, Source, buffer::SamplesBuffer};
use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════════════════════
// Audio scheme data model (serde)
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Deserialize, Serialize, Clone)]
pub struct AudioScheme {
    pub name: String,
    pub display_name: String,
    pub files: Vec<String>,
    pub non_unique_count: u8,
    pub key_audio_map: BTreeMap<u8, u8>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// Pre-decoded audio
// ═══════════════════════════════════════════════════════════════════════════════

pub struct AudioData {
    samples: Vec<f32>,
    sample_rate: u32,
    channels: u16,
}

impl AudioData {
    pub fn from_file(path: &str) -> Result<AudioData, String> {
        let file = BufReader::new(File::open(path).map_err(|e| e.to_string())?);
        let decoder = Decoder::new(file).map_err(|e| format!("decode {}: {}", path, e))?;
        let channels = decoder.channels();
        let sample_rate = decoder.sample_rate();
        let samples: Vec<f32> = decoder.convert_samples().collect();
        Ok(AudioData { samples, sample_rate, channels })
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Commands (CGEventTap → audio worker)
// ═══════════════════════════════════════════════════════════════════════════════

pub(crate) enum AudioCommand {
    Play(usize),
    ReloadScheme(Vec<AudioData>),
    SetVolume(f32),
    SetSpeed(f32),
}

// ═══════════════════════════════════════════════════════════════════════════════
// Global send channel (set once at startup)
// ═══════════════════════════════════════════════════════════════════════════════

static AUDIO_TX: OnceLock<Sender<AudioCommand>> = OnceLock::new();

/// Called from CGEventTap callback. Real-time safe: non-blocking try_send.
/// On channel full, silently drops the event.
pub fn send_play_command(index: usize) {
    if let Some(tx) = AUDIO_TX.get() {
        let _ = tx.try_send(AudioCommand::Play(index));
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Audio worker
// ═══════════════════════════════════════════════════════════════════════════════

/// Spawns one audio worker for the process lifetime.
/// Returns (tx, handle). tx is cloned into AUDIO_TX for global access.
pub fn spawn_audio_worker(
) -> Result<(Sender<AudioCommand>, std::thread::JoinHandle<()>), String> {
    let (tx, rx) = bounded::<AudioCommand>(64);

    AUDIO_TX.set(tx.clone())
        .map_err(|_| "AUDIO_TX already set".to_string())?;

    let handle = std::thread::spawn(move || {
        audio_worker_main(rx);
    });

    Ok((tx, handle))
}

fn audio_worker_main(rx: Receiver<AudioCommand>) {
    let (_stream, handle) = match OutputStream::try_default() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("audio worker: OutputStream failed: {}", e);
            return;
        }
    };

    let mut player = PlayerState {
        handle,
        data: vec![],
        volume: 1.0,
        speed: 1.0,
    };

    for cmd in rx {
        player.handle_cmd(cmd);
    }
}

struct PlayerState {
    handle: OutputStreamHandle,
    data: Vec<AudioData>,
    volume: f32,
    speed: f32,
}

impl PlayerState {
    fn handle_cmd(&mut self, cmd: AudioCommand) {
        match cmd {
            AudioCommand::Play(idx) => {
                if let Some(buf) = self.data.get(idx) {
                    let source = SamplesBuffer::new(
                        buf.channels, buf.sample_rate, buf.samples.clone(),
                    )
                    .amplify(self.volume)
                    .speed(self.speed);
                    let _ = self.handle.play_raw(source.convert_samples());
                }
            }
            AudioCommand::ReloadScheme(data) => {
                self.data = data;
            }
            AudioCommand::SetVolume(v) => {
                self.volume = v;
            }
            AudioCommand::SetSpeed(s) => {
                self.speed = s;
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Tickeys struct
// ═══════════════════════════════════════════════════════════════════════════════

pub struct Tickeys {
    volume: f32,
    pitch: f32,
    mute: bool,
    keymap: BTreeMap<u8, u8>,
    first_n_non_unique: i16,
    last_keys: VecDeque<u8>,
    schemes: Vec<AudioScheme>,
    on_keydown: Option<fn(&Tickeys, u8)>,
    audio_tx: Sender<AudioCommand>,
}

impl Tickeys {
    pub fn new(schemes: Vec<AudioScheme>, audio_tx: Sender<AudioCommand>) -> Tickeys {
        Tickeys {
            volume: 0.5,
            pitch: 1.0,
            mute: false,
            keymap: BTreeMap::new(),
            first_n_non_unique: -1,
            last_keys: VecDeque::with_capacity(8),
            schemes,
            on_keydown: None,
            audio_tx,
        }
    }

    pub fn get_schemes(&self) -> &Vec<AudioScheme> { &self.schemes }

    fn find_scheme(&self, name: &str) -> AudioScheme {
        self.schemes.iter().find(|s| s.name == name).cloned().unwrap()
    }

    pub fn load_scheme(&mut self, dir: &str, scheme_name: &str) {
        let scheme = self.find_scheme(scheme_name);
        let mut audio_data = Vec::with_capacity(scheme.files.len());

        for f in &scheme.files {
            let path = format!("{}/{}", dir, f);
            println!("loading audio: {}", path);
            let audio = AudioData::from_file(&path)
                .unwrap_or_else(|e| panic!("failed to load {}: {}", f, e));
            audio_data.push(audio);
        }

        let _ = self.audio_tx.send(AudioCommand::ReloadScheme(audio_data));
        self.keymap = scheme.key_audio_map.clone();
        self.first_n_non_unique = scheme.non_unique_count as i16;
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume;
        let _ = self.audio_tx.try_send(AudioCommand::SetVolume(volume));
    }

    pub fn set_pitch(&mut self, pitch: f32) {
        self.pitch = pitch;
        let _ = self.audio_tx.try_send(AudioCommand::SetSpeed(pitch));
    }

    pub fn set_mute(&mut self, mute: bool) { self.mute = mute; }
    pub fn get_volume(&self) -> f32 { self.volume }
    pub fn get_pitch(&self) -> f32 { self.pitch }
    pub fn get_last_keys(&self) -> &VecDeque<u8> { &self.last_keys }

    pub fn set_on_keydown(&mut self, cb: Option<fn(&Tickeys, u8)>) {
        self.on_keydown = cb;
    }

    /// Called from CGEventTap callback (via send_play_command).
    pub fn handle_keydown(&mut self, keycode: u8) {
        self.last_keys.push_back(keycode);
        if self.last_keys.len() > 6 {
            self.last_keys.pop_front();
        }

        if let Some(cb) = self.on_keydown {
            cb(self, keycode);
        }

        if self.mute { return; }

        let index: i32 = match self.keymap.get(&keycode) {
            Some(idx) => *idx as i32,
            None => {
                if self.first_n_non_unique <= 0 { -1 }
                else { (keycode % (self.first_n_non_unique as u8)) as i32 }
            }
        };

        if index != -1 {
            send_play_command(index as usize);
        }
    }
}
