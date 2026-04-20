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
