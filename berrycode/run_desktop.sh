#!/bin/bash
# Run BerryCode Desktop App (egui Native)

set -e

echo "🖥️  BerryCode Desktop App (egui Native)"
echo "======================================="
echo ""
echo "Starting native desktop app..."
echo "  - Framework: egui + eframe"
echo "  - Backend: 100% Pure Rust"
echo "  - Rendering: WGPU (no WebView)"
echo ""
echo "Press Ctrl+C to stop"
echo ""

# Set Rust environment
export CARGO_HOME="$HOME/.cargo"
export RUSTUP_HOME="$HOME/.rustup"
export PATH="$HOME/.cargo/bin:$PATH"

# Run egui desktop app
cd "$(dirname "$0")"
cargo run --bin berrycode-egui
