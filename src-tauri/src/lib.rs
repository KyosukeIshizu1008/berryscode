// Library exports for testing
pub mod app_database;  // Application database (sessions, settings, workflow logs)
pub mod database;      // External database connections (PostgreSQL, MySQL, etc.)
pub mod terminal;
pub mod workflow;
pub mod persistent_terminal;

// Git operations (unified module)
pub mod git_core;

// LSP operations (unified module)
pub mod lsp_core;

// BerryCode CLI modules (integrated from parent)
pub mod berrycode;
