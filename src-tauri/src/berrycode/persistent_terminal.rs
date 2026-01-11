//! Persistent Terminal Session Manager
//!
//! Devin-style persistent terminal sessions using PTY (Pseudo-Terminal).
//! Unlike std::process::Command which is stateless, this maintains:
//! - Working directory state (cd persists)
//! - Environment variables
//! - Background processes (npm run dev, servers, etc.)
//!
//! Inspired by portable-pty (WezTerm) and zellij architectures.

use anyhow::{anyhow, Result};
use portable_pty::{Child, CommandBuilder, PtySize, native_pty_system};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

/// Process status
#[derive(Debug, Clone, PartialEq)]
pub enum ProcessStatus {
    Running,
    Completed(i32), // Exit code
    Failed(String),
}

/// Background process information
#[derive(Debug)]
pub struct BackgroundProcess {
    pub id: String,
    pub command: String,
    pub pid: u32,
    pub status: ProcessStatus,
    pub output_buffer: Vec<String>,
}

/// Persistent terminal session
///
/// Maintains state across multiple command executions.
/// This is the core of Devin-style terminal management.
pub struct PersistentTerminal {
    /// Session ID
    pub session_id: String,

    /// Working directory
    pub working_dir: PathBuf,

    /// PTY writer
    pty_writer: Arc<Mutex<Box<dyn Write + Send>>>,

    /// PTY reader
    pty_reader: Arc<Mutex<Box<dyn Read + Send>>>,

    /// PTY child process
    pty_child: Arc<Mutex<Box<dyn Child + Send + Sync>>>,

    /// Background processes
    background_processes: Arc<RwLock<HashMap<String, BackgroundProcess>>>,

    /// Command history
    command_history: Arc<RwLock<Vec<String>>>,

    /// Environment variables
    env_vars: Arc<RwLock<HashMap<String, String>>>,
}

