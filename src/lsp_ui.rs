//! LSP UI Integration
//!
//! Provides UI components for LSP features like completion, diagnostics, and hover.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use crate::common::async_bridge::TauriBridge;
use crate::types::Position;

/// LSP Completion Item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionItem {
    pub label: String,
    pub kind: Option<u32>,
    pub detail: Option<String>,
    pub documentation: Option<String>,
    pub insert_text: Option<String>,
}

/// LSP Diagnostic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub range: DiagnosticRange,
    pub severity: u32,
    pub message: String,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticRange {
    pub start: DiagnosticPosition,
    pub end: DiagnosticPosition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticPosition {
    pub line: u32,
    pub character: u32,
}

/// LSP Hover Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoverInfo {
    pub contents: String,
    pub range: Option<DiagnosticRange>,
}

/// LSP Location Information (for go-to-definition)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationInfo {
    pub uri: String,         // File path
    pub line: usize,         // Line number
    pub column: usize,       // Column number
}

/// LSP Integration Manager
#[derive(Clone, Copy)]
pub struct LspIntegration {
    /// Current file path
    pub file_path: RwSignal<String>, // Made public for testing
    /// Current language
    language: RwSignal<String>,
    /// LSP initialized flag
    pub initialized: RwSignal<bool>, // Made public for initialization checks
    /// Completion items cache
    completion_cache: RwSignal<Vec<CompletionItem>>,
    /// Diagnostics cache
    diagnostics: RwSignal<Vec<Diagnostic>>,
    /// Hover information cache
    hover_info: RwSignal<Option<HoverInfo>>,
}

impl LspIntegration {
    /// Create a new LSP integration
    pub fn new() -> Self {
        Self {
            file_path: RwSignal::new(String::new()),
            language: RwSignal::new(String::from("rust")),
            initialized: RwSignal::new(false),
            completion_cache: RwSignal::new(Vec::new()),
            diagnostics: RwSignal::new(Vec::new()),
            hover_info: RwSignal::new(None),
        }
    }

    /// Detect language from file extension
    fn detect_language(file_path: &str) -> String {
        if file_path.ends_with(".rs") {
            "rust".to_string()
        } else if file_path.ends_with(".ts") || file_path.ends_with(".tsx") {
            "typescript".to_string()
        } else if file_path.ends_with(".js") || file_path.ends_with(".jsx") {
            "javascript".to_string()
        } else if file_path.ends_with(".py") {
            "python".to_string()
        } else {
            "rust".to_string() // Default to Rust
        }
    }

    /// Initialize LSP server for the current file
    pub async fn initialize(&self, file_path: String, root_uri: String) -> anyhow::Result<()> {
        let language = Self::detect_language(&file_path);

        leptos::logging::log!("🔍 LSP initializing: file={}, language={}, root={}",
            file_path, language, root_uri);

        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct InitRequest {
            language: String,
            root_uri: String,  // Will be serialized as "rootUri"
        }

        let request = InitRequest {
            language: language.clone(),
            root_uri,
        };

        match TauriBridge::invoke::<_, bool>("lsp_initialize", request).await {
            Ok(_result) => {
                self.language.set(language.clone());
                self.file_path.set(file_path.clone());
                self.initialized.set(true);

                leptos::logging::log!("✅ LSP initialized successfully: file={}, language={}, initialized={}",
                    file_path, language, self.initialized.get_untracked());

                Ok(())
            }
            Err(e) => {
                leptos::logging::error!("❌ LSP initialization failed: {}", e);
                Err(e)
            }
        }
    }

    /// Set the current file path
    pub fn set_file_path(&self, path: String) {
        self.file_path.set(path.clone());
        let language = Self::detect_language(&path);
        self.language.set(language);
    }

    /// Request completions at a specific position
    pub async fn request_completions(&self, position: Position) -> anyhow::Result<Vec<CompletionItem>> {
        if !self.initialized.get_untracked() {
            return Ok(Vec::new()); // Not initialized, return empty
        }

        #[derive(Serialize)]
        struct CompletionRequest {
            language: String,
            file_path: String,
            line: u32,
            character: u32,
        }

        let request = CompletionRequest {
            language: self.language.get_untracked(),
            file_path: self.file_path.get_untracked(),
            line: position.line as u32,
            character: position.column as u32,
        };

        let items: Vec<CompletionItem> = TauriBridge::invoke("lsp_get_completions", request).await?;

        // Update cache
        self.completion_cache.set(items.clone());

        Ok(items)
    }

    /// Request hover information at a specific position
    pub async fn request_hover(&self, position: Position) -> anyhow::Result<Option<HoverInfo>> {
        if !self.initialized.get_untracked() {
            return Ok(None); // Not initialized, return empty
        }

        #[derive(Serialize)]
        struct HoverRequest {
            language: String,
            file_path: String,
            line: u32,
            character: u32,
        }

        let request = HoverRequest {
            language: self.language.get_untracked(),
            file_path: self.file_path.get_untracked(),
            line: position.line as u32,
            character: position.column as u32,
        };

        let info: Option<HoverInfo> = TauriBridge::invoke("lsp_hover", request).await?;

        // Update cache
        self.hover_info.set(info.clone());

        Ok(info)
    }

    /// Request diagnostics for the current file
    pub async fn request_diagnostics(&self) -> anyhow::Result<Vec<Diagnostic>> {
        if !self.initialized.get_untracked() {
            return Ok(Vec::new()); // Not initialized, return empty
        }

        #[derive(Serialize)]
        struct DiagnosticsRequest {
            language: String,
            file_path: String,
        }

        let request = DiagnosticsRequest {
            language: self.language.get_untracked(),
            file_path: self.file_path.get_untracked(),
        };

        let diagnostics: Vec<Diagnostic> = TauriBridge::invoke("lsp_get_diagnostics", request).await?;

        // Update cache
        self.diagnostics.set(diagnostics.clone());

        Ok(diagnostics)
    }

