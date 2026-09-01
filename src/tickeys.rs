//! Tickeys audio engine — rodio + crossbeam bounded channel.
//!
//! Architecture:
//!   - One bounded crossbeam channel (capacity 64), process lifetime.
//!   - One audio worker thread owning the rodio OutputStream.
//!   - CGEventTap callback → try_send(Play(idx)) — non-blocking, no alloc.
//!   - Scheme reload → try_send(ReloadScheme(data)) — reuses worker.

use std::collections::{BTreeMap, VecDeque};
use std::fs::File;
use std::io::BufReader;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use crossbeam::channel::{Receiver, RecvTimeoutError, Sender, bounded};
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
        Ok(AudioData {
            samples,
            sample_rate,
            channels,
        })
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Commands (CGEventTap → audio worker)
// ═══════════════════════════════════════════════════════════════════════════════

pub enum AudioCommand {
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
pub fn spawn_audio_worker() -> Result<(Sender<AudioCommand>, std::thread::JoinHandle<()>), String> {
    let (tx, rx) = bounded::<AudioCommand>(64);

    AUDIO_TX
        .set(tx.clone())
        .map_err(|_| "AUDIO_TX already set".to_string())?;

    let handle = std::thread::spawn(move || {
        audio_worker_main(rx);
    });

    Ok((tx, handle))
}

const AUDIO_RETRY_INITIAL_DELAY: Duration = Duration::from_secs(1);
const AUDIO_RETRY_MAX_DELAY: Duration = Duration::from_secs(30);

trait AudioOutput {
    fn play(&self, audio: &AudioData, volume: f32, speed: f32) -> Result<(), String>;
}

struct RodioOutput {
    _stream: OutputStream,
    handle: OutputStreamHandle,
}

impl RodioOutput {
    fn try_default() -> Result<Self, String> {
        let (stream, handle) = OutputStream::try_default()
            .map_err(|error| format!("OutputStream::try_default failed: {error}"))?;
        Ok(Self {
            _stream: stream,
            handle,
        })
    }
}

impl AudioOutput for RodioOutput {
    fn play(&self, audio: &AudioData, volume: f32, speed: f32) -> Result<(), String> {
        let source = SamplesBuffer::new(audio.channels, audio.sample_rate, audio.samples.clone())
            .amplify(volume)
            .speed(speed);
        self.handle
            .play_raw(source.convert_samples())
            .map_err(|error| format!("play_raw failed: {error}"))
    }
}

struct RetrySchedule {
    initial_delay: Duration,
    max_delay: Duration,
    current_delay: Duration,
    next_attempt: Instant,
}

impl RetrySchedule {
    fn new(now: Instant, initial_delay: Duration, max_delay: Duration) -> Self {
        Self {
            initial_delay,
            max_delay,
            current_delay: initial_delay,
            next_attempt: now,
        }
    }

    fn is_due(&self, now: Instant) -> bool {
        now >= self.next_attempt
    }

    fn wait_duration(&self, now: Instant) -> Duration {
        self.next_attempt.saturating_duration_since(now)
    }

    fn record_failure(&mut self, now: Instant) {
        self.next_attempt = now + self.current_delay;
        self.current_delay = self.current_delay.saturating_mul(2).min(self.max_delay);
    }

