//! Git integration module
//!
//! This module provides Tauri commands for Git operations.
//! The actual Git operations are implemented in the `git_core` module.

pub mod commands;

pub use commands::GitManager;
