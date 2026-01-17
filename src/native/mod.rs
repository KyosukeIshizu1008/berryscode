//! Native platform integration module
//! Replaces Tauri IPC commands with direct Rust function calls

pub mod fs;
pub mod git;
pub mod grpc;
pub mod lsp;
pub mod search;
pub mod slack;
pub mod terminal;
pub mod watcher;

// Re-exports for convenience
pub use fs::*;
pub use git::*;
pub use grpc::*;
pub use lsp::*;
pub use search::*;
pub use slack::*;
pub use terminal::*;
pub use watcher::*;
