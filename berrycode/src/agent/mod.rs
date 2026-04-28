//! BerryCode coding-agent abstraction.
//!
//! `v0.4.5` / Phase 4 of the AI roadmap delegates "Agent mode" /
//! "Apply diff" / autonomous file-editing features to external CLI
//! tools rather than reimplementing the agent loop in Rust:
//!
//! - **Claude Code** (`claude` binary, Anthropic) — implemented in
//!   [`claude`].
//! - **Codex CLI** (`codex` binary, OpenAI) — stub in [`codex`]; the
//!   spawn details are wired up but `check_installed` returns `None`
//!   on machines that don't ship the binary, so the UI can show a
//!   helpful "install codex via …" hint instead of failing.
//!
//! Both impls share the [`CodingAgent`] trait so the rest of the IDE
//! is provider-neutral. Higher layers pick which agent to drive based
//! on the user's selection in `Settings → AI Providers`.
//!
//! ## Lifecycle
//!
//! 1. Caller picks a [`CodingAgent`] impl and asks for a session via
//!    [`CodingAgent::run`].
//! 2. The impl spawns the CLI as a child process, plumbs stdout
//!    through a parser, and emits [`AgentEvent`]s into the returned
//!    [`AgentSession::events`] receiver.
//! 3. The caller drives the UI off those events. Dropping the session
//!    kills the child — see [`AgentSession`]'s `Drop`.
//!
//! `run` is intentionally async + non-blocking: parsing happens in a
//! tokio task spawned by the impl so the UI thread isn't pinned.

pub mod claude;
pub mod codex;

use std::path::{Path, PathBuf};
use tokio::process::Child;
use tokio::sync::mpsc;

/// Identifies one of the supported coding-agent backends. Used in
/// settings persistence and surfaced in the agent picker dropdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AgentId {
    /// Anthropic's Claude Code CLI (`claude` binary).
    ClaudeCode,
    /// OpenAI's Codex CLI (`codex` binary).
    Codex,
}

impl AgentId {
    pub fn label(&self) -> &'static str {
        match self {
            Self::ClaudeCode => "Claude Code",
            Self::Codex => "Codex",
        }
    }
}

/// Optional knobs passed to a single agent run. Anything here that the
/// concrete CLI doesn't support is silently ignored — keep this trait
/// surface stable across implementations.
#[derive(Debug, Clone, Default)]
pub struct AgentRunOpts {
    /// Concrete model id, e.g. `"claude-sonnet-4-6"` or `"gpt-5-codex"`.
    /// `None` lets the CLI pick its default.
    pub model: Option<String>,
    /// Soft USD cap. Forwarded to `claude --max-budget-usd` when set;
    /// Codex gets no native cap (we surface it in the cost panel only).
    pub max_budget_usd: Option<f32>,
    /// Extra directories to grant the agent read/write access to. The
    /// project root (the `cwd` argument to `run`) is always included.
    pub additional_dirs: Vec<PathBuf>,
    /// System prompt appended to the agent's default. Used by the
    /// Bevy-aware preset to nudge the model toward Bevy 0.18 idioms.
    pub append_system_prompt: Option<String>,
}

/// One observable event from a running agent. The set is intentionally
/// small — UIs render `Output` as streaming text, surface `Edit` in a
/// diff viewer with Approve/Reject, and show the rest as status.
#[derive(Debug, Clone)]
pub enum AgentEvent {
    /// Raw text the agent printed. The UI can route this to a console
    /// pane verbatim.
    Output(String),
    /// Higher-level "the assistant said this" message, parsed out of
    /// structured output formats. Falls back to `Output` for impls
    /// that don't expose a separate message channel.
    AssistantMessage(String),
    /// The agent invoked a tool (Bash, Edit, Read, …). Useful for
    /// surfacing what's happening at a glance.
    ToolUse {
        tool: String,
        input: serde_json::Value,
    },
    /// A file edit the agent wants to make. The UI presents this in a
    /// diff viewer; until the user approves, no on-disk change has
    /// been made (when the agent is run in a permission-gated mode).
    Edit {
        path: PathBuf,
        before: Option<String>,
        after: String,
    },
    /// The agent finished. `success` is false if the child exited
    /// non-zero or panicked while parsing.
    Done {
        success: bool,
        usage: Option<crate::ai::TokenUsage>,
    },
    /// Non-fatal warning or error message. Fatal cases close the
    /// channel after emitting `Done { success: false, .. }`.
    Error(String),
}

/// Errors thrown synchronously from [`CodingAgent::run`] (i.e. before
/// the child is even started). Errors that surface mid-run come back
/// as `AgentEvent::Error`.
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("{0} CLI not found in PATH — install it to use this agent")]
    NotInstalled(&'static str),
    #[error("API key not configured for {0}")]
    MissingApiKey(&'static str),
    #[error("failed to spawn {bin}: {source}")]
    Spawn {
        bin: &'static str,
        #[source]
        source: std::io::Error,
    },
}

/// Active agent session. Holds the receiver end of the event channel
/// and the child process; dropping the session kills the child so a
/// closed UI panel can't leave a runaway agent in the background.
pub struct AgentSession {
    pub events: mpsc::UnboundedReceiver<AgentEvent>,
    /// The spawned CLI process. Boxed so the impl can replace it
    /// without changing the public type.
    child: Option<Child>,
}

impl AgentSession {
    pub fn new(events: mpsc::UnboundedReceiver<AgentEvent>, child: Child) -> Self {
        Self {
            events,
            child: Some(child),
        }
    }

    /// Try to terminate the child gracefully. Idempotent.
    pub fn cancel(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.start_kill();
        }
    }
}

impl Drop for AgentSession {
    fn drop(&mut self) {
        // Best-effort kill; a dropped session must not leak the child.
        self.cancel();
    }
}

/// Common trait every agent backend implements.
#[async_trait::async_trait]
pub trait CodingAgent: Send + Sync {
    fn id(&self) -> AgentId;

    /// Filename of the CLI binary as expected on `PATH`. The default
    /// `check_installed` runs `<binary> --version` to detect presence;
    /// impls can override to use a different probe.
    fn binary_name(&self) -> &'static str;

    /// Best-effort version detection. Returns the trimmed first line
    /// of `<binary> --version` stdout, or `None` if the binary isn't
    /// installed or fails to launch. Implementations should not block
    /// for more than a couple of seconds.
    fn check_installed(&self) -> Option<String> {
        // Synchronous probe so the Settings UI can call it from a
        // render closure without an async runtime.
        let output = std::process::Command::new(self.binary_name())
            .arg("--version")
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let line = String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .to_string();
        if line.is_empty() {
            None
        } else {
            Some(line)
        }
    }

    /// Spawn the agent on `prompt`, with `cwd` as the project root the
    /// agent has full read/write access to. Returns immediately; the
    /// session streams events as they arrive.
    async fn run(
        &self,
        prompt: &str,
        cwd: &Path,
        opts: AgentRunOpts,
    ) -> Result<AgentSession, AgentError>;
}
