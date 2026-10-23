# Tickeys Redux 1.0.8 — source completion notes

Status: **source version 1.0.8; official signing and publication not yet performed.** The persistent release certificate/private key remains on the maintainer's original signing Mac. Local ad-hoc builds do not replace that identity. Test execution and acceptance status are recorded in [development-audio-settings.md](development-audio-settings.md).

## Changes

- Input Monitoring now follows one recovery path at launch, activation, opening Preferences, wake and event-tap invalidation. A tap that cannot be re-enabled is dropped and rebuilt on the main thread after its callback returns. The system permission request is attempted at most once per process launch; recovery guidance remains in interactive Preferences rather than a blocking app modal.
- Menu and Preferences share state. Fractional volume, the `0.25x–2.0x` pitch range, monitor/audio status and permission-independent Test Sound are consistent. Ambiguous old low-volume preferences are preserved as percentages and marked with a format version.
- Audio settings use a separate latest-value mailbox, schemes decode on the worker, overlapping sounds share PCM, and stale key events are dropped. This reduces queue pressure and avoids replaying old keystrokes after a delay or scheme change.
- About is localized in English and Simplified Chinese. Its five links cover the original project, Redux, Releases, LICENSE and third-party notices. The last two prefer bundled offline files. Opening a web link is user initiated; there are still no background network requests.

## Sound-pack changes

**32 WAVs in five packs have changed sound.** Their recording-specific redistribution terms could not be established, so they were replaced entirely with original procedural synthesis. They contain no old recordings, third-party samples, film audio or recordings of real Cherry keyboards. The generator and new WAVs use the project MIT license.

| Existing scheme ID | New displayed scheme |
|---|---|
| `typewriter` | Synth Typewriter / 合成打字机 |
| `sword` | Synth Blades / 合成刀剑 |
| `Cherry_G80_3000` | Synth Click / 合成脆响 |
| `Cherry_G80_3494` | Synth Thock / 合成闷响 |
| `starwars` | Synth Space / 合成太空 |

Saved scheme selections and key mappings remain compatible. Bubble, Mechanical and Drum retain their 21 existing recordings under CC BY 3.0 or CC0. Attribution, sources, modifications and all 53 WAV hashes are documented in [audio-assets.md](audio-assets.md) and bundled with the App. These terms do not relicense audio in old releases or Git history.

## Build and release integrity

`scripts/package_app.sh` creates a fresh App under `dist/builds/v<version>-<commit>-<profile>-<mode>-<random>/`. It supports `--debug` and `--dmg [path]`; previous builds and existing DMGs are preserved. Local builds are ad-hoc signed by default.

`scripts/package_release.sh` requires a clean source tree, an annotated `v1.0.8` tag at HEAD and the original signing identity. The certificate's SHA-1 and SHA-256 are fixed in release policy. Packaging verifies complete resources, source commit, language metadata, arm64/macOS 13 requirements and the signatures inside both ZIP and DMG. It creates checksums and build metadata and refuses to overwrite existing release artifacts.

The App carries project and audio licenses, full notices for 46 target-resolved normal/build dependencies (103 license/notice files), and runtime license texts from the Rust toolchain actually used. Its development region is English, with Base, English and Simplified Chinese resources. CI covers stable Rust and the Rust 1.85 minimum; hosted execution is tracked separately from configuration.

## Acceptance still required

- Real macOS Input Monitoring grant, denial/regrant and revocation; global-key capture and post-sleep recovery.
- Menu and already-open Preferences interaction in both directions, including persistence after relaunch.
- Official signing, distribution archive verification and publication on the original signing Mac.

Policy regressions cover lifecycle decisions, including six input-recovery tests, but cannot prove actual macOS permission prompts or device behavior. Startup audio failure and synchronous playback-error recovery are implemented; **asynchronous CoreAudio hot-plug/route recovery is not a new verified feature of 1.0.8**. Runtime sound-pack import, app filtering and the other v1.1/v1.2 features remain on the [Roadmap](../ROADMAP.md).

## Installation policy

Apple Silicon and macOS 13+ are required. Official releases continue using the same free self-signed identity, without paid Developer ID or Apple notarization. Gatekeeper may require **System Settings → Privacy & Security → Open Anyway** after an initial launch attempt. Input Monitoring is the only input permission; microphone and Accessibility permissions are not required. The original Tickeys macOS 1.1.0 remains the functional comparison baseline.

## 中文说明

1.0.8 的实现和文档已经进入源码，正式签名与公开发布尚未执行。输入恢复、设置同步、音频队列、关于页面、离线许可证和发布校验均已实现；真实键盘权限和睡眠唤醒仍须实机验收。五套音效的 32 个 WAV 已改为原创合成，另三套 21 个有许可录音保留；旧方案 ID 与映射继续兼容。原签名证书仍由维护者的原签名 Mac 保管，不建立新身份，不购买 Developer ID，也不新增后台联网。
