// ! LLM gRPC client for berry_api integration
//!
//! This module provides a gRPC client for the UnifiedLLMService in berry_api.
//! It replaces the direct HTTP API calls with gRPC calls to the centralized LLM service.

use crate::berrycode::Result;
use crate::berrycode::llm::{Message, LLMResponse};
use crate::berrycode::tools::{Tool, ToolCall};
use tonic::transport::Channel;
use anyhow::anyhow;

// Include generated proto code
pub mod llm_proto {
    tonic::include_proto!("berry.llm");
}

use llm_proto::unified_llm_service_client::UnifiedLlmServiceClient;

/// gRPC client for berry_api's UnifiedLLMService
pub struct LLMGrpcClient {
    client: UnifiedLlmServiceClient<Channel>,
    provider: String,
    model: String,
    api_key: Option<String>,
    api_base: Option<String>,
}

impl LLMGrpcClient {
    /// Connect to berry_api gRPC server
    pub async fn connect(server_addr: &str, provider: &str, model: &str) -> Result<Self> {
        let channel = Channel::from_shared(server_addr.to_string())
            .map_err(|e| anyhow!("Invalid server address: {}", e))?
            .connect()
            .await
            .map_err(|e| anyhow!("Failed to connect to berry_api: {}", e))?;

        Ok(Self {
            client: UnifiedLlmServiceClient::new(channel),
            provider: provider.to_string(),
            model: model.to_string(),
            api_key: None,
            api_base: None,
        })
    }

    /// Set API key (overrides environment variable)
    pub fn set_api_key(&mut self, api_key: String) {
        self.api_key = Some(api_key);
    }

    /// Set API base URL (for custom endpoints)
    pub fn set_api_base(&mut self, api_base: String) {
        self.api_base = Some(api_base);
    }

    /// Chat with the LLM (non-streaming)
    pub async fn chat(&mut self, messages: Vec<Message>) -> Result<(String, usize, usize)> {
        let request = llm_proto::ChatRequest {
            provider: self.provider.clone(),
            model: self.model.clone(),
            messages: messages.iter().map(|m| self.convert_message(m)).collect(),
            temperature: Some(0.1),
            max_tokens: Some(8192),
            api_key: self.api_key.clone(),
            api_base: self.api_base.clone(),
            stream: Some(false),
            prefill: None,
        };

        let mut stream = self.client.chat(request).await?.into_inner();

        let mut full_text = String::new();
        let mut input_tokens = 0;
        let mut output_tokens = 0;

        while let Some(chunk) = stream.message().await? {
            if let Some(response_type) = chunk.response_type {
                match response_type {
                    llm_proto::chat_chunk::ResponseType::Text(text) => {
                        full_text.push_str(&text);
                    }
                    llm_proto::chat_chunk::ResponseType::ToolCalls(_) => {
                        // Not supported in non-streaming mode
                    }
                }
            }

            if chunk.is_final {
                if let Some(metadata) = chunk.metadata {
                    input_tokens = metadata.input_tokens as usize;
                    output_tokens = metadata.output_tokens as usize;
                }
            }
        }

        Ok((full_text, input_tokens, output_tokens))
    }

    /// Chat with prefill (Anthropic only)
    pub async fn chat_with_prefill(&mut self, messages: Vec<Message>, prefill: Option<String>) -> Result<(String, usize, usize)> {
        let request = llm_proto::ChatRequest {
            provider: self.provider.clone(),
            model: self.model.clone(),
            messages: messages.iter().map(|m| self.convert_message(m)).collect(),
            temperature: Some(0.1),
            max_tokens: Some(8192),
            api_key: self.api_key.clone(),
            api_base: self.api_base.clone(),
            stream: Some(false),
            prefill,
        };

        let mut stream = self.client.chat(request).await?.into_inner();

        let mut full_text = String::new();
        let mut input_tokens = 0;
        let mut output_tokens = 0;

        while let Some(chunk) = stream.message().await? {
            if let Some(response_type) = chunk.response_type {
                match response_type {
                    llm_proto::chat_chunk::ResponseType::Text(text) => {
                        full_text.push_str(&text);
                    }
                    llm_proto::chat_chunk::ResponseType::ToolCalls(_) => {}
                }
            }

            if chunk.is_final {
                if let Some(metadata) = chunk.metadata {
                    input_tokens = metadata.input_tokens as usize;
                    output_tokens = metadata.output_tokens as usize;
                }
            }
        }

        Ok((full_text, input_tokens, output_tokens))
    }