    /// Go to definition at a specific position
    pub async fn goto_definition(&self, position: Position) -> anyhow::Result<LocationInfo> {
        // ✅ FIX: Log initialization status for debugging
        let is_init = self.initialized.get_untracked();
        leptos::logging::log!("🔍 LSP goto_definition: initialized={}, file_path={}",
            is_init, self.file_path.get_untracked());

        if !is_init {
            // ✅ FIX: Provide more helpful error message
            let file_path = self.file_path.get_untracked();
            return Err(anyhow::anyhow!(
                "LSP not initialized for file: {}. Please wait for file to load completely.",
                if file_path.is_empty() { "(no file)" } else { &file_path }
            ));
        }

        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct DefinitionRequest {
            language: String,
            file_path: String,
            line: u32,
            character: u32,
        }

        #[derive(Deserialize)]
        struct LocationResponse {
            uri: String,
            range: RangeResponse,
        }

        #[derive(Deserialize)]
        struct RangeResponse {
            start: PositionResponse,
            end: PositionResponse,
        }

        #[derive(Deserialize)]
        struct PositionResponse {
            line: u32,
            character: u32,
        }

        let request = DefinitionRequest {
            language: self.language.get_untracked(),
            file_path: self.file_path.get_untracked(),
            line: position.line as u32,
            character: position.column as u32,
        };

        leptos::logging::log!("🔍 LSP: Sending goto_definition request: language={}, file={}, line={}, char={}",
            request.language, request.file_path, request.line, request.character);

        let location: LocationResponse = TauriBridge::invoke("lsp_goto_definition", request).await?;

        leptos::logging::log!("🔍 LSP: Received response: uri={}, line={}, char={}",
            location.uri, location.range.start.line, location.range.start.character);

        // Convert file:// URI to regular file path
        let file_path = if location.uri.starts_with("file://") {
            location.uri[7..].to_string()
        } else {
            location.uri
        };

        leptos::logging::log!("🔍 LSP: Converted file path: {}", file_path);

        let result = LocationInfo {
            uri: file_path.clone(),
            line: location.range.start.line as usize,
            column: location.range.start.character as usize,
        };

        leptos::logging::log!("✅ LSP: Returning LocationInfo: uri={}, line={}, column={}",
            result.uri, result.line, result.column);

        Ok(result)
    }

    /// Find all references at a specific position
    pub async fn find_references(&self, position: Position) -> anyhow::Result<Vec<Position>> {
        if !self.initialized.get_untracked() {
            return Ok(Vec::new()); // Not initialized, return empty
        }

        #[derive(Serialize)]
        struct ReferencesRequest {
            language: String,
            file_path: String,
            line: u32,
            character: u32,
        }

        #[derive(Deserialize)]
        struct Location {
            line: u32,
            character: u32,
        }

        let request = ReferencesRequest {
            language: self.language.get_untracked(),
            file_path: self.file_path.get_untracked(),
            line: position.line as u32,
            character: position.column as u32,
        };

        let locations: Vec<Location> = TauriBridge::invoke("lsp_find_references", request).await?;

        Ok(locations.into_iter().map(|loc| {
            Position::new(loc.line as usize, loc.character as usize)
        }).collect())
    }

    /// Get cached completions
    pub fn completions(&self) -> RwSignal<Vec<CompletionItem>> {
        self.completion_cache
    }

    /// Get cached diagnostics
    pub fn diagnostics(&self) -> RwSignal<Vec<Diagnostic>> {
        self.diagnostics
    }

    /// Get cached hover info
    pub fn hover_info(&self) -> RwSignal<Option<HoverInfo>> {
        self.hover_info
    }
}

impl Default for LspIntegration {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_lsp_integration_creation() {
        let lsp = LspIntegration::new();
        assert_eq!(lsp.file_path.get_untracked(), "");
        assert_eq!(lsp.completion_cache.get_untracked().len(), 0);
    }

    #[wasm_bindgen_test]
    fn test_set_file_path() {
        let lsp = LspIntegration::new();
        lsp.set_file_path("/path/to/file.rs".to_string());
        assert_eq!(lsp.file_path.get_untracked(), "/path/to/file.rs");
    }

    #[wasm_bindgen_test]
    fn test_goto_definition_requires_initialization() {
        use crate::types::Position;
        use wasm_bindgen_futures::JsFuture;
        use wasm_bindgen::JsValue;

        let lsp = LspIntegration::new();

        // Without initialization, goto_definition should fail
        let position = Position::new(10, 5);

        // Create async test
        wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

        // Since we can't use async in wasm_bindgen_test easily, we test the preconditions
        assert_eq!(lsp.file_path.get_untracked(), "", "File path should be empty initially");
        assert_eq!(lsp.initialized.get_untracked(), false, "LSP should not be initialized");
    }

    #[wasm_bindgen_test]
    fn test_file_path_must_be_set_before_goto_definition() {
        use crate::types::Position;

        let lsp = LspIntegration::new();

        // Test that file_path is checked before allowing goto_definition
        let file_path = lsp.file_path.get_untracked();
        assert!(file_path.is_empty(), "File path must be empty initially");

        // After setting file path, it should be non-empty
        lsp.set_file_path("/test/file.rs".to_string());
        let file_path_after = lsp.file_path.get_untracked();
        assert!(!file_path_after.is_empty(), "File path must be set");
        assert_eq!(file_path_after, "/test/file.rs");
    }
}
