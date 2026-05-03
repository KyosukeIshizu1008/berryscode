//! REST client for `berry-core-api` (OpenAPI 0.1.0, default port 7001).
//!
//! Tracks the upstream API surface verbatim — see
//! `http://<host>:7001/openapi.yaml` for the authoritative spec. v0.7.10
//! refreshes this client to match the API rev that landed on
//! `192.168.10.147:7001`: the legacy `/plan` chat path moved to `/chat`,
//! `/edit` switched from `{repo_path, prompt, coder_model}` to
//! file-based `{file_path, new_code, target, run_tests}`, and `/review`
//! / `/search` gained typed parameters (`target_files`, `limit`,
//! `collection_name`) so callers no longer roll the request body by
//! hand.
//!
//! Methods are 1:1 with the OpenAPI operations; if the server adds a
//! new endpoint, add a method here rather than letting callers build
//! ad-hoc `reqwest` requests.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct RestClient {
    http: reqwest::Client,
    base_url: String,
}

impl RestClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("Failed to create HTTP client"),
            base_url: base_url.into(),
        }
    }

    /// `GET /health` — Ollama + Qdrant probe. Returns `true` if the
    /// server itself is reachable, regardless of which downstream is
    /// healthy. Per spec the endpoint always returns 200; an
    /// unreachable berry-core-api is the only way this returns `false`.
    pub async fn is_healthy(&self) -> bool {
        self.http
            .get(format!("{}/health", self.base_url))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    /// `POST /chat` — generic Ollama chat. Returns the assistant's
    /// reply as a string. Streaming (`stream: true`) is not yet wired
    /// — the editor consumes whole responses for now.
    pub async fn chat(&self, req: ChatRequest) -> Result<ChatResponse> {
        let resp = self
            .http
            .post(format!("{}/chat", self.base_url))
            .json(&req)
            .send()
            .await
            .context("Failed to send /chat request")?;
        ensure_2xx(&resp.status(), "POST /chat")?;
        resp.json().await.context("Failed to parse /chat response")
    }

    /// `POST /edit` — overwrite a file (or replace a target block) and
    /// optionally run `cargo check` / `cargo test`.
    pub async fn edit(&self, req: EditRequest) -> Result<EditResponse> {
        let resp = self
            .http
            .post(format!("{}/edit", self.base_url))
            .json(&req)
            .send()
            .await
            .context("Failed to send /edit request")?;
        ensure_2xx(&resp.status(), "POST /edit")?;
        resp.json().await.context("Failed to parse /edit response")
    }

    /// `POST /review` — LLM code review. `target_files` defaults to
    /// `None` so the LLM picks files from the repo tree itself.
    pub async fn review(&self, req: ReviewRequest) -> Result<serde_json::Value> {
        let resp = self
            .http
            .post(format!("{}/review", self.base_url))
            .json(&req)
            .send()
            .await
            .context("Failed to send /review request")?;
        ensure_2xx(&resp.status(), "POST /review")?;
        resp.json()
            .await
            .context("Failed to parse /review response")
    }

    /// `POST /search` — semantic search across the indexed Qdrant
    /// collection.
    pub async fn search(&self, req: SearchRequest) -> Result<SearchResponse> {
        let resp = self
            .http
            .post(format!("{}/search", self.base_url))
            .json(&req)
            .send()
            .await
            .context("Failed to send /search request")?;
        ensure_2xx(&resp.status(), "POST /search")?;
        resp.json()
            .await
            .context("Failed to parse /search response")
    }

    /// `POST /plan` — produce a `FilePlan[]` without writing anything.
    /// `coder_model` starting with `claude-` routes through the Anthropic
    /// API; otherwise it's an Ollama model name.
    pub async fn plan(&self, req: PlanRequest) -> Result<serde_json::Value> {
        let resp = self
            .http
            .post(format!("{}/plan", self.base_url))
            .json(&req)
            .send()
            .await
            .context("Failed to send /plan request")?;
        ensure_2xx(&resp.status(), "POST /plan")?;
        resp.json().await.context("Failed to parse /plan response")
    }
}

