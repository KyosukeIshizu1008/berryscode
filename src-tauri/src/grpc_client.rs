//! gRPC Client for berry_api
//!
//! This module provides a high-level interface to berry_api's gRPC services,
//! particularly the BerryCodeCLIService for LSP operations.

use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::sync::Mutex;

// Include generated protobuf code
mod berry_api_proto {
    tonic::include_proto!("berry_api");
}

// Re-export types for easier access
pub use berry_api_proto::*;

use berry_code_cli_service_client::BerryCodeCliServiceClient;
use tonic::transport::Channel;

/// Global gRPC client for berry_api
#[derive(Clone)]
pub struct BerryApiClient {
    client: Arc<Mutex<BerryCodeCliServiceClient<Channel>>>,
    session_id: Arc<Mutex<Option<String>>>,
    server_url: String,
}

impl BerryApiClient {
    /// Create a new client connecting to berry_api server
    pub async fn connect(server_url: &str) -> Result<Self> {
        let client = BerryCodeCliServiceClient::connect(server_url.to_string())
            .await
            .context("Failed to connect to berry_api server")?;

        Ok(Self {
            client: Arc::new(Mutex::new(client)),
            session_id: Arc::new(Mutex::new(None)),
            server_url: server_url.to_string(),
        })
    }

    /// Initialize CLI session
    pub async fn init_session(&self, project_root: &str) -> Result<String> {
        let mut client = self.client.lock().await;

        let request = tonic::Request::new(berry_api_proto::InitCliRequest {
            project_root: project_root.to_string(),
            model: None,
            dangerously_skip_permissions: Some(false),
        });

        let response = client
            .initialize_cli(request)
            .await
            .context("Failed to initialize CLI session")?
            .into_inner();

        if !response.success {
            anyhow::bail!(
                "Session initialization failed: {}",
                response.error.unwrap_or_else(|| "Unknown error".to_string())
            );
        }

        let session_id = response.session_id;
        *self.session_id.lock().await = Some(session_id.clone());

        Ok(session_id)
    }

    /// Get current session ID
    pub async fn get_session_id(&self) -> Option<String> {
        self.session_id.lock().await.clone()
    }

    /// Initialize LSP for a language
    pub async fn initialize_lsp(&self, language: &str, root_uri: &str) -> Result<()> {
        let session_id = self
            .get_session_id()
            .await
            .context("No active session. Call init_session first.")?;

        let mut client = self.client.lock().await;

        let request = tonic::Request::new(berry_api_proto::InitLspRequest {
            session_id,
            language: language.to_string(),
            root_uri: root_uri.to_string(),
        });

        let response = client
            .initialize_lsp(request)
            .await
            .context("Failed to initialize LSP")?
            .into_inner();

        if !response.success {
            anyhow::bail!(
                "LSP initialization failed: {}",
                response.error.unwrap_or_else(|| "Unknown error".to_string())
            );
        }

        Ok(())
    }

    /// Get code completions
    pub async fn get_completions(
        &self,
        file_uri: &str,
        line: u32,
        character: u32,
    ) -> Result<Vec<CompletionItem>> {
        let session_id = self
            .get_session_id()
            .await
            .context("No active session")?;

        let mut client = self.client.lock().await;

        let request = tonic::Request::new(berry_api_proto::CompletionsRequest {
            session_id,
            file_uri: file_uri.to_string(),
            position: Some(berry_api_proto::Position { line, character }),
            trigger_character: None,
        });

        let response = client
            .get_completions(request)
            .await
            .context("Failed to get completions")?
            .into_inner();

        Ok(response.items)
    }

    /// Get hover information
    pub async fn get_hover(
        &self,
        file_uri: &str,
        line: u32,
        character: u32,
    ) -> Result<Option<HoverResponse>> {
        let session_id = self
            .get_session_id()
            .await
            .context("No active session")?;

        let mut client = self.client.lock().await;

        let request = tonic::Request::new(berry_api_proto::HoverRequest {
            session_id,
            file_uri: file_uri.to_string(),
            position: Some(berry_api_proto::Position { line, character }),
        });

        let response = client
            .get_hover(request)
            .await
            .context("Failed to get hover info")?
            .into_inner();

        Ok(Some(response))
    }

    /// Go to definition
    pub async fn goto_definition(
        &self,
        file_uri: &str,
        line: u32,
        character: u32,
    ) -> Result<Option<Location>> {
        let session_id = self
            .get_session_id()
            .await
            .context("No active session")?;

        let mut client = self.client.lock().await;

        let request = tonic::Request::new(berry_api_proto::GotoDefRequest {
            session_id,
            file_uri: file_uri.to_string(),
            position: Some(berry_api_proto::Position { line, character }),
        });

        let response = client
            .goto_definition_cli(request)
            .await
            .context("Failed to goto definition")?
            .into_inner();

        Ok(response.locations.into_iter().next())
    }

    /// Find references
    pub async fn find_references(
        &self,
        file_uri: &str,
        line: u32,
        character: u32,
        include_declaration: bool,
    ) -> Result<Vec<Location>> {
        let session_id = self
            .get_session_id()
            .await
            .context("No active session")?;

        let mut client = self.client.lock().await;

        let request = tonic::Request::new(berry_api_proto::FindRefsRequest {
            session_id,
            file_uri: file_uri.to_string(),
            position: Some(berry_api_proto::Position { line, character }),
            include_declaration,
        });

        let response = client
            .find_references_cli(request)
            .await
            .context("Failed to find references")?
            .into_inner();

        Ok(response.locations)
    }

    /// Get diagnostics
    pub async fn get_diagnostics(&self, file_uri: &str) -> Result<Vec<Diagnostic>> {
        let session_id = self
            .get_session_id()
            .await
            .context("No active session")?;

        let mut client = self.client.lock().await;

        let request = tonic::Request::new(berry_api_proto::DiagnosticsReq {
            session_id,
            file_uri: file_uri.to_string(),
        });

        let response = client
            .get_diagnostics_cli(request)
            .await
            .context("Failed to get diagnostics")?
            .into_inner();

        Ok(response.diagnostics)
    }

    /// Shutdown LSP
    pub async fn shutdown_lsp(&self, language: &str) -> Result<()> {
        let session_id = self
            .get_session_id()
            .await
            .context("No active session")?;

        let mut client = self.client.lock().await;

        let request = tonic::Request::new(berry_api_proto::ShutdownLspRequest {
            session_id,
            language: language.to_string(),
        });

        let response = client
            .shutdown_lsp(request)
            .await
            .context("Failed to shutdown LSP")?
            .into_inner();

        if !response.success {
            anyhow::bail!("LSP shutdown failed");
        }

        Ok(())
    }

    /// Add file to context for LSP operations
    pub async fn add_file_to_context(&self, file_path: &str) -> Result<()> {
        let session_id = self
            .get_session_id()
            .await
            .context("No active session")?;

        let mut client = self.client.lock().await;

        let request = tonic::Request::new(berry_api_proto::AddFileRequest {
            session_id,
            file_path: file_path.to_string(),
        });

        let response = client
            .add_file_to_context(request)
            .await
            .context("Failed to add file to context")?
            .into_inner();

        if !response.success {
            anyhow::bail!(
                "Failed to add file to context: {}",
                response.error.unwrap_or_else(|| "Unknown error".to_string())
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        // This test requires berry_api server running
        let result = BerryApiClient::connect("http://localhost:50051").await;
        assert!(result.is_ok(), "Failed to connect to berry_api server");
    }
}