    /// Chat with tools
    pub async fn chat_with_tools(&mut self, messages: Vec<Message>, tools: Vec<Tool>) -> Result<(LLMResponse, usize, usize)> {
        let chat_request = llm_proto::ChatRequest {
            provider: self.provider.clone(),
            model: self.model.clone(),
            messages: messages.iter().map(|m| self.convert_message(m)).collect(),
            temperature: Some(0.1),
            max_tokens: Some(8192),
            api_key: self.api_key.clone(),
            api_base: self.api_base.clone(),
            stream: Some(false),
            prefill: None,
        };

        let request = llm_proto::ChatWithToolsRequest {
            chat_request: Some(chat_request),
            tools: tools.iter().map(|t| self.convert_tool(t)).collect(),
        };

        let mut stream = self.client.chat_with_tools(request).await?.into_inner();

        let mut full_text = String::new();
        let mut tool_calls: Option<Vec<ToolCall>> = None;
        let mut input_tokens = 0;
        let mut output_tokens = 0;

        while let Some(chunk) = stream.message().await? {
            if let Some(response_type) = chunk.response_type {
                match response_type {
                    llm_proto::chat_chunk::ResponseType::Text(text) => {
                        full_text.push_str(&text);
                    }
                    llm_proto::chat_chunk::ResponseType::ToolCalls(tc_list) => {
                        tool_calls = Some(tc_list.calls.iter().map(|tc| self.convert_tool_call_from_proto(tc)).collect());
                    }
                }
            }

            if chunk.is_final {
                if let Some(metadata) = chunk.metadata {
                    input_tokens = metadata.input_tokens as usize;
                    output_tokens = metadata.output_tokens as usize;
                }
            }
        }

        let response = if let Some(calls) = tool_calls {
            LLMResponse::ToolCalls(calls)
        } else if !full_text.is_empty() {
            LLMResponse::Text(full_text)
        } else {
            return Err(anyhow!("Empty response from LLM").into());
        };

        Ok((response, input_tokens, output_tokens))
    }

    /// Chat with streaming
    pub async fn chat_with_tools_stream<F>(
        &mut self,
        messages: Vec<Message>,
        tools: Vec<Tool>,
        mut on_chunk: F,
    ) -> Result<(LLMResponse, usize, usize)>
    where
        F: FnMut(&str) + Send,
    {
        let chat_request = llm_proto::ChatRequest {
            provider: self.provider.clone(),
            model: self.model.clone(),
            messages: messages.iter().map(|m| self.convert_message(m)).collect(),
            temperature: Some(0.1),
            max_tokens: Some(8192),
            api_key: self.api_key.clone(),
            api_base: self.api_base.clone(),
            stream: Some(true),
            prefill: None,
        };

        let request = llm_proto::ChatWithToolsRequest {
            chat_request: Some(chat_request),
            tools: tools.iter().map(|t| self.convert_tool(t)).collect(),
        };

        let mut stream = self.client.chat_with_tools(request).await?.into_inner();

        let mut full_text = String::new();
        let mut tool_calls: Option<Vec<ToolCall>> = None;
        let mut input_tokens = 0;
        let mut output_tokens = 0;

        while let Some(chunk) = stream.message().await? {
            if let Some(response_type) = chunk.response_type {
                match response_type {
                    llm_proto::chat_chunk::ResponseType::Text(text) => {
                        if !text.is_empty() {
                            on_chunk(&text);
                            full_text.push_str(&text);
                        }
                    }
                    llm_proto::chat_chunk::ResponseType::ToolCalls(tc_list) => {
                        tool_calls = Some(tc_list.calls.iter().map(|tc| self.convert_tool_call_from_proto(tc)).collect());
                    }
                }
            }

            if chunk.is_final {
                if let Some(metadata) = chunk.metadata {
                    input_tokens = metadata.input_tokens as usize;
                    output_tokens = metadata.output_tokens as usize;
                }
            }
        }

        let response = if let Some(calls) = tool_calls {
            LLMResponse::ToolCalls(calls)
        } else if !full_text.is_empty() {
            LLMResponse::Text(full_text)
        } else {
            return Err(anyhow!("Empty response from LLM").into());
        };

        Ok((response, input_tokens, output_tokens))
    }

    /// List available providers
    pub async fn list_providers(&mut self) -> Result<Vec<String>> {
        let request = llm_proto::ListProvidersRequest {};
        let response = self.client.list_providers(request).await?.into_inner();

        Ok(response.providers.iter().map(|p| p.name.clone()).collect())
    }

    /// List available models
    pub async fn list_models(&mut self, provider: Option<String>) -> Result<Vec<String>> {
        let request = llm_proto::ListModelsRequest { provider };
        let response = self.client.list_models(request).await?.into_inner();

        Ok(response.models.iter().map(|m| m.name.clone()).collect())
    }

    // Helper methods for type conversion

    fn convert_message(&self, msg: &Message) -> llm_proto::Message {
        llm_proto::Message {
            role: msg.role.clone(),
            content: msg.content.clone(),
            tool_calls: vec![], // TODO: Convert tool calls if needed
            tool_call_id: msg.tool_call_id.clone(),
        }
    }

    fn convert_tool(&self, tool: &Tool) -> llm_proto::Tool {
        llm_proto::Tool {
            r#type: "function".to_string(),
            function: Some(llm_proto::FunctionDefinition {
                name: tool.function.name.clone(),
                description: tool.function.description.clone(),
                parameters: tool.function.parameters.to_string(),
            }),
        }
    }

    fn convert_tool_call_from_proto(&self, tc: &llm_proto::ToolCall) -> ToolCall {
        ToolCall {
            id: tc.id.clone(),
            tool_type: tc.r#type.clone(),
            function: crate::berrycode::tools::FunctionCall {
                name: tc.function.as_ref().map(|f| f.name.clone()).unwrap_or_default(),
                arguments: tc.function.as_ref().map(|f| f.arguments.clone()).unwrap_or_default(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires berry_api server running
    async fn test_grpc_client_connection() {
        let result = LLMGrpcClient::connect("http://localhost:50051", "ollama", "llama4:scout").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore] // Requires berry_api server running
    async fn test_list_providers() {
        let mut client = LLMGrpcClient::connect("http://localhost:50051", "ollama", "llama4:scout")
            .await
            .unwrap();
        let providers = client.list_providers().await.unwrap();
        assert!(!providers.is_empty());
    }
}
