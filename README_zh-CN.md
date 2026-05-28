# Tickeys Redux

> macOS 机械键盘音效模拟。每次敲击，即时音频反馈。

Fork 自 [Tickeys](https://github.com/yingDev/Tickeys)（应元东） — 移植至 arm64 macOS，使用现代 Rust，零遗留依赖。

[English](README.md) | 中文

## v1.0.0 更新

| | 原版 | Redux |
|---|---|---|
| **架构** | x86_64 | **arm64 原生** |
| **音频引擎** | OpenAL + libalut (.dylib) | **rodio**（纯 Rust → CoreAudio） |
| **UI 框架** | cocoa 0.2 + XIB | **objc2 0.6** + NSStatusBar |
| **Rust edition** | 2015 | **2021** |
| **设置界面** | 未完成的 XIB 窗口 | 🎹 **菜单栏** — 方案/音量/音调 |
| **权限** | 无 | **输入监控**（系统原生弹窗） |
| **更新检测** | 内置 | **已移除** |
| **macOS 支持** | 10.10+ | **14+**（已在 26 上测试） |

## 安装

从 [Releases](https://github.com/E-R-Butch/TickeysRedux/releases) 下载 DMG，或从源码构建：

```sh
git clone https://github.com/E-R-Butch/TickeysRedux.git
cd TickeysRedux
cargo build --release
```

需要 Rust 1.77+。二进制文件位于 `target/release/tickeys-redux`。

## 使用

1. 启动 `Tickeys Redux.app`
2. 系统弹出权限提示时，授予「输入监控」权限
3. 点击菜单栏 🎹 图标：
   - 切换音效方案（泡泡、樱桃 G80-3000/3494、架子鼓、机械键盘、刀剑、打字机……）
   - 调整音量（25%/50%/75%/100%）
   - 调整音调（0.5×–2.0×）
4. 开始打字 — 即时键音

## 打包 App Bundle

```sh
cargo build --release

# 创建 .app 结构
mkdir -p "Tickeys Redux.app/Contents/MacOS"
mkdir -p "Tickeys Redux.app/Contents/Resources"
cp target/release/tickeys-redux "Tickeys Redux.app/Contents/MacOS/"
cp -R Tickeys.app/Contents/Resources/data "Tickeys Redux.app/Contents/Resources/"
cp Tickeys.app/Contents/Resources/tickeys.icns "Tickeys Redux.app/Contents/Resources/"

# 写入 Info.plist（LSUIElement = true，仅菜单栏，无 Dock 图标）
cat > "Tickeys Redux.app/Contents/Info.plist" << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>tickeys-redux</string>
    <key>CFBundleIdentifier</key>
    <string>com.tickeys.redux</string>
    <key>CFBundleName</key>
    <string>Tickeys Redux</string>
    <key>CFBundleVersion</key>
    <string>1.0.0</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>LSUIElement</key>
    <true/>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
EOF
```

## 自定义音效方案

在 `Resources/data/` 下添加自己的 `.wav` 文件，并编辑 `schemes.json`：

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

Tickeys Redux 使用 `CGEventTapCreate` 监听全局按键事件，需要 macOS「输入监控」权限。首次启动时系统会自动弹出权限请求，无需手动操作。

注意：每次 `cargo build` 会改变二进制的 ad-hoc 签名哈希，重建后需重新授权。使用正式的 Developer ID 签名可消除此问题。

## 许可证

MIT — 原版作者 [应元东](https://github.com/yingDev)，Redux 移植 [Sinclair](https://github.com/E-R-Butch)。
