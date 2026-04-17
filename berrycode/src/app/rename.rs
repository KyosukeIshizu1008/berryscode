//! Rename Symbol dialog and logic

use super::BerryCodeApp;

impl BerryCodeApp {
    /// Render the rename dialog window
    pub(crate) fn render_rename_dialog(&mut self, ctx: &egui::Context) {
        if !self.rename_dialog_open {
            return;
        }

        let mut should_execute = false;
        let mut should_close = false;

        egui::Window::new("Rename Symbol")
            .collapsible(false)
            .resizable(false)
            .default_width(300.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("New name:");
                    let response = ui.text_edit_singleline(&mut self.rename_new_name);
                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        should_execute = true;
                    }
                    // Auto-focus the text field when dialog opens
                    response.request_focus();
                });
                ui.horizontal(|ui| {
                    if ui.button("Rename").clicked() {
                        should_execute = true;
                    }
                    if ui.button("Cancel").clicked() {
                        should_close = true;
                    }
                });
            });

        if should_execute {
            self.execute_rename();
        } else if should_close {
            self.rename_dialog_open = false;
        }
    }

    /// Open the rename dialog, pre-filling with the word at cursor
    pub(crate) fn open_rename_dialog(&mut self) {
        if self.editor_tabs.is_empty() {
            return;
        }

        let tab = match self.editor_tabs.get(self.active_tab_idx) {
            Some(t) => t,
            None => return,
        };

        let text = tab.buffer.to_string();
        let cursor_line = tab.cursor_line;
        let cursor_col = tab.cursor_col;

        // Calculate byte offset from line/col
        let cursor_pos = {
            let mut pos = 0;
            for (line_idx, line) in text.lines().enumerate() {
                if line_idx == cursor_line {
                    pos += cursor_col.min(line.len());
                    break;
                }
                pos += line.len() + 1;
            }
            pos
        };

        let word = self.extract_word_at_position(&text, cursor_pos);
        self.rename_new_name = word;
        self.rename_dialog_open = true;
    }

    /// Execute the rename operation via LSP
    fn execute_rename(&mut self) {
        if self.rename_new_name.is_empty() {
            self.rename_dialog_open = false;
            return;
        }

        let tab = match self.editor_tabs.get(self.active_tab_idx) {
            Some(t) => t,
            None => {
                self.rename_dialog_open = false;
                return;
            }
        };

        let file_path = tab.file_path.clone();
        let line = tab.cursor_line as u32;
        let col = {
            let text = tab.buffer.to_string();
            let lines: Vec<&str> = text.lines().collect();
            if (line as usize) < lines.len() {
                super::utils::utf8_offset_to_utf16(lines[line as usize], tab.cursor_col) as u32
            } else {
                tab.cursor_col as u32
            }
        };
        let new_name = self.rename_new_name.clone();

        if let Some(lang) = crate::native::lsp_native::detect_server_language(&file_path) {
            if let Some(client) = &self.lsp_native_client {
                let client = client.clone();
                let language = lang.to_string();
                self.lsp_runtime.spawn(async move {
                    match client
                        .rename_symbol(&language, file_path.clone(), line, col, &new_name)
                        .await
                    {
                        Ok(Some(edit)) => {
                            // Apply workspace edit
                            if let Some(changes) = &edit.changes {
                                for (uri, edits) in changes {
                                    let path = uri.to_file_path().unwrap_or_default();
                                    let path_str = path.to_string_lossy().to_string();
                                    if let Ok(content) = std::fs::read_to_string(&path) {
                                        let new_content = apply_text_edits(&content, edits);
                                        let _ = std::fs::write(&path, &new_content);
                                        tracing::info!("Renamed in {}", path_str);
                                    }
                                }
                            }
                        }
                        Ok(None) => {
                            tracing::warn!("Rename not supported at this position");
                        }
                        Err(e) => {
                            tracing::error!("Rename failed: {}", e);
                        }
                    }
                });
            }
        }

        self.rename_dialog_open = false;
    }
}

/// Apply LSP TextEdits to a string (edits must be applied in reverse order)
fn apply_text_edits(content: &str, edits: &[lsp_types::TextEdit]) -> String {
    let mut result = content.to_string();
    let mut sorted_edits: Vec<_> = edits.to_vec();
    sorted_edits.sort_by(|a, b| {
        b.range
            .start
            .line
            .cmp(&a.range.start.line)
            .then(b.range.start.character.cmp(&a.range.start.character))
    });

    for edit in &sorted_edits {
        let start = lsp_position_to_offset(&result, edit.range.start);
        let end = lsp_position_to_offset(&result, edit.range.end);
        if let (Some(s), Some(e)) = (start, end) {
            result.replace_range(s..e, &edit.new_text);
        }
    }
    result
}

/// Convert an LSP Position (line, character) to a byte offset in the text
fn lsp_position_to_offset(text: &str, pos: lsp_types::Position) -> Option<usize> {
    let mut offset = 0;
    for (idx, line) in text.lines().enumerate() {
        if idx == pos.line as usize {
            return Some(offset + (pos.character as usize).min(line.len()));
        }
        offset += line.len() + 1; // +1 for newline
    }
    None
}
