//! Ollama (local) backend for [`CodingAgent`].
//!
//! Runs an in-process Chat-Completions agent loop against a local Ollama
//! server. Unlike [`super::native`] (Responses API, OpenAI only) this
//! talks to Ollama's OpenAI-compatible `/v1/chat/completions` endpoint
//! so it works with any tool-capable Ollama model (`llama3.1`,
//! `qwen2.5-coder`, `mistral`, …).
//!
//! ## Why a separate agent
//!
//! Ollama supports the Chat Completions API but not the Responses API
//! that [`super::native`]'s loop is built around. The two protocols
//! differ in request shape (`messages` vs `input`), tool-call envelope
//! (`tool_calls` vs `function_call`), and streaming event taxonomy, so
//! sharing a loop would mean a runtime branch on every interaction.
//! Keeping them as siblings under the same [`CodingAgent`] trait is
//! cleaner and lets us evolve them independently.
//!
//! ## Streaming
//!
//! Each turn opens an SSE stream against `/v1/chat/completions` with
//! `stream: true`. Content deltas surface immediately as
//! [`AgentEvent::AssistantMessage`] events so the chat panel feels
//! responsive even on cold local models. `tool_calls` arrive
//! fragmented across chunks (id + name in one delta, JSON arguments
//! piecewise across many) and are reassembled by `index` before
//! dispatch.
//!
//! ## Tools
//!
//! Same set as [`super::native`] (`read_file`, `write_file`,
//! `list_files`, `run_bash`); dispatch lives in [`super::tools`].

use std::path::Path;
use std::sync::Arc;

use futures_util::StreamExt;
use serde_json::{json, Value};
use tokio::sync::mpsc;

use super::tools;
use super::{AgentError, AgentEvent, AgentId, AgentRunOpts, AgentSession, CodingAgent};
use crate::ai::{settings::AiSettings, TokenUsage};

const MAX_TURNS: usize = 30;

pub struct OllamaAgent {
    settings: Arc<AiSettings>,
}

impl OllamaAgent {
    pub fn new(settings: AiSettings) -> Self {
        Self {
            settings: Arc::new(settings),
        }
    }
}

#[async_trait::async_trait]
impl CodingAgent for OllamaAgent {
    fn id(&self) -> AgentId {
        AgentId::Ollama
    }

    fn binary_name(&self) -> &'static str {
        "ollama"
    }

    fn check_installed(&self) -> Option<String> {
        // In-process — what we actually need is a running Ollama
        // server, not a binary on PATH. The Settings UI does an async
        // `/api/version` probe separately; this just confirms the agent
        // backend is compiled in.
        Some(format!(
            "ollama (endpoint: {})",
            self.settings.ollama_endpoint
        ))
    }

    async fn run(
        &self,
        prompt: &str,
        cwd: &Path,
        opts: AgentRunOpts,
    ) -> Result<AgentSession, AgentError> {
        let (tx, rx) = mpsc::unbounded_channel();
        let cwd = cwd.to_path_buf();
        let prompt = prompt.to_string();
        let model = opts
            .model
            .clone()
            .unwrap_or_else(|| self.settings.chat_model.clone());
        let extra_system = opts.append_system_prompt.clone();
        let endpoint = self.settings.ollama_endpoint.clone();

        tokio::spawn(async move {
            let result = run_loop(prompt, cwd, model, endpoint, extra_system, tx.clone()).await;
            let (success, usage, err) = match result {
                Ok(usage) => (true, usage, None),
                Err(e) => (false, None, Some(e.to_string())),
            };
            if let Some(msg) = err {
                let _ = tx.send(AgentEvent::Error(msg));
            }
            let _ = tx.send(AgentEvent::Done { success, usage });
        });

        Ok(AgentSession::new_native(rx))
    }
}

// ─── Loop ────────────────────────────────────────────────────────

