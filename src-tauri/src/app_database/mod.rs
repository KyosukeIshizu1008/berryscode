// Application Database Module for Tauri
// Manages persistent storage for sessions, settings, and workflow logs

pub mod schema;
pub mod operations;
pub mod types;

pub use operations::AppDatabase;
pub use types::*;
