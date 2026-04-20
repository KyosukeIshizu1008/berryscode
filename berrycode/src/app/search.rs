//! Search panel, search dialog, and search operations

use super::types::SearchMatch;
use super::BerryCodeApp;
use crate::buffer::TextBuffer;
use crate::native;

impl BerryCodeApp {
    /// Render Search panel (Phase 5.2: project-wide search)
    pub(crate) fn render_search_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("🔍 Search in Files");
        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Search:");
            let response = ui.text_edit_singleline(&mut self.search_query);

            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.perform_project_search();
            }

            if ui.button("Go").clicked() {
                self.perform_project_search();
            }
        });

        ui.checkbox(&mut self.search_case_sensitive, "Case sensitive");

        ui.separator();

        // Display search results
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if !self.search_results.is_empty() {
                    ui.label(format!("Found {} matches:", self.search_results.len()));
                    ui.add_space(4.0);

                    // Clone results to avoid borrowing issues
                    let results = self.search_results.clone();
                    for (idx, result) in results.iter().enumerate() {
                        let is_selected = idx == self.current_search_index;

                        // Prepare display text and file path outside closure
                        let display_text = if let Some(ref file_path) = result.file_path {
                            // Extract filename from path
                            let filename = std::path::Path::new(file_path)
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or(file_path);
                            format!(
                                "{}:{} - {}",
                                filename,
                                result.line_number + 1,
                                result.line_text.trim()
                            )
                        } else {
                            // In-file search, just show line number
                            format!(
                                "Line {}: {}",
                                result.line_number + 1,
                                result.line_text.trim()
                            )
                        };
                        let file_path_clone = result.file_path.clone();

                        ui.horizontal(|ui| {
                            if ui.selectable_label(is_selected, display_text).clicked() {
                                self.current_search_index = idx;

                                // If clicking on a project-wide search result, open the file
                                if let Some(file_path) = file_path_clone {
                                    self.open_file_from_path(&file_path);
                                }
                                // TODO: Jump to line in editor
                            }
                        });
                    }
                } else if !self.search_query.is_empty() {
                    ui.label("No results found");
                }
            });
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
            false, // use_regex: false (literal search)
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