async fn run_loop(
    prompt: String,
    cwd: std::path::PathBuf,
    model: String,
    endpoint: String,
    extra_system: Option<String>,
    tx: mpsc::UnboundedSender<AgentEvent>,
) -> anyhow::Result<Option<TokenUsage>> {
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(180))
        .build()?;

    let url = format!("{}/v1/chat/completions", endpoint.trim_end_matches('/'));

    let mut messages: Vec<Value> = Vec::new();
    messages.push(json!({
        "role": "system",
        "content": build_system_prompt(&cwd, extra_system.as_deref()),
    }));
    messages.push(json!({
        "role": "user",
        "content": prompt,
    }));

    let tools_schema = build_tools_schema();
    let mut total_usage: Option<TokenUsage> = None;

    for _turn in 0..MAX_TURNS {
        let body = json!({
            "model": model,
            "messages": messages,
            "tools": tools_schema,
            "tool_choice": "auto",
            "stream": true,
        });

        let TurnResult {
            content,
            tool_calls,
            usage,
        } = stream_one_turn(&http, &url, &body, &tx).await?;
        accumulate_usage(&mut total_usage, usage.as_ref());

        // Reconstruct the assistant message for the next turn's history.
        // Chat Completions requires the full prior conversation, including
        // any tool_calls the assistant emitted, so subsequent tool replies
        // can reference them by id.
        let mut assistant_msg = json!({ "role": "assistant" });
        if !content.is_empty() {
            assistant_msg["content"] = Value::String(content);
        } else {
            // Some servers reject `null` content but accept empty string.
            assistant_msg["content"] = Value::String(String::new());
        }
        if !tool_calls.is_empty() {
            let tc_array: Vec<Value> = tool_calls
                .iter()
                .map(|tc| {
                    json!({
                        "id": tc.id,
                        "type": "function",
                        "function": {
                            "name": tc.name,
                            "arguments": tc.arguments,
                        }
                    })
                })
                .collect();
            assistant_msg["tool_calls"] = Value::Array(tc_array);
        }
        messages.push(assistant_msg);

        if tool_calls.is_empty() {
            return Ok(total_usage);
        }

        for call in tool_calls {
            // Ollama emits `arguments` as a JSON string fragment stream.
            // Parse the fully assembled string here; if the model produced
            // malformed JSON we still want to dispatch with `Null` rather
            // than abort the whole turn.
            let args: Value = if call.arguments.is_empty() {
                Value::Null
            } else {
                serde_json::from_str(&call.arguments).unwrap_or(Value::Null)
            };

            let _ = tx.send(AgentEvent::ToolUse {
                tool: call.name.clone(),
                input: args.clone(),
            });

            let output_str = match dispatch_tool(&call.name, &args, &cwd, &tx).await {
                Ok(s) => s,
                Err(e) => format!("ERROR: {}", e),
            };

            messages.push(json!({
                "role": "tool",
                "tool_call_id": call.id,
                "content": output_str,
            }));
        }
    }

    anyhow::bail!("agent exceeded MAX_TURNS ({})", MAX_TURNS)
}

#[derive(Default, Debug)]
struct ToolCallAcc {
    id: String,
    name: String,
    /// Streamed JSON fragment of the function arguments — concatenated as
    /// the chunks arrive. Parsing is deferred until the turn finishes so
    /// a partial fragment never gets `serde_json::from_str`'d.
    arguments: String,
}

struct TurnResult {
    content: String,
    tool_calls: Vec<ToolCallAcc>,
    usage: Option<Value>,
}

/// Stream one turn of `/v1/chat/completions`. Emits content deltas to
/// `tx` live; assembles `tool_calls` by `index` until the stream ends.
async fn stream_one_turn(
    http: &reqwest::Client,
    url: &str,
    body: &Value,
    tx: &mpsc::UnboundedSender<AgentEvent>,
) -> anyhow::Result<TurnResult> {
    let resp = http
        .post(url)
        .header("Accept", "text/event-stream")
        .json(body)
        .send()
        .await?;
    let status = resp.status();
    if !status.is_success() {
        let err_body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Ollama {}: {}", status.as_u16(), err_body);
    }

    let mut stream = resp.bytes_stream();
    let mut buf = String::new();
    let mut content = String::new();
    // Tool-call accumulator keyed by the `index` field. A small `Vec`
    // is fine — models almost never emit more than a handful of calls
    // in one turn and indices are dense from zero.
    let mut tool_accs: Vec<ToolCallAcc> = Vec::new();
    let mut usage: Option<Value> = None;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        buf.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(idx) = buf.find("\n\n") {
            let frame = buf[..idx].to_string();
            buf.drain(..idx + 2);
            handle_chat_frame(&frame, &mut content, &mut tool_accs, &mut usage, tx);
        }
    }

    Ok(TurnResult {
        content,
        tool_calls: tool_accs,
        usage,
    })
}

