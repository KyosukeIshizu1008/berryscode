//! BerryCode AI provider layer.
//!
//! v0.4.5 introduces **bring-your-own-key (BYOK)** AI integration: the
//! IDE talks to Claude / GPT / Ollama directly with the user's own API
//! credentials, instead of routing through a hosted backend.
//!
//! - `Provider` trait — minimal `complete` / `stream` surface every
//!   backend implements.
//! - `anthropic`, `openai`, `ollama` — concrete providers.
//! - `settings` — persisted BYOK configuration (per-provider keys + the
//!   model selected for chat / inline-completion).
//!
//! The legacy [`crate::app::ai_chat`] module still exists and continues
//! to drive the chat sidebar via `berry-core-api` over REST; once Phase
//! 1 stabilises we'll switch the chat panel to call this layer instead.

pub mod settings;

pub mod anthropic;
pub mod ollama;
pub mod openai;

use serde::{Deserialize, Serialize};

/// One turn in a conversation. The `role` is `"user"`, `"assistant"`,
/// or `"system"`. Multi-modal attachments aren't represented yet —
/// they'll come in Phase 2 with the chat sidebar refresh.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Generation request. Each provider maps these fields onto its native
/// API; fields the provider doesn't support are silently ignored.
#[derive(Debug, Clone)]
pub struct CompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub system: Option<String>,
    pub max_tokens: u32,
    pub temperature: f32,
}

impl CompletionRequest {
    pub fn new(model: impl Into<String>, messages: Vec<ChatMessage>) -> Self {
        Self {
            model: model.into(),
            messages,
            system: None,
            max_tokens: 1024,
            temperature: 0.2,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CompletionResponse {
    pub text: String,
    /// Optional usage stats (prompt / completion tokens) when the
    /// provider returns them. Used by the cost panel in Phase 5.
    pub usage: Option<TokenUsage>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}

/// Identifies which underlying API a configuration entry talks to. The
/// concrete model name (`claude-opus-4-7`, `gpt-5-codex`, …) lives in
/// [`settings::AiSettings`] alongside the API key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderKind {
    Anthropic,
    OpenAi,
    Ollama,
}

impl ProviderKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Anthropic => "Anthropic (Claude)",
            Self::OpenAi => "OpenAI (GPT / Codex)",
            Self::Ollama => "Ollama (local)",
        }
    }
}

/// Errors that can come out of any provider. Kept deliberately small —
/// the UI just shows the message, retries are explicit user actions.
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("API key not configured for {0}")]
    MissingKey(&'static str),
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("provider returned {status}: {body}")]
    Status { status: u16, body: String },
    #[error("failed to parse response: {0}")]
    Parse(String),
}

/// Async backend that converts a `CompletionRequest` into either a
/// single response or a stream of token chunks. Concrete providers live
/// in `anthropic.rs`, `openai.rs`, `ollama.rs`.
#[async_trait::async_trait]
pub trait Provider: Send + Sync {
    fn kind(&self) -> ProviderKind;

    async fn complete(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse, ProviderError>;
}
