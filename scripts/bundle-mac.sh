#!/usr/bin/env bash
# Build the release binary and assemble a distributable macOS app bundle at
# target/Amnézia.app. The layout matches the release `asset_root()` resolver:
# the executable at Contents/MacOS/amnezia resolves assets via
# ../Resources/assets, so the bundle is self-contained and double-clickable.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT_DIR/target}"

APP_NAME="Amnézia"
BIN_NAME="amnezia"
# Tracks the crate version in Cargo.toml.
VERSION="0.1.0"

APP="$TARGET_DIR/$APP_NAME.app"
BIN="$TARGET_DIR/release/$BIN_NAME"

cd "$ROOT_DIR"

echo "==> Building $BIN_NAME (release)"
cargo build -p "$BIN_NAME" --release

echo "==> Assembling $APP"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS"
mkdir -p "$APP/Contents/Resources/assets"

cp "$BIN" "$APP/Contents/MacOS/$BIN_NAME"
chmod +x "$APP/Contents/MacOS/$BIN_NAME"

# Copy the converted assets, skipping dev-only MIDI intermediates and macOS
# Finder metadata so the shipped bundle carries only what the game reads.
rsync -a --exclude='*.mid' --exclude='.DS_Store' \
    "$ROOT_DIR/assets/" "$APP/Contents/Resources/assets/"

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleName</key>
	<string>${APP_NAME}</string>
	<key>CFBundleDisplayName</key>
	<string>${APP_NAME}</string>
	<key>CFBundleExecutable</key>
	<string>${BIN_NAME}</string>
	<key>CFBundleIdentifier</key>
	<string>com.moondragon.amnezia</string>
	<key>CFBundlePackageType</key>
	<string>APPL</string>
	<key>CFBundleInfoDictionaryVersion</key>
	<string>6.0</string>
	<key>CFBundleVersion</key>
	<string>${VERSION}</string>
	<key>CFBundleShortVersionString</key>
	<string>${VERSION}</string>
	<key>LSMinimumSystemVersion</key>
	<string>11.0</string>
	<key>NSHighResolutionCapable</key>
	<true/>
</dict>
</plist>
PLIST

echo "==> Done: $APP"
