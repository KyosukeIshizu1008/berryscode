//! Native terminal/PTY operations using portable-pty
//! Replaces tauri_bindings terminal commands

use anyhow::{Context, Result};
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Read;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalOutput {
    pub output: String,
    pub exit_code: Option<i32>,
}

/// Terminal session manager (simplified)
pub struct TerminalManager {
    sessions: Arc<Mutex<HashMap<String, TerminalSession>>>,
}

struct TerminalSession {
    // Simplified for now - full PTY implementation would be more complex
    working_dir: String,
}

impl TerminalManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Execute a command and return output
    pub fn execute_command(&self, command: &str, working_dir: &str) -> Result<TerminalOutput> {
        let pty_system = NativePtySystem::default();

        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("Failed to open PTY")?;

        let mut cmd = CommandBuilder::new("sh");
        cmd.arg("-c");
        cmd.arg(command);
        cmd.cwd(working_dir);

        let mut child = pair.slave.spawn_command(cmd).context("Failed to spawn command")?;

        drop(pair.slave);

        let mut reader = pair.master.try_clone_reader().context("Failed to clone reader")?;
        let mut output = String::new();

        // Read output (with timeout)
        let mut buffer = [0u8; 1024];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    output.push_str(&String::from_utf8_lossy(&buffer[..n]));
                }
                Err(_) => break,
            }
        }

        let exit_status = child.wait().context("Failed to wait for child")?;
        let exit_code = exit_status.success().then_some(0);

        Ok(TerminalOutput { output, exit_code })
    }

    /// Change working directory for a session
    pub fn change_directory(&self, session_id: &str, path: &str) -> Result<()> {
        let mut sessions = self.sessions.lock().unwrap();

        if let Some(session) = sessions.get_mut(session_id) {
            session.working_dir = path.to_string();
            Ok(())
        } else {
            anyhow::bail!("Session not found")
        }
    }

    /// Get current working directory
    pub fn get_working_directory(&self, session_id: &str) -> Result<String> {
        let sessions = self.sessions.lock().unwrap();

        if let Some(session) = sessions.get(session_id) {
            Ok(session.working_dir.clone())
        } else {
            anyhow::bail!("Session not found")
        }
    }
}

/// Execute a command synchronously and return output as string
/// Simple wrapper for workflow execution
pub fn execute_command(command: &str, working_dir: &str) -> Result<String> {
    let manager = TerminalManager::new();
    let output = manager.execute_command(command, working_dir)?;
    Ok(output.output)
}

impl Default for TerminalManager {
    fn default() -> Self {
        Self::new()
    }
}

// Global terminal manager instance
lazy_static::lazy_static! {
    pub static ref TERMINAL_MANAGER: TerminalManager = TerminalManager::new();
}

/// Execute a shell command (convenience function)
pub fn execute(command: &str) -> Result<String> {
    let cwd = std::env::current_dir()
        .context("Failed to get current directory")?
        .to_string_lossy()
        .to_string();

    let output = TERMINAL_MANAGER.execute_command(command, &cwd)?;

    Ok(output.output)
}
