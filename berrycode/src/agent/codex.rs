//! Codex CLI (`codex` binary) backend for [`CodingAgent`].
//!
//! Uses `codex exec --json` for non-interactive runs with JSON Lines
//! event streaming. The CLI surface, sourced from
//! `openai/codex` (`codex-rs/utils/cli/src/shared_options.rs`):
//!
//! - `--cd <PATH>` — working directory
//! - `--add-dir <PATH>` — extra writable directories
//! - `--model <NAME>` — model id (e.g. `gpt-5-codex`)
//! - `--full-auto` — auto-approve sandboxed command execution
//! - `--skip-git-repo-check` — allow running outside a git repo
//! - `--ephemeral` — don't persist this session
//! - `--json` — JSONL event stream on stdout (the parser channel)
//!
//! The stream-json event shapes haven't been characterised on a real
//! installation yet, so the parser is intentionally forgiving:
//! recognised types map to structured [`AgentEvent`]s, everything
//! else passes through as raw [`AgentEvent::Output`] so we never
//! silently drop information. Refining the parser to extract Edit /
//! ToolUse semantics is a follow-up once we have a `codex` binary
//! to validate against.

use std::path::Path;
use std::process::Stdio;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

use super::{AgentError, AgentEvent, AgentId, AgentRunOpts, AgentSession, CodingAgent};
use crate::ai::TokenUsage;

const BINARY: &str = "codex";

pub struct CodexAgent;

impl CodexAgent {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CodexAgent {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl CodingAgent for CodexAgent {
    fn id(&self) -> AgentId {
        AgentId::Codex
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
        // `codex exec` is the non-interactive subcommand; everything
        // else here just shapes a single one-shot run that the IDE
        // can stream back to the user.
        let mut cmd = Command::new(self.binary_path());
        cmd.current_dir(cwd)
            .arg("exec")
            .arg("--json")
            .arg("--cd")
            .arg(cwd)
            // codex 0.128+ deprecated `--full-auto`; the equivalent
            // policy ("write inside the workspace, no approval prompts
            // for in-tree edits") is now `--sandbox workspace-write`.
            .arg("--sandbox")
            .arg("workspace-write")
            .arg("--skip-git-repo-check")
            .arg("--ephemeral");

        for extra in &opts.additional_dirs {
            cmd.arg("--add-dir").arg(extra);
        }
        if let Some(model) = opts.model.as_ref() {
            cmd.arg("--model").arg(model);
        }
        // Codex doesn't expose `max_budget_usd` or
        // `append_system_prompt` flags today — those parts of
        // `AgentRunOpts` are silently ignored, matching the trait
        // contract that says unsupported fields are ok to skip.

        cmd.arg(prompt);

        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|source| {
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

        // stderr → AgentEvent::Error on its own task so it doesn't
        // back-pressure stdout draining.
        {
            let tx = tx.clone();
            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let _ = tx.send(AgentEvent::Error(line));
                }
            });
        }

        // stdout → parse JSONL events.
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
                    Ok(v) => parse_event(v, &line, &tx, &mut final_usage, &mut success),
                    Err(_) => {
                        // Non-JSON line — pre-stream chatter or a
                        // human-readable banner. Forward verbatim.
                        let _ = tx.send(AgentEvent::Output(line));
                    }
                }
            }

            let exit_ok = child.wait().await.map(|s| s.success()).unwrap_or(false);

            let _ = tx.send(AgentEvent::Done {
                success: success && exit_ok,
                usage: final_usage,
            });
        });

        // The real child handle has been moved into the parser task
        // above; we leave the session's `child` slot empty (`None`),
        // matching `claude.rs`. Cancellation works by dropping the
        // receiver, which closes the pipe and lets the child exit on
        // its next stdout write.
        Ok(AgentSession {
            events: rx,
            child: None,
        })
    }
}

/// Best-effort dispatch for one Codex JSONL event.
///
/// The parser is deliberately forgiving — Codex's exact event shapes
/// haven't been characterised against a live binary yet, so anything
/// unrecognised drops down to `AgentEvent::Output` to keep
/// information visible without us having to commit to a schema we
/// can't verify.
fn parse_event(
    v: serde_json::Value,
    raw: &str,
    tx: &mpsc::UnboundedSender<AgentEvent>,
    final_usage: &mut Option<TokenUsage>,
    success: &mut bool,
) {
    let kind = v
        .get("type")
        .or_else(|| v.get("kind"))
        .or_else(|| v.get("event"))
        .and_then(|t| t.as_str())
        .unwrap_or("");

    // Plain assistant text. Codex tends to surface these under
    // `text` or `delta.text` depending on streaming vs final.
    if let Some(text) = v
        .get("text")
        .and_then(|t| t.as_str())
        .or_else(|| {
            v.get("delta")
                .and_then(|d| d.get("text"))
                .and_then(|t| t.as_str())
        })
        .or_else(|| v.get("content").and_then(|t| t.as_str()))
    {
        if !text.is_empty() {
            let _ = tx.send(AgentEvent::AssistantMessage(text.to_string()));
            return;
        }
    }

    // Token usage tends to appear on a terminal "result" / "complete"
    // event. We try a few common field names.
    if matches!(kind, "result" | "complete" | "done" | "finish") {
        let usage = v
            .get("usage")
            .or_else(|| v.get("token_usage"))
            .map(parse_usage);
        if usage.is_some() {
            *final_usage = usage;
        }
        if v.get("error").is_some() || v.get("is_error").and_then(|b| b.as_bool()).unwrap_or(false)
        {
            *success = false;
        }
        return;
    }

    // Tool / command invocations. Names vary across Codex versions
    // (`tool_call`, `function_call`, `command`, …) — group them so
    // the UI shows a consistent `[tool]` decoration.
    if matches!(kind, "tool_call" | "tool_use" | "function_call" | "command") {
        let tool = v
            .get("name")
            .or_else(|| v.get("tool"))
            .and_then(|n| n.as_str())
            .unwrap_or("(tool)")
            .to_string();
        let input = v
            .get("input")
            .or_else(|| v.get("arguments"))
            .or_else(|| v.get("args"))
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let _ = tx.send(AgentEvent::ToolUse { tool, input });
        return;
    }

    if matches!(kind, "error") {
        *success = false;
        let msg = v
            .get("message")
            .or_else(|| v.get("error"))
            .and_then(|m| m.as_str())
            .unwrap_or("(unspecified error)")
            .to_string();
        let _ = tx.send(AgentEvent::Error(msg));
        return;
    }

    // Unknown event — pass the raw JSON through so nothing is lost.
    let _ = tx.send(AgentEvent::Output(raw.to_string()));
}

fn parse_usage(u: &serde_json::Value) -> TokenUsage {
    TokenUsage {
        prompt_tokens: u
            .get("prompt_tokens")
            .or_else(|| u.get("input_tokens"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32,
        completion_tokens: u
            .get("completion_tokens")
            .or_else(|| u.get("output_tokens"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32,
        // Codex's prompt cache stats (if exposed) likely use OpenAI's
        // `cached_tokens` shape; fall back to 0 if not present.
        cache_read_tokens: u
            .get("cached_tokens")
            .or_else(|| u.get("cache_read_input_tokens"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32,
        cache_write_tokens: 0,
    }
}
