// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app_database; // ✅ App Database: SQLite for persistent app state
mod berrycode_commands; // ✅ BerryCode CLI integration
mod database; // ✅ Database Tools: Connection management
mod fs_commands;
mod git_core; // ✅ Unified Git operations
mod git;
mod grpc_client; // ✅ gRPC client for berry_api integration
mod lsp_core; // ✅ Unified LSP operations
mod hyper_search;
mod indexer; // ✅ IntelliJ Pro: Background symbol indexing
mod lsp;
mod persistent_terminal; // ✅ Terminal: PTY-based persistent sessions
mod search_commands;
mod streaming; // ✅ Async streaming for large files
mod syntax_highlighter; // ✅ Parallel syntax highlighting with rayon // ✅ Strategy 3: Zero-memory parallel search
mod terminal; // ✅ Terminal: Tauri commands for terminal management
mod workflow; // ✅ Workflow Automation: Pipeline execution

use app_database::AppDatabase;
use berrycode_commands::BerryCodeState;
use database::DbManager;
use git::GitManager;
use indexer::SymbolIndex;
use lsp::LspManager;
use terminal::TerminalManagerState;
use workflow::WorkflowManager;
use std::sync::{Arc, Mutex};
use tauri::Manager;

fn main() {
    // Create LSP manager
    let lsp_manager = Arc::new(LspManager::new());

    // Create Git manager
    let git_manager = GitManager::new();

    // ✅ IntelliJ Pro: Create Symbol Index for background indexing
    let symbol_index = Arc::new(Mutex::new(SymbolIndex::new()));

    // ✅ Terminal: Create Terminal Manager
    let terminal_manager = TerminalManagerState::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(lsp_manager)
        .manage(git_manager)
        .manage(symbol_index) // ✅ IntelliJ Pro: Manage symbol index state
        .manage(terminal_manager) // ✅ Terminal: Manage terminal sessions
        .invoke_handler(tauri::generate_handler![
            fs_commands::get_current_dir,
            fs_commands::read_file,
            fs_commands::read_file_partial, // ✅ IntelliJ Pro: Lazy file loading
            fs_commands::read_file_chunk,   // ✅ IntelliJ Pro: Streaming large files
            fs_commands::write_file,
            fs_commands::read_dir,
            fs_commands::create_file,
            fs_commands::delete_file,
            fs_commands::rename_file,
            fs_commands::get_file_metadata,
            fs_commands::select_folder,
            search_commands::search_in_files,
            lsp::commands::lsp_initialize,
            lsp::commands::lsp_get_completions,
            lsp::commands::lsp_get_hover,
            lsp::commands::lsp_goto_definition,
            lsp::commands::lsp_get_diagnostics,
            lsp::commands::lsp_find_references,
            lsp::commands::lsp_shutdown,
            lsp::commands::lsp_add_file_to_context,
            git::commands::git_set_repo_path,
            git::commands::git_status,
            git::commands::git_list_branches,
            git::commands::git_current_branch,
            git::commands::git_stage_file,
            git::commands::git_unstage_file,
            git::commands::git_stage_all,
            git::commands::git_commit,
            git::commands::git_checkout_branch,
            git::commands::git_create_branch,
            git::commands::git_delete_branch,
            git::commands::git_log,
            git::commands::git_diff_file,
            git::commands::git_blame,
            // ✅ IntelliJ Pro: Background indexing commands
            indexer::commands::index_workspace,
            indexer::commands::search_symbols,
            indexer::commands::index_file,
            indexer::commands::get_symbol_count,
            // ✅ Parallel syntax highlighting commands
            syntax_highlighter::highlight_file_parallel,
            syntax_highlighter::invalidate_syntax_cache,
            syntax_highlighter::get_syntax_cache_stats,
            // ✅ Streaming commands for large files
            streaming::stream_large_file,
            streaming::read_file_auto,
            // ✅ Hyper-parallel search commands
            hyper_search::hyper_search,
            hyper_search::hyper_replace,
            // ✅ Database Tools commands
            database::commands::db_list_connections,
            database::commands::db_add_connection,
            database::commands::db_update_connection,
            database::commands::db_delete_connection,
            database::commands::db_test_connection,
            database::commands::db_execute_query,
            // ✅ Workflow Automation commands
            workflow::commands::workflow_list_presets,
            workflow::commands::workflow_start,
            workflow::commands::workflow_get_status,
            workflow::commands::workflow_pause,
            workflow::commands::workflow_resume,
            workflow::commands::workflow_cancel,
            // ✅ Terminal commands
            terminal::commands::terminal_execute_command,
            terminal::commands::terminal_get_history,
            terminal::commands::terminal_list_background_processes,
            terminal::commands::terminal_kill_process,
            terminal::commands::terminal_change_directory,
            terminal::commands::terminal_get_current_directory,
            // ✅ BerryCode CLI commands
            berrycode_commands::berrycode_init,
            berrycode_commands::berrycode_chat,
            berrycode_commands::berrycode_add_file,
            berrycode_commands::berrycode_drop_file,
            berrycode_commands::berrycode_list_files,
            berrycode_commands::berrycode_get_history,
            berrycode_commands::berrycode_clear_history,
            berrycode_commands::berrycode_list_models,
            berrycode_commands::berrycode_get_context,
            berrycode_commands::berrycode_cargo_check,
            berrycode_commands::berrycode_cargo_test,
            // ✅ BerryCode Chat History commands
            berrycode_commands::berrycode_create_chat_session,
            berrycode_commands::berrycode_list_chat_sessions,
            berrycode_commands::berrycode_load_chat_messages,
            berrycode_commands::berrycode_save_message,
            berrycode_commands::berrycode_delete_chat_session,
            berrycode_commands::berrycode_update_chat_title,
        ])
        .setup(|app| {
            // ✅ App Database: Initialize SQLite database for persistent app state
            let app_data_dir = app.path().app_data_dir().expect("Failed to get app data directory");
            std::fs::create_dir_all(&app_data_dir).expect("Failed to create app data directory");
            let db_path = app_data_dir.join("berrycode.db");
            let app_db = Arc::new(AppDatabase::new(db_path).expect("Failed to initialize app database"));
            app.manage(app_db);

            // ✅ BerryCode CLI: Initialize BerryCodeState
            let berrycode_state = BerryCodeState::default();
            app.manage(berrycode_state);

            // ✅ Database Tools: Initialize DbManager
            let db_manager = DbManager::new(&app.handle());
            app.manage(db_manager);

            // ✅ Workflow Automation: Initialize WorkflowManager
            let workflow_manager = WorkflowManager::new();
            app.manage(workflow_manager);

            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
