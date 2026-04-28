//! Ollama (local) provider — talks to a self-hosted Ollama server,
//! defaults to <http://localhost:11434>. Uses the OpenAI-compatible
//! `/v1/chat/completions` shim that ships with Ollama 0.1.30+ so the
//! code-path mirrors `openai.rs` and supports the same model list.

use super::{
    CompletionRequest, CompletionResponse, Provider, ProviderError, ProviderKind, TokenUsage,
};

pub struct OllamaProvider {
    endpoint: String,
    http: reqwest::Client,
}

impl OllamaProvider {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(180))
                .build()
                .expect("HTTP client"),
        }
    }
}

#[async_trait::async_trait]
impl Provider for OllamaProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Ollama
    }

    async fn complete(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse, ProviderError> {
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
            "stream": false,
            "options": {
                "temperature": request.temperature,
                "num_predict": request.max_tokens,
            },
        });

        let url = format!("{}/api/chat", self.endpoint.trim_end_matches('/'));
        let resp = self.http.post(&url).json(&body).send().await?;

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
            .get("message")
            .and_then(|m| m.get("content"))
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string();

        let usage = Some(TokenUsage {
            prompt_tokens: json
                .get("prompt_eval_count")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32,
            completion_tokens: json.get("eval_count").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            // Ollama runs locally — no API-level prompt cache to expose.
            cache_read_tokens: 0,
            cache_write_tokens: 0,
        });

        Ok(CompletionResponse { text, usage })
    }
}
