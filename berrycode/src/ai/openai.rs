//! OpenAI (GPT / Codex) provider.
//!
//! Talks to either the legacy Chat Completions API
//! (<https://api.openai.com/v1/chat/completions>) or, when
//! `OPENAI_BASE_URL` points at a Responses-API endpoint such as Azure
//! OpenAI's `…/openai/responses?api-version=…`, the newer Responses API.
//!
//! Detection rules for `OPENAI_BASE_URL`:
//!   - URL contains `/responses` → POST as-is, Responses-API schema
//!     (`input` / `max_output_tokens` / `instructions`).
//!   - URL ends with `/chat/completions` → POST as-is, legacy schema.
//!   - Otherwise treated as a base; we append `/chat/completions`.

use super::{
    CompletionRequest, CompletionResponse, Provider, ProviderError, ProviderKind, TokenUsage,
};

const DEFAULT_BASE: &str = "https://api.openai.com/v1";

pub struct OpenAiProvider {
    api_key: String,
    endpoint: String,
    use_responses_api: bool,
    http: reqwest::Client,
}

impl OpenAiProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        // Env wins, Settings is the fallback. Matches the behavior
        // every OpenAI SDK / 12-factor app expects, and keeps a single
        // source of truth so users can't get confused about which key
        // is in flight.
        let api_key = std::env::var("OPENAI_API_KEY")
            .ok()
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| api_key.into());

        let (endpoint, use_responses_api) = resolve_endpoint();

        tracing::debug!(
            "OpenAiProvider endpoint = {} (responses_api={})",
            endpoint,
            use_responses_api
        );

        Self {
            api_key,
            endpoint,
            use_responses_api,
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("HTTP client"),
        }
    }
}

fn resolve_endpoint() -> (String, bool) {
    match std::env::var("OPENAI_BASE_URL") {
        Ok(url) if url.contains("/responses") => (url, true),
        Ok(url) if url.trim_end_matches('/').ends_with("/chat/completions") => (url, false),
        Ok(url) => {
            let trimmed = url.trim_end_matches('/');
            (format!("{}/chat/completions", trimmed), false)
        }
        Err(_) => (format!("{}/chat/completions", DEFAULT_BASE), false),
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

        let body = if self.use_responses_api {
            build_responses_body(&request)
        } else {
            build_chat_completions_body(&request)
        };

        let resp = self
            .http
            .post(&self.endpoint)
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

        let (text, usage) = if self.use_responses_api {
            parse_responses_payload(&json)
        } else {
            parse_chat_completions_payload(&json)
        };

        Ok(CompletionResponse { text, usage })
    }
}

fn build_chat_completions_body(request: &CompletionRequest) -> serde_json::Value {
    let mut messages: Vec<serde_json::Value> = Vec::new();
    if let Some(system) = request.system.as_ref() {
        messages.push(serde_json::json!({ "role": "system", "content": system }));
    }
    for m in &request.messages {
        messages.push(serde_json::json!({ "role": m.role, "content": m.content }));
    }
    serde_json::json!({
        "model": request.model,
        "messages": messages,
        "max_tokens": request.max_tokens,
        "temperature": request.temperature,
    })
}

fn build_responses_body(request: &CompletionRequest) -> serde_json::Value {
    let input: Vec<serde_json::Value> = request
        .messages
        .iter()
        .map(|m| serde_json::json!({ "role": m.role, "content": m.content }))
        .collect();

    let mut body = serde_json::json!({
        "model": request.model,
        "input": input,
        "max_output_tokens": request.max_tokens,
    });
    if let Some(system) = request.system.as_ref() {
        body["instructions"] = serde_json::Value::String(system.clone());
    }
    body
}

fn parse_chat_completions_payload(json: &serde_json::Value) -> (String, Option<TokenUsage>) {
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
        // OpenAI doesn't expose prompt-cache stats here.
        cache_read_tokens: 0,
        cache_write_tokens: 0,
    });

    (text, usage)
}

fn parse_responses_payload(json: &serde_json::Value) -> (String, Option<TokenUsage>) {
    // `output[]` carries a mix of `reasoning` and `message` items; we
    // want the first `message`, then concatenate every `output_text`
    // chunk inside its `content[]` (usually one chunk, but the schema
    // allows several when the model interleaves text and tool calls).
    let text = json
        .get("output")
        .and_then(|o| o.as_array())
        .and_then(|arr| {
            arr.iter()
                .find(|item| item.get("type").and_then(|t| t.as_str()) == Some("message"))
        })
        .and_then(|msg| msg.get("content"))
        .and_then(|c| c.as_array())
        .map(|arr| {
            arr.iter()
                .filter(|item| item.get("type").and_then(|t| t.as_str()) == Some("output_text"))
                .filter_map(|item| item.get("text").and_then(|t| t.as_str()))
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();

    let usage = json.get("usage").map(|u| TokenUsage {
        prompt_tokens: u.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        completion_tokens: u.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        cache_read_tokens: u
            .get("input_tokens_details")
            .and_then(|d| d.get("cached_tokens"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32,
        cache_write_tokens: 0,
    });

    (text, usage)
}
