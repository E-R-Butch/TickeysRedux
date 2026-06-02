#!/usr/bin/env bash
set -euo pipefail

# ── Config ────────────────────────────────────────────────────────────────────
APP_NAME="Tickeys Redux"
BUNDLE_ID="com.sinclair.tickeys-redux"
VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
ICON_SRC="assets/tickeys_redux.icns"
DATA_SRC="assets/data"
LPROJ_BASE="assets/lproj/Base.lproj"
LPROJ_ZH="assets/lproj/zh-Hans.lproj"
BUNDLE_DIR="$APP_NAME.app"
CONTENTS="$BUNDLE_DIR/Contents"
MACOS="$CONTENTS/MacOS"
RESOURCES="$CONTENTS/Resources"

echo "=== Building Tickeys Redux v${VERSION} ==="

PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT_DIR"

# ── Step 1: Release build ─────────────────────────────────────────────────────
echo "[1/7] cargo build --release"
cargo build --release

# ── Step 2: Clean old bundle ──────────────────────────────────────────────────
echo "[2/7] cleaning old bundle"
rm -rf "$BUNDLE_DIR"

# ── Step 3: Create bundle structure ───────────────────────────────────────────
echo "[3/7] creating bundle structure"
mkdir -p "$MACOS"
mkdir -p "$RESOURCES"

# ── Step 4: Copy binary ───────────────────────────────────────────────────────
echo "[4/7] copying binary"
cp target/release/tickeys-redux "$MACOS/"

# ── Step 5: Copy resources ────────────────────────────────────────────────────
echo "[5/7] copying resources"
cp -R "$DATA_SRC" "$RESOURCES/data"
cp "$ICON_SRC" "$RESOURCES/tickeys.icns"
cp -R "$LPROJ_BASE" "$RESOURCES/Base.lproj"
cp -R "$LPROJ_ZH" "$RESOURCES/zh-Hans.lproj"

# ── Step 6: Write Info.plist ──────────────────────────────────────────────────
echo "[6/7] writing Info.plist"
cat > "$CONTENTS/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>zh_CN</string>
    <key>CFBundleExecutable</key>
    <string>tickeys-redux</string>
    <key>CFBundleIconFile</key>
    <string>tickeys</string>
    <key>CFBundleIdentifier</key>
    <string>${BUNDLE_ID}</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>${APP_NAME}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>${VERSION}</string>
    <key>CFBundleVersion</key>
    <string>${VERSION}</string>
    <key>LSMinimumSystemVersion</key>
    <string>13.0</string>
    <key>LSUIElement</key>
    <true/>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSSupportsAutomaticGraphicsSwitching</key>
    <true/>
</dict>
</plist>
EOF

# ── Step 7: Ad-hoc codesign + verify ──────────────────────────────────────────
echo "[7/7] signing and verifying"
codesign --force --deep --sign - "$BUNDLE_DIR"
codesign --verify --deep --strict --verbose=2 "$BUNDLE_DIR"
plutil -lint "$CONTENTS/Info.plist"

echo ""
echo "=== Done: $BUNDLE_DIR ==="
echo "Binary:  $(md5 -q "$MACOS/tickeys-redux")"
echo "Version: ${VERSION}"
echo ""
echo "To test: open \"$BUNDLE_DIR\""
