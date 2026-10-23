## 1.0.8 — source complete, not yet published

- Recheck Input Monitoring at launch, activation, opening Preferences, wake and event-tap invalidation. Defer failed-tap rebuilding to the main thread; limit the system permission request to once per launch and keep Preferences usable.
- Share menu/Preferences state, use the engine's `0.25x–2.0x` pitch range, show fractional volume and scheme display names, and add audio/input status plus independent Test Sound.
- Separate latest settings from the bounded key queue, decode schemes on the audio worker, share PCM storage, and discard superseded requests and stale events.
- Preserve ambiguous unversioned low volume values as percentages and record the preference format version.
- Localize About and all five links; prefer bundled LICENSE and THIRD_PARTY_NOTICES for offline access.
- Replace **32 WAVs in five undocumented packs** with reproducible original synthesis. Retain 21 licensed Bubble/Mechanical/Drum recordings. Preserve scheme IDs and key maps; use Synth Click/Thock/Space/Blades/Typewriter names instead of implying original Cherry or film recordings.
- Bundle complete audio credits, 46 target-resolved dependency inventories with 103 license/notice files, and Rust runtime licenses from the build toolchain.
- Give each local release/debug build a fresh output directory. Official packaging requires a clean tree and annotated version tag at HEAD, pins the original certificate's SHA-1/SHA-256, verifies resources and ZIP/DMG contents, and refuses to replace existing artifacts.
- Align Base/en/zh-Hans localization and build metadata; add stable/MSRV CI, input-recovery policy regression coverage, Python release-policy tests and generated-resource checks.

The source version is 1.0.8. Official signing and publication have not been performed and require the maintainer's original signing Mac. Physical permission/keyboard/sleep-wake acceptance remains pending; asynchronous audio-device changes remain a disclosed limitation. See [release notes](docs/release-v1.0.8.md) and the [verification record](docs/development-audio-settings.md) for execution status.

## 1.0.7 (2026-09-01)

- Force-load ServiceManagement and repair Start at Login state handling, NSError propagation, approval guidance, and UI rollback
- Migrate legacy 0..1 menu volume values to the canonical 0..100 percentage format
- Retry CoreAudio initialization after startup and rebuild audio output when a playback call reports a device error
- Attempt to re-enable the keyboard event tap after timeout, user disablement, and system wake
- Add a localized Input Monitoring recovery prompt for stale permissions after upgrades
- Sign official releases with one persistent, free self-signed identity so future updates can retain the same Input Monitoring identity
- Correct IOKit power message constants and acknowledge sleep notifications
- Reopen Preferences when an LSUIElement app has no visible menu-bar icon
- Stop leaking rebuilt NSMenu submenus
- Synchronize version and macOS metadata and verify DMG/ZIP/checksum release artifacts
- Add regression tests for preference migration, power constants, and bundled scheme manifest/file references

Known v1.0.7 limitations, including incomplete permission re-check, event-tap rebuild, asynchronous device-change coverage, UI synchronization, and low-volume migration edge cases, are documented in [docs/release-v1.0.7.md](docs/release-v1.0.7.md).

## 1.0.0 (2026-05-28)
Tickeys Redux — arm64 native port.

- Replace OpenAL with rodio (pure Rust, system CoreAudio)
- Migrate from cocoa 0.2 to objc2 0.6 for macOS 26+ compatibility
- Remove libalut from the Redux runtime; the rebuilt app ships no bundled third-party dylibs
- NSStatusBar menu with scheme/volume/pitch controls
- Global key detection through CGEventTap
- EN/zh-Hans localization via NSBundle Localizable.strings
- Drop update checker
- Edition 2021, arm64 only

## Selected entries from the original Tickeys source changelog

The following selected entries come from the public upstream source changelog and are retained as attribution and compatibility context. They are not the complete original release history; see the [original repository](https://github.com/yingDev/Tickeys) and [official release history](https://www.yingdev.com/projects/tickeys).

### 0.5.0
增加"爆裂鼓手"音效
增加排除列表
设置界面改变
再次运行程序自动打开设置界面

### 0.4.2
修正系统睡眠恢复后失效问题

### 0.4.1
修正因在10.11下编译导致10.10中无法运行的问题

### 0.4.0
修正快速输入时声音不连贯问题
检查更新显示更新内容
增加2款Cherry音效
