//! AI-powered project structure generator

use anyhow::{Context, Result};
use serde_json;

use super::prompts::{ProjectType, ScaffoldingResponse};

/// Generate project structure using AI
pub async fn generate_project_structure(
    description: &str,
    project_name: &str,
    project_type: ProjectType,
    api_key: &str,
) -> Result<ScaffoldingResponse> {
    let system_prompt = project_type.system_prompt();

    let user_prompt = format!(
        "Project Name: {}\nProject Type: {:?}\n\nDescription:\n{}",
        project_name, project_type, description
    );

    // Call Claude API to generate project structure
    let response = call_claude_api(system_prompt, &user_prompt, api_key).await?;

    // Parse JSON response
    let scaffolding: ScaffoldingResponse = serde_json::from_str(&response)
        .context("Failed to parse AI response as JSON")?;

    // Validate paths (security: prevent ../ escapes)
    for file in &scaffolding.files {
        if file.path.contains("..") || file.path.starts_with('/') {
            anyhow::bail!("Invalid file path: {}", file.path);
        }
    }

    Ok(scaffolding)
}

/// Call Claude API with retry logic
async fn call_claude_api(
    system_prompt: &str,
    user_prompt: &str,
    api_key: &str,
) -> Result<String> {
    use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};

    let client = reqwest::Client::new();

    let mut headers = HeaderMap::new();
    headers.insert("x-api-key", HeaderValue::from_str(api_key)?);
    headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let request_body = serde_json::json!({
        "model": "claude-3-5-sonnet-20241022",
        "max_tokens": 4096,
        "system": system_prompt,
        "messages": [
            {
                "role": "user",
                "content": user_prompt
            }
        ]
    });

    tracing::debug!("Sending request to Claude API for scaffolding");

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .headers(headers)
        .json(&request_body)
        .send()
        .await
        .context("Failed to send request to Claude API")?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        anyhow::bail!("Claude API request failed with status {}: {}", status, error_text);
    }

    let response_json: serde_json::Value = response
        .json()
        .await
        .context("Failed to parse Claude API response")?;

    // Extract text from response
    let text = response_json
        .get("content")
        .and_then(|c| c.as_array())
        .and_then(|arr| arr.first())
        .and_then(|item| item.get("text"))
        .and_then(|t| t.as_str())
        .ok_or_else(|| anyhow::anyhow!("Failed to extract text from Claude response"))?;

    tracing::debug!("Received scaffolding response from Claude");

    Ok(text.to_string())
}
