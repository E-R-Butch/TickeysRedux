# Roadmap

## v1.1 — 体验补完

### 开机自启
菜单栏增加 "Launch at Login" 开关，写入 `~/Library/LaunchAgents/com.tickeys.redux.plist` 实现。无需 helper app。

### 全局静音热键
`Ctrl+Option+Shift+M` 一键静音/取消。CGEventTap 已能捕获全局按键，hotkey 匹配加 mute toggle 即可。

### 音效预览
菜单中 hover/选中方案时播放一次示范音。避免切方案后打字试音的来回操作。

---

## v1.2 — 音效增强

### 随机音调偏移
同一 keycode 永远播同一 WAV 会让连续打字有机械重复感。在 playback 路径上对每次播放加 ±3% 随机 pitch jitter，模拟真实键盘的键间差异。仅在用户设置 pitch > 1.0 时不生效（用户已刻意调速）。

### 精准音量
现在 25%/50%/75%/100% 四档对机械键盘方案太粗。改为连续滑块，或至少 10 档（10% 步进）。

---

## v1.3 — 自定义

### 自定义音效导入
菜单 "Import WAV…"，通过 `NSOpenPanel` 选择文件，复制到 `~/Library/Application Support/Tickeys Redux/custom/`。scheme 动态注册，schemes.json 标记 `"source": "user"` 以区分内置/自定义。导出同理。

### 方案分享
将自定义方案导出为 `.tkrx` 包（JSON + WAV 的 zip），拖入菜单栏即可导入。

### 更多语言
已有 EN / 简体中文。欢迎社区贡献：

- 日本語（ja）
- 한국어（ko）
- 繁體中文（zh-Hant）

只需翻译 `Localizable.strings`（约 30 行），无需改代码。

---

## v2.0 — 跨平台

macOS 版本成熟后，将核心音频引擎抽离为平台无关库，各平台对接原生输入/UI 层。

| 层 | macOS (当前) | Linux | Windows |
|---|---|---|---|
| 键盘监听 | CGEventTap | evdev / X11 | Win32 raw input |
| 菜单栏 UI | objc2 NSStatusBar | libappindicator / kde tray | Win32 tray icon |
| 系统事件 | IOKit 电源监控 | 可砍掉 | 可砍掉 |
| 音频 | rodio（已跨平台） | 无需改 | 无需改 |

`src/tickeys.rs` 已是纯平台无关代码，不依赖 AppKit/CoreFoundation。跨平台工作的核心是将 `event_tap.rs` 和 `settings_ui.rs` 做 feature-gate 拆分：

```
src/
├── tickeys.rs          # 平台无关
├── platform/
│   ├── macos/
│   │   ├── input.rs    # CGEventTap
│   │   └── ui.rs       # NSStatusBar
│   ├── linux/
│   │   ├── input.rs    # evdev
│   │   └── ui.rs       # libappindicator
│   └── windows/
│       ├── input.rs    # Win32
│       └── ui.rs       # Win32 tray
```

---

## 不做

| 功能 | 原因 |
|------|------|
| 更新检测 | 主动推送 break 用户注意力（已移除） |
| 云端同步 | 本地工具不需要服务器依赖 |
| 方案市场 | 过早社区化，vibe coding 项目不需要 |
| 统计/遥测 | 不收集用户数据，永远不做 |
| Intel 支持 | 原版 Tickeys 已覆盖 |

---

## 技术债（需要时再清理）

| 项 | 说明 |
|----|------|
| 代码签名 | ad-hoc 每次 rebuild 需重授权，签名后消除 |
| 公证 | 不公证则每次 Gatekeeper 弹窗需右键打开 |
| objc2 API 废弃警告 | `msg_send!` 无逗号语法将在未来 Rust 版本失效 |
