# Releasing v1.0.8

The source and local test builds are version 1.0.8. An ad-hoc local build is not an official release. Perform the following on the original signing Mac with the certificate in [signing.md](signing.md), after reviewing/merging the version's changes and checking CI on the intended commit.

## Prerequisites and acceptance

- Apple Silicon, macOS 13+, Xcode Command Line Tools, Rust 1.85+ with rustfmt/Clippy, Python 3.9+, and `create-dmg` (`brew install create-dmg`). The local packaging path is also tested with Rust 1.85's older copyright-file layout.
- Verify the expected private-key-backed identity appears in `security find-identity -p codesigning`. Keep its private key in the keychain; do not export it for CI.
- Exercise menu → open Preferences and Preferences → menu for scheme/volume/pitch, including 0.25% volume and 0.25x pitch. Test hidden-icon reopen.
- On the release candidate, exercise permission denial, granting/revoking Input Monitoring, return from System Settings, and sleep/wake. UI status and mock recovery tests are not substitutes for these OS-controlled transitions.
- Test Sound and ordinary key sounds should be heard on the intended device. Real asynchronous output-device disconnect/route changes remain a known limitation; initialization and synchronous submission recovery must not be described as complete hot-plug support.
- Review the audible changes: 32 WAVs in five packs are now original synthetic sounds. The 21 credited recordings are retained. Stable IDs and key mappings are preserved.

## Build on the signing Mac

Check out the reviewed release commit, then create its annotated version tag if it does not already exist. Do not move an existing published tag.

```sh
git status --short
git rev-parse HEAD
git tag -a v1.0.8 -m "Tickeys Redux v1.0.8"
cargo fetch --locked
./scripts/package_release.sh --check
./scripts/package_release.sh
```

For an existing tag, use `git switch --detach v1.0.8` instead of creating it again. `--check` checks the clean worktree, exact annotated tag/commit and original signing identity. The build command also runs formatting, tests, Clippy, release-policy tests, asset checks and license checks. It verifies the actual certificate's SHA-1/SHA-256 and designated requirement inside the app, ZIP and DMG. Failure does not remove earlier artifacts.

Outputs in `dist/`:

- `Tickeys.Redux.v1.0.8.zip`
- `Tickeys.Redux.v1.0.8.dmg`
- `Tickeys.Redux.v1.0.8.build.json` (source commit/tag, build toolchain, signing fingerprints)
- `Tickeys.Redux.v1.0.8.sha256.txt` (checksums of the three files above)

```sh
cd dist
shasum -a 256 -c Tickeys.Redux.v1.0.8.sha256.txt
```

Publication stages each complete file on the destination filesystem and claims its name atomically. A file, directory, or symlink created during packaging is never overwritten. The checksum file is written last. If publication fails partway through, already-published files are retained and the command reports failure; inspect them rather than deleting outputs or retrying blindly. The destination filesystem must support hard links, otherwise publication fails without an overwrite fallback.

## Publish only the verified artifacts

After final acceptance, push the annotated tag and publish the four verified artifacts with [the v1.0.8 release notes](release-v1.0.8.md). No script in this repository publishes automatically or sends private signing material anywhere. Keep existing releases and tags intact.

Update `Casks/tickeys-redux.rb` only after the official DMG exists: use version `1.0.8` and its actual SHA-256 from the verified checksum file. Until that step, the cask intentionally continues to reference the last published version. Do not insert the hash of an ad-hoc test package.

The official signing and publication step has not been run in the development environment that prepared this change; its original signing identity was unavailable there. The source changes and local ad-hoc build can be reviewed independently.
