# Tickeys Redux v1.0.5

Native Apple Silicon menu bar app that brings mechanical keyboard sounds back to modern macOS.

## Highlights

- Native arm64 macOS app bundle.
- Free, open source, and local-only.
- No microphone, telemetry, cloud service, or update beacon.
- Classic Tickeys sound packs bundled in the app.
- Menu bar controls for sound scheme, volume, and pitch.
- Input Monitoring permission prompt handled through macOS.
- Release bundle is ad-hoc signed and verified by `scripts/package_app.sh`.

## Download

Download `Tickeys.Redux.v1.0.5.dmg`, open it, and copy `Tickeys Redux.app` to Applications.

## First Launch

1. Launch `Tickeys Redux.app`.
2. Grant Input Monitoring permission when macOS asks.
3. Use the menu bar keyboard icon to choose a sound scheme and adjust volume or pitch.

## Verification

```sh
codesign --verify --deep --strict --verbose=2 "Tickeys Redux.app"
plutil -lint "Tickeys Redux.app/Contents/Info.plist"
```

## Notes

This release targets Apple Silicon Macs. Intel users should use the original Tickeys project.
