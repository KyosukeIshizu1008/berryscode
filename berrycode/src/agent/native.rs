//! Native agent loop — Responses API + tool calling, no subprocess.
//!
//! Mirrors the slice of `codex exec` behaviour we actually use:
//!   - one-shot prompt → multi-turn loop where the model picks tools
//!   - streams events back to the chat panel exactly the same way the
//!     CLI-backed [`super::codex`] / [`super::claude`] impls do
//!
//! Provider routing reuses the chat-side settings: chat provider must
//! be `OpenAI` (which transparently includes Azure OpenAI via
//! `OPENAI_BASE_URL`). Anthropic's tool-use API is shaped differently
//! and isn't covered by this MVP — Anthropic users still get the
//! Claude Code subprocess backend.
//!
//! Tools (intentionally a small set; the model can still use `run_bash`
//! to grep / cat / cargo-check / etc.):
//!   - `read_file(path)` — relative path, capped at 200 KB
//!   - `write_file(path, contents)` — overwrites, emits an
//!     [`super::AgentEvent::Edit`] so the chat panel diff viewer
//!     can render before/after
//!   - `list_files(path)` — shallow `ls`-style listing, ignores
//!     `target/` / `.git/`
//!   - `run_bash(command)` — sandboxed only by the OS itself; MVP
//!     trusts the user's machine

use std::path::{Path, PathBuf};
use std::sync::Arc;

use futures_util::StreamExt;
use serde_json::{json, Value};
use tokio::sync::mpsc;

use super::tools;
use super::{AgentError, AgentEvent, AgentId, AgentRunOpts, AgentSession, CodingAgent};
use crate::ai::{settings::AiSettings, ProviderKind, TokenUsage};

/// Hard upper bound on the number of model→tool→model round trips a
/// single run can do. Stops accidental infinite loops if the model
/// keeps calling tools without making progress.
const MAX_TURNS: usize = 30;

pub struct NativeAgent {
    settings: Arc<AiSettings>,
}

impl NativeAgent {
    pub fn new(settings: AiSettings) -> Self {
        Self {
            settings: Arc::new(settings),
        }
    }
}

#[async_trait::async_trait]
impl CodingAgent for NativeAgent {
    fn id(&self) -> AgentId {
        AgentId::Native
    }

