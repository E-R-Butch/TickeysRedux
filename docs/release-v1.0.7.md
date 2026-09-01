# Tickeys Redux v1.0.7

This maintenance release repairs Start at Login and several reliability problems left by the initial Rust rewrite.

## Fixes

- ServiceManagement is now actually loaded by the app; Start at Login handles all `SMAppService` states, propagates the real macOS error, rolls the checkbox back after failure, and guides users to Login Items when approval is required.
- Menu-bar volume values are stored consistently. Existing 1.0.6 preferences that used `0.25`–`1.0` are migrated automatically to `25`–`100`.
- The audio worker now retries when CoreAudio is not ready during login startup and rebuilds its output after a device error, instead of leaving the running app permanently silent.
- Keyboard monitoring is re-enabled if macOS disables the event tap or after the Mac wakes from sleep.
- Input Monitoring failures now show a visible localized recovery prompt with a direct link to the correct System Settings page, including guidance for stale permission entries left by an upgrade.
- Sleep notifications are acknowledged correctly; the app no longer relaunches itself on wake.
- Reopening the app brings Preferences forward, so hiding the menu-bar icon no longer removes the only UI entry point.
- Rebuilding the menu no longer leaks retained submenu objects.
- About, documentation, deployment target, Cargo metadata, Cask metadata, and release packaging now use one version line.

## Requirements

- Apple Silicon Mac
- macOS 13 or later
- Input Monitoring permission

## Install

Download `Tickeys.Redux.v1.0.7.dmg`, open it, and drag `Tickeys Redux.app` to Applications. Launch it once and grant Input Monitoring permission. Open the app again after granting access; if key sounds are still silent, quit Tickeys Redux completely and relaunch it.

The public build uses the project's persistent, free self-signed release identity. It does not use a paid Apple Developer ID and is not notarized, so macOS may still require Control-clicking the app and choosing Open. Users upgrading from an older ad-hoc build must repair Input Monitoring once; later releases signed with the same identity can retain that application identity. Start at Login registration errors are now shown directly instead of being silently ignored.

## Verify Downloads

Place the DMG, ZIP, and checksum file in one directory, then run:

```sh
shasum -a 256 -c Tickeys.Redux.v1.0.7.sha256.txt
```
