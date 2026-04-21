//! Editor area rendering and syntax highlighting

use super::peek::render_peek_standalone;
use super::types::{ColorTheme, LspInlayHint};
use super::ui_colors;
use super::BerryCodeApp;
use crate::app::i18n::t;
use crate::syntax::{SyntaxHighlighter, TokenType};

impl BerryCodeApp {
    /// Render Editor area (Phase 3: full implementation with TextEdit)
    pub(crate) fn render_editor_area(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(ui_colors::SIDEBAR_BG) // #191A1C - Match sidebar background
                    .inner_margin(egui::Margin::same(8.0)),
            )
            .show(ctx, |ui| {
                // Save the full panel rect before any layout happens
                let _full_panel_rect = ui.max_rect();

                if self.editor_tabs.is_empty() {
                    // No file open - show placeholder
                    ui.vertical_centered(|ui| {
                        ui.add_space(100.0);
                        ui.heading(t(self.ui_language, "BerryCode Editor"));
                        ui.add_space(16.0);
                        ui.label(t(self.ui_language, "Select a file from the file tree"));
                        ui.add_space(8.0);
                        ui.label(format!(
                            "{} {}",
                            t(self.ui_language, "Project:"),
                            self.root_path
                        ));
                    });
                    return;
                }

                // Tab bar with close buttons
                let mut tab_to_close: Option<usize> = None;

                ui.horizontal(|ui| {
                    // Larger font for tabs
                    ui.style_mut()
                        .text_styles
                        .insert(egui::TextStyle::Body, egui::FontId::proportional(14.0));

                    // Collect tab info first to avoid borrow checker issues
                    let tab_info: Vec<(usize, String, &'static str, egui::Color32)> = self
                        .editor_tabs
                        .iter()
                        .enumerate()
                        .map(|(idx, t)| {
                            let filename = t
                                .file_path
                                .split('/')
                                .last()
                                .unwrap_or(&t.file_path)
                                .to_string();
                            let (icon, color) = Self::get_file_icon_with_color(&filename);
                            (idx, filename, icon, color)
                        })
                        .collect();

                    for (idx, filename, file_icon, icon_color) in tab_info {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 4.0;

                                // Colored icon
                                ui.label(
                                    egui::RichText::new(file_icon)
                                        .color(icon_color)
                                        .family(egui::FontFamily::Name("codicon".into())),
                                );

                                // Tab label (clickable to switch)
                                let filename_text = egui::RichText::new(&filename)
                                    .color(egui::Color32::from_rgb(0xD4, 0xD4, 0xD4));
                                if ui
                                    .selectable_label(idx == self.active_tab_idx, filename_text)
                                    .clicked()
                                {
                                    self.active_tab_idx = idx;
                                }

                                // Close button - Codicon: \u{ea76} = codicon-close
                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new("\u{ea76}")
                                                .family(egui::FontFamily::Name("codicon".into())),
                                        )
                                        .small(),
                                    )
                                    .clicked()
                                {
                                    tab_to_close = Some(idx);
                                }
                            });
                        });
                    }
                });

                // Close tab if requested (after the loop to avoid borrow issues)
                if let Some(close_idx) = tab_to_close {
                    // Save file path before removing the tab for LSP didClose
                    let closed_file_path = self.editor_tabs[close_idx].file_path.clone();

                    self.editor_tabs.remove(close_idx);

                    // Adjust active tab index
                    if self.editor_tabs.is_empty() {
                        self.active_tab_idx = 0;
                    } else if self.active_tab_idx >= self.editor_tabs.len() {
                        self.active_tab_idx = self.editor_tabs.len() - 1;
                    } else if close_idx <= self.active_tab_idx && self.active_tab_idx > 0 {
                        self.active_tab_idx -= 1;
                    }

                    // Notify LSP about the closed file (textDocument/didClose)
                    if let Some(lang) =
                        crate::native::lsp_native::detect_server_language(&closed_file_path)
                    {
                        if let Some(client) = &self.lsp_native_client {
                            let client = client.clone();
                            let runtime = self.lsp_runtime.clone();
                            let path = closed_file_path.clone();
                            let language = lang.to_string();
                            runtime.spawn(async move {
                                let _ = client.close_file(&language, &path).await;
                            });
                        }
                    }

                    tracing::info!("✅ Closed tab at index {}", close_idx);
                }

                // Early return if all tabs are closed
                if self.editor_tabs.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(100.0);
                        ui.heading(t(self.ui_language, "BerryCode Editor"));
                        ui.add_space(16.0);
                        ui.label(t(self.ui_language, "Select a file from the file tree"));
                    });
                    return;
                }

                ui.separator();

                // If the active tab is an image, render the image preview instead of the text editor
                if self.editor_tabs[self.active_tab_idx].is_image {
                    self.render_image_preview(ui, ctx);
                    return;
                }

                // If the active tab is a 3D model, render the model preview
                if self.editor_tabs[self.active_tab_idx].is_model {
                    self.render_model_preview(ui);
                    return;
                }

                // Snapshot data that we need from self before taking &mut tab
                let inlay_hints_snapshot: Vec<LspInlayHint> = if self.inlay_hints_enabled {
                    self.lsp_inlay_hints.clone()
                } else {
                    Vec::new()
                };
                let code_action_line = self.code_action_line;
                let has_code_actions = !self.lsp_code_actions.is_empty();

                // Get active tab (after tab bar to avoid borrowing issues)
                let tab = &mut self.editor_tabs[self.active_tab_idx];

                // Editor content
                let _ = tab.get_text(); // ensure cache is up to date
                let original_text = std::mem::take(&mut tab.text_cache);

                // Apply code folding if any regions are folded
                let (mut text, _fold_mapping) = if tab.folded_regions.is_empty() {
                    (original_text.clone(), Vec::new())
                } else {
                    let (folded, mapping) =
                        super::folding::apply_folding(&original_text, &tab.folded_regions);
                    (folded, mapping)
                };
                let is_folded = !tab.folded_regions.is_empty();
                // Keep original for restoring cache
                let original_for_cache = original_text;

                // Detect language from file extension (syntect uses extension, not language name)
                let extension = if tab.file_path.ends_with(".rs") {
                    "rs"
                } else if tab.file_path.ends_with(".toml") {
                    "toml"
                } else if tab.file_path.ends_with(".md") {
                    "md"
                } else if tab.file_path.ends_with(".js") {
                    "js"
                } else if tab.file_path.ends_with(".ts") {
                    "ts"
                } else if tab.file_path.ends_with(".py") {
                    "py"
                } else if tab.file_path.ends_with(".json") {
                    "json"
                } else if tab.file_path.ends_with(".yaml") || tab.file_path.ends_with(".yml") {
                    "yaml"
                } else {
                    "txt"
                };

                // Set language for syntax highlighter (only log on change)
                let _ = self.syntax_highlighter.set_language(extension);

                // Clone highlighter AFTER setting the language
                let highlighter = self.syntax_highlighter.clone();

                // Copy color theme (to avoid borrowing issues in layouter closure)
                let color_theme = ColorTheme {
                    keyword: self.keyword_color,
                    function: self.function_color,
                    type_: self.type_color,
                    string: self.string_color,
                    number: self.number_color,
                    comment: self.comment_color,
                    doc_comment: self.doc_comment_color,
                    macro_: self.macro_color,
                    attribute: self.attribute_color,
                    constant: self.constant_color,
                    lifetime: self.lifetime_color,
                    namespace: self.namespace_color,
                    variable: self.variable_color,
                    operator: self.operator_color,
                };

                // Read-only warning banner
                let is_readonly = tab.is_readonly;
                if is_readonly {
                    ui.colored_label(
                        egui::Color32::from_rgb(255, 200, 0),
                        "⚠️ This file is read-only (standard library source)",
                    );
                    ui.add_space(4.0);
                }

                // Check for pending cursor jump
                let (cursor_range_to_set, scroll_to_y) =
                    if let Some((jump_line, jump_col)) = tab.pending_cursor_jump {
                        // Calculate character offset from line/column
                        let char_offset = {
                            let mut offset = 0;
                            for (line_idx, line) in text.lines().enumerate() {
                                if line_idx == jump_line {
                                    offset += jump_col.min(line.len());
                                    break;
                                }
                                offset += line.len() + 1; // +1 for newline
                            }
                            offset
                        };

                        // Calculate Y position for scrolling
                        // Approximate line height (will be refined by TextEdit rendering)
                        const APPROX_LINE_HEIGHT: f32 = 19.5; // 13 * 1.5
                        let target_y = jump_line as f32 * APPROX_LINE_HEIGHT;

                        tracing::info!(
                            "📍 Jumping to line {} col {} (char offset: {}, y: {})",
                            jump_line,
                            jump_col,
                            char_offset,
                            target_y
                        );

                        // Create cursor range for both primary and secondary cursors at the same position
                        (
                            Some(egui::text::CCursorRange::one(egui::text::CCursor::new(
                                char_offset,
                            ))),
                            Some(target_y),
                        )
                    } else {
                        (None, None)
                    };

                // Dark background for scroll area
                ui.style_mut().visuals.extreme_bg_color = ui_colors::SIDEBAR_BG;
                ui.style_mut().visuals.widgets.noninteractive.bg_fill = ui_colors::SIDEBAR_BG;
                ui.style_mut().visuals.window_fill = ui_colors::SIDEBAR_BG;
                ui.style_mut().visuals.panel_fill = ui_colors::SIDEBAR_BG;
                // Also override the faint_bg_color used for scroll bar track
                ui.style_mut().visuals.faint_bg_color = ui_colors::SIDEBAR_BG;

                let scroll_area = egui::ScrollArea::vertical().auto_shrink([false; 2]);

                let scroll_output = scroll_area.show(ui, |ui| {
                    ui.style_mut().visuals.extreme_bg_color = ui_colors::SIDEBAR_BG;
                    ui.style_mut().visuals.widgets.noninteractive.bg_fill = ui_colors::SIDEBAR_BG;

                    // CRITICAL: Disable text color override to allow syntax highlighting
                    ui.style_mut().visuals.override_text_color = None;

                    // Gutter = 64px left margin in TextEdit
                    let gutter_width = 64.0_f32;

                    let output = egui::TextEdit::multiline(&mut text)
                        .code_editor()
                        .desired_width(f32::INFINITY)
                        .lock_focus(true)
                        .margin(egui::Margin {
                            left: gutter_width,
                            right: 4.0,
                            top: 0.0,
                            bottom: 0.0,
                        })
                        .interactive(!is_readonly)
                        .layouter(&mut |ui, text, _wrap_width| {
                            // For large files, skip syntax highlighting to keep UI responsive
                            let job = if text.len() > 200_000 {
                                let mut job = egui::text::LayoutJob::single_section(
                                    text.to_string(),
                                    egui::TextFormat {
                                        font_id: egui::FontId::monospace(13.0),
                                        color: ui_colors::TEXT_DEFAULT,
                                        ..Default::default()
                                    },
                                );
                                job.wrap.max_width = f32::INFINITY;
                                job
                            } else {
                                let mut job = Self::syntax_highlight_layouter(
                                    ui,
                                    text,
                                    &highlighter,
                                    &color_theme,
                                );
                                job.wrap.max_width = f32::INFINITY;
                                job
                            };
                            ui.fonts(|f| f.layout_job(job))
                        })
                        .show(ui);

                    // Auto-close brackets: if a bracket was just typed, insert closing pair
                    if !is_readonly && output.response.changed() {
                        if let Some(cr) = output.cursor_range {
                            let cursor_pos = cr.primary.ccursor.index;
                            if cursor_pos > 0 {
                                let chars: Vec<char> = text.chars().collect();
                                let just_typed = chars.get(cursor_pos - 1).copied();
                                let closing = match just_typed {
                                    Some('(') => Some(')'),
                                    Some('{') => Some('}'),
                                    Some('[') => Some(']'),
                                    Some('"') => {
                                        // Don't auto-close if it's a closing quote
                                        let count = chars[..cursor_pos]
                                            .iter()
                                            .filter(|&&c| c == '"')
                                            .count();
                                        if count % 2 == 1 {
                                            Some('"')
                                        } else {
                                            None
                                        }
                                    }
                                    Some('\'') => {
                                        let count = chars[..cursor_pos]
                                            .iter()
                                            .filter(|&&c| c == '\'')
                                            .count();
                                        if count % 2 == 1 {
                                            Some('\'')
                                        } else {
                                            None
                                        }
                                    }
                                    _ => None,
                                };
                                if let Some(close_char) = closing {
                                    // Insert closing bracket at cursor position
                                    text.insert(cursor_pos, close_char);
                                    // Don't move cursor - it should stay between the brackets
                                }
                            }
                        }
                    }

                    // Auto-indent: when a newline was just inserted, copy indentation from previous line
                    if !is_readonly && output.response.changed() {
                        if let Some(cr) = output.cursor_range {
                            let cursor_pos = cr.primary.ccursor.index;
                            if cursor_pos > 0 {
                                let chars: Vec<char> = text.chars().collect();
                                if chars.get(cursor_pos - 1) == Some(&'\n') {
                                    // Find the previous line's indentation
                                    let mut line_start =
                                        if cursor_pos >= 2 { cursor_pos - 2 } else { 0 };
                                    while line_start > 0 && chars[line_start] != '\n' {
                                        line_start -= 1;
                                    }
                                    if line_start > 0 || chars[line_start] == '\n' {
                                        if chars[line_start] == '\n' {
                                            line_start += 1;
                                        }
                                    }

                                    let mut indent = String::new();
                                    for i in line_start..cursor_pos.saturating_sub(1) {
                                        if chars[i] == ' ' || chars[i] == '\t' {
                                            indent.push(chars[i]);
                                        } else {
                                            break;
                                        }
                                    }

                                    // If previous line ends with '{', add extra indent
                                    let prev_line_trimmed_end = chars
                                        [line_start..cursor_pos.saturating_sub(1)]
                                        .iter()
                                        .rev()
                                        .find(|c| !c.is_whitespace());
                                    if prev_line_trimmed_end == Some(&'{') {
                                        indent.push_str("    "); // 4 spaces
                                    }

                                    if !indent.is_empty() {
                                        text.insert_str(cursor_pos, &indent);
                                    }
                                }
                            }
                        }
                    }

                    // Sync changes back to Rope buffer (only if text was actually edited)
                    if !is_readonly && output.response.changed() {
                        tab.buffer = crate::buffer::TextBuffer::from_str(&text);
                        tab.text_cache_version = tab.buffer.version();
                        tab.is_dirty = true;

                        // Notify LSP about changes
                        if let Some(lang) =
                            crate::native::lsp_native::detect_server_language(&tab.file_path)
                        {
                            if let Some(client) = &self.lsp_native_client {
                                let client = client.clone();
                                let path = tab.file_path.clone();
                                let text_clone = text.clone();
                                let language = lang.to_string();
                                self.lsp_runtime.spawn(async move {
                                    let _ =
                                        client.notify_change(&language, &path, &text_clone).await;
                                });
                            }
                        }
                    }

                    // Extract positions for overlays
                    let editor_rect = output.response.rect;
                    let galley = &output.galley;
                    // text_draw_pos is the screen position where galley starts drawing
                    let text_origin = output.galley_pos;

                    // Sync cursor_line from egui cursor position
                    if let Some(cr) = output.cursor_range {
                        let idx = cr.primary.ccursor.index;
                        let mut line = 0;
                        let mut count = 0;
                        for ch in text.chars() {
                            if count >= idx {
                                break;
                            }
                            if ch == '\n' {
                                line += 1;
                            }
                            count += 1;
                        }
                        tab.cursor_line = line;
                        tab.cursor_col = idx
                            - text
                                .lines()
                                .take(line)
                                .map(|l| l.len() + 1)
                                .sum::<usize>()
                                .min(idx);
                    }

                    // Build char offset for start of each line (used by line numbers and git gutter)
                    let mut line_char_offsets: Vec<usize> = vec![0];
                    for (i, ch) in text.chars().enumerate() {
                        if ch == '\n' {
                            line_char_offsets.push(i + 1);
                        }
                    }

                    // === Gutter: [BP dot | line number | fold icon] ===
                    // All positions relative to editor_rect.min.x (left edge of TextEdit)
                    // gutter_width = 64px, text starts at editor_rect.min.x + 64
                    // editor_rect = inner rect (margin excluded), text starts at editor_rect.min.x
                    // Gutter is in the margin area: BEFORE editor_rect.min.x
                    let gutter_left = editor_rect.min.x - gutter_width;
                    let bp_center_x = gutter_left + 10.0; // breakpoint dot
                    let line_num_right_x = gutter_left + 42.0; // line number right-align
                    let fold_center_x = gutter_left + 54.0; // fold icon center

                    let mut bp_toggle_line: Option<usize> = None;
                    {
                        let total_lines = text.lines().count();
                        let clip = ui.clip_rect();

                        for line_idx in 0..total_lines {
                            let char_offset = line_char_offsets.get(line_idx).copied().unwrap_or(0);
                            let cc = egui::text::CCursor::new(char_offset);
                            let cursor_obj = galley.from_ccursor(cc);
                            let pos_rect = galley.pos_from_cursor(&cursor_obj);
                            let y = text_origin.y + pos_rect.min.y;
                            let lh = (pos_rect.max.y - pos_rect.min.y).max(1.0);
                            let center_y = y + lh / 2.0;

                            if y + lh < clip.min.y {
                                continue;
                            }
                            if y > clip.max.y {
                                break;
                            }

                            // --- Breakpoint dot (leftmost) ---
                            let bp_area = egui::Rect::from_center_size(
                                egui::pos2(bp_center_x, center_y),
                                egui::vec2(16.0, lh),
                            );
                            let bp_hover = ui.input(|i| {
                                i.pointer
                                    .hover_pos()
                                    .map(|p| bp_area.contains(p))
                                    .unwrap_or(false)
                            });
                            // bp click handled outside ScrollArea

                            let has_bp = self
                                .debug_state
                                .breakpoints
                                .iter()
                                .any(|bp| bp.file_path == tab.file_path && bp.line == line_idx);
                            if has_bp {
                                ui.painter().circle_filled(
                                    egui::pos2(bp_center_x, center_y),
                                    5.0,
                                    egui::Color32::from_rgb(230, 50, 50),
                                );
                            } else if bp_hover {
                                ui.painter().circle_filled(
                                    egui::pos2(bp_center_x, center_y),
                                    4.0,
                                    egui::Color32::from_rgba_premultiplied(230, 50, 50, 60),
                                );
                            }

                            // --- Line number (center) ---
                            let num_color = if line_idx == tab.cursor_line {
                                egui::Color32::from_rgb(200, 200, 200)
                            } else {
                                egui::Color32::from_rgb(90, 90, 90)
                            };
                            ui.painter().text(
                                egui::pos2(line_num_right_x, y),
                                egui::Align2::RIGHT_TOP,
                                format!("{}", line_idx + 1),
                                egui::FontId::monospace(13.0),
                                num_color,
                            );

                            // --- Inlay hints (ghost text after tokens) ---
                            let hints: Vec<_> = inlay_hints_snapshot
                                .iter()
                                .filter(|h| h.line == line_idx)
                                .collect();
                            for h in &hints {
                                let col = h.column;
                                let label = &h.label;
                                let kind: &str = h.kind;
                                // Calculate x position: use galley to find the char position
                                let line_start =
                                    line_char_offsets.get(line_idx).copied().unwrap_or(0);
                                let hint_offset = line_start + col;
                                let cc = egui::text::CCursor::new(hint_offset.min(text.len()));
                                let cursor_obj = galley.from_ccursor(cc);
                                let hint_pos = galley.pos_from_cursor(&cursor_obj);
                                let hint_x = text_origin.x + hint_pos.max.x + 2.0;

                                let hint_color = if kind == "parameter" {
                                    egui::Color32::from_rgba_premultiplied(140, 180, 220, 160)
                                } else {
                                    egui::Color32::from_rgba_premultiplied(120, 160, 140, 160)
                                };

                                let display = if kind == "parameter" {
                                    format!("{}:", label)
                                } else {
                                    format!(": {}", label)
                                };

                                ui.painter().text(
                                    egui::pos2(hint_x, y),
                                    egui::Align2::LEFT_TOP,
                                    &display,
                                    egui::FontId::monospace(12.0),
                                    hint_color,
                                );
                            }

                            // --- Code action lightbulb (💡) ---
                            if line_idx == code_action_line
                                && has_code_actions
                                && line_idx == tab.cursor_line
                            {
                                let bulb_x = gutter_left + 54.0;
                                ui.painter().text(
                                    egui::pos2(bulb_x, y),
                                    egui::Align2::CENTER_TOP,
                                    "\u{eb2f}", // codicon: lightbulb
                                    egui::FontId::proportional(14.0),
                                    egui::Color32::from_rgb(255, 204, 0),
                                );
                            }
                        }
                    }

                    // Bracket matching - highlight matching bracket pair
                    if let Some(cr) = output.cursor_range {
                        let cursor_idx = cr.primary.ccursor.index;
                        let chars: Vec<char> = text.chars().collect();

                        if cursor_idx < chars.len() {
                            let bracket_pairs = [('(', ')'), ('{', '}'), ('[', ']')];
                            let ch = chars[cursor_idx];

                            // Find matching bracket
                            let matching_idx = if let Some(&(open, close)) =
                                bracket_pairs.iter().find(|(o, _)| *o == ch)
                            {
                                // Forward search for closing bracket
                                let mut depth = 0;
                                let mut found = None;
                                for i in cursor_idx..chars.len() {
                                    if chars[i] == open {
                                        depth += 1;
                                    }
                                    if chars[i] == close {
                                        depth -= 1;
                                        if depth == 0 {
                                            found = Some(i);
                                            break;
                                        }
                                    }
                                }
                                found
                            } else if let Some(&(open, close)) =
                                bracket_pairs.iter().find(|(_, c)| *c == ch)
                            {
                                // Backward search for opening bracket
                                let mut depth = 0;
                                let mut found = None;
                                for i in (0..=cursor_idx).rev() {
                                    if chars[i] == close {
                                        depth += 1;
                                    }
                                    if chars[i] == open {
                                        depth -= 1;
                                        if depth == 0 {
                                            found = Some(i);
                                            break;
                                        }
                                    }
                                }
                                found
                            } else if cursor_idx > 0 {
                                // Also check character before cursor
                                let prev_ch = chars[cursor_idx - 1];
                                if let Some(&(open, close)) =
                                    bracket_pairs.iter().find(|(_, c)| *c == prev_ch)
                                {
                                    let mut depth = 0;
                                    let mut found = None;
                                    for i in (0..cursor_idx).rev() {
                                        if chars[i] == close {
                                            depth += 1;
                                        }
                                        if chars[i] == open {
                                            depth -= 1;
                                            if depth == 0 {
                                                found = Some(i);
                                                break;
                                            }
                                        }
                                    }
                                    found
                                } else {
                                    None
                                }
                            } else {
                                None
                            };

                            if let Some(match_idx) = matching_idx {
                                let highlight_color =
                                    egui::Color32::from_rgba_premultiplied(255, 255, 255, 30);
                                // Highlight both brackets using galley positions
                                for idx in [cursor_idx, match_idx] {
                                    if idx < chars.len() {
                                        let c = egui::text::CCursor::new(idx);
                                        let cursor_obj = galley.from_ccursor(c);
                                        let rect = galley.pos_from_cursor(&cursor_obj);
                                        let c_next = egui::text::CCursor::new(idx + 1);
                                        let cursor_next = galley.from_ccursor(c_next);
                                        let rect_next = galley.pos_from_cursor(&cursor_next);

                                        let highlight_rect = egui::Rect::from_min_max(
                                            egui::pos2(
                                                editor_rect.min.x + rect.min.x,
                                                editor_rect.min.y + rect.min.y,
                                            ),
                                            egui::pos2(
                                                editor_rect.min.x + rect_next.min.x,
                                                editor_rect.min.y + rect.max.y,
                                            ),
                                        );
                                        ui.painter().rect_filled(
                                            highlight_rect,
                                            2.0,
                                            highlight_color,
                                        );
                                    }
                                }
                            }
                        }
                    }

                    // Draw diagnostic underlines (errors = red, warnings = yellow)
                    {
                        let current_file = &tab.file_path;
                        for diag in &self.lsp_diagnostics {
                            if diag.source.as_deref() == Some(current_file) {
                                let diag_line = diag.line;
                                let diag_col = diag.column;

                                // Calculate position from line/column
                                let mut char_offset = 0;
                                for (idx, line_text) in text.lines().enumerate() {
                                    if idx == diag_line {
                                        char_offset += diag_col.min(line_text.len());
                                        break;
                                    }
                                    char_offset += line_text.len() + 1;
                                }

                                // Get word length at diagnostic position (approximate)
                                let chars: Vec<char> = text.chars().collect();
                                let mut end = char_offset;
                                while end < chars.len()
                                    && (chars[end].is_alphanumeric() || chars[end] == '_')
                                {
                                    end += 1;
                                }
                                if end == char_offset {
                                    end = (char_offset + 1).min(chars.len());
                                }

                                let start_c = egui::text::CCursor::new(char_offset);
                                let end_c = egui::text::CCursor::new(end);
                                let start_cursor = galley.from_ccursor(start_c);
                                let end_cursor = galley.from_ccursor(end_c);
                                let start_rect = galley.pos_from_cursor(&start_cursor);
                                let end_rect = galley.pos_from_cursor(&end_cursor);

                                let color = match diag.severity {
                                    super::types::DiagnosticSeverity::Error => {
                                        egui::Color32::from_rgb(255, 80, 80)
                                    }
                                    super::types::DiagnosticSeverity::Warning => {
                                        egui::Color32::from_rgb(255, 200, 0)
                                    }
                                    _ => egui::Color32::from_rgb(100, 180, 255),
                                };

                                // Draw squiggly underline
                                let y = editor_rect.min.y + start_rect.max.y;
                                let x_start = editor_rect.min.x + start_rect.min.x;
                                let x_end = editor_rect.min.x + end_rect.min.x;

                                // Simple wave pattern
                                let mut points = Vec::new();
                                let mut x = x_start;
                                let mut up = true;
                                while x < x_end {
                                    let dy = if up { -2.0 } else { 2.0 };
                                    points.push(egui::pos2(x, y + dy));
                                    x += 3.0;
                                    up = !up;
                                }
                                if points.len() >= 2 {
                                    for window in points.windows(2) {
                                        ui.painter().line_segment(
                                            [window[0], window[1]],
                                            egui::Stroke::new(1.0, color),
                                        );
                                    }
                                }
                            }
                        }
                    }

                    // Cmd+hover: underline the word under cursor & change color
                    // Cmd+Click: go-to-definition
                    let mut go_to_def_data = None;

                    let cmd_held = ui.input(|i| i.modifiers.command);
                    let hover_pos = ui.input(|i| i.pointer.hover_pos());
                    let pointer_released = ui.input(|i| i.pointer.any_released());

                    if cmd_held {
                        if let Some(pos) = hover_pos {
                            if editor_rect.contains(pos) {
                                // Convert screen position to galley-local position
                                let local_pos = pos - editor_rect.min;
                                let cursor = galley.cursor_from_pos(local_pos);
                                let char_idx = cursor.ccursor.index;

                                // Extract the word at this position
                                let chars: Vec<char> = text.chars().collect();
                                if char_idx < chars.len() {
                                    let mut start = char_idx;
                                    while start > 0
                                        && (chars[start - 1].is_alphanumeric()
                                            || chars[start - 1] == '_')
                                    {
                                        start -= 1;
                                    }
                                    let mut end = char_idx;
                                    while end < chars.len()
                                        && (chars[end].is_alphanumeric() || chars[end] == '_')
                                    {
                                        end += 1;
                                    }

                                    if end > start {
                                        // Get the pixel positions of the word start and end
                                        let start_cursor = egui::text::CCursor::new(start);
                                        let end_cursor = egui::text::CCursor::new(end);
                                        let start_rect = galley.pos_from_cursor(
                                            &egui::epaint::text::cursor::Cursor {
                                                ccursor: start_cursor,
                                                rcursor: galley.from_ccursor(start_cursor).rcursor,
                                                pcursor: galley.from_ccursor(start_cursor).pcursor,
                                            },
                                        );
                                        let end_rect = galley.pos_from_cursor(
                                            &egui::epaint::text::cursor::Cursor {
                                                ccursor: end_cursor,
                                                rcursor: galley.from_ccursor(end_cursor).rcursor,
                                                pcursor: galley.from_ccursor(end_cursor).pcursor,
                                            },
                                        );

                                        // Draw underline
                                        let link_color = egui::Color32::from_rgb(86, 156, 214); // VS Code link blue
                                        let underline_y = editor_rect.min.y + start_rect.max.y;
                                        let underline_start = egui::pos2(
                                            editor_rect.min.x + start_rect.min.x,
                                            underline_y,
                                        );
                                        let underline_end = egui::pos2(
                                            editor_rect.min.x + end_rect.min.x,
                                            underline_y,
                                        );

                                        ui.painter().line_segment(
                                            [underline_start, underline_end],
                                            egui::Stroke::new(1.0, link_color),
                                        );

                                        // Draw colored overlay text
                                        let word_str: String = chars[start..end].iter().collect();
                                        let text_pos = egui::pos2(
                                            editor_rect.min.x + start_rect.min.x,
                                            editor_rect.min.y + start_rect.min.y,
                                        );
                                        // Paint a background rect to hide the original text, then draw colored text
                                        let bg_rect = egui::Rect::from_min_max(
                                            text_pos,
                                            egui::pos2(
                                                editor_rect.min.x + end_rect.min.x,
                                                editor_rect.min.y + start_rect.max.y,
                                            ),
                                        );
                                        ui.painter().rect_filled(
                                            bg_rect,
                                            0.0,
                                            ui_colors::SIDEBAR_BG,
                                        );
                                        ui.painter().text(
                                            text_pos,
                                            egui::Align2::LEFT_TOP,
                                            &word_str,
                                            egui::FontId::monospace(13.0),
                                            link_color,
                                        );

                                        // Change cursor to pointing hand
                                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);

                                        // Request repaint to clear underline when Cmd is released
                                        ui.ctx().request_repaint();
                                    }
                                }

                                // Cmd+Click detection (pointer released while Cmd held)
                                if pointer_released {
                                    go_to_def_data = Some((text.clone(), char_idx));
                                }
                            }
                        }
                    }

                    // Sync cursor position (simplified for MVP)
                    let _cursor = output.cursor_range;

                    // Manually set cursor if we have a pending jump
                    // Do this AFTER all other operations on output
                    if let Some(cursor_range) = cursor_range_to_set {
                        let response_id = output.response.id;
                        let mut state = output.state.clone();
                        state.cursor.set_char_range(Some(cursor_range));
                        state.store(ui.ctx(), response_id);

                        // Request focus to ensure the TextEdit scrolls to cursor
                        output.response.request_focus();

                        // Force scroll to cursor position
                        if let Some(y) = scroll_to_y {
                            const APPROX_LINE_HEIGHT: f32 = 19.5;
                            // Create a rect at the cursor position
                            let cursor_rect = egui::Rect::from_min_size(
                                egui::pos2(0.0, y),
                                egui::vec2(100.0, APPROX_LINE_HEIGHT * 3.0), // Show a few lines around cursor
                            );
                            // Scroll to make this rect visible
                            ui.scroll_to_rect(cursor_rect, Some(egui::Align::Center));
                            tracing::info!("📜 Scrolling to rect at y={}", y);
                        }
                    }

                    // === Indent Guides: draw vertical lines at each indentation level ===
                    // Only for code files (not markdown, plaintext, etc.)
                    let is_code_file = tab.file_path.ends_with(".rs")
                        || tab.file_path.ends_with(".js")
                        || tab.file_path.ends_with(".ts")
                        || tab.file_path.ends_with(".c")
                        || tab.file_path.ends_with(".cpp")
                        || tab.file_path.ends_with(".json")
                        || tab.file_path.ends_with(".java")
                        || tab.file_path.ends_with(".py")
                        || tab.file_path.ends_with(".toml");
                    if is_code_file {
                        let indent_color = egui::Color32::from_rgba_premultiplied(80, 80, 80, 20);
                        let tab_size = 4_usize;

                        // Get actual character width from galley by measuring position of char 0 vs char 1
                        let char_width = {
                            let c0 = galley.from_ccursor(egui::text::CCursor::new(0));
                            let c1 = galley.from_ccursor(egui::text::CCursor::new(1));
                            let r0 = galley.pos_from_cursor(&c0);
                            let r1 = galley.pos_from_cursor(&c1);
                            (r1.min.x - r0.min.x).max(1.0)
                        };

                        // Get actual line height from galley
                        let line_height = {
                            let c0 = galley.from_ccursor(egui::text::CCursor::new(0));
                            let r0 = galley.pos_from_cursor(&c0);
                            (r0.max.y - r0.min.y).max(1.0)
                        };

                        // Determine visible lines
                        let scroll_offset_y = ui.clip_rect().min.y - editor_rect.min.y;
                        let visible_height = ui.clip_rect().height();
                        let first_visible =
                            (scroll_offset_y / line_height).floor().max(0.0) as usize;
                        let last_visible =
                            ((scroll_offset_y + visible_height) / line_height).ceil() as usize + 5;

                        for (line_idx, line_text) in text.lines().enumerate() {
                            if line_idx < first_visible {
                                continue;
                            }
                            if line_idx > last_visible {
                                break;
                            }

                            let indent_chars = line_text.chars().take_while(|c| *c == ' ').count();
                            let indent_levels = indent_chars / tab_size;

                            for level in 1..=indent_levels {
                                let x_offset = (level * tab_size) as f32 * char_width;
                                let y_start = line_idx as f32 * line_height;
                                let y_end = (line_idx + 1) as f32 * line_height;

                                ui.painter().line_segment(
                                    [
                                        egui::pos2(
                                            editor_rect.min.x + x_offset,
                                            editor_rect.min.y + y_start,
                                        ),
                                        egui::pos2(
                                            editor_rect.min.x + x_offset,
                                            editor_rect.min.y + y_end,
                                        ),
                                    ],
                                    egui::Stroke::new(0.5, indent_color),
                                );
                            }
                        }
                    }

                    // === Git Gutter Diff Markers ===
                    // Thin colored bar at the left edge of the text area
                    if tab.git_changes_loaded && !tab.git_line_changes.is_empty() {
                        let clip = ui.clip_rect();
                        let bar_width = 3.0_f32;
                        let bar_x = text_origin.x - 4.0; // just to the left of code text

                        for change in &tab.git_line_changes {
                            if change.line >= line_char_offsets.len() {
                                continue;
                            }
                            let char_offset = line_char_offsets[change.line];
                            let cc = egui::text::CCursor::new(char_offset);
                            let cursor_obj = galley.from_ccursor(cc);
                            let pos_rect = galley.pos_from_cursor(&cursor_obj);
                            let y = text_origin.y + pos_rect.min.y;
                            let lh = (pos_rect.max.y - pos_rect.min.y).max(1.0);

                            if y + lh < clip.min.y {
                                continue;
                            }
                            if y > clip.max.y {
                                break;
                            }

                            let color = match change.change_type {
                                crate::native::git::LineChangeType::Added => {
                                    egui::Color32::from_rgb(80, 200, 80)
                                }
                                crate::native::git::LineChangeType::Modified => {
                                    egui::Color32::from_rgb(80, 150, 255)
                                }
                                crate::native::git::LineChangeType::Deleted => {
                                    egui::Color32::from_rgb(255, 80, 80)
                                }
                            };

                            let rect = egui::Rect::from_min_size(
                                egui::pos2(bar_x, y),
                                egui::vec2(bar_width, lh),
                            );
                            ui.painter().rect_filled(rect, 0.0, color);
                        }
                    }

                    // Breakpoint dots are now rendered in the line number gutter above

                    // === Fold Gutter: show fold/unfold arrows for foldable lines ===
                    // Only show for languages with braces (Rust, JS, TS, C, C++, JSON)
                    let is_foldable_language = tab.file_path.ends_with(".rs")
                        || tab.file_path.ends_with(".js")
                        || tab.file_path.ends_with(".ts")
                        || tab.file_path.ends_with(".c")
                        || tab.file_path.ends_with(".cpp")
                        || tab.file_path.ends_with(".json")
                        || tab.file_path.ends_with(".java");
                    let mut fold_toggle_line: Option<usize> = None;
                    if is_foldable_language {
                        let clip = ui.clip_rect();

                        for (line_idx, line_text) in text.lines().enumerate() {
                            if line_text.contains('{') {
                                // Get Y from galley
                                let char_offset =
                                    line_char_offsets.get(line_idx).copied().unwrap_or(0);
                                let cc = egui::text::CCursor::new(char_offset);
                                let cursor_obj = galley.from_ccursor(cc);
                                let pos_rect = galley.pos_from_cursor(&cursor_obj);
                                let y = text_origin.y + pos_rect.min.y;
                                let lh = (pos_rect.max.y - pos_rect.min.y).max(1.0);

                                if y + lh < clip.min.y {
                                    continue;
                                }
                                if y > clip.max.y {
                                    break;
                                }

                                let is_folded =
                                    tab.folded_regions.iter().any(|(s, _)| *s == line_idx);
                                let icon = if is_folded { "\u{25B6}" } else { "\u{25BC}" };
                                let fold_rect = egui::Rect::from_center_size(
                                    egui::pos2(fold_center_x, y + lh / 2.0),
                                    egui::vec2(14.0, lh),
                                );

                                let fold_click = false; // click handled outside ScrollArea
                                if fold_click {
                                    fold_toggle_line = Some(line_idx);
                                }

                                let fold_hover = ui.input(|i| {
                                    i.pointer
                                        .hover_pos()
                                        .map(|p| fold_rect.contains(p))
                                        .unwrap_or(false)
                                });
                                let fold_color = if fold_hover {
                                    egui::Color32::from_rgb(200, 200, 200)
                                } else {
                                    egui::Color32::from_rgb(100, 100, 100)
                                };

                                ui.painter().text(
                                    fold_rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    icon,
                                    egui::FontId::proportional(9.0),
                                    fold_color,
                                );
                            }
                        }
                    }

                    // === Multi-cursor rendering ===
                    {
                        for &cursor_pos in &self.multi_cursors {
                            if cursor_pos <= text.len() {
                                let c = egui::text::CCursor::new(cursor_pos);
                                let cursor_obj = galley.from_ccursor(c);
                                let rect = galley.pos_from_cursor(&cursor_obj);
                                let x = editor_rect.min.x + rect.min.x;
                                let y_start = editor_rect.min.y + rect.min.y;
                                let y_end = editor_rect.min.y + rect.max.y;
                                ui.painter().line_segment(
                                    [egui::pos2(x, y_start), egui::pos2(x, y_end)],
                                    egui::Stroke::new(1.5, egui::Color32::WHITE),
                                );
                            }
                        }
                    }

                    // Minimap is rendered outside ScrollArea (see below)

                    // === Peek Definition ===
                    if let Some(peek) = &self.peek_definition {
                        render_peek_standalone(ui, peek, editor_rect);
                    }

                    // === Compute cursor line for inline blame ===
                    let cursor_line_for_blame: Option<usize> = output.cursor_range.map(|cr| {
                        let idx = cr.primary.ccursor.index;
                        text[..idx.min(text.len())].matches('\n').count()
                    });

                    // Store text back into cache (avoids re-conversion next frame)
                    // Restore original (unfolded) text to cache, not the folded version
                    tab.text_cache = if tab.folded_regions.is_empty() {
                        text
                    } else {
                        original_for_cache
                    };

                    // Return gutter layout info for click handling outside ScrollArea
                    let gutter_info = (gutter_left, fold_center_x, bp_center_x, editor_rect);
                    (
                        output,
                        go_to_def_data,
                        cursor_line_for_blame,
                        fold_toggle_line,
                        bp_toggle_line,
                        gutter_info,
                    )
                });

                // If we had a scroll target, ensure we scroll there
                if let Some(_y) = scroll_to_y {
                    // Force another repaint to ensure scroll takes effect
                    ctx.request_repaint();
                }

                // Clear pending cursor jump after rendering
                if let Some(tab) = self.editor_tabs.get_mut(self.active_tab_idx) {
                    if tab.pending_cursor_jump.is_some() {
                        tab.pending_cursor_jump = None;
                    }
                }

                // Handle fold toggle
                if let Some(line) = scroll_output.inner.3 {
                    self.toggle_fold_at_line(line);
                }

                // Gutter click handling (outside ScrollArea to avoid TextEdit consuming events)
                {
                    let (gutter_left, _fold_cx, _bp_cx, ed_rect) = scroll_output.inner.5;
                    let clicked = ctx.input(|i| i.pointer.any_pressed());
                    let click_pos = ctx.input(|i| i.pointer.interact_pos());

                    if clicked {
                        if let Some(pos) = click_pos {
                            // Check if click is in the gutter area (between gutter_left and editor_rect.min.x)
                            if pos.x >= gutter_left
                                && pos.x < ed_rect.min.x
                                && pos.y >= ed_rect.min.y
                                && pos.y <= ed_rect.max.y
                            {
                                // Calculate line from Y position
                                let relative_y =
                                    pos.y - ed_rect.min.y + scroll_output.state.offset.y;
                                // Use approximate line height (galley coords)
                                let line_height = 15.0_f32; // from galley pos_rect
                                let clicked_line = (relative_y / line_height).floor() as usize;
                                let tab = &self.editor_tabs[self.active_tab_idx];
                                let total_lines = tab.text_cache.lines().count();

                                if clicked_line < total_lines {
                                    // Determine if it's a BP click or fold click based on X
                                    let bp_zone_right = gutter_left + 22.0;
                                    let fold_zone_left = gutter_left + 44.0;

                                    if pos.x < bp_zone_right {
                                        // Breakpoint toggle
                                        let file_path = tab.file_path.clone();
                                        if let Some(idx) =
                                            self.debug_state.breakpoints.iter().position(|bp| {
                                                bp.file_path == file_path && bp.line == clicked_line
                                            })
                                        {
                                            self.debug_state.breakpoints.remove(idx);
                                        } else {
                                            self.debug_state.breakpoints.push(
                                                crate::native::dap::DapBreakpoint {
                                                    line: clicked_line,
                                                    verified: false,
                                                    file_path,
                                                },
                                            );
                                        }
                                    } else if pos.x >= fold_zone_left {
                                        // Fold toggle
                                        let line_text: &str =
                                            tab.text_cache.lines().nth(clicked_line).unwrap_or("");
                                        if line_text.contains('{') {
                                            self.toggle_fold_at_line(clicked_line);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Handle go-to-definition outside the closure
                if let Some((text, cursor_pos)) = scroll_output.inner.1 {
                    tracing::info!("Triggering go-to-definition at position {}", cursor_pos);
                    self.handle_go_to_definition(&text, cursor_pos);
                }

                // === Inline Git Blame ===
                {
                    let cursor_line_for_blame = scroll_output.inner.2;
                    if let Some(current_line) = cursor_line_for_blame {
                        let active_file = self.editor_tabs[self.active_tab_idx].file_path.clone();
                        if current_line != self.blame_cache_line
                            || active_file != self.blame_cache_file
                        {
                            self.blame_cache_line = current_line;
                            self.blame_cache_file = active_file.clone();
                            match crate::native::git::get_line_blame(
                                &self.root_path,
                                &active_file,
                                current_line,
                            ) {
                                Ok(Some(blame)) => {
                                    let time_str =
                                        chrono::DateTime::from_timestamp(blame.timestamp, 0)
                                            .map(|dt| dt.format("%Y-%m-%d").to_string())
                                            .unwrap_or_default();
                                    self.blame_cache_text = format!(
                                        "{} \u{2022} {} \u{2014} {}",
                                        blame.author, time_str, blame.message
                                    );
                                }
                                _ => {
                                    self.blame_cache_text = String::new();
                                }
                            }
                        }
                    }
                }

                // LSP Status bar at bottom
                ui.separator();
                ui.horizontal(|ui| {
                    // Connection status
                    let status_text = if self.lsp_connected {
                        "LSP: Connected"
                    } else {
                        "LSP: Disconnected"
                    };
                    ui.label(status_text);

                    ui.separator();

                    // Diagnostics count
                    ui.label(format!(
                        "{} {}",
                        t(self.ui_language, "Diagnostics:"),
                        self.lsp_diagnostics.len()
                    ));

                    // Inline blame info in status bar
                    if !self.blame_cache_text.is_empty() {
                        ui.separator();
                        ui.label(
                            egui::RichText::new(&self.blame_cache_text)
                                .size(11.0)
                                .color(egui::Color32::from_rgb(128, 128, 128)),
                        );
                    }

                    ui.separator();

                    // Completion trigger button
                    if ui
                        .button(t(self.ui_language, "Show Completions (Ctrl+Space)"))
                        .clicked()
                    {
                        self.trigger_lsp_completions();
                    }
                });
            });

        // Handle keyboard shortcuts for LSP
        self.handle_lsp_shortcuts(ctx);

        // Render completion popup
        if self.lsp_show_completions && !self.lsp_completions.is_empty() {
            self.render_lsp_completions(ctx);
        }

        // Render code actions popup (💡 menu)
        if self.show_code_actions && !self.lsp_code_actions.is_empty() {
            self.render_code_actions_window(ctx);
        }
    }

    /// Syntax highlighting layouter for egui::TextEdit
    /// Regex-based syntax highlighting with One Dark theme
    pub(crate) fn syntax_highlight_layouter(
        _ui: &egui::Ui,
        text: &str,
        highlighter: &SyntaxHighlighter,
        color_theme: &ColorTheme,
    ) -> egui::text::LayoutJob {
        let mut job = egui::text::LayoutJob::default();

        // Font size: 13px for optimal readability
        const FONT_SIZE: f32 = 13.0;
        // Default color unified white (#D4D4D4)
        let default_color = ui_colors::TEXT_DEFAULT;

        for line in text.lines() {
            // Get tokens from regex-based highlighter
            let tokens = highlighter.highlight_line(line);

            if tokens.is_empty() {
                // No tokens, just add the whole line in default color
                job.append(
                    line,
                    0.0,
                    egui::TextFormat {
                        font_id: egui::FontId::monospace(FONT_SIZE),
                        color: default_color,
                        ..Default::default()
                    },
                );
            } else {
                let mut pos = 0;

                for token in tokens {
                    // Add any text before this token (whitespace, punctuation, etc.)
                    if token.start > pos {
                        let before = &line[pos..token.start];
                        job.append(
                            before,
                            0.0,
                            egui::TextFormat {
                                font_id: egui::FontId::monospace(FONT_SIZE),
                                color: default_color,
                                ..Default::default()
                            },
                        );
                    }

                    // Map TokenType to VS Code Dark+ color scheme
                    let color = match token.token_type {
                        TokenType::Keyword => color_theme.keyword,
                        TokenType::Function => color_theme.function,
                        TokenType::Type => color_theme.type_,
                        TokenType::String => color_theme.string,
                        TokenType::Number => color_theme.number,
                        TokenType::Comment => color_theme.comment,
                        TokenType::DocComment => color_theme.doc_comment,
                        TokenType::Macro => color_theme.macro_,
                        TokenType::Attribute => color_theme.attribute,
                        TokenType::Constant => color_theme.constant,
                        TokenType::Lifetime => color_theme.lifetime,
                        TokenType::Identifier => color_theme.variable,
                        TokenType::Namespace => default_color,
                        TokenType::Operator => color_theme.operator,
                        TokenType::EscapeSequence => color_theme.string,
                    };

                    job.append(
                        &token.text,
                        0.0,
                        egui::TextFormat {
                            font_id: egui::FontId::monospace(FONT_SIZE),
                            color,
                            ..Default::default()
                        },
                    );

                    pos = token.end;
                }

                // Add any remaining text at the end of the line
                if pos < line.len() {
                    let remaining = &line[pos..];
                    job.append(
                        remaining,
                        0.0,
                        egui::TextFormat {
                            font_id: egui::FontId::monospace(FONT_SIZE),
                            color: default_color,
                            ..Default::default()
                        },
                    );
                }
            }

            // Add newline
            job.append(
                "\n",
                0.0,
                egui::TextFormat {
                    font_id: egui::FontId::monospace(FONT_SIZE),
                    color: default_color,
                    ..Default::default()
                },
            );
        }

        job
    }
}