    fn binary_name(&self) -> &'static str {
        "native"
    }

    fn check_installed(&self) -> Option<String> {
        // In-process — always "installed" as long as the build links us.
        Some("native (in-process)".to_string())
    }

    async fn run(
        &self,
        prompt: &str,
        cwd: &Path,
        opts: AgentRunOpts,
    ) -> Result<AgentSession, AgentError> {
        if self.settings.chat_provider != ProviderKind::OpenAi {
            return Err(AgentError::MissingApiKey(
                "Native agent requires an OpenAI-compatible chat provider",
            ));
        }

        let (tx, rx) = mpsc::unbounded_channel();
        let cwd = cwd.to_path_buf();
        let prompt = prompt.to_string();
        let model = opts
            .model
            .clone()
            .unwrap_or_else(|| self.settings.chat_model.clone());
        let extra_system = opts.append_system_prompt.clone();
        let settings = Arc::clone(&self.settings);

        tokio::spawn(async move {
            let result = run_loop(prompt, cwd, model, settings, extra_system, tx.clone()).await;
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
    cwd: PathBuf,
    model: String,
    settings: Arc<AiSettings>,
    extra_system: Option<String>,
    tx: mpsc::UnboundedSender<AgentEvent>,
) -> anyhow::Result<Option<TokenUsage>> {
    let api_key = resolve_api_key(&settings)?;
    let endpoint = resolve_endpoint();
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(180))
        .build()?;

    // Accumulator for the Responses-API `input` array. We append the
    // model's `output[]` items + our own `function_call_output` items
    // each turn, then re-send the whole thing — that's how Responses
    // API maintains conversation continuity within a single turn.
    let mut input: Vec<Value> = Vec::new();
    input.push(json!({
        "type": "message",
        "role": "user",
        "content": prompt,
    }));

    let instructions = build_instructions(&cwd, extra_system.as_deref());
    let tools_schema = build_tools_schema();

    let mut total_usage: Option<TokenUsage> = None;

    for _turn in 0..MAX_TURNS {
        let body = json!({
            "model": model,
            "instructions": instructions,
            "input": input,
            "tools": tools_schema,
            "tool_choice": "auto",
            "stream": true,
        });

        let TurnResult {
            output_items,
            usage,
        } = stream_one_turn(&http, &endpoint, &api_key, &body, &tx).await?;
        accumulate_usage(&mut total_usage, usage.as_ref());

        let mut produced_function_call = false;

        for item in &output_items {
            // Append the raw item back into `input` so the next turn
            // sees the full conversation state.
            input.push(item.clone());

            let item_type = item.get("type").and_then(|t| t.as_str()).unwrap_or("");
            if item_type == "function_call" {
                produced_function_call = true;
                let call_id = item
                    .get("call_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let name = item
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let args_str = item
                    .get("arguments")
                    .and_then(|v| v.as_str())
                    .unwrap_or("{}");
                let args: Value = serde_json::from_str(args_str).unwrap_or(json!({}));

                let _ = tx.send(AgentEvent::ToolUse {
                    tool: name.clone(),
                    input: args.clone(),
                });

                let output_str = match dispatch_tool(&name, &args, &cwd, &tx).await {
                    Ok(s) => s,
                    Err(e) => format!("ERROR: {}", e),
                };

                input.push(json!({
                    "type": "function_call_output",
                    "call_id": call_id,
                    "output": output_str,
                }));
            }
            // `message` items already had their text streamed live
            // via Output deltas; `reasoning` is opaque to us.
        }

        if !produced_function_call {
            return Ok(total_usage);
        }
    }

    anyhow::bail!("agent exceeded MAX_TURNS ({})", MAX_TURNS)
}

struct TurnResult {
    /// Final output items from `response.completed`. Used to drive the
    /// next turn's input — `message` items echo back what the user
    /// already saw streamed, `function_call` items carry the tool args
    /// we need to dispatch.
    output_items: Vec<Value>,
    usage: Option<Value>,
}

/// Drive one model turn over a streaming SSE response, emitting text
/// deltas as they arrive and returning the full output structure once
/// the stream terminates with `response.completed`.
async fn stream_one_turn(
    http: &reqwest::Client,
    endpoint: &str,
    api_key: &str,
    body: &Value,
    tx: &mpsc::UnboundedSender<AgentEvent>,
) -> anyhow::Result<TurnResult> {
    let resp = http
        .post(endpoint)
        .bearer_auth(api_key)
        .header("Accept", "text/event-stream")
        .json(body)
        .send()
        .await?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Responses API {}: {}", status.as_u16(), body);
    }

    let mut stream = resp.bytes_stream();
    let mut buf = String::new();
    let mut output_items: Vec<Value> = Vec::new();
    let mut usage: Option<Value> = None;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        buf.push_str(&String::from_utf8_lossy(&chunk));

        // SSE frames are delimited by a blank line. Pull complete
        // frames out of the buffer; the partial tail (if any) stays
        // for the next iteration.
        while let Some(idx) = buf.find("\n\n") {
            let frame = buf[..idx].to_string();
            buf.drain(..idx + 2);
            handle_sse_frame(&frame, &mut output_items, &mut usage, tx);
        }
    }

    Ok(TurnResult {
        output_items,
        usage,
    })
}

