//! Common utilities and components for BerryEditor
//!
//! This module contains reusable functionality to ensure zero code duplication.

// Platform abstraction layer
pub mod platform;
pub mod events;
pub mod tauri_bridge;

// Fuzzy matching for search (command palette, file tree, symbols)
pub mod fuzzy;

// Type-safe Codicon icon constants
pub mod icons;

// Existing modules
pub mod async_bridge;
pub mod dialogs;
pub mod event_handler;
pub mod ui_components;
pub mod validation;