    fn reset(&mut self, now: Instant) {
        self.current_delay = self.initial_delay;
        self.next_attempt = now;
    }
}

fn audio_worker_main(rx: Receiver<AudioCommand>) {
    audio_worker_with_factory(rx, AUDIO_RETRY_INITIAL_DELAY, AUDIO_RETRY_MAX_DELAY, || {
        RodioOutput::try_default().map(|output| Box::new(output) as Box<dyn AudioOutput>)
    });
}

fn audio_worker_with_factory<F>(
    rx: Receiver<AudioCommand>,
    retry_initial_delay: Duration,
    retry_max_delay: Duration,
    mut create_output: F,
) where
    F: FnMut() -> Result<Box<dyn AudioOutput>, String>,
{
    let mut player = PlayerState::new();
    let mut retry = RetrySchedule::new(Instant::now(), retry_initial_delay, retry_max_delay);
    let mut consecutive_failures = 0u32;

    loop {
        let now = Instant::now();
        if player.output.is_none() && retry.is_due(now) {
            match create_output() {
                Ok(output) => {
                    if consecutive_failures > 0 {
                        eprintln!(
                            "audio worker: output recovered after {consecutive_failures} failure(s)"
                        );
                    }
                    player.output = Some(output);
                    consecutive_failures = 0;
                    retry.reset(now);
                }
                Err(error) => {
                    consecutive_failures = consecutive_failures.saturating_add(1);
                    eprintln!(
                        "audio worker: {error}; retrying in {:?}",
                        retry.current_delay
                    );
                    retry.record_failure(now);
                }
            }
        }

        let command = if player.output.is_some() {
            match rx.recv() {
                Ok(command) => command,
                Err(_) => break,
            }
        } else {
            match rx.recv_timeout(retry.wait_duration(Instant::now())) {
                Ok(command) => command,
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => break,
            }
        };

        if let Some(error) = player.handle_cmd(command) {
            eprintln!("audio worker: {error}; rebuilding output");
            retry.reset(Instant::now());
            retry.record_failure(Instant::now());
            consecutive_failures = 1;
        }
    }
}

struct PlayerState {
    output: Option<Box<dyn AudioOutput>>,
    data: Vec<AudioData>,
    volume: f32,
    speed: f32,
}

impl PlayerState {
    fn new() -> Self {
        Self {
            output: None,
            data: vec![],
            volume: 1.0,
            speed: 1.0,
        }
    }

