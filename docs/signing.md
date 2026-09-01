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
- `scripts/package_release.sh` defaults both the signing identity and expected certificate SHA-1 to the official value, then verifies the resulting designated requirement and bundle identifier.
- Self-signed builds explicitly disable Apple's timestamp service because it does not support this non-Apple identity.

## Audited release-process limitation

In v1.0.7, `scripts/package_release.sh` still permits the environment to override both `TICKEYS_SIGNING_IDENTITY` and `TICKEYS_EXPECTED_SIGNING_CERT_SHA1`. If both are changed together, the checks only prove that those two override values agree; they do not enforce the certificate recorded above.

Until the script is hardened, official release builds must start with both variables unset and the final certificate fingerprint and designated requirement must be checked against the public identity above. The next maintenance release should make the expected SHA-1 a non-overridable source constant; an intentional certificate rotation should require a reviewed source change.

The published v1.0.7 DMG and ZIP were verified against the public identity above. This limitation concerns future release procedure, not the identity of the existing v1.0.7 artifacts.
