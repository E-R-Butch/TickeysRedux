# Tickeys Redux

专为 Apple Silicon Mac 打造的原生菜单栏应用，让现代 macOS 重新拥有机械键盘敲击声。

[English](README.md) | 中文

![Tickeys Redux 预览](docs/hero.svg)

**免费、开源、本地运行。不用麦克风。没有遥测。没有云服务。**

[从 Releases 下载](https://github.com/E-R-Butch/TickeysRedux/releases) · [从源码构建](#打包-app-bundle)

Tickeys Redux 是 [Tickeys](https://github.com/yingDev/Tickeys)（应元东）的现代 macOS 移植版。它保留了原版“敲一下就有反馈”的快乐，但把运行时重建为 Apple Silicon 原生：Rust、objc2、rodio、CoreAudio、菜单栏控制和原生偏好设置窗口。

> **仅支持 Apple Silicon。** 本项目面向 arm64 Mac。Intel Mac 用户请使用 [原版 Tickeys](https://github.com/yingDev/Tickeys)。

## 为什么值得试试

- **即时键音反馈**：内置泡泡、Cherry G80-3000、Cherry G80-3494、架子鼓、机械键盘、星球大战、刀剑和打字机 8 套方案。
- **原生控制界面**：从菜单栏快速切换方案、调音量和音调，也可以使用三个标签页的偏好设置窗口。
- **隐私边界清楚**：使用 macOS「输入监控」知道“有按键发生”，不是麦克风监听。
- **没有后台网络服务**：没有遥测、云同步、分析上报或更新 beacon。
- **现代原生技术栈**：Rust 2024、objc2、rodio、CoreAudio，以及可审计的一键打包脚本。
- **保留 Tickeys 的味道**：保留原版趣味，移除旧 dylib 依赖。

## Demo

应用安静待在菜单栏。选择音效方案，设置音量和音调，然后开始打字。每次 key-down 都会立刻播放本地 WAV 采样。偏好设置包含“音效”“通用”“关于”三个标签页。

```text
菜单栏 -> 音效方案 -> Bubble / Cherry G80-3000 / Cherry G80-3494 / Drum
                     Mechanical / Star Wars / Sword / Typewriter
菜单栏 -> 音量     -> 25% / 50% / 75% / 100%
菜单栏 -> 音调     -> 0.5x / 0.75x / 1.0x / 1.5x / 2.0x
```

## 安装

从 [Releases](https://github.com/E-R-Butch/TickeysRedux/releases) 下载最新 `.dmg`，打开后把 `Tickeys Redux.app` 拖到 Applications。

正式发布版使用一张长期不变的免费自签名证书，不购买 Apple Developer ID，也不做公证。如果 macOS 首次启动时拦截，请先尝试打开一次，再前往「系统设置 → 隐私与安全性」，滚动到“安全性”，点击「仍要打开」，最后确认「打开」。由管理员管控的 Mac 可能不允许这样放行。

首次启动时，macOS 会请求「输入监控」权限。Tickeys Redux 只使用按键码选择本地音效，不把输入内容写入日志或持久化、不还原文本内容、也不传输输入事件，更不会使用麦克风。授权后请再次打开应用；若仍无声，请完全退出 Tickeys Redux 后重新启动。

## v1.0.7 更新

- 开机自启现在完整处理 `SMAppService` 状态、显示真实注册错误，并在 macOS 要求批准时打开“登录项”设置。
- 修复从菜单栏设置音量后，重启时音量再次除以 100、几乎听不见的问题。
- 如果登录启动时 CoreAudio 尚未就绪，音频线程会重试；播放调用明确返回设备错误时，会重建音频输出。
- 键盘监听被系统禁用或电脑睡眠后，会尝试重新启用 Event Tap，不再重启整个进程。
- 升级后若「输入监控」仍绑定旧版本，应用会显示可见的恢复提示并直达正确的系统设置页面。
- 即使隐藏了菜单栏图标，再次双击应用也会打开偏好设置。
- 修复菜单重复操作导致的对象泄漏，并强化发布包校验。

### v1.0.7 已知限制

- 从系统设置返回应用时，不一定会自动重新检查「输入监控」权限；Event Tap 重新启用失败后也不会完整重建监听。仍然无声时，需要再次打开应用或彻底退出后重启。
- 菜单栏和已经创建的偏好设置窗口还不能在所有方向上即时同步。
- 旧版音量迁移可能把精确的 `0.25%`、`0.5%`、`0.75%` 或 `1%` 误认为旧分数值；音调滑杆还显示了低于引擎实际下限 `0.25x` 的范围。
- 当前只验证了启动失败和同步播放错误后的音频恢复；部分异步 CoreAudio 设备或路由变化仍可能需要重启应用。
- 尚不支持运行时导入音效包；自定义方案目前需要从源码重新构建。

## 与原版 Tickeys 的真实位置

对照基线是原作者最终发布的 macOS [Tickeys 1.1.0](https://www.yingdev.com/projects/tickeys)，而不只是 GitHub 上较旧的源码快照。Redux 已经更新了运行时和权限模型，但 v1.0.7 还没有恢复原版全部用户功能。

| 能力 | 原版 macOS 1.1.0 | Redux 1.0.7 |
|---|---|---|
| **平台与运行时** | Intel 时代版本 | **Apple Silicon 原生** |
| **权限** | 辅助功能 | **仅输入监控** |
| **设置界面** | 偏好设置和状态栏控制 | **三标签偏好设置**和菜单栏控制 |
| **基础控制** | 方案、音量、音调、快速启用/禁用 | 方案、音量、手动音调；快速开关待补 |
| **App 过滤** | 当前 App 排除、黑/白名单 | 待补 |
| **按键选项** | 可选修饰键、模拟按键音效 | 待补 |
| **自动音调** | 随打字速度提升音调 | 待补 |
| **隐藏图标后唤出** | 全局修饰键序列 | 重新打开 App；全局手势待补 |
| **自定义方案** | 公开源码版[文档说明的 App 资源编辑](https://github.com/yingDev/Tickeys#add-custom-schemes) | 仅开发者源码定制；安全导入待补 |
| **更新检查** | 内置 | 坚持无后台更新 beacon；计划提供用户主动检查 |
| **macOS 支持** | 10.15+；允许 10.14 运行但未经测试 | 13+ |

功能对齐与超越原版的安排见 [ROADMAP.md](ROADMAP.md)。

原版公开的 0.5 源码使用 OpenAL + libalut；Redux 将其替换为 rodio + CoreAudio。原版 1.1.0 发布说明没有公开其内部音频引擎版本。

## 使用

1. 启动 `Tickeys Redux.app`
2. 系统弹出权限提示时，授予「输入监控」权限
3. 点击菜单栏 🎹 图标，或打开「偏好设置」：
   - 在全部 8 套音效方案之间切换
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

脚本会自动完成：release 构建 → 创建 `.app` 结构 → 复制二进制和资源 → 写入 `Info.plist` → 默认对完整 App Bundle 进行 ad-hoc 签名和校验。

打包脚本是 App Bundle 目录、资源、部署目标和签名步骤的唯一准确说明。在 Apple Silicon 上，单独运行 `cargo build --release --locked` 会生成由链接器 ad-hoc 签名的 `target/release/tickeys-redux`，但不会生成可安装的 App，也不会使用项目的长期发布身份。

### 校验

```sh
./scripts/package_app.sh
codesign --verify --deep --strict --verbose=2 "Tickeys Redux.app"
plutil -lint "Tickeys Redux.app/Contents/Info.plist"
```

## v1.0.7 的开发者自定义音效方案

v1.0.7 还没有面向普通用户的安全导入流程。不要直接修改已经安装的正式版 App Bundle：这会破坏代码签名，并可能影响「输入监控」中的应用身份。若要把自定义方案编入源码构建，请在 `assets/data/` 下添加 `.wav` 文件，并编辑 `assets/data/schemes.json`：

```json
{
    "name": "myScheme",
    "display_name": "我的方案",
    "files": ["1.wav", "2.wav", "3.wav"],
    "non_unique_count": 3,
    "key_audio_map": {}
}
```

`name` 必须与音效目录名一致；没有对应本地化项时，当前 UI 会显示这个字段。`display_name` 是 v1.0.7 数据格式的必填项，但当前 UI 尚未使用。后续将从 Application Support 安全导入，见 [ROADMAP.md](ROADMAP.md)。

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

Tickeys Redux 使用 `CGEventTapCreate` 接收全局按键码，并据此选择本地音效，需要 macOS「输入监控」权限。应用不会把事件写入磁盘或传输到外部。首次启动时系统会显示授权提示；在系统设置中启用后，请再次打开应用。若仍无声，请完全退出 Tickeys Redux 后重新启动。

在 Apple Silicon 上，`cargo build` 生成由链接器 ad-hoc 签名的 Mach-O，而不是项目的长期发布身份。`scripts/package_app.sh` 默认对完整本地 App Bundle 进行 ad-hoc 签名，因此重新打包后 macOS 可能要求再次授予「输入监控」权限。从 v1.0.7 起，正式发布版使用 [docs/signing.md](docs/signing.md) 记录的长期自签名身份。从旧的 ad-hoc 版本升级时需要修复一次权限；后续使用同一身份签名的版本可以继续沿用。

开机自启使用 Apple 的 `SMAppService`，因此需要 macOS 13 或更高版本。v1.0.7 会显示 macOS 返回的真实注册错误，不再静默假装设置成功。

## 项目元数据

建议设置的 GitHub topics 写在 [docs/github-metadata.md](docs/github-metadata.md)。

## 许可证与音效资源

Redux 源代码采用 MIT 许可证。原版作者 [应元东](https://github.com/yingDev)，Redux 移植 [Sinclair](https://github.com/E-R-Butch)。内置音效包可能采用不同条款，来源状态记录在 [docs/audio-assets.md](docs/audio-assets.md)。
