#!/usr/bin/env bash
set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT_DIR"

VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
APP_NAME="Tickeys Redux"
APP_BUNDLE="${APP_NAME}.app"
DIST_DIR="dist"
ZIP_PATH="${DIST_DIR}/Tickeys.Redux.v${VERSION}.zip"
DMG_PATH="${DIST_DIR}/Tickeys.Redux.v${VERSION}.dmg"
SHA_PATH="${DIST_DIR}/Tickeys.Redux.v${VERSION}.sha256.txt"
RELEASE_SIGNING_CERT_SHA1="C35F68EA9702560D619BDA339ED7E735511F6A85"

# Official releases use the project's persistent, free self-signed identity.
# Contributors can still use package_app.sh directly for an ad-hoc local build.
export TICKEYS_SIGNING_IDENTITY="${TICKEYS_SIGNING_IDENTITY:-$RELEASE_SIGNING_CERT_SHA1}"
export TICKEYS_EXPECTED_SIGNING_CERT_SHA1="${TICKEYS_EXPECTED_SIGNING_CERT_SHA1:-$RELEASE_SIGNING_CERT_SHA1}"

echo "=== Packaging Tickeys Redux release v${VERSION} ==="

mkdir -p "$DIST_DIR"
rm -f "$ZIP_PATH" "$DMG_PATH" "$SHA_PATH"

"${PROJECT_DIR}/scripts/package_app.sh" --dmg "$DMG_PATH"

echo "[release] creating zip"
ditto -c -k --sequesterRsrc --keepParent "$APP_BUNDLE" "$ZIP_PATH"
unzip -t "$ZIP_PATH"

echo "[release] verifying zip contents"
VERIFY_DIR="$(mktemp -d)"
trap 'rm -rf "$VERIFY_DIR"' EXIT
ditto -x -k "$ZIP_PATH" "$VERIFY_DIR"
codesign --verify --deep --strict --verbose=2 "$VERIFY_DIR/$APP_BUNDLE"
ZIP_BUNDLE_VERSION=$(plutil -extract CFBundleShortVersionString raw "$VERIFY_DIR/$APP_BUNDLE/Contents/Info.plist")
if [[ "$ZIP_BUNDLE_VERSION" != "$VERSION" ]]; then
  echo "❌  App version inside ZIP is ${ZIP_BUNDLE_VERSION}, expected ${VERSION}"
  exit 1
fi
rm -rf "$VERIFY_DIR"
trap - EXIT

echo "[release] writing checksums"
(
  cd "$DIST_DIR"
  shasum -a 256 "$(basename "$ZIP_PATH")" "$(basename "$DMG_PATH")" > "$(basename "$SHA_PATH")"
  shasum -a 256 -c "$(basename "$SHA_PATH")"
)

echo ""
echo "=== Release artifacts ==="
ls -lh "$ZIP_PATH" "$DMG_PATH" "$SHA_PATH"
echo ""
cat "$SHA_PATH"
