# BerryCode - Bevy-Powered Native IDE for Rust & Game Development

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![GitHub Sponsors](https://img.shields.io/github/sponsors/KyosukeIshizu1008)](https://github.com/sponsors/KyosukeIshizu1008)

A fully-featured native IDE built on **Bevy + bevy_egui + WGPU**, designed for Rust and Bevy game development. Zero web technologies — 100% Rust from rendering to logic.

## Screenshots

<p align="center">
  <img src="docs/demo/01_startup.png" width="80%" alt="BerryCode Editor">
</p>

## Features

### Code Editor
- Syntax highlighting (Rust, Python, JavaScript, C/C++) via Tree-sitter + Syntect
- LSP integration (completions, hover, go-to-definition, references, diagnostics)
- Code actions, inlay hints, rename refactoring
- Vim mode
- Minimap
- Multi-tab editing with image/3D model preview

### Project & Files
- File explorer with drag & drop
- Project-wide search & replace (regex, parallel via Rayon)
- Git integration (status, diff viewer, branches, stash, commit graph)

### Terminal
- iTerm2-class PTY terminal emulator (VT100/xterm)
- ANSI color rendering
- Multiple terminal sessions

### Bevy Game Engine Tools
- **Scene Editor** — Unity-class 3D viewport with gizmos, hierarchy, inspector
- **ECS Inspector** — entities, components, resources
- **Asset Browser** — textures, models, audio preview
- **Game View** — play-in-editor with live window capture
- **System Graph** — visualize Bevy system ordering
- **Event Monitor** — real-time Bevy event log
- **Query Visualizer** — inspect ECS queries
- **State Editor** — manage Bevy states
- **Animation** — timeline, dopesheet, animator editor
- **Visual Scripting** & **Shader Graph** editors
- **Bevy Templates** — quick project scaffolding
- **Plugin Browser** — search crates.io for Bevy plugins

### Developer Tools
- Debugger panel (variables, call stack, watch, breakpoints)
- Run/Build panel with console output
- Dockable tool panel (Console / Timeline / Dopesheet / Profiler)
- AI Chat assistant (via berry-api gRPC server)
- Live collaboration
- Remote development
- Custom snippet system
- Plugin system

## Quick Start

### Prerequisites

- Rust toolchain (stable 1.75+)
- On Linux: `libx11-dev`, `libasound2-dev`, `libudev-dev`

### Run

```bash
cargo run --bin berrycode
```

### Run with AI Features

```bash
# Terminal 1: Start API server
cd berry_api && cargo run

# Terminal 2: Start BerryCode
cargo run --bin berrycode
```

### Demo Mode (Feature Showcase + Screenshots + Video)

```bash
BERRYCODE_DEMO=1 cargo run --bin berrycode
# Outputs: docs/demo/*.png + docs/demo/demo.mp4
```

### Build Release

```bash
cargo build --release --bin berrycode
```

## Architecture

| Layer | Technology |
|-------|-----------|
| Window & Rendering | Bevy 0.15 + WGPU (Metal / Vulkan / DX12) |
| UI Framework | bevy_egui + egui 0.30 |
| Text Buffer | Ropey (rope-based) |
| Syntax | Tree-sitter + Syntect |
| Terminal | portable-pty + VTE parser |
| Git | libgit2 (via git2 crate) |
| Search | Rayon (parallel) + regex |
| LSP | lsp-types (native client) |
| AI Backend | gRPC (tonic + prost) |
| 3D Assets | gltf, tobj (OBJ), image |
| Window Capture | xcap |

## Project Structure

```
berrycode/
├── src/
│   ├── bin/berrycode-egui.rs   # Entry point
│   ├── app/                     # UI modules
│   │   ├── editor.rs           # Code editor
│   │   ├── terminal_emulator.rs # PTY terminal
│   │   ├── git.rs              # Git panel
│   │   ├── scene_editor/       # 3D scene editor (hierarchy, inspector, gizmo, etc.)
│   │   ├── debugger.rs         # Debugger
│   │   ├── ai_chat.rs          # AI assistant
│   │   ├── demo_capture.rs     # Feature showcase capture
│   │   └── ...                 # 50+ feature modules
│   ├── native/                  # Platform abstraction (fs, git, search, terminal, LSP)
│   ├── bevy_plugin.rs          # Bevy plugin integration
│   └── lib.rs
berry_api/                       # AI backend server (gRPC)
```

## Platform Support

| Platform | Backend | Status |
|----------|---------|--------|
| macOS | Metal | Supported |
| Linux | Vulkan / OpenGL | Supported |
| Windows | DirectX 12 | Supported |

## License

MIT
