// Library exports for testing
pub mod app_database;  // Application database (sessions, settings, workflow logs)
pub mod database;      // External database connections (PostgreSQL, MySQL, etc.)
pub mod terminal;
pub mod workflow;
pub mod persistent_terminal;

// Git operations (unified module)
pub mod git_core;

// gRPC client for berry_api integration
pub mod grpc_client;

// LSP operations (now using gRPC client)
pub mod lsp_core;

// LSP manager for Tauri commands
pub mod lsp;

// BerryCode CLI modules (integrated from parent)
pub mod berrycode;
