//! Tauri command bindings for WASM frontend
//!
//! Pure Rust implementation - no JavaScript required!
//! Uses wasm-bindgen to directly call Tauri's window.__TAURI__.core.invoke

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileNode {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub children: Option<Vec<FileNode>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileMetadata {
    pub size: u64,
    pub modified: Option<u64>,
    pub is_readonly: bool,
}

// ✅ IntelliJ Pro: Symbol indexing types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SymbolKind {
    Function,
    Struct,
    Enum,
    Trait,
    Impl,
    Const,
    Static,
    Module,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub file_path: String,
    pub line_number: usize,
    pub signature: Option<String>,
}

// ✅ Parallel Syntax Highlighting types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighlightedToken {
    pub text: String,
    pub color: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighlightResult {
    pub line_number: usize,
    pub tokens: Vec<HighlightedToken>,
}

// ✅ Streaming file types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChunk {
    pub start_line: usize,
    pub end_line: usize,
    pub lines: Vec<String>,
    pub is_final: bool,
    pub total_lines: Option<usize>,
}

// ========================================
// Tauri Invoke Bridge (via berry_invoke)
// ========================================

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    /// Call Tauri commands via the berry_invoke bridge defined in index.html
    /// This is more reliable than accessing __TAURI__ directly
    /// The `catch` attribute makes this return Result<JsValue, JsValue> to handle JavaScript exceptions
    #[wasm_bindgen(js_name = berry_invoke, catch)]
    async fn tauri_invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

// ========================================
// is_tauri_context - berry_invoke Check
// ========================================

/// Check if running in Tauri context
/// Returns false in test environment (wasm-bindgen-test)
pub fn is_tauri_context() -> bool {
    // ✅ Check if window.berry_invoke exists to detect Tauri environment
    // In WASM test environment (wasm-bindgen-test), this will be false
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        if let Some(window) = web_sys::window() {
            // Check for berry_invoke bridge function
            let js_val = js_sys::Reflect::get(&window, &"berry_invoke".into()).ok();
            if js_val.is_some() && !js_val.unwrap().is_undefined() {
                return true;
            }
            // Fallback: Check for __TAURI_INTERNALS__ (Tauri v2)
            let js_val = js_sys::Reflect::get(&window, &"__TAURI_INTERNALS__".into()).ok();
            return js_val.is_some() && !js_val.unwrap().is_undefined();
        }
        false
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        false
    }
}

// ========================================
// File Operations
// ========================================

/// Get current working directory
#[cfg(target_arch = "wasm32")]
pub async fn get_current_dir() -> Result<String, String> {
    if !is_tauri_context() {
        // Web mode: return a default path
        return Ok(".".to_string());
    }

    let args = serde_wasm_bindgen::to_value(&serde_json::json!({}))
        .map_err(|e| format!("Failed to serialize args: {}", e))?;

    let result = tauri_invoke("get_current_dir", args)
        .await
        .map_err(|e| format!("Failed to get current directory: {:?}", e))?;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Failed to deserialize result: {}", e))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn get_current_dir() -> Result<String, String> {
    Err("get_current_dir only available in WASM context".to_string())
}

/// Read file contents
#[cfg(target_arch = "wasm32")]
pub async fn read_file(path: &str) -> Result<String, String> {
    if !is_tauri_context() {
        // Web mode: return dummy data to prevent infinite retry loops
        return Ok(format!(
            "// Content of {}\n// (Running in Web Mode - Tauri integration pending)",
            path
        ));
    }

    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "path": path
    }))
    .map_err(|e| format!("Failed to serialize args: {}", e))?;

    let result = tauri_invoke("read_file", args)
        .await
        .map_err(|e| format!("Failed to read file: {:?}", e))?;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Failed to deserialize result: {}", e))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn read_file(_path: &str) -> Result<String, String> {
    Err("read_file only available in WASM context".to_string())
}

