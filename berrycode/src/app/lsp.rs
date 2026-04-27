//! LSP integration: completions, diagnostics, hover, go-to-definition, find references

use super::types;
use super::types::{
    DiagnosticSeverity, LspCompletionItem, LspDiagnostic, LspHoverInfo, LspLocation, LspResponse,
    PendingGotoDefinition,
};
use super::utils::{
    calculate_line_column, parse_lsp_location, utf16_offset_to_utf8, utf8_offset_to_utf16,
};
use super::BerryCodeApp;
use crate::focus_stack::FocusLayer;
use crate::native;

impl BerryCodeApp {
    /// Handle LSP keyboard shortcuts
    pub(crate) fn handle_lsp_shortcuts(&mut self, ctx: &egui::Context) {
        if self.active_focus != FocusLayer::Editor || self.editor_tabs.is_empty() {
            return;
        }

        ctx.input(|i| {
            if i.modifiers.command && i.key_pressed(egui::Key::Space) {
                if i.modifiers.shift {
                    self.trigger_lsp_signature_help();
                } else {
                    self.trigger_lsp_completions();
                }
            }

            if i.key_pressed(egui::Key::Escape) {
                if self.lsp_show_completions {
                    self.lsp_show_completions = false;
                    self.lsp_completions.clear();
                }
                if self.lsp_signature_help.is_some() {
                    self.lsp_signature_help = None;
                }
            }
        });
    }

    /// Trigger LSP completions (or Cargo.toml/snippet completions)
    pub(crate) fn trigger_lsp_completions(&mut self) {
        tracing::info!("💡 Triggering completions");

        let tab = match self.editor_tabs.get(self.active_tab_idx) {
            Some(t) => t,
            None => return,
        };

        // Cargo.toml → use crates.io completion
        if tab.file_path.ends_with("Cargo.toml") {
            self.trigger_cargo_completion();
            return;
        }

        let file_path = tab.file_path.clone();
        let line = tab.cursor_line;
        let utf8_column = tab.cursor_col;

        let utf16_column = {
            let text = tab.buffer.to_string();
            let lines: Vec<&str> = text.lines().collect();
            if line < lines.len() {
                utf8_offset_to_utf16(lines[line], utf8_column)
            } else {
                utf8_column
            }
        };

        let client = match &self.lsp_native_client {
            Some(c) => std::sync::Arc::clone(c),
            None => {
                tracing::warn!("⚠️ LSP client not initialized");
                return;
            }
        };

        let tx = match &self.lsp_response_tx {
            Some(t) => t.clone(),
            None => return,
        };

        let runtime = std::sync::Arc::clone(&self.lsp_runtime);

        runtime.spawn(async move {
            tracing::info!(
                "🚀 Requesting LSP completions at {}:{} (UTF-16)",
                line,
                utf16_column
            );

            let lang = match crate::native::lsp_native::detect_server_language(&file_path) {
                Some(l) => l,
                None => {
                    tracing::debug!("No LSP server for file: {}", file_path);
                    return;
                }
            };
            match client
                .get_completions(lang, file_path.clone(), line as u32, utf16_column as u32)
                .await
            {
                Ok(items) => {
                    tracing::info!("📋 LSP returned {} completion items", items.len());

                    let lsp_completions: Vec<LspCompletionItem> = items
                        .into_iter()
                        .map(|item| {
                            use lsp_types::CompletionItemKind;
                            let is_snippet = item.insert_text_format
                                == Some(lsp_types::InsertTextFormat::SNIPPET);
                            let insert_text = item.insert_text.clone();
                            LspCompletionItem {
                                label: item.label,
                                detail: item.detail,
                                insert_text,
                                is_snippet,
                                kind: match item.kind {
                                    Some(CompletionItemKind::TEXT) => "text",
                                    Some(CompletionItemKind::METHOD) => "method",
                                    Some(CompletionItemKind::FUNCTION) => "function",
                                    Some(CompletionItemKind::CONSTRUCTOR) => "constructor",
                                    Some(CompletionItemKind::FIELD) => "field",
                                    Some(CompletionItemKind::VARIABLE) => "variable",
                                    Some(CompletionItemKind::CLASS) => "class",
                                    Some(CompletionItemKind::INTERFACE) => "interface",
                                    Some(CompletionItemKind::MODULE) => "module",
                                    Some(CompletionItemKind::PROPERTY) => "property",
                                    Some(CompletionItemKind::UNIT) => "unit",
                                    Some(CompletionItemKind::VALUE) => "value",
                                    Some(CompletionItemKind::ENUM) => "enum",
                                    Some(CompletionItemKind::KEYWORD) => "keyword",
                                    Some(CompletionItemKind::SNIPPET) => "snippet",
                                    Some(CompletionItemKind::COLOR) => "color",
                                    Some(CompletionItemKind::FILE) => "file",
                                    Some(CompletionItemKind::REFERENCE) => "reference",
                                    Some(CompletionItemKind::FOLDER) => "folder",
                                    Some(CompletionItemKind::ENUM_MEMBER) => "enum_member",
                                    Some(CompletionItemKind::CONSTANT) => "constant",
                                    Some(CompletionItemKind::STRUCT) => "struct",
                                    Some(CompletionItemKind::EVENT) => "event",
                                    Some(CompletionItemKind::OPERATOR) => "operator",
                                    Some(CompletionItemKind::TYPE_PARAMETER) => "type_parameter",
                                    _ => "unknown",
                                }
                                .to_string(),
                            }
                        })
                        .collect();

                    if let Err(e) = tx.send(LspResponse::Completions(lsp_completions)) {
                        tracing::error!("❌ Failed to send LSP completions: {}", e);
                    }
                }
                Err(e) => {
                    tracing::warn!("⚠️ LSP get_completions failed: {}", e);
                }
            }
        });

        self.lsp_show_completions = true;
    }

