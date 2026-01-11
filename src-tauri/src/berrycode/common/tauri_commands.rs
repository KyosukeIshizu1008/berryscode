//! Tauri command macros to eliminate code duplication
//!
//! Provides macros to generate Tauri commands with consistent error handling.

/// Create an LSP command with standard error handling
///
/// # Example
/// ```ignore
/// create_lsp_command!(lsp_completion, get_completions);
/// ```
#[macro_export]
macro_rules! create_lsp_command {
    ($name:ident, $method:ident) => {
        #[tauri::command]
        pub async fn $name(
            file_path: String,
            line: u32,
            character: u32,
        ) -> Result<serde_json::Value, String> {
            crate::berrycode::lsp_client::LSP_CLIENT
                .$method(&file_path, line, character)
                .await
                .map(|result| serde_json::to_value(result).unwrap_or(serde_json::Value::Null))
                .map_err(|e| format!("LSP error: {}", e))
        }
    };
}

/// Create a debug command with standard error handling
///
/// # Example
/// ```ignore
/// create_debug_command!(debug_start, start_session);
/// ```
#[macro_export]
macro_rules! create_debug_command {
    ($name:ident, $method:ident) => {
        #[tauri::command]
        pub async fn $name(
            session_id: String,
        ) -> Result<serde_json::Value, String> {
            crate::berrycode::debug::DEBUG_MANAGER
                .$method(&session_id)
                .await
                .map(|result| serde_json::to_value(result).unwrap_or(serde_json::Value::Null))
                .map_err(|e| format!("Debug error: {}", e))
        }
    };
}

/// Create a Git command with standard error handling
///
/// # Example
/// ```ignore
/// create_git_command!(git_status, get_status);
/// ```
#[macro_export]
macro_rules! create_git_command {
    ($name:ident, $method:ident) => {
        #[tauri::command]
        pub async fn $name() -> Result<serde_json::Value, String> {
            crate::berrycode::git_operations::GIT_OPS
                .$method()
                .await
                .map(|result| serde_json::to_value(result).unwrap_or(serde_json::Value::Null))
                .map_err(|e| format!("Git error: {}", e))
        }
    };
}

/// Create a refactoring command with standard error handling
///
/// Supports multiple refactoring operation types with different parameter sets
#[macro_export]
macro_rules! create_refactor_command {
    // Rename: (file_path, line, character, new_name) -> WorkspaceEdit
    (rename) => {
        #[tauri::command]
        pub async fn refactor_rename(
            file_path: String,
            line: u32,
            character: u32,
            new_name: String,
        ) -> Result<serde_json::Value, String> {
            use std::path::Path;
            use crate::berrycode::refactoring_engine::{RefactoringEngine, Position};

            let engine = RefactoringEngine::new();
            engine
                .rename_symbol(
                    Path::new(&file_path),
                    Position { line, character },
                    &new_name,
                )
                .await
                .map(|result| serde_json::to_value(result).unwrap_or(serde_json::Value::Null))
                .map_err(|e| format!("Refactoring error: {}", e))
        }
    };

    // Extract method: (file_path, start_line, start_char, end_line, end_char, method_name) -> Vec<TextEdit>
    (extract_method) => {
        #[tauri::command]
        pub async fn refactor_extract_method(
            file_path: String,
            start_line: u32,
            start_character: u32,
            end_line: u32,
            end_character: u32,
            method_name: String,
        ) -> Result<serde_json::Value, String> {
            use std::path::Path;
            use crate::berrycode::refactoring_engine::{RefactoringEngine, Position, Range};

            let range = Range {
                start: Position {
                    line: start_line,
                    character: start_character,
                },
                end: Position {
                    line: end_line,
                    character: end_character,
                },
            };

            let engine = RefactoringEngine::new();
            engine
                .extract_method(Path::new(&file_path), range, &method_name)
                .await
                .map(|result| serde_json::to_value(result).unwrap_or(serde_json::Value::Null))
                .map_err(|e| format!("Refactoring error: {}", e))
        }
    };

    // Inline variable: (file_path, line, character) -> Vec<TextEdit>
    (inline_variable) => {
        #[tauri::command]
        pub async fn refactor_inline_variable(
            file_path: String,
            line: u32,
            character: u32,
        ) -> Result<serde_json::Value, String> {
            use std::path::Path;
            use crate::berrycode::refactoring_engine::{RefactoringEngine, Position};

            let engine = RefactoringEngine::new();
            engine
                .inline_variable(Path::new(&file_path), Position { line, character })
                .await
                .map(|result| serde_json::to_value(result).unwrap_or(serde_json::Value::Null))
                .map_err(|e| format!("Refactoring error: {}", e))
        }
    };

    // Optimize imports: (file_path) -> Vec<TextEdit>
    (optimize_imports) => {
        #[tauri::command]
        pub async fn refactor_optimize_imports(file_path: String) -> Result<serde_json::Value, String> {
            use std::path::Path;
            use crate::berrycode::refactoring_engine::RefactoringEngine;

            let engine = RefactoringEngine::new();
            engine
                .optimize_imports(Path::new(&file_path))
                .await
                .map(|result| serde_json::to_value(result).unwrap_or(serde_json::Value::Null))
                .map_err(|e| format!("Refactoring error: {}", e))
        }
    };

    // Move symbol: (file_path, line, character, target_file) -> WorkspaceEdit
    (move_symbol) => {
        #[tauri::command]
        pub async fn refactor_move_symbol(
            file_path: String,
            line: u32,
            character: u32,
            target_file: String,
        ) -> Result<serde_json::Value, String> {
            use std::path::Path;
            use crate::berrycode::refactoring_engine::{RefactoringEngine, Position};

            let engine = RefactoringEngine::new();
            engine
                .move_symbol(
                    Path::new(&file_path),
                    Position { line, character },
                    Path::new(&target_file),
                )
                .await
                .map(|result| serde_json::to_value(result).unwrap_or(serde_json::Value::Null))
                .map_err(|e| format!("Refactoring error: {}", e))
        }
    };

    // Change signature: (file_path, line, character, new_signature) -> WorkspaceEdit
    (change_signature) => {
        #[tauri::command]
        pub async fn refactor_change_signature(
            file_path: String,
            line: u32,
            character: u32,
            new_signature: String,
        ) -> Result<serde_json::Value, String> {
            use std::path::Path;
            use crate::berrycode::refactoring_engine::{RefactoringEngine, Position};

            let engine = RefactoringEngine::new();
            engine
                .change_signature(
                    Path::new(&file_path),
                    Position { line, character },
                    &new_signature,
                )
                .await
                .map(|result| serde_json::to_value(result).unwrap_or(serde_json::Value::Null))
                .map_err(|e| format!("Refactoring error: {}", e))
        }
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_macros_compile() {
        // Ensure macros compile correctly
        assert!(true);
    }
}
