# v1.0.8 implementation and verification

The source and package metadata now identify version **1.0.8**. The maintenance implementation is complete in this tree; official signing and publication have not been performed. The original certificate/private key remains on the maintainer's original signing Mac. Local builds use separate output directories and do not replace an installed app.

## Behavior and implementation

- `app_state` owns the preferences and engine on the main thread. Menu and window actions update that state, persist it, then refresh both surfaces. Scheme manifests are read once at startup instead of on every control action.
- The engine keeps at most 64 key events and one latest-settings mailbox. Wake-up notifications may coalesce, but volume/pitch and the newest scheme request remain in the mailbox. No file decoding or settings mutex runs in the event-tap callback.
- The audio worker decodes an entire scheme before using its mapping. Rapid selections replace pending requests; a finished decode cannot overwrite a newer request. Failed decoding reports an error and silences that selection instead of crashing or playing the previous pack under the new name.
- Key events carry a scheme generation and an enqueue time. Events from older schemes or older than 150 ms are dropped to prevent incorrect mappings and delayed bursts.
- Each overlapping sound uses an independent cursor into an `Arc<[f32]>`. The full PCM allocation is shared and remains alive until all active sounds finish.
- Volume uses percentage points throughout preferences and the UI, then converts once to the engine's 0–1 gain. The old unversioned values cannot distinguish, for example, a legacy `0.5` fraction from a legitimate `0.5%` setting. They are preserved as percentages and stamped with `audio_pref_schema_version = 1`; users of old fractional menu values can reselect their intended volume. Already-amplified values from previous versions cannot be reconstructed automatically.
- The UI uses the engine's `0.25x–2.0x` pitch limits, displays two decimals, uses `display_name` when localization is absent, and keeps a fixed window size. Status polling stops when Preferences is closed. Test Sound uses the current scheme, volume, and pitch without keyboard permission.
- Input Monitoring recovery has one path for launch, activation, opening Preferences, wake and tap invalidation. Destruction/rebuilding is deferred to the main thread after the event callback returns. Healthy taps are reused, revoked permission releases the tap, and failed re-enabling rebuilds it. Bounded retries stop until another lifecycle signal. The system request runs at most once per launch, without an app recovery modal blocking Preferences.
- About uses matching English/Simplified Chinese strings and five links. LICENSE and THIRD_PARTY_NOTICES prefer the bundled files; project/Releases links open only on user action.
- Five sound packs now contain 32 original synthetic WAVs. The 21 licensed recordings are retained unchanged. IDs, filenames and key mappings remain compatible, but synthetic display names no longer imply film audio or real Cherry recordings. See [audio provenance](audio-assets.md).
- Unused direct `ureq`/`libc`, broad crossbeam components, non-WAV codecs and unused framework features are removed. The notices inventory covers 46 normal/build dependencies resolved for arm64 macOS, with 103 license/notice files; that target inventory is distinct from the multi-platform Cargo.lock package count.
- Packaging includes project/dependency/audio licenses and Rust runtime notices from the actual toolchain, Base/en/zh-Hans resources and source/build metadata. Each build has a fresh output directory. Official packaging pins the original certificate's SHA-1/SHA-256 and requires a clean tree and annotated version tag at HEAD; it verifies the extracted ZIP and mounted DMG and refuses to overwrite prior artifacts.

## Verification

Run from the repository root on an Apple Silicon Mac with the Command Line Tools:

```sh
cargo fmt --all -- --check
cargo test --all-targets --locked
cargo clippy --all-targets --locked -- -D warnings
cargo +1.85.0 test --all-targets --locked
python3 scripts/generate_audio_assets.py --check
python3 scripts/generate_notices.py --check
python3 -m unittest discover -s scripts/tests -v
./scripts/package_app.sh
cargo run --release --bin audio_smoke --locked
```

