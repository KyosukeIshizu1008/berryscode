//! Claude Code (`claude` binary) backend for [`CodingAgent`].
//!
//! Spawns the official Anthropic CLI with `--print --output-format=
//! stream-json` so we get a JSON Lines event stream instead of having
//! to scrape free-form text. Each line is parsed into one or more
//! [`AgentEvent`]s; lines we don't recognise pass through as raw
//! [`AgentEvent::Output`] so nothing gets silently dropped.
//!
//! ## Streaming format
//!
//! Claude Code's `stream-json` emits one JSON object per line. The
//! shapes we care about (parser is forgiving on unknown variants):
//!
//! ```jsonc
//! { "type": "assistant",
//!   "message": { "content": [
//!       { "type": "text",     "text": "..." },
//!       { "type": "tool_use", "name": "Edit", "input": { ... } } ] } }
//! { "type": "result", "stop_reason": "end_turn",
//!   "usage": { "input_tokens": …, "output_tokens": … } }
//! ```
//!
//! Edit-tool invocations get hoisted to a structured
//! [`AgentEvent::Edit`] so the IDE can offer Approve / Reject in its
//! diff viewer instead of merely echoing the JSON blob.

use std::path::Path;
use std::process::Stdio;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

use super::{AgentError, AgentEvent, AgentId, AgentRunOpts, AgentSession, CodingAgent};
use crate::ai::TokenUsage;

const BINARY: &str = "claude";

pub struct ClaudeCodeAgent;

impl ClaudeCodeAgent {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ClaudeCodeAgent {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl CodingAgent for ClaudeCodeAgent {
    fn id(&self) -> AgentId {
        AgentId::ClaudeCode
    }

    fn binary_name(&self) -> &'static str {
        BINARY
    }

    async fn run(
        &self,
        prompt: &str,
        cwd: &Path,
        opts: AgentRunOpts,
    ) -> Result<AgentSession, AgentError> {
        // Build the argv. `--print` makes claude exit after the run
        // (vs. dropping into REPL); `--output-format=stream-json` is
        // the structured parser channel; `--include-partial-messages`
        // surfaces partial chunks so the UI feels live; `--add-dir`
        // grants the agent read/write on the project root (and any
        // extras the caller asked for).
        let mut cmd = Command::new(BINARY);
        cmd.current_dir(cwd)
            .arg("--print")
            .arg("--output-format=stream-json")
            .arg("--include-partial-messages")
            .arg("--add-dir")
            .arg(cwd);

        for extra in &opts.additional_dirs {
            cmd.arg("--add-dir").arg(extra);
        }

        if let Some(model) = opts.model.as_ref() {
            cmd.arg("--model").arg(model);
        }
        if let Some(cap) = opts.max_budget_usd {
            cmd.arg("--max-budget-usd").arg(format!("{}", cap));
        }
        if let Some(sys) = opts.append_system_prompt.as_ref() {
            cmd.arg("--append-system-prompt").arg(sys);
        }

        // `--input-format=text` is the default; the prompt goes as
        // the trailing positional argument.
        cmd.arg(prompt);

        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|source| {
            // ENOENT on macOS / Linux; "program not found" wording on
            // Windows. Map both to a friendly NotInstalled so the UI
            // can show an install hint instead of a raw OS error.
            if source.kind() == std::io::ErrorKind::NotFound {
                AgentError::NotInstalled(BINARY)
            } else {
                AgentError::Spawn {
                    bin: BINARY,
                    source,
                }
            }
        })?;

        let stdout = child.stdout.take().expect("piped stdout configured above");
        let stderr = child.stderr.take().expect("piped stderr configured above");

        let (tx, rx) = mpsc::unbounded_channel();

