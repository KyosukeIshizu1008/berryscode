//! REST client for berry-core-api
//! Replaces gRPC client with HTTP REST calls to berry-core-api

use anyhow::{Context, Result};

/// REST client for berry-core-api
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

    /// Health check — returns true if berry-core-api is reachable
    pub async fn is_healthy(&self) -> bool {
        self.http
            .get(format!("{}/health", self.base_url))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    /// POST /plan — send a prompt and get AI response
    pub async fn chat(
        &self,
        repo_path: &str,
        message: &str,
        model: Option<&str>,
    ) -> Result<String> {
        let body = serde_json::json!({
            "repo_path": repo_path,
            "prompt": message,
            "coder_model": model.unwrap_or("claude-sonnet-4-20250514"),
        });

        let resp = self
            .http
            .post(format!("{}/plan", self.base_url))
            .json(&body)
            .send()
            .await
            .context("Failed to send request to berry-core-api")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "berry-core-api returned {}: {}",
                status,
                text
            ));
        }

        let json: serde_json::Value = resp.json().await.context("Failed to parse response")?;

        // Extract the response text from the plan response
        if let Some(explanation) = json.get("explanation").and_then(|e| e.as_str()) {
            Ok(explanation.to_string())
        } else if let Some(plan) = json.get("plan") {
            Ok(serde_json::to_string_pretty(plan).unwrap_or_default())
        } else {
            Ok(json.to_string())
        }
    }

    /// POST /edit — apply edits to files
    pub async fn edit(&self, repo_path: &str, prompt: &str, model: Option<&str>) -> Result<String> {
        let body = serde_json::json!({
            "repo_path": repo_path,
            "prompt": prompt,
            "coder_model": model.unwrap_or("claude-sonnet-4-20250514"),
        });

        let resp = self
            .http
            .post(format!("{}/edit", self.base_url))
            .json(&body)
            .send()
            .await
            .context("Failed to send edit request")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "berry-core-api edit returned {}: {}",
                status,
                text
            ));
        }

        let json: serde_json::Value = resp.json().await.context("Failed to parse edit response")?;
        Ok(json.to_string())
    }

    /// POST /review — code review
    pub async fn review(&self, repo_path: &str, prompt: &str) -> Result<String> {
        let body = serde_json::json!({
            "repo_path": repo_path,
            "prompt": prompt,
        });

        let resp = self
            .http
            .post(format!("{}/review", self.base_url))
            .json(&body)
            .send()
            .await
            .context("Failed to send review request")?;

        let json: serde_json::Value = resp
            .json()
            .await
            .context("Failed to parse review response")?;
        Ok(json.to_string())
    }

    /// POST /search — semantic code search
    pub async fn search(&self, repo_path: &str, query: &str) -> Result<String> {
        let body = serde_json::json!({
            "repo_path": repo_path,
            "query": query,
        });

        let resp = self
            .http
            .post(format!("{}/search", self.base_url))
            .json(&body)
            .send()
            .await
            .context("Failed to send search request")?;

        let json: serde_json::Value = resp.json().await?;
        Ok(json.to_string())
    }
}

/// Global singleton
static REST_CLIENT: once_cell::sync::Lazy<RestClient> = once_cell::sync::Lazy::new(|| {
    let url =
        std::env::var("BERRY_CORE_API_URL").unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());
    RestClient::new(url)
});

pub fn get_client() -> &'static RestClient {
    &REST_CLIENT
}
