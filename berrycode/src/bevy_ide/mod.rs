//! Bevy-specialized IDE support for BerryCode
//!
//! Provides Bevy project detection, optimized rust-analyzer configuration,
//! documentation lookup, and code templates for Bevy game development.

pub mod assets;
pub mod detection;
pub mod docs;
pub mod inspector;
pub mod lsp_config;
pub mod scene_preview;
pub mod templates;
