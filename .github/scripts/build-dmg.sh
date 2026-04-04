#!/usr/bin/env bash

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

export PATH="$HOME/.cargo/bin:$PATH"

APP=Historia
BIN=historia
VER="${VER:-$(grep '^version' "$ROOT/Cargo.toml" | head -1 | sed 's/.*"\(.*\)"/\1/')}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"

ICON="$ROOT/assets/logo.png"

DIR="$CARGO_TARGET_DIR/dmg-build"
rm -rf "$DIR"
mkdir -p "$DIR"

ICON_SET_DIR="$DIR/icon.iconset"
mkdir -p "$ICON_SET_DIR"
for s in 16,16x16 32,16x16@2x 32,32x32 64,32x32@2x 128,128x128 256,128x128@2x 256,256x256 512,256x256@2x 512,512x512 1024,512x512@2x; do
    IFS=, read sz name <<< "$s"
    sips -z "$sz" "$sz" "$ICON" --out "$ICON_SET_DIR/icon_${name}.png" >/dev/null
done
iconutil -c icns "$ICON_SET_DIR" -o "$DIR/AppIcon.icns"
rm -rf "$ICON_SET_DIR"

rustup target list --installed | grep -q "^aarch64-apple-darwin$" || rustup target add aarch64-apple-darwin
cargo build --release --target aarch64-apple-darwin

rustup target list --installed | grep -q "^x86_64-apple-darwin$" || rustup target add x86_64-apple-darwin
cargo build --release --target x86_64-apple-darwin

package_dmg() {
    local target="$1" label="$2"
    local bundle="$DIR/$APP.app"
    local bin_path="$CARGO_TARGET_DIR/$target/release/$BIN"

    [[ -f "$bin_path" ]] || { echo "Binary not found: $bin_path" >&2; exit 1; }

    rm -rf "$bundle"
    mkdir -p "$bundle/Contents/MacOS" "$bundle/Contents/Resources"
    cp "$bin_path" "$bundle/Contents/MacOS/$BIN"
    cp "$DIR/AppIcon.icns" "$bundle/Contents/Resources/AppIcon.icns"

    cat > "$bundle/Contents/Info.plist" << PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>$APP</string>
    <key>CFBundleDisplayName</key><string>$APP</string>
    <key>CFBundleIdentifier</key><string>com.vox.app</string>
    <key>CFBundleVersion</key><string>$VER</string>
    <key>CFBundleShortVersionString</key><string>$VER</string>
    <key>CFBundleExecutable</key><string>$BIN</string>
    <key>CFBundleIconFile</key><string>AppIcon</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
    <key>LSMinimumSystemVersion</key><string>11.0</string>
    <key>LSUIElement</key><true/>
    <key>NSHighResolutionCapable</key><true/>
    <key>NSAutoFillRequiresTextContentTypeForOneTimeCodeOnMac</key><true/>
    <key>NSMicrophoneUsageDescription</key><string>$APP requires microphone access for speech recognition.</string>
</dict>
</plist>
PLIST

    codesign --force --deep -s - "$bundle"

    local staging="$DIR/staging"
    rm -rf "$staging"
    mkdir -p "$staging"
    cp -R "$bundle" "$staging/"
    ln -s /Applications "$staging/Applications"
    hdiutil create -volname "$APP" -srcfolder "$staging" -ov -format ULMO "$DIR/$BIN-macos-$label.dmg"
    rm -rf "$staging" "$bundle"
}

package_dmg aarch64-apple-darwin arm64
package_dmg x86_64-apple-darwin x86_64

rm -f "$DIR/AppIcon.icns"