# Release signing

Tickeys Redux does not use a paid Apple Developer ID and is not notarized. Starting with v1.0.7, official release artifacts use one persistent, locally generated self-signed code-signing certificate so macOS can recognize later builds as the same application for Input Monitoring permission.

## Public identity

- Common Name: `Tickeys Redux Local Release Signing`
- SHA-256: `152B9E7DCA7778BB36909EC5DB85CEF96430CB26135333084F526CC13A644839`
- SHA-1 used by the designated requirement: `C35F68EA9702560D619BDA339ED7E735511F6A85`
- Bundle identifier: `com.sinclair.tickeys-redux`

The private key is stored only in the maintainer's login keychain and is never committed to this repository. Losing or replacing that key changes the application's designated requirement and requires users to grant Input Monitoring again.

The self-signed identity provides continuity and prevents an ad-hoc application that merely copies the bundle identifier from inheriting permission. It does not provide Apple trust, notarization, or a smoother Gatekeeper prompt. Users must create a local Gatekeeper exception, and administrator-managed Macs may not permit that override.

## Build behavior

- `scripts/package_app.sh` creates an ad-hoc local build unless `TICKEYS_SIGNING_IDENTITY` is supplied.
- `scripts/package_release.sh` pins the official certificate SHA-1 and fails if the resulting designated requirement is not bound to that certificate and the expected bundle identifier.
- Self-signed builds explicitly disable Apple's timestamp service because it does not support this non-Apple identity.
