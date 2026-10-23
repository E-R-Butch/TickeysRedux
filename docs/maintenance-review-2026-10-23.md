# Reliability maintenance review — 2026-10-23

Base: `abae1ec1510c0e3ca349ffc610c0d1fab5ab3e66`, the head of PR #1. This follow-up does not repeat that PR's offline documentation-link fix or change its signing identity, sound assets, settings schema, release version, or upstream dependencies' versions.

## Confirmed findings and fixes

### Corrupt WAV resources could silently succeed or stop the audio worker

`AudioData::from_file` validated PCM only after rodio conversion. The locked rodio 0.20.1 WAV adapter pads sample-read errors with zero and converts non-finite float values before those checks. It also divides by the declared sample rate when constructing the decoder, before the application's sample-rate guard.

Three minimal fixtures demonstrated the failure on hosted macOS: truncated PCM and non-finite float samples were accepted; a zero sample-rate header panicked. See the [unfixed regression run](https://github.com/E-R-Butch/TickeysRedux/actions/runs/39918608146), which contains the regressions without the production fix.

The loader now uses the already-locked hound 3.5.1 parser explicitly to validate metadata and every raw sample before invoking the unchanged rodio conversion. This is a local guard, not an upstream upgrade or a claim to implement output-device hot-plug recovery. Corrupt selections report `SchemeStatus::Failed`; a later valid selection recovers. Regression coverage checks rejection without panic, subsequent recovery, and exact decoded-sample equality with the previous conversion for all 53 bundled WAVs. No audio file was modified. Validation adds a sample scan when loading a scheme, not on the key-event or playback path.

### Packaging could overwrite an output created after its preflight check

`create_dmg` and final release publication checked for existing names, then used `shutil.move`. An integration regression created an existing DMG during the mocked unmount step and showed that the old code silently replaced it.

A shared publisher now copies to a private staging file on the destination filesystem, flushes it, and claims the final name with atomic `link()`. Files, directories, and symlinks, including dangling symlinks, cause failure without replacement. Copy/link failures clean up only the private staging file; existing outputs are never removed. Filesystems without hard-link support fail closed. Release checksums are published last, and partial publication reports failure while retaining prior outputs for inspection.

Tests cover the actual DMG and release call paths with macOS commands mocked, two simultaneous publishers, destination collisions, symlinks, partial-copy failure, unsupported filesystems, and checksum consistency. No official release artifact was produced, overwritten, or deleted.

## Review coverage

- Audio worker: latest-settings mailbox, bounded key queue, generation/expiry filtering, decoding, shared playback samples, retry state, synchronous output failures, and shutdown signaling.
- Input/lifecycle: main-run-loop ownership, deferred event-tap recovery, permission-request completion, bounded retries, sleep/wake notifications, and monitor teardown.
- Settings/UI: persisted normalization, scheme selection, shared menu/window state, status polling, reopen behavior, login-item state projection, and bundled About links.
- Packaging: exact source/tag checks, certificate pins, metadata/resource hashes, localization and notices assembly, and final output publication.
- Existing baseline checks passed locally: 13 Python tests and the 53-file/32-synthetic audio check. The final Python suite contains 21 tests. Final Rust/build/license verification is recorded in the pull request's hosted macOS checks.

This is a focused source-and-regression review, not proof that every path is defect-free. No additional high-confidence source defect was established in the reviewed lifecycle/settings paths.

## Exclusions and remaining acceptance

The editing environment was Linux with Python, without an available Rust/macOS toolchain. Rust tests, formatting, Clippy, dependency-notice verification and arm64 app-bundle checks therefore run on the existing hosted macOS workflow. Packaging race tests use fixture files and mocked build/sign/mount commands, not official signing.

Actual TCC grant/revoke, global keyboard capture, audible output, device route/hot-plug changes, status-bar interactions, and physical sleep/wake require a real Mac and remain unverified in this review. Official signing, merging, tagging, release publication and Homebrew cask updates are outside this change. Pure upstream constraints are not expanded into new maintenance work.
