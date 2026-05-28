# Roadmap

## v1.1 — 设置面板

用 NSPopover 或 NSWindow 实现完整设置面板，替代当前纯 NSMenu 子菜单。菜单栏保留方案快速切换 + 静音，"Preferences…" 打开面板。

面板内容：

- **音量 / 音调** — NSSlider 滑杆 + 可编辑输入框，滑杆在 25/50/75/100%（音量）和 0.5/1.0/1.5/2.0×（音调）点位吸附
- **开机自启** — 勾选框，写入 `~/Library/LaunchAgents/`
- **按 App 排除** — 列表，勾选的应用不触发音效。CGEventTap 取目标进程 bundle ID 比对
- **静音热键** — 快捷键录制控件，默认 `Ctrl+Option+Shift+M`
- **About** — 版本号、作者、GitHub 链接、许可证
- **日夜间模式** — NSStatusBar/NSMenu 已自动跟随系统。面板窗口需显式继承 `NSAppearance` 以支持 Dark Mode

参考：`legacy/assets/TickeysGUI/` 原版 Xcode 项目的 XIB 布局。

---

## v1.2 — 音效增强

### 按 App 分配音效
不是简单的排除/不排除，而是不同应用自动切换不同方案：Slack → Bubble，Terminal → Typewriter，VS Code → Cherry。CGEventTap 取目标进程 bundle ID，在用户配置的映射表中查找对应方案。

### 抑制系统警告音
删除键在边界时 macOS 蜂鸣声和 Tickeys 音效同时响。

### 音效预览
菜单中 hover 方案时播放一次示范音。

### 自定义音效导入
"Import WAV…" → NSOpenPanel → 复制到 `~/Library/Application Support/Tickeys Redux/`。导入时允许用户指定按键类型：普通按键、Space、Enter、Backspace，自动生成 `key_audio_map` 映射。也支持导入整套方案（多个 WAV + 映射配置）。

---

## v1.3 — 社区

### 更多语言
已有 EN / 简体中文。欢迎贡献：

- 日本語（ja）
- 한국어（ko）
- 繁體中文（zh-Hant）

只需翻译 `Localizable.strings`（约 30 行），无需改代码。

### 方案分享
.tkrx 包拖入菜单栏即导入。社区可互相分享自定义方案。

---

## v2.0 — 跨平台

macOS 版成熟后，将核心音频引擎（`src/tickeys.rs`）抽离为平台无关 crate，各平台对接原生输入/UI：

| 层 | macOS (当前) | Linux | Windows |
|---|---|---|---|
| 键盘监听 | CGEventTap | evdev / X11 | Win32 raw input |
| 系统托盘 | objc2 NSStatusBar | libappindicator | Win32 tray |
| 音频 | rodio（已跨平台） | 无需改 | 无需改 |

```
src/
├── tickeys.rs          # 平台无关
├── platform/
│   ├── macos/
│   │   ├── input.rs
│   │   └── ui.rs
│   ├── linux/
│   └── windows/
```

---

## 不做

| 功能 | 原因 |
|------|------|
| 更新检测 | 主动推送干扰用户（已移除） |
| 云端同步 | 本地工具，不需要服务器 |
| 方案市场 | 过早社区化 |
| 统计/遥测 | 永远不收集数据 |
| Intel 支持 | 原版 Tickeys 已覆盖 |

---

## 技术债

| 项 | 说明 |
|----|------|
| 代码签名 | ad-hoc 每次 rebuild 需重授权，正式签名后消除 |
| 公证 | 未公证则 Gatekeeper 弹窗需右键打开 |
| objc2 废弃语法 | `msg_send!` 无逗号语法将在未来 Rust 版本失效 |
