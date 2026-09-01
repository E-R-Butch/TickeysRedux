## 1.0.7 (2026-09-01)

- Force-load ServiceManagement and repair Start at Login state handling, NSError propagation, approval guidance, and UI rollback
- Migrate legacy 0..1 menu volume values to the canonical 0..100 percentage format
- Retry CoreAudio initialization after login startup and rebuild audio output after device errors
- Re-enable the keyboard event tap after timeout, user disablement, and system wake
- Add a localized Input Monitoring recovery prompt for stale permissions after upgrades
- Sign official releases with one persistent, free self-signed identity so future updates can retain the same Input Monitoring identity
- Correct IOKit power message constants and acknowledge sleep notifications
- Reopen Preferences when an LSUIElement app has no visible menu-bar icon
- Stop leaking rebuilt NSMenu submenus
- Synchronize version and macOS metadata and verify DMG/ZIP/checksum release artifacts
- Add regression tests for preference migration, power constants, and bundled audio assets

## 1.0.0 (2026-05-28)
Tickeys Redux — arm64 native port.

- Replace OpenAL with rodio (pure Rust, system CoreAudio)
- Migrate from cocoa 0.2 to objc2 0.6 for macOS 26+ compatibility
- Remove libalut dependency — zero native dylibs
- NSStatusBar menu with scheme/volume/pitch controls
- Native Input Monitoring permission flow
- EN/zh-Hans localization via NSBundle Localizable.strings
- Drop update checker
- Edition 2021, arm64 only

## 0.5.0
增加"爆裂鼓手"音效
增加排除列表
设置界面改变
再次运行程序自动打开设置界面

## 0.4.2
修正系统睡眠恢复后失效问题

## 0.4.1
修正因在10.11下编译导致10.10中无法运行的问题

## 0.4.0
修正快速输入时声音不连贯问题
检查更新显示更新内容
增加2款Cherry音效
