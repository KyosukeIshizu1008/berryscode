use crate::berry_api::berry_code_service_server::BerryCodeService;
use crate::berry_api::*;
use crate::llm::LlmClient;
use crate::session::SessionManager;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

pub struct BerryCodeServiceImpl {
    session_manager: Arc<RwLock<SessionManager>>,
    llm_client: Option<LlmClient>,
}

impl BerryCodeServiceImpl {
    pub fn new() -> Self {
        // Try to create LLM client
        let llm_client = match LlmClient::new() {
            Ok(client) => {
                tracing::info!("✅ LLM client initialized — multi-model routing enabled (Ollama)");
                Some(client)
            }
            Err(e) => {
                tracing::warn!("⚠️  LLM client not available: {}. Using mock responses.", e);
                None
            }
        };

        Self {
            session_manager: Arc::new(RwLock::new(SessionManager::new())),
            llm_client,
        }
    }
}

#[tonic::async_trait]
impl BerryCodeService for BerryCodeServiceImpl {
    async fn start_session(
        &self,
        request: Request<StartSessionRequest>,
    ) -> Result<Response<StartSessionResponse>, Status> {
        let req = request.into_inner();

        tracing::info!(
            "📝 Starting new session: project_path={:?}",
            req.project_path
        );

        let session_id = uuid::Uuid::new_v4().to_string();
        let model = req.model.clone().unwrap_or_else(|| "auto".to_string());
        let mode = req.mode.clone().unwrap_or_else(|| "code".to_string());

        // Store session in manager, evicting any expired sessions first
        let mut manager = self.session_manager.write().await;
        manager.cleanup_expired();
        manager.create_session(session_id.clone(), req.clone());

        tracing::info!("✅ Session created: {}", session_id);

        Ok(Response::new(StartSessionResponse {
            session_id,
            model,
            mode,
            files: req.files,
            git_root: req.project_path,
        }))
    }

    type ChatStream = ReceiverStream<Result<ChatChunk, Status>>;

