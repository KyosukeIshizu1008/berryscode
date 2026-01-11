//! Tauri bindings for search functionality
//!
//! Pure Rust implementation - no JavaScript required!
//! Uses wasm-bindgen to directly call Tauri's window.__TAURI__.core.invoke

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub path: String,
    pub line_number: usize,
    pub column: usize,
    pub line_text: String,
    pub match_start: usize,
    pub match_end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchOptions {
    pub case_sensitive: bool,
    pub use_regex: bool,
    pub whole_word: bool,
    pub include_pattern: Option<String>,
    pub exclude_pattern: Option<String>,
    pub max_results: Option<usize>,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            case_sensitive: false,
            use_regex: false,
            whole_word: false,
            include_pattern: None,
            exclude_pattern: None,
            max_results: Some(1000),
        }
    }
}

// ========================================
// Direct Tauri Invoke Binding (100% Rust)
// ========================================

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    /// Direct access to Tauri's invoke function: window.__TAURI__.core.invoke
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
    async fn tauri_invoke(cmd: &str, args: JsValue) -> JsValue;
}

/// Search in files using Tauri command
#[cfg(target_arch = "wasm32")]
pub async fn search_in_files(
    query: &str,
    root_path: &str,
    options: Option<SearchOptions>,
) -> Result<Vec<SearchResult>, String> {
    if !crate::tauri_bindings::is_tauri_context() {
        return Err("Not running in Tauri context".to_string());
    }

    let opts = options.unwrap_or_default();
    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "query": query,
        "rootPath": root_path,
        "options": opts
    }))
    .map_err(|e| format!("Failed to serialize options: {}", e))?;

    let result = tauri_invoke("search_in_files", args).await;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Failed to deserialize results: {}", e))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn search_in_files(
    _query: &str,
    _root_path: &str,
    _options: Option<SearchOptions>,
) -> Result<Vec<SearchResult>, String> {
    Err("search_in_files only available in WASM context".to_string())
}
