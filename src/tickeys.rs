//! Audio worker with a bounded key queue and a separate latest-settings mailbox.
//! Scheme decoding and key mapping live on the worker. The event-tap callback
//! only queues a keycode; playing a sound shares its PCM allocation via Arc.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufReader, Seek};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender, bounded, select_biased};
use rodio::{Decoder, OutputStream, OutputStreamHandle, Source};
use serde::{Deserialize, Serialize};

pub const MIN_PITCH: f32 = 0.25;
pub const MAX_PITCH: f32 = 2.0;
const PLAY_QUEUE_CAPACITY: usize = 64;
const MAX_PLAY_AGE: Duration = Duration::from_millis(150);
const AUDIO_RETRY_INITIAL_DELAY: Duration = Duration::from_secs(1);
const AUDIO_RETRY_MAX_DELAY: Duration = Duration::from_secs(30);

pub fn normalize_pitch(pitch: f32) -> f32 {
    if pitch.is_finite() {
        pitch.clamp(MIN_PITCH, MAX_PITCH)
    } else {
        1.0
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AudioScheme {
    pub name: String,
    pub display_name: String,
    pub files: Vec<String>,
    pub non_unique_count: u8,
    pub key_audio_map: BTreeMap<u8, u8>,
}

impl AudioScheme {
    fn validate(&self) -> Result<(), String> {
        if self.files.is_empty()
            || usize::from(self.non_unique_count) > self.files.len()
            || self
                .key_audio_map
                .values()
                .any(|i| usize::from(*i) >= self.files.len())
        {
            return Err(format!("invalid audio mapping for {}", self.name));
        }
        Ok(())
    }

    fn sound_index(&self, keycode: u8) -> Option<usize> {
        self.key_audio_map
            .get(&keycode)
            .map(|i| usize::from(*i))
            .or_else(|| {
                (self.non_unique_count > 0).then(|| usize::from(keycode % self.non_unique_count))
            })
    }
}

pub struct AudioData {
    samples: Arc<[f32]>,
    sample_rate: u32,
    channels: u16,
}

impl AudioData {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref();
        let file =
            BufReader::new(File::open(path).map_err(|e| format!("{}: {e}", path.display()))?);
        // rodio's WAV adapter substitutes sample-read failures with silence and
        // calculates duration before our PCM checks. Validate the header and
        // every sample first, so corrupt resources cannot panic the worker or
        // report Ready with silent padding. Keep rodio's existing conversion
        // for valid files, including the bundled PCM16 recordings.
        let mut wav = hound::WavReader::new(file)
            .map_err(|e| format!("decode {}: {e}", path.display()))?;
        let spec = wav.spec();
        if spec.channels == 0 || spec.sample_rate == 0 || wav.len() == 0 {
            return Err(format!("invalid WAV metadata: {}", path.display()));
        }
        match spec.sample_format {
            hound::SampleFormat::Int => {
                for sample in wav.samples::<i32>() {
                    sample.map_err(|e| format!("decode {}: {e}", path.display()))?;
                }
            }
            hound::SampleFormat::Float => {
                for sample in wav.samples::<f32>() {
                    let sample = sample.map_err(|e| format!("decode {}: {e}", path.display()))?;
                    if !sample.is_finite() {
                        return Err(format!("invalid PCM data: {}", path.display()));
                    }
                }
            }
        }
        let mut file = wav.into_inner();
        file.rewind().map_err(|e| format!("{}: {e}", path.display()))?;
        let decoder = Decoder::new(file).map_err(|e| format!("decode {}: {e}", path.display()))?;
        let channels = decoder.channels();
        let sample_rate = decoder.sample_rate();
        let samples: Vec<f32> = decoder.convert_samples().collect();
        if channels == 0
            || sample_rate == 0
            || samples.is_empty()
            || samples.len() % usize::from(channels) != 0
            || samples.iter().any(|sample| !sample.is_finite())
        {
            return Err(format!("invalid PCM data: {}", path.display()));
        }
        Ok(Self {
            samples: samples.into(),
            sample_rate,
            channels,
        })
    }

    fn source(&self) -> SharedSamples {
        SharedSamples {
            samples: Arc::clone(&self.samples),
            sample_rate: self.sample_rate,
            channels: self.channels,
            position: 0,
        }
    }
}

#[derive(Clone)]
struct SharedSamples {
    samples: Arc<[f32]>,
    sample_rate: u32,
    channels: u16,
    position: usize,
}

impl Iterator for SharedSamples {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let sample = self.samples.get(self.position).copied()?;
        self.position += 1;
        Some(sample)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.samples.len() - self.position;
        (remaining, Some(remaining))
    }
}