fn handle_sse_frame(
    frame: &str,
    output_items: &mut Vec<Value>,
    usage: &mut Option<Value>,
    tx: &mpsc::UnboundedSender<AgentEvent>,
) {
    // SSE frame format: zero or more `field: value` lines. We only
    // care about `data:`; everything else (event:, id:, retry:) is
    // metadata Responses API doesn't actually use here.
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
    let event_type = event.get("type").and_then(|t| t.as_str()).unwrap_or("");

    match event_type {
        // Per-token text from a message-typed output item. This is
        // what makes the chat panel feel responsive — every delta
        // flushes a chunk to the UI.
        "response.output_text.delta" => {
            if let Some(delta) = event.get("delta").and_then(|d| d.as_str()) {
                let _ = tx.send(AgentEvent::AssistantMessage(delta.to_string()));
            }
        }
        // The complete output item arrives once it's fully streamed.
        // We hold onto these to feed the next turn's `input` array.
        "response.output_item.done" => {
            if let Some(item) = event.get("item") {
                output_items.push(item.clone());
            }
        }
        // Final usage stats come bundled with `response.completed`.
        "response.completed" => {
            if let Some(u) = event.pointer("/response/usage") {
                *usage = Some(u.clone());
            }
        }
        "response.failed" | "error" => {
            let msg = event
                .pointer("/response/error/message")
                .or_else(|| event.get("message"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown stream error");
            let _ = tx.send(AgentEvent::Error(msg.to_string()));
        }
        _ => {
            // Ignore the rest of the event taxonomy
            // (response.created, response.in_progress,
            // response.output_text.done, function_call_arguments.delta,
            // etc.) — `output_item.done` already gives us the
            // canonical structured form.
        }
    }
}

fn accumulate_usage(total: &mut Option<TokenUsage>, usage: Option<&Value>) {
    let Some(u) = usage else {
        return;
    };
    let prompt = u.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    let completion = u.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    let cached = u
        .get("input_tokens_details")
        .and_then(|d| d.get("cached_tokens"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;

    let acc = total.get_or_insert(TokenUsage::default());
    acc.prompt_tokens = acc.prompt_tokens.saturating_add(prompt);
    acc.completion_tokens = acc.completion_tokens.saturating_add(completion);
    acc.cache_read_tokens = acc.cache_read_tokens.saturating_add(cached);
}

fn build_instructions(cwd: &Path, extra: Option<&str>) -> String {
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

// ─── Tool dispatch ───────────────────────────────────────────────

fn build_tools_schema() -> Vec<Value> {
    vec![
        json!({
            "type": "function",
            "name": "read_file",
            "description": "Read the contents of a file (relative to the working directory). Returns the file contents or an error message.",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative path to the file." }
                },
                "required": ["path"],
            },
        }),
        json!({
            "type": "function",
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
        }),
        json!({
            "type": "function",
            "name": "list_files",
            "description": "List files in a directory (shallow). Skips target/, .git/, node_modules/.",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative directory; \".\" for project root." }
                },
                "required": ["path"],
            },
        }),
        json!({
            "type": "function",
            "name": "run_bash",
            "description": "Execute a shell command in the working directory. Returns stdout+stderr (truncated).",
            "parameters": {
                "type": "object",
                "properties": {
                    "command": { "type": "string" }
                },
                "required": ["command"],
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

// ─── Endpoint / key resolution ───────────────────────────────────

fn resolve_api_key(settings: &AiSettings) -> anyhow::Result<String> {
    // Same priority as the OpenAI provider — env wins, settings is
    // the fallback.
    if let Ok(env_key) = std::env::var("OPENAI_API_KEY") {
        if !env_key.is_empty() {
            return Ok(env_key);
        }
    }
    if !settings.openai_api_key.is_empty() {
        return Ok(settings.openai_api_key.clone());
    }
    anyhow::bail!("OpenAI API key not configured (set OPENAI_API_KEY or paste in Settings)")
}

fn resolve_endpoint() -> String {
    // Mirrors the logic in `ai::openai::resolve_endpoint`, but always
    // returns a Responses-API URL (the loop only knows that schema).
    match std::env::var("OPENAI_BASE_URL") {
        Ok(url) if url.contains("/responses") => url,
        Ok(url) => {
            let trimmed = url.trim_end_matches('/');
            format!("{}/responses", trimmed)
        }
        Err(_) => "https://api.openai.com/v1/responses".to_string(),
    }
}
