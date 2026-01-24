use crate::berry_api::berry_code_service_server::BerryCodeService;
use crate::berry_api::*;
use crate::session::SessionManager;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

pub struct BerryCodeServiceImpl {
    session_manager: Arc<RwLock<SessionManager>>,
}

impl BerryCodeServiceImpl {
    pub fn new() -> Self {
        Self {
            session_manager: Arc::new(RwLock::new(SessionManager::new())),
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

        tracing::info!("📝 Starting new session: project_path={:?}", req.project_path);

        let session_id = uuid::Uuid::new_v4().to_string();
        let model = req.model.clone().unwrap_or_else(|| "gpt-4".to_string());
        let mode = req.mode.clone().unwrap_or_else(|| "code".to_string());

        // Store session in manager
        let mut manager = self.session_manager.write().await;
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

        tracing::info!("💬 Chat request: session={}, message={}", req.session_id, req.message);

        let (tx, rx) = tokio::sync::mpsc::channel(100);

        // Spawn async task to handle streaming response
        tokio::spawn(async move {
            // Simulate LLM streaming response
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
                    name: "gpt-4".to_string(),
                    provider: "openai".to_string(),
                    description: Some("GPT-4 Model".to_string()),
                    context_window: Some(8192),
                    cost_per_1k_tokens: Some(0.03),
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
        _request: Request<CloseSessionRequest>,
    ) -> Result<Response<CloseSessionResponse>, Status> {
        Ok(Response::new(CloseSessionResponse {
            success: true,
            history_file: None,
        }))
    }
}
