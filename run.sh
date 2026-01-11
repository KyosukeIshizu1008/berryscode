#!/bin/bash
# Run BerryEditor standalone (WASM in browser)

set -e

echo "🚀 BerryEditor Standalone"
echo "========================="
echo ""

# Set Rust environment
export CARGO_HOME="$HOME/.cargo"
export RUSTUP_HOME="$HOME/.rustup"
export PATH="$HOME/.cargo/bin:$PATH"

echo "📦 Building and serving BerryEditor..."
echo "   Open browser at: http://127.0.0.1:8080"
echo ""
echo "   Press Ctrl+C to stop"
echo ""

trunk serve --open
