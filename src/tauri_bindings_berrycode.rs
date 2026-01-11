//! Tauri bindings for BerryCode commands
//!
//! This module provides Rust wrappers around Tauri commands for BerryCode AI functionality.

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub provider: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContext {
    pub project_root: String,
    pub files: Vec<String>,
    pub git_status: Option<GitStatus>,
    pub diagnostics: Option<Vec<DiagnosticInfo>>,
    pub symbols: Option<Vec<SymbolInfo>>,
    pub recent_files: Option<Vec<String>>,
    pub file_stats: Option<FileStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatus {
    pub branch: String,
    pub uncommitted_changes: usize,
    pub untracked_files: usize,
    pub ahead: usize,
    pub behind: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticInfo {
    pub file_path: String,
    pub line: u32,
    pub column: u32,
    pub severity: String,
    pub message: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: String,
    pub file_path: String,
    pub line: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileStats {
    pub total: usize,
    pub rust: usize,
    pub javascript: usize,
    pub typescript: usize,
    pub python: usize,
    pub other: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub success: bool,
    pub output: String,
    pub errors: Vec<CompilationError>,
    pub warnings: Vec<CompilationWarning>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilationError {
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub message: String,
    pub code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilationWarning {
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSessionData {
    pub id: String,
    pub session_id: String,
    pub title: String,
    pub created_at: String,
    pub last_message_at: String,
    pub message_count: i32,
}

/// Initialize BerryCode session
pub async fn berrycode_init(
    model: Option<String>,
    mode: Option<String>,
    project_root: Option<String>,
) -> Result<String, String> {
    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
        async fn invoke(cmd: &str, args: JsValue) -> JsValue;
    }

    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "model": model,
        "mode": mode,
        "projectRoot": project_root,
    }))
    .map_err(|e| format!("Serialization error: {}", e))?;

    let result = invoke("berrycode_init", args).await;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Deserialization error: {}", e))
}

/// Send chat message
pub async fn berrycode_chat(message: String) -> Result<String, String> {
    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
        async fn invoke(cmd: &str, args: JsValue) -> JsValue;
    }

    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "message": message,
    }))
    .map_err(|e| format!("Serialization error: {}", e))?;

    let result = invoke("berrycode_chat", args).await;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Deserialization error: {}", e))
}

/// Add file to context
pub async fn berrycode_add_file(file_path: String) -> Result<String, String> {
    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
        async fn invoke(cmd: &str, args: JsValue) -> JsValue;
    }

    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "filePath": file_path,
    }))
    .map_err(|e| format!("Serialization error: {}", e))?;

    let result = invoke("berrycode_add_file", args).await;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Deserialization error: {}", e))
}

/// Remove file from context
pub async fn berrycode_drop_file(file_path: String) -> Result<String, String> {
    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
        async fn invoke(cmd: &str, args: JsValue) -> JsValue;
    }

    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "filePath": file_path,
    }))
    .map_err(|e| format!("Serialization error: {}", e))?;

    let result = invoke("berrycode_drop_file", args).await;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Deserialization error: {}", e))
}

/// List context files
pub async fn berrycode_list_files() -> Result<Vec<String>, String> {
    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
        async fn invoke(cmd: &str, args: JsValue) -> JsValue;
    }

    let result = invoke("berrycode_list_files", JsValue::NULL).await;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Deserialization error: {}", e))
}

/// List available models
pub async fn berrycode_list_models() -> Result<Vec<ModelInfo>, String> {
    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
        async fn invoke(cmd: &str, args: JsValue) -> JsValue;
    }

    let result = invoke("berrycode_list_models", JsValue::NULL).await;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Deserialization error: {}", e))
}

/// Get chat history
pub async fn berrycode_get_history() -> Result<Vec<ChatMessage>, String> {
    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
        async fn invoke(cmd: &str, args: JsValue) -> JsValue;
    }

    let result = invoke("berrycode_get_history", JsValue::NULL).await;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Deserialization error: {}", e))
}

