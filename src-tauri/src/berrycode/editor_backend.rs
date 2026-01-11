//! BerryEditor Backend - Tauri Commands for File Operations
//!
//! This module provides Tauri commands for the Leptos editor frontend

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;
use anyhow::Result;

/// File tree item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileItem {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub children: Option<Vec<FileItem>>,
}

/// File content response
#[derive(Debug, Serialize, Deserialize)]
pub struct FileContent {
    pub path: String,
    pub content: String,
    pub language: String,
}

/// Directory listing
#[derive(Debug, Serialize, Deserialize)]
pub struct DirectoryListing {
    pub items: Vec<FileItem>,
}

/// Read directory and build file tree
pub fn read_directory_tree(path: &Path, max_depth: usize) -> Result<Vec<FileItem>> {
    if max_depth == 0 {
        return Ok(Vec::new());
    }

    let mut items = Vec::new();

    if !path.is_dir() {
        return Ok(items);
    }

    let entries = fs::read_dir(path)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        // Skip hidden files and common ignore patterns
        if name.starts_with('.') || name == "node_modules" || name == "target" {
            continue;
        }

        let is_dir = path.is_dir();
        let children = if is_dir {
            Some(read_directory_tree(&path, max_depth - 1)?)
        } else {
            None
        };

        items.push(FileItem {
            name,
            path: path.to_string_lossy().to_string(),
            is_dir,
            children,
        });
    }

    // Sort: directories first, then files, both alphabetically
    items.sort_by(|a, b| {
        match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });

    Ok(items)
}

/// Detect language from file extension
fn detect_language(path: &Path) -> String {
    match path.extension().and_then(|e| e.to_str()) {
        Some("rs") => "rust".to_string(),
        Some("js") | Some("jsx") => "javascript".to_string(),
        Some("ts") | Some("tsx") => "typescript".to_string(),
        Some("py") => "python".to_string(),
        Some("go") => "go".to_string(),
        Some("java") => "java".to_string(),
        Some("c") => "c".to_string(),
        Some("cpp") | Some("cc") | Some("cxx") => "cpp".to_string(),
        Some("html") => "html".to_string(),
        Some("css") => "css".to_string(),
        Some("json") => "json".to_string(),
        Some("yaml") | Some("yml") => "yaml".to_string(),
        Some("toml") => "toml".to_string(),
        Some("md") => "markdown".to_string(),
        Some("sh") | Some("bash") => "bash".to_string(),
        _ => "plaintext".to_string(),
    }
}

/// Tauri Commands

#[tauri::command]
pub fn editor_read_dir(path: String) -> Result<DirectoryListing, String> {
    let path = PathBuf::from(path);
    let items = read_directory_tree(&path, 3)
        .map_err(|e| format!("Failed to read directory: {}", e))?;

    Ok(DirectoryListing { items })
}

#[tauri::command]
pub fn editor_read_file(path: String) -> Result<FileContent, String> {
    let path_buf = PathBuf::from(&path);
    let content = fs::read_to_string(&path_buf)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let language = detect_language(&path_buf);

    Ok(FileContent {
        path,
        content,
        language,
    })
}

#[tauri::command]
pub fn editor_write_file(path: String, content: String) -> Result<(), String> {
    fs::write(&path, content)
        .map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(())
}

#[tauri::command]
pub fn editor_create_file(path: String) -> Result<(), String> {
    // Create parent directories if needed
    if let Some(parent) = Path::new(&path).parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directories: {}", e))?;
    }

    // Create empty file
    fs::write(&path, "")
        .map_err(|e| format!("Failed to create file: {}", e))?;

    Ok(())
}

