//! Common utilities and components for BerryEditor
//!
//! Framework-agnostic utilities (no Dioxus, HTML, CSS, or JS)

// Platform abstraction layer
pub mod platform;
pub mod events;

// Fuzzy matching for search
pub mod fuzzy;

// Type-safe Codicon icon constants
pub mod icons;

// Input validation
pub mod validation;
