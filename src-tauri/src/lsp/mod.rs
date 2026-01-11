//! LSP (Language Server Protocol) Module
//! Manages Language Server processes and communication
//!
//! This module now uses the unified lsp_core implementation
//! to eliminate code duplication.

pub mod commands;

// Re-export from lsp_core
pub use crate::lsp_core::{LspClient, LspMessage, LspNotification, LspRequest, LspResponse};
pub use commands::register_lsp_commands;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Global LSP manager
pub struct LspManager {
    clients: Arc<Mutex<HashMap<String, Arc<Mutex<LspClient>>>>>,
}

impl LspManager {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Get or create LSP client for a language
    pub fn get_client(&self, language: &str) -> Option<Arc<Mutex<LspClient>>> {
        let clients = self.clients.lock().unwrap();
        clients.get(language).cloned()
    }

    /// Initialize LSP client for a language
    pub fn initialize_client(&self, language: String, root_uri: String) -> Result<(), String> {
        let mut clients = self.clients.lock().unwrap();

        if clients.contains_key(&language) {
            return Ok(()); // Already initialized
        }

        let client = LspClient::new(&language, &root_uri)?;
        clients.insert(language, Arc::new(Mutex::new(client)));

        Ok(())
    }

    /// Shutdown LSP client for a language
    pub fn shutdown_client(&self, language: &str) -> Result<(), String> {
        let mut clients = self.clients.lock().unwrap();

        if let Some(client_arc) = clients.remove(language) {
            let mut client = client_arc.lock().unwrap();
            client.shutdown()?;
        }

        Ok(())
    }

    /// Shutdown all LSP clients
    pub fn shutdown_all(&self) -> Result<(), String> {
        let mut clients = self.clients.lock().unwrap();

        for (_lang, client_arc) in clients.drain() {
            let mut client = client_arc.lock().unwrap();
            if let Err(e) = client.shutdown() {
                eprintln!("Error shutting down LSP client: {}", e);
            }
        }

        Ok(())
    }
}

impl Default for LspManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lsp_manager_creation() {
        let manager = LspManager::new();
        assert!(manager.get_client("rust").is_none());
    }

    #[test]
    fn test_lsp_manager_default() {
        let manager = LspManager::default();
        assert!(manager.get_client("typescript").is_none());
    }
}
