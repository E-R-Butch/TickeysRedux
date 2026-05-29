# Roadmap

> ✓ = 已完成

## v1.0 — 已发布 ✓

- arm64 原生移植（OpenAL→rodio, cocoa→objc2, edition 2021）
- 8 套内置音效方案（EN/zh-Hans 本地化）
- NSStatusBar 菜单栏设置（方案/音量/音调切换）
- 输入监控权限（系统原生弹窗）
- Liquid Glass 风格应用图标
- DMG 安装包（拖拽至 Applications 引导）
- 中英双语文档
- 仓库整理（legacy 归档、.gitignore）

### v1.0.1 — 音效资产维护 ✓

- **响度归一化** ✓ — 四档系统音量 × 麦克风实测峰值，全部 8 套方案内部极差 < 2.5×（实测峰值）。typewriter 从 33× 降至 1.5×，drum 从 6× 降至 1.8×。原始文件保留为 `.wav.bak`。

---

## v1.1 — 设置面板

### 首次启动向导
新用户首次打开时，用最小步骤完成初始化：选方案 → 听预览 → 授权输入监控。不过度设计，不塞教程。

用 NSPopover 或 NSWindow 实现完整设置面板，替代当前纯 NSMenu 子菜单。菜单栏保留方案快速切换 + 静音，"Preferences…" 打开面板。

面板内容：

- **音量 / 音调** — NSSlider 滑杆 + 可编辑输入框，滑杆在 25/50/75/100%（音量）和 0.5/1.0/1.5/2.0×（音调）点位吸附
- **开机自启** — 勾选框，写入 `~/Library/LaunchAgents/`
- **按 App 排除** — 列表，勾选的应用不触发音效。CGEventTap 取目标进程 bundle ID 比对
- **静音热键** — 快捷键录制控件，默认 `Ctrl+Option+Shift+M`
- **About** — 版本号、作者、GitHub 链接、许可证
- **日夜间模式** ✓ — NSStatusBar/NSMenu 已自动跟随系统。面板窗口需显式继承 `NSAppearance` 以支持 Dark Mode

参考：`legacy/assets/TickeysGUI/` 原版 Xcode 项目的 XIB 布局。

---

## v1.2 — 音效增强

### 按 App 分配音效
不是简单的排除/不排除，而是不同应用自动切换不同方案：Slack → Bubble，Terminal → Typewriter，VS Code → Cherry。CGEventTap 取目标进程 bundle ID，在用户配置的映射表中查找对应方案。

### 抑制系统警告音
删除键在输入框边界时，Tickeys 打字音和系统边界警告音同时触发，两个声音重叠。系统警告音本身有存在价值（告知用户已达边界），需要找到只消除重叠、不消除警告的方案。

### 音效预览
点击方案时，按 0.5× → 1.0× → 2.0× 三个音调各播放 2-3 个按键音，让用户快速感受方案在不同音调区间的表现。而非只播一个音节。

### 打字进度环
菜单栏图标外围一圈环形进度条。按键时进度递增，停止后衰减。连续爆打时环闭合+微光。NSStatusBarButton 高度仅 22px，需用 CoreGraphics 手绘弧线合成 NSImage。

### 菜单栏图标动态切换
根据当前方案和静音状态切换菜单栏图标：Cherry→⌨️、Bubble→🫧、Sword→⚔️、静音→🔇。

### 系统遵从

Tickeys 作为常驻后台应用，应在以下场景自动调整行为。用户可在设置面板中逐项开关。

| 场景 | 行为 | 检测方式 |
|------|------|----------|
| Focus Mode 开启 | 自动静音，退出时恢复 | `NSDistributedNotificationCenter` |
| 屏幕锁定 | 静音 | `CGSessionCopyCurrentUserHasActiveScreenLock` |
| 电池供电 | 降低音频采样率 | IOKit `IOPSNotification` |
| 通话进行中 | Duck 音量（降低 80%）| `AVAudioSession` 或 `AudioObjectGetPropertyData` |
| 耳机拔出 | 不自动切回扬声器 | `AudioHardware` 默认输出设备变更回调 |
| 全屏/演示模式 | 静音 | `NSWorkspace.activeSpaceDidChange` |
| **睡眠/唤醒** ✓ | 暂停/恢复键盘监听 | 已实现（`iokit.rs` 电源事件） |
| **暗色模式** ✓ | 菜单栏图标自适应 | 已实现（NSStatusBar 自动跟随系统） |

设计原则：不打断用户，不抢系统焦点，不在不该出声的时候出声。

### 自定义音效导入
"Import WAV…" → NSOpenPanel → 复制到 `~/Library/Application Support/Tickeys Redux/`。导入时允许用户指定按键类型：普通按键、Space、Enter、Backspace，自动生成 `key_audio_map` 映射。也支持导入整套方案（多个 WAV + 映射配置）。

---

## v1.3 — 社区

### 更多语言
已有 EN / 简体中文 ✓。欢迎贡献：

- 日本語（ja）
- 한국어（ko）
- 繁體中文（zh-Hant）

只需翻译 `Localizable.strings`（约 30 行），无需改代码。

### 打字弹琴
用户可主动启用的"演奏模式"。设置面板中展示可选曲目列表（每首从高潮段落开始，避免冗长前奏），选中后打字即演奏——每次按键推一个音符。暂停超过阈值则从头开始。打字速度 = 演奏速度。

---

## v2.0 — 跨平台

macOS 版成熟后，将核心音频引擎（`src/tickeys.rs`）抽离为平台无关 crate，各平台对接原生输入/UI：

| 层 | macOS (当前) | Linux | Windows |
|---|---|---|---|
| 键盘监听 | CGEventTap | evdev / X11 | Win32 raw input |
| 系统托盘 | objc2 NSStatusBar | libappindicator | Win32 tray |
| 音频 | rodio（已跨平台） | 无需改 | 无需改 |

---

## 技术债

| 项 | 说明 |
|----|------|
| 代码签名 | ad-hoc 每次 rebuild 需重授权，正式签名后消除 |
| 公证 | 未公证则 Gatekeeper 弹窗需右键打开 |
| objc2 废弃语法 ✓ | `msg_send!` 无逗号语法已修正（settings_ui.rs 两处 `forKey`） |