/// ✅ IntelliJ Pro: Read file with partial loading (first N bytes only)
/// Returns: (content, is_partial, total_size)
#[cfg(target_arch = "wasm32")]
pub async fn read_file_partial(
    path: &str,
    max_bytes: Option<usize>,
) -> Result<(String, bool, u64), String> {
    if !is_tauri_context() {
        return Ok((format!("// Web mode: {}", path), false, 0));
    }

    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "path": path,
        "maxBytes": max_bytes
    }))
    .map_err(|e| format!("Failed to serialize args: {}", e))?;

    let result = tauri_invoke("read_file_partial", args)
        .await
        .map_err(|e| format!("Failed to read file partial: {:?}", e))?;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Failed to deserialize result: {}", e))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn read_file_partial(
    _path: &str,
    _max_bytes: Option<usize>,
) -> Result<(String, bool, u64), String> {
    Err("read_file_partial only available in WASM context".to_string())
}

/// ✅ IntelliJ Pro: Read file chunk (for streaming large files)
#[cfg(target_arch = "wasm32")]
pub async fn read_file_chunk(path: &str, offset: u64, length: usize) -> Result<String, String> {
    if !is_tauri_context() {
        return Ok(format!("// Web mode chunk"));
    }

    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "path": path,
        "offset": offset,
        "length": length
    }))
    .map_err(|e| format!("Failed to serialize args: {}", e))?;

    let result = tauri_invoke("read_file_chunk", args)
        .await
        .map_err(|e| format!("Failed to read file chunk: {:?}", e))?;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Failed to deserialize result: {}", e))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn read_file_chunk(_path: &str, _offset: u64, _length: usize) -> Result<String, String> {
    Err("read_file_chunk only available in WASM context".to_string())
}

/// Write file contents
#[cfg(target_arch = "wasm32")]
pub async fn write_file(path: &str, contents: &str) -> Result<(), String> {
    if !is_tauri_context() {
        // Web mode: silently succeed to prevent error loops
        return Ok(());
    }

    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "path": path,
        "contents": contents
    }))
    .map_err(|e| format!("Failed to serialize args: {}", e))?;

    tauri_invoke("write_file", args)
        .await
        .map_err(|e| format!("Failed to write file: {:?}", e))?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn write_file(_path: &str, _contents: &str) -> Result<(), String> {
    Err("write_file only available in WASM context".to_string())
}

/// Select folder using native file picker
#[cfg(target_arch = "wasm32")]
pub async fn select_folder() -> Result<Option<String>, String> {
    if !is_tauri_context() {
        // Web mode: not supported
        return Err("Folder selection not available in web mode".to_string());
    }

    let args = serde_wasm_bindgen::to_value(&serde_json::json!({}))
        .map_err(|e| format!("Failed to serialize args: {}", e))?;

    let result = tauri_invoke("select_folder", args)
        .await
        .map_err(|e| format!("Failed to select folder: {:?}", e))?;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Failed to deserialize result: {}", e))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn select_folder() -> Result<Option<String>, String> {
    Err("select_folder only available in WASM context".to_string())
}

/// Read directory contents
#[cfg(target_arch = "wasm32")]
pub async fn read_dir(path: &str, max_depth: Option<usize>) -> Result<Vec<FileNode>, String> {
    if !is_tauri_context() {
        // Web mode: return empty directory to prevent error loops
        return Ok(Vec::new());
    }

    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "path": path,
        "maxDepth": max_depth
    }))
    .map_err(|e| format!("Failed to serialize args: {}", e))?;

    let result = tauri_invoke("read_dir", args)
        .await
        .map_err(|e| format!("Failed to read directory: {:?}", e))?;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Failed to deserialize result: {}", e))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn read_dir(_path: &str, _max_depth: Option<usize>) -> Result<Vec<FileNode>, String> {
    Err("read_dir only available in WASM context".to_string())
}

/// Create a new file
#[cfg(target_arch = "wasm32")]
pub async fn create_file(path: &str, contents: Option<String>) -> Result<(), String> {
    if !is_tauri_context() {
        // Web mode: silently succeed
        return Ok(());
    }

    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "path": path,
        "contents": contents
    }))
    .map_err(|e| format!("Failed to serialize args: {}", e))?;

    tauri_invoke("create_file", args)
        .await
        .map_err(|e| format!("Failed to create file: {:?}", e))?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn create_file(_path: &str, _contents: Option<String>) -> Result<(), String> {
    Err("create_file only available in WASM context".to_string())
}

