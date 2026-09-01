# Tickeys Redux

Native Apple Silicon menu bar app that brings mechanical keyboard sounds back to modern macOS.

[English](README.md) | [中文](README_zh-CN.md)

![Tickeys Redux preview](docs/hero.svg)

**Free, open source, local-only. No microphone. No telemetry. No cloud.**

[Download from Releases](https://github.com/E-R-Butch/TickeysRedux/releases) · [Build from source](#building-the-app-bundle) · [中文说明](README_zh-CN.md)

Tickeys Redux is a modern macOS port of [Tickeys](https://github.com/yingDev/Tickeys) by Ying Yuandong. It keeps the playful typing feedback of the original app, but rebuilds the runtime for Apple Silicon with Rust, objc2, rodio, CoreAudio, a menu-bar controller, and a native Preferences window.

> **Apple Silicon only.** This project targets arm64 Macs. Intel Mac users should use the [original Tickeys](https://github.com/yingDev/Tickeys).

## Why Try It

- **Instant typing feedback** with eight bundled schemes: Bubble, Cherry G80-3000, Cherry G80-3494, Drum, Mechanical, Star Wars, Sword, and Typewriter.
- **Native controls**: switch schemes, adjust volume and pitch from the menu bar, or use the three-tab Preferences window.
- **Privacy-respecting by design**: uses macOS Input Monitoring to detect key events, not a microphone.
- **No background web services**: no telemetry, cloud sync, analytics, or update beacon.
- **Modern native stack**: Rust 2024, objc2, rodio, CoreAudio, and an auditable app bundle script.
- **Classic Tickeys spirit**: preserves the original fun while removing legacy dylib dependencies.

## Demo

The app lives quietly in the menu bar. Pick a sound scheme, set volume and pitch, then type. Each key-down event plays a short local WAV sample immediately. Preferences also contains Sound, General, and About tabs.

```text
Menu bar -> Sound Scheme -> Bubble / Cherry G80-3000 / Cherry G80-3494 / Drum
                           Mechanical / Star Wars / Sword / Typewriter
Menu bar -> Volume       -> 25% / 50% / 75% / 100%
Menu bar -> Pitch        -> 0.5x / 0.75x / 1.0x / 1.5x / 2.0x
```

## Install

Download the latest `.dmg` from [Releases](https://github.com/E-R-Butch/TickeysRedux/releases), open it, and copy `Tickeys Redux.app` to Applications.

Official releases use a persistent, free self-signed identity—not a paid Apple Developer ID—and are not notarized. If macOS blocks the first launch, try opening the app once, then go to **System Settings → Privacy & Security**, scroll to Security, click **Open Anyway**, and confirm **Open**. Administrator-managed Macs may not allow this override.

On first launch, grant **Input Monitoring** permission when macOS asks. Tickeys Redux uses key codes to choose local sound samples; it does not log or persist typed text, reconstruct text content, or transmit input events, and it does not use the microphone. Open the app again after granting access; if key sounds are still silent, quit Tickeys Redux completely and relaunch it.

## What's New in v1.0.7

- Start at Login now uses the complete `SMAppService` state machine, reports registration errors, and opens Login Items settings when macOS approval is required.
- Fixed menu-bar volume persistence that could make the app 100× quieter after relaunch.
- The audio worker retries if CoreAudio is unavailable during startup and rebuilds its output when a playback call reports a device error.
- Keyboard monitoring attempts to re-enable the event tap after macOS disables it and after sleep, without relaunching the process.
- A visible recovery prompt now explains how to repair stale Input Monitoring permission after an upgrade and opens the correct System Settings page.
- Reopening the app shows Preferences, even when the menu-bar icon is hidden.
- Fixed a deterministic menu-object leak and hardened release packaging/verification.

### Known limitations in v1.0.7

- Returning from System Settings does not always trigger a fresh Input Monitoring check, and a failed event-tap re-enable does not yet rebuild the monitor. Reopen the app or quit and relaunch it if typing remains silent.
- Menu-bar controls and an already-created Preferences window do not yet refresh each other in every direction.
- The legacy volume migration can mistake exact `0.25%`, `0.5%`, `0.75%`, or `1%` settings for old fractional values; the pitch slider also exposes values below the engine's actual `0.25x` minimum.
- Recovery is tested for startup failure and synchronous playback errors; some asynchronous CoreAudio device or route changes may still require a relaunch.
- Runtime sound-pack import is not implemented. Custom packs currently require a source build.

## Position Against Original Tickeys

The comparison baseline is the original author's final published macOS release, [Tickeys 1.1.0](https://www.yingdev.com/projects/tickeys), not only the older source snapshot visible on GitHub. Redux modernizes the runtime and permission model, but v1.0.7 has not yet restored every original user-facing feature.

| Capability | Original macOS 1.1.0 | Redux 1.0.7 |
|---|---|---|
| **Platform/runtime** | Intel-era release | **Apple Silicon native** |
| **Permissions** | Accessibility | **Input Monitoring only** |
| **Settings** | Preferences UI and status-bar controls | **Three-tab Preferences** and menu-bar controls |
| **Core controls** | Scheme, volume, pitch, quick enable/disable | Scheme, volume, manual pitch; quick toggle pending |
| **App filtering** | Current-app exclusion and black/white lists | Pending |
| **Key-event options** | Optional modifier and synthetic-key sounds | Pending |
| **Automatic pitch** | Typing-speed-based pitch | Pending |
| **Hidden-icon recovery** | Global modifier-key sequence | Reopen the app; global gesture pending |
| **Custom schemes** | [Manual app-resource editing](https://github.com/yingDev/Tickeys#add-custom-schemes), documented in the public source release | Developer-only source customization; safe runtime import pending |
| **Update checking** | Built in | No background update beacon by design; user-initiated check planned |
| **macOS support** | 10.15+; 10.14 allowed but untested | 13+ |

The parity and beyond-original work is tracked in [ROADMAP.md](ROADMAP.md).

The original public 0.5 source uses OpenAL + libalut; Redux replaces that stack with rodio + CoreAudio. The original 1.1.0 release notes do not publish its internal audio-engine version.

## Usage

1. Launch `Tickeys Redux.app`
2. Grant **Input Monitoring** permission when the system prompt appears
3. Click 🎹 in the menu bar, or open **Preferences**, to:
   - Switch among all eight sound schemes
   - Adjust volume (25%/50%/75%/100%)
   - Adjust pitch (0.5x–2.0x)
4. Start typing — instant key sounds

## Building the App Bundle

Requires Rust 1.85+.

```sh
git clone https://github.com/E-R-Butch/TickeysRedux.git
cd TickeysRedux
./scripts/package_app.sh
```

This script handles:

- `cargo build --release --locked`
- Creates `Tickeys Redux.app` with all required runtime resources
- Writes `Info.plist` (version read from `Cargo.toml`)
- Ad-hoc signs and verifies the complete app bundle by default.

The packaging script is the canonical description of the App Bundle layout, resources, deployment target, and signing steps. On Apple Silicon, `cargo build --release --locked` creates a linker-signed ad-hoc Mach-O at `target/release/tickeys-redux`; it does not create an installable app or use the project's persistent release identity.

### Verification

After building, verify the bundle is healthy:

```sh
./scripts/package_app.sh
codesign --verify --deep --strict --verbose=2 "Tickeys Redux.app"
plutil -lint "Tickeys Redux.app/Contents/Info.plist"
```

## Developer-only Custom Sound Schemes in v1.0.7

v1.0.7 does not provide a safe end-user import flow. Do not edit an installed release bundle: doing so invalidates its code signature and may break its Input Monitoring identity. To bundle a custom scheme in a source build, add `.wav` files under `assets/data/` and edit `assets/data/schemes.json`:

```json
{
    "name": "myScheme",
    "display_name": "My Scheme",
    "files": ["1.wav", "2.wav", "3.wav"],
    "non_unique_count": 3,
    "key_audio_map": {}
}
```

`name` must match the sound directory and is the current UI fallback when no localization exists. `display_name` is required by the v1.0.7 data format but is not yet displayed by the UI. Runtime import from Application Support is planned in [ROADMAP.md](ROADMAP.md).

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

Tickeys Redux uses `CGEventTapCreate` to receive global key-down key codes and select local sound samples. This requires **Input Monitoring** permission on macOS. The app does not log events to disk or transmit them. The system prompt appears automatically on first launch. After enabling it, open Tickeys Redux again; if key sounds are still silent, quit the app completely and relaunch it. No Accessibility permission is needed.

On Apple Silicon, `cargo build` produces a linker-signed ad-hoc Mach-O, not the project's persistent release identity. `scripts/package_app.sh` ad-hoc signs the complete local app bundle by default, so rebuilding it may require Input Monitoring permission again. Official releases from v1.0.7 onward use the persistent self-signed identity documented in [docs/signing.md](docs/signing.md). Upgrading from an older ad-hoc release requires one permission repair; later releases signed with the same identity can retain it.

Start at Login uses Apple's `SMAppService` API and therefore requires macOS 13 or later. v1.0.7 surfaces any registration error from macOS instead of silently showing a checked box.

## Project Metadata

Suggested GitHub topics are listed in [docs/github-metadata.md](docs/github-metadata.md).

## License and Sound Assets

The Redux source code is MIT licensed. Original work by [应元东](https://github.com/yingDev); Redux port by [Sinclair](https://github.com/E-R-Butch). Bundled sound packs may have separate terms and are tracked in [docs/audio-assets.md](docs/audio-assets.md).
