//! LLM API client integration
//!
//! This module has been migrated to use berry_api's UnifiedLLMService via gRPC.
//! The public API remains the same, but the internal implementation now delegates
//! to the centralized LLM service running in berry_api.

use crate::berrycode::Result;
use crate::berrycode::models::Model;
use crate::berrycode::tools::{Tool, ToolCall};
use crate::berrycode::llm_grpc_client::LLMGrpcClient;
use serde::{Deserialize, Serialize};
use anyhow::anyhow;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl Message {
    pub fn user(content: String) -> Self {
        Self {
            role: "user".to_string(),
            content: Some(content),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn assistant(content: String) -> Self {
        Self {
            role: "assistant".to_string(),
            content: Some(content),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn assistant_with_tools(tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: None,
            tool_calls: Some(tool_calls),
            tool_call_id: None,
        }
    }

    pub fn tool(tool_call_id: String, content: String) -> Self {
        Self {
            role: "tool".to_string(),
            content: Some(content),
            tool_calls: None,
            tool_call_id: Some(tool_call_id),
        }
    }

    pub fn system(content: String) -> Self {
        Self {
            role: "system".to_string(),
            content: Some(content),
            tool_calls: None,
            tool_call_id: None,
        }
    }
}

#[derive(Debug, Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<Tool>>,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    choices: Vec<Choice>,
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
    #[allow(dead_code)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct StreamChoice {
    delta: Delta,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Delta {
    content: Option<String>,
    tool_calls: Option<Vec<ToolCall>>,
}

/// Response from LLM (supports both text and tool calls)
#[derive(Debug, Clone)]
pub enum LLMResponse {
    Text(String),
    ToolCalls(Vec<ToolCall>),
}

#[derive(Debug, Deserialize)]
struct Usage {
    prompt_tokens: usize,
    completion_tokens: usize,
    #[allow(dead_code)]
    total_tokens: usize,
}

#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<Vec<AnthropicContent>>,
    messages: Vec<AnthropicMessage>,
    max_tokens: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Debug, Serialize, Clone)]
struct AnthropicMessage {
    role: String,
    content: Vec<AnthropicContent>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct AnthropicContent {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    cache_control: Option<CacheControl>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct CacheControl {
    #[serde(rename = "type")]
    cache_type: String,  // "ephemeral"
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<ContentBlock>,
    usage: Option<AnthropicUsage>,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    text: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: usize,
    output_tokens: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    cache_creation_input_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cache_read_input_tokens: Option<usize>,
}

pub struct LLMClient {
    grpc_client: Mutex<LLMGrpcClient>,
    model: String,
    provider: LLMProvider,
}

#[derive(Debug, Clone)]
pub enum LLMProvider {
    OpenAI,
    Anthropic,
    OpenRouter,
    Custom,
}

impl LLMClient {
    pub fn new(model: &Model, api_key: String) -> Result<Self> {
        let provider = Self::detect_provider(&model.name);
        let provider_name = Self::provider_to_string(&provider);

        // Get berry_api server address from environment
        let server_addr = std::env::var("BERRY_API_GRPC_ADDR")
            .unwrap_or_else(|_| "http://localhost:50051".to_string());

        // Create gRPC client (async, will be connected on first use)
        let grpc_client = tokio::runtime::Runtime::new()
            .map_err(|e| anyhow!("Failed to create tokio runtime: {}", e))?
            .block_on(async {
                let mut client = LLMGrpcClient::connect(&server_addr, &provider_name, &model.name).await?;
                client.set_api_key(api_key);

                // Override with environment variable if set
                if let Ok(env_base) = std::env::var("OPENAI_API_BASE") {
                    client.set_api_base(env_base);
                }

                Ok::<_, anyhow::Error>(client)
            })?;

        Ok(Self {
            grpc_client: Mutex::new(grpc_client),
            model: model.name.clone(),
            provider,
        })
    }

    fn detect_provider(model_name: &str) -> LLMProvider {
        if model_name.contains("gpt") || model_name.contains("o1") || model_name.contains("o3") || model_name.contains("deepseek") {
            LLMProvider::OpenAI
        } else if model_name.contains("claude") || model_name.contains("sonnet") || model_name.contains("opus") {
            LLMProvider::Anthropic
        } else if model_name.contains("llama") {
            LLMProvider::Custom // Ollama
        } else {
            LLMProvider::Custom
        }
    }

    fn provider_to_string(provider: &LLMProvider) -> String {
        match provider {
            LLMProvider::OpenAI => "openai".to_string(),
            LLMProvider::Anthropic => "anthropic".to_string(),
            LLMProvider::OpenRouter => "openai".to_string(), // OpenRouter uses OpenAI format
            LLMProvider::Custom => "ollama".to_string(),
        }
    }

    pub async fn chat(&self, messages: Vec<Message>) -> Result<(String, usize, usize)> {
        let mut client = self.grpc_client.lock().await;
        client.chat(messages).await
    }

    /// Chat with optional prefill (forces AI to start response with specific text)
    /// Only works with Anthropic provider
    pub async fn chat_with_prefill(&self, messages: Vec<Message>, prefill: Option<String>) -> Result<(String, usize, usize)> {
        match self.provider {
            LLMProvider::Anthropic => {
                let mut client = self.grpc_client.lock().await;
                client.chat_with_prefill(messages, prefill).await
            }
            _ => {
                tracing::warn!("Prefill is only supported for Anthropic provider, ignoring");
                self.chat(messages).await
            }
        }
    }

    /// Chat with tool support
    pub async fn chat_with_tools(&self, messages: Vec<Message>, tools: Vec<Tool>) -> Result<(LLMResponse, usize, usize)> {
        let mut client = self.grpc_client.lock().await;
        client.chat_with_tools(messages, tools).await
    }

    /// Chat with tools and streaming support
    pub async fn chat_with_tools_stream<F>(
        &self,
        messages: Vec<Message>,
        tools: Vec<Tool>,
        on_chunk: F,
    ) -> Result<(LLMResponse, usize, usize)>
    where
        F: FnMut(&str) + Send,
    {
        let mut client = self.grpc_client.lock().await;
        client.chat_with_tools_stream(messages, tools, on_chunk).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_provider() {
        assert!(matches!(
            LLMClient::detect_provider("gpt-4"),
            LLMProvider::OpenAI
        ));
        assert!(matches!(
            LLMClient::detect_provider("claude-3-opus"),
            LLMProvider::Anthropic
        ));
        assert!(matches!(
            LLMClient::detect_provider("llama4:scout"),
            LLMProvider::Custom
        ));
    }
}
