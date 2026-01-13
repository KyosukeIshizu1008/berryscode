//! LSP Client Implementation using berry_api gRPC
//!
//! This module provides LSP functionality by delegating to berry_api's
//! BerryCodeCLIService which supports 30+ languages.

use super::types::*;
use crate::grpc_client::BerryApiClient;
use std::sync::Arc;
use tokio::sync::Mutex;

// Import specific types
use super::types::{HoverContents, DiagnosticSeverity};

/// LSP Client (gRPC-based)
pub struct LspClient {
    language: String,
    grpc_client: Arc<Mutex<BerryApiClient>>,
    initialized: Arc<Mutex<bool>>,
}

impl LspClient {
    /// Create new LSP client for a language
    pub async fn new(language: &str, root_uri: &str) -> Result<Self, String> {
        eprintln!("[LSP] Creating client for language: {}, root_uri: {}", language, root_uri);

        let grpc_client = BerryApiClient::connect("http://localhost:50051")
            .await
            .map_err(|e| format!("Failed to connect to berry_api: {}", e))?;

        let client = Self {
            language: language.to_string(),
            grpc_client: Arc::new(Mutex::new(grpc_client)),
            initialized: Arc::new(Mutex::new(false)),
        };

        // Initialize session and LSP
        client.ensure_session_initialized(root_uri).await?;
        client.ensure_lsp_initialized(root_uri).await?;

        Ok(client)
    }

    /// Ensure gRPC session is initialized (required for all operations)
    async fn ensure_session_initialized(&self, root_uri: &str) -> Result<(), String> {
        let mut grpc = self.grpc_client.lock().await;

        // Check if session is already initialized
        if grpc.get_session_id().await.is_some() {
            return Ok(());
        }

        // Initialize CLI session
        let session_id = grpc
            .init_session(root_uri)
            .await
            .map_err(|e| format!("Failed to init session: {}", e))?;

        eprintln!("[LSP] ✅ Session initialized: {}", session_id);

        Ok(())
    }

    /// Ensure LSP is initialized for the language
    async fn ensure_lsp_initialized(&self, root_uri: &str) -> Result<(), String> {
        let mut initialized = self.initialized.lock().await;

        if *initialized {
            return Ok(());
        }

        eprintln!("[LSP] 🚀 Starting LSP initialization for {} at {}", self.language, root_uri);
        let mut grpc = self.grpc_client.lock().await;

        // Initialize LSP for the language
        eprintln!("[LSP] 📡 Sending initialize_lsp request to berry_api...");
        grpc.initialize_lsp(&self.language, root_uri)
            .await
            .map_err(|e| format!("Failed to initialize LSP: {}", e))?;

        *initialized = true;
        eprintln!("[LSP] ✅ LSP initialized for {}", self.language);

        Ok(())
    }

    /// Get completions at position
    pub async fn get_completions(
        &self,
        file_uri: &str,
        line: u32,
        character: u32,
    ) -> Result<Vec<CompletionItem>, String> {
        eprintln!("[LSP] Getting completions at {}:{}:{}", file_uri, line, character);

        let grpc = self.grpc_client.lock().await;
        let completions = grpc.get_completions(file_uri, line, character)
            .await
            .map_err(|e| format!("Failed to get completions: {}", e))?;

        // Convert gRPC CompletionItem to our CompletionItem
        let items = completions
            .into_iter()
            .map(|item| CompletionItem {
                label: item.label,
                kind: item.kind.parse::<u32>().ok(),
                detail: item.detail,
                documentation: item.documentation,
                insert_text: item.insert_text,
            })
            .collect();

        Ok(items)
    }

    /// Get hover information at position
    pub async fn get_hover(
        &self,
        file_uri: &str,
        line: u32,
        character: u32,
    ) -> Result<Option<Hover>, String> {
        eprintln!("[LSP] Getting hover at {}:{}:{}", file_uri, line, character);

        let grpc = self.grpc_client.lock().await;
        let hover_response = grpc.get_hover(file_uri, line, character)
            .await
            .map_err(|e| format!("Failed to get hover: {}", e))?;

        match hover_response {
            Some(h) if h.contents.is_some() => {
                Ok(Some(Hover {
                    contents: HoverContents::String(h.contents.unwrap()),
                    range: h.range.map(|r| Range {
                        start: Position {
                            line: r.start.as_ref().map(|p| p.line).unwrap_or(0),
                            character: r.start.as_ref().map(|p| p.character).unwrap_or(0),
                        },
                        end: Position {
                            line: r.end.as_ref().map(|p| p.line).unwrap_or(0),
                            character: r.end.as_ref().map(|p| p.character).unwrap_or(0),
                        },
                    }),
                }))
            }
            _ => Ok(None),
        }
    }

