# Release signing

Tickeys Redux does not use a paid Apple Developer ID and is not notarized. Starting with v1.0.7, official release artifacts use one persistent, locally generated self-signed code-signing certificate so macOS can recognize later builds as the same application for Input Monitoring permission.

## Public identity

- Common Name: `Tickeys Redux Local Release Signing`
- SHA-256: `152B9E7DCA7778BB36909EC5DB85CEF96430CB26135333084F526CC13A644839`
- SHA-1 used by the designated requirement: `C35F68EA9702560D619BDA339ED7E735511F6A85`
- Bundle identifier: `com.sinclair.tickeys-redux`

The release private key must never be committed to this repository. The working copy is kept in the maintainer's login keychain. Losing or replacing that key changes the application's designated requirement and requires users to grant Input Monitoring again, so an encrypted offline backup and a documented recovery check are recommended.

The self-signed identity provides continuity and prevents an ad-hoc application that merely copies the bundle identifier from inheriting permission. It does not provide Apple trust, notarization, or a smoother Gatekeeper prompt. Users must create a local Gatekeeper exception, and administrator-managed Macs may not permit that override.

## Build behavior

- On Apple Silicon, a bare `cargo build` produces a linker-signed ad-hoc Mach-O, not the project's persistent release identity or a complete App Bundle.
- `scripts/package_app.sh` creates an ad-hoc-signed local app unless `TICKEYS_SIGNING_IDENTITY` is supplied.
- `scripts/package_release.sh` and `scripts/package_app.sh --official` enforce the SHA-1 and SHA-256 constants in `scripts/release_policy.py`. Environment overrides cannot change them. The actual leaf certificate and designated requirement are checked after signing and after unpacking ZIP/mounting DMG.
- Official builds require a clean worktree (including untracked files) and an annotated `v<version>` tag pointing at the exact build commit. The version, commit, dirty flag, target, and toolchain versions are included in `Resources/build-info.json`; source commit/dirty state are also in Info.plist.
- Local builds are created in separate directories under `dist/builds/`; earlier apps are retained. Official ZIP/DMG/checksum/metadata files cannot be overwritten by rerunning the script.
- A local build rejects `TICKEYS_EXPECTED_SIGNING_CERT_SHA1`; use `--official` when asserting the release identity rather than relying on a local signature check.
- Self-signed builds explicitly disable Apple's timestamp service because it does not support this non-Apple identity.

## Original signing Mac

If this identity and its private key are absent, the official packaging command stops. Do not create a replacement identity or label an ad-hoc build as an official release. Run the release procedure on the maintainer's original signing Mac, or use an existing encrypted backup through the maintainer's normal keychain recovery process. Private keys and passwords must not be added to the repository or release artifacts.

See [the release procedure](releasing.md) for preflight, tagged builds, artifact verification and the remaining manual acceptance checks.

In v1.0.7 both identity environment values could be overridden together. v1.0.8 removes this bypass; changing the official identity now requires a reviewed source change to both fingerprints. This does not change the signing identity of already-published v1.0.7 artifacts.
