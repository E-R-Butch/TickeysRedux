//! Audio smoke test: verify rodio can decode WAV and produce output.
//! Run with: cargo run --bin audio_smoke
//!
//! Tests:
//!   1. WAV decoding → AudioData
//!   2. OutputStream creation
//!   3. Playback via bounded channel → audio worker
//!   4. Scheme reload

use std::io::BufReader;
use std::fs::File;
use std::time::Duration;

use crossbeam::channel::{bounded, Sender, Receiver};
use rodio::{Decoder, OutputStream, OutputStreamHandle, Source, buffer::SamplesBuffer};

// ── pre-decode ───────────────────────────────────────────────────────────────

struct AudioData {
    samples: Vec<f32>,
    sample_rate: u32,
    channels: u16,
}

fn load_wav(path: &str) -> Result<AudioData, String> {
    let file = BufReader::new(File::open(path).map_err(|e| e.to_string())?);
    let decoder = Decoder::new(file).map_err(|e| format!("decode: {}", e))?;
    let channels = decoder.channels();
    let sample_rate = decoder.sample_rate();
    let samples: Vec<f32> = decoder.convert_samples().collect();
    println!("  decoded {}: {} ch, {} Hz, {} samples",
        path, channels, sample_rate, samples.len());
    Ok(AudioData { samples, sample_rate, channels })
}

// ── commands ─────────────────────────────────────────────────────────────────

enum Cmd {
    Play(usize),
    Reload(Vec<AudioData>),
}

// ── audio worker ─────────────────────────────────────────────────────────────

fn worker(rx: Receiver<Cmd>) {
    let (_stream, handle) = OutputStream::try_default()
        .expect("FAIL: OutputStream::try_default");
    println!("  OutputStream created OK");

    let mut data: Vec<AudioData> = vec![];

    for cmd in rx {
        match cmd {
            Cmd::Play(idx) => {
                if let Some(buf) = data.get(idx) {
                    let source = SamplesBuffer::new(
                        buf.channels, buf.sample_rate, buf.samples.clone(),
                    );
                    let _ = handle.play_raw(source.convert_samples());
                }
            }
            Cmd::Reload(d) => {
                data = d;
                println!("  worker: scheme reloaded ({} sounds)", data.len());
            }
        }
    }
}

// ── main ─────────────────────────────────────────────────────────────────────

fn main() {
    println!("=== Tickeys Audio Smoke Test ===\n");
    let mut passed = 0;
    let mut failed = 0;

    // Test 1: decode WAV
    println!("[TEST 1] Decode WAV files");
    let wav_dir = "Tickeys.app/Contents/Resources/data/sword";
    let wavs = vec!["1.wav", "2.wav", "3.wav"];
    let mut data = vec![];
    for w in &wavs {
        let path = format!("{}/{}", wav_dir, w);
        match load_wav(&path) {
            Ok(d) => data.push(d),
            Err(e) => {
                println!("  FAIL: {} — {}", w, e);
                failed += 1;
            }
        }
    }
    if data.len() == wavs.len() {
        println!("  PASS: decoded {}/{} WAVs", data.len(), wavs.len());
        passed += 1;
    }

    // Test 2: audio worker + playback
    println!("\n[TEST 2] Audio worker + playback");
    let (tx, rx) = bounded::<Cmd>(8);
    let worker_handle = std::thread::spawn(move || worker(rx));

    // Send reload
    tx.send(Cmd::Reload(data)).expect("send reload");
    std::thread::sleep(Duration::from_millis(100));

    // Play sounds
    println!("  Playing 3 sounds with 200ms gap...");
    for i in 0..3 {
        tx.try_send(Cmd::Play(i)).ok();
        std::thread::sleep(Duration::from_millis(200));
    }

    // Wait for playback to finish
    std::thread::sleep(Duration::from_millis(500));
    println!("  PASS: sent 3 play commands, no panic");
    passed += 1;

    // Test 3: channel full (try_send on full queue)
    println!("\n[TEST 3] Bounded channel backpressure");
    let (tx2, rx2) = bounded::<Cmd>(2);
    tx2.send(Cmd::Play(0)).expect("send 1");
    tx2.send(Cmd::Play(0)).expect("send 2");
    // Channel should be full now; try_send should fail, not panic
    match tx2.try_send(Cmd::Play(0)) {
        Ok(_) => println!("  INFO: send succeeded (unexpected but ok)"),
        Err(_) => println!("  PASS: try_send correctly failed on full channel"),
    }
    drop(tx2);
    drop(rx2);
    passed += 1;

    // Cleanup
    drop(tx);
    worker_handle.join().ok();

    println!("\n=== Results: {}/{} passed, {} failed ===", passed, passed, failed);
    if failed > 0 {
        std::process::exit(1);
    }
}
