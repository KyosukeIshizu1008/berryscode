//! Inlay hints: fetch from LSP and render inline ghost text (type annotations, parameter names)

use super::BerryCodeApp;
use super::types::{LspInlayHint, LspResponse};

impl BerryCodeApp {
    /// Periodically request inlay hints from LSP for the current file
    pub(crate) fn poll_inlay_hints(&mut self) {
        if !self.inlay_hints_enabled || !self.lsp_connected {
            return;
        }

        let now = std::time::Instant::now();
        let should_request = self
            .inlay_hints_last_request
            .map_or(true, |last| now.duration_since(last).as_millis() > 1000);

        if !should_request {
            return;
        }

        let tab = match self.editor_tabs.get(self.active_tab_idx) {
            Some(t) => t,
            None => return,
        };

        if !tab.file_path.ends_with(".rs") {
            return;
        }

        let file_path = tab.file_path.clone();
        let total_lines = tab.buffer.to_string().lines().count() as u32;

        let client = match &self.lsp_native_client {
            Some(c) => std::sync::Arc::clone(c),
            None => return,
        };

        let tx = match &self.lsp_response_tx {
            Some(t) => t.clone(),
            None => return,
        };

        self.inlay_hints_last_request = Some(now);

        let runtime = self.lsp_runtime.clone();
        runtime.spawn(async move {
            match client
                .get_inlay_hints("rust", file_path, 0, total_lines)
                .await
            {
                Ok(hints) => {
                    let lsp_hints: Vec<LspInlayHint> = hints
                        .iter()
                        .filter_map(|h| {
                            let label = match &h.label {
                                lsp_types::InlayHintLabel::String(s) => s.clone(),
                                lsp_types::InlayHintLabel::LabelParts(parts) => {
                                    parts.iter().map(|p| p.value.as_str()).collect()
                                }
                            };
                            let kind = match h.kind {
                                Some(lsp_types::InlayHintKind::TYPE) => "type",
                                Some(lsp_types::InlayHintKind::PARAMETER) => "parameter",
                                _ => "type",
                            };
                            Some(LspInlayHint {
                                line: h.position.line as usize,
                                column: h.position.character as usize,
                                label,
                                kind,
                            })
                        })
                        .collect();
                    let _ = tx.send(LspResponse::InlayHints(lsp_hints));
                }
                Err(e) => {
                    tracing::debug!("Inlay hints error: {}", e);
                }
            }
        });
    }

    /// Render inlay hints as ghost text in the editor gutter area.
    /// Call this from within the editor rendering loop, after drawing each line's text.
    ///
    /// Returns a list of (line, x_offset, label) to draw after the main text.
    pub(crate) fn get_inlay_hints_for_line(&self, line_idx: usize) -> Vec<(usize, String, &'static str)> {
        if !self.inlay_hints_enabled {
            return Vec::new();
        }
        self.lsp_inlay_hints
            .iter()
            .filter(|h| h.line == line_idx)
            .map(|h| (h.column, h.label.clone(), h.kind))
            .collect()
    }
}
