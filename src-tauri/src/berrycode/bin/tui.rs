//! BerryCode TUI - Terminal User Interface
//!
//! Launch the VS Code-like 3-pane editor interface

use anyhow::Result;
use std::env;
use std::path::PathBuf;

fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Get project root from current directory or argument
    let project_root = if let Some(path) = env::args().nth(1) {
        PathBuf::from(path)
    } else {
        env::current_dir()?
    };

    println!("🚀 Launching BerryCode TUI...");
    println!("📂 Project: {:?}", project_root);
    println!();

    // Run TUI
    berrycode::tui::run_tui(project_root)?;

    println!("\n👋 Goodbye!");

    Ok(())
}
