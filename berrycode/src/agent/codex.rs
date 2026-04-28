//! Codex CLI (`codex` binary) backend for [`CodingAgent`].
//!
//! Stub implementation: the spawn argv is sketched out and ready to
//! parse, but the developer machine BerryCode v0.4.5 was authored on
//! didn't have `codex` installed, so this file deliberately falls
//! back to [`AgentError::NotInstalled`] from `run` until we can
//! verify the real CLI surface end-to-end.
//!
//! Once `codex` is available, the parsing path mirrors
//! [`super::claude`] — both emit JSON-Lines style streams that map
//! cleanly onto [`super::AgentEvent`].

use std::path::Path;

use super::{AgentError, AgentId, AgentRunOpts, AgentSession, CodingAgent};

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
        _prompt: &str,
        _cwd: &Path,
        _opts: AgentRunOpts,
    ) -> Result<AgentSession, AgentError> {
        // The default `check_installed` from the trait already returns
        // `None` when the binary isn't on PATH; the UI uses that to
        // disable the Codex option in the agent picker. Returning
        // `NotInstalled` here too means a caller that bypasses the
        // picker (programmatic usage / tests) gets the same friendly
        // signal instead of a generic spawn failure.
        Err(AgentError::NotInstalled(BINARY))
    }
}
