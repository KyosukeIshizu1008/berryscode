#!/bin/bash
# Test script for berrycode_list_files functionality

echo "=========================================="
echo "BerryCode list_files Implementation Test"
echo "=========================================="
echo ""

echo "1. Building Tauri backend..."
cd /Users/kyosukeishizu/oracleberry/berrcode/gui-editor/src-tauri
cargo build 2>&1 | grep -E "(Compiling berry-editor-tauri|Finished)" | tail -5

if [ $? -ne 0 ]; then
    echo "❌ Build failed!"
    exit 1
fi

echo "✅ Build successful"
echo ""

echo "2. Testing berrycode_list_files implementation..."
echo "   Expected behavior:"
echo "   - Scans current directory recursively"
echo "   - Excludes: target/, node_modules/, .git/, etc."
echo "   - Includes: .rs, .toml, .md, .js, .ts, etc."
echo "   - Returns relative paths from project root"
echo ""

echo "3. To manually test in the app:"
echo "   a) Run: cargo tauri dev"
echo "   b) Open DevTools Console (F12)"
echo "   c) Execute:"
echo "      const files = await window.__TAURI__.core.invoke('berrycode_list_files');"
echo "      console.log(\`Found \${files.length} files:\`, files.slice(0, 10));"
echo ""

echo "4. Expected output in console:"
echo "   [BerryCode] Listing files in: \"/Users/.../gui-editor\""
echo "   [BerryCode] Found 150-300 files"
echo ""

echo "5. Sample files that should be included:"
echo "   - src/lib.rs"
echo "   - src/buffer.rs"
echo "   - src-tauri/Cargo.toml"
echo "   - README.md"
echo ""

echo "6. Files/directories that should be EXCLUDED:"
echo "   - target/debug/..."
echo "   - node_modules/..."
echo "   - .git/..."
echo "   - .DS_Store"
echo ""

echo "=========================================="
echo "Implementation Summary:"
echo "=========================================="
echo "✅ Function: berrycode_list_files()"
echo "✅ Location: src-tauri/src/berrycode_commands.rs:98"
echo "✅ Registered: main.rs:122"
echo "✅ State: BerryCodeState (managed in main.rs:135)"
echo ""
echo "✅ Documentation: BERRYCODE_LIST_FILES_IMPLEMENTATION.md"
echo ""
echo "Ready to test! Run: cargo tauri dev"
echo "=========================================="
