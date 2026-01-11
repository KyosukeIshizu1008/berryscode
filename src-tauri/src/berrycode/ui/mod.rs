//! Leptos UI Components for BerryCode
//!
//! This module contains Leptos-based UI components for gradual migration from JS to Rust.
//!
//! Migration Strategy:
//! - Phase 1: Settings Window (this module)
//! - Phase 2: Sidebar/File Tree
//! - Phase 3: Header/Toolbar
//! - Phase 4: Full Shell (with Rust-based editor)

#![cfg(feature = "leptos-ui")]

pub mod settings;

pub use settings::SettingsApp;
