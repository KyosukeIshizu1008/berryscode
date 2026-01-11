//! LSP (Language Server Protocol) Client Implementation
//! 100% Rust - No JavaScript!

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub uri: String,
    pub range: Range,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionItem {
    pub label: String,
    pub kind: u32,
    pub detail: Option<String>,
    pub documentation: Option<String>,
    pub insert_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub range: Range,
    pub severity: u32,
    pub message: String,
    pub source: Option<String>,
}

#[derive(Clone)]
pub struct LspClient {
    language: String,
    server_url: Option<String>,
    /// ✅ FIX: Offline mode flag - set when LSP initialization fails
    /// When true, all LSP operations return empty results without attempting connection
    offline_mode: bool,
}

impl LspClient {
    pub fn new(language: &str) -> Self {
        Self {
            language: language.to_string(),
            server_url: None,
            offline_mode: false,
        }
    }

    /// Check if LSP is in offline mode (connection failed)
    pub fn is_offline(&self) -> bool {
        self.offline_mode
    }

    pub async fn initialize(&mut self, root_uri: String) -> Result<(), String> {
        // ✅ FIX: If already in offline mode, skip initialization
        if self.offline_mode {
            return Err("LSP is in offline mode".to_string());
        }

        // Initialize LSP connection via Tauri
        #[wasm_bindgen]
        extern "C" {
            #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
            async fn tauri_invoke(cmd: &str, args: JsValue) -> JsValue;
        }

        let args = serde_wasm_bindgen::to_value(&serde_json::json!({
            "language": self.language,
            "rootUri": root_uri
        }))
        .map_err(|e| format!("Failed to serialize args: {}", e))?;

        let result = tauri_invoke("lsp_initialize", args).await;

        // ✅ FIX: Check for undefined/null (success) or error strings (EOF)
        // Tauri commands that return Result<(), String> send null on Ok(())
        if result.is_undefined() || result.is_null() {
            #[cfg(target_arch = "wasm32")]
            leptos::logging::log!("✅ LSP initialized successfully for {}", self.language);
            return Ok(());
        }

        // ✅ FIX: Check if the result is an error string (e.g., "LSP server closed connection (EOF)")
        if let Some(err_str) = result.as_string() {
            if err_str.contains("closed connection") || err_str.contains("EOF") || err_str.contains("error") {
                self.offline_mode = true;
                #[cfg(target_arch = "wasm32")]
                leptos::logging::warn!("🔌 LSP connection failed: {}. Entering offline mode.", err_str);
                return Err(err_str);
            }
        }

        // ✅ FIX: Try to parse as bool for backwards compatibility
        match serde_wasm_bindgen::from_value::<bool>(result.clone()) {
            Ok(true) => {
                #[cfg(target_arch = "wasm32")]
                leptos::logging::log!("✅ LSP initialized (bool=true) for {}", self.language);
                Ok(())
            }
            Ok(false) => {
                self.offline_mode = true;
                #[cfg(target_arch = "wasm32")]
                leptos::logging::warn!("🔌 LSP initialization failed (bool=false), entering offline mode");
                Err(format!("LSP initialization failed for {}", self.language))
            }
            Err(e) => {
                self.offline_mode = true;
                #[cfg(target_arch = "wasm32")]
                leptos::logging::warn!("🔌 LSP connection error: {:?}. Entering offline mode.", e);
                Err(format!("LSP initialization error: {}", e))
            }
        }
    }

    /// Get code completions at cursor position
    pub async fn get_completions(
        &self,
        file_path: String,
        position: Position,
    ) -> Result<Vec<CompletionItem>, String> {
        // ✅ FIX: Return empty completions in offline mode
        if self.offline_mode {
            return Ok(Vec::new());
        }

        #[wasm_bindgen]
        extern "C" {
            #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
            async fn tauri_invoke(cmd: &str, args: JsValue) -> JsValue;
        }

        let args = serde_wasm_bindgen::to_value(&serde_json::json!({
            "filePath": file_path,
            "line": position.line,
            "character": position.character
        }))
        .map_err(|e| format!("Failed to serialize args: {}", e))?;

        let result = tauri_invoke("lsp_completion", args).await;
        serde_wasm_bindgen::from_value(result)
            .map_err(|e| format!("Failed to deserialize completions: {}", e))
    }

