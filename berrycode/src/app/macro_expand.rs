//! Macro expansion: send rust-analyzer/expandMacro request and display result

use super::BerryCodeApp;
use super::types::LspResponse;

impl BerryCodeApp {
    /// Expand macro at current cursor position (rust-analyzer custom request)
    pub(crate) fn expand_macro_at_cursor(&mut self) {
        if !self.lsp_connected {
            self.status_message = "LSP not connected".to_string();
            self.status_message_timestamp = Some(std::time::Instant::now());
            return;
        }

        let tab = match self.editor_tabs.get(self.active_tab_idx) {
            Some(t) => t,
            None => return,
        };

        if !tab.file_path.ends_with(".rs") {
            self.status_message = "Macro expansion only works for Rust files".to_string();
            self.status_message_timestamp = Some(std::time::Instant::now());
            return;
        }

        let file_path = tab.file_path.clone();
        let line = tab.cursor_line as u32;
        let col = tab.cursor_col as u32;

        let client = match &self.lsp_native_client {
            Some(c) => std::sync::Arc::clone(c),
            None => return,
        };

        let tx = match &self.lsp_response_tx {
            Some(t) => t.clone(),
            None => return,
        };

        self.status_message = "Expanding macro...".to_string();
        self.status_message_timestamp = Some(std::time::Instant::now());

        let runtime = self.lsp_runtime.clone();
        runtime.spawn(async move {
            // rust-analyzer/expandMacro is a custom request
            // We use the generic send_request method
            let result = expand_macro_request(&client, &file_path, line, col).await;
            match result {
                Ok(Some((name, expansion))) => {
                    let _ = tx.send(LspResponse::MacroExpansion(name, expansion));
                }
                Ok(None) => {
                    tracing::info!("No macro at cursor position");
                }
                Err(e) => {
                    tracing::debug!("Macro expansion error: {}", e);
                }
            }
        });
    }

    /// Handle keyboard shortcut for macro expansion (Ctrl+Shift+M)
    pub(crate) fn handle_macro_expand_shortcut(&mut self, ctx: &egui::Context) {
        let triggered = ctx.input(|i| {
            (i.modifiers.command || i.modifiers.ctrl) && i.modifiers.shift && i.key_pressed(egui::Key::M)
        });
        if triggered {
            self.expand_macro_at_cursor();
        }
    }
}

/// Send the rust-analyzer/expandMacro custom request
async fn expand_macro_request(
    client: &std::sync::Arc<crate::native::lsp_native::NativeLspClient>,
    file_path: &str,
    line: u32,
    character: u32,
) -> anyhow::Result<Option<(String, String)>> {
    // The rust-analyzer/expandMacro request takes a TextDocumentPositionParams
    // and returns { name: string, expansion: string }
    let uri = lsp_types::Url::from_file_path(file_path)
        .map_err(|_| anyhow::anyhow!("Invalid file path"))?;

    let params = lsp_types::TextDocumentPositionParams {
        text_document: lsp_types::TextDocumentIdentifier { uri },
        position: lsp_types::Position { line, character },
    };

    // Use the client's generic send method for custom requests
    let response = client
        .send_custom_request("rust", "rust-analyzer/expandMacro", params)
        .await?;

    if let Some(result) = response.get("result") {
        if !result.is_null() {
            let name = result
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("macro")
                .to_string();
            let expansion = result
                .get("expansion")
                .and_then(|e| e.as_str())
                .unwrap_or("")
                .to_string();
            if !expansion.is_empty() {
                return Ok(Some((name, expansion)));
            }
        }
    }

    Ok(None)
}