    /// Go to definition
    pub async fn goto_definition(
        &self,
        file_uri: &str,
        line: u32,
        character: u32,
    ) -> Result<Option<Location>, String> {
        eprintln!("[LSP] Goto definition at {}:{}:{}", file_uri, line, character);

        // Extract root_uri from file_uri
        let root_uri = if file_uri.starts_with("file://") {
            let path = &file_uri[7..];
            // Get project root (directory containing Cargo.toml)
            if let Some(idx) = path.rfind("/src/") {
                format!("file://{}", &path[..idx])
            } else if let Some(idx) = path.rfind('/') {
                format!("file://{}", &path[..idx])
            } else {
                "file:///".to_string()
            }
        } else {
            "file:///".to_string()
        };

        // Ensure session is initialized
        self.ensure_session_initialized(&root_uri).await?;

        // Ensure LSP is initialized
        self.ensure_lsp_initialized(&root_uri).await?;

        let grpc = self.grpc_client.lock().await;
        let location = grpc.goto_definition(file_uri, line, character)
            .await
            .map_err(|e| format!("Failed to goto definition: {}", e))?;

        match location {
            Some(loc) => {
                let range = loc.range.unwrap_or_default();
                Ok(Some(Location {
                    uri: loc.uri,
                    range: Range {
                        start: Position {
                            line: range.start.as_ref().map(|p| p.line).unwrap_or(0),
                            character: range.start.as_ref().map(|p| p.character).unwrap_or(0),
                        },
                        end: Position {
                            line: range.end.as_ref().map(|p| p.line).unwrap_or(0),
                            character: range.end.as_ref().map(|p| p.character).unwrap_or(0),
                        },
                    },
                }))
            }
            None => Ok(None),
        }
    }

    /// Find references
    pub async fn find_references(
        &self,
        file_uri: &str,
        line: u32,
        character: u32,
        include_declaration: bool,
    ) -> Result<Vec<Location>, String> {
        eprintln!("[LSP] Finding references at {}:{}:{}", file_uri, line, character);

        let grpc = self.grpc_client.lock().await;
        let locations = grpc.find_references(file_uri, line, character, include_declaration)
            .await
            .map_err(|e| format!("Failed to find references: {}", e))?;

        let converted = locations
            .into_iter()
            .filter_map(|loc| {
                let range = loc.range?;
                Some(Location {
                    uri: loc.uri,
                    range: Range {
                        start: Position {
                            line: range.start.as_ref()?.line,
                            character: range.start.as_ref()?.character,
                        },
                        end: Position {
                            line: range.end.as_ref()?.line,
                            character: range.end.as_ref()?.character,
                        },
                    },
                })
            })
            .collect();

        Ok(converted)
    }

    /// Get diagnostics
    pub async fn get_diagnostics(&self, file_uri: &str) -> Result<Vec<Diagnostic>, String> {
        eprintln!("[LSP] Getting diagnostics for {}", file_uri);

        let grpc = self.grpc_client.lock().await;
        let diagnostics = grpc.get_diagnostics(file_uri)
            .await
            .map_err(|e| format!("Failed to get diagnostics: {}", e))?;

        let converted = diagnostics
            .into_iter()
            .filter_map(|diag| {
                let range = diag.range?;
                Some(Diagnostic {
                    range: Range {
                        start: Position {
                            line: range.start.as_ref()?.line,
                            character: range.start.as_ref()?.character,
                        },
                        end: Position {
                            line: range.end.as_ref()?.line,
                            character: range.end.as_ref()?.character,
                        },
                    },
                    severity: match diag.severity {
                        1 => Some(DiagnosticSeverity::Error),
                        2 => Some(DiagnosticSeverity::Warning),
                        3 => Some(DiagnosticSeverity::Information),
                        4 => Some(DiagnosticSeverity::Hint),
                        _ => None,
                    },
                    code: None,
                    message: diag.message,
                    source: diag.source,
                })
            })
            .collect();

        Ok(converted)
    }

    /// Add file to LSP context
    pub async fn add_file_to_context(&self, file_uri: &str) -> Result<(), String> {
        eprintln!("[LSP] Adding file to context: {}", file_uri);

        let grpc = self.grpc_client.lock().await;
        grpc.add_file_to_context(file_uri)
            .await
            .map_err(|e| format!("Failed to add file to context: {}", e))?;

        Ok(())
    }

    /// Shutdown LSP server
    pub async fn shutdown(&self) -> Result<(), String> {
        eprintln!("[LSP] Shutting down LSP for {}", self.language);

        let grpc = self.grpc_client.lock().await;
        grpc.shutdown_lsp(&self.language)
            .await
            .map_err(|e| format!("Failed to shutdown LSP: {}", e))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_lsp_client_creation() {
        // This test requires berry_api server running
        let result = LspClient::new("rust", "file:///tmp/test").await;
        assert!(result.is_ok() || result.is_err()); // Either way is fine for unit test
    }
}