/// Delete a file or directory
#[cfg(target_arch = "wasm32")]
pub async fn delete_file(path: &str) -> Result<(), String> {
    if !is_tauri_context() {
        // Web mode: silently succeed
        return Ok(());
    }

    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "path": path
    }))
    .map_err(|e| format!("Failed to serialize args: {}", e))?;

    tauri_invoke("delete_file", args)
        .await
        .map_err(|e| format!("Failed to delete file: {:?}", e))?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn delete_file(_path: &str) -> Result<(), String> {
    Err("delete_file only available in WASM context".to_string())
}

/// Rename/move a file or directory
#[cfg(target_arch = "wasm32")]
pub async fn rename_file(old_path: &str, new_path: &str) -> Result<(), String> {
    if !is_tauri_context() {
        // Web mode: silently succeed
        return Ok(());
    }

    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "oldPath": old_path,
        "newPath": new_path
    }))
    .map_err(|e| format!("Failed to serialize args: {}", e))?;

    tauri_invoke("rename_file", args)
        .await
        .map_err(|e| format!("Failed to rename file: {:?}", e))?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn rename_file(_old_path: &str, _new_path: &str) -> Result<(), String> {
    Err("rename_file only available in WASM context".to_string())
}

/// Get file metadata
#[cfg(target_arch = "wasm32")]
pub async fn get_file_metadata(path: &str) -> Result<FileMetadata, String> {
    if !is_tauri_context() {
        // Web mode: return dummy metadata
        return Ok(FileMetadata {
            size: 0,
            modified: None,
            is_readonly: false,
        });
    }

    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "path": path
    }))
    .map_err(|e| format!("Failed to serialize args: {}", e))?;

    let result = tauri_invoke("get_file_metadata", args)
        .await
        .map_err(|e| format!("Failed to get file metadata: {:?}", e))?;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Failed to deserialize result: {}", e))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn get_file_metadata(_path: &str) -> Result<FileMetadata, String> {
    Err("get_file_metadata only available in WASM context".to_string())
}

// ========================================
// ✅ IntelliJ Pro: Background Indexing
// ========================================

/// Index all Rust files in a workspace directory
#[cfg(target_arch = "wasm32")]
pub async fn index_workspace(path: &str) -> Result<usize, String> {
    if !is_tauri_context() {
        return Ok(0);
    }

    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "path": path
    }))
    .map_err(|e| format!("Failed to serialize args: {}", e))?;

    let result = tauri_invoke("index_workspace", args)
        .await
        .map_err(|e| format!("Failed to index workspace: {:?}", e))?;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Failed to deserialize result: {}", e))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn index_workspace(_path: &str) -> Result<usize, String> {
    Err("index_workspace only available in WASM context".to_string())
}

/// Search for symbols by query string
#[cfg(target_arch = "wasm32")]
pub async fn search_symbols(query: &str) -> Result<Vec<Symbol>, String> {
    if !is_tauri_context() {
        return Ok(Vec::new());
    }

    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "query": query
    }))
    .map_err(|e| format!("Failed to serialize args: {}", e))?;

    let result = tauri_invoke("search_symbols", args)
        .await
        .map_err(|e| format!("Failed to search symbols: {:?}", e))?;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Failed to deserialize result: {}", e))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn search_symbols(_query: &str) -> Result<Vec<Symbol>, String> {
    Err("search_symbols only available in WASM context".to_string())
}

/// Update index for a single file (incremental indexing after edits)
#[cfg(target_arch = "wasm32")]
pub async fn index_file(path: &str, content: &str) -> Result<(), String> {
    if !is_tauri_context() {
        return Ok(());
    }

    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "path": path,
        "content": content
    }))
    .map_err(|e| format!("Failed to serialize args: {}", e))?;

    tauri_invoke("index_file", args)
        .await
        .map_err(|e| format!("Failed to index file: {:?}", e))?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn index_file(_path: &str, _content: &str) -> Result<(), String> {
    Err("index_file only available in WASM context".to_string())
}

