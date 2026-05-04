//! BerryCode - Bevy IDE
//!
//! A Bevy-specialized code editor built with Bevy + bevy_egui.
//! GPU-accelerated rendering via WGPU.
//!
//! ## Architecture
//!
//! - **app/**: Bevy-based UI (egui panels, editor, git, terminal, etc.)
//! - **bevy_ide/**: Bevy-specific features (templates, ECS inspector, scene preview, assets)
//! - **bevy_plugin.rs**: Bevy Plugin integrating the editor
//! - **native/**: Platform operations (fs, git, LSP, terminal, search)
//! - **buffer, cursor, syntax**: Core text editing

// ===== Core Text Editing =====
pub mod buffer;
pub mod cursor;
pub mod syntax;

// ===== Native Platform Modules =====
pub mod native;

// ===== AI providers (BYOK direct API clients) =====
pub mod ai;

// ===== Coding agents (subprocess wrappers around Codex CLI / Claude Code) =====
// Provides Agent mode, Apply diff, and tool-calling capabilities by
// shelling out to mature external CLIs rather than reimplementing the
// agent loop in Rust. v0.4.5 / Phase 4.
pub mod agent;

// ===== Common Utilities =====
pub mod common;
pub mod focus_stack;
pub mod types;

// ===== Bevy Application =====
pub mod app;
pub mod bevy_ide;
pub mod bevy_plugin;

// Backwards-compatible re-export
pub mod egui_app {
    pub use crate::app::BerryCodeApp;
}

// ===== Search =====
pub mod search;

// ===== Settings =====
pub mod settings;

// ===== Git Integration =====
pub mod git;

// ===== Godot Project Read-Only Support (v0.8.x Migration & interop) =====
// Read `project.godot` + `.tscn` scene trees so users migrating from
// Godot can browse their existing project alongside new Bevy code.
// Strictly read-only: BerryCode is a bridge, not a converter.
pub mod godot_import;
