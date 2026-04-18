#!/bin/bash
# Build BerryCode release for the current platform
# Usage: ./scripts/build-release.sh [version]
#
# macOS:  produces BerryCode.app + .dmg
# Linux:  produces tarball with binary + .desktop
# Windows: run in PowerShell — see build-release.ps1

set -euo pipefail

VERSION="${1:-0.2.0}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT_DIR"

echo "=== BerryCode Release Build v${VERSION} ==="
echo ""

# ─── Build release binary ──────────────────────────────────────
echo "[1/3] Building release binary..."
cargo build --release --bin berrycode
echo "  Binary: target/release/berrycode"

OS="$(uname -s)"
case "$OS" in
  Darwin)
    # ─── macOS: Create .app bundle + DMG ─────────────────────
    echo "[2/3] Creating macOS .app bundle..."

    APP="BerryCode.app"
    rm -rf "$APP"
    mkdir -p "$APP/Contents/MacOS"
    mkdir -p "$APP/Contents/Resources"

    cp target/release/berrycode "$APP/Contents/MacOS/berrycode"
    chmod +x "$APP/Contents/MacOS/berrycode"
    cp berrycode/assets/icon.icns "$APP/Contents/Resources/AppIcon.icns"
    cp -r berrycode/assets "$APP/Contents/Resources/assets"

    cat > "$APP/Contents/Info.plist" << PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key>
  <string>BerryCode</string>
  <key>CFBundleDisplayName</key>
  <string>BerryCode</string>
  <key>CFBundleIdentifier</key>
  <string>com.berrycode.editor</string>
  <key>CFBundleVersion</key>
  <string>${VERSION}</string>
  <key>CFBundleShortVersionString</key>
  <string>${VERSION}</string>
  <key>CFBundleExecutable</key>
  <string>berrycode</string>
  <key>CFBundleIconFile</key>
  <string>AppIcon</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>LSMinimumSystemVersion</key>
  <string>11.0</string>
  <key>NSHighResolutionCapable</key>
  <true/>
  <key>LSApplicationCategoryType</key>
  <string>public.app-category.developer-tools</string>
</dict>
</plist>
PLIST

    echo "  Created: $APP"

    echo "[3/3] Creating DMG..."
    DMG_NAME="BerryCode-${VERSION}-macOS.dmg"
    rm -rf dmg_tmp "$DMG_NAME"
    mkdir -p dmg_tmp
    cp -r "$APP" dmg_tmp/
    ln -s /Applications dmg_tmp/Applications

    hdiutil create -volname "BerryCode" \
      -srcfolder dmg_tmp \
      -ov -format UDZO \
      "$DMG_NAME"

    rm -rf dmg_tmp

    echo ""
    echo "=== Done ==="
    echo "  .app : $APP"
    echo "  .dmg : $DMG_NAME"
    echo ""
    echo "To install: open $DMG_NAME and drag BerryCode to Applications"
    ;;

  Linux)
    # ─── Linux: Create tarball ───────────────────────────────
    echo "[2/3] Creating Linux package..."

    DIR="berrycode-${VERSION}-linux-x86_64"
    rm -rf "$DIR"
    mkdir -p "$DIR"
    cp target/release/berrycode "$DIR/"
    cp -r berrycode/assets "$DIR/"
    cp LICENSE "$DIR/"
    cp README.md "$DIR/"
    cp berrycode/assets/icon_256.png "$DIR/berrycode.png"

    cat > "$DIR/berrycode.desktop" << DESKTOP
[Desktop Entry]
Name=BerryCode
Comment=Bevy Game Engine IDE
Exec=berrycode
Icon=berrycode
Terminal=false
Type=Application
Categories=Development;IDE;
DESKTOP

    echo "[3/3] Creating tarball..."
    ARCHIVE="${DIR}.tar.gz"
    tar czf "$ARCHIVE" "$DIR"
    rm -rf "$DIR"

    echo ""
    echo "=== Done ==="
    echo "  Archive: $ARCHIVE"
    echo ""
    echo "To install:"
    echo "  tar xzf $ARCHIVE"
    echo "  sudo cp ${DIR}/berrycode /usr/local/bin/"
    echo "  cp ${DIR}/berrycode.desktop ~/.local/share/applications/"
    ;;

  *)
    echo "Unsupported OS: $OS"
    echo "On Windows, use build-release.ps1"
    exit 1
    ;;
esac