    /// Go to definition
    pub async fn goto_definition(
        &self,
        file_path: String,
        position: Position,
    ) -> Result<Vec<Location>, String> {
        // ✅ FIX: Return empty locations in offline mode
        if self.offline_mode {
            return Ok(Vec::new());
        }

        #[wasm_bindgen]
        extern "C" {
            #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
            async fn tauri_invoke(cmd: &str, args: JsValue) -> JsValue;
        }

        let args = serde_wasm_bindgen::to_value(&serde_json::json!({
            "filePath": file_path,
            "line": position.line,
            "character": position.character
        }))
        .map_err(|e| format!("Failed to serialize args: {}", e))?;

        let result = tauri_invoke("lsp_goto_definition", args).await;
        serde_wasm_bindgen::from_value(result)
            .map_err(|e| format!("Failed to deserialize locations: {}", e))
    }

    /// Find all references
    pub async fn find_references(
        &self,
        file_path: String,
        position: Position,
        include_declaration: bool,
    ) -> Result<Vec<Location>, String> {
        // ✅ FIX: Return empty references in offline mode
        if self.offline_mode {
            return Ok(Vec::new());
        }

        #[wasm_bindgen]
        extern "C" {
            #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
            async fn tauri_invoke(cmd: &str, args: JsValue) -> JsValue;
        }

        let args = serde_wasm_bindgen::to_value(&serde_json::json!({
            "filePath": file_path,
            "line": position.line,
            "character": position.character,
            "includeDeclaration": include_declaration
        }))
        .map_err(|e| format!("Failed to serialize args: {}", e))?;

        let result = tauri_invoke("lsp_find_references", args).await;
        serde_wasm_bindgen::from_value(result)
            .map_err(|e| format!("Failed to deserialize references: {}", e))
    }

    /// Get diagnostics (errors/warnings)
    pub async fn get_diagnostics(&self, file_path: String) -> Result<Vec<Diagnostic>, String> {
        // ✅ FIX: Return empty diagnostics in offline mode
        if self.offline_mode {
            return Ok(Vec::new());
        }

        #[wasm_bindgen]
        extern "C" {
            #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
            async fn tauri_invoke(cmd: &str, args: JsValue) -> JsValue;
        }

        let args = serde_wasm_bindgen::to_value(&serde_json::json!({
            "filePath": file_path
        }))
        .map_err(|e| format!("Failed to serialize args: {}", e))?;

        let result = tauri_invoke("lsp_diagnostics", args).await;
        serde_wasm_bindgen::from_value(result)
            .map_err(|e| format!("Failed to deserialize diagnostics: {}", e))
    }

    /// Rename symbol
    pub async fn rename(
        &self,
        file_path: String,
        position: Position,
        new_name: String,
    ) -> Result<(), String> {
        // ✅ FIX: Return error in offline mode
        if self.offline_mode {
            return Err("LSP is offline - rename unavailable".to_string());
        }

        #[wasm_bindgen]
        extern "C" {
            #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
            async fn tauri_invoke(cmd: &str, args: JsValue) -> JsValue;
        }

        let args = serde_wasm_bindgen::to_value(&serde_json::json!({
            "filePath": file_path,
            "line": position.line,
            "character": position.character,
            "newName": new_name
        }))
        .map_err(|e| format!("Failed to serialize args: {}", e))?;

        let result = tauri_invoke("lsp_rename", args).await;
        serde_wasm_bindgen::from_value::<bool>(result)
            .map(|_| ())
            .map_err(|e| format!("Failed to rename: {}", e))
    }

