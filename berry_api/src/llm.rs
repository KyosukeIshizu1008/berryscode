// LLM integration module - Ollama support with Tool Calling
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;

// Chat API structures (for tool calling)
#[derive(Debug, Clone, Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OllamaTool>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Debug, Clone, Serialize)]
struct OllamaTool {
    #[serde(rename = "type")]
    tool_type: String,
    function: OllamaFunction,
}

#[derive(Debug, Clone, Serialize)]
struct OllamaFunction {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    message: OllamaMessage,
    done: bool,
}

#[derive(Debug, Deserialize)]
struct OllamaChatStreamResponse {
    model: String,
    message: OllamaMessage,
    done: bool,
}

#[derive(Debug, Deserialize)]
struct ToolCall {
    #[serde(rename = "type")]
    _tool_type: String,
    name: String,
    parameters: serde_json::Value,
}

/// Parse tool call from various formats (JSON or XML-like)
fn parse_tool_call(content: &str) -> Option<(String, serde_json::Value)> {
    // Try JSON format first: {"type": "function", "name": "tool_name", "parameters": {...}}
    if let Ok(tool_call) = serde_json::from_str::<ToolCall>(content) {
        return Some((tool_call.name, tool_call.parameters));
    }

    // Also try to find JSON within the content
    if let Some(start) = content.find("{\"type\"") {
        if let Some(end) = content[start..].rfind('}') {
            let json_str = &content[start..start + end + 1];
            if let Ok(tool_call) = serde_json::from_str::<ToolCall>(json_str) {
                return Some((tool_call.name, tool_call.parameters));
            }
        }
    }

    // Try XML-like format: <function=tool_name><parameter=key>value</parameter>...
    // This format can appear anywhere in the content, even after explanatory text
    if content.contains("<function=") {
        // Extract function name
        let function_name = if let Some(start) = content.find("<function=") {
            let start = start + "<function=".len();
            if let Some(end) = content[start..].find('>') {
                content[start..start + end].trim().to_string()
            } else {
                return None;
            }
        } else {
            return None;
        };

        // Extract parameters
        let mut parameters = serde_json::Map::new();
        let mut search_pos = 0;

        while let Some(param_start) = content[search_pos..].find("<parameter=") {
            let param_start = search_pos + param_start;
            let name_start = param_start + "<parameter=".len();

            if let Some(name_end) = content[name_start..].find('>') {
                let param_name = content[name_start..name_start + name_end].trim().to_string();
                let value_start = name_start + name_end + 1;

                if let Some(value_end) = content[value_start..].find("</parameter>") {
                    let param_value = content[value_start..value_start + value_end].trim().to_string();
                    parameters.insert(param_name, serde_json::Value::String(param_value));
                    search_pos = value_start + value_end + "</parameter>".len();
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        if !parameters.is_empty() {
            tracing::debug!("📝 Parsed XML tool call: {} with {} parameters", function_name, parameters.len());
            return Some((function_name, serde_json::Value::Object(parameters)));
        }
    }

    None
}

#[derive(Debug, Clone)]
pub enum ModelType {
    Design,   // llama4:scout - for design, architecture, planning
    Coding,   // qwen3-coder:30b - for coding, implementation
}

impl ModelType {
    fn model_name(&self) -> &str {
        match self {
            ModelType::Design => "llama4:scout",
            ModelType::Coding => "qwen3-coder:30b",
        }
    }

    /// Auto-detect model type based on message content
    pub fn detect_from_message(message: &str) -> Self {
        let message_lower = message.to_lowercase();

        // Keywords indicating coding tasks
        let coding_keywords = [
            "コード", "実装", "バグ", "エラー", "関数", "クラス",
            "code", "implement", "bug", "error", "function", "class",
            "fix", "debug", "refactor", "test",
        ];

        // Keywords indicating design tasks
        let design_keywords = [
            "設計", "アーキテクチャ", "構造", "概要", "どう",
            "design", "architecture", "structure", "overview", "how",
            "plan", "approach", "strategy",
        ];

        let has_coding = coding_keywords.iter().any(|k| message_lower.contains(k));
        let has_design = design_keywords.iter().any(|k| message_lower.contains(k));

        if has_coding && !has_design {
            ModelType::Coding
        } else if has_design && !has_coding {
            ModelType::Design
        } else {
            // Default to Design for general questions
            ModelType::Design
        }
    }
}

#[derive(Clone)]
pub struct LlmClient {
    base_url: String,
    client: reqwest::Client,
}

impl LlmClient {
    pub fn new() -> Result<Self> {
        // Use local Ollama server by default
        let base_url = std::env::var("OLLAMA_BASE_URL")
            .unwrap_or_else(|_| "http://KyosukenoMac-Studio.local:11434".to_string());

        Ok(Self {
            base_url,
            client: reqwest::Client::new(),
        })
    }

    /// Execute a tool call
    async fn execute_tool(&self, tool_name: &str, parameters: &serde_json::Value, project_path: Option<&str>) -> Result<String> {
        tracing::info!("🔧 Executing tool: {} with params: {}", tool_name, parameters);

        match tool_name {
            "read_file" => {
                let path = parameters.get("path")
                    .and_then(|v| v.as_str())
                    .context("Missing 'path' parameter")?;

                let full_path = if let Some(proj_path) = project_path {
                    std::path::Path::new(proj_path).join(path)
                } else {
                    std::path::PathBuf::from(path)
                };

                match tokio::fs::read_to_string(&full_path).await {
                    Ok(content) => {
                        tracing::info!("✅ Read file: {} ({} bytes)", full_path.display(), content.len());
                        Ok(content)
                    }
                    Err(e) => {
                        let error = format!("Failed to read file {}: {}", full_path.display(), e);
                        tracing::error!("❌ {}", error);
                        Ok(error)
                    }
                }
            }
            "write_file" => {
                let path = parameters.get("path")
                    .and_then(|v| v.as_str())
                    .context("Missing 'path' parameter")?;
                let content = parameters.get("content")
                    .and_then(|v| v.as_str())
                    .context("Missing 'content' parameter")?;

                let full_path = if let Some(proj_path) = project_path {
                    std::path::Path::new(proj_path).join(path)
                } else {
                    std::path::PathBuf::from(path)
                };

                match tokio::fs::write(&full_path, content).await {
                    Ok(_) => {
                        tracing::info!("✅ Wrote file: {}", full_path.display());
                        Ok(format!("Successfully wrote {} bytes to {}", content.len(), full_path.display()))
                    }
                    Err(e) => {
                        let error = format!("Failed to write file {}: {}", full_path.display(), e);
                        tracing::error!("❌ {}", error);
                        Ok(error)
                    }
                }
            }
            "execute_command" => {
                let command = parameters.get("command")
                    .and_then(|v| v.as_str())
                    .context("Missing 'command' parameter")?;

                tracing::info!("🔧 Executing command: {}", command);

                let output = tokio::process::Command::new("sh")
                    .arg("-c")
                    .arg(command)
                    .current_dir(project_path.unwrap_or("."))
                    .output()
                    .await
                    .context("Failed to execute command")?;

                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                let result = if output.status.success() {
                    format!("✅ Command succeeded:\n{}", stdout)
                } else {
                    format!("❌ Command failed (exit code: {}):\nstdout: {}\nstderr: {}",
                        output.status.code().unwrap_or(-1), stdout, stderr)
                };

                tracing::info!("Command output: {}", result);
                Ok(result)
            }
            "search_code" => {
                let query = parameters.get("query")
                    .and_then(|v| v.as_str())
                    .context("Missing 'query' parameter")?;

                tracing::info!("🔍 Searching code for: {}", query);

                let search_dir = project_path.unwrap_or(".");
                let output = tokio::process::Command::new("rg")
                    .arg(query)
                    .arg("--max-count=10")
                    .arg("--heading")
                    .arg("--line-number")
                    .current_dir(search_dir)
                    .output()
                    .await
                    .context("Failed to search code")?;

                let result = String::from_utf8_lossy(&output.stdout);
                tracing::info!("Search results: {} bytes", result.len());
                Ok(result.to_string())
            }
            _ => {
                let error = format!("Unknown tool: {}", tool_name);
                tracing::error!("❌ {}", error);
                Ok(error)
            }
        }
    }

    /// Get available tools for autonomous mode
    fn get_autonomous_tools() -> Vec<OllamaTool> {
        vec![
            OllamaTool {
                tool_type: "function".to_string(),
                function: OllamaFunction {
                    name: "execute_command".to_string(),
                    description: "Execute a shell command and return the output".to_string(),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "command": {
                                "type": "string",
                                "description": "The shell command to execute"
                            }
                        },
                        "required": ["command"]
                    }),
                },
            },
            OllamaTool {
                tool_type: "function".to_string(),
                function: OllamaFunction {
                    name: "read_file".to_string(),
                    description: "Read the contents of a file".to_string(),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "The file path to read"
                            }
                        },
                        "required": ["path"]
                    }),
                },
            },
            OllamaTool {
                tool_type: "function".to_string(),
                function: OllamaFunction {
                    name: "write_file".to_string(),
                    description: "Write content to a file".to_string(),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "The file path to write"
                            },
                            "content": {
                                "type": "string",
                                "description": "The content to write"
                            }
                        },
                        "required": ["path", "content"]
                    }),
                },
            },
            OllamaTool {
                tool_type: "function".to_string(),
                function: OllamaFunction {
                    name: "search_code".to_string(),
                    description: "Search for code in the project".to_string(),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "query": {
                                "type": "string",
                                "description": "The search query (regex supported)"
                            }
                        },
                        "required": ["query"]
                    }),
                },
            },
        ]
    }

    /// Send a chat message to Ollama and get streaming response
    /// In autonomous mode, handles tool calling loop
    pub async fn chat_stream(
        &self,
        message: String,
        model_type: ModelType,
        autonomous: bool,
        project_path: Option<String>,
    ) -> Result<std::pin::Pin<Box<dyn futures::Stream<Item = Result<String>> + Send>>> {
        let model = model_type.model_name().to_string();
        let base_url = self.base_url.clone();
        let client = self.client.clone();

        tracing::info!("🤖 Using model: {} (type: {:?}, autonomous: {}, project_path: {:?})",
            model, model_type, autonomous, project_path);

        let stream = async_stream::stream! {
            if autonomous {
                // Autonomous mode: handle tool calling loop
                let tools = Self::get_autonomous_tools();

                // Add system prompt to guide autonomous behavior
                let mut messages = vec![
                    OllamaMessage {
                        role: "system".to_string(),
                        content: "You are an autonomous AI assistant with access to tools. YOU MUST USE TOOLS to complete tasks.\n\
\n\
CRITICAL RULES:\n\
1. When asked to READ a file: You MUST call read_file tool immediately\n\
2. When asked to WRITE/CREATE a file: You MUST call write_file tool immediately\n\
3. When asked to RUN a command: You MUST call execute_command tool immediately\n\
4. When asked to SEARCH code: You MUST call search_code tool immediately\n\
5. NEVER just explain what you would do - ACTUALLY DO IT using the tools\n\
6. For multi-step tasks, use tools multiple times in sequence\n\
7. Always verify your work by reading files you created or running commands\n\
\n\
AVAILABLE TOOLS:\n\
- read_file: Read file contents (use for ANY file reading task)\n\
- write_file: Write/create files (use for ANY file writing/creation task)\n\
- execute_command: Run shell commands (use for ls, cat, compilation, etc)\n\
- search_code: Search in codebase\n\
\n\
EXAMPLE:\n\
User: \"Read buggy.rs and fix the bug\"\n\
You MUST: 1) Call read_file(\"buggy.rs\"), 2) Analyze, 3) Call write_file with fixed code\n\
You MUST NOT: Just say \"I'll read the file\" without calling the tool".to_string(),
                    },
                    OllamaMessage {
                        role: "user".to_string(),
                        content: message.clone(),
                    }
                ];

                let max_iterations = 10;
                for iteration in 0..max_iterations {
                    tracing::info!("🔄 Autonomous iteration {}/{}", iteration + 1, max_iterations);
                    tracing::debug!("📝 Current messages count: {}", messages.len());
                    for (i, msg) in messages.iter().enumerate() {
                        tracing::debug!("  Message {}: role={}, content_len={}", i, msg.role, msg.content.len());
                    }

                    let request = OllamaChatRequest {
                        model: model.clone(),
                        messages: messages.clone(),
                        stream: false,
                        tools: Some(tools.clone()),
                    };

                    let url = format!("{}/api/chat", base_url);
                    let response = match client
                        .post(&url)
                        .json(&request)
                        .send()
                        .await
                    {
                        Ok(resp) => resp,
                        Err(e) => {
                            yield Err(anyhow::anyhow!("Failed to send request: {}", e));
                            break;
                        }
                    };

                    if !response.status().is_success() {
                        let status = response.status();
                        let body = response.text().await.unwrap_or_default();
                        yield Err(anyhow::anyhow!("Ollama API error {}: {}", status, body));
                        break;
                    }

                    let chat_response: OllamaChatResponse = match response.json().await {
                        Ok(resp) => resp,
                        Err(e) => {
                            yield Err(anyhow::anyhow!("Failed to parse response: {}", e));
                            break;
                        }
                    };

                    let assistant_message = chat_response.message;
                    tracing::info!("🤖 Assistant response ({} bytes): {}",
                        assistant_message.content.len(),
                        if assistant_message.content.len() > 100 {
                            format!("{}...", &assistant_message.content[..100])
                        } else {
                            assistant_message.content.clone()
                        }
                    );

                    // Check if the response contains a tool call (JSON or XML format)
                    if let Some((tool_name, tool_params)) = parse_tool_call(&assistant_message.content) {
                        // Tool call detected!
                        tracing::info!("🔧 Tool call detected: {} with params: {:?}", tool_name, tool_params);

                        // Yield a status message
                        yield Ok(format!("\n**[🔧 実行中: {}]**\n", tool_name));

                        // Execute the tool
                        let tool_result = match Self::execute_tool(
                            &Self { base_url: base_url.clone(), client: client.clone() },
                            &tool_name,
                            &tool_params,
                            project_path.as_deref(),
                        ).await {
                            Ok(result) => result,
                            Err(e) => format!("Error executing tool: {}", e),
                        };

                        tracing::info!("✅ Tool result: {} bytes", tool_result.len());

                        // Yield the tool result for user visibility
                        // For large results, show summary instead of full content
                        let display_result = if tool_result.len() > 2000 {
                            let lines: Vec<&str> = tool_result.lines().collect();
                            let total_lines = lines.len();
                            let preview_lines = 10;

                            if total_lines > preview_lines * 2 {
                                let first_part = lines[..preview_lines].join("\n");
                                let last_part = lines[total_lines - preview_lines..].join("\n");
                                format!("{}\n\n... ({} lines omitted) ...\n\n{}\n\n[Total: {} lines, {} bytes]",
                                    first_part, total_lines - preview_lines * 2, last_part, total_lines, tool_result.len())
                            } else {
                                format!("{}\n\n[Total: {} lines, {} bytes]", tool_result, total_lines, tool_result.len())
                            }
                        } else {
                            tool_result.clone()
                        };
                        yield Ok(format!("```\n{}\n```\n", display_result));

                        // Add both the assistant's tool call and the tool result to messages
                        messages.push(assistant_message);
                        let tool_message = OllamaMessage {
                            role: "tool".to_string(),
                            content: tool_result.clone(),
                        };
                        messages.push(tool_message);

                        tracing::info!("📨 Added tool result to conversation ({} bytes)", tool_result.len());

                        // Continue to next iteration
                        continue;
                    } else {
                        // No tool call - this is the final answer
                        tracing::info!("✅ Final answer received");
                        yield Ok(assistant_message.content);
                        break;
                    }
                }
            } else {
                // Non-autonomous mode: simple chat without tools
                let request = OllamaChatRequest {
                    model: model.clone(),
                    messages: vec![OllamaMessage {
                        role: "user".to_string(),
                        content: message,
                    }],
                    stream: false,
                    tools: None,
                };

                let url = format!("{}/api/chat", base_url);
                let response = match client.post(&url).json(&request).send().await {
                    Ok(resp) => resp,
                    Err(e) => {
                        yield Err(anyhow::anyhow!("Failed to send request: {}", e));
                        return;
                    }
                };

                if !response.status().is_success() {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    yield Err(anyhow::anyhow!("Ollama API error {}: {}", status, body));
                    return;
                }

                let chat_response: OllamaChatResponse = match response.json().await {
                    Ok(resp) => resp,
                    Err(e) => {
                        yield Err(anyhow::anyhow!("Failed to parse response: {}", e));
                        return;
                    }
                };

                yield Ok(chat_response.message.content);
            }
        };

        Ok(Box::pin(stream))
    }

}
