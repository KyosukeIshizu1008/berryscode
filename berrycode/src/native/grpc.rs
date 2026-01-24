//! gRPC client for berry-api-server
//! Provides streaming AI chat functionality

use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::transport::Channel;

// Include generated code from proto
pub mod berry_api {
    tonic::include_proto!("berry_api");
}

use berry_api::berry_code_service_client::BerryCodeServiceClient;
use berry_api::{ChatRequest, StartSessionRequest};

/// Global gRPC client instance
#[derive(Clone)]
pub struct GrpcClient {
    client: Arc<RwLock<Option<BerryCodeServiceClient<Channel>>>>,
    endpoint: String,
}

impl GrpcClient {
    /// Create a new gRPC client
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            client: Arc::new(RwLock::new(None)),
            endpoint: endpoint.into(),
        }
    }

    /// Connect to berry-api-server
    pub async fn connect(&self) -> Result<()> {
        tracing::info!("🔌 Connecting to berry-api-server at {}", self.endpoint);

        let channel = Channel::from_shared(self.endpoint.clone())
            .context("Invalid gRPC endpoint")?
            .connect()
            .await
            .context("Failed to connect to berry-api-server")?;

        let client = BerryCodeServiceClient::new(channel);
        *self.client.write().await = Some(client);

        tracing::info!("✅ Connected to berry-api-server");
        Ok(())
    }

    /// Check if connected
    pub async fn is_connected(&self) -> bool {
        self.client.read().await.is_some()
    }

    /// Start a new chat session
    pub async fn start_session(&self, project_path: String, autonomous: bool) -> Result<String> {
        let mut client = self.client.write().await;
        let client = client.as_mut().context("Not connected to berry-api-server")?;

        let request = tonic::Request::new(StartSessionRequest {
            model: None, // Use default
            mode: None,  // Use default
            files: vec![],
            project_path: Some(project_path),
            git_enabled: Some(true),
            api_key: None,
            api_base: None,
            autonomous: Some(autonomous),
        });

        let response = client
            .start_session(request)
            .await
            .context("Failed to start session")?
            .into_inner();

        Ok(response.session_id)
    }

    /// Send a chat message and receive streaming response
    /// Returns a channel that receives chunks as they arrive
    pub async fn chat_stream(
        &self,
        session_id: String,
        message: String,
        autonomous: bool,
    ) -> Result<tokio::sync::mpsc::Receiver<String>> {
        // Clone the client to avoid holding write lock during streaming
        let mut client = {
            let client_guard = self.client.read().await;
            client_guard.as_ref().context("Not connected to berry-api-server")?.clone()
        };

        let request = tonic::Request::new(ChatRequest {
            session_id,
            message,
            stream: Some(true), // Enable streaming
            autonomous: Some(autonomous),
        });

        let mut stream = client
            .chat(request)
            .await
            .context("Failed to start chat stream")?
            .into_inner();

        // Create a channel for streaming chunks
        let (tx, rx) = tokio::sync::mpsc::channel(100);

        // Spawn a task to receive and forward chunks
        tokio::spawn(async move {
            tracing::info!("🔄 Starting to receive chunks from gRPC stream");
            let mut chunk_count = 0;

            loop {
                match stream.message().await {
                    Ok(Some(chunk)) => {
                        chunk_count += 1;
                        tracing::info!("📦 Received chunk #{}: {} chars", chunk_count, chunk.content.len());

                        if let Err(e) = tx.send(chunk.content).await {
                            tracing::error!("❌ Failed to send chunk to channel: {}", e);
                            break;
                        }
                        tracing::info!("✅ Chunk #{} sent to channel", chunk_count);
                    }
                    Ok(None) => {
                        tracing::info!("📨 Chat stream ended normally (received {} chunks)", chunk_count);
                        break;
                    }
                    Err(e) => {
                        tracing::error!("❌ Error receiving chunk: {}", e);
                        break;
                    }
                }
            }
        });

        Ok(rx)
    }
}

/// Global singleton instance
static GRPC_CLIENT: once_cell::sync::Lazy<GrpcClient> = once_cell::sync::Lazy::new(|| {
    GrpcClient::new("http://[::1]:50051")
});

/// Get the global gRPC client
pub fn get_client() -> &'static GrpcClient {
    &GRPC_CLIENT
}

/// Initialize and connect to berry-api-server
pub async fn initialize() -> Result<()> {
    get_client().connect().await
}

/// Check if connected
pub async fn is_connected() -> bool {
    get_client().is_connected().await
}

/// Start a new chat session
pub async fn start_session(project_path: String, autonomous: bool) -> Result<String> {
    get_client().start_session(project_path, autonomous).await
}

/// Send a chat message and get streaming response
pub async fn chat_stream(
    session_id: String,
    message: String,
    autonomous: bool,
) -> Result<tokio::sync::mpsc::Receiver<String>> {
    get_client().chat_stream(session_id, message, autonomous).await
}
