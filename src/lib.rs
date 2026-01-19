//! BerryEditor - 100% Rust Native Desktop Code Editor
//!
//! Built with egui 0.29 + eframe (Pure Native, No WebView)
//! GPU-accelerated rendering via WGPU
//!
//! ## Architecture
//!
//! - **main.rs + egui_app.rs**: Pure egui-based UI
//! - **native/ modules**: Framework-agnostic platform operations
//! - **buffer, syntax, theme**: Core text editing (framework-agnostic)
//!
//! ## Clean Architecture
//!
//! All Dioxus, HTML, CSS, and JavaScript code has been removed.
//! Binary size: ~6.4MB (down from 7.9MB)

// ===== Core Text Editing =====
pub mod buffer;
pub mod cursor;
pub mod syntax;
pub mod syntax_syntect; // Syntect-based syntax highlighting with One Dark theme

// ===== Native Platform Modules =====
pub mod native;

// ===== Common Utilities =====
pub mod common;
pub mod types;
pub mod focus_stack;

// ===== egui Application (Main UI) =====
pub mod egui_app;
pub mod egui_app_slack; // Slack-like chat UI

// ===== Search =====
pub mod search;

// ===== Settings =====
pub mod settings;

// ===== Git Integration =====
pub mod git;
