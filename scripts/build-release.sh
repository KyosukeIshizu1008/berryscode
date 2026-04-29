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
echo "[1/4] Building release binary..."
cargo build --release --bin berrycode
echo "  Binary: target/release/berrycode"

# ─── Fetch bundled Codex agent (prebuilt) ──────────────────────
# We ship the upstream `codex` CLI inside the .app / tarball so end
# users don't have to install Node + `@openai/codex` first. We
# download OpenAI's official release artifact instead of building
# from source — building takes ~30 min, downloading takes seconds,
# and we get the exact same binary they sign and ship.
CODEX_TAG="${CODEX_TAG:-rust-v0.125.0}"

# Map host (uname) to release artifact name. Cross-platform release
# would override these — for now we follow the "build for the host"
# convention the rest of this script uses.
HOST_OS="$(uname -s)"
HOST_ARCH="$(uname -m)"
case "$HOST_OS-$HOST_ARCH" in
  Darwin-arm64)        CODEX_ASSET="codex-aarch64-apple-darwin.tar.gz" ;;
  Darwin-x86_64)       CODEX_ASSET="codex-x86_64-apple-darwin.tar.gz" ;;
  Linux-aarch64)       CODEX_ASSET="codex-aarch64-unknown-linux-gnu.tar.gz" ;;
  Linux-x86_64)        CODEX_ASSET="codex-x86_64-unknown-linux-gnu.tar.gz" ;;
  *) echo "Unsupported host for Codex bundling: $HOST_OS-$HOST_ARCH"; exit 1 ;;
esac

CODEX_PREFIX="target/codex-prebuilt/$CODEX_TAG"
CODEX_BIN="$CODEX_PREFIX/codex"
echo "[2/4] Fetching bundled Codex agent ($CODEX_TAG, $CODEX_ASSET)..."
if [ -x "$CODEX_BIN" ]; then
  echo "  cached: $CODEX_BIN"
else
  mkdir -p "$CODEX_PREFIX"
  CODEX_URL="https://github.com/openai/codex/releases/download/$CODEX_TAG/$CODEX_ASSET"
  curl -fsSL "$CODEX_URL" -o "$CODEX_PREFIX/$CODEX_ASSET"
  tar -xzf "$CODEX_PREFIX/$CODEX_ASSET" -C "$CODEX_PREFIX"
  # The release tarballs name the binary with a platform suffix
  # (`codex-aarch64-apple-darwin`); normalise to plain `codex` so
  # the rest of the script doesn't have to know about it.
  if [ ! -x "$CODEX_BIN" ]; then
    EXTRACTED="$(find "$CODEX_PREFIX" -maxdepth 2 -type f -name 'codex-*' ! -name '*.tar.gz' | head -1)"
    [ -n "$EXTRACTED" ] && mv "$EXTRACTED" "$CODEX_BIN"
  fi
  chmod +x "$CODEX_BIN"
  rm -f "$CODEX_PREFIX/$CODEX_ASSET"
fi
ls -lh "$CODEX_BIN"

OS="$(uname -s)"
case "$OS" in
  Darwin)
    # ─── macOS: Create .app bundle + DMG ─────────────────────
    echo "[3/4] Creating macOS .app bundle..."

    APP="BerryCode.app"
    rm -rf "$APP"
    mkdir -p "$APP/Contents/MacOS"
    mkdir -p "$APP/Contents/Resources"
    mkdir -p "$APP/Contents/Resources/bin"

    cp target/release/berrycode "$APP/Contents/MacOS/berrycode"
    chmod +x "$APP/Contents/MacOS/berrycode"
    cp berrycode/assets/icon.icns "$APP/Contents/Resources/AppIcon.icns"
    cp -r berrycode/assets "$APP/Contents/Resources/assets"

    # Bundle Codex CLI. `bundled_binary_path` in `agent::mod` looks
    # for it at this exact path.
    cp "$CODEX_BIN" "$APP/Contents/Resources/bin/codex"
    chmod +x "$APP/Contents/Resources/bin/codex"

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

    echo "[4/4] Creating DMG..."
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
    echo "[3/4] Creating Linux package..."

    DIR="berrycode-${VERSION}-linux-x86_64"
    rm -rf "$DIR"
    mkdir -p "$DIR/bin"
    cp target/release/berrycode "$DIR/"
    cp -r berrycode/assets "$DIR/"
    cp LICENSE "$DIR/"
    cp README.md "$DIR/"
    cp berrycode/assets/icon_256.png "$DIR/berrycode.png"

    # Bundle Codex CLI alongside the main binary; resolved by
    # `bundled_binary_path` at runtime.
    cp "$CODEX_BIN" "$DIR/bin/codex"
    chmod +x "$DIR/bin/codex"

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

    echo "[4/4] Creating tarball..."
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