fn handle_chat_frame(
    frame: &str,
    content: &mut String,
    tool_accs: &mut Vec<ToolCallAcc>,
    usage: &mut Option<Value>,
    tx: &mpsc::UnboundedSender<AgentEvent>,
) {
    let mut data = String::new();
    for line in frame.lines() {
        if let Some(rest) = line.strip_prefix("data:") {
            if !data.is_empty() {
                data.push('\n');
            }
            data.push_str(rest.trim_start());
        }
    }
    if data.is_empty() || data == "[DONE]" {
        return;
    }

    let Ok(event) = serde_json::from_str::<Value>(&data) else {
        return;
    };

    // Final usage stats arrive on a chunk with empty choices when
    // Ollama is launched with `stream_options: {include_usage: true}`,
    // which we don't set, but Ollama still attaches `usage` to the
    // last meaningful chunk on some versions. Capture if present.
    if let Some(u) = event.get("usage") {
        *usage = Some(u.clone());
    }

    let Some(choice) = event.get("choices").and_then(|c| c.get(0)) else {
        return;
    };
    let delta = choice.get("delta").cloned().unwrap_or(Value::Null);

    if let Some(text) = delta.get("content").and_then(|c| c.as_str()) {
        if !text.is_empty() {
            content.push_str(text);
            let _ = tx.send(AgentEvent::AssistantMessage(text.to_string()));
        }
    }

    if let Some(tcs) = delta.get("tool_calls").and_then(|t| t.as_array()) {
        for entry in tcs {
            let idx = entry.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            while tool_accs.len() <= idx {
                tool_accs.push(ToolCallAcc::default());
            }
            let acc = &mut tool_accs[idx];
            if let Some(id) = entry.get("id").and_then(|v| v.as_str()) {
                if !id.is_empty() {
                    acc.id = id.to_string();
                }
            }
            if let Some(func) = entry.get("function") {
                if let Some(name) = func.get("name").and_then(|v| v.as_str()) {
                    if !name.is_empty() {
                        acc.name = name.to_string();
                    }
                }
                // Arguments stream as JSON-string fragments. Some Ollama
                // versions deliver it as a parsed object when the model
                // finishes the whole call in one chunk; serialize back to
                // string in that case so downstream parsing is uniform.
                match func.get("arguments") {
                    Some(Value::String(s)) => acc.arguments.push_str(s),
                    Some(other) if !other.is_null() => {
                        if let Ok(s) = serde_json::to_string(other) {
                            acc.arguments.push_str(&s);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

fn accumulate_usage(total: &mut Option<TokenUsage>, usage: Option<&Value>) {
    let Some(u) = usage else {
        return;
    };
    let prompt = u.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    let completion = u
        .get("completion_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    let acc = total.get_or_insert(TokenUsage::default());
    acc.prompt_tokens = acc.prompt_tokens.saturating_add(prompt);
    acc.completion_tokens = acc.completion_tokens.saturating_add(completion);
    // Ollama runs locally — no API-level prompt cache to expose.
}

fn build_system_prompt(cwd: &Path, extra: Option<&str>) -> String {
    let mut s = format!(
        "You are BerryCode's built-in coding agent for Bevy / Rust game development.\n\
         Working directory: {}\n\
         \n\
         Available tools:\n\
         - read_file(path): read a file relative to the working directory.\n\
         - write_file(path, contents): create or overwrite a file. Edits are auto-applied.\n\
         - list_files(path): shallow listing; pass \".\" for the project root.\n\
         - run_bash(command): execute a shell command in the working directory.\n\
         \n\
         Prefer concise, code-first answers. When you modify code, use write_file directly\n\
         rather than narrating the change. Use run_bash for cargo check / cargo test to\n\
         verify your edits before declaring the task done.",
        cwd.display()
    );
    if let Some(extra) = extra {
        s.push_str("\n\n");
        s.push_str(extra);
    }
    s
}

fn build_tools_schema() -> Vec<Value> {
    // Same surface as `super::native` but in the Chat-Completions
    // wrapper shape (`{"type": "function", "function": { ... }}`)
    // rather than Responses API's flat `{"type": "function", ...}`.
    vec![
        json!({
            "type": "function",
            "function": {
                "name": "read_file",
                "description": "Read the contents of a file (relative to the working directory).",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Relative path to the file." }
                    },
                    "required": ["path"],
                },
            },
        }),
        json!({
            "type": "function",
            "function": {
                "name": "write_file",
                "description": "Create a new file or overwrite an existing one. The change is auto-applied; the IDE shows a diff in the chat panel.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" },
                        "contents": { "type": "string" },
                    },
                    "required": ["path", "contents"],
                },
            },
        }),
        json!({
            "type": "function",
            "function": {
                "name": "list_files",
                "description": "List files in a directory (shallow). Skips target/, .git/, node_modules/.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Relative directory; \".\" for project root." }
                    },
                    "required": ["path"],
                },
            },
        }),
        json!({
            "type": "function",
            "function": {
                "name": "run_bash",
                "description": "Execute a shell command in the working directory. Returns stdout+stderr (truncated).",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "command": { "type": "string" }
                    },
                    "required": ["command"],
                },
            },
        }),
    ]
}

async fn dispatch_tool(
    name: &str,
    args: &Value,
    cwd: &Path,
    tx: &mpsc::UnboundedSender<AgentEvent>,
) -> anyhow::Result<String> {
    match name {
        "read_file" => {
            let path = args
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing 'path'"))?;
            tools::read_file(cwd, path)
        }
        "write_file" => {
            let path = args
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing 'path'"))?;
            let contents = args
                .get("contents")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing 'contents'"))?;
            tools::write_file(cwd, path, contents, tx)
        }
        "list_files" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
            tools::list_files(cwd, path)
        }
        "run_bash" => {
            let command = args
                .get("command")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing 'command'"))?;
            tools::run_bash(cwd, command).await
        }
        other => anyhow::bail!("unknown tool: {}", other),
    }
}
