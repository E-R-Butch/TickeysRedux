# Tickeys Redux

专为 Apple Silicon Mac 打造的原生菜单栏应用，为打字提供本地音效。

[English](README.md) | 中文

![Tickeys Redux 预览](docs/hero.svg)

**免费、开源、本地运行。不用麦克风。没有遥测。没有云服务。**

**源码版本：1.0.8。** 本目录已完成实现，正式签名和发布尚未执行。见 [v1.0.8 说明与待验收项](docs/release-v1.0.8.md)。

[从 Releases 下载](https://github.com/E-R-Butch/TickeysRedux/releases) · [从源码构建](#打包-app-bundle)

Tickeys Redux 是 [Tickeys](https://github.com/yingDev/Tickeys)（应元东）的现代 macOS 移植版。它保留了原版“敲一下就有反馈”的快乐，但把运行时重建为 Apple Silicon 原生：Rust、objc2、rodio、CoreAudio、菜单栏控制和原生偏好设置窗口。

> **仅支持 Apple Silicon。** 本项目面向 arm64 Mac。Intel Mac 用户请使用 [原版 Tickeys](https://github.com/yingDev/Tickeys)。

## 为什么值得试试

- **即时键音反馈**：内置泡泡、架子鼓、机械键盘、合成脆响、合成闷响、合成太空、合成刀剑和合成打字机 8 套方案。其中 32 个来源未确认的录音已替换为原创合成音效；另 21 个录音保留原许可。
- **原生控制界面**：从菜单栏快速切换方案、调音量和音调，也可以使用三个标签页的偏好设置窗口。
- **隐私边界清楚**：使用 macOS「输入监控」知道“有按键发生”，不是麦克风监听。
- **没有后台网络服务**：没有遥测、云同步、分析上报或更新 beacon。
- **现代原生技术栈**：Rust 2024、objc2、rodio、CoreAudio，以及可审计的一键打包脚本。
- **保留 Tickeys 的味道**：保留原版趣味，移除旧 dylib 依赖。

## Demo

应用安静待在菜单栏。选择音效方案，设置音量和音调，然后开始打字。每次 key-down 都会立刻播放本地 WAV 采样。偏好设置包含“音效”“通用”“关于”三个标签页。

```text
菜单栏 -> 音效方案 -> 泡泡 / 架子鼓 / 机械键盘 / 合成脆响
                     合成闷响 / 合成太空 / 合成刀剑 / 合成打字机
菜单栏 -> 音量     -> 25% / 50% / 75% / 100%
菜单栏 -> 音调     -> 0.5x / 0.75x / 1.0x / 1.5x / 2.0x
```

## 安装

从 [Releases](https://github.com/E-R-Butch/TickeysRedux/releases) 下载最新 `.dmg`，打开后把 `Tickeys Redux.app` 拖到 Applications。

正式发布版使用一张长期不变的免费自签名证书，不购买 Apple Developer ID，也不做公证。如果 macOS 首次启动时拦截，请先尝试打开一次，再前往「系统设置 → 隐私与安全性」，滚动到“安全性”，点击「仍要打开」，最后确认「打开」。由管理员管控的 Mac 可能不允许这样放行。

首次启动时，macOS 会请求「输入监控」权限。Tickeys Redux 只使用按键码选择本地音效，不把输入内容写入日志或持久化、不还原文本内容、也不传输输入事件，更不会使用麦克风。v1.0.8 在返回应用或打开偏好设置时会重新检查权限。偏好设置显示监听状态，并提供系统设置入口；若升级后旧权限仍失效，可从该入口修复应用条目。

## v1.0.8 源码更新

- 启动、重新激活、打开偏好设置、唤醒和 Event Tap 失效时统一重检「输入监控」；重新启用失败后，在回调返回后由主线程延迟重建。系统授权请求每次启动最多一次，不用应用模态提示阻塞偏好设置。
- 菜单栏与偏好设置共用状态，正确显示小数音量，音调范围统一为 `0.25×–2.0×`；提供音频/监听状态及无需键盘权限的「测试声音」。
- 最新设置与有界击键队列分离，后台解码，共享 PCM 采样，并丢弃过期击键。
- 百分比设置带版本标记；旧版无法区分的低值原样保留，避免意外放大。若旧分数设置导致音量过轻，请重新选择所需音量。
- 「关于」完整支持中英文，提供五个可点击入口；许可证和第三方声明优先打开 App 内的离线文件。
- **五套音效的 32 个 WAV 改为原创程序合成**，旧方案 ID 与按键映射兼容；泡泡、机械键盘和鼓声的 21 个有许可录音保留。v1.0.8 不再包含原星球大战或真实 Cherry 录音，详见 [音效来源](docs/audio-assets.md)。
- 打包附带依赖、音效和 Rust 运行时许可，核对源码与资源，每次输出独立的新构建目录；正式发布强制检查版本标签、干净工作树及原签名证书。

### 已知限制与验收状态

输入生命周期和发布策略已有自动化回归覆盖；真实 macOS 权限授予/撤销、全局键盘捕获、睡眠唤醒，以及菜单栏与窗口交互仍需实机验收。异步 CoreAudio 设备或路由切换可能仍需重启；v1.0.8 没有新增已验证的设备热插拔恢复功能。运行时导入音效仍在计划中。托管 CI、正式签名和发布状态单独记录在 [验证记录](docs/development-audio-settings.md)。

## 与原版 Tickeys 的真实位置

对照基线是原作者最终发布的 macOS [Tickeys 1.1.0](https://www.yingdev.com/projects/tickeys)，而不只是 GitHub 上较旧的源码快照。Redux 已经更新了运行时和权限模型，但 v1.0.8 还没有恢复原版全部用户功能。

| 能力 | 原版 macOS 1.1.0 | Redux 1.0.8 源码 |
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
   - 调整音调（菜单预设 0.5×–2.0×；偏好设置 0.25×–2.0×）
4. 开始打字 — 即时键音

## 打包 App Bundle

需要 Apple Silicon Mac、macOS 13+、Command Line Tools、Rust 1.85+ 和 Python 3。

```sh
git clone https://github.com/E-R-Butch/TickeysRedux.git
cd TickeysRedux
./scripts/package_app.sh
# 可选：本地 debug 构建
./scripts/package_app.sh --debug
# 可选：本地 DMG，需要 create-dmg
./scripts/package_app.sh --dmg dist/Tickeys.Redux.v1.0.8.local.dmg
```

每次成功构建都会输出并打印新的 `dist/builds/v<版本>-<提交>-<profile>-<mode>-<随机值>/Tickeys Redux.app` 路径，保留以前的构建。`--dmg` 可以不带路径，默认使用 `dist/` 中带版本号的本地 DMG 名称；已有 DMG 不会被覆盖。

脚本构建面向 macOS 13 的 arm64 程序，组装资源与语言元数据，复制项目、依赖、音效及实际 Rust 工具链的运行时许可，记录源码提交，默认进行 ad-hoc 签名并验证完整 App。单独执行 `cargo build` 只产生二进制，不生成可安装的 App Bundle。

正式产物使用 `scripts/package_release.sh`：要求干净工作树、指向 HEAD 的附注标签 `v1.0.8`，以及维护者原签名 Mac 上的既有证书/私钥。脚本硬固定证书 SHA-1 和 SHA-256，校验 ZIP、DMG 内的 App，生成校验和及构建信息，并拒绝覆盖已有正式产物。见 [签名政策](docs/signing.md)；无需建立新身份或购买 Developer ID。

### 校验

```sh
cargo fmt --all -- --check
cargo test --all-targets --locked
cargo clippy --all-targets --locked -- -D warnings
python3 scripts/generate_audio_assets.py --check
python3 scripts/generate_notices.py --check
python3 -m unittest discover -s scripts/tests -v
```

已完成检查与实机验收边界见 [验证记录](docs/development-audio-settings.md)。

## 开发者自定义音效方案

目前没有运行时导入功能。不要修改已安装的签名 App Bundle，否则会改变签名并可能影响输入监控身份。源码定制需要在 `assets/data/` 添加 WAV、在 `assets/data/schemes.json` 定义方案，并在 `scripts/generate_audio_assets.py` 登记作者和许可策略，以便重新生成和校验完整音频清单。

```json
{
    "name": "myScheme",
    "display_name": "我的方案",
    "files": ["1.wav", "2.wav", "3.wav"],
    "non_unique_count": 3,
    "key_audio_map": {}
}
```

`name` 必须与音效目录一致；没有本地化项时使用 `display_name`。打包前同步 [音效来源](docs/audio-assets.md)、包内许可及生成的清单。从 Application Support 安全导入仍属于 [Roadmap](ROADMAP.md) 的后续功能。

## 技术栈

| 组件 | 库 | 用途 |
|---|---|---|
| 音频 | rodio 0.20 | WAV 解码 + CoreAudio 播放 |
| UI | objc2 0.6 | NSStatusBar, NSMenu, NSAlert |
| 键盘 | CGEventTap (FFI) | 全局按键监听 |
| 并发 | crossbeam-channel 0.5 | 有界击键队列与设置唤醒通道 |
| 配置 | serde + serde_json | 方案定义解析 |
| 偏好 | NSUserDefaults | 持久化方案/音量/音调 |

## 权限说明

Tickeys Redux 使用 `CGEventTapCreate` 接收全局按键码，并据此选择本地音效，需要 macOS「输入监控」权限。应用不会把事件写入磁盘或传输到外部。缺少权限时，每次进程启动最多请求一次系统授权。偏好设置保持可操作，显示状态并提供「测试声音」和系统设置入口；重新激活、再次打开偏好设置及唤醒会触发重检，不需要辅助功能权限。

在 Apple Silicon 上，`cargo build` 生成由链接器 ad-hoc 签名的 Mach-O，而不是项目的长期发布身份。`scripts/package_app.sh` 默认对完整本地 App Bundle 进行 ad-hoc 签名，因此重新打包后 macOS 可能要求再次授予「输入监控」权限。从 v1.0.7 起，正式发布版使用 [docs/signing.md](docs/signing.md) 记录的长期自签名身份。从旧的 ad-hoc 版本升级时需要修复一次权限；后续使用同一身份签名的版本可以继续沿用。

开机自启使用 Apple 的 `SMAppService`，因此需要 macOS 13 或更高版本。应用会显示 macOS 返回的真实注册错误，不再静默假装设置成功。

## 项目元数据

建议设置的 GitHub topics 写在 [docs/github-metadata.md](docs/github-metadata.md)。

## 许可证与音效资源

Redux 源代码采用 MIT 许可证。原版作者 [应元东](https://github.com/yingDev)，Redux 移植 [Sinclair](https://github.com/E-R-Butch)。21 个保留录音采用 CC BY 3.0 或 CC0，32 个新合成 WAV 采用项目 MIT。完整的 [音效来源](docs/audio-assets.md)、[依赖声明](THIRD_PARTY_NOTICES.md) 和许可文本随 App 提供离线访问。依赖清单覆盖目标平台的 46 个普通/构建依赖、103 个声明和许可文件；Rust 运行时许可来自实际构建工具链。