fn ensure_2xx(status: &reqwest::StatusCode, op: &str) -> Result<()> {
    if status.is_success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("{op} returned {status}"))
    }
}

// ── Request / response shapes — mirror the OpenAPI 0.1.0 schemas ────────

#[derive(Debug, Clone, Serialize)]
pub struct ChatRequest {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(default)]
    pub stream: bool,
}

impl ChatRequest {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            model: None,
            session_id: None,
            system: None,
            stream: false,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatResponse {
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EditRequest {
    pub file_path: String,
    pub new_code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(default)]
    pub run_tests: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EditResponse {
    #[serde(default)]
    pub success: bool,
    #[serde(default)]
    pub stdout: String,
    #[serde(default)]
    pub stderr: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReviewRequest {
    pub repo_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_files: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instruction: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coder_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchRequest {
    pub query: String,
    #[serde(default = "default_search_limit")]
    pub limit: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection_name: Option<String>,
}

fn default_search_limit() -> u32 {
    5
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchResponse {
    #[serde(default)]
    pub results: Vec<SearchResult>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchResult {
    pub file_path: String,
    #[serde(default)]
    pub chunk_index: u32,
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlanRequest {
    pub repo_path: String,
    pub instruction: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coder_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_model: Option<String>,
}

/// Global singleton — `BERRY_CORE_API_URL` overrides the default. The
/// default tracks the LAN host the editor's OracleBerry panel was
/// originally pointed at.
static REST_CLIENT: once_cell::sync::Lazy<RestClient> = once_cell::sync::Lazy::new(|| {
    let url = std::env::var("BERRY_CORE_API_URL")
        .unwrap_or_else(|_| "http://192.168.10.147:7001".to_string());
    RestClient::new(url)
});

pub fn get_client() -> &'static RestClient {
    &REST_CLIENT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_request_skips_optional_nulls() {
        let r = ChatRequest::new("hi");
        let s = serde_json::to_string(&r).unwrap();
        assert!(s.contains("\"message\":\"hi\""));
        assert!(!s.contains("model"), "None model should be skipped: {s}");
        assert!(
            !s.contains("session_id"),
            "None session_id should be skipped: {s}"
        );
    }

    #[test]
    fn edit_request_omits_target_when_none() {
        let r = EditRequest {
            file_path: "/x.rs".into(),
            new_code: "fn x(){}".into(),
            target: None,
            run_tests: false,
        };
        let s = serde_json::to_string(&r).unwrap();
        assert!(!s.contains("target"), "None target should be skipped: {s}");
        assert!(s.contains("\"run_tests\":false"));
    }

    #[test]
    fn search_request_default_limit_is_five() {
        // Mirror the OpenAPI default so callers that don't set a limit
        // still get a stable 5-result page on the server.
        assert_eq!(default_search_limit(), 5);
    }

    #[test]
    fn review_request_serializes_target_files() {
        let r = ReviewRequest {
            repo_path: "/repo".into(),
            target_files: Some(vec!["src/lib.rs".into(), "src/main.rs".into()]),
            instruction: Some("focus on safety".into()),
            collection_name: None,
            coder_model: None,
            session_id: None,
        };
        let s = serde_json::to_string(&r).unwrap();
        assert!(s.contains("\"target_files\":[\"src/lib.rs\",\"src/main.rs\"]"));
        assert!(!s.contains("collection_name"));
    }

    #[test]
    fn plan_request_keeps_coder_model_when_set() {
        let r = PlanRequest {
            repo_path: "/repo".into(),
            instruction: "add foo".into(),
            collection_name: None,
            coder_model: Some("claude-sonnet-4-6".into()),
            reasoning_model: None,
        };
        let s = serde_json::to_string(&r).unwrap();
        assert!(s.contains("\"coder_model\":\"claude-sonnet-4-6\""));
        assert!(!s.contains("reasoning_model"));
    }
}
