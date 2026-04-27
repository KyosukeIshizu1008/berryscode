//! Search panel, search dialog, and search operations

use super::types::SearchMatch;
use super::BerryCodeApp;
use crate::buffer::TextBuffer;
use crate::native;

impl BerryCodeApp {
    /// Render Search panel (project-wide search) — VS Code-style.
    pub(crate) fn render_search_panel(&mut self, ui: &mut egui::Ui) {
        // Palette borrowed from VS Code's Dark+ theme so the panel feels
        // identical to anyone coming from VS Code.
        let text_primary = egui::Color32::from_rgb(204, 204, 204);
        let text_muted = egui::Color32::from_rgb(133, 133, 133);
        let text_dim = egui::Color32::from_rgb(110, 110, 110);
        let input_bg = egui::Color32::from_rgb(60, 60, 60);
        let input_border = egui::Color32::from_rgb(60, 60, 60);
        let input_border_focus = egui::Color32::from_rgb(0, 122, 204);
        let toggle_active_bg = egui::Color32::from_rgba_premultiplied(99, 122, 168, 80);
        let toggle_active_border = egui::Color32::from_rgb(99, 122, 168);
        let match_highlight = egui::Color32::from_rgba_premultiplied(234, 92, 0, 80);
        let row_hover_bg = egui::Color32::from_rgba_premultiplied(255, 255, 255, 12);

        // === SEARCH header with action icons (refresh / clear / collapse) ===
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("SEARCH")
                    .size(11.0)
                    .color(text_muted)
                    .strong(),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                let collapse_btn = ui.add(
                    egui::Button::new(
                        egui::RichText::new("⊟").size(12.0).color(text_muted),
                    )
                    .frame(false)
                    .min_size(egui::vec2(18.0, 18.0)),
                );
                if collapse_btn
                    .on_hover_text("Collapse All")
                    .clicked()
                    && !self.search_results.is_empty()
                {
                    let files: std::collections::HashSet<String> = self
                        .search_results
                        .iter()
                        .filter_map(|r| r.file_path.clone())
                        .collect();
                    self.search_collapsed_files = files;
                }
                let clear_btn = ui.add(
                    egui::Button::new(
                        egui::RichText::new("✕").size(11.0).color(text_muted),
                    )
                    .frame(false)
                    .min_size(egui::vec2(18.0, 18.0)),
                );
                if clear_btn
                    .on_hover_text("Clear Search Results")
                    .clicked()
                {
                    self.search_results.clear();
                    self.search_query.clear();
                    self.replace_query.clear();
                    self.current_search_index = 0;
                    self.search_collapsed_files.clear();
                }
                let refresh_btn = ui.add(
                    egui::Button::new(
                        egui::RichText::new("↻").size(12.0).color(text_muted),
                    )
                    .frame(false)
                    .min_size(egui::vec2(18.0, 18.0)),
                );
                if refresh_btn.on_hover_text("Refresh").clicked() {
                    self.perform_project_search();
                }
            });
        });
        ui.add_space(4.0);

        // === Search input row: [chevron] [text input + Aa ab .*] ===
        let mut do_search = false;
        let mut do_replace_all = false;

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;

            // Toggle replace chevron — VS Code shows ▶ / ▼ at the very left.
            let chevron = if self.search_show_replace { "⌄" } else { "›" };
            let chevron_btn = ui.add(
                egui::Button::new(
                    egui::RichText::new(chevron).size(13.0).color(text_muted),
                )
                .frame(false)
                .min_size(egui::vec2(14.0, 22.0)),
            );
            if chevron_btn
                .on_hover_text("Toggle Replace")
                .clicked()
            {
                self.search_show_replace = !self.search_show_replace;
            }

            // Single rounded frame containing the text input AND the three
            // option toggles (Aa / ab / .*), exactly like VS Code.
            let frame_stroke_color = input_border;
            let row_frame = egui::Frame::NONE
                .fill(input_bg)
                .stroke(egui::Stroke::new(1.0, frame_stroke_color))
                .corner_radius(egui::CornerRadius::same(2))
                .inner_margin(egui::Margin::symmetric(4, 2));

            row_frame.show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    let toggles_w = 3.0 * 22.0 + 2.0 * 2.0; // 3 toggles + gaps
                    let input_w = (ui.available_width() - toggles_w - 4.0).max(40.0);
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.search_query)
                            .desired_width(input_w)
                            .frame(false)
                            .text_color(text_primary)
                            .hint_text(
                                egui::RichText::new("Search").color(text_dim),
                            ),
                    );
                    if response.has_focus() {
                        ui.painter().rect_stroke(
                            response.rect.expand(2.0),
                            egui::CornerRadius::same(2),
                            egui::Stroke::new(1.0, input_border_focus),
                            egui::StrokeKind::Outside,
                        );
                    }
                    if response.lost_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter))
                    {
                        do_search = true;
                    }

                    // Helper: render one of the three inline toggle buttons.
                    let toggle = |ui: &mut egui::Ui,
                                       label: &str,
                                       state: &mut bool,
                                       hover: &str| {
                        let (bg, border) = if *state {
                            (toggle_active_bg, toggle_active_border)
                        } else {
                            (egui::Color32::TRANSPARENT, egui::Color32::TRANSPARENT)
                        };
                        let color = if *state {
                            egui::Color32::from_rgb(220, 220, 220)
                        } else {
                            text_muted
                        };
                        let btn = ui.add(
                            egui::Button::new(
                                egui::RichText::new(label).size(11.0).color(color),
                            )
                            .fill(bg)
                            .stroke(egui::Stroke::new(1.0, border))
                            .corner_radius(egui::CornerRadius::same(2))
                            .min_size(egui::vec2(22.0, 20.0)),
                        );
                        if btn.on_hover_text(hover).clicked() {
                            *state = !*state;
                        }
                    };

                    toggle(ui, "Aa", &mut self.search_case_sensitive, "Match Case");
                    toggle(ui, "ab", &mut self.search_whole_word, "Match Whole Word");
                    toggle(ui, ".*", &mut self.search_use_regex, "Use Regular Expression");
                });
            });
        });

        // === Replace input row (only when chevron is expanded) ===
        if self.search_show_replace {
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                // Spacer matching the chevron column width above (14px).
                ui.add_space(14.0);

                let row_frame = egui::Frame::NONE
                    .fill(input_bg)
                    .stroke(egui::Stroke::new(1.0, input_border))
                    .corner_radius(egui::CornerRadius::same(2))
                    .inner_margin(egui::Margin::symmetric(4, 2));

                row_frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        // Replace All button on the right; reserve ~24px.
                        let btn_w = 24.0;
                        let input_w =
                            (ui.available_width() - btn_w - 4.0).max(40.0);
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut self.replace_query)
                                .desired_width(input_w)
                                .frame(false)
                                .text_color(text_primary)
                                .hint_text(
                                    egui::RichText::new("Replace").color(text_dim),
                                ),
                        );
                        if response.has_focus() {
                            ui.painter().rect_stroke(
                                response.rect.expand(2.0),
                                egui::CornerRadius::same(2),
                                egui::Stroke::new(1.0, input_border_focus),
                                egui::StrokeKind::Outside,
                            );
                        }
                        let replace_all = ui.add(
                            egui::Button::new(
                                egui::RichText::new("⇄").size(12.0).color(text_muted),
                            )
                            .frame(false)
                            .min_size(egui::vec2(btn_w, 20.0)),
                        );
                        if replace_all
                            .on_hover_text("Replace All")
                            .clicked()
                        {
                            do_replace_all = true;
                        }
                    });
                });
            });
        }

        // === Files-to-include / Files-to-exclude (collapsed under "...") ===
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.add_space(14.0);
            let dots_label = if self.search_show_details {
                "▾"
            } else {
                "…"
            };
            let dots = ui.add(
                egui::Button::new(
                    egui::RichText::new(dots_label).size(11.0).color(text_muted),
                )
                .frame(false)
                .min_size(egui::vec2(20.0, 18.0)),
            );
            if dots
                .on_hover_text("Toggle Search Details")
                .clicked()
            {
                self.search_show_details = !self.search_show_details;
            }
        });

        if self.search_show_details {
            for (label, value) in [
                ("files to include", &mut self.search_include_glob),
                ("files to exclude", &mut self.search_exclude_glob),
            ] {
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.add_space(14.0);
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new(label).size(10.5).color(text_muted),
                        );
                        let row_frame = egui::Frame::NONE
                            .fill(input_bg)
                            .stroke(egui::Stroke::new(1.0, input_border))
                            .corner_radius(egui::CornerRadius::same(2))
                            .inner_margin(egui::Margin::symmetric(4, 2));
                        row_frame.show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::singleline(value)
                                    .desired_width(ui.available_width())
                                    .frame(false)
                                    .text_color(text_primary)
                                    .hint_text(
                                        egui::RichText::new("e.g. *.rs, src/**/*.toml")
                                            .color(text_dim),
                                    ),
                            );
                        });
                    });
                });
            }
        }

        if do_search {
            self.perform_project_search();
        }
        if do_replace_all {
            self.perform_project_replace_all();
        }

        ui.add_space(6.0);

        // === Results summary ===
        if !self.search_results.is_empty() {
            let file_count: std::collections::HashSet<&Option<String>> =
                self.search_results.iter().map(|r| &r.file_path).collect();
            ui.horizontal(|ui| {
                ui.add_space(2.0);
                ui.label(
                    egui::RichText::new(format!(
                        "{} results in {} file{}",
                        self.search_results.len(),
                        file_count.len(),
                        if file_count.len() == 1 { "" } else { "s" }
                    ))
                    .size(11.0)
                    .color(text_muted),
                );
            });
            ui.add_space(2.0);
        } else if !self.search_query.is_empty() {
            ui.add_space(2.0);
            ui.label(
                egui::RichText::new("No results found.")
                    .size(11.0)
                    .color(text_muted),
            );
        }

        // === Results list (collapsible per-file groups) ===
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if self.search_results.is_empty() {
                    return;
                }

                ui.spacing_mut().item_spacing.y = 0.0;

                // Group results by file_path while preserving order.
                let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
                for (idx, r) in self.search_results.iter().enumerate() {
                    let fp = match &r.file_path {
                        Some(p) => p.clone(),
                        None => continue,
                    };
                    if let Some(last) = groups.last_mut() {
                        if last.0 == fp {
                            last.1.push(idx);
                            continue;
                        }
                    }
                    groups.push((fp, vec![idx]));
                }

                let results = self.search_results.clone();
                let mut click_idx: Option<usize> = None;
                let mut toggle_collapse: Option<String> = None;
                let mut dismiss_file: Option<String> = None;

                for (file_path, indices) in &groups {
                    let collapsed =
                        self.search_collapsed_files.contains(file_path);
                    let chevron = if collapsed { "›" } else { "⌄" };
                    let filename = std::path::Path::new(file_path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(file_path)
                        .to_string();
                    let parent = std::path::Path::new(file_path)
                        .parent()
                        .and_then(|p| p.to_str())
                        .map(|p| {
                            // Strip the workspace root for a tidy display.
                            p.strip_prefix(&self.root_path)
                                .unwrap_or(p)
                                .trim_start_matches('/')
                                .to_string()
                        })
                        .unwrap_or_default();

                    // === File header row ===
                    let header_resp = ui
                        .scope(|ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 4.0;
                                ui.add_space(2.0);
                                ui.label(
                                    egui::RichText::new(chevron)
                                        .size(12.0)
                                        .color(text_muted),
                                );
                                ui.label(
                                    egui::RichText::new("📄")
                                        .size(11.5)
                                        .color(text_muted),
                                );
                                ui.label(
                                    egui::RichText::new(&filename)
                                        .size(12.0)
                                        .color(text_primary),
                                );
                                if !parent.is_empty() {
                                    ui.label(
                                        egui::RichText::new(&parent)
                                            .size(10.5)
                                            .color(text_dim),
                                    );
                                }
                                ui.with_layout(
                                    egui::Layout::right_to_left(
                                        egui::Align::Center,
                                    ),
                                    |ui| {
                                        let dismiss = ui.add(
                                            egui::Button::new(
                                                egui::RichText::new("✕")
                                                    .size(10.0)
                                                    .color(text_muted),
                                            )
                                            .frame(false)
                                            .min_size(egui::vec2(16.0, 16.0)),
                                        );
                                        if dismiss
                                            .on_hover_text("Dismiss")
                                            .clicked()
                                        {
                                            dismiss_file =
                                                Some(file_path.clone());
                                        }
                                        // Match-count badge (VS Code shows
                                        // a small grey pill on hover; we
                                        // keep it always visible to save a
                                        // hover state).
                                        let badge_frame = egui::Frame::NONE
                                            .fill(egui::Color32::from_rgb(
                                                70, 70, 70,
                                            ))
                                            .corner_radius(
                                                egui::CornerRadius::same(8),
                                            )
                                            .inner_margin(
                                                egui::Margin::symmetric(5, 0),
                                            );
                                        badge_frame.show(ui, |ui| {
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{}",
                                                    indices.len()
                                                ))
                                                .size(10.0)
                                                .color(text_primary),
                                            );
                                        });
                                    },
                                );
                            })
                            .response
                        })
                        .inner
                        .interact(egui::Sense::click());

                    if header_resp.hovered() {
                        ui.painter().rect_filled(
                            header_resp.rect,
                            egui::CornerRadius::ZERO,
                            row_hover_bg,
                        );
                    }
                    if header_resp.clicked() {
                        toggle_collapse = Some(file_path.clone());
                    }

                    if collapsed {
                        continue;
                    }

                    // === Match rows ===
                    for &idx in indices {
                        let r = &results[idx];
                        let is_selected = idx == self.current_search_index;

                        let row_resp = ui
                            .scope(|ui| {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 0.0;
                                    ui.add_space(20.0);

                                    // Build a LayoutJob so the matched
                                    // substring inside the line is
                                    // highlighted, exactly like VS Code.
                                    let mut job =
                                        egui::text::LayoutJob::default();
                                    let font =
                                        egui::FontId::proportional(11.5);
                                    let line = &r.line_text;
                                    let start = r.start_col.min(line.len());
                                    let end = r.end_col.min(line.len()).max(start);

                                    // Trim long leading whitespace for a
                                    // tidier display, but keep cursor cols
                                    // mapped correctly.
                                    let leading = line
                                        .chars()
                                        .take_while(|c| c.is_whitespace())
                                        .count();
                                    let cut = leading.min(start);
                                    let display = &line[cut..];
                                    let s = start - cut;
                                    let e = end - cut;

                                    job.append(
                                        &display[..s.min(display.len())],
                                        0.0,
                                        egui::TextFormat {
                                            font_id: font.clone(),
                                            color: text_primary,
                                            ..Default::default()
                                        },
                                    );
                                    if e > s && e <= display.len() {
                                        job.append(
                                            &display[s..e],
                                            0.0,
                                            egui::TextFormat {
                                                font_id: font.clone(),
                                                color: egui::Color32::from_rgb(
                                                    255, 255, 255,
                                                ),
                                                background: match_highlight,
                                                ..Default::default()
                                            },
                                        );
                                        job.append(
                                            &display[e..],
                                            0.0,
                                            egui::TextFormat {
                                                font_id: font.clone(),
                                                color: text_primary,
                                                ..Default::default()
                                            },
                                        );
                                    }

                                    ui.add(egui::Label::new(job).truncate());
                                })
                                .response
                            })
                            .inner
                            .interact(egui::Sense::click());

                        // Hover/selected background fills the row.
                        if is_selected {
                            ui.painter().rect_filled(
                                row_resp.rect,
                                egui::CornerRadius::ZERO,
                                egui::Color32::from_rgba_premultiplied(
                                    99, 122, 168, 60,
                                ),
                            );
                        } else if row_resp.hovered() {
                            ui.painter().rect_filled(
                                row_resp.rect,
                                egui::CornerRadius::ZERO,
                                row_hover_bg,
                            );
                        }

                        if row_resp.clicked() {
                            click_idx = Some(idx);
                        }
                    }
                }

                if let Some(fp) = toggle_collapse {
                    if !self.search_collapsed_files.remove(&fp) {
                        self.search_collapsed_files.insert(fp);
                    }
                }
                if let Some(fp) = dismiss_file {
                    self.search_results
                        .retain(|r| r.file_path.as_deref() != Some(fp.as_str()));
                    self.search_collapsed_files.remove(&fp);
                    if self.current_search_index >= self.search_results.len() {
                        self.current_search_index = 0;
                    }
                }
                if let Some(idx) = click_idx {
                    self.current_search_index = idx;
                    if let Some(fp) = results[idx].file_path.clone() {
                        self.open_file_from_path(&fp);
                    }
                    if let Some(tab) = self.editor_tabs.get_mut(self.active_tab_idx)
                    {
                        tab.pending_cursor_jump =
                            Some((results[idx].line_number, results[idx].start_col));
                    }
                }
            });
    }

    /// Project-wide replace driven by the search panel's Replace All button.
    /// Re-runs the current search afterwards so the result list reflects the
    /// post-replace state.
    pub(crate) fn perform_project_replace_all(&mut self) {
        if self.search_query.is_empty() || self.search_results.is_empty() {
            return;
        }
        match native::search::replace_in_files(
            &self.root_path,
            &self.search_query,
            &self.replace_query,
            self.search_case_sensitive,
        ) {
            Ok(modified) => {
                tracing::info!(
                    "✏️  Replaced '{}' → '{}' across {} files",
                    self.search_query,
                    self.replace_query,
                    modified.len()
                );
                // Reload any open buffers whose file was rewritten on disk
                // so the editor stays in sync with the new contents.
                for path in &modified {
                    if let Some(tab) = self
                        .editor_tabs
                        .iter_mut()
                        .find(|t| t.file_path == *path)
                    {
                        if let Ok(new_content) = std::fs::read_to_string(path) {
                            tab.buffer = TextBuffer::from_str(&new_content);
                            tab.text_cache = new_content;
                            tab.text_cache_version = tab.buffer.version();
                            tab.is_dirty = false;
                        }
                    }
                }
                self.perform_project_search();
            }
            Err(e) => {
                tracing::error!("❌ Project replace failed: {}", e);
            }
        }
    }

    /// Render search dialog
    pub(crate) fn render_search_dialog(&mut self, ctx: &egui::Context) {
        let mut close_dialog = false;

        let window_title = if self.show_replace {
            "🔍 Find & Replace"
        } else {
            "🔍 Search"
        };

        egui::Window::new(window_title)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_TOP, [0.0, 100.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Find:");
                    let response = ui.text_edit_singleline(&mut self.search_query);

                    // Auto-focus on open
                    if self.search_results.is_empty() && !self.search_query.is_empty() {
                        response.request_focus();
                    }

                    // Search on Enter
                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.perform_search();
                        response.request_focus();
                    }

                    if ui.button("Search").clicked() {
                        self.perform_search();
                    }
                });

                // Replace input field (only show in replace mode)
                if self.show_replace {
                    ui.horizontal(|ui| {
                        ui.label("Replace:");
                        ui.text_edit_singleline(&mut self.replace_query);

                        if ui.button("Replace").clicked() {
                            self.perform_replace_current();
                        }

                        if ui.button("Replace All").clicked() {
                            self.perform_replace_all();
                        }
                    });
                }

                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.search_case_sensitive, "Case sensitive");
                });

                ui.separator();

                // Display search results
                if !self.search_results.is_empty() {
                    ui.label(format!(
                        "Found {} matches (showing {}/{})",
                        self.search_results.len(),
                        self.current_search_index + 1,
                        self.search_results.len()
                    ));

                    ui.horizontal(|ui| {
                        if ui.button("⬆ Previous").clicked() {
                            self.go_to_previous_match();
                        }
                        if ui.button("⬇ Next").clicked() {
                            self.go_to_next_match();
                        }
                    });

                    ui.separator();

                    // Show all results in a scrollable list
                    let mut clicked_index: Option<usize> = None;

                    egui::ScrollArea::vertical()
                        .max_height(300.0)
                        .show(ui, |ui| {
                            for (idx, match_result) in self.search_results.iter().enumerate() {
                                let is_current = idx == self.current_search_index;

                                // Format the display text
                                let display_text = if let Some(file_path) = &match_result.file_path
                                {
                                    // Project-wide search: show file path and line
                                    let filename = file_path.split('/').last().unwrap_or(file_path);
                                    format!(
                                        "{}:{}: {}",
                                        filename,
                                        match_result.line_number + 1,
                                        match_result.line_text.trim()
                                    )
                                } else {
                                    // In-file search: just show line number
                                    format!(
                                        "Line {}: {}",
                                        match_result.line_number + 1,
                                        match_result.line_text.trim()
                                    )
                                };

                                // Make each result clickable
                                let response = ui.selectable_label(is_current, display_text);

                                if response.clicked() {
                                    clicked_index = Some(idx);
                                }
                            }
                        });

                    // Jump to clicked result (outside the borrow)
                    if let Some(idx) = clicked_index {
                        self.current_search_index = idx;
                        self.jump_to_current_match();
                    }
                } else if !self.search_query.is_empty() {
                    ui.label("No matches found");
                }

                ui.separator();

                if ui.button("Close").clicked() {
                    close_dialog = true;
                }

                // ESC to close
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    close_dialog = true;
                }
            });

        if close_dialog {
            self.search_dialog_open = false;
            self.search_results.clear();
            self.search_query.clear();
        }
    }

    /// Perform search in current file
    pub(crate) fn perform_search(&mut self) {
        self.search_results.clear();
        self.current_search_index = 0;

        if self.search_query.is_empty() || self.editor_tabs.is_empty() {
            return;
        }

        let tab = &self.editor_tabs[self.active_tab_idx];
        let content = tab.buffer.to_string();

        let query = if self.search_case_sensitive {
            self.search_query.clone()
        } else {
            self.search_query.to_lowercase()
        };

        for (line_number, line) in content.lines().enumerate() {
            let search_line = if self.search_case_sensitive {
                line.to_string()
            } else {
                line.to_lowercase()
            };

            let mut start_pos = 0;
            while let Some(pos) = search_line[start_pos..].find(&query) {
                let actual_pos = start_pos + pos;
                self.search_results.push(SearchMatch {
                    file_path: None, // In-file search doesn't need file_path
                    line_number,
                    start_col: actual_pos,
                    end_col: actual_pos + self.search_query.len(),
                    line_text: line.to_string(),
                });
                start_pos = actual_pos + 1;
            }
        }

        tracing::info!(
            "🔍 Search found {} matches for '{}'",
            self.search_results.len(),
            self.search_query
        );

        // Jump to first match if any results found
        if !self.search_results.is_empty() {
            self.jump_to_current_match();
        }
    }

    /// Perform project-wide search using native::search
    pub(crate) fn perform_project_search(&mut self) {
        self.search_results.clear();
        self.current_search_index = 0;

        if self.search_query.is_empty() {
            return;
        }

        tracing::info!(
            "🔍 Starting project-wide search for '{}' in {}",
            self.search_query,
            self.root_path
        );

        // Use native::search::search_in_files() for parallel search
        match native::search::search_in_files(
            &self.root_path,
            &self.search_query,
            self.search_case_sensitive,
            self.search_use_regex,
            self.search_whole_word,
        ) {
            Ok(results) => {
                // Convert native::search::SearchResult to our SearchMatch
                self.search_results = results
                    .into_iter()
                    .map(|r| SearchMatch {
                        file_path: Some(r.file_path),
                        line_number: r.line_number - 1, // native returns 1-based, we use 0-based
                        start_col: r.match_start,
                        end_col: r.match_end,
                        line_text: r.line_content,
                    })
                    .collect();

                tracing::info!(
                    "🔍 Project search found {} matches for '{}'",
                    self.search_results.len(),
                    self.search_query
                );

                // Jump to first match if any results found
                if !self.search_results.is_empty() {
                    self.jump_to_current_match();
                }
            }
            Err(e) => {
                tracing::error!("❌ Project search failed: {}", e);
            }
        }
    }

    /// Go to next search match
    pub(crate) fn go_to_next_match(&mut self) {
        if self.search_results.is_empty() {
            return;
        }

        self.current_search_index = (self.current_search_index + 1) % self.search_results.len();
        tracing::info!(
            "🔍 Next match: {}/{}",
            self.current_search_index + 1,
            self.search_results.len()
        );

        // Jump to the match location
        self.jump_to_current_match();
    }

    /// Go to previous search match
    pub(crate) fn go_to_previous_match(&mut self) {
        if self.search_results.is_empty() {
            return;
        }

        if self.current_search_index == 0 {
            self.current_search_index = self.search_results.len() - 1;
        } else {
            self.current_search_index -= 1;
        }
        tracing::info!(
            "🔍 Previous match: {}/{}",
            self.current_search_index + 1,
            self.search_results.len()
        );

        // Jump to the match location
        self.jump_to_current_match();
    }

    /// Jump to the current search match location
    pub(crate) fn jump_to_current_match(&mut self) {
        // Clone the match result to avoid borrowing issues
        let match_result = if let Some(m) = self.search_results.get(self.current_search_index) {
            m.clone()
        } else {
            return;
        };

        // If this is a project-wide search result with a file path, open that file first
        if let Some(file_path) = &match_result.file_path {
            // Check if the file is already open
            let file_already_open = self
                .editor_tabs
                .iter()
                .position(|tab| tab.file_path == *file_path);

            if let Some(tab_idx) = file_already_open {
                // File is already open, just switch to it
                self.active_tab_idx = tab_idx;
            } else {
                // Open the file
                self.open_file_from_path(file_path);
            }
        }

        // Set cursor position to the match location
        if let Some(tab) = self.editor_tabs.get_mut(self.active_tab_idx) {
            tab.cursor_line = match_result.line_number;
            tab.cursor_col = match_result.start_col;

            tracing::info!(
                "⚡ Jumped to {}:{}:{}",
                tab.file_path.split('/').last().unwrap_or(&tab.file_path),
                match_result.line_number + 1,
                match_result.start_col + 1
            );
        }
    }

    /// Replace current search match
    pub(crate) fn perform_replace_current(&mut self) {
        if self.search_results.is_empty() || self.editor_tabs.is_empty() {
            return;
        }

        let match_result = if let Some(m) = self.search_results.get(self.current_search_index) {
            m.clone()
        } else {
            return;
        };

        // Get current tab
        if let Some(tab) = self.editor_tabs.get_mut(self.active_tab_idx) {
            let content = tab.buffer.to_string();
            let lines: Vec<&str> = content.lines().collect();

            if match_result.line_number >= lines.len() {
                return;
            }

            let line = lines[match_result.line_number];
            let before = &line[..match_result.start_col];
            let after = &line[match_result.end_col..];
            let new_line = format!("{}{}{}", before, self.replace_query, after);

            // Replace the line in the buffer
            let mut new_lines: Vec<String> = lines.iter().map(|s| s.to_string()).collect();
            new_lines[match_result.line_number] = new_line;

            let new_content = new_lines.join("\n");
            tab.buffer = TextBuffer::from_str(&new_content);
            tab.mark_dirty();

            tracing::info!(
                "✏️  Replaced '{}' with '{}' at line {}",
                self.search_query,
                self.replace_query,
                match_result.line_number + 1
            );

            // Remove this match and move to next
            self.search_results.remove(self.current_search_index);
            if !self.search_results.is_empty()
                && self.current_search_index >= self.search_results.len()
            {
                self.current_search_index = 0;
            }

            // Re-run search to update matches
            self.perform_search();
        }
    }

    /// Replace all search matches
    pub(crate) fn perform_replace_all(&mut self) {
        if self.search_results.is_empty() || self.editor_tabs.is_empty() {
            return;
        }

        let tab = &mut self.editor_tabs[self.active_tab_idx];
        let content = tab.buffer.to_string();

        // Perform replace using simple string replacement
        let new_content = if self.search_case_sensitive {
            content.replace(&self.search_query, &self.replace_query)
        } else {
            // Case-insensitive replacement
            let mut result = content.clone();
            let query_lower = self.search_query.to_lowercase();
            let mut start = 0;

            while let Some(pos) = result[start..].to_lowercase().find(&query_lower) {
                let actual_pos = start + pos;
                result.replace_range(
                    actual_pos..actual_pos + self.search_query.len(),
                    &self.replace_query,
                );
                start = actual_pos + self.replace_query.len();
            }
            result
        };

        let count = self.search_results.len();
        tab.buffer = TextBuffer::from_str(&new_content);
        tab.mark_dirty();

        tracing::info!(
            "✏️  Replaced {} occurrences of '{}' with '{}'",
            count,
            self.search_query,
            self.replace_query
        );

        // Clear search results
        self.search_results.clear();
        self.current_search_index = 0;
    }
}