        // stderr → AgentEvent::Error. Claude Code occasionally writes
        // structured warnings here (rate-limit hints, etc.). Run on
        // its own task so it doesn't block stdout draining.
        {
            let tx = tx.clone();
            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let _ = tx.send(AgentEvent::Error(line));
                }
            });
        }

        // stdout → parse stream-json events.
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            let mut final_usage: Option<TokenUsage> = None;
            let mut success = true;

            while let Ok(Some(line)) = lines.next_line().await {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                match serde_json::from_str::<serde_json::Value>(trimmed) {
                    Ok(v) => {
                        let kind = v.get("type").and_then(|t| t.as_str()).unwrap_or("");
                        match kind {
                            "assistant" | "stream_event" => {
                                emit_assistant_content(&v, &tx);
                            }
                            "result" => {
                                if let Some(usage) = v.get("usage") {
                                    final_usage = Some(parse_usage(usage));
                                }
                                if v.get("is_error").and_then(|b| b.as_bool()).unwrap_or(false) {
                                    success = false;
                                }
                            }
                            "error" => {
                                success = false;
                                if let Some(msg) = v.get("message").and_then(|m| m.as_str()) {
                                    let _ = tx.send(AgentEvent::Error(msg.to_string()));
                                }
                            }
                            _ => {
                                // Unrecognised type — surface raw so we
                                // never silently drop information.
                                let _ = tx.send(AgentEvent::Output(line));
                            }
                        }
                    }
                    Err(_) => {
                        // Not JSON. Probably setup chatter from the
                        // CLI before stream-json mode kicked in.
                        let _ = tx.send(AgentEvent::Output(line));
                    }
                }
            }

            // Wait for the child so we get its exit status reflected
            // in `success`. The borrow ends with the loop above so it
            // is safe to wait now.
            let exit_ok = child.wait().await.map(|s| s.success()).unwrap_or(false);

            let _ = tx.send(AgentEvent::Done {
                success: success && exit_ok,
                usage: final_usage,
            });
        });

        // We've already moved `child` into the parser task to await
        // its exit; rebuild a placeholder so `AgentSession` can carry
        // a kill handle. Use the no-op pattern below: cancellation
        // closes the stdout pipe by dropping the parser task on the
        // session drop, which makes the child exit on the next write.
        // (A cleaner future fix is to `Arc<Mutex<Option<Child>>>` it.)
        Ok(AgentSession {
            events: rx,
            child: None,
        })
    }
}

/// Walk an `assistant` / `stream_event` payload and emit one event per
/// content block. Falls back to `Output` for unfamiliar block types.
fn emit_assistant_content(v: &serde_json::Value, tx: &mpsc::UnboundedSender<AgentEvent>) {
    let blocks = v
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_array())
        .or_else(|| v.get("content").and_then(|c| c.as_array()));
    let Some(blocks) = blocks else {
        return;
    };
    for block in blocks {
        let kind = block.get("type").and_then(|t| t.as_str()).unwrap_or("");
        match kind {
            "text" => {
                if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                    let _ = tx.send(AgentEvent::AssistantMessage(text.to_string()));
                }
            }
            "tool_use" => {
                let tool = block
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("(unknown)")
                    .to_string();
                let input = block
                    .get("input")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null);
                // Surface Edit / MultiEdit specially so the IDE can
                // open the diff viewer instead of dumping raw JSON.
                if matches!(tool.as_str(), "Edit" | "MultiEdit" | "Write") {
                    if let (Some(path), Some(after)) = (
                        input.get("file_path").and_then(|p| p.as_str()),
                        input
                            .get("new_string")
                            .or_else(|| input.get("content"))
                            .and_then(|s| s.as_str()),
                    ) {
                        let before = input
                            .get("old_string")
                            .and_then(|s| s.as_str())
                            .map(|s| s.to_string());
                        let _ = tx.send(AgentEvent::Edit {
                            path: std::path::PathBuf::from(path),
                            before,
                            after: after.to_string(),
                        });
                        continue;
                    }
                }
                let _ = tx.send(AgentEvent::ToolUse { tool, input });
            }
            _ => {
                // Other block kinds (thinking, image, …) — skip for
                // now; can extend in later phases.
            }
        }
    }
}

fn parse_usage(u: &serde_json::Value) -> TokenUsage {
    TokenUsage {
        prompt_tokens: u.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        completion_tokens: u.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        cache_read_tokens: u
            .get("cache_read_input_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32,
        cache_write_tokens: u
            .get("cache_creation_input_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32,
    }
}