impl Source for SharedSamples {
    fn current_frame_len(&self) -> Option<usize> {
        Some(self.samples.len() - self.position)
    }
    fn channels(&self) -> u16 {
        self.channels
    }
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    fn total_duration(&self) -> Option<Duration> {
        Some(Duration::from_secs_f64(
            self.samples.len() as f64 / f64::from(self.channels) / f64::from(self.sample_rate),
        ))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OutputStatus {
    Starting,
    Ready,
    Retrying(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SchemeStatus {
    Empty,
    Loading,
    Ready,
    Failed(String),
}

#[derive(Clone, Debug)]
pub struct AudioStatus {
    pub output: OutputStatus,
    pub scheme: SchemeStatus,
    /// Successfully submitted to rodio, not proof of audible hardware output.
    pub submitted_plays: u64,
}

struct SchemeRequest {
    generation: u64,
    directory: String,
    scheme: AudioScheme,
}

struct DesiredState {
    volume: f32,
    pitch: f32,
    generation: u64,
    scheme: Option<SchemeRequest>,
}

struct PlayRequest {
    generation: u64,
    keycode: u8,
    queued_at: Instant,
}

/// Only wake-ups coalesce; settings never compete for space with key events.
pub struct AudioController {
    play_tx: Sender<PlayRequest>,
    wake_tx: Sender<()>,
    desired: Arc<Mutex<DesiredState>>,
    status: Arc<Mutex<AudioStatus>>,
}

struct WorkerChannels {
    play_rx: Receiver<PlayRequest>,
    wake_rx: Receiver<()>,
    desired: Arc<Mutex<DesiredState>>,
    status: Arc<Mutex<AudioStatus>>,
}

fn audio_channels() -> (AudioController, WorkerChannels) {
    let (play_tx, play_rx) = bounded(PLAY_QUEUE_CAPACITY);
    let (wake_tx, wake_rx) = bounded(1);
    let desired = Arc::new(Mutex::new(DesiredState {
        volume: 0.5,
        pitch: 1.0,
        generation: 0,
        scheme: None,
    }));
    let status = Arc::new(Mutex::new(AudioStatus {
        output: OutputStatus::Starting,
        scheme: SchemeStatus::Empty,
        submitted_plays: 0,
    }));
    (
        AudioController {
            play_tx,
            wake_tx,
            desired: Arc::clone(&desired),
            status: Arc::clone(&status),
        },
        WorkerChannels {
            play_rx,
            wake_rx,
            desired,
            status,
        },
    )
}

pub fn spawn_audio_worker() -> Result<(AudioController, std::thread::JoinHandle<()>), String> {
    let (controller, channels) = audio_channels();
    let handle = std::thread::Builder::new()
        .name("tickeys-audio".into())
        .spawn(move || {
            audio_worker_with_factory(
                channels,
                AUDIO_RETRY_INITIAL_DELAY,
                AUDIO_RETRY_MAX_DELAY,
                || {
                    RodioOutput::try_default()
                        .map(|output| Box::new(output) as Box<dyn AudioOutput>)
                },
            );
        })
        .map_err(|e| e.to_string())?;
    Ok((controller, handle))
}

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
        self.handle
            .play_raw(audio.source().amplify(volume).speed(speed))
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

struct LoadedScheme {
    generation: u64,
    scheme: AudioScheme,
    data: Vec<AudioData>,
}

fn decode_scheme(request: SchemeRequest) -> Result<LoadedScheme, String> {
    request.scheme.validate()?;
    let data = request
        .scheme
        .files
        .iter()
        .map(|file| AudioData::from_file(Path::new(&request.directory).join(file)))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(LoadedScheme {
        generation: request.generation,
        scheme: request.scheme,
        data,
    })
}

struct PlayerState {
    output: Option<Box<dyn AudioOutput>>,
    scheme: Option<LoadedScheme>,
    volume: f32,
    pitch: f32,
}

impl PlayerState {
    fn new() -> Self {
        Self {
            output: None,
            scheme: None,
            volume: 0.5,
            pitch: 1.0,
        }
    }

    fn apply_settings(&mut self, channels: &WorkerChannels) {
        self.apply_settings_with_decoder(channels, decode_scheme);
    }

    fn apply_settings_with_decoder(
        &mut self,
        channels: &WorkerChannels,
        decode: impl FnOnce(SchemeRequest) -> Result<LoadedScheme, String>,
    ) {
        let request = {
            let mut desired = channels.desired.lock().expect("audio settings lock");
            self.volume = desired.volume;
            self.pitch = desired.pitch;
            desired.scheme.take()
        };
        if let Some(request) = request {
            let generation = request.generation;
            // No settings or status lock is held while reading/decoding files.
            let decoded = decode(request);
            let desired = channels.desired.lock().expect("audio settings lock");
            self.volume = desired.volume;
            self.pitch = desired.pitch;
            if generation != desired.generation {
                return;
            }
            let mut status = channels.status.lock().expect("audio status lock");
            match decoded {
                Ok(scheme) => {
                    self.scheme = Some(scheme);
                    status.scheme = SchemeStatus::Ready;
                }
                Err(error) => {
                    eprintln!("audio worker: {error}");
                    self.scheme = None;
                    status.scheme = SchemeStatus::Failed(error);
                }
            }
        }
    }

    fn play(&mut self, request: PlayRequest) -> Result<bool, String> {
        let Some(scheme) = self.scheme.as_ref() else {
            return Ok(false);
        };
        if request.generation != scheme.generation || request.queued_at.elapsed() > MAX_PLAY_AGE {
            return Ok(false);
        }
        let Some(audio) = scheme
            .scheme
            .sound_index(request.keycode)
            .and_then(|i| scheme.data.get(i))
        else {
            return Ok(false);
        };
        let Some(output) = self.output.as_ref() else {
            return Ok(false);
        };
        if let Err(error) = output.play(audio, self.volume, self.pitch) {
            self.output = None;
            return Err(error);
        }
        Ok(true)
    }
}

fn audio_worker_with_factory<F>(
    channels: WorkerChannels,
    retry_initial_delay: Duration,
    retry_max_delay: Duration,
    mut create_output: F,
) where
    F: FnMut() -> Result<Box<dyn AudioOutput>, String>,
{
    let mut player = PlayerState::new();
    let mut retry = RetrySchedule::new(Instant::now(), retry_initial_delay, retry_max_delay);
    loop {
        player.apply_settings(&channels);
        if player.output.is_none() && retry.is_due(Instant::now()) {
            match create_output() {
                Ok(output) => {
                    player.output = Some(output);
                    channels.status.lock().expect("audio status lock").output = OutputStatus::Ready;
                    retry.reset(Instant::now());
                }
                Err(error) => {
                    eprintln!(
                        "audio worker: {error}; retrying in {:?}",
                        retry.current_delay
                    );
                    channels.status.lock().expect("audio status lock").output =
                        OutputStatus::Retrying(error);
                    retry.record_failure(Instant::now());
                }
            }
        }
        let timeout = if player.output.is_some() {
            crossbeam_channel::never()
        } else {
            crossbeam_channel::after(retry.wait_duration(Instant::now()))
        };
        select_biased! {
            recv(channels.wake_rx) -> wake => { if wake.is_err() { break; } }
            recv(channels.play_rx) -> request => {
                let Ok(request) = request else { break; };
                match player.play(request) {
                    Ok(true) => channels.status.lock().expect("audio status lock").submitted_plays += 1,
                    Ok(false) => {},
                    Err(error) => {
                        eprintln!("audio worker: {error}; rebuilding output");
                        channels.status.lock().expect("audio status lock").output = OutputStatus::Retrying(error);
                        retry.reset(Instant::now());
                        retry.record_failure(Instant::now());
                    }
                }
            }
            recv(timeout) -> _ => {}
        }
    }
}

pub struct Tickeys {
    schemes: Vec<AudioScheme>,
    audio: AudioController,
    generation: u64,
}

impl Tickeys {
    pub fn new(schemes: Vec<AudioScheme>, audio: AudioController) -> Self {
        Self {
            schemes,
            audio,
            generation: 0,
        }
    }

    pub fn load_scheme(&mut self, directory: &str, name: &str) -> Result<(), String> {
        let scheme = self
            .schemes
            .iter()
            .find(|s| s.name == name)
            .ok_or_else(|| format!("unknown audio scheme: {name}"))?
            .clone();
        scheme.validate()?;
        self.generation += 1;
        {
            let mut desired = self.audio.desired.lock().expect("audio settings lock");
            desired.generation = self.generation;
            desired.scheme = Some(SchemeRequest {
                generation: self.generation,
                directory: directory.into(),
                scheme,
            });
            self.audio.status.lock().expect("audio status lock").scheme = SchemeStatus::Loading;
        }
        let _ = self.audio.wake_tx.try_send(());
        Ok(())
    }

    pub fn set_volume(&mut self, volume: f32) {
        let volume = if volume.is_finite() {
            volume.clamp(0.0, 1.0)
        } else {
            0.5
        };
        self.audio
            .desired
            .lock()
            .expect("audio settings lock")
            .volume = volume;
        let _ = self.audio.wake_tx.try_send(());
    }

    pub fn set_pitch(&mut self, pitch: f32) {
        self.audio
            .desired
            .lock()
            .expect("audio settings lock")
            .pitch = normalize_pitch(pitch);
        let _ = self.audio.wake_tx.try_send(());
    }

    /// Non-blocking and allocation-free; a full key queue drops only a key.
    pub fn handle_keydown(&self, keycode: u8) {
        let _ = self.audio.play_tx.try_send(PlayRequest {
            generation: self.generation,
            keycode,
            queued_at: Instant::now(),
        });
    }

    /// Uses the same audio path without requiring an event tap or keyboard access.
    pub fn test_sound(&self) {
        self.handle_keydown(0);
    }

    pub fn audio_status(&self) -> AudioStatus {
        self.audio.status.lock().expect("audio status lock").clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::sync::atomic::{AtomicUsize, Ordering};

    type RecordedPlay = (usize, f32, f32);
    struct RecordingOutput(Sender<RecordedPlay>);
    impl AudioOutput for RecordingOutput {
        fn play(&self, audio: &AudioData, volume: f32, pitch: f32) -> Result<(), String> {
            self.0.send((audio.samples.len(), volume, pitch)).unwrap();
            Ok(())
        }
    }

    struct FailingOutput;
    impl AudioOutput for FailingOutput {
        fn play(&self, _: &AudioData, _: f32, _: f32) -> Result<(), String> {
            Err("simulated device loss".into())
        }
    }

    fn schemes() -> Vec<AudioScheme> {
        serde_json::from_str(include_str!("../assets/data/schemes.json")).unwrap()
    }
    fn root(name: &str) -> String {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets/data")
            .join(name)
            .to_string_lossy()
            .into()
    }
    fn player() -> (Tickeys, WorkerChannels, PlayerState, Receiver<RecordedPlay>) {
        let (audio, channels) = audio_channels();
        let (tx, rx) = crossbeam_channel::unbounded();
        let mut tickeys = Tickeys::new(schemes(), audio);
        tickeys.load_scheme(&root("bubble"), "bubble").unwrap();
        let mut player = PlayerState::new();
        player.output = Some(Box::new(RecordingOutput(tx)));
        player.apply_settings(&channels);
        (tickeys, channels, player, rx)
    }

    fn with_wav_fixture(
        format: u16,
        rate: u32,
        bits: u16,
        declared: u32,
        data: &[u8],
        test: impl FnOnce(&Path),
    ) {
        use std::io::Write;
        static NEXT_FIXTURE: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "tickeys-wav-{}-{}.wav",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        struct Cleanup(std::path::PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_file(&self.0);
            }
        }
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        let _cleanup = Cleanup(path.clone());
        file.write_all(b"RIFF").unwrap();
        file.write_all(&(36 + declared).to_le_bytes()).unwrap();
        file.write_all(b"WAVEfmt ").unwrap();
        file.write_all(&16_u32.to_le_bytes()).unwrap();
        file.write_all(&format.to_le_bytes()).unwrap();
        file.write_all(&1_u16.to_le_bytes()).unwrap();
        file.write_all(&rate.to_le_bytes()).unwrap();
        file.write_all(&(rate * u32::from(bits / 8)).to_le_bytes())
            .unwrap();
        file.write_all(&(bits / 8).to_le_bytes()).unwrap();
        file.write_all(&bits.to_le_bytes()).unwrap();
        file.write_all(b"data").unwrap();
        file.write_all(&declared.to_le_bytes()).unwrap();
        file.write_all(data).unwrap();
        drop(file);
        test(&path);
    }

    #[test]
    fn truncated_wav_samples_are_errors_not_silent_padding() {
        with_wav_fixture(1, 44_100, 16, 4, &[0, 64], |path| {
            assert!(
                AudioData::from_file(path).is_err(),
                "truncated PCM must be rejected"
            );
        });
    }

    #[test]
    fn zero_sample_rate_is_rejected_without_panicking() {
        with_wav_fixture(1, 0, 16, 2, &[0, 64], |path| {
            let result = std::panic::catch_unwind(|| AudioData::from_file(path));
            assert!(
                result.is_ok(),
                "invalid WAV metadata must not panic the audio worker"
            );
            assert!(result.unwrap().is_err());
        });
    }

    #[test]
    fn nonfinite_float_wav_samples_are_rejected_before_conversion() {
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            with_wav_fixture(3, 44_100, 32, 4, &value.to_le_bytes(), |path| {
                assert!(
                    AudioData::from_file(path).is_err(),
                    "nonfinite PCM must be rejected"
                );
            });
        }
    }

    #[test]
    fn corrupt_scheme_reports_failure_and_a_later_selection_recovers() {
        with_wav_fixture(1, 0, 16, 2, &[0, 64], |path| {
            let (mut tickeys, channels, mut player, plays) = player();
            tickeys.schemes.push(AudioScheme {
                name: "corrupt".into(),
                display_name: "Corrupt fixture".into(),
                files: vec![path.file_name().unwrap().to_str().unwrap().into()],
                non_unique_count: 1,
                key_audio_map: BTreeMap::new(),
            });
            tickeys
                .load_scheme(path.parent().unwrap().to_str().unwrap(), "corrupt")
                .unwrap();
            player.apply_settings(&channels);
            assert!(matches!(
                tickeys.audio_status().scheme,
                SchemeStatus::Failed(_)
            ));
            tickeys.test_sound();
            assert!(!player.play(channels.play_rx.recv().unwrap()).unwrap());
            assert!(plays.is_empty());
            tickeys.load_scheme(&root("bubble"), "bubble").unwrap();
            player.apply_settings(&channels);
            assert_eq!(tickeys.audio_status().scheme, SchemeStatus::Ready);
            tickeys.test_sound();
            assert!(player.play(channels.play_rx.recv().unwrap()).unwrap());
            assert!(plays.try_recv().is_ok());
        });
    }

    #[test]
    fn every_bundled_wav_decodes_and_manifest_is_valid() {
        let mut names = HashSet::new();
        for scheme in schemes() {
            assert!(names.insert(scheme.name.clone()), "duplicate scheme name");
            scheme.validate().unwrap();
            let directory = root(&scheme.name);
            let loaded = decode_scheme(SchemeRequest {
                generation: 1,
                directory: directory.clone(),
                scheme,
            })
            .unwrap();
            assert_eq!(loaded.data.len(), loaded.scheme.files.len());
            for entry in std::fs::read_dir(directory).unwrap() {
                let path = entry.unwrap().path();
                if path.extension().is_some_and(|ext| ext == "wav") {
                    let audio = AudioData::from_file(&path).unwrap();
                    let legacy: Vec<f32> = Decoder::new(BufReader::new(File::open(path).unwrap()))
                        .unwrap()
                        .convert_samples()
                        .collect();
                    assert_eq!(audio.samples.as_ref(), legacy.as_slice());
                }
            }
        }
    }

    #[test]
    fn playback_sources_share_pcm_but_have_independent_cursors() {
        let audio = AudioData {
            samples: vec![0.25, -0.5].into(),
            sample_rate: 2,
            channels: 1,
        };
        let mut first = audio.source();
        let second = audio.source();
        assert!(Arc::ptr_eq(&first.samples, &second.samples));
        assert_eq!(first.total_duration(), Some(Duration::from_secs(1)));
        assert_eq!(first.next(), Some(0.25));
        assert_eq!(first.size_hint(), (1, Some(1)));
        drop(audio);
        assert_eq!(first.collect::<Vec<_>>(), vec![-0.5]);
        assert_eq!(second.collect::<Vec<_>>(), vec![0.25, -0.5]);
    }

    #[test]
    fn full_play_queue_cannot_drop_latest_volume_or_pitch() {
        let (mut tickeys, channels, mut player, plays) = player();
        for _ in 0..PLAY_QUEUE_CAPACITY + 10 {
            tickeys.handle_keydown(0);
        }
        for i in 0..1000 {
            tickeys.set_volume(i as f32 / 1000.0);
            tickeys.set_pitch(0.5);
        }
        tickeys.set_volume(0.0025);
        tickeys.set_pitch(1.75);
        assert_eq!(channels.play_rx.len(), PLAY_QUEUE_CAPACITY);
        assert_eq!(channels.wake_rx.len(), 1);
        player.apply_settings(&channels);
        // This assertion concerns backpressure, not expiration.
        let mut request = channels.play_rx.recv().unwrap();
        request.queued_at = Instant::now();
        assert!(player.play(request).unwrap());
        let (_, volume, pitch) = plays.recv().unwrap();
        assert_eq!((volume, pitch), (0.0025, 1.75));
    }

    #[test]
    fn pending_scheme_requests_coalesce_and_old_keys_are_not_remapped() {
        let (mut tickeys, channels, mut player, plays) = player();
        tickeys.handle_keydown(36);
        // The nonexistent path must not be decoded: the latest request supersedes it.
        tickeys.load_scheme("/does-not-exist", "sword").unwrap();
        tickeys
            .load_scheme(&root("typewriter"), "typewriter")
            .unwrap();
        player.apply_settings(&channels);
        assert!(!player.play(channels.play_rx.recv().unwrap()).unwrap());
        tickeys.handle_keydown(36);
        assert!(player.play(channels.play_rx.recv().unwrap()).unwrap());
        let expected =
            AudioData::from_file(Path::new(&root("typewriter")).join("return-new.wav")).unwrap();
        assert_eq!(plays.recv().unwrap().0, expected.samples.len());
        assert_eq!(tickeys.audio_status().scheme, SchemeStatus::Ready);
    }

    #[test]
    fn failed_decode_is_reported_without_panicking_or_playing_old_scheme() {
        let (mut tickeys, channels, mut player, _) = player();
        tickeys.load_scheme("/does-not-exist", "sword").unwrap();
        assert_eq!(tickeys.audio_status().scheme, SchemeStatus::Loading);
        player.apply_settings(&channels);
        assert!(matches!(
            tickeys.audio_status().scheme,
            SchemeStatus::Failed(_)
        ));
        tickeys.handle_keydown(0);
        assert!(!player.play(channels.play_rx.recv().unwrap()).unwrap());
        assert!(tickeys.load_scheme("", "missing").is_err());
    }

    #[test]
    fn a_decode_finishing_after_a_new_selection_cannot_overwrite_it() {
        let (mut tickeys, channels, mut player, _) = player();
        tickeys.load_scheme(&root("sword"), "sword").unwrap();
        player.apply_settings_with_decoder(&channels, |request| {
            // Simulate UI changes while the worker is decoding. This also
            // verifies that decoding never holds the settings mailbox lock.
            tickeys
                .load_scheme(&root("typewriter"), "typewriter")
                .unwrap();
            tickeys.set_volume(0.125);
            tickeys.set_pitch(0.75);
            decode_scheme(request)
        });
        assert_eq!(tickeys.audio_status().scheme, SchemeStatus::Loading);
        assert_eq!(player.scheme.as_ref().unwrap().scheme.name, "bubble");
        player.apply_settings(&channels);
        assert_eq!(player.scheme.as_ref().unwrap().scheme.name, "typewriter");
        assert_eq!((player.volume, player.pitch), (0.125, 0.75));
        assert_eq!(tickeys.audio_status().scheme, SchemeStatus::Ready);
    }

    #[test]
    fn expired_keys_are_not_replayed_after_a_stall() {
        let (tickeys, channels, mut player, _) = player();
        tickeys.handle_keydown(0);
        let mut request = channels.play_rx.recv().unwrap();
        request.queued_at = Instant::now() - MAX_PLAY_AGE - Duration::from_millis(1);
        assert!(!player.play(request).unwrap());
    }

    #[test]
    fn audio_worker_retries_initial_output_and_preserves_settings() {
        let (audio, channels) = audio_channels();
        let mut tickeys = Tickeys::new(schemes(), audio);
        tickeys.load_scheme(&root("bubble"), "bubble").unwrap();
        tickeys.set_volume(0.4);
        tickeys.set_pitch(1.5);
        let attempts = Arc::new(AtomicUsize::new(0));
        let (ready_tx, ready_rx) = bounded(1);
        let (plays_tx, plays_rx) = crossbeam_channel::unbounded();
        let counter = Arc::clone(&attempts);
        let handle = std::thread::spawn(move || {
            audio_worker_with_factory(channels, Duration::ZERO, Duration::ZERO, move || {
                if counter.fetch_add(1, Ordering::SeqCst) == 0 {
                    Err("simulated output unavailable".into())
                } else {
                    ready_tx.send(()).unwrap();
                    Ok(Box::new(RecordingOutput(plays_tx.clone())) as Box<dyn AudioOutput>)
                }
            });
        });
        ready_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        tickeys.test_sound();
        let (_, volume, pitch) = plays_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!((volume, pitch), (0.4, 1.5));
        drop(tickeys);
        handle.join().unwrap();
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn playback_error_discards_output_but_preserves_scheme_and_settings() {
        let (mut tickeys, channels, mut player, _) = player();
        tickeys.set_volume(0.25);
        tickeys.set_pitch(1.75);
        player.apply_settings(&channels);
        player.output = Some(Box::new(FailingOutput));
        tickeys.test_sound();
        assert!(player.play(channels.play_rx.recv().unwrap()).is_err());
        assert!(player.output.is_none());
        let (tx, rx) = bounded(1);
        player.output = Some(Box::new(RecordingOutput(tx)));
        tickeys.test_sound();
        assert!(player.play(channels.play_rx.recv().unwrap()).unwrap());
        let (_, volume, pitch) = rx.recv().unwrap();
        assert_eq!((volume, pitch), (0.25, 1.75));
    }

    #[test]
    fn audio_worker_rebuilds_after_a_playback_error() {
        let (audio, channels) = audio_channels();
        let mut tickeys = Tickeys::new(schemes(), audio);
        tickeys.load_scheme(&root("bubble"), "bubble").unwrap();
        tickeys.set_volume(0.25);
        tickeys.set_pitch(1.75);
        let (ready_tx, ready_rx) = bounded(2);
        let (plays_tx, plays_rx) = crossbeam_channel::unbounded();
        let handle = std::thread::spawn(move || {
            let mut attempts = 0;
            audio_worker_with_factory(channels, Duration::ZERO, Duration::ZERO, move || {
                attempts += 1;
                ready_tx.send(attempts).unwrap();
                if attempts == 1 {
                    Ok(Box::new(FailingOutput) as Box<dyn AudioOutput>)
                } else {
                    Ok(Box::new(RecordingOutput(plays_tx.clone())) as Box<dyn AudioOutput>)
                }
            });
        });
        assert_eq!(ready_rx.recv_timeout(Duration::from_secs(5)).unwrap(), 1);
        tickeys.test_sound();
        assert_eq!(ready_rx.recv_timeout(Duration::from_secs(5)).unwrap(), 2);
        tickeys.test_sound();
        let (_, volume, pitch) = plays_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!((volume, pitch), (0.25, 1.75));
        drop(tickeys);
        handle.join().unwrap();
    }

    #[test]
    fn output_retry_delay_grows_and_is_capped() {
        let start = Instant::now();
        let mut retry = RetrySchedule::new(start, Duration::from_secs(1), Duration::from_secs(4));
        for seconds in [1, 2, 4, 4] {
            retry.record_failure(start);
            assert_eq!(retry.wait_duration(start), Duration::from_secs(seconds));
        }
        retry.reset(start);
        assert!(retry.is_due(start));
    }

    #[test]
    fn invalid_pitch_never_reaches_the_player() {
        for (input, expected) in [
            (0.1, MIN_PITCH),
            (3.0, MAX_PITCH),
            (f32::NAN, 1.0),
            (f32::INFINITY, 1.0),
        ] {
            assert_eq!(normalize_pitch(input), expected);
        }
    }
}
