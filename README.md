# BerryCode - Pure Rust Native Code Editor

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A fully-featured code editor built entirely in Rust using egui for 100% native performance.

## Features

- 🦀 **100% Pure Rust** - No JavaScript, no HTML, no CSS
- 🚀 **Native Desktop** - Direct GPU rendering with egui + WGPU
- 🎨 **Syntax Highlighting** - Tree-sitter based highlighting for multiple languages
- 📁 **File Tree** - Navigate project files with native performance
- 🔍 **Search & Replace** - Fast parallel search with regex support
- 💬 **AI Chat** - Integrated LLM support via berry-api
- 🔧 **LSP Support** - Code intelligence via Language Server Protocol
- 🌳 **Git Integration** - View status, stage, and commit changes
- 🖥️ **Terminal** - Integrated PTY terminal

## Development

### Prerequisites

- Rust toolchain (stable, 1.70+)
- No additional dependencies required (all native)

### Run Desktop App

```bash
# Quick start
cargo run --bin berrycode-egui

# Or use the convenience script
./run_desktop.sh

# Release build (optimized)
cargo build --release --bin berrycode-egui
./target/release/berrycode-egui
```

### Run with AI Features

To enable AI chat and completions, start the berry-api server:

```bash
# Terminal 1: Start API server
cd ../berry_api
./start-all.sh

# Terminal 2: Start BerryCode
cd berrycode
cargo run --bin berrycode-egui
```

### Run Tests

```bash
# Unit tests
cargo test --lib

# All tests
cargo test

# With logging
RUST_LOG=debug cargo test
```

## Architecture

- **egui 0.29** - Immediate mode GUI framework
- **eframe** - Native window management
- **WGPU** - Direct GPU rendering backend
- **Ropey** - Efficient rope-based text buffer
- **Tree-sitter** - Fast incremental parsing for syntax highlighting
- **git2** - Native git operations
- **portable-pty** - Cross-platform terminal support

## Project Structure

```
berrycode/
├── src/
│   ├── bin/
│   │   └── berrycode-egui.rs  # Application entry point
│   ├── egui_app.rs             # Main egui app logic
│   ├── native/                 # Native platform modules
│   │   ├── fs.rs              # File system operations
│   │   ├── git.rs             # Git operations (git2)
│   │   ├── search.rs          # Parallel search (rayon)
│   │   ├── terminal.rs        # PTY terminal
│   │   ├── lsp.rs             # LSP client (gRPC)
│   │   └── grpc.rs            # gRPC client for AI
│   ├── buffer.rs               # Text buffer (rope-based)
│   ├── syntax.rs               # Syntax highlighting
│   └── theme.rs                # Color scheme
├── assets/
│   └── codicon.ttf             # Icon font
├── Cargo.toml                  # Dependencies
└── CLAUDE.md                   # Architecture documentation
```

## Binary Size

- **Debug**: ~15MB
- **Release**: ~6.4MB (stripped)

## Performance

- **Startup time**: <500ms
- **Memory usage**: <200MB idle
- **Rendering**: 60fps smooth on all platforms

## Platform Support

- ✅ macOS (native Metal backend)
- ✅ Linux (native Vulkan/OpenGL backend)
- ✅ Windows (native DirectX 12 backend)

## License

MIT
