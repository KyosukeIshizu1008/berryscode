//! Code actions: lightbulb indicator + popup menu + workspace edit application

use super::BerryCodeApp;
use super::types::{LspCodeAction, LspResponse};
use crate::native;

impl BerryCodeApp {
    /// Request code actions from LSP for the current cursor position
    pub(crate) fn trigger_code_actions(&mut self) {
        if !self.lsp_connected {
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
        let line = tab.cursor_line as u32;
        let col = tab.cursor_col as u32;

        // Collect diagnostics for current line to send as context
        let line_diagnostics: Vec<lsp_types::Diagnostic> = self
            .lsp_diagnostics
            .iter()
            .filter(|d| d.line == tab.cursor_line)
            .map(|d| {
                let severity = match d.severity {
                    super::types::DiagnosticSeverity::Error => lsp_types::DiagnosticSeverity::ERROR,
                    super::types::DiagnosticSeverity::Warning => lsp_types::DiagnosticSeverity::WARNING,
                    super::types::DiagnosticSeverity::Information => lsp_types::DiagnosticSeverity::INFORMATION,
                    super::types::DiagnosticSeverity::Hint => lsp_types::DiagnosticSeverity::HINT,
                };
                lsp_types::Diagnostic {
                    range: lsp_types::Range {
                        start: lsp_types::Position { line, character: d.column as u32 },
                        end: lsp_types::Position { line, character: d.column as u32 },
                    },
                    severity: Some(severity),
                    message: d.message.clone(),
                    ..Default::default()
                }
            })
            .collect();

        let client = match &self.lsp_native_client {
            Some(c) => std::sync::Arc::clone(c),
            None => return,
        };

        let tx = match &self.lsp_response_tx {
            Some(t) => t.clone(),
            None => return,
        };

        self.code_action_line = tab.cursor_line;

        let runtime = self.lsp_runtime.clone();
        runtime.spawn(async move {
            match client
                .get_code_actions("rust", file_path, line, col, line, col, line_diagnostics)
                .await
            {
                Ok(actions) => {
                    let lsp_actions: Vec<LspCodeAction> = actions
                        .iter()
                        .filter_map(|a| match a {
                            lsp_types::CodeActionOrCommand::CodeAction(ca) => {
                                Some(LspCodeAction {
                                    title: ca.title.clone(),
                                    kind: ca.kind.as_ref().map(|k| k.as_str().to_string()),
                                    edit_json: ca.edit.as_ref().and_then(|e| serde_json::to_string(e).ok()),
                                    command_json: ca.command.as_ref().and_then(|c| serde_json::to_string(c).ok()),
                                })
                            }
                            lsp_types::CodeActionOrCommand::Command(cmd) => {
                                Some(LspCodeAction {
                                    title: cmd.title.clone(),
                                    kind: None,
                                    edit_json: None,
                                    command_json: serde_json::to_string(cmd).ok(),
                                })
                            }
                        })
                        .collect();
                    let _ = tx.send(LspResponse::CodeActions(lsp_actions));
                }
                Err(e) => {
                    tracing::debug!("Code actions error: {}", e);
                }
            }
        });
    }

    /// Render the code actions popup as a floating window
    pub(crate) fn render_code_actions_window(&mut self, ctx: &egui::Context) {
        if !self.show_code_actions || self.lsp_code_actions.is_empty() {
            return;
        }

        let mut selected_action: Option<usize> = None;

        egui::Window::new("Code Actions")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .fixed_size(egui::vec2(400.0, 0.0))
            .anchor(egui::Align2::LEFT_TOP, egui::vec2(100.0, 120.0))
            .frame(egui::Frame::window(&ctx.style()).fill(egui::Color32::from_rgb(37, 37, 38)))
            .show(ctx, |ui| {
                ui.style_mut().spacing.item_spacing.y = 1.0;

                for (idx, action) in self.lsp_code_actions.iter().enumerate() {
                    let icon = match action.kind.as_deref() {
                        Some(k) if k.starts_with("quickfix") => "\u{eb2f}",
                        Some(k) if k.starts_with("refactor") => "\u{eb44}",
                        Some(k) if k.starts_with("source") => "\u{ea82}",
                        _ => "\u{eb2f}",
                    };

                    let btn = ui.add(
                        egui::Button::new(
                            egui::RichText::new(format!("{} {}", icon, action.title))
                                .size(12.0)
                                .color(egui::Color32::from_rgb(210, 210, 210)),
                        )
                        .frame(false)
                        .min_size(egui::vec2(390.0, 22.0)),
                    );

                    if btn.clicked() {
                        selected_action = Some(idx);
                    }
                }
            });

        // Close on Escape
        let escape = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        if escape {
            self.show_code_actions = false;
            self.lsp_code_actions.clear();
        }

        // Apply selected action
        if let Some(idx) = selected_action {
            let action = self.lsp_code_actions[idx].clone();
            self.apply_code_action(&action);
            self.show_code_actions = false;
            self.lsp_code_actions.clear();
        }
    }

    /// Apply a code action's workspace edit
    fn apply_code_action(&mut self, action: &LspCodeAction) {
        if let Some(edit_json) = &action.edit_json {
            if let Ok(edit) = serde_json::from_str::<lsp_types::WorkspaceEdit>(edit_json) {
                self.apply_workspace_edit(&edit);
            }
        }
        self.status_message = format!("Applied: {}", action.title);
        self.status_message_timestamp = Some(std::time::Instant::now());
    }

    /// Apply a WorkspaceEdit (used by code actions and rename)
    pub(crate) fn apply_workspace_edit(&mut self, edit: &lsp_types::WorkspaceEdit) {
        if let Some(changes) = &edit.changes {
            for (uri, text_edits) in changes {
                let file_path = uri.to_file_path().ok();
                let file_path = match file_path {
                    Some(p) => p.to_string_lossy().to_string(),
                    None => continue,
                };

                // Find or open the tab
                let tab_idx = self.editor_tabs.iter().position(|t| t.file_path == file_path);
                let tab_idx = match tab_idx {
                    Some(i) => i,
                    None => {
                        // Read and open the file
                        if let Ok(content) = native::fs::read_file(&file_path) {
                            let tab = super::types::EditorTab::new(file_path.clone(), content);
                            self.editor_tabs.push(tab);
                            self.editor_tabs.len() - 1
                        } else {
                            continue;
                        }
                    }
                };

                // Apply edits in reverse order (so offsets don't shift)
                let mut sorted_edits = text_edits.clone();
                sorted_edits.sort_by(|a, b| {
                    b.range.start.line.cmp(&a.range.start.line)
                        .then(b.range.start.character.cmp(&a.range.start.character))
                });

                let tab = &mut self.editor_tabs[tab_idx];
                let mut text = tab.buffer.to_string();

                for te in &sorted_edits {
                    let lines: Vec<&str> = text.lines().collect();
                    let start_line = te.range.start.line as usize;
                    let end_line = te.range.end.line as usize;
                    let start_char = te.range.start.character as usize;
                    let end_char = te.range.end.character as usize;

                    // Calculate byte offsets
                    let start_byte: usize = lines.iter().take(start_line).map(|l| l.len() + 1).sum::<usize>() + start_char;
                    let end_byte: usize = lines.iter().take(end_line).map(|l| l.len() + 1).sum::<usize>() + end_char;

                    let start_byte = start_byte.min(text.len());
                    let end_byte = end_byte.min(text.len());

                    text.replace_range(start_byte..end_byte, &te.new_text);
                }

                tab.buffer = crate::buffer::TextBuffer::from_str(&text);
                tab.mark_dirty();
            }
        }
    }

    /// Handle Ctrl+. (or Cmd+.) to trigger code actions
    pub(crate) fn handle_code_action_shortcut(&mut self, ctx: &egui::Context) {
        let triggered = ctx.input(|i| {
            (i.modifiers.command || i.modifiers.ctrl) && i.key_pressed(egui::Key::Period)
        });
        if triggered {
            self.trigger_code_actions();
        }
    }
}