    /// Trigger `textDocument/signatureHelp` for the cursor position. Called
    /// when `(` or `,` is typed inside a function call, or via Cmd+Shift+Space.
    pub(crate) fn trigger_lsp_signature_help(&mut self) {
        let tab = match self.editor_tabs.get(self.active_tab_idx) {
            Some(t) => t,
            None => return,
        };
        let file_path = tab.file_path.clone();
        let line = tab.cursor_line;
        let utf8_column = tab.cursor_col;

        let utf16_column = {
            let text = tab.buffer.to_string();
            let lines: Vec<&str> = text.lines().collect();
            if line < lines.len() {
                utf8_offset_to_utf16(lines[line], utf8_column)
            } else {
                utf8_column
            }
        };

        let client = match &self.lsp_native_client {
            Some(c) => std::sync::Arc::clone(c),
            None => return,
        };
        let tx = match &self.lsp_response_tx {
            Some(t) => t.clone(),
            None => return,
        };
        let runtime = std::sync::Arc::clone(&self.lsp_runtime);

        runtime.spawn(async move {
            let lang = match crate::native::lsp_native::detect_server_language(&file_path) {
                Some(l) => l,
                None => return,
            };
            match client
                .get_signature_help(lang, file_path, line as u32, utf16_column as u32)
                .await
            {
                Ok(Some(sig)) => {
                    let signatures: Vec<types::LspSignatureInfo> = sig
                        .signatures
                        .iter()
                        .map(|s| {
                            let label = s.label.clone();
                            let documentation = s.documentation.as_ref().and_then(|d| match d {
                                lsp_types::Documentation::String(s) => Some(s.clone()),
                                lsp_types::Documentation::MarkupContent(m) => Some(m.value.clone()),
                            });
                            let param_ranges: Vec<(usize, usize)> = s
                                .parameters
                                .as_ref()
                                .map(|params| {
                                    params
                                        .iter()
                                        .filter_map(|p| match &p.label {
                                            lsp_types::ParameterLabel::Simple(s) => {
                                                label.find(s.as_str()).map(|i| (i, i + s.len()))
                                            }
                                            lsp_types::ParameterLabel::LabelOffsets(off) => {
                                                Some((off[0] as usize, off[1] as usize))
                                            }
                                        })
                                        .collect()
                                })
                                .unwrap_or_default();
                            types::LspSignatureInfo {
                                label,
                                documentation,
                                param_ranges,
                            }
                        })
                        .collect();
                    let normalised = types::LspSignatureHelp {
                        signatures,
                        active_signature: sig.active_signature.unwrap_or(0) as usize,
                        active_parameter: sig.active_parameter.map(|p| p as usize),
                    };
                    let _ = tx.send(LspResponse::SignatureHelp(Some(normalised)));
                }
                _ => {
                    let _ = tx.send(LspResponse::SignatureHelp(None));
                }
            }
        });
    }

