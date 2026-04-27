//! Anthropic (Claude) provider — direct API calls with the user's
//! own key, no proxy. Uses the standard `messages` endpoint at
//! <https://api.anthropic.com/v1/messages>.

use super::{
    ChatMessage, CompletionRequest, CompletionResponse, Provider, ProviderError, ProviderKind,
    TokenUsage,
};

const API_BASE: &str = "https://api.anthropic.com/v1";
const ANTHROPIC_VERSION: &str = "2023-06-01";

pub struct AnthropicProvider {
    api_key: String,
    http: reqwest::Client,
}

impl AnthropicProvider {
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
impl Provider for AnthropicProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Anthropic
    }

    async fn complete(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse, ProviderError> {
        if self.api_key.is_empty() {
            return Err(ProviderError::MissingKey("Anthropic"));
        }

        // Anthropic separates the system prompt from `messages` and
        // restricts roles to `user` / `assistant`.
        let messages: Vec<serde_json::Value> = request
            .messages
            .iter()
            .filter(|m| m.role != "system")
            .map(|m: &ChatMessage| serde_json::json!({ "role": m.role, "content": m.content }))
            .collect();

        let mut body = serde_json::json!({
            "model": request.model,
            "max_tokens": request.max_tokens,
            "temperature": request.temperature,
            "messages": messages,
        });
        if let Some(system) = request.system.as_ref() {
            body["system"] = serde_json::Value::String(system.clone());
        }

        let resp = self
            .http
            .post(format!("{}/messages", API_BASE))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
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

        // Concatenate every text block in the `content` array. The API
        // also returns `tool_use` blocks etc. — those are ignored at
        // this stage and will be wired up when agent mode lands in
        // Phase 4.
        let text = json
            .get("content")
            .and_then(|c| c.as_array())
            .map(|blocks| {
                blocks
                    .iter()
                    .filter_map(|b| {
                        if b.get("type")?.as_str()? == "text" {
                            b.get("text")?.as_str().map(|s| s.to_string())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("")
            })
            .unwrap_or_default();

        let usage = json.get("usage").map(|u| TokenUsage {
            prompt_tokens: u.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            completion_tokens: u.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        });

        Ok(CompletionResponse { text, usage })
    }
}
