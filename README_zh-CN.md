# Tickeys Redux

专为 Apple Silicon Mac 打造的原生菜单栏应用，让现代 macOS 重新拥有机械键盘敲击声。

[English](README.md) | 中文

![Tickeys Redux 预览](docs/hero.svg)

**免费、开源、本地运行。不用麦克风。没有遥测。没有云服务。**

[从 Releases 下载](https://github.com/E-R-Butch/TickeysRedux/releases) · [从源码构建](#打包-app-bundle)

Tickeys Redux 是 [Tickeys](https://github.com/yingDev/Tickeys)（应元东）的现代 macOS 移植版。它保留了原版“敲一下就有反馈”的快乐，但把运行时重建为 Apple Silicon 原生：Rust、objc2、rodio、CoreAudio，以及干净的菜单栏体验。

> **仅支持 Apple Silicon。** 本项目面向 arm64 Mac。Intel Mac 用户请使用 [原版 Tickeys](https://github.com/yingDev/Tickeys)。

## 为什么值得试试

- **即时键音反馈**：内置机械键盘、打字机、刀剑、架子鼓、泡泡、Cherry G80 等音效包。
- **菜单栏即控制台**：切换方案、调音量、调音调，不需要完整窗口常驻。
- **隐私边界清楚**：使用 macOS「输入监控」知道“有按键发生”，不是麦克风监听。
- **没有后台网络服务**：没有遥测、云同步、分析上报或更新 beacon。
- **现代原生技术栈**：Rust 2024、objc2、rodio、CoreAudio，以及可审计的一键打包脚本。
- **保留 Tickeys 的味道**：保留原版趣味，移除旧 dylib 依赖。

## Demo

应用安静待在菜单栏。选择音效方案，设置音量和音调，然后开始打字。每次 key-down 都会立刻播放本地 WAV 采样。

```text
菜单栏 -> 音效方案 -> Mechanical / Typewriter / Sword / Drum / Bubble / Cherry
菜单栏 -> 音量     -> 25% / 50% / 75% / 100%
菜单栏 -> 音调     -> 0.5x / 1.0x / 1.5x / 2.0x
```

## 安装

从 [Releases](https://github.com/E-R-Butch/TickeysRedux/releases) 下载最新 `.dmg`，打开后把 `Tickeys Redux.app` 拖到 Applications。

正式发布版使用一张长期不变的免费自签名证书，不购买 Apple Developer ID，也不做公证。如果 macOS 首次启动时拦截，请按住 Control 点击 `Tickeys Redux.app`，选择「打开」，再确认「打开」。

首次启动时，macOS 会请求「输入监控」权限。Tickeys Redux 只需要知道“按键发生了”，不会记录文本，也不会使用麦克风。授权后请再次打开应用；若仍无声，请完全退出 Tickeys Redux 后重新启动。

## v1.0.7 更新

- 开机自启现在完整处理 `SMAppService` 状态、显示真实注册错误，并在 macOS 要求批准时打开“登录项”设置。
- 修复从菜单栏设置音量后，重启时音量再次除以 100、几乎听不见的问题。
- 如果登录启动时 CoreAudio 尚未就绪或输出设备中途断开，音频现在会在后台重连，不再永久无声。
- 键盘监听被系统禁用或电脑睡眠后会自行恢复，不再重启整个进程。
- 升级后若「输入监控」仍绑定旧版本，应用会显示可见的恢复提示并直达正确的系统设置页面。
- 即使隐藏了菜单栏图标，再次双击应用也会打开偏好设置。
- 修复菜单重复操作导致的对象泄漏，并强化发布包校验。

| | 原版 | Redux |
|---|---|---|
| **架构** | x86_64 | **arm64 原生** |
| **音频引擎** | OpenAL + libalut (.dylib) | **rodio**（纯 Rust → CoreAudio） |
| **UI 框架** | cocoa 0.2 + XIB | **objc2 0.6** + NSStatusBar |
| **Rust edition** | 2015 | **2024** |
| **设置界面** | 未完成的 XIB 窗口 | 🎹 **菜单栏** — 方案/音量/音调 |
| **权限** | 无 | **输入监控**（系统原生弹窗） |
| **更新检测** | 内置 | **已移除** |
| **macOS 支持** | 10.10+ | **13+** |

## 使用

1. 启动 `Tickeys Redux.app`
2. 系统弹出权限提示时，授予「输入监控」权限
3. 点击菜单栏 🎹 图标：
   - 切换音效方案（泡泡、樱桃 G80-3000/3494、架子鼓、机械键盘、刀剑、打字机……）
   - 调整音量（25%/50%/75%/100%）
   - 调整音调（0.5×–2.0×）
4. 开始打字 — 即时键音

## 打包 App Bundle

需要 Rust 1.85+。

使用一键脚本：

```sh
git clone https://github.com/E-R-Butch/TickeysRedux.git
cd TickeysRedux
./scripts/package_app.sh
```

脚本会自动完成：release 构建 → 创建 `.app` 结构 → 复制二进制 & 资源 → 写入 `Info.plist` → ad-hoc 签名 & 校验。

也可以手动操作（不推荐）：

```sh
export MACOSX_DEPLOYMENT_TARGET=13.0
cargo build --release --locked

# 创建 .app 结构
mkdir -p "Tickeys Redux.app/Contents/MacOS"
mkdir -p "Tickeys Redux.app/Contents/Resources"
cp target/release/tickeys-redux "Tickeys Redux.app/Contents/MacOS/"
cp -R assets/data "Tickeys Redux.app/Contents/Resources/data"
cp -R assets/lproj/Base.lproj "Tickeys Redux.app/Contents/Resources/Base.lproj"
cp -R assets/lproj/zh-Hans.lproj "Tickeys Redux.app/Contents/Resources/zh-Hans.lproj"
cp assets/tickeys_redux.icns "Tickeys Redux.app/Contents/Resources/tickeys.icns"

# 写入 Info.plist …（见 scripts/package_app.sh）
```

## 自定义音效方案

在 `assets/data/` 下添加自己的 `.wav` 文件，并编辑 `assets/data/schemes.json`：

```json
{
    "name": "myScheme",
    "display_name": "我的方案",
    "files": ["1.wav", "2.wav", "3.wav"],
    "non_unique_count": 3,
    "key_audio_map": {}
}
```

## 技术栈

| 组件 | 库 | 用途 |
|---|---|---|
| 音频 | rodio 0.20 | WAV 解码 + CoreAudio 播放 |
| UI | objc2 0.6 | NSStatusBar, NSMenu, NSAlert |
| 键盘 | CGEventTap (FFI) | 全局按键监听 |
| 并发 | crossbeam 0.8 | 音频工作线程通道 |
| 配置 | serde + serde_json | 方案定义解析 |
| 偏好 | NSUserDefaults | 持久化方案/音量/音调 |

## 权限说明

Tickeys Redux 使用 `CGEventTapCreate` 监听全局按键事件，需要 macOS「输入监控」权限。首次启动时系统会显示授权提示；在系统设置中启用后，请再次打开应用。若仍无声，请完全退出 Tickeys Redux 后重新启动。

每次本地 `cargo build` 仍会使用会变化的 ad-hoc 身份，因此重建后 macOS 可能要求重新授予「输入监控」权限。从 v1.0.7 起，正式发布版使用 [docs/signing.md](docs/signing.md) 记录的长期自签名身份。从旧的 ad-hoc 版本升级时需要修复一次权限；后续使用同一身份签名的版本可以继续沿用。

开机自启使用 Apple 的 `SMAppService`，因此需要 macOS 13 或更高版本。v1.0.7 会显示 macOS 返回的真实注册错误，不再静默假装设置成功。

## 项目元数据

建议设置的 GitHub topics 写在 [docs/github-metadata.md](docs/github-metadata.md)。

## 许可证

MIT — 原版作者 [应元东](https://github.com/yingDev)，Redux 移植 [Sinclair](https://github.com/E-R-Butch)。
