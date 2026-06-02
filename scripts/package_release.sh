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

echo "=== Packaging Tickeys Redux release v${VERSION} ==="

"${PROJECT_DIR}/scripts/package_app.sh"

mkdir -p "$DIST_DIR"

echo "[release] creating zip"
ditto -c -k --keepParent "$APP_BUNDLE" "$ZIP_PATH"

echo "[release] creating dmg"
hdiutil create \
  -volname "$APP_NAME" \
  -srcfolder "$APP_BUNDLE" \
  -ov \
  -format UDZO \
  "$DMG_PATH"

echo "[release] writing checksums"
shasum -a 256 "$ZIP_PATH" "$DMG_PATH" > "$SHA_PATH"

echo ""
echo "=== Release artifacts ==="
ls -lh "$ZIP_PATH" "$DMG_PATH" "$SHA_PATH"
echo ""
cat "$SHA_PATH"
