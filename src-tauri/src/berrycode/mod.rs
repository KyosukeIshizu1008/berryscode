//! BerryCode CLI Integration
//!
//! This module integrates all BerryCode CLI functionality into the Tauri editor.

pub mod args;
pub mod io;
pub mod display;
pub mod models;
pub mod repo;
pub mod repomap;
pub mod coders;
pub mod commands;
pub mod utils;
pub mod analytics;
pub mod exceptions;
pub mod history;
pub mod linter;
pub mod editor;
pub mod watch;
pub mod copypaste;
pub mod scrape;
pub mod onboarding;
pub mod versioncheck;
pub mod format_settings;
pub mod llm;
pub mod llm_grpc_client;
pub mod project_manager;
pub mod scaffolding;
pub mod workflow;
pub mod pipeline;
pub mod git_ops;
pub mod agents;
pub mod asset_registry;
pub mod berryflow_config;
pub mod notifications;
pub mod diff;
pub mod prompts;
pub mod prompt_optimizer;
pub mod tools;
pub mod cache;
pub mod retry;
pub mod context;
pub mod status;
pub mod summarizer;
pub mod tool_monitor;
pub mod agent;
pub mod task_spawner;
pub mod plan_mode;
pub mod welcome;
pub mod conversation_engine;
pub mod context_window;
pub mod project_analyzer;
pub mod pattern_detector;
pub mod dependency_tracker;
pub mod mcp;
pub mod modes;
pub mod memory;
pub mod artifacts;
pub mod tui;
pub mod lsp_client;
pub mod lsp_http_client;  // HTTP-based LSP client for Berry API
pub mod refactoring_engine;
pub mod git_operations;
pub mod common;
pub mod speculative_executor;
pub mod vector_search;
pub mod proactive_agent;
pub mod self_healing;
pub mod vision;
pub mod knowledge_graph;
pub mod swarm;
pub mod github_oauth;
pub mod persistent_terminal;
pub mod aci;
pub mod planner;
pub mod race_executor;
pub mod router;
pub mod static_responses;
pub mod embeddings;
pub mod chunker;
pub mod hybrid_search;
pub mod reranker;
pub mod trigger;
pub mod debug;
pub mod collaboration;

#[cfg(feature = "jupyter")]
pub mod jupyter;

// Re-export commonly used types
pub use args::Args;
pub use io::InputOutput;
pub use models::Model;
pub use repo::GitRepo;
pub use coders::Coder;

/// Version of berrycode
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Result type alias using anyhow::Error
pub type Result<T> = anyhow::Result<T>;