Automated regression coverage includes all 8 manifests and 53 bundled WAVs, full-queue control updates, superseded scheme requests/results, mapping and stale events, decode failure, shared samples, low-volume preservation, pitch normalization and output initialization/playback failures. Six input-recovery policy tests cover permission revocation, tap reuse, failed-reenable replacement, later recovery after creation failure, bounded retries/sleep suppression, and request-once behavior. About tests cover localization parity and offline legal links. Python tests exercise release-source, certificate/resource and packaging policies; these tests do not exercise macOS TCC or physical devices.

The smoke executable opens the real CoreAudio output and verifies playback submissions through the production engine, including a scheme change. An OS sandbox can prevent opening the device; run it in a normal local development terminal. Successful submission does not establish that speakers/headphones were audible or that asynchronous unplug/replug recovery works.

### Historical first-batch validation — 2026-10-13

The earlier audio/settings batch passed 19 tests on Rust 1.99.0 and 1.85.0, formatting, Clippy with warnings denied, arm64 packaging/signature checks, localization parsing, and real CoreAudio initialization/playback submissions. A separate QA bundle was inspected without keyboard permission: status and Test Sound remained available, scheme switching completed, and low volume/pitch settings survived relaunch. The status-bar menu was not exposed by that UI automation, so its complete interaction with the window was not established.

These results precede the final input recovery, About, synthetic audio and release-policy changes. They are not a claim that the complete v1.0.8 build passed those checks.

### Final v1.0.8 validation record

On 2026-10-15, the complete source passed 26 Rust tests on Rust 1.99.0 and Rust 1.85.0, formatting, Clippy with warnings denied, and 10 Python release-policy tests. The generated-audio check verified all 53 WAVs and the 32 reproducible replacements; the offline notice check verified 46 dependencies and 103 texts. Debug and release App bundles passed architecture, deployment-target, resource, metadata and ad-hoc-signature verification. A local ad-hoc DMG was created with create-dmg 1.3.0, mounted read-only, and verified for its app, Applications link, background, Finder layout and bundle integrity; this is not verification of an official certificate-signed distribution. The production smoke test opened CoreAudio, loaded Bubble and Synth Typewriter, and successfully submitted six sounds.

A separate debug QA bundle was inspected without granting keyboard permission. Sound/General/About remained responsive; the Chinese About layout and all five link controls were visible. Both legal-document buttons opened files from its own Resources directory in TextEdit. The input-settings button opened the correct macOS Input Monitoring page without changing any permission. Volume 0.25% and pitch 0.25x remained correct after closing/reopening Preferences, without repeated app permission modals. Hiding the menu-bar icon in the QA preferences and reopening the app also restored Preferences with the hidden setting preserved. Full acceptance still includes the manual checks below.

Hosted GitHub Actions [run 38692966914](https://github.com/E-R-Butch/TickeysRedux/actions/runs/38692966914) passed on 2026-10-15 for source commit `f50d8f6f8671b101670d545d67dd7c4b16e292f5`. Both stable Rust and Rust 1.85.0 passed Rust/Python tests, audio/license manifest checks and the arm64 build; stable also passed formatting, Clippy and complete App Bundle verification. See [PR #1 checks](https://github.com/E-R-Butch/TickeysRedux/pull/1/checks) for subsequent commits.

## Remaining acceptance and release steps

- Exercise the status-bar menu with an already-open Preferences window in both directions, then relaunch and verify persisted settings.
- On a real Mac, grant/deny/regrant and revoke Input Monitoring; verify global-key capture and recovery after returning from System Settings, reopening Preferences, event-tap failure and sleep/wake. Policy tests do not substitute for macOS permission acceptance.
- Record audible output separately from successful CoreAudio submissions. Asynchronous unplug/replug and output-route recovery remain a disclosed limitation, not an implemented/verified new v1.0.8 hot-plug feature.
- Use the original signing Mac for official signing and ZIP/DMG verification before publication; local and hosted automated validation have passed. See [v1.0.8 notes](release-v1.0.8.md) and [signing policy](signing.md).

The v1.1 feature work (global toggle, app filtering, modifier/synthetic-key options, automatic pitch and hidden-icon gesture) remains separate. The project retains the original macOS Tickeys 1.1.0 comparison baseline, local-only operation and free persistent self-signing policy.