    async fn chat(
        &self,
        request: Request<ChatRequest>,
    ) -> Result<Response<Self::ChatStream>, Status> {
        let req = request.into_inner();

        tracing::info!(
            "💬 Chat request: session={}, message={}",
            req.session_id,
            req.message
        );

        // Get project path: prefer per-request field, fall back to session storage
        let project_path = {
            if let Some(ref p) = req.project_path {
                if !p.is_empty() {
                    Some(p.clone())
                } else {
                    None
                }
            } else {
                let manager = self.session_manager.read().await;
                manager
                    .get_session(&req.session_id)
                    .and_then(|session| session.request.project_path.clone())
            }
        };

        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let llm_client = self.llm_client.clone();

        // Spawn async task to handle streaming response
        tokio::spawn(async move {
            if let Some(client) = llm_client {
                let autonomous = req.autonomous.unwrap_or(false);

                // Always use the router model (llama3.2:3b) to classify intent
                let role = client.classify_with_router(&req.message).await;

                tracing::info!(
                    "🤖 Chat mode: role={:?}, autonomous={}, project_path={:?}",
                    role,
                    autonomous,
                    project_path
                );

                match client
                    .chat_stream(req.message.clone(), role, autonomous, project_path)
                    .await
                {
                    Ok(mut stream) => {
                        use futures::StreamExt;
                        let mut chunk_count = 0;

                        while let Some(result) = stream.next().await {
                            match result {
                                Ok(text) => {
                                    chunk_count += 1;
                                    let chat_chunk = ChatChunk {
                                        content: text,
                                        is_final: false,
                                        thinking: None,
                                        metadata: None,
                                    };

                                    if tx.send(Ok(chat_chunk)).await.is_err() {
                                        break;
                                    }
                                }
                                Err(e) => {
                                    tracing::error!("❌ Stream error: {}", e);
                                    let error_chunk = ChatChunk {
                                        content: format!("\n\n❌ Error: {}", e),
                                        is_final: false,
                                        thinking: None,
                                        metadata: None,
                                    };
                                    let _ = tx.send(Ok(error_chunk)).await;
                                    break;
                                }
                            }
                        }

                        // Send final chunk with metadata
                        let final_chunk = ChatChunk {
                            content: String::new(),
                            is_final: true,
                            thinking: None,
                            metadata: Some(ChatMetadata {
                                // Note: chunk_count is the number of stream chunks, not token count.
                                // Accurate token usage requires Ollama's /api/chat eval_count field.
                                tokens_used: chunk_count,
                                model: "ollama".to_string(),
                                finish_reason: Some("stop".to_string()),
                            }),
                        };
                        let _ = tx.send(Ok(final_chunk)).await;
                    }
                    Err(e) => {
                        tracing::error!("❌ Failed to call Ollama API: {}", e);
                        let error_chunk = ChatChunk {
                            content: format!("Error: {}", e),
                            is_final: true,
                            thinking: None,
                            metadata: None,
                        };
                        let _ = tx.send(Ok(error_chunk)).await;
                    }
                }
            } else {
                // Fallback to mock response if LLM client is not available
                tracing::warn!("⚠️  Using mock response (Ollama not reachable)");
                let response_text = format!("Echo: {}", req.message);
                let chunks: Vec<&str> = response_text.split_whitespace().collect();

                for (i, chunk) in chunks.iter().enumerate() {
                    let is_final = i == chunks.len() - 1;

                    let chat_chunk = ChatChunk {
                        content: format!("{} ", chunk),
                        is_final,
                        thinking: None,
                        metadata: if is_final {
                            Some(ChatMetadata {
                                tokens_used: chunks.len() as u32,
                                model: "mock-model".to_string(),
                                finish_reason: Some("stop".to_string()),
                            })
                        } else {
                            None
                        },
                    };

                    if tx.send(Ok(chat_chunk)).await.is_err() {
                        break;
                    }

                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn edit_files(
        &self,
        _request: Request<EditFilesRequest>,
    ) -> Result<Response<Self::EditFilesStream>, Status> {
        Err(Status::unimplemented("edit_files not implemented"))
    }

    type EditFilesStream = ReceiverStream<Result<EditProgress, Status>>;

    async fn generate_code(
        &self,
        _request: Request<GenerateCodeRequest>,
    ) -> Result<Response<Self::GenerateCodeStream>, Status> {
        Err(Status::unimplemented("generate_code not implemented"))
    }

    type GenerateCodeStream = ReceiverStream<Result<CodeGenerationChunk, Status>>;

    async fn git_operation(
        &self,
        _request: Request<GitOperationRequest>,
    ) -> Result<Response<GitOperationResponse>, Status> {
        Err(Status::unimplemented("git_operation not implemented"))
    }

    async fn list_models(
        &self,
        _request: Request<ListModelsRequest>,
    ) -> Result<Response<ListModelsResponse>, Status> {
        Ok(Response::new(ListModelsResponse {
            models: vec![
                ModelInfo {
                    name: "qwen2.5-coder:32b-instruct-q8_0".to_string(),
                    provider: "ollama".to_string(),
                    description: Some("Coder / Reviewer — Rust implementation, debugging, security audit".to_string()),
                    context_window: Some(65_536),
                    cost_per_1k_tokens: Some(0.0),
                },
                ModelInfo {
                    name: "deepseek-r1:32b".to_string(),
                    provider: "ollama".to_string(),
                    description: Some("Architect / Summarizer / DocRag — reasoning model, design and documentation".to_string()),
                    context_window: Some(131_072),
                    cost_per_1k_tokens: Some(0.0),
                },
                ModelInfo {
                    name: "llama3.2:3b-instruct-q8_0".to_string(),
                    provider: "ollama".to_string(),
                    description: Some("Router — intent classification dispatcher".to_string()),
                    context_window: Some(16_384),
                    cost_per_1k_tokens: Some(0.0),
                },
                ModelInfo {
                    name: "llama3.2:1b-instruct-q8_0".to_string(),
                    provider: "ollama".to_string(),
                    description: Some("CliGit — commit messages, shell commands, file ops".to_string()),
                    context_window: Some(8_192),
                    cost_per_1k_tokens: Some(0.0),
                },
                ModelInfo {
                    name: "llama3.2-vision:11b".to_string(),
                    provider: "ollama".to_string(),
                    description: Some("Vision — UI layout audit from screenshots".to_string()),
                    context_window: Some(32_768),
                    cost_per_1k_tokens: Some(0.0),
                },
            ],
        }))
    }

    async fn get_session_info(
        &self,
        _request: Request<SessionInfoRequest>,
    ) -> Result<Response<SessionInfoResponse>, Status> {
        Err(Status::unimplemented("get_session_info not implemented"))
    }

    async fn close_session(
        &self,
        request: Request<CloseSessionRequest>,
    ) -> Result<Response<CloseSessionResponse>, Status> {
        let req = request.into_inner();
        let mut manager = self.session_manager.write().await;
        manager.remove_session(&req.session_id);
        tracing::info!("🗑 Session closed: {}", req.session_id);
        Ok(Response::new(CloseSessionResponse {
            success: true,
            history_file: None,
        }))
    }
}