/// Get total symbol count (for UI status display)
#[cfg(target_arch = "wasm32")]
pub async fn get_symbol_count() -> Result<usize, String> {
    if !is_tauri_context() {
        return Ok(0);
    }

    let args = serde_wasm_bindgen::to_value(&serde_json::json!({}))
        .map_err(|e| format!("Failed to serialize args: {}", e))?;

    let result = tauri_invoke("get_symbol_count", args)
        .await
        .map_err(|e| format!("Failed to get symbol count: {:?}", e))?;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Failed to deserialize result: {}", e))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn get_symbol_count() -> Result<usize, String> {
    Err("get_symbol_count only available in WASM context".to_string())
}

// ========================================
// ✅ Parallel Syntax Highlighting
// ========================================

/// Highlight multiple lines in parallel using rayon (Tauri backend)
#[cfg(target_arch = "wasm32")]
pub async fn highlight_file_parallel(
    file_path: &str,
    lines: Vec<(usize, String)>,
) -> Result<Vec<HighlightResult>, String> {
    if !is_tauri_context() {
        return Ok(Vec::new());
    }

    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "filePath": file_path,
        "lines": lines
    }))
    .map_err(|e| format!("Failed to serialize args: {}", e))?;

    let result = tauri_invoke("highlight_file_parallel", args)
        .await
        .map_err(|e| format!("Failed to highlight file: {:?}", e))?;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Failed to deserialize result: {}", e))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn highlight_file_parallel(
    _file_path: &str,
    _lines: Vec<(usize, String)>,
) -> Result<Vec<HighlightResult>, String> {
    Err("highlight_file_parallel only available in WASM context".to_string())
}

/// Clear syntax highlighting cache for a file
#[cfg(target_arch = "wasm32")]
pub async fn invalidate_syntax_cache(file_path: &str) -> Result<(), String> {
    if !is_tauri_context() {
        return Ok(());
    }

    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "filePath": file_path
    }))
    .map_err(|e| format!("Failed to serialize args: {}", e))?;

    tauri_invoke("invalidate_syntax_cache", args)
        .await
        .map_err(|e| format!("Failed to invalidate syntax cache: {:?}", e))?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn invalidate_syntax_cache(_file_path: &str) -> Result<(), String> {
    Err("invalidate_syntax_cache only available in WASM context".to_string())
}

/// Get syntax highlighting cache statistics (size, capacity)
#[cfg(target_arch = "wasm32")]
pub async fn get_syntax_cache_stats() -> Result<(usize, usize), String> {
    if !is_tauri_context() {
        return Ok((0, 0));
    }

    let args = serde_wasm_bindgen::to_value(&serde_json::json!({}))
        .map_err(|e| format!("Failed to serialize args: {}", e))?;

    let result = tauri_invoke("get_syntax_cache_stats", args)
        .await
        .map_err(|e| format!("Failed to get cache stats: {:?}", e))?;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Failed to deserialize result: {}", e))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn get_syntax_cache_stats() -> Result<(usize, usize), String> {
    Err("get_syntax_cache_stats only available in WASM context".to_string())
}

// ========================================
// ✅ Streaming File Operations
// ========================================

/// Stream large file (chunks sent via Tauri events)
#[cfg(target_arch = "wasm32")]
pub async fn stream_large_file(file_path: &str) -> Result<(), String> {
    if !is_tauri_context() {
        return Ok(());
    }

    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "path": file_path
    }))
    .map_err(|e| format!("Failed to serialize args: {}", e))?;

    tauri_invoke("stream_large_file", args)
        .await
        .map_err(|e| format!("Failed to stream file: {:?}", e))?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn stream_large_file(_file_path: &str) -> Result<(), String> {
    Err("stream_large_file only available in WASM context".to_string())
}

/// Read file with automatic streaming detection (> 1MB uses streaming)
#[cfg(target_arch = "wasm32")]
pub async fn read_file_auto(file_path: &str) -> Result<String, String> {
    if !is_tauri_context() {
        return Ok(format!("// Auto-read: {}", file_path));
    }

    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "path": file_path
    }))
    .map_err(|e| format!("Failed to serialize args: {}", e))?;

    let result = tauri_invoke("read_file_auto", args)
        .await
        .map_err(|e| format!("Failed to read file auto: {:?}", e))?;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Failed to deserialize result: {}", e))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn read_file_auto(_file_path: &str) -> Result<String, String> {
    Err("read_file_auto only available in WASM context".to_string())
}
