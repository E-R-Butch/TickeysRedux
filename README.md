# Tickeys Redux

Native Apple Silicon menu bar app that brings mechanical keyboard sounds back to modern macOS.

[English](README.md) | [中文](README_zh-CN.md)

![Tickeys Redux preview](docs/hero.svg)

**Free, open source, local-only. No microphone. No telemetry. No cloud.**

[Download from Releases](https://github.com/E-R-Butch/TickeysRedux/releases) · [Build from source](#building-the-app-bundle) · [中文说明](README_zh-CN.md)

Tickeys Redux is a modern macOS port of [Tickeys](https://github.com/yingDev/Tickeys) by Ying Yuandong. It keeps the playful typing feedback of the original app, but rebuilds the runtime for Apple Silicon with Rust, objc2, rodio, CoreAudio, and a clean menu bar interface.

> **Apple Silicon only.** This project targets arm64 Macs. Intel Mac users should use the [original Tickeys](https://github.com/yingDev/Tickeys).

## Why Try It

- **Instant typing feedback** with bundled mechanical, typewriter, sword, drum, bubble, and Cherry G80 sound packs.
- **Native menu bar workflow**: switch schemes, adjust volume, and tune pitch without opening a full app window.
- **Privacy-respecting by design**: uses macOS Input Monitoring to detect key events, not a microphone.
- **No background web services**: no telemetry, cloud sync, analytics, or update beacon.
- **Modern native stack**: Rust 2024, objc2, rodio, CoreAudio, and an auditable app bundle script.
- **Classic Tickeys spirit**: preserves the original fun while removing legacy dylib dependencies.

## Demo

The app lives quietly in the menu bar. Pick a sound scheme, set volume and pitch, then type. Each key-down event plays a short local WAV sample immediately.

```text
Menu bar -> Sound Scheme -> Mechanical / Typewriter / Sword / Drum / Bubble / Cherry
Menu bar -> Volume       -> 25% / 50% / 75% / 100%
Menu bar -> Pitch        -> 0.5x / 1.0x / 1.5x / 2.0x
```

## Install

Download `Tickeys.Redux.v1.0.5.dmg` from [Releases](https://github.com/E-R-Butch/TickeysRedux/releases), open it, and copy `Tickeys Redux.app` to Applications.

On first launch, grant **Input Monitoring** permission when macOS asks. Tickeys Redux needs this permission to know that a key was pressed; it does not record text or use the microphone.

## What's New in v1.0.5

| | Original | Redux |
|---|---|---|
| **Architecture** | x86_64 | **arm64 native** |
| **Audio engine** | OpenAL + libalut (.dylib) | **rodio** (pure Rust → CoreAudio) |
| **UI framework** | cocoa 0.2 + XIB | **objc2 0.6** + NSStatusBar |
| **Rust edition** | 2015 | **2024** |
| **Settings** | Unfinished XIB window | 🎹 **Menu bar** — scheme/volume/pitch |
| **Permissions** | None | **Input Monitoring** (native macOS prompt) |
| **Update checker** | Built-in | **Removed** |
| **macOS target** | 10.10+ | **11+** (arm64 baseline) |

## Usage

1. Launch `Tickeys Redux.app`
2. Grant **Input Monitoring** permission when the system prompt appears
3. Click 🎹 in the menu bar to:
   - Switch sound schemes (bubble, Cherry G80-3000/3494, drum, mechanical, sword, typewriter...)
   - Adjust volume (25%/50%/75%/100%)
   - Adjust pitch (0.5x–2.0x)
4. Start typing — instant key sounds

## Building the App Bundle

Requires Rust 1.77+.

```sh
git clone https://github.com/E-R-Butch/TickeysRedux.git
cd TickeysRedux
./scripts/package_app.sh
```

This script handles:
- `cargo build --release`
- Creates `Tickeys Redux.app` bundle with all resources
- Writes `Info.plist` (version read from `Cargo.toml`)
- Ad-hoc codesigns and verifies the bundle

### Manual Build

```sh
cargo build --release
mkdir -p "Tickeys Redux.app/Contents/MacOS"
mkdir -p "Tickeys Redux.app/Contents/Resources"
cp target/release/tickeys-redux "Tickeys Redux.app/Contents/MacOS/"
rsync -a --exclude='*.bak' --exclude='*.wav.bak' assets/data/ "Tickeys Redux.app/Contents/Resources/data/"
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

Add your own `.wav` files under `assets/data/` and edit `assets/data/schemes.json`:

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

## Project Metadata

Suggested GitHub topics are listed in [docs/github-metadata.md](docs/github-metadata.md).

## License

MIT — original work by [应元东](https://github.com/yingDev), Redux port by [Sinclair](https://github.com/E-R-Butch).
