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
