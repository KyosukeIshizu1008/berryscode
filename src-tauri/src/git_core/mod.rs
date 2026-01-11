//! Git Core - Unified Git operations module
//!
//! This module provides a centralized implementation of Git operations
//! that can be used by both Tauri commands and CLI code.
//!
//! ## Architecture
//!
//! - `types.rs`: Common type definitions
//! - `operations.rs`: Git operations using git2-rs (native Rust bindings)
//!
//! ## Usage
//!
//! ```rust
//! use git_core::{get_status, commit};
//!
//! // Get repository status
//! let status = get_status(&repo_path)?;
//!
//! // Create a commit
//! let commit_id = commit(&repo_path, "Initial commit")?;
//! ```

pub mod types;
pub mod operations;

// Re-export commonly used types and functions
pub use types::*;
pub use operations::*;