    /// Returns an error when the current output should be rebuilt.
    fn handle_cmd(&mut self, cmd: AudioCommand) -> Option<String> {
        match cmd {
            AudioCommand::Play(idx) => {
                if let Some(buf) = self.data.get(idx) {
                    let play_result = self
                        .output
                        .as_ref()
                        .map(|output| output.play(buf, self.volume, self.speed));
                    if let Some(Err(error)) = play_result {
                        self.output = None;
                        return Some(error);
                    }
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
        None
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

    fn find_scheme(&self, name: &str) -> AudioScheme {
        self.schemes
            .iter()
            .find(|s| s.name == name)
            .cloned()
            .unwrap()
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
        let volume = volume.clamp(0.0, 1.0);
        self.volume = volume;
        let _ = self.audio_tx.try_send(AudioCommand::SetVolume(volume));
    }

    pub fn set_pitch(&mut self, pitch: f32) {
        let pitch = pitch.clamp(0.25, 2.0);
        self.pitch = pitch;
        let _ = self.audio_tx.try_send(AudioCommand::SetSpeed(pitch));
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

        if self.mute {
            return;
        }

        let index: i32 = match self.keymap.get(&keycode) {
            Some(idx) => *idx as i32,
            None => {
                if self.first_n_non_unique <= 0 {
                    -1
                } else {
                    (keycode % (self.first_n_non_unique as u8)) as i32
                }
            }
        };

        if index != -1 {
            send_play_command(index as usize);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::path::Path;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    use crossbeam::channel::bounded;

    use super::{
        AudioCommand, AudioData, AudioOutput, AudioScheme, RetrySchedule, audio_worker_with_factory,
    };

    struct RecordingOutput {
        plays: Arc<Mutex<Vec<(usize, f32, f32)>>>,
    }

    impl AudioOutput for RecordingOutput {
        fn play(&self, audio: &AudioData, volume: f32, speed: f32) -> Result<(), String> {
            self.plays.lock().expect("recording output lock").push((
                audio.samples.len(),
                volume,
                speed,
            ));
            Ok(())
        }
    }

    struct FailingOutput;

    impl AudioOutput for FailingOutput {
        fn play(&self, _audio: &AudioData, _volume: f32, _speed: f32) -> Result<(), String> {
            Err("simulated device loss".to_string())
        }
    }

    fn test_audio_data() -> AudioData {
        AudioData {
            samples: vec![0.0, 0.0],
            sample_rate: 44_100,
            channels: 2,
        }
    }

    #[test]
    fn bundled_scheme_manifest_references_valid_audio_files() {
        let schemes: Vec<AudioScheme> =
            serde_json::from_str(include_str!("../assets/data/schemes.json"))
                .expect("bundled schemes.json must parse");
        assert!(!schemes.is_empty());

        let mut names = HashSet::new();
        let data_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/data");
        for scheme in schemes {
            assert!(names.insert(scheme.name.clone()), "duplicate scheme name");
            assert!(
                !scheme.files.is_empty(),
                "{} has no audio files",
                scheme.name
            );
            assert!(
                usize::from(scheme.non_unique_count) <= scheme.files.len(),
                "{} has an invalid non_unique_count",
                scheme.name
            );

            for file in &scheme.files {
                assert!(
                    data_root.join(&scheme.name).join(file).is_file(),
                    "missing audio file {}/{}",
                    scheme.name,
                    file
                );
            }
            for index in scheme.key_audio_map.values() {
                assert!(
                    usize::from(*index) < scheme.files.len(),
                    "{} maps a key to missing audio index {}",
                    scheme.name,
                    index
                );
            }
        }
    }

    #[test]
    fn audio_worker_retries_initial_output_and_keeps_pending_state() {
        let attempts = Arc::new(AtomicUsize::new(0));
        let plays = Arc::new(Mutex::new(vec![]));
        let (tx, rx) = bounded(8);
        tx.send(AudioCommand::ReloadScheme(vec![test_audio_data()]))
            .unwrap();
        tx.send(AudioCommand::SetVolume(0.4)).unwrap();
        tx.send(AudioCommand::SetSpeed(1.5)).unwrap();
        tx.send(AudioCommand::Play(0)).unwrap();
        drop(tx);

        audio_worker_with_factory(rx, Duration::ZERO, Duration::ZERO, {
            let attempts = Arc::clone(&attempts);
            let plays = Arc::clone(&plays);
            move || {
                if attempts.fetch_add(1, Ordering::SeqCst) == 0 {
                    Err("simulated output unavailable".to_string())
                } else {
                    Ok(Box::new(RecordingOutput {
                        plays: Arc::clone(&plays),
                    }) as Box<dyn AudioOutput>)
                }
            }
        });

        assert_eq!(attempts.load(Ordering::SeqCst), 2);
        assert_eq!(*plays.lock().expect("recorded plays"), vec![(2, 0.4, 1.5)]);
    }

    #[test]
    fn play_failure_rebuilds_output_without_losing_state() {
        let attempts = Arc::new(AtomicUsize::new(0));
        let plays = Arc::new(Mutex::new(vec![]));
        let (tx, rx) = bounded(8);
        tx.send(AudioCommand::ReloadScheme(vec![test_audio_data()]))
            .unwrap();
        tx.send(AudioCommand::SetVolume(0.25)).unwrap();
        tx.send(AudioCommand::SetSpeed(1.75)).unwrap();
        tx.send(AudioCommand::Play(0)).unwrap();
        tx.send(AudioCommand::Play(0)).unwrap();
        drop(tx);

        audio_worker_with_factory(rx, Duration::ZERO, Duration::ZERO, {
            let attempts = Arc::clone(&attempts);
            let plays = Arc::clone(&plays);
            move || {
                if attempts.fetch_add(1, Ordering::SeqCst) == 0 {
                    Ok(Box::new(FailingOutput) as Box<dyn AudioOutput>)
                } else {
                    Ok(Box::new(RecordingOutput {
                        plays: Arc::clone(&plays),
                    }) as Box<dyn AudioOutput>)
                }
            }
        });

        assert_eq!(attempts.load(Ordering::SeqCst), 2);
        assert_eq!(
            *plays.lock().expect("recorded plays"),
            vec![(2, 0.25, 1.75)]
        );
    }

    #[test]
    fn output_retry_delay_grows_and_is_capped() {
        let start = Instant::now();
        let mut retry = RetrySchedule::new(start, Duration::from_secs(1), Duration::from_secs(4));

        retry.record_failure(start);
        assert_eq!(retry.wait_duration(start), Duration::from_secs(1));

        let second = start + Duration::from_secs(1);
        retry.record_failure(second);
        assert_eq!(retry.wait_duration(second), Duration::from_secs(2));

        let third = second + Duration::from_secs(2);
        retry.record_failure(third);
        assert_eq!(retry.wait_duration(third), Duration::from_secs(4));

        let fourth = third + Duration::from_secs(4);
        retry.record_failure(fourth);
        assert_eq!(retry.wait_duration(fourth), Duration::from_secs(4));

        retry.reset(fourth);
        assert!(retry.is_due(fourth));
    }
}
