# BerryEditor - 100% Rust Code Editor

[![Tests](https://github.com/Oracleberry/berry-editor/workflows/Tests/badge.svg)](https://github.com/Oracleberry/berry-editor/actions)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A fully-featured code editor built entirely in Rust using Leptos and WebAssembly.

## Features

- 🦀 **100% Rust** - No JavaScript required
- 🚀 **WASM-powered** - Runs natively in the browser
- 🎨 **Syntax Highlighting** - Support for Rust, JavaScript, Python, and more
- 📁 **File Tree** - Navigate project files
- 🔍 **Search & Replace** - Powerful text search with regex support
- 🗺️ **Minimap** - Code overview navigation
- 📝 **Multi-cursor** - Edit multiple locations simultaneously
- 🔧 **LSP Support** - Code intelligence via Language Server Protocol
- 🌳 **Git Integration** - View diffs and manage changes

## Development

### Prerequisites

- Rust toolchain (stable)
- `trunk` for building and serving
- `wasm-pack` for testing

### Install Trunk

```bash
cargo install trunk
```

### Run Development Server

#### Option 1: Desktop App (Recommended - Full Features)

```bash
./run_desktop.sh
# OR
cargo tauri dev
```

This runs the **Tauri desktop app** with:
- ✅ Full file system access (file tree, save, load)
- ✅ Native OS integration
- ✅ All Tauri APIs available
- ✅ Better performance

#### Option 2: Browser Mode (Limited - WASM Only)

```bash
./run.sh
# OR
trunk serve
```

Then open http://127.0.0.1:8080/berry-editor/

This runs **WASM standalone in browser** with:
- ⚠️ No file system access (Tauri APIs unavailable)
- ⚠️ Limited to in-memory editing
- ✅ Good for testing Canvas rendering
- ✅ No installation required

### Run Tests

```bash
# Unit tests
cargo test --lib

# WASM integration tests
wasm-pack test --headless --firefox

# E2E tests (requires geckodriver)
./run_e2e_tests.sh

# All tests (CI simulation)
cargo test --lib && \
wasm-pack test --headless --firefox && \
./run_e2e_tests.sh
```

**Test Coverage**:
- 80 unit tests
- 230+ WASM integration tests
- 5 E2E tests (Syntax, Rendering, Codicon, Database, Terminal)

## Architecture

- **Leptos 0.7** - Reactive UI framework
- **Ropey** - Efficient rope-based text buffer
- **Web-sys** - Direct browser API bindings
- **wasm-bindgen** - Rust/WASM/JavaScript interop

## Project Structure

```
gui-editor/
├── src/
│   ├── lib.rs           # WASM entry point
│   ├── main.rs          # Application entry
│   ├── components.rs    # UI components
│   ├── editor.rs        # Editor panel
│   ├── file_tree.rs     # File explorer
│   ├── buffer.rs        # Text buffer (rope-based)
│   ├── syntax.rs        # Syntax highlighting
│   ├── cursor.rs        # Multi-cursor support
│   ├── search.rs        # Search & replace
│   ├── minimap.rs       # Code minimap
│   ├── lsp.rs           # LSP client
│   └── git.rs           # Git integration
├── index.html           # HTML entry point
├── Cargo.toml           # Rust dependencies
└── Trunk.toml           # Trunk configuration
```

## License

MIT