/// Clear chat history
pub async fn berrycode_clear_history() -> Result<String, String> {
    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
        async fn invoke(cmd: &str, args: JsValue) -> JsValue;
    }

    let result = invoke("berrycode_clear_history", JsValue::NULL).await;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Deserialization error: {}", e))
}

/// Get comprehensive project context for AI (RAG)
pub async fn berrycode_get_context() -> Result<ProjectContext, String> {
    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
        async fn invoke(cmd: &str, args: JsValue) -> JsValue;
    }

    let result = invoke("berrycode_get_context", JsValue::NULL).await;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Deserialization error: {}", e))
}

/// Run cargo check and get structured results
pub async fn berrycode_cargo_check() -> Result<TestResult, String> {
    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
        async fn invoke(cmd: &str, args: JsValue) -> JsValue;
    }

    let result = invoke("berrycode_cargo_check", JsValue::NULL).await;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Deserialization error: {}", e))
}

/// Run cargo test and get results
pub async fn berrycode_cargo_test() -> Result<TestResult, String> {
    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
        async fn invoke(cmd: &str, args: JsValue) -> JsValue;
    }

    let result = invoke("berrycode_cargo_test", JsValue::NULL).await;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Deserialization error: {}", e))
}

/// Create a new chat session
pub async fn berrycode_create_chat_session(title: Option<String>) -> Result<String, String> {
    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
        async fn invoke(cmd: &str, args: JsValue) -> JsValue;
    }

    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "title": title,
    }))
    .map_err(|e| format!("Serialization error: {}", e))?;

    let result = invoke("berrycode_create_chat_session", args).await;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Deserialization error: {}", e))
}

/// List all chat sessions for current project
pub async fn berrycode_list_chat_sessions() -> Result<Vec<ChatSessionData>, String> {
    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
        async fn invoke(cmd: &str, args: JsValue) -> JsValue;
    }

    let result = invoke("berrycode_list_chat_sessions", JsValue::NULL).await;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Deserialization error: {}", e))
}

/// Load messages for a specific chat session
pub async fn berrycode_load_chat_messages(chat_session_id: String) -> Result<Vec<ChatMessage>, String> {
    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
        async fn invoke(cmd: &str, args: JsValue) -> JsValue;
    }

    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "chatSessionId": chat_session_id,
    }))
    .map_err(|e| format!("Serialization error: {}", e))?;

    let result = invoke("berrycode_load_chat_messages", args).await;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Deserialization error: {}", e))
}

/// Save a message (called after sending user message or receiving AI response)
pub async fn berrycode_save_message(chat_session_id: String, role: String, content: String) -> Result<i64, String> {
    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
        async fn invoke(cmd: &str, args: JsValue) -> JsValue;
    }

    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "chatSessionId": chat_session_id,
        "role": role,
        "content": content,
    }))
    .map_err(|e| format!("Serialization error: {}", e))?;

    let result = invoke("berrycode_save_message", args).await;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Deserialization error: {}", e))
}

/// Delete a chat session
pub async fn berrycode_delete_chat_session(chat_session_id: String) -> Result<(), String> {
    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
        async fn invoke(cmd: &str, args: JsValue) -> JsValue;
    }

    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "chatSessionId": chat_session_id,
    }))
    .map_err(|e| format!("Serialization error: {}", e))?;

    let result = invoke("berrycode_delete_chat_session", args).await;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Deserialization error: {}", e))
}

/// Update chat session title
pub async fn berrycode_update_chat_title(chat_session_id: String, title: String) -> Result<(), String> {
    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
        async fn invoke(cmd: &str, args: JsValue) -> JsValue;
    }

    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "chatSessionId": chat_session_id,
        "title": title,
    }))
    .map_err(|e| format!("Serialization error: {}", e))?;

    let result = invoke("berrycode_update_chat_title", args).await;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Deserialization error: {}", e))
}