    /// Render the floating signature-help popup (VS Code-style: monospace
    /// signature with the active parameter underlined, optional doc below).
    pub(crate) fn render_lsp_signature_help(&mut self, ctx: &egui::Context) {
        let help = match &self.lsp_signature_help {
            Some(h) if !h.signatures.is_empty() => h.clone(),
            _ => return,
        };

        let signature = match help.signatures.get(help.active_signature) {
            Some(s) => s,
            None => return,
        };

        // Anchor the popup near the editor's cursor. We don't have the exact
        // pixel position from this scope, so place it under the active editor
        // panel using a reasonable offset; the user can scroll the editor
        // independently and the popup tracks pointer / cursor position.
        let pointer = ctx.input(|i| i.pointer.hover_pos());
        let anchor = pointer
            .map(|p| egui::pos2(p.x, p.y + 24.0))
            .unwrap_or_else(|| egui::pos2(80.0, 80.0));

        egui::Area::new(egui::Id::new("lsp_signature_help_popup"))
            .order(egui::Order::Foreground)
            .fixed_pos(anchor)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .fill(egui::Color32::from_rgb(35, 36, 40))
                    .stroke(egui::Stroke::new(
                        1.0,
                        egui::Color32::from_rgb(75, 110, 175),
                    ))
                    .inner_margin(egui::Margin::same(8))
                    .show(ui, |ui| {
                        ui.set_max_width(560.0);

                        // Build a coloured layout job: active parameter is
                        // bright + bold, the rest dim.
                        let mut job = egui::text::LayoutJob::default();
                        let active = help.active_parameter.unwrap_or(usize::MAX);
                        let active_range = signature.param_ranges.get(active).copied();
                        let label = &signature.label;

                        let dim = egui::Color32::from_rgb(180, 180, 190);
                        let bright = egui::Color32::from_rgb(255, 255, 255);
                        let accent = egui::Color32::from_rgb(120, 200, 255);

                        match active_range {
                            Some((s, e)) if s < label.len() && e <= label.len() && s < e => {
                                job.append(
                                    &label[..s],
                                    0.0,
                                    egui::TextFormat {
                                        font_id: egui::FontId::monospace(13.0),
                                        color: dim,
                                        ..Default::default()
                                    },
                                );
                                job.append(
                                    &label[s..e],
                                    0.0,
                                    egui::TextFormat {
                                        font_id: egui::FontId::monospace(13.0),
                                        color: accent,
                                        underline: egui::Stroke::new(1.0, accent),
                                        ..Default::default()
                                    },
                                );
                                job.append(
                                    &label[e..],
                                    0.0,
                                    egui::TextFormat {
                                        font_id: egui::FontId::monospace(13.0),
                                        color: dim,
                                        ..Default::default()
                                    },
                                );
                            }
                            _ => {
                                job.append(
                                    label,
                                    0.0,
                                    egui::TextFormat {
                                        font_id: egui::FontId::monospace(13.0),
                                        color: bright,
                                        ..Default::default()
                                    },
                                );
                            }
                        }
                        ui.label(job);

                        if help.signatures.len() > 1 {
                            ui.label(
                                egui::RichText::new(format!(
                                    "{}/{} overloads — Cmd+Shift+Space to cycle",
                                    help.active_signature + 1,
                                    help.signatures.len()
                                ))
                                .small()
                                .color(egui::Color32::from_rgb(140, 140, 150)),
                            );
                        }

                        if let Some(doc) = &signature.documentation {
                            if !doc.trim().is_empty() {
                                ui.separator();
                                ui.label(
                                    egui::RichText::new(doc)
                                        .small()
                                        .color(egui::Color32::from_rgb(200, 200, 210)),
                                );
                            }
                        }
                    });
            });
    }

    /// Render LSP completion popup (VS Code style)
    pub(crate) fn render_lsp_completions(&mut self, ctx: &egui::Context) {
        // Get the current word being typed (for filtering)
        let current_word = if let Some(tab) = self.editor_tabs.get(self.active_tab_idx) {
            let text = tab.text_cache.clone();
            let cursor = tab.cursor_col + tab.buffer.line_to_char(tab.cursor_line);
            let chars: Vec<char> = text.chars().collect();
            let mut start = cursor;
            while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
                start -= 1;
            }
            chars[start..cursor]
                .iter()
                .collect::<String>()
                .to_lowercase()
        } else {
            String::new()
        };

        // Filter completions: must START with current word (not just contain)
        let filtered: Vec<_> = self
            .lsp_completions
            .iter()
            .filter(|item| {
                if current_word.is_empty() {
                    true
                } else {
                    item.label.to_lowercase().starts_with(&current_word)
                }
            })
            .collect();

        // No matches — dismiss
        if filtered.is_empty() {
            self.lsp_show_completions = false;
            self.lsp_completions.clear();
            return;
        }

        // Keyboard: ↑↓ to navigate, Enter/Tab to accept, Esc to dismiss
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.lsp_show_completions = false;
            self.lsp_completions.clear();
            return;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
            self.lsp_completion_index =
                (self.lsp_completion_index + 1).min(filtered.len().saturating_sub(1));
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
            self.lsp_completion_index = self.lsp_completion_index.saturating_sub(1);
        }
        // Clamp index
        if self.lsp_completion_index >= filtered.len() {
            self.lsp_completion_index = 0;
        }

        let mut selected_item: Option<String> = None;

        // Accept on Tab/Enter. The actual key was consumed in `editor.rs`
        // *before* the `TextEdit` widget rendered so it never reached the
        // editor as a newline; this flag is the signal that happened.
        let accepted = std::mem::take(&mut self.lsp_completion_accept_pending);
        if accepted {
            if let Some(item) = filtered.get(self.lsp_completion_index) {
                selected_item = Some(item.insert_text.clone().unwrap_or(item.label.clone()));
            }
        }

        // Position popup below cursor
        let popup_pos = if let Some(tab) = self.editor_tabs.get(self.active_tab_idx) {
            // Approximate: gutter(64) + sidebar(280) + char_width(7.8) * col
            let x = 64.0 + 280.0 + (tab.cursor_col as f32 * 7.8);
            // header(32) + line_height(19.5) * (visible_line + 1)
            let y = 32.0 + ((tab.cursor_line as f32 + 1.0) * 19.5).min(500.0);
            egui::pos2(x.min(600.0), y)
        } else {
            egui::pos2(350.0, 150.0)
        };

        // Click outside to dismiss. Use the actual popup rect (not a stale
        // hard-coded one) so clicks INSIDE the popup are correctly treated
        // as item-clicks rather than as outside-clicks that dismiss it.
        let popup_rect = egui::Rect::from_min_size(
            popup_pos,
            egui::vec2(400.0, (filtered.len().min(10) as f32) * 20.0 + 4.0),
        );
        if ctx.input(|i| i.pointer.any_pressed()) {
            let click_pos = ctx.input(|i| i.pointer.interact_pos());
            if let Some(pos) = click_pos {
                if !popup_rect.contains(pos) {
                    self.lsp_show_completions = false;
                    self.lsp_completions.clear();
                    return;
                }
            }
        }

        if selected_item.is_none() {
            let bg = egui::Color32::from_rgb(30, 30, 30);
            let border = egui::Color32::from_rgb(69, 69, 69);
            let sel_bg = egui::Color32::from_rgb(4, 57, 94);
            let text_color = egui::Color32::from_rgb(212, 212, 212);
            let detail_color = egui::Color32::from_rgb(110, 110, 110);
            let max_items = 10;

            egui::Area::new(egui::Id::new("lsp_completions"))
                .order(egui::Order::Foreground)
                .fixed_pos(popup_pos)
                .show(ctx, |ui| {
                    egui::Frame::NONE
                        .fill(bg)
                        .stroke(egui::Stroke::new(1.0, border))
                        .inner_margin(egui::Margin::same(0))
                        .show(ui, |ui| {
                            ui.set_width(400.0);

                            for (idx, item) in filtered.iter().take(max_items).enumerate() {
                                let (rect, response) = ui.allocate_exact_size(
                                    egui::vec2(400.0, 20.0),
                                    egui::Sense::click(),
                                );

                                // Highlight selected
                                if idx == self.lsp_completion_index || response.hovered() {
                                    ui.painter().rect_filled(rect, 0.0, sel_bg);
                                }

                                let (icon, icon_color) = match item.kind.as_str() {
                                    "Function" | "Method" => {
                                        ("f", egui::Color32::from_rgb(220, 170, 250))
                                    }
                                    "Variable" => ("v", egui::Color32::from_rgb(120, 180, 240)),
                                    "Field" => ("f", egui::Color32::from_rgb(120, 180, 240)),
                                    "Struct" | "Class" => {
                                        ("S", egui::Color32::from_rgb(240, 200, 80))
                                    }
                                    "Module" => ("M", egui::Color32::from_rgb(200, 200, 200)),
                                    "Keyword" => ("k", egui::Color32::from_rgb(86, 156, 214)),
                                    "Enum" | "EnumMember" => {
                                        ("E", egui::Color32::from_rgb(240, 200, 80))
                                    }
                                    "Constant" => ("C", egui::Color32::from_rgb(100, 180, 255)),
                                    "Trait" | "TypeParameter" => {
                                        ("T", egui::Color32::from_rgb(78, 201, 176))
                                    }
                                    _ => ("a", egui::Color32::from_rgb(150, 150, 150)),
                                };

                                ui.painter().text(
                                    egui::pos2(rect.left() + 10.0, rect.center().y),
                                    egui::Align2::LEFT_CENTER,
                                    icon,
                                    egui::FontId::monospace(12.0),
                                    icon_color,
                                );

                                ui.painter().text(
                                    egui::pos2(rect.left() + 28.0, rect.center().y),
                                    egui::Align2::LEFT_CENTER,
                                    &item.label,
                                    egui::FontId::monospace(12.0),
                                    text_color,
                                );

                                if let Some(ref detail) = item.detail {
                                    let short = if detail.len() > 30 {
                                        format!("{}...", &detail[..27])
                                    } else {
                                        detail.clone()
                                    };
                                    ui.painter().text(
                                        egui::pos2(rect.right() - 6.0, rect.center().y),
                                        egui::Align2::RIGHT_CENTER,
                                        &short,
                                        egui::FontId::monospace(10.0),
                                        detail_color,
                                    );
                                }

                                if response.clicked() {
                                    selected_item = Some(
                                        item.insert_text.clone().unwrap_or(item.label.clone()),
                                    );
                                }
                            }
                        });
                });
        }

        // Insert selected completion
        if let Some(ref insert_text) = selected_item {
            self.lsp_show_completions = false;
            self.lsp_completions.clear();
            self.lsp_completion_index = 0;

            if let Some(tab) = self.editor_tabs.get_mut(self.active_tab_idx) {
                let text = tab.buffer.to_string();
                let cursor = tab.cursor_col + tab.buffer.line_to_char(tab.cursor_line);
                let chars: Vec<char> = text.chars().collect();
                let mut word_start = cursor;
                while word_start > 0
                    && (chars[word_start - 1].is_alphanumeric() || chars[word_start - 1] == '_')
                {
                    word_start -= 1;
                }
                let mut new_text = String::new();
                new_text.push_str(&text[..word_start]);
                new_text.push_str(insert_text);
                new_text.push_str(&text[cursor..]);
                tab.buffer = crate::buffer::TextBuffer::from_str(&new_text);
                tab.text_cache = new_text.clone();
                tab.text_cache_version = tab.buffer.version();
                tab.is_dirty = true;
                let new_cursor = word_start + insert_text.len();
                tab.cursor_line = new_text[..new_cursor].matches('\n').count();
                tab.cursor_col = new_cursor
                    - new_text[..new_cursor]
                        .rfind('\n')
                        .map(|p| p + 1)
                        .unwrap_or(0);
            }
        }
    }

    /// Request diagnostics for the current file
    #[allow(dead_code)]
    pub(crate) fn request_diagnostics(&mut self) {
        let tab = match self.editor_tabs.get(self.active_tab_idx) {
            Some(t) => t,
            None => return,
        };

        let file_path = tab.file_path.clone();

        let client = match &self.lsp_native_client {
            Some(c) => std::sync::Arc::clone(c),
            None => {
                tracing::warn!("⚠️ LSP client not initialized");
                return;
            }
        };

        let tx = match &self.lsp_response_tx {
            Some(t) => t.clone(),
            None => return,
        };

        let runtime = std::sync::Arc::clone(&self.lsp_runtime);

        runtime.spawn(async move {
            tracing::info!("🚀 Requesting LSP diagnostics for {}", file_path);

            let lang = match crate::native::lsp_native::detect_server_language(&file_path) {
                Some(l) => l,
                None => {
                    tracing::debug!("No LSP server for file: {}", file_path);
                    return;
                }
            };
            match client.get_diagnostics(lang, file_path.clone()).await {
                Ok(diagnostics) => {
                    tracing::info!("📋 LSP returned {} diagnostics", diagnostics.len());

                    let lsp_diagnostics: Vec<LspDiagnostic> = diagnostics
                        .into_iter()
                        .map(|diag| {
                            use lsp_types::DiagnosticSeverity as LspSeverity;

                            LspDiagnostic {
                                line: diag.range.start.line as usize,
                                column: diag.range.start.character as usize,
                                severity: match diag.severity {
                                    Some(LspSeverity::ERROR) => DiagnosticSeverity::Error,
                                    Some(LspSeverity::WARNING) => DiagnosticSeverity::Warning,
                                    Some(LspSeverity::INFORMATION) => {
                                        DiagnosticSeverity::Information
                                    }
                                    Some(LspSeverity::HINT) => DiagnosticSeverity::Hint,
                                    _ => DiagnosticSeverity::Error,
                                },
                                message: diag.message,
                                source: diag.source,
                            }
                        })
                        .collect();

                    if let Err(e) = tx.send(LspResponse::Diagnostics(lsp_diagnostics)) {
                        tracing::error!("❌ Failed to send LSP diagnostics: {}", e);
                    }
                }
                Err(e) => {
                    tracing::warn!("⚠️ LSP get_diagnostics failed: {}", e);
                }
            }
        });
    }

    /// Render diagnostics in the editor (gutter icons and inline messages)
    #[allow(dead_code)]
    pub(crate) fn render_diagnostics_in_editor(&self, ui: &mut egui::Ui, line_number: usize) {
        let diagnostics_on_line: Vec<&LspDiagnostic> = self
            .lsp_diagnostics
            .iter()
            .filter(|d| d.line == line_number)
            .collect();

        if diagnostics_on_line.is_empty() {
            return;
        }

        for diagnostic in &diagnostics_on_line {
            let (icon, color) = match diagnostic.severity {
                DiagnosticSeverity::Error => ("❌", egui::Color32::from_rgb(255, 80, 80)),
                DiagnosticSeverity::Warning => ("⚠️", egui::Color32::from_rgb(255, 200, 100)),
                DiagnosticSeverity::Information => ("ℹ️", egui::Color32::from_rgb(100, 150, 255)),
                DiagnosticSeverity::Hint => ("💡", egui::Color32::from_rgb(150, 150, 150)),
            };

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(icon).color(color));
                ui.label(egui::RichText::new(&diagnostic.message).color(color));
            });
        }
    }

    /// Render diagnostics panel at the bottom of the editor
    pub(crate) fn render_diagnostics_panel(&mut self, ctx: &egui::Context) {
        let mut clear_diags = false;

        // Filter out diagnostics for non-Rust files (TOML, etc.)
        let rs_diagnostics = super::utils::filter_rust_diagnostics(&self.lsp_diagnostics);

        if rs_diagnostics.is_empty() {
            return;
        }

        egui::TopBottomPanel::bottom("diagnostics_panel")
            .resizable(true)
            .default_height(80.0)
            .max_height(120.0)
            .min_height(40.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("Problems ({})", rs_diagnostics.len()))
                            .size(11.0)
                            .strong(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("×")
                                        .size(14.0)
                                        .color(egui::Color32::from_rgb(180, 180, 180)),
                                )
                                .frame(false),
                            )
                            .clicked()
                        {
                            clear_diags = true;
                        }
                    });
                });
                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 1.0;

                    for diagnostic in &rs_diagnostics {
                        let color = match diagnostic.severity {
                            DiagnosticSeverity::Error => egui::Color32::from_rgb(255, 80, 80),
                            DiagnosticSeverity::Warning => egui::Color32::from_rgb(255, 200, 100),
                            DiagnosticSeverity::Information => {
                                egui::Color32::from_rgb(100, 150, 255)
                            }
                            DiagnosticSeverity::Hint => egui::Color32::from_rgb(150, 150, 150),
                        };

                        let file_name = diagnostic
                            .source
                            .as_ref()
                            .and_then(|s| s.split('/').last())
                            .unwrap_or("unknown");

                        let loc = format!(
                            "{}:{}:{}",
                            file_name,
                            diagnostic.line + 1,
                            diagnostic.column + 1
                        );

                        // Truncate message to avoid overlap
                        let msg = if diagnostic.message.len() > 80 {
                            format!("{}...", &diagnostic.message[..77])
                        } else {
                            diagnostic.message.clone()
                        };

                        // Build as LayoutJob for clean rendering
                        let mut job = egui::text::LayoutJob::default();
                        let font = egui::FontId::monospace(11.5);
                        job.append(
                            &loc,
                            0.0,
                            egui::TextFormat {
                                font_id: font.clone(),
                                color: egui::Color32::from_rgb(86, 156, 214),
                                ..Default::default()
                            },
                        );
                        job.append(
                            "  ",
                            0.0,
                            egui::TextFormat {
                                font_id: font.clone(),
                                color: egui::Color32::TRANSPARENT,
                                ..Default::default()
                            },
                        );
                        job.append(
                            &msg,
                            0.0,
                            egui::TextFormat {
                                font_id: font.clone(),
                                color,
                                ..Default::default()
                            },
                        );
                        job.wrap.max_width = ui.available_width();
                        job.wrap.max_rows = 1;

                        let response = ui.add(egui::Label::new(job).sense(egui::Sense::click()));

                        if response.clicked() {
                            if let Some(tab) = self.editor_tabs.get_mut(self.active_tab_idx) {
                                tab.cursor_line = diagnostic.line;
                                tab.cursor_col = diagnostic.column;
                            }
                        }
                    }
                });
            });

        if clear_diags {
            self.lsp_diagnostics.clear();
        }
    }

    /// Request hover information
    #[allow(dead_code)]
    pub(crate) fn request_hover(&mut self, line: usize, column: usize) {
        let tab = match self.editor_tabs.get(self.active_tab_idx) {
            Some(t) => t,
            None => return,
        };

        let file_path = tab.file_path.clone();

        let client = match &self.lsp_native_client {
            Some(c) => std::sync::Arc::clone(c),
            None => {
                tracing::warn!("⚠️ LSP client not initialized");
                return;
            }
        };

        let tx = match &self.lsp_response_tx {
            Some(t) => t.clone(),
            None => return,
        };

        let runtime = std::sync::Arc::clone(&self.lsp_runtime);

        runtime.spawn(async move {
            tracing::info!("🚀 Requesting LSP hover at {}:{}", line, column);

            let lang = match crate::native::lsp_native::detect_server_language(&file_path) {
                Some(l) => l,
                None => {
                    tracing::debug!("No LSP server for file: {}", file_path);
                    return;
                }
            };
            match client
                .get_hover(lang, file_path.clone(), line as u32, column as u32)
                .await
            {
                Ok(hover_opt) => {
                    if let Some(hover) = hover_opt {
                        tracing::info!("💡 LSP returned hover info");

                        use lsp_types::{HoverContents, MarkedString};
                        let contents_string = match hover.contents {
                            HoverContents::Scalar(marked) => match marked {
                                MarkedString::String(s) => s,
                                MarkedString::LanguageString(ls) => {
                                    format!("```{}\n{}\n```", ls.language, ls.value)
                                }
                            },
                            HoverContents::Array(arr) => arr
                                .into_iter()
                                .map(|marked| match marked {
                                    MarkedString::String(s) => s,
                                    MarkedString::LanguageString(ls) => {
                                        format!("```{}\n{}\n```", ls.language, ls.value)
                                    }
                                })
                                .collect::<Vec<_>>()
                                .join("\n\n"),
                            HoverContents::Markup(markup) => markup.value,
                        };

                        let lsp_hover = LspHoverInfo {
                            contents: contents_string,
                            line,
                            column,
                        };

                        if let Err(e) = tx.send(LspResponse::Hover(Some(lsp_hover))) {
                            tracing::error!("❌ Failed to send LSP hover: {}", e);
                        }
                    } else {
                        tracing::info!("ℹ️ No hover info available");
                        let _ = tx.send(LspResponse::Hover(None));
                    }
                }
                Err(e) => {
                    tracing::warn!("⚠️ LSP get_hover failed: {}", e);
                }
            }
        });
    }

    /// Check if mouse is hovering over text
    #[allow(dead_code)]
    pub(crate) fn check_hover_in_editor(&mut self, _response: &egui::Response) {
        // Disabled
    }

    /// Request definition locations
    #[allow(dead_code)]
    pub(crate) fn request_definition(&mut self) {
        tracing::debug!("LSP go-to-definition disabled (no Tokio runtime)");
    }

    /// Handle keyboard shortcut for Go to Definition (F12)
    pub(crate) fn handle_goto_definition_shortcut(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            if i.key_pressed(egui::Key::F12) && !i.modifiers.shift {
                self.trigger_goto_definition_at_cursor();
            }
        });
    }

    /// Handle keyboard shortcut for Find References (Shift+F12)
    pub(crate) fn handle_find_references_shortcut(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            if i.modifiers.shift && i.key_pressed(egui::Key::F12) {
                self.trigger_find_references_at_cursor();
            }
        });
    }

    /// Trigger find references at current cursor position
    pub(crate) fn trigger_find_references_at_cursor(&mut self) {
        if self.editor_tabs.is_empty() {
            return;
        }

        let tab = match self.editor_tabs.get(self.active_tab_idx) {
            Some(t) => t,
            None => return,
        };

        let file_path = tab.file_path.clone();
        let cursor_line = tab.cursor_line;
        let utf8_cursor_col = tab.cursor_col;

        let utf16_cursor_col = {
            let text = tab.buffer.to_string();
            let lines: Vec<&str> = text.lines().collect();
            if cursor_line < lines.len() {
                utf8_offset_to_utf16(lines[cursor_line], utf8_cursor_col)
            } else {
                utf8_cursor_col
            }
        };

        tracing::info!(
            "🔍 Triggering find references at {}:{}:{} (UTF-16)",
            file_path.split('/').last().unwrap_or(&file_path),
            cursor_line + 1,
            utf16_cursor_col + 1
        );

        self.spawn_find_references_request(file_path, cursor_line, utf16_cursor_col, true);
    }

    /// Trigger go-to-definition at current cursor position
    pub(crate) fn trigger_goto_definition_at_cursor(&mut self) {
        let tab = match self.editor_tabs.get(self.active_tab_idx) {
            Some(t) => t,
            None => return,
        };

        let text = tab.buffer.to_string();
        let cursor_line = tab.cursor_line;
        let cursor_col = tab.cursor_col;

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

        self.handle_go_to_definition(&text, cursor_pos);
    }

    /// Render LSP hover tooltip
    pub(crate) fn render_lsp_hover(&mut self, ctx: &egui::Context) {
        if let Some(hover_info) = self.lsp_hover_info.clone() {
            let mut close_hover = false;

            egui::Window::new("💡 Hover Information")
                .collapsible(false)
                .resizable(false)
                .default_pos([400.0, 300.0])
                .show(ctx, |ui| {
                    ui.label(&hover_info.contents);
                    ui.separator();
                    if ui.button("Close (Esc)").clicked() {
                        close_hover = true;
                    }
                });

            if close_hover {
                self.lsp_show_hover = false;
                self.lsp_hover_info = None;
            }
        }
    }

    /// Render definition picker window (for multiple definitions)
    pub(crate) fn render_definition_picker(&mut self, ctx: &egui::Context) {
        let locations = self.definition_picker_locations.clone();
        let mut selected_location: Option<LspLocation> = None;
        let mut close_picker = false;

        egui::Window::new("📋 Choose Definition")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .default_size([600.0, 400.0])
            .show(ctx, |ui| {
                ui.label(format!("{} definitions found:", locations.len()));
                ui.separator();

                egui::ScrollArea::vertical()
                    .max_height(400.0)
                    .show(ui, |ui| {
                        for (idx, loc) in locations.iter().enumerate() {
                            let file_name =
                                loc.file_path.split('/').last().unwrap_or(&loc.file_path);
                            let label = format!(
                                "{}  {}:{}  ({})",
                                idx + 1,
                                file_name,
                                loc.line + 1,
                                loc.file_path
                            );

                            if ui.button(&label).clicked() {
                                selected_location = Some(loc.clone());
                                close_picker = true;
                            }
                        }
                    });

                ui.separator();
                if ui.button("❌ Cancel").clicked() {
                    close_picker = true;
                }
            });

        if let Some(location) = selected_location {
            self.navigate_to_location(&location);
            self.show_definition_picker = false;
            self.definition_picker_locations.clear();
        } else if close_picker {
            self.show_definition_picker = false;
            self.definition_picker_locations.clear();
        }
    }

    /// Render References panel
    pub(crate) fn render_references_panel(&mut self, ctx: &egui::Context) {
        let references = self.lsp_references.clone();
        let mut selected_location: Option<LspLocation> = None;
        let mut close_panel = false;

        egui::Window::new("🔍 References")
            .collapsible(false)
            .resizable(true)
            .anchor(egui::Align2::RIGHT_TOP, egui::Vec2::new(-10.0, 50.0))
            .default_size([600.0, 400.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(format!("{} references found", references.len()));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("❌").clicked() {
                            close_panel = true;
                        }
                    });
                });
                ui.separator();

                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        for (_idx, loc) in references.iter().enumerate() {
                            let file_name =
                                loc.file_path.split('/').last().unwrap_or(&loc.file_path);
                            let location_text =
                                format!("{}:{}:{}", file_name, loc.line + 1, loc.column + 1);
                            if ui.link(&location_text).clicked() {
                                selected_location = Some(loc.clone());
                            }
                        }
                    });
            });

        if let Some(location) = selected_location {
            self.navigate_to_location(&location);
        } else if close_panel {
            self.show_references_panel = false;
            self.lsp_references.clear();
        }
    }

    /// Handle Cmd+Click go-to-definition (Hybrid: LSP priority + regex fallback)
    pub(crate) fn handle_go_to_definition(&mut self, text: &str, cursor_pos: usize) {
        let word = self.extract_word_at_position(text, cursor_pos);
        if word.is_empty() {
            tracing::debug!("No word found at cursor position");
            return;
        }

        tracing::info!("🔍 Looking for definition of: '{}'", word);

        let current_file = match self.editor_tabs.get(self.active_tab_idx) {
            Some(tab) => tab.file_path.clone(),
            None => return,
        };

        let (line, utf8_column) = calculate_line_column(text, cursor_pos);

        if self.lsp_connected && self.lsp_native_client.is_some() {
            let utf16_column = {
                let lines: Vec<&str> = text.lines().collect();
                if line < lines.len() {
                    utf8_offset_to_utf16(lines[line], utf8_column)
                } else {
                    utf8_column
                }
            };

            tracing::info!(
                "🚀 Requesting LSP goto_definition for '{}' at {}:{} (UTF-8: {}, UTF-16: {})",
                word,
                line,
                utf16_column,
                utf8_column,
                utf16_column
            );
            self.spawn_goto_definition_request(current_file, line, utf16_column);

            self.pending_goto_definition = Some(PendingGotoDefinition {
                word: word.clone(),
                original_text: text.to_string(),
            });

            return;
        }

        tracing::info!("📝 LSP unavailable, using local regex search");
        self.fallback_goto_definition(text, &word);
    }

    /// Regex-based local search (fallback when LSP unavailable)
    pub(crate) fn fallback_goto_definition(&mut self, text: &str, word: &str) {
        let patterns = vec![
            format!(r"fn\s+{}\s*\(", word),
            format!(r"pub\s+fn\s+{}\s*\(", word),
            format!(r"struct\s+{}\s*[{{<]", word),
            format!(r"pub\s+struct\s+{}\s*[{{<]", word),
            format!(r"enum\s+{}\s*[{{<]", word),
            format!(r"pub\s+enum\s+{}\s*[{{<]", word),
            format!(r"trait\s+{}\s*[{{<]", word),
            format!(r"pub\s+trait\s+{}\s*[{{<]", word),
            format!(r"type\s+{}\s*=", word),
            format!(r"const\s+{}\s*:", word),
            format!(r"static\s+{}\s*:", word),
            format!(r"impl\s+{}\s*[{{<]", word),
            format!(r"impl.*for\s+{}\s*[{{<]", word),
        ];

        for (line_idx, line) in text.lines().enumerate() {
            for pattern in &patterns {
                if let Ok(regex) = regex::Regex::new(pattern) {
                    if regex.is_match(line) {
                        tracing::info!(
                            "✅ Found definition at line {}: {}",
                            line_idx + 1,
                            line.trim()
                        );

                        if let Some(tab) = self.editor_tabs.get_mut(self.active_tab_idx) {
                            tab.cursor_line = line_idx;
                            tab.cursor_col = 0;
                            tab.pending_cursor_jump = Some((line_idx, 0));
                            tracing::info!("⏭️ Scheduled cursor jump to line {}", line_idx);
                        }
                        return;
                    }
                }
            }
        }

        tracing::info!("🔍 Searching in project for '{}'", word);
        self.search_definition_in_project(word);
    }

    /// Extract word at cursor position
    pub(crate) fn extract_word_at_position(&self, text: &str, pos: usize) -> String {
        if pos > text.len() {
            return String::new();
        }

        let chars: Vec<char> = text.chars().collect();
        if pos >= chars.len() {
            return String::new();
        }

        let mut start = pos;
        while start > 0 {
            let ch = chars[start - 1];
            if !ch.is_alphanumeric() && ch != '_' {
                break;
            }
            start -= 1;
        }

        let mut end = pos;
        while end < chars.len() {
            let ch = chars[end];
            if !ch.is_alphanumeric() && ch != '_' {
                break;
            }
            end += 1;
        }

        chars[start..end].iter().collect()
    }

    /// Search for definition across the project
    pub(crate) fn search_definition_in_project(&mut self, word: &str) {
        let search_patterns = vec![
            format!(r"pub fn {}", word),
            format!(r"pub struct {}", word),
            format!(r"pub enum {}", word),
            format!(r"pub trait {}", word),
            format!(r"pub type {}", word),
            format!(r"pub const {}", word),
            format!(r"fn {}", word),
            format!(r"struct {}", word),
            format!(r"enum {}", word),
            format!(r"trait {}", word),
            format!(r"type {}", word),
            format!(r"const {}", word),
        ];

        for pattern in search_patterns {
            match native::search::search_in_files(&self.root_path, &pattern, false, true) {
                Ok(results) => {
                    if !results.is_empty() {
                        let first_result = &results[0];

                        tracing::info!(
                            "✅ Found definition in {}: line {}",
                            first_result.file_path,
                            first_result.line_number
                        );

                        let file_path = first_result.file_path.clone();
                        let line_number = first_result.line_number - 1;

                        let file_already_open = self
                            .editor_tabs
                            .iter()
                            .position(|tab| tab.file_path == file_path);

                        if let Some(tab_idx) = file_already_open {
                            self.active_tab_idx = tab_idx;
                        } else {
                            self.open_file_from_path(&file_path);
                        }

                        if let Some(tab) = self.editor_tabs.get_mut(self.active_tab_idx) {
                            tab.cursor_line = line_number;
                            tab.cursor_col = 0;
                            tab.pending_cursor_jump = Some((line_number, 0));
                            tracing::info!(
                                "⏭️ Scheduled cursor jump to line {} in {}",
                                line_number,
                                file_path
                            );
                        }

                        return;
                    }
                }
                Err(e) => {
                    tracing::error!("❌ Search error: {}", e);
                }
            }
        }

        tracing::warn!("⚠️ Definition not found for '{}'", word);
    }

    /// Spawn LSP goto_definition request asynchronously
    pub(crate) fn spawn_goto_definition_request(
        &self,
        file_path: String,
        line: usize,
        column: usize,
    ) {
        let client = match &self.lsp_native_client {
            Some(c) => std::sync::Arc::clone(c),
            None => {
                tracing::warn!("⚠️ LSP client not initialized");
                return;
            }
        };

        let tx = match &self.lsp_response_tx {
            Some(t) => t.clone(),
            None => return,
        };

        let runtime = std::sync::Arc::clone(&self.lsp_runtime);

        runtime.spawn(async move {
            tracing::info!("🚀 Requesting LSP goto_definition");
            tracing::info!("   File: {}", file_path);
            tracing::info!("   Position: line={}, column={}", line, column);

            let lang = match crate::native::lsp_native::detect_server_language(&file_path) {
                Some(l) => l,
                None => {
                    tracing::debug!("No LSP server for file: {}", file_path);
                    return;
                }
            };
            match client
                .goto_definition(lang, file_path.clone(), line as u32, column as u32)
                .await
            {
                Ok(locations) => {
                    tracing::info!("📍 LSP returned {} locations", locations.len());
                    for (i, loc) in locations.iter().enumerate() {
                        tracing::info!("   Location {}: {}", i + 1, loc.uri);
                    }

                    let lsp_locations: Vec<LspLocation> = locations
                        .into_iter()
                        .filter_map(parse_lsp_location)
                        .collect();

                    if let Err(e) = tx.send(LspResponse::Definition(lsp_locations)) {
                        tracing::error!("❌ Failed to send LSP response: {}", e);
                    }
                }
                Err(e) => {
                    tracing::warn!("⚠️ LSP goto_definition failed: {} (will use fallback)", e);
                    let _ = tx.send(LspResponse::Definition(vec![]));
                }
            }
        });
    }

    /// Navigate to a specific location (file + line + column)
    pub(crate) fn navigate_to_location(&mut self, location: &LspLocation) {
        tracing::info!("📍 Navigating to location:");
        tracing::info!("   File: {}", location.file_path);
        tracing::info!("   Line: {}, Column: {}", location.line, location.column);

        let is_stdlib =
            location.file_path.contains("/.rustup/") || location.file_path.contains("\\.rustup\\");

        if is_stdlib {
            tracing::info!("📖 Detected standard library file");
        }

        let file_already_open = self
            .editor_tabs
            .iter()
            .position(|tab| tab.file_path == location.file_path);

        if let Some(tab_idx) = file_already_open {
            self.active_tab_idx = tab_idx;
        } else {
            self.open_file_from_path(&location.file_path);

            if is_stdlib {
                if let Some(tab) = self.editor_tabs.last_mut() {
                    tab.is_readonly = true;
                    tracing::info!("📖 Opened as read-only (stdlib)");
                }
            }
        }

        if let Some(tab) = self.editor_tabs.get_mut(self.active_tab_idx) {
            let utf8_column = {
                let text = tab.buffer.to_string();
                let lines: Vec<&str> = text.lines().collect();
                if location.line < lines.len() {
                    let line_text = lines[location.line];
                    utf16_offset_to_utf8(line_text, location.column)
                } else {
                    location.column
                }
            };

            tab.cursor_line = location.line;
            tab.cursor_col = utf8_column;
            tab.pending_cursor_jump = Some((location.line, utf8_column));
            tracing::info!(
                "⏭️ Scheduled cursor jump to line {} col {} (UTF-16: {}, UTF-8: {})",
                location.line,
                utf8_column,
                location.column,
                utf8_column
            );
        }

        self.status_message = format!(
            "✅ Jumped to {}",
            location.file_path.split('/').last().unwrap_or("")
        );
        self.status_message_timestamp = Some(std::time::Instant::now());
    }

    /// Spawn LSP find_references request asynchronously
    pub(crate) fn spawn_find_references_request(
        &self,
        file_path: String,
        line: usize,
        column: usize,
        include_declaration: bool,
    ) {
        let client = match &self.lsp_native_client {
            Some(c) => std::sync::Arc::clone(c),
            None => {
                tracing::warn!("⚠️ LSP client not initialized");
                return;
            }
        };

        let tx = match &self.lsp_response_tx {
            Some(t) => t.clone(),
            None => return,
        };

        let runtime = std::sync::Arc::clone(&self.lsp_runtime);

        runtime.spawn(async move {
            tracing::info!("🔍 Requesting LSP find_references");
            tracing::info!("   File: {}", file_path);
            tracing::info!(
                "   Position: line={}, column={}, include_decl={}",
                line,
                column,
                include_declaration
            );

            let lang = match crate::native::lsp_native::detect_server_language(&file_path) {
                Some(l) => l,
                None => {
                    tracing::debug!("No LSP server for file: {}", file_path);
                    return;
                }
            };
            match client
                .find_references(
                    lang,
                    file_path.clone(),
                    line as u32,
                    column as u32,
                    include_declaration,
                )
                .await
            {
                Ok(locations) => {
                    tracing::info!("📍 LSP returned {} references", locations.len());
                    for (i, loc) in locations.iter().enumerate() {
                        tracing::info!("   Reference {}: {}", i + 1, loc.uri);
                    }

                    let lsp_locations: Vec<LspLocation> = locations
                        .into_iter()
                        .filter_map(parse_lsp_location)
                        .collect();

                    if let Err(e) = tx.send(LspResponse::References(lsp_locations)) {
                        tracing::error!("❌ Failed to send LSP references: {}", e);
                    }
                }
                Err(e) => {
                    tracing::warn!("⚠️ LSP find_references failed: {}", e);
                    let _ = tx.send(LspResponse::References(vec![]));
                }
            }
        });
    }
}