impl PersistentTerminal {
    /// Create a new persistent terminal session
    pub fn new(working_dir: PathBuf) -> Result<Self> {
        let session_id = Uuid::new_v4().to_string();

        // Get native PTY system
        let pty_system = native_pty_system();

        // Create PTY pair
        let pty_pair = pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        // Spawn shell process
        let mut cmd = CommandBuilder::new("bash");
        cmd.cwd(&working_dir);

        // Add initialization commands to make shell more predictable
        cmd.args(&[
            "-c",
            "PS1='$ '; unset PROMPT_COMMAND; exec bash --norc --noprofile",
        ]);

        let child = pty_pair.slave.spawn_command(cmd)?;

        // Split reader and writer
        let reader = pty_pair.master.try_clone_reader()?;
        let writer = pty_pair.master.take_writer()?;

        tracing::info!(
            "Created persistent terminal session: {} in {:?}",
            session_id,
            working_dir
        );

        Ok(Self {
            session_id,
            working_dir,
            pty_writer: Arc::new(Mutex::new(writer)),
            pty_reader: Arc::new(Mutex::new(reader)),
            pty_child: Arc::new(Mutex::new(child)),
            background_processes: Arc::new(RwLock::new(HashMap::new())),
            command_history: Arc::new(RwLock::new(Vec::new())),
            env_vars: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Execute a command in the persistent session
    ///
    /// Unlike std::process::Command, this maintains state:
    /// - `cd /tmp` persists for next command
    /// - Environment variables persist
    /// - Background processes continue running
    pub async fn execute(&self, command: &str) -> Result<String> {
        // Add to history
        self.command_history.write().await.push(command.to_string());

        // Write command to PTY
        let mut writer = self.pty_writer.lock().await;
        let command_bytes = format!("{}\n", command).into_bytes();
        writer.write_all(&command_bytes)?;
        writer.flush()?;
        drop(writer);

        // Read output with timeout
        let output = self.read_output_with_timeout(Duration::from_secs(10)).await?;

        tracing::debug!(
            "Executed command '{}' in session {}: {} bytes output",
            command,
            self.session_id,
            output.len()
        );

        Ok(output)
    }

    /// Execute a command in background (for long-running processes)
    ///
    /// Example: `npm run dev`, `python -m http.server`
    /// The process continues running and output is buffered.
    pub async fn execute_background(&self, command: &str) -> Result<String> {
        let process_id = Uuid::new_v4().to_string();

        // Execute in background with &
        let bg_command = format!("{} &", command);
        let output = self.execute(&bg_command).await?;

        // Extract PID from output (bash prints "[1] <PID>" format)
        let pid = self.extract_pid_from_output(&output).unwrap_or(0);

        // Create background process record
        let bg_process = BackgroundProcess {
            id: process_id.clone(),
            command: command.to_string(),
            pid,
            status: ProcessStatus::Running,
            output_buffer: Vec::new(),
        };

        self.background_processes
            .write()
            .await
            .insert(process_id.clone(), bg_process);

        tracing::info!(
            "Started background process '{}' with ID {} (PID: {})",
            command,
            process_id,
            pid
        );

        Ok(process_id)
    }

    /// Check if a background process is still running
    pub async fn is_process_running(&self, process_id: &str) -> bool {
        if let Some(process) = self.background_processes.read().await.get(process_id) {
            matches!(process.status, ProcessStatus::Running)
        } else {
            false
        }
    }

    /// Kill a background process
    pub async fn kill_process(&self, process_id: &str) -> Result<()> {
        let processes = self.background_processes.read().await;
        let process = processes
            .get(process_id)
            .ok_or_else(|| anyhow!("Process not found: {}", process_id))?;

        let pid = process.pid;
        drop(processes);

        // Send SIGTERM
        let kill_command = format!("kill {}", pid);
        self.execute(&kill_command).await?;

        // Update status
        self.background_processes
            .write()
            .await
            .get_mut(process_id)
            .unwrap()
            .status = ProcessStatus::Completed(143); // SIGTERM exit code

        tracing::info!("Killed background process {} (PID: {})", process_id, pid);

        Ok(())
    }

    /// Get all background processes
    pub async fn list_background_processes(&self) -> Vec<BackgroundProcess> {
        self.background_processes
            .read()
            .await
            .values()
            .map(|p| BackgroundProcess {
                id: p.id.clone(),
                command: p.command.clone(),
                pid: p.pid,
                status: p.status.clone(),
                output_buffer: p.output_buffer.clone(),
            })
            .collect()
    }

    /// Change working directory (persists across commands)
    pub async fn cd(&self, path: &str) -> Result<()> {
        let cd_command = format!("cd {}", path);
        self.execute(&cd_command).await?;

        // Update internal working_dir tracking
        let pwd_output = self.execute("pwd").await?;
        let new_dir = PathBuf::from(pwd_output.trim());

        tracing::info!(
            "Changed directory in session {} to {:?}",
            self.session_id,
            new_dir
        );

        Ok(())
    }

    /// Set environment variable (persists across commands)
    pub async fn set_env(&self, key: &str, value: &str) -> Result<()> {
        let export_command = format!("export {}='{}'", key, value);
        self.execute(&export_command).await?;

        self.env_vars
            .write()
            .await
            .insert(key.to_string(), value.to_string());

        tracing::debug!("Set environment variable {}={}", key, value);

        Ok(())
    }

    /// Get current working directory
    pub async fn get_cwd(&self) -> Result<PathBuf> {
        let output = self.execute("pwd").await?;

        // Parse output which may contain command echo and prompt
        // Format: "pwd\r\n/tmp\r\nbash-3.2$" or just "/tmp"
        // Find the line that looks like a path (starts with /)
        let lines: Vec<&str> = output.lines().collect();

        let path = lines.iter()
            .map(|line| line.trim())
            .find(|line| line.starts_with('/'))
            .unwrap_or_else(|| output.trim());

        Ok(PathBuf::from(path))
    }

    /// Get command history
    pub async fn get_history(&self) -> Vec<String> {
        self.command_history.read().await.clone()
    }

    /// Read output from PTY with timeout
    async fn read_output_with_timeout(&self, _timeout: Duration) -> Result<String> {
        // For now, use a simple blocking read
        // TODO: Implement proper async reading with timeout
        let mut reader = self.pty_reader.lock().await;
        let mut output = Vec::new();
        let mut buf = [0u8; 4096];

        // Read with a simple timeout (blocking)
        std::thread::sleep(Duration::from_millis(500));

        loop {
            match reader.read(&mut buf) {
                Ok(0) => break, // EOF
                Ok(n) => {
                    output.extend_from_slice(&buf[..n]);
                    let text = String::from_utf8_lossy(&output);

                    // Simple heuristic: stop after seeing prompt
                    if text.contains("$ ") || text.ends_with("$ ") {
                        break;
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    break; // No more data available
                }
                Err(e) => return Err(anyhow!("Failed to read from PTY: {}", e)),
            }

            // Prevent infinite loop
            if output.len() > 1_000_000 {
                break;
            }
        }

        Ok(String::from_utf8_lossy(&output).to_string())
    }

    /// Extract PID from bash background job output
    fn extract_pid_from_output(&self, output: &str) -> Option<u32> {
        // Parse "[1] 12345" format
        output
            .lines()
            .find(|line| line.starts_with('['))
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|pid_str| pid_str.parse().ok())
    }

    /// Check if terminal is still alive
    pub async fn is_alive(&self) -> bool {
        let mut child = self.pty_child.lock().await;
        child.try_wait().is_ok()
    }

    /// Cleanup and terminate session
    pub async fn terminate(&self) -> Result<()> {
        // Kill all background processes
        let process_ids: Vec<String> = self
            .background_processes
            .read()
            .await
            .keys()
            .cloned()
            .collect();

        for process_id in process_ids {
            let _ = self.kill_process(&process_id).await;
        }

        // Send exit to shell
        let _ = self.execute("exit").await;

        tracing::info!("Terminated persistent terminal session {}", self.session_id);

        Ok(())
    }
}

/// Terminal session manager
///
/// Manages multiple persistent terminal sessions.
/// Similar to tmux/zellij but programmatic.
pub struct TerminalManager {
    sessions: Arc<RwLock<HashMap<String, Arc<PersistentTerminal>>>>,
}

impl TerminalManager {
    /// Create new terminal manager
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new session
    pub async fn create_session(&self, working_dir: PathBuf) -> Result<String> {
        let terminal = Arc::new(PersistentTerminal::new(working_dir)?);
        let session_id = terminal.session_id.clone();

        self.sessions
            .write()
            .await
            .insert(session_id.clone(), terminal);

        Ok(session_id)
    }

    /// Get a session by ID
    pub async fn get_session(&self, session_id: &str) -> Option<Arc<PersistentTerminal>> {
        self.sessions.read().await.get(session_id).cloned()
    }

    /// List all sessions
    pub async fn list_sessions(&self) -> Vec<String> {
        self.sessions.read().await.keys().cloned().collect()
    }

    /// Terminate a session
    pub async fn terminate_session(&self, session_id: &str) -> Result<()> {
        if let Some(terminal) = self.sessions.write().await.remove(session_id) {
            terminal.terminate().await?;
        }
        Ok(())
    }

    /// Terminate all sessions
    pub async fn terminate_all(&self) -> Result<()> {
        let session_ids: Vec<String> = self.list_sessions().await;
        for session_id in session_ids {
            self.terminate_session(&session_id).await?;
        }
        Ok(())
    }
}

impl Default for TerminalManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[tokio::test]
    async fn test_persistent_cd() {
        let terminal = PersistentTerminal::new(env::current_dir().unwrap()).unwrap();

        // Change directory
        terminal.cd("/tmp").await.unwrap();

        // Verify it persists
        let cwd = terminal.get_cwd().await.unwrap();
        assert_eq!(cwd, PathBuf::from("/tmp"));
    }

    #[tokio::test]
    async fn test_command_execution() {
        let terminal = PersistentTerminal::new(env::current_dir().unwrap()).unwrap();

        let output = terminal.execute("echo 'Hello, World!'").await.unwrap();
        assert!(output.contains("Hello, World!"));
    }

    #[tokio::test]
    async fn test_environment_variables() {
        let terminal = PersistentTerminal::new(env::current_dir().unwrap()).unwrap();

        terminal.set_env("TEST_VAR", "test_value").await.unwrap();

        let output = terminal.execute("echo $TEST_VAR").await.unwrap();
        assert!(output.contains("test_value"));
    }

    #[tokio::test]
    async fn test_terminal_manager() {
        let manager = TerminalManager::new();

        let session_id = manager
            .create_session(env::current_dir().unwrap())
            .await
            .unwrap();

        let terminal = manager.get_session(&session_id).await.unwrap();
        let output = terminal.execute("pwd").await.unwrap();
        assert!(!output.is_empty());

        manager.terminate_session(&session_id).await.unwrap();
    }
}
