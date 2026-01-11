#!/bin/bash
# Run BerryEditor Desktop App (with Tauri APIs)

set -e

echo "🖥️  BerryEditor Desktop App"
echo "========================="
echo ""
echo "Starting Tauri desktop app..."
echo "  - Frontend: trunk serve --port 8081"
echo "  - Backend: Rust + Tauri APIs"
echo "  - Window: Native desktop app"
echo ""
echo "Press Ctrl+C to stop"
echo ""

# Set Rust environment
export CARGO_HOME="$HOME/.cargo"
export RUSTUP_HOME="$HOME/.rustup"
export PATH="$HOME/.cargo/bin:$PATH"

# Run Tauri dev (this will automatically start trunk serve)
cd "$(dirname "$0")"
cargo tauri dev
