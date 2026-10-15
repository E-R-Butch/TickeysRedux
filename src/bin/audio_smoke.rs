//! Exercise the production audio engine without requesting keyboard permission.
//! Run with `cargo run --bin audio_smoke` from the repository root.

use std::path::Path;
use std::time::{Duration, Instant};
use tickeys_lib::{
    AudioData, AudioScheme, AudioStatus, OutputStatus, SchemeStatus, Tickeys, spawn_audio_worker,
};

fn wait_for(tickeys: &Tickeys, ready: impl Fn(&AudioStatus) -> bool) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let status = tickeys.audio_status();
        if let SchemeStatus::Failed(error) = &status.scheme {
            return Err(error.clone());
        }
        if ready(&status) {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(format!("audio timed out: {status:?}"));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn run() -> Result<(), String> {
    let schemes: Vec<AudioScheme> =
        serde_json::from_str(include_str!("../../assets/data/schemes.json"))
            .map_err(|e| e.to_string())?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/data");
    let mut decoded = 0;
    for scheme in &schemes {
        for file in &scheme.files {
            AudioData::from_file(root.join(&scheme.name).join(file))?;
            decoded += 1;
        }
    }
    println!(
        "PASS: decoded {decoded} manifest WAVs across {} schemes",
        schemes.len()
    );

    let (audio, worker) = spawn_audio_worker()?;
    let mut tickeys = Tickeys::new(schemes, audio);
    tickeys.set_volume(0.15);
    tickeys.set_pitch(1.0);
    let playback = (|| {
        for name in ["bubble", "typewriter"] {
            tickeys.load_scheme(&root.join(name).to_string_lossy(), name)?;
            wait_for(&tickeys, |s| {
                s.scheme == SchemeStatus::Ready && s.output == OutputStatus::Ready
            })?;
            for keycode in [0, 36, 49] {
                let before = tickeys.audio_status().submitted_plays;
                tickeys.handle_keydown(keycode);
                wait_for(&tickeys, |s| s.submitted_plays > before)?;
                std::thread::sleep(Duration::from_millis(200));
            }
            println!(
                "PASS: {name} loaded and three sounds submitted through the production worker"
            );
        }
        std::thread::sleep(Duration::from_millis(600));
        Ok(())
    })();
    drop(tickeys);
    worker
        .join()
        .map_err(|_| "audio worker panicked".to_string())?;
    playback
}

fn main() {
    if let Err(error) = run() {
        eprintln!("FAIL: {error}");
        std::process::exit(1);
    }
    println!(
        "PASS: output stream and playback submissions verified; confirm audible output by listening."
    );
}
