//! Common utilities and components for BerryEditor
//!
//! Framework-agnostic utilities (no Dioxus, HTML, CSS, or JS)

// Platform abstraction layer
pub mod events;
pub mod platform;

// Fuzzy matching for search
pub mod fuzzy;

// Type-safe Codicon icon constants
pub mod icons;

// Input validation
pub mod validation;

// Shell quoting for safely interpolating user-controlled values into
// `sh -c` commands. Used by the mobile runner, remote-dev SSH bridge,
// and anywhere else we hand a string to a child shell.
pub mod shell;
