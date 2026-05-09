//! Ollama (local) provider — talks to a self-hosted Ollama server,
//! defaults to <http://localhost:11434>. Uses the OpenAI-compatible
//! `/v1/chat/completions` shim that ships with Ollama 0.1.30+ so the
//! code-path mirrors `openai.rs` and supports the same model list.
//!
//! Free functions [`probe_version`] and [`list_installed_models`] are
//! used by the Settings UI to surface "is the server up?" status and
//! a dynamic model picker respectively. They're plain HTTP calls — no
//! `Provider` instance required — so the UI can call them with just
//! the endpoint string.

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

// ─── Standalone probes (used by Settings UI) ─────────────────────

/// Hit `/api/version` against the given Ollama endpoint. Returns the
/// reported server version on success, `None` if the server is not
/// reachable (timeout, connection refused, non-2xx, malformed JSON).
///
/// Has a tight 2 s timeout so a misconfigured endpoint doesn't freeze
/// the Settings panel for the default 30+ s reqwest timeout.
pub async fn probe_version(endpoint: &str) -> Option<String> {
    let url = format!("{}/api/version", endpoint.trim_end_matches('/'));
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .ok()?;
    let resp = http.get(&url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let json: serde_json::Value = resp.json().await.ok()?;
    json.get("version")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Hit `/api/tags` and return the names of locally installed models in
/// the order Ollama reports them. Returns `None` if the probe fails.
/// Names are the canonical `name:tag` form (e.g. `qwen2.5-coder:7b`).
pub async fn list_installed_models(endpoint: &str) -> Option<Vec<String>> {
    let url = format!("{}/api/tags", endpoint.trim_end_matches('/'));
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .ok()?;
    let resp = http.get(&url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let json: serde_json::Value = resp.json().await.ok()?;
    let models = json.get("models")?.as_array()?;
    let names: Vec<String> = models
        .iter()
        .filter_map(|m| {
            m.get("name")
                .and_then(|n| n.as_str())
                .map(|s| s.to_string())
        })
        .collect();
    Some(names)
}
