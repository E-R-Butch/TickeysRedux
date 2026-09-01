# Tickeys Redux v1.0.7

This maintenance release repairs Start at Login and several reliability problems left by the initial Rust rewrite.

## Fixes

- ServiceManagement is now actually loaded by the app; Start at Login handles all `SMAppService` states, propagates the real macOS error, rolls the checkbox back after failure, and guides users to Login Items when approval is required.
- Menu-bar volume values are stored consistently. Existing 1.0.6 preferences that used `0.25`–`1.0` are migrated automatically to `25`–`100`.
- The audio worker now retries when CoreAudio is unavailable during startup and rebuilds its output when a playback call reports a device error.
- Keyboard monitoring attempts to re-enable the event tap if macOS disables it and after the Mac wakes from sleep, without relaunching the process.
- Input Monitoring failures now show a visible localized recovery prompt with a direct link to the correct System Settings page, including guidance for stale permission entries left by an upgrade.
- Sleep notifications are acknowledged correctly; the app no longer relaunches itself on wake.
- Reopening the app brings Preferences forward, so hiding the menu-bar icon no longer removes the only UI entry point.
- Rebuilding the menu no longer leaks retained submenu objects.
- About, documentation, deployment target, Cargo metadata, Cask metadata, and release packaging now use one version line.

## Known Limitations

- Returning from System Settings does not always trigger a new Input Monitoring check. Reopen the app or quit and relaunch it if typing remains silent.
- Event-tap recovery attempts to re-enable the existing monitor; a failed re-enable is not yet followed by a full monitor rebuild.
- Audio recovery is covered for startup initialization failure and synchronous playback errors. Asynchronous CoreAudio device or route changes are not yet fully covered and may require a relaunch.
- Menu-bar controls and an already-created Preferences window may temporarily show different values.
- The legacy volume migration can mistake exact `0.25%`, `0.5%`, `0.75%`, or `1%` settings for old fractional values.

These limitations are tracked in [ROADMAP.md](../ROADMAP.md).

## Requirements

- Apple Silicon Mac
- macOS 13 or later
- Input Monitoring permission

## Install

Download `Tickeys.Redux.v1.0.7.dmg`, open it, and drag `Tickeys Redux.app` to Applications. Launch it once and grant Input Monitoring permission. Open the app again after granting access; if key sounds are still silent, quit Tickeys Redux completely and relaunch it.

The public build uses the project's persistent, free self-signed release identity. It does not use a paid Apple Developer ID and is not notarized. If macOS blocks the first launch, try opening the app once, then go to System Settings → Privacy & Security, scroll to Security, click Open Anyway, and confirm Open. Administrator-managed Macs may not allow this override. Users upgrading from an older ad-hoc build must repair Input Monitoring once; later releases signed with the same identity can retain that application identity. Start at Login registration errors are now shown directly instead of being silently ignored.

## Verify Downloads

Place the DMG, ZIP, and checksum file in one directory, then run:

```sh
shasum -a 256 -c Tickeys.Redux.v1.0.7.sha256.txt
```
