//! LSP Core - Unified Language Server Protocol operations
//!
//! This module provides a centralized implementation of LSP operations
//! that can be used by both Tauri commands and CLI code.
//!
//! ## Architecture
//!
//! - `types.rs`: Common LSP protocol type definitions
//! - `client.rs`: LSP client implementation for managing language servers
//!
//! ## Usage
//!
//! ```rust
//! use lsp_core::{LspClient, Position};
//!
//! // Create LSP client for a language
//! let client = LspClient::new("rust", "/workspace/root")?;
//!
//! // Get completions
//! let completions = client.get_completions("file:///path/to/file.rs", 10, 5)?;
//! ```

pub mod types;
pub mod client;

// Re-export commonly used types and functions
pub use types::*;
pub use client::LspClient;