    /// Format document
    pub async fn format_document(&self, file_path: String) -> Result<String, String> {
        // ✅ FIX: Return error in offline mode
        if self.offline_mode {
            return Err("LSP is offline - formatting unavailable".to_string());
        }

        #[wasm_bindgen]
        extern "C" {
            #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
            async fn tauri_invoke(cmd: &str, args: JsValue) -> JsValue;
        }

        let args = serde_wasm_bindgen::to_value(&serde_json::json!({
            "filePath": file_path
        }))
        .map_err(|e| format!("Failed to serialize args: {}", e))?;

        let result = tauri_invoke("lsp_format", args).await;
        serde_wasm_bindgen::from_value(result)
            .map_err(|e| format!("Failed to format document: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_position_creation() {
        let pos = Position { line: 10, character: 5 };
        assert_eq!(pos.line, 10);
        assert_eq!(pos.character, 5);
    }

    #[wasm_bindgen_test]
    fn test_range_creation() {
        let start = Position { line: 1, character: 0 };
        let end = Position { line: 1, character: 10 };
        let range = Range { start, end };

        assert_eq!(range.start.line, 1);
        assert_eq!(range.end.character, 10);
    }

    #[wasm_bindgen_test]
    fn test_location_creation() {
        let location = Location {
            uri: "file:///path/to/file.rs".to_string(),
            range: Range {
                start: Position { line: 0, character: 0 },
                end: Position { line: 0, character: 5 },
            },
        };

        assert_eq!(location.uri, "file:///path/to/file.rs");
        assert_eq!(location.range.start.line, 0);
    }

    #[wasm_bindgen_test]
    fn test_completion_item_creation() {
        let item = CompletionItem {
            label: "println!".to_string(),
            kind: 3, // Function
            detail: Some("macro".to_string()),
            documentation: Some("Prints to stdout".to_string()),
            insert_text: Some("println!(\"{}\", )".to_string()),
        };

        assert_eq!(item.label, "println!");
        assert_eq!(item.kind, 3);
        assert!(item.detail.is_some());
        assert!(item.documentation.is_some());
        assert!(item.insert_text.is_some());
    }

    #[wasm_bindgen_test]
    fn test_completion_item_minimal() {
        let item = CompletionItem {
            label: "foo".to_string(),
            kind: 1,
            detail: None,
            documentation: None,
            insert_text: None,
        };

        assert_eq!(item.label, "foo");
        assert!(item.detail.is_none());
    }

    #[wasm_bindgen_test]
    fn test_diagnostic_creation() {
        let diag = Diagnostic {
            range: Range {
                start: Position { line: 10, character: 5 },
                end: Position { line: 10, character: 15 },
            },
            severity: 1, // Error
            message: "unused variable: `x`".to_string(),
            source: Some("rust-analyzer".to_string()),
        };

        assert_eq!(diag.severity, 1);
        assert_eq!(diag.message, "unused variable: `x`");
        assert_eq!(diag.source, Some("rust-analyzer".to_string()));
    }

    #[wasm_bindgen_test]
    fn test_lsp_client_creation() {
        let client = LspClient::new("rust");
        assert_eq!(client.language, "rust");
        assert!(client.server_url.is_none());
    }

    #[wasm_bindgen_test]
    fn test_position_serialization() {
        let pos = Position { line: 5, character: 10 };
        let json = serde_json::to_string(&pos).unwrap();
        assert!(json.contains("\"line\":5"));
        assert!(json.contains("\"character\":10"));
    }

    #[wasm_bindgen_test]
    fn test_range_serialization() {
        let range = Range {
            start: Position { line: 1, character: 0 },
            end: Position { line: 1, character: 10 },
        };
        let json = serde_json::to_string(&range).unwrap();
        assert!(json.contains("\"start\""));
        assert!(json.contains("\"end\""));
    }
}
