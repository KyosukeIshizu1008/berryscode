//! LSP client for berry-api-server
//!
//! Provides Language Server Protocol functionality through gRPC backend.

use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::transport::Channel;

// Include generated LSP service code
pub mod lsp_service {
    tonic::include_proto!("lsp_service");
}

use lsp_service::lsp_service_client::LspServiceClient;
use lsp_service::*;

/// LSP client for communicating with berry-api-server
pub struct LspClient {
    client: Arc<RwLock<Option<LspServiceClient<Channel>>>>,
    endpoint: String,
}

impl LspClient {
    /// Create a new LSP client
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            client: Arc::new(RwLock::new(None)),
            endpoint: endpoint.into(),
        }
    }

    /// Connect to berry-api-server LSP service
    pub async fn connect(&self) -> Result<()> {
        tracing::info!("🔌 Connecting to LSP service at {}", self.endpoint);

        let channel = Channel::from_shared(self.endpoint.clone())
            .context("Invalid LSP endpoint")?
            .connect()
            .await
            .context("Failed to connect to LSP service")?;

        let client = LspServiceClient::new(channel);
        *self.client.write().await = Some(client);

        tracing::info!("✅ Connected to LSP service");
        Ok(())
    }

    /// Check if connected
    pub async fn is_connected(&self) -> bool {
        self.client.read().await.is_some()
    }

    /// Initialize LSP for a language
    pub async fn initialize(
        &self,
        language: impl Into<String>,
        root_uri: impl Into<String>,
        workspace_folder: Option<String>,
    ) -> Result<InitializeResponse> {
        let mut client = self.client.write().await;
        let client = client
            .as_mut()
            .context("LSP client not connected - call connect() first")?;

        let request = InitializeRequest {
            language: language.into(),
            root_uri: root_uri.into(),
            workspace_folder,
        };

        let response = client
            .initialize(request)
            .await
            .context("Failed to initialize LSP")?
            .into_inner();

        if !response.success {
            if let Some(error) = &response.error {
                anyhow::bail!("LSP initialization failed: {}", error);
            }
        }

        tracing::info!("✅ LSP initialized");
        Ok(response)
    }

    /// Get code completions at a position
    pub async fn get_completions(
        &self,
        language: impl Into<String>,
        file_path: impl Into<String>,
        line: u32,
        character: u32,
    ) -> Result<Vec<CompletionItem>> {
        let mut client = self.client.write().await;
        let client = client
            .as_mut()
            .context("LSP client not connected - call connect() first")?;

        let request = CompletionRequest {
            language: language.into(),
            file_path: file_path.into(),
            position: Some(Position { line, character }),
        };

        let response = client
            .get_completions(request)
            .await
            .context("Failed to get completions")?
            .into_inner();

        Ok(response.items)
    }

    /// Get hover information at a position
    pub async fn get_hover(
        &self,
        language: impl Into<String>,
        file_path: impl Into<String>,
        line: u32,
        character: u32,
    ) -> Result<Option<HoverInfo>> {
        let mut client = self.client.write().await;
        let client = client
            .as_mut()
            .context("LSP client not connected - call connect() first")?;

        let request = HoverRequest {
            language: language.into(),
            file_path: file_path.into(),
            position: Some(Position { line, character }),
        };

        let response = client
            .get_hover(request)
            .await
            .context("Failed to get hover info")?
            .into_inner();

        Ok(response.hover)
    }

    /// Go to definition of a symbol
    pub async fn goto_definition(
        &self,
        language: impl Into<String>,
        file_path: impl Into<String>,
        line: u32,
        character: u32,
    ) -> Result<Vec<Location>> {
        let mut client = self.client.write().await;
        let client = client
            .as_mut()
            .context("LSP client not connected - call connect() first")?;

        let request = GotoDefinitionRequest {
            language: language.into(),
            file_path: file_path.into(),
            position: Some(Position { line, character }),
        };

        let response = client
            .goto_definition(request)
            .await
            .context("Failed to go to definition")?
            .into_inner();

        Ok(response.locations)
    }

    /// Find all references to a symbol
    pub async fn find_references(
        &self,
        language: impl Into<String>,
        file_path: impl Into<String>,
        line: u32,
        character: u32,
        include_declaration: bool,
    ) -> Result<Vec<Location>> {
        let mut client = self.client.write().await;
        let client = client
            .as_mut()
            .context("LSP client not connected - call connect() first")?;

        let request = FindReferencesRequest {
            language: language.into(),
            file_path: file_path.into(),
            position: Some(Position { line, character }),
            include_declaration,
        };

        let response = client
            .find_references(request)
            .await
            .context("Failed to find references")?
            .into_inner();

        Ok(response.locations)
    }

    /// Get diagnostics for a file
    pub async fn get_diagnostics(
        &self,
        language: impl Into<String>,
        file_path: impl Into<String>,
    ) -> Result<Vec<Diagnostic>> {
        let mut client = self.client.write().await;
        let client = client
            .as_mut()
            .context("LSP client not connected - call connect() first")?;

        let request = DiagnosticsRequest {
            language: language.into(),
            file_path: file_path.into(),
        };

        let response = client
            .get_diagnostics(request)
            .await
            .context("Failed to get diagnostics")?
            .into_inner();

        Ok(response.diagnostics)
    }

    /// Shutdown LSP client for a language
    pub async fn shutdown(&self, language: impl Into<String>) -> Result<()> {
        let mut client = self.client.write().await;
        let client = client
            .as_mut()
            .context("LSP client not connected - call connect() first")?;

        let request = ShutdownRequest {
            language: language.into(),
        };

        client
            .shutdown(request)
            .await
            .context("Failed to shutdown LSP")?;

        tracing::info!("✅ LSP shutdown");
        Ok(())
    }

    /// Shutdown all LSP clients
    pub async fn shutdown_all(&self) -> Result<()> {
        let mut client = self.client.write().await;
        let client = client
            .as_mut()
            .context("LSP client not connected - call connect() first")?;

        client
            .shutdown_all(())
            .await
            .context("Failed to shutdown all LSP clients")?;

        tracing::info!("✅ All LSP clients shutdown");
        Ok(())
    }
}

impl Default for LspClient {
    fn default() -> Self {
        // Default to localhost:50051 (berry-api-server default port)
        Self::new("http://127.0.0.1:50051")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lsp_client_creation() {
        let client = LspClient::new("http://127.0.0.1:50051");
        assert_eq!(client.endpoint, "http://127.0.0.1:50051");
    }

    #[test]
    fn test_lsp_client_default() {
        let client = LspClient::default();
        assert_eq!(client.endpoint, "http://127.0.0.1:50051");
    }

    #[tokio::test]
    async fn test_lsp_client_not_connected() {
        let client = LspClient::new("http://127.0.0.1:50051");
        assert!(!client.is_connected().await);
    }

    // Note: Integration tests require berry-api-server to be running
    // Run with: cargo test --test lsp_integration_test
}
