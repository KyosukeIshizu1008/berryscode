//! HTTP-based LSP client for Berry API integration
//!
//! This module provides LSP functionality by communicating with the Berry API server
//! at http://localhost:8081 instead of spawning local language server processes.
//!
//! Benefits:
//! - No need to spawn/manage LSP server processes
//! - Centralized LSP server management via Berry API
//! - Consistent session handling across multiple editors
//! - No process cleanup required (handled by Berry API)

use anyhow::{anyhow, Result};
use lsp_types::{Location, Diagnostic, CompletionItem};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// HTTP-based LSP client that calls Berry API endpoints
#[derive(Clone)]
pub struct LspHttpClient {
    /// Berry API base URL (default: http://localhost:8081)
    base_url: String,
    /// Session ID for LSP client management
    session_id: String,
    /// HTTP client
    client: reqwest::Client,
}

// ============================================================================
// Request/Response types matching Berry API format
// ============================================================================

/// Go to definition request
#[derive(Debug, Serialize)]
struct GotoDefinitionRequest {
    session_id: String,
    path: String,
    line: u32,
    column: u32,
}

/// Find references request
#[derive(Debug, Serialize)]
struct FindReferencesRequest {
    session_id: String,
    path: String,
    line: u32,
    column: u32,
}

/// Hover request
#[derive(Debug, Serialize)]
struct HoverRequest {
    session_id: String,
    path: String,
    line: u32,
    column: u32,
}

/// Completion request
#[derive(Debug, Serialize)]
struct CompletionRequest {
    session_id: String,
    path: String,
    line: u32,
    column: u32,
}

/// Diagnostics request
#[derive(Debug, Serialize)]
struct DiagnosticsRequest {
    session_id: String,
    path: String,
}

/// Location response from Berry API
#[derive(Debug, Deserialize)]
struct LocationResponse {
    file_path: String,
    line: u32,
    column: u32,
    end_line: u32,
    end_column: u32,
}

/// Go to definition response
#[derive(Debug, Deserialize)]
struct GotoDefinitionResponse {
    success: bool,
    location: Option<LocationResponse>,
    error: Option<String>,
}

/// Find references response
#[derive(Debug, Deserialize)]
struct FindReferencesResponse {
    success: bool,
    locations: Vec<LocationResponse>,
    error: Option<String>,
}

/// Hover response
#[derive(Debug, Deserialize)]
struct HoverResponse {
    success: bool,
    contents: Option<String>,
    error: Option<String>,
}

/// Completion response
#[derive(Debug, Deserialize)]
struct CompletionResponse {
    success: bool,
    items: Vec<CompletionItem>,
    error: Option<String>,
}

/// Diagnostics response
#[derive(Debug, Deserialize)]
struct DiagnosticsResponse {
    success: bool,
    diagnostics: Vec<Diagnostic>,
    error: Option<String>,
}

// ============================================================================
// Implementation
// ============================================================================

impl LspHttpClient {
    /// Create a new HTTP LSP client
    ///
    /// # Arguments
    /// * `base_url` - Berry API base URL (e.g., "http://localhost:8081")
    /// * `session_id` - Session ID for LSP client management
    pub fn new(base_url: String, session_id: String) -> Self {
        Self {
            base_url,
            session_id,
            client: reqwest::Client::new(),
        }
    }