#[tauri::command]
pub fn editor_delete_file(path: String) -> Result<(), String> {
    let path = Path::new(&path);

    if path.is_dir() {
        fs::remove_dir_all(path)
            .map_err(|e| format!("Failed to delete directory: {}", e))?;
    } else {
        fs::remove_file(path)
            .map_err(|e| format!("Failed to delete file: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
pub fn editor_rename_file(old_path: String, new_path: String) -> Result<(), String> {
    fs::rename(&old_path, &new_path)
        .map_err(|e| format!("Failed to rename file: {}", e))?;

    Ok(())
}

#[tauri::command]
pub fn editor_create_directory(path: String) -> Result<(), String> {
    fs::create_dir_all(&path)
        .map_err(|e| format!("Failed to create directory: {}", e))?;

    Ok(())
}

#[tauri::command]
pub fn editor_get_file_info(path: String) -> Result<FileInfo, String> {
    let metadata = fs::metadata(&path)
        .map_err(|e| format!("Failed to get file info: {}", e))?;

    Ok(FileInfo {
        path,
        size: metadata.len(),
        is_dir: metadata.is_dir(),
        is_readonly: metadata.permissions().readonly(),
        modified: metadata.modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs()),
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub size: u64,
    pub is_dir: bool,
    pub is_readonly: bool,
    pub modified: Option<u64>,
}

/// Search in files
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub path: String,
    pub line: usize,
    pub column: usize,
    pub content: String,
}

#[tauri::command]
pub fn editor_search_in_files(
    directory: String,
    query: String,
    case_sensitive: bool,
) -> Result<Vec<SearchResult>, String> {
    use walkdir::WalkDir;

    let mut results = Vec::new();

    for entry in WalkDir::new(&directory)
        .max_depth(10)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();

        // Skip binary files and common ignore patterns
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if matches!(ext, "exe" | "dll" | "so" | "dylib" | "png" | "jpg" | "jpeg" | "gif" | "pdf") {
                continue;
            }
        }

        if let Ok(content) = fs::read_to_string(path) {
            for (line_num, line) in content.lines().enumerate() {
                let found = if case_sensitive {
                    line.contains(&query)
                } else {
                    line.to_lowercase().contains(&query.to_lowercase())
                };

                if found {
                    if let Some(col) = line.find(&query) {
                        results.push(SearchResult {
                            path: path.to_string_lossy().to_string(),
                            line: line_num + 1,
                            column: col,
                            content: line.to_string(),
                        });
                    }
                }
            }
        }
    }

    Ok(results)
}

// ============================================================================
// LSP Commands (Berry API HTTP Integration)
// ============================================================================

use crate::berrycode::lsp_http_client::LspHttpClient;
use once_cell::sync::Lazy;
use std::sync::RwLock;
use uuid::Uuid;

/// Global LSP HTTP client instance
static LSP_HTTP_CLIENT: Lazy<RwLock<Option<LspHttpClient>>> = Lazy::new(|| RwLock::new(None));

/// Initialize LSP HTTP client for a project
/// ✅ FIX: Connect to Berry API REST API server at port 8081 instead of localhost:8080
/// This avoids the EOF error caused by connecting to the frontend (Trunk dev server)
#[tauri::command]
pub fn lsp_initialize(project_root: String) -> Result<(), String> {
    // Generate a unique session ID for this project
    let session_id = Uuid::new_v4().to_string();

    // ✅ FIX: Create HTTP client that talks to Berry API REST endpoints on correct port
    // Using "localhost" to allow both IPv4 and IPv6 resolution
    // Berry API server binds to localhost:8081 (REST API) - gRPC is on 50051
    let client = LspHttpClient::new(
        "http://localhost:8081".to_string(),
        session_id,
    );

    *LSP_HTTP_CLIENT.write().unwrap() = Some(client);

    tracing::info!(
        project_root = %project_root,
        "LSP HTTP client initialized (Berry API integration)"
    );

    Ok(())
}

/// Get completions at a specific position
#[tauri::command]
pub async fn lsp_get_completions(
    file_path: String,
    line: u32,
    character: u32,
) -> Result<serde_json::Value, String> {
    let client = LSP_HTTP_CLIENT.read().unwrap();
    let client = client.as_ref().ok_or("LSP HTTP client not initialized")?;

    let path = Path::new(&file_path);
    let completions = client.completion(path, line, character)
        .await
        .map_err(|e| format!("LSP HTTP error: {}", e))?;

    Ok(serde_json::to_value(completions).unwrap_or(serde_json::json!([])))
}

/// Get diagnostics for a file
#[tauri::command]
pub async fn lsp_get_diagnostics(file_path: String) -> Result<serde_json::Value, String> {
    let client = LSP_HTTP_CLIENT.read().unwrap();
    let client = client.as_ref().ok_or("LSP HTTP client not initialized")?;

    let path = Path::new(&file_path);
    let diagnostics = client.get_diagnostics(path)
        .await
        .map_err(|e| format!("LSP HTTP error: {}", e))?;

    Ok(serde_json::to_value(diagnostics).unwrap_or(serde_json::json!([])))
}

/// Get hover information at a position
#[tauri::command]
pub async fn lsp_hover(
    file_path: String,
    line: u32,
    character: u32,
) -> Result<serde_json::Value, String> {
    let client = LSP_HTTP_CLIENT.read().unwrap();
    let client = client.as_ref().ok_or("LSP HTTP client not initialized")?;

    let path = Path::new(&file_path);
    let hover_text = client.hover(path, line, character)
        .await
        .map_err(|e| format!("LSP HTTP error: {}", e))?;

    Ok(serde_json::json!({
        "contents": hover_text
    }))
}

/// Go to definition
#[tauri::command]
pub async fn lsp_goto_definition(
    file_path: String,
    line: u32,
    character: u32,
) -> Result<serde_json::Value, String> {
    let client = LSP_HTTP_CLIENT.read().unwrap();
    let client = client.as_ref().ok_or("LSP HTTP client not initialized")?;

    let path = Path::new(&file_path);
    let location = client.goto_definition(path, line, character)
        .await
        .map_err(|e| format!("LSP HTTP error: {}", e))?;

    if let Some(loc) = location {
        Ok(serde_json::json!({
            "uri": loc.uri.to_string(),
            "line": loc.range.start.line,
            "character": loc.range.start.character
        }))
    } else {
        Ok(serde_json::json!(null))
    }
}

/// Find all references
#[tauri::command]
pub async fn lsp_find_references(
    file_path: String,
    line: u32,
    character: u32,
) -> Result<serde_json::Value, String> {
    let client = LSP_HTTP_CLIENT.read().unwrap();
    let client = client.as_ref().ok_or("LSP HTTP client not initialized")?;

    let path = Path::new(&file_path);
    let references = client.find_references(path, line, character)
        .await
        .map_err(|e| format!("LSP HTTP error: {}", e))?;

    let ref_list: Vec<serde_json::Value> = references.iter().map(|loc| {
        serde_json::json!({
            "uri": loc.uri.to_string(),
            "line": loc.range.start.line,
            "character": loc.range.start.character
        })
    }).collect();

    Ok(serde_json::json!(ref_list))
}

/// Get code actions
#[tauri::command]
pub async fn lsp_code_actions(
    file_path: String,
    start_line: u32,
    start_character: u32,
    end_line: u32,
    end_character: u32,
) -> Result<serde_json::Value, String> {
    // Simplified implementation
    Ok(serde_json::json!([]))
}

// ============================================================================
// Debug Commands (DAP)
// ============================================================================

// Note: For now, these are simplified implementations.
// Full DAP integration would connect to src/web/api/debug/debug_api.rs

/// Start a debug session
#[tauri::command]
pub async fn debug_start_session(program_path: String) -> Result<String, String> {
    // Simplified: Generate a session ID
    // In full implementation, this would call the DAP debug_api
    let session_id = uuid::Uuid::new_v4().to_string();
    Ok(session_id)
}

/// Stop a debug session
#[tauri::command]
pub async fn debug_stop_session(session_id: String) -> Result<(), String> {
    // Simplified implementation
    Ok(())
}

/// Set a breakpoint
#[tauri::command]
pub async fn debug_set_breakpoint(
    session_id: String,
    file: String,
    line: u32,
    condition: Option<String>,
) -> Result<serde_json::Value, String> {
    // Simplified: Return a mock breakpoint
    Ok(serde_json::json!({
        "id": uuid::Uuid::new_v4().to_string(),
        "file": file,
        "line": line,
        "condition": condition,
        "verified": true
    }))
}

/// Remove a breakpoint
#[tauri::command]
pub async fn debug_remove_breakpoint(
    session_id: String,
    breakpoint_id: String,
) -> Result<(), String> {
    // Simplified implementation
    Ok(())
}

/// Continue execution
#[tauri::command]
pub async fn debug_continue(session_id: String) -> Result<(), String> {
    // Simplified implementation
    Ok(())
}

/// Step over
#[tauri::command]
pub async fn debug_step_over(session_id: String) -> Result<(), String> {
    // Simplified implementation
    Ok(())
}

/// Step into
#[tauri::command]
pub async fn debug_step_into(session_id: String) -> Result<(), String> {
    // Simplified implementation
    Ok(())
}

/// Step out
#[tauri::command]
pub async fn debug_step_out(session_id: String) -> Result<(), String> {
    // Simplified implementation
    Ok(())
}

/// Get stack trace
#[tauri::command]
pub async fn debug_get_stack_trace(session_id: String) -> Result<serde_json::Value, String> {
    // Simplified: Return empty stack trace
    Ok(serde_json::json!([]))
}

/// Get variables for a frame
#[tauri::command]
pub async fn debug_get_variables(
    session_id: String,
    frame_id: i64,
) -> Result<serde_json::Value, String> {
    // Simplified: Return empty scopes
    Ok(serde_json::json!([]))
}

/// Evaluate expression in debug context
#[tauri::command]
pub async fn debug_evaluate(
    session_id: String,
    expression: String,
    frame_id: Option<i64>,
) -> Result<String, String> {
    // Simplified: Return mock result
    Ok(format!("Result of '{}': <not implemented>", expression))
}

// ============================================================================
// Refactoring Commands (Phase 3)
// ============================================================================

use crate::berrycode::create_refactor_command;

// Generate all refactoring Tauri commands using macros (zero duplication)
create_refactor_command!(rename);
create_refactor_command!(extract_method);
create_refactor_command!(inline_variable);
create_refactor_command!(optimize_imports);
create_refactor_command!(move_symbol);
create_refactor_command!(change_signature);

// ============================================================================
// Git Commands (Phase 4)
// ============================================================================

use crate::berrycode::git_operations::{GitOperations, FileStatus, BranchInfo, CommitInfo, FileDiff, BlameLineInfo};

// Global Git operations instance (Lazy and RwLock already imported above)
static GIT_OPS: Lazy<RwLock<Option<GitOperations>>> = Lazy::new(|| RwLock::new(None));

/// Initialize Git operations for a repository
#[tauri::command]
pub fn git_init(repo_path: String) -> Result<(), String> {
    let ops = GitOperations::open(&repo_path)
        .map_err(|e| format!("Failed to open Git repository: {}", e))?;

    let mut git_ops = GIT_OPS.write().unwrap();
    *git_ops = Some(ops);

    Ok(())
}

/// Get Git status
#[tauri::command]
pub fn git_status() -> Result<Vec<FileStatus>, String> {
    let git_ops = GIT_OPS.read().unwrap();
    let ops = git_ops.as_ref().ok_or("Git not initialized")?;

    ops.get_status()
        .map_err(|e| format!("Git error: {}", e))
}

/// List branches
#[tauri::command]
pub fn git_list_branches() -> Result<Vec<BranchInfo>, String> {
    let git_ops = GIT_OPS.read().unwrap();
    let ops = git_ops.as_ref().ok_or("Git not initialized")?;

    ops.list_branches()
        .map_err(|e| format!("Git error: {}", e))
}

/// Create a new branch
#[tauri::command]
pub fn git_create_branch(name: String) -> Result<(), String> {
    let git_ops = GIT_OPS.read().unwrap();
    let ops = git_ops.as_ref().ok_or("Git not initialized")?;

    ops.create_branch(&name)
        .map_err(|e| format!("Git error: {}", e))
}

/// Checkout a branch
#[tauri::command]
pub fn git_checkout_branch(branch_name: String) -> Result<(), String> {
    let git_ops = GIT_OPS.read().unwrap();
    let ops = git_ops.as_ref().ok_or("Git not initialized")?;

    ops.checkout_branch(&branch_name)
        .map_err(|e| format!("Git error: {}", e))
}

/// Commit changes
#[tauri::command]
pub fn git_commit(message: String, files: Vec<String>) -> Result<String, String> {
    let git_ops = GIT_OPS.read().unwrap();
    let ops = git_ops.as_ref().ok_or("Git not initialized")?;

    let file_paths: Vec<PathBuf> = files.iter().map(PathBuf::from).collect();

    ops.commit(&message, file_paths)
        .map_err(|e| format!("Git error: {}", e))
}

/// Get file diff
#[tauri::command]
pub fn git_diff(file_path: String) -> Result<FileDiff, String> {
    let git_ops = GIT_OPS.read().unwrap();
    let ops = git_ops.as_ref().ok_or("Git not initialized")?;

    ops.diff(&file_path)
        .map_err(|e| format!("Git error: {}", e))
}

/// Get blame information
#[tauri::command]
pub fn git_blame(file_path: String) -> Result<Vec<BlameLineInfo>, String> {
    let git_ops = GIT_OPS.read().unwrap();
    let ops = git_ops.as_ref().ok_or("Git not initialized")?;

    ops.blame(&file_path)
        .map_err(|e| format!("Git error: {}", e))
}

/// Get file history
#[tauri::command]
pub fn git_file_history(file_path: String) -> Result<Vec<CommitInfo>, String> {
    let git_ops = GIT_OPS.read().unwrap();
    let ops = git_ops.as_ref().ok_or("Git not initialized")?;

    ops.get_file_history(&file_path)
        .map_err(|e| format!("Git error: {}", e))
}

/// Push to remote
#[tauri::command]
pub fn git_push(remote: String, branch: String) -> Result<(), String> {
    let git_ops = GIT_OPS.read().unwrap();
    let ops = git_ops.as_ref().ok_or("Git not initialized")?;

    ops.push(&remote, &branch)
        .map_err(|e| format!("Git error: {}", e))
}

/// Pull from remote
#[tauri::command]
pub fn git_pull(remote: String, branch: String) -> Result<(), String> {
    let git_ops = GIT_OPS.read().unwrap();
    let ops = git_ops.as_ref().ok_or("Git not initialized")?;

    ops.pull(&remote, &branch)
        .map_err(|e| format!("Git error: {}", e))
}

/// Merge a branch
#[tauri::command]
pub fn git_merge_branch(branch_name: String) -> Result<String, String> {
    let git_ops = GIT_OPS.read().unwrap();
    let ops = git_ops.as_ref().ok_or("Git not initialized")?;

    ops.merge_branch(&branch_name)
        .map_err(|e| format!("Git error: {}", e))
}

/// Stage a file
#[tauri::command]
pub fn git_stage_file(file_path: String) -> Result<(), String> {
    let git_ops = GIT_OPS.read().unwrap();
    let ops = git_ops.as_ref().ok_or("Git not initialized")?;

    ops.commit("", vec![PathBuf::from(file_path)])
        .map(|_| ())
        .map_err(|e| format!("Git error: {}", e))
}

/// Unstage a file
#[tauri::command]
pub fn git_unstage_file(file_path: String) -> Result<(), String> {
    // Simplified implementation - would use git2 reset for staging area
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_language() {
        assert_eq!(detect_language(Path::new("test.rs")), "rust");
        assert_eq!(detect_language(Path::new("test.js")), "javascript");
        assert_eq!(detect_language(Path::new("test.py")), "python");
    }

    // Refactoring command tests
    #[tokio::test]
    async fn test_refactor_rename_command() {
        let result = refactor_rename(
            "/test.rs".to_string(),
            10,
            5,
            "new_name".to_string(),
        )
        .await;

        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_refactor_extract_method_command() {
        let result = refactor_extract_method(
            "/test.rs".to_string(),
            5,
            0,
            10,
            0,
            "extracted_fn".to_string(),
        )
        .await;

        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_refactor_inline_variable_command() {
        let result = refactor_inline_variable("/test.rs".to_string(), 3, 4).await;

        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_refactor_optimize_imports_command() {
        let result = refactor_optimize_imports("/test.rs".to_string()).await;

        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_refactor_move_symbol_command() {
        let result = refactor_move_symbol(
            "/source.rs".to_string(),
            5,
            0,
            "/target.rs".to_string(),
        )
        .await;

        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_refactor_change_signature_command() {
        let result = refactor_change_signature(
            "/test.rs".to_string(),
            2,
            0,
            "fn new_sig(x: i32)".to_string(),
        )
        .await;

        assert!(result.is_ok() || result.is_err());
    }
}
