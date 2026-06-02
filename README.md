# Tickeys Redux

> Mechanical keyboard sound effects for macOS. Instant audio feedback for every keystroke.

[English](README.md) | [中文](README_zh-CN.md)

Fork of [Tickeys](https://github.com/yingDev/Tickeys) by 应元东 — ported to arm64 macOS with modern Rust, zero legacy dependencies.

> **⚠️ This project targets Apple Silicon (arm64) only. It does not run on Intel Macs. Intel users should use the [original Tickeys](https://github.com/yingDev/Tickeys) instead.**

## What's New in v1.0.5

| | Original | Redux |
|---|---|---|
| **Architecture** | x86_64 | **arm64 native** |
| **Audio engine** | OpenAL + libalut (.dylib) | **rodio** (pure Rust → CoreAudio) |
| **UI framework** | cocoa 0.2 + XIB | **objc2 0.6** + NSStatusBar |
| **Rust edition** | 2015 | **2021** |
| **Settings** | Unfinished XIB window | 🎹 **Menu bar** — scheme/volume/pitch |
| **Permissions** | None | **Input Monitoring** (native macOS prompt) |
| **Update checker** | Built-in | **Removed** |
| **macOS target** | 10.10+ | **11+** (arm64 baseline) |

## Install

Download from [Releases](https://github.com/E-R-Butch/TickeysRedux/releases), or build from source:

```sh
git clone https://github.com/E-R-Butch/TickeysRedux.git
cd TickeysRedux
cargo build --release
```

Requires Rust 1.77+. Binary at `target/release/tickeys-redux`.

## Usage

1. Launch `Tickeys Redux.app`
2. Grant **Input Monitoring** permission when the system prompt appears
3. Click 🎹 in the menu bar to:
   - Switch sound schemes (bubble, Cherry G80-3000/3494, drum, mechanical, sword, typewriter...)
   - Adjust volume (25%/50%/75%/100%)
   - Adjust pitch (0.5x–2.0x)
4. Start typing — instant key sounds

## Building the App Bundle

```sh
./scripts/package_app.sh
```

This script handles:
- `cargo build --release`
- Creates `Tickeys Redux.app` bundle with all resources
- Writes `Info.plist` (version read from `Cargo.toml`)
- Ad-hoc codesigns and verifies the bundle

### Manual build

```sh
cargo build --release
mkdir -p "Tickeys Redux.app/Contents/MacOS"
mkdir -p "Tickeys Redux.app/Contents/Resources"
cp target/release/tickeys-redux "Tickeys Redux.app/Contents/MacOS/"
cp -R assets/data "Tickeys Redux.app/Contents/Resources/"
cp assets/tickeys_redux.icns "Tickeys Redux.app/Contents/Resources/tickeys.icns"
# Write Info.plist, then:
codesign --force --deep --sign - "Tickeys Redux.app"
codesign --verify --deep --strict "Tickeys Redux.app"
```

### Verification

After building, verify the bundle is healthy:

```sh
cargo build --release
./scripts/package_app.sh
codesign --verify --deep --strict --verbose=2 "Tickeys Redux.app"
plutil -lint "Tickeys Redux.app/Contents/Info.plist"
```

## Custom Sound Schemes

Add your own `.wav` files under `Resources/data/` and edit `schemes.json`:

```json
{
    "name": "myScheme",
    "display_name": "My Scheme",
    "files": ["1.wav", "2.wav", "3.wav"],
    "non_unique_count": 3,
    "key_audio_map": {}
}
```

## Tech Stack

| Component | Library | Purpose |
|---|---|---|
| Audio | rodio 0.20 | WAV decode + playback via CoreAudio |
| UI | objc2 0.6 | NSStatusBar, NSMenu, NSAlert |
| Keyboard | CGEventTap (FFI) | Global key-down monitoring |
| Concurrency | crossbeam 0.8 | Audio worker thread channel |
| Config | serde + serde_json | Scheme definition parsing |
| Prefs | NSUserDefaults | Persist scheme/volume/pitch |

## Permissions

Tickeys Redux uses `CGEventTapCreate` to listen for global key-down events. This requires **Input Monitoring** permission on macOS. The system prompt appears automatically on first launch. No Accessibility permission needed.

Note: each `cargo build` changes the binary's ad-hoc code signature hash. Re-grant Input Monitoring permission after rebuilding. A proper Developer ID signature eliminates this.

## License

MIT — original work by [应元东](https://github.com/yingDev), Redux port by [Sinclair](https://github.com/E-R-Butch).