    /// Go to definition of a symbol
    ///
    /// # Arguments
    /// * `file_path` - Path to the file
    /// * `line` - Line number (1-based)
    /// * `character` - Character offset (1-based)
    ///
    /// # Returns
    /// Location where the symbol is defined, or None if not found
    pub async fn goto_definition(
        &self,
        file_path: &Path,
        line: u32,
        character: u32,
    ) -> Result<Option<Location>> {
        let url = format!("{}/api/lsp/goto-definition", self.base_url);

        let request = GotoDefinitionRequest {
            session_id: self.session_id.clone(),
            path: file_path.to_string_lossy().to_string(),
            line: line + 1, // Convert to 1-based
            column: character + 1, // Convert to 1-based
        };

        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to send goto definition request: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow!("LSP API error: {}", response.status()));
        }

        let result: GotoDefinitionResponse = response.json().await
            .map_err(|e| anyhow!("Failed to parse goto definition response: {}", e))?;

        if !result.success {
            if let Some(error) = result.error {
                return Err(anyhow!("LSP error: {}", error));
            }
            return Ok(None);
        }

        // Convert LocationResponse to lsp_types::Location
        if let Some(loc) = result.location {
            let uri = lsp_types::Url::from_file_path(&loc.file_path)
                .map_err(|_| anyhow!("Invalid file path: {}", loc.file_path))?;

            let location = Location {
                uri,
                range: lsp_types::Range {
                    start: lsp_types::Position {
                        line: loc.line.saturating_sub(1), // Convert to 0-based
                        character: loc.column.saturating_sub(1), // Convert to 0-based
                    },
                    end: lsp_types::Position {
                        line: loc.end_line.saturating_sub(1),
                        character: loc.end_column.saturating_sub(1),
                    },
                },
            };

            Ok(Some(location))
        } else {
            Ok(None)
        }
    }

    /// Find all references to a symbol
    ///
    /// # Arguments
    /// * `file_path` - Path to the file
    /// * `line` - Line number (1-based)
    /// * `character` - Character offset (1-based)
    ///
    /// # Returns
    /// List of locations where the symbol is referenced
    pub async fn find_references(
        &self,
        file_path: &Path,
        line: u32,
        character: u32,
    ) -> Result<Vec<Location>> {
        let url = format!("{}/api/lsp/find-references", self.base_url);

        let request = FindReferencesRequest {
            session_id: self.session_id.clone(),
            path: file_path.to_string_lossy().to_string(),
            line: line + 1, // Convert to 1-based
            column: character + 1, // Convert to 1-based
        };

        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to send find references request: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow!("LSP API error: {}", response.status()));
        }

        let result: FindReferencesResponse = response.json().await
            .map_err(|e| anyhow!("Failed to parse find references response: {}", e))?;

        if !result.success {
            if let Some(error) = result.error {
                return Err(anyhow!("LSP error: {}", error));
            }
            return Ok(Vec::new());
        }

        // Convert LocationResponse to lsp_types::Location
        let locations: Result<Vec<Location>> = result.locations.into_iter().map(|loc| {
            let uri = lsp_types::Url::from_file_path(&loc.file_path)
                .map_err(|_| anyhow!("Invalid file path: {}", loc.file_path))?;

            Ok(Location {
                uri,
                range: lsp_types::Range {
                    start: lsp_types::Position {
                        line: loc.line.saturating_sub(1),
                        character: loc.column.saturating_sub(1),
                    },
                    end: lsp_types::Position {
                        line: loc.end_line.saturating_sub(1),
                        character: loc.end_column.saturating_sub(1),
                    },
                },
            })
        }).collect();

        locations
    }

    /// Get hover information (type, documentation) for a symbol
    ///
    /// # Arguments
    /// * `file_path` - Path to the file
    /// * `line` - Line number (1-based)
    /// * `character` - Character offset (1-based)
    ///
    /// # Returns
    /// Hover text, or None if not available
    pub async fn hover(
        &self,
        file_path: &Path,
        line: u32,
        character: u32,
    ) -> Result<Option<String>> {
        let url = format!("{}/api/lsp/hover", self.base_url);

        let request = HoverRequest {
            session_id: self.session_id.clone(),
            path: file_path.to_string_lossy().to_string(),
            line: line + 1, // Convert to 1-based
            column: character + 1, // Convert to 1-based
        };

        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to send hover request: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow!("LSP API error: {}", response.status()));
        }

        let result: HoverResponse = response.json().await
            .map_err(|e| anyhow!("Failed to parse hover response: {}", e))?;

        if !result.success {
            if let Some(error) = result.error {
                return Err(anyhow!("LSP error: {}", error));
            }
            return Ok(None);
        }

        Ok(result.contents)
    }

    /// Get completion items at a position
    ///
    /// # Arguments
    /// * `file_path` - Path to the file
    /// * `line` - Line number (1-based)
    /// * `character` - Character offset (1-based)
    ///
    /// # Returns
    /// List of completion items
    pub async fn completion(
        &self,
        file_path: &Path,
        line: u32,
        character: u32,
    ) -> Result<Vec<CompletionItem>> {
        let url = format!("{}/api/lsp/completion", self.base_url);

        let request = CompletionRequest {
            session_id: self.session_id.clone(),
            path: file_path.to_string_lossy().to_string(),
            line: line + 1, // Convert to 1-based
            column: character + 1, // Convert to 1-based
        };

        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to send completion request: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow!("LSP API error: {}", response.status()));
        }

        let result: CompletionResponse = response.json().await
            .map_err(|e| anyhow!("Failed to parse completion response: {}", e))?;

        if !result.success {
            if let Some(error) = result.error {
                return Err(anyhow!("LSP error: {}", error));
            }
            return Ok(Vec::new());
        }

        Ok(result.items)
    }

    /// Get diagnostics for a file
    ///
    /// # Arguments
    /// * `file_path` - Path to the file
    ///
    /// # Returns
    /// List of diagnostics (errors, warnings, etc.)
    pub async fn get_diagnostics(&self, file_path: &Path) -> Result<Vec<Diagnostic>> {
        let url = format!("{}/api/lsp/diagnostics", self.base_url);

        let request = DiagnosticsRequest {
            session_id: self.session_id.clone(),
            path: file_path.to_string_lossy().to_string(),
        };

        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to send diagnostics request: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow!("LSP API error: {}", response.status()));
        }

        let result: DiagnosticsResponse = response.json().await
            .map_err(|e| anyhow!("Failed to parse diagnostics response: {}", e))?;

        if !result.success {
            if let Some(error) = result.error {
                return Err(anyhow!("LSP error: {}", error));
            }
            return Ok(Vec::new());
        }

        Ok(result.diagnostics)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lsp_http_client_creation() {
        let client = LspHttpClient::new(
            "http://localhost:8081".to_string(),
            "test-session".to_string(),
        );

        assert_eq!(client.base_url, "http://localhost:8081");
        assert_eq!(client.session_id, "test-session");
    }

    // Note: Integration tests require Berry API server to be running
    // Run with: cargo test --test lsp_http_integration_test
}
