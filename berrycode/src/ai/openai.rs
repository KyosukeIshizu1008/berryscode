//! OpenAI (GPT / Codex) provider — direct calls to
//! <https://api.openai.com/v1/chat/completions> with the user's key.

use super::{
    CompletionRequest, CompletionResponse, Provider, ProviderError, ProviderKind, TokenUsage,
};

const API_BASE: &str = "https://api.openai.com/v1";

pub struct OpenAiProvider {
    api_key: String,
    http: reqwest::Client,
}

impl OpenAiProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("HTTP client"),
        }
    }
}

#[async_trait::async_trait]
impl Provider for OpenAiProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::OpenAi
    }

    async fn complete(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse, ProviderError> {
        if self.api_key.is_empty() {
            return Err(ProviderError::MissingKey("OpenAI"));
        }

        // OpenAI's `chat/completions` accepts `system`, `user`,
        // `assistant` roles all in one `messages` array, so we promote
        // the explicit system prompt into a synthetic message.
        let mut messages: Vec<serde_json::Value> = Vec::new();
        if let Some(system) = request.system.as_ref() {
            messages.push(serde_json::json!({ "role": "system", "content": system }));
        }
        for m in &request.messages {
            messages.push(serde_json::json!({ "role": m.role, "content": m.content }));
        }

        let body = serde_json::json!({
            "model": request.model,
            "messages": messages,
            "max_tokens": request.max_tokens,
            "temperature": request.temperature,
        });

        let resp = self
            .http
            .post(format!("{}/chat/completions", API_BASE))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(ProviderError::Status {
                status: status.as_u16(),
                body,
            });
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ProviderError::Parse(e.to_string()))?;

        let text = json
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string();

        let usage = json.get("usage").map(|u| TokenUsage {
            prompt_tokens: u.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            completion_tokens: u
                .get("completion_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32,
        });

        Ok(CompletionResponse { text, usage })
    }
}
