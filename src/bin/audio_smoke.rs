//! Audio smoke test: verify rodio can decode WAV and produce output.
//! Run with: cargo run --bin audio_smoke
//!
//! Tests:
//!   1. WAV decoding → AudioData
//!   2. OutputStream creation
//!   3. Playback via bounded channel → audio worker
//!   4. Scheme reload

use std::fs::File;
use std::io::BufReader;
use std::time::Duration;

use crossbeam::channel::{Receiver, Sender, TrySendError, bounded};
use rodio::{Decoder, OutputStream, Source, buffer::SamplesBuffer};

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
    println!(
        "  decoded {}: {} ch, {} Hz, {} samples",
        path,
        channels,
        sample_rate,
        samples.len()
    );
    Ok(AudioData {
        samples,
        sample_rate,
        channels,
    })
}

// ── commands ─────────────────────────────────────────────────────────────────

enum Cmd {
    Play(usize),
    Reload(Vec<AudioData>),
}

// ── audio worker ─────────────────────────────────────────────────────────────

fn worker(rx: Receiver<Cmd>, ready: Sender<Result<(), String>>) -> Result<(), String> {
    let (_stream, handle) = match OutputStream::try_default() {
        Ok(stream) => stream,
        Err(error) => {
            let message = format!("OutputStream::try_default failed: {error}");
            let _ = ready.send(Err(message.clone()));
            return Err(message);
        }
    };
    println!("  OutputStream created OK");
    ready
        .send(Ok(()))
        .map_err(|_| "audio smoke readiness receiver was dropped".to_string())?;

    let mut data: Vec<AudioData> = vec![];

    for cmd in rx {
        match cmd {
            Cmd::Play(idx) => {
                let buf = data
                    .get(idx)
                    .ok_or_else(|| format!("audio index {idx} is not loaded"))?;
                let source = SamplesBuffer::new(buf.channels, buf.sample_rate, buf.samples.clone());
                handle
                    .play_raw(source.convert_samples())
                    .map_err(|error| format!("play_raw failed for index {idx}: {error}"))?;
            }
            Cmd::Reload(d) => {
                data = d;
                println!("  worker: scheme reloaded ({} sounds)", data.len());
            }
        }
    }

    Ok(())
}

fn join_worker(handle: std::thread::JoinHandle<Result<(), String>>) -> Result<(), String> {
    match handle.join() {
        Ok(result) => result,
        Err(payload) => {
            let reason = if let Some(message) = payload.downcast_ref::<&str>() {
                (*message).to_string()
            } else if let Some(message) = payload.downcast_ref::<String>() {
                message.clone()
            } else {
                "unknown panic payload".to_string()
            };
            Err(format!("audio worker panicked: {reason}"))
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
    let wav_dir = "assets/data/sword";
    let wavs = vec!["1.wav", "2.wav", "3.wav"];
    let mut data = vec![];
    let mut decode_errors = vec![];
    for w in &wavs {
        let path = format!("{}/{}", wav_dir, w);
        match load_wav(&path) {
            Ok(d) => data.push(d),
            Err(e) => {
                println!("  FAIL: {} — {}", w, e);
                decode_errors.push(e);
            }
        }
    }
    if data.len() == wavs.len() {
        println!("  PASS: decoded {}/{} WAVs", data.len(), wavs.len());
        passed += 1;
    } else {
        println!(
            "  FAIL: {} WAV file(s) could not be decoded",
            decode_errors.len()
        );
        failed += 1;
    }

    // Test 2: audio worker + playback
    println!("\n[TEST 2] Audio worker + playback");
    let (tx, rx) = bounded::<Cmd>(8);
    let (ready_tx, ready_rx) = bounded::<Result<(), String>>(1);
    let worker_handle = std::thread::spawn(move || worker(rx, ready_tx));

    let playback_result = match ready_rx.recv() {
        Ok(Ok(())) => {
            let commands_result = (|| -> Result<(), String> {
                tx.send(Cmd::Reload(data))
                    .map_err(|_| "audio worker disconnected before scheme reload".to_string())?;
                std::thread::sleep(Duration::from_millis(100));

                println!("  Playing 3 sounds with 200ms gap...");
                for i in 0..3 {
                    tx.send(Cmd::Play(i))
                        .map_err(|_| format!("audio worker disconnected before play {i}"))?;
                    std::thread::sleep(Duration::from_millis(200));
                }

                // The longest sample used here is under 600 ms.
                std::thread::sleep(Duration::from_millis(500));
                Ok(())
            })();
            drop(tx);
            let worker_result = join_worker(worker_handle);
            match (commands_result, worker_result) {
                (_, Err(error)) | (Err(error), Ok(())) => Err(error),
                (Ok(()), Ok(())) => Ok(()),
            }
        }
        Ok(Err(error)) => {
            drop(tx);
            let worker_error = join_worker(worker_handle).err();
            Err(worker_error.unwrap_or(error))
        }
        Err(_) => {
            drop(tx);
            join_worker(worker_handle).and(Err(
                "audio worker exited before reporting readiness".to_string()
            ))
        }
    };

    match playback_result {
        Ok(()) => {
            println!("  PASS: audio worker completed all play commands");
            passed += 1;
        }
        Err(error) => {
            println!("  FAIL: {error}");
            failed += 1;
        }
    }

    // Test 3: channel full (try_send on full queue)
    println!("\n[TEST 3] Bounded channel backpressure");
    let (tx2, rx2) = bounded::<Cmd>(2);
    tx2.send(Cmd::Play(0)).expect("send 1");
    tx2.send(Cmd::Play(0)).expect("send 2");
    // Channel should be full now; try_send should fail, not panic
    match tx2.try_send(Cmd::Play(0)) {
        Err(TrySendError::Full(_)) => {
            println!("  PASS: try_send correctly failed on full channel");
            passed += 1;
        }
        Ok(_) => {
            println!("  FAIL: try_send unexpectedly succeeded on a full channel");
            failed += 1;
        }
        Err(TrySendError::Disconnected(_)) => {
            println!("  FAIL: backpressure receiver disconnected unexpectedly");
            failed += 1;
        }
    }
    drop(tx2);
    drop(rx2);

    println!(
        "\n=== Results: {}/{} passed, {} failed ===",
        passed,
        passed + failed,
        failed
    );
    if failed > 0 {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::join_worker;

    #[test]
    fn join_worker_propagates_returned_errors() {
        let handle = std::thread::spawn(|| Err("stream failed".to_string()));
        assert_eq!(join_worker(handle), Err("stream failed".to_string()));
    }

    #[test]
    fn join_worker_converts_panics_to_errors() {
        let handle = std::thread::spawn(|| -> Result<(), String> {
            panic!("worker boom");
        });
        let error = join_worker(handle).expect_err("panic must be reported as an error");
        assert!(error.contains("worker boom"));
    }
}
