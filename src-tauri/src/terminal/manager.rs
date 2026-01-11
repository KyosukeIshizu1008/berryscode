use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;

// Import from the parent project's persistent_terminal module
// We'll need to copy or link that module into this project
use crate::persistent_terminal::{TerminalManager as CoreTerminalManager, PersistentTerminal};

/// Terminal manager state for Tauri app
#[derive(Clone)]
pub struct TerminalManagerState {
    /// Core terminal manager
    pub core_manager: Arc<CoreTerminalManager>,
    /// Map of project path to terminal session ID
    pub project_terminals: Arc<RwLock<HashMap<String, String>>>,
}

impl TerminalManagerState {
    pub fn new() -> Self {
        Self {
            core_manager: Arc::new(CoreTerminalManager::new()),
            project_terminals: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get or create a terminal session for a project
    pub async fn get_or_create_terminal(&self, project_path: PathBuf) -> Result<Arc<PersistentTerminal>> {
        let project_path_str = project_path.to_string_lossy().to_string();

        // Check if terminal session exists
        let project_terminals = self.project_terminals.read().await;
        if let Some(terminal_session_id) = project_terminals.get(&project_path_str) {
            if let Some(terminal) = self.core_manager.get_session(terminal_session_id).await {
                return Ok(terminal);
            }
        }
        drop(project_terminals);

        // Create new terminal session
        let terminal_session_id = self.core_manager.create_session(project_path.clone()).await?;

        // Store mapping
        self.project_terminals
            .write()
            .await
            .insert(project_path_str, terminal_session_id.clone());

        // Get and return terminal
        self.core_manager
            .get_session(&terminal_session_id)
            .await
            .ok_or_else(|| anyhow::anyhow!("Failed to get terminal session"))
    }

    /// Terminate all terminal sessions
    pub async fn terminate_all(&self) -> Result<()> {
        self.core_manager.terminate_all().await
    }
}

impl Default for TerminalManagerState {
    fn default() -> Self {
        Self::new()
    }
}
