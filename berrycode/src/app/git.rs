//! Git panel rendering and operations

use super::component_colors;
use super::types::GitTab;
use super::ui_colors;
use super::BerryCodeApp;
use crate::app::i18n::t;
use crate::native;

impl BerryCodeApp {
    /// Render Git panel (VS Code style)
    pub(crate) fn render_git_panel(&mut self, ui: &mut egui::Ui) {
        // Header
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("SOURCE CONTROL")
                    .size(11.0)
                    .strong()
                    .color(egui::Color32::from_rgb(187, 187, 187)),
            );
        });
        ui.add_space(4.0);

        // Tab bar (VS Code style — flat, underline active)
        self.render_git_tab_bar(ui);

        ui.add_space(4.0);

        // Render the active tab
        match self.git_active_tab {
            GitTab::Status => self.render_git_status_tab(ui),
            GitTab::History => self.render_git_history_tab(ui),
            GitTab::Branches => self.render_git_branches_tab(ui),
            GitTab::Remotes => self.render_git_remotes_tab(ui),
            GitTab::Tags => self.render_git_tags_tab(ui),
            GitTab::Stash => self.render_git_stash_tab(ui),
        }
    }

    /// Render Git tab bar (VS Code flat style)
    fn render_git_tab_bar(&mut self, ui: &mut egui::Ui) {
        let tabs = [
            (GitTab::Status, self.tr("Status")),
            (GitTab::History, self.tr("History")),
            (GitTab::Branches, self.tr("Branches")),
            (GitTab::Remotes, self.tr("Remotes")),
            (GitTab::Tags, self.tr("Tags")),
            (GitTab::Stash, self.tr("Stash")),
        ];
        super::utils::render_tab_bar(ui, &tabs, &mut self.git_active_tab);
    }

    /// Render Status tab (VS Code style)
    fn render_git_status_tab(&mut self, ui: &mut egui::Ui) {
        let btn_text = component_colors::BUTTON_TEXT;
        let btn_bg = component_colors::BUTTON_BG;
        let accent = component_colors::ACCENT;

        // Branch + refresh
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(&self.git_current_branch)
                    .size(11.0)
                    .color(egui::Color32::from_rgb(180, 180, 180)),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(
                        egui::Button::new(egui::RichText::new("↻").size(13.0).color(btn_text))
                            .frame(false),
                    )
                    .on_hover_text(self.tr("Refresh"))
                    .clicked()
                {
                    self.refresh_git_status();
                }
            });
        });

        ui.add_space(4.0);

        // Commit message
        ui.add(
            egui::TextEdit::singleline(&mut self.git_commit_message)
                .hint_text(t(self.ui_language, "Message:"))
                .desired_width(f32::INFINITY)
                .font(egui::FontId::proportional(12.0)),
        );

        ui.add_space(4.0);

        // Commit + Stage All buttons (VS Code flat style)
        ui.horizontal(|ui| {
            let commit_btn = egui::Button::new(
                egui::RichText::new(self.tr("Commit"))
                    .size(11.0)
                    .color(egui::Color32::WHITE),
            )
            .fill(accent)
            .corner_radius(3)
            .min_size(egui::vec2(70.0, 22.0));
            if ui.add(commit_btn).clicked() {
                self.perform_git_commit();
            }

            let stage_btn = egui::Button::new(
                egui::RichText::new(self.tr("Stage All"))
                    .size(11.0)
                    .color(btn_text),
            )
            .fill(btn_bg)
            .corner_radius(3)
            .min_size(egui::vec2(70.0, 22.0));
            if ui.add(stage_btn).clicked() {
                self.perform_git_stage_all();
            }
        });

        ui.add_space(4.0);

        // Changed files list with grouping
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if self.git_status.is_empty() {
                    ui.label(self.tr("No changes"));
                } else {
                    // Group files by staged/unstaged (index-based to avoid borrow conflict with self)
                    let staged_indices: Vec<usize> = self
                        .git_status
                        .iter()
                        .enumerate()
                        .filter(|(_, s)| s.is_staged)
                        .map(|(i, _)| i)
                        .collect();
                    let unstaged_indices: Vec<usize> = self
                        .git_status
                        .iter()
                        .enumerate()
                        .filter(|(_, s)| !s.is_staged)
                        .map(|(i, _)| i)
                        .collect();

                    // Staged changes
                    if !staged_indices.is_empty() {
                        ui.heading(format!("Staged Changes ({})", staged_indices.len()));
                        ui.add_space(4.0);
                        for i in staged_indices {
                            let status = self.git_status[i].clone();
                            self.render_file_status_row(ui, &status);
                        }
                        ui.add_space(8.0);
                    }

                    // Unstaged changes
                    if !unstaged_indices.is_empty() {
                        ui.heading(format!("Unstaged Changes ({})", unstaged_indices.len()));
                        ui.add_space(4.0);
                        for i in unstaged_indices {
                            let status = self.git_status[i].clone();
                            self.render_file_status_row(ui, &status);
                        }
                    }
                }
            });
    }

    /// Helper function to render a file status row (VS Code style)
    fn render_file_status_row(&mut self, ui: &mut egui::Ui, status: &native::git::GitStatus) {
        let status_letter = match status.status.as_str() {
            "modified" => "M",
            "added" => "A",
            "deleted" => "D",
            "renamed" => "R",
            _ => "?",
        };
        let status_color = match status.status.as_str() {
            "modified" => egui::Color32::from_rgb(255, 198, 109),
            "added" => egui::Color32::from_rgb(106, 180, 89),
            "deleted" => egui::Color32::from_rgb(255, 100, 100),
            _ => egui::Color32::LIGHT_GRAY,
        };
        let is_staged = status.is_staged;
        let path = status.path.clone();

        // Get file icon (same as file tree)
        let filename = path.rsplit('/').next().unwrap_or(&path);
        let (file_icon, icon_color) = Self::get_file_icon_with_color(filename);

        let row_height = 22.0;
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), row_height),
            egui::Sense::click(),
        );

        if ui.is_rect_visible(rect) {
            // Hover background
            if response.hovered() {
                ui.painter()
                    .rect_filled(rect, 2.0, egui::Color32::from_rgb(45, 47, 50));
            }

            let painter = ui.painter();

            // File icon (codicon)
            let icon_x = rect.left() + 10.0;
            painter.text(
                egui::pos2(icon_x, rect.center().y),
                egui::Align2::LEFT_CENTER,
                file_icon,
                egui::FontId::new(14.0, egui::FontFamily::Name("codicon".into())),
                icon_color,
            );

            // File name
            let name_x = icon_x + 20.0;
            painter.text(
                egui::pos2(name_x, rect.center().y),
                egui::Align2::LEFT_CENTER,
                filename,
                egui::FontId::proportional(13.0),
                egui::Color32::from_rgb(200, 205, 215),
            );

            // Status letter (M/A/D) on the right
            let status_x = rect.right() - 30.0;
            painter.text(
                egui::pos2(status_x, rect.center().y),
                egui::Align2::LEFT_CENTER,
                status_letter,
                egui::FontId::proportional(12.0),
                status_color,
            );

            // Stage/Unstage icon on hover (+ or -)
            if response.hovered() {
                let action_icon = if is_staged { "\u{eb2a}" } else { "\u{ea7a}" }; // codicon: remove / add
                let action_x = rect.right() - 50.0;
                let action_rect = egui::Rect::from_center_size(
                    egui::pos2(action_x, rect.center().y),
                    egui::vec2(18.0, 18.0),
                );
                painter.text(
                    action_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    action_icon,
                    egui::FontId::new(14.0, egui::FontFamily::Name("codicon".into())),
                    egui::Color32::from_rgb(200, 200, 200),
                );

                // Check click on action icon
                if response.clicked() {
                    if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                        if action_rect.contains(pos) {
                            if is_staged {
                                self.perform_git_unstage(&path);
                            } else {
                                self.perform_git_stage(&path);
                            }
                            return;
                        }
                    }
                }
            }
        }

        // Click on row → show diff
        if response.clicked() {
            self.load_git_diff(&path);
        }
    }

    /// Load git diff for a file and display in center panel
    pub(crate) fn load_git_diff(&mut self, file_path: &str) {
        tracing::info!("🔍 Loading diff for: {}", file_path);
        match native::git::get_diff(&self.root_path, file_path) {
            Ok(diff) => {
                self.git_diff_state.selected_file = Some(file_path.to_string());
                self.git_diff_state.diff = Some(diff);
                tracing::info!("✅ Diff loaded for: {}", file_path);
            }
            Err(e) => {
                tracing::error!("❌ Failed to load diff for {}: {}", file_path, e);
                self.git_diff_state.selected_file = None;
                self.git_diff_state.diff = None;
            }
        }
    }

    /// Render History tab (commit graph)
    fn render_git_history_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button(format!("🔄 {}", self.tr("Refresh"))).clicked() {
                self.refresh_git_history();
            }

            ui.checkbox(
                &mut self.git_history_state.show_all_branches,
                "All branches",
            );

            if ui.button(self.tr("Load More")).clicked() {
                self.git_history_state.page_limit += 100;
                self.refresh_git_history();
            }
        });

        ui.separator();

        // Filter inputs
        ui.horizontal(|ui| {
            ui.label(self.tr("Author:"));
            ui.text_edit_singleline(&mut self.git_history_state.filter_author);
            ui.label(self.tr("Message:"));
            ui.text_edit_singleline(&mut self.git_history_state.filter_message);
        });

        ui.separator();

        // 3-pane layout: Graph | Commit List | Details
        ui.horizontal(|ui| {
            // Left: Commit graph and list (60% width)
            ui.vertical(|ui| {
                ui.set_width(ui.available_width() * 0.6);

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if self.git_history_state.graph_nodes.is_empty() {
                            ui.label(self.tr("No commits. Click Refresh to load."));
                        } else {
                            self.render_commit_graph(ui);
                        }
                    });
            });

            ui.separator();

            // Right: Commit details (40% width)
            ui.vertical(|ui| {
                ui.set_width(ui.available_width());
                self.render_commit_details(ui);
            });
        });
    }

    /// Render commit graph with egui painter
    fn render_commit_graph(&mut self, ui: &mut egui::Ui) {
        let node_count = self.git_history_state.graph_nodes.len();

        const NODE_RADIUS: f32 = 4.0;
        const COLUMN_WIDTH: f32 = 16.0;
        const ROW_HEIGHT: f32 = 24.0;

        // 8-color palette for branches
        let colors = [
            egui::Color32::from_rgb(106, 180, 89),  // Green
            egui::Color32::from_rgb(100, 181, 246), // Blue
            egui::Color32::from_rgb(255, 198, 109), // Yellow
            egui::Color32::from_rgb(239, 83, 80),   // Red
            egui::Color32::from_rgb(171, 128, 255), // Purple
            egui::Color32::from_rgb(255, 138, 128), // Coral
            egui::Color32::from_rgb(128, 222, 234), // Cyan
            egui::Color32::from_rgb(255, 171, 64),  // Orange
        ];

        for idx in 0..node_count {
            // Extract display data before mutable closure
            let node = &self.git_history_state.graph_nodes[idx];
            let graph_lines: Vec<_> = node
                .graph_lines
                .iter()
                .map(|l| {
                    (
                        l.from_column,
                        l.to_column,
                        l.color_index,
                        l.line_type.clone(),
                    )
                })
                .collect();
            let graph_column = node.graph_column;
            let commit_id = node.commit.id.clone();
            let commit_message = node.commit.message.clone();
            let commit_author = node.commit.author.clone();
            let is_selected =
                self.git_history_state.selected_commit_id.as_ref() == Some(&commit_id);

            ui.horizontal(|ui| {
                // Graph column (left side)
                let (graph_rect, _graph_response) = ui.allocate_exact_size(
                    egui::vec2(COLUMN_WIDTH * 8.0, ROW_HEIGHT),
                    egui::Sense::click(),
                );

                if ui.is_rect_visible(graph_rect) {
                    let painter = ui.painter();

                    // Draw graph lines
                    for (from_col, to_col, color_idx, line_type) in &graph_lines {
                        let from_pos = graph_rect.min
                            + egui::vec2(
                                *from_col as f32 * COLUMN_WIDTH + COLUMN_WIDTH / 2.0,
                                NODE_RADIUS,
                            );
                        let to_pos = graph_rect.min
                            + egui::vec2(
                                *to_col as f32 * COLUMN_WIDTH + COLUMN_WIDTH / 2.0,
                                ROW_HEIGHT,
                            );

                        let color = colors[color_idx % colors.len()];

                        if *line_type == native::git::GraphLineType::Direct {
                            // Straight line
                            painter.line_segment([from_pos, to_pos], egui::Stroke::new(2.0, color));
                        } else {
                            // Bezier curve for merge
                            painter.add(egui::Shape::CubicBezier(
                                egui::epaint::CubicBezierShape::from_points_stroke(
                                    [
                                        from_pos,
                                        from_pos + egui::vec2(0.0, ROW_HEIGHT * 0.3),
                                        to_pos - egui::vec2(0.0, ROW_HEIGHT * 0.3),
                                        to_pos,
                                    ],
                                    false,
                                    egui::Color32::TRANSPARENT,
                                    egui::Stroke::new(2.0, color),
                                ),
                            ));
                        }
                    }

                    // Draw node circle
                    let node_pos = graph_rect.min
                        + egui::vec2(
                            graph_column as f32 * COLUMN_WIDTH + COLUMN_WIDTH / 2.0,
                            NODE_RADIUS,
                        );
                    let node_color = colors[graph_column % colors.len()];
                    painter.circle_filled(node_pos, NODE_RADIUS, node_color);
                }

                // Commit info (right side)
                let _text_color = if is_selected {
                    egui::Color32::from_rgb(0xAB, 0xB2, 0xBF)
                } else {
                    egui::Color32::from_rgb(180, 180, 180)
                };

                if ui
                    .add(egui::Button::new(&commit_message).fill(if is_selected {
                        egui::Color32::from_rgb(60, 60, 80)
                    } else {
                        egui::Color32::TRANSPARENT
                    }))
                    .clicked()
                {
                    self.git_history_state.selected_commit_id = Some(commit_id.clone());
                    // Load commit details
                    if let Ok(detail) = native::git::get_commit_detail(&self.root_path, &commit_id)
                    {
                        self.git_history_state.commit_details = Some(detail);
                    }
                }

                ui.colored_label(egui::Color32::GRAY, &commit_author);

                // Branch/tag badges
                for branch_name in &node.branch_names {
                    ui.colored_label(
                        egui::Color32::from_rgb(106, 180, 89),
                        format!(" [{}]", branch_name),
                    );
                }
                for tag_name in &node.tag_names {
                    ui.colored_label(
                        egui::Color32::from_rgb(255, 198, 109),
                        format!(" 🏷{}", tag_name),
                    );
                }
            });
        }
    }

    /// Render commit details panel
    fn render_commit_details(&mut self, ui: &mut egui::Ui) {
        if let Some(detail) = &self.git_history_state.commit_details {
            ui.heading(self.tr("Commit Details"));
            ui.separator();

            ui.label(format!("ID: {}", detail.commit.id));
            ui.label(format!("Author: {}", detail.commit.author));
            ui.label(format!(
                "Date: {}",
                Self::format_timestamp(detail.commit.date)
            ));
            ui.label(format!("Message: {}", detail.commit.message));

            ui.add_space(8.0);
            ui.label(format!(
                "Stats: +{} -{}",
                detail.total_additions, detail.total_deletions
            ));

            ui.separator();
            ui.heading(format!("Changed Files ({})", detail.changed_files.len()));

            egui::ScrollArea::vertical().show(ui, |ui| {
                for file in &detail.changed_files {
                    let (icon, color) = match file.status.as_str() {
                        "added" => ("➕", egui::Color32::from_rgb(106, 180, 89)),
                        "modified" => ("📝", egui::Color32::from_rgb(255, 198, 109)),
                        "deleted" => ("🗑️", egui::Color32::from_rgb(255, 100, 100)),
                        _ => ("❓", egui::Color32::GRAY),
                    };

                    ui.horizontal(|ui| {
                        ui.colored_label(color, icon);
                        ui.label(&file.path);
                        ui.colored_label(egui::Color32::GREEN, format!("+{}", file.additions));
                        ui.colored_label(egui::Color32::RED, format!("-{}", file.deletions));
                    });
                }
            });
        } else {
            ui.label(self.tr("Select a commit to view details"));
        }
    }

    /// Render Branches tab
    fn render_git_branches_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button(format!("🔄 {}", self.tr("Refresh"))).clicked() {
                self.refresh_git_branches();
            }
        });

        ui.separator();

        // Create new branch
        ui.horizontal(|ui| {
            ui.label("New branch:");
            ui.text_edit_singleline(&mut self.git_branch_state.new_branch_name);
            if ui.button("➕ Create").clicked() && !self.git_branch_state.new_branch_name.is_empty()
            {
                self.perform_create_branch();
            }
        });

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            // Local branches
            ui.heading(format!(
                "Local Branches ({})",
                self.git_branch_state.local_branches.len()
            ));
            ui.add_space(4.0);

            let local_branches = self.git_branch_state.local_branches.clone();
            for branch in &local_branches {
                ui.horizontal(|ui| {
                    let icon = if branch.is_current { "✓" } else { " " };
                    let color = if branch.is_current {
                        egui::Color32::from_rgb(106, 180, 89)
                    } else {
                        egui::Color32::LIGHT_GRAY
                    };

                    ui.colored_label(color, icon);
                    ui.label(&branch.name);

                    if !branch.is_current {
                        if ui.small_button("Checkout").clicked() {
                            self.perform_checkout_branch(&branch.name);
                        }
                        if ui.small_button("Merge").clicked() {
                            self.perform_merge_branch(&branch.name);
                        }
                        if ui.small_button("Delete").clicked() {
                            self.perform_delete_branch(&branch.name);
                        }
                    }
                });
            }

            ui.add_space(8.0);

            // Remote branches
            ui.heading(format!(
                "Remote Branches ({})",
                self.git_branch_state.remote_branches.len()
            ));
            ui.add_space(4.0);

            let remote_branches = self.git_branch_state.remote_branches.clone();
            for branch in &remote_branches {
                ui.horizontal(|ui| {
                    ui.colored_label(egui::Color32::from_rgb(100, 181, 246), "📡");
                    ui.label(&branch.name);
                });
            }
        });
    }

    /// Render Remotes tab
    fn render_git_remotes_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button(format!("🔄 {}", self.tr("Refresh"))).clicked() {
                self.refresh_git_remotes();
            }
        });

        ui.separator();

        // Add new remote
        ui.horizontal(|ui| {
            ui.label("Name:");
            ui.add(
                egui::TextEdit::singleline(&mut self.git_remote_state.new_remote_name)
                    .desired_width(ui.available_width() - 40.0),
            );
        });
        ui.horizontal(|ui| {
            ui.label("URL:");
            ui.add(
                egui::TextEdit::singleline(&mut self.git_remote_state.new_remote_url)
                    .desired_width(ui.available_width() - 40.0),
            );
        });
        if ui.button("➕ Add").clicked() && !self.git_remote_state.new_remote_name.is_empty() {
            self.perform_add_remote();
        }

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            if self.git_remote_state.remotes.is_empty() {
                ui.label("No remotes configured");
            } else {
                let remotes = self.git_remote_state.remotes.clone();
                for remote in &remotes {
                    ui.group(|ui| {
                        ui.heading(&remote.name);
                        ui.label(format!("Fetch URL: {}", remote.fetch_url));
                        ui.label(format!("Push URL: {}", remote.push_url));

                        ui.horizontal(|ui| {
                            if ui.button("Fetch").clicked() {
                                self.perform_fetch(&remote.name);
                            }
                            if ui.button("Pull").clicked() {
                                self.perform_pull(&remote.name);
                            }
                            if ui.button("Push").clicked() {
                                self.perform_push(&remote.name);
                            }
                            if ui.button("Remove").clicked() {
                                self.perform_remove_remote(&remote.name);
                            }
                        });
                    });
                    ui.add_space(8.0);
                }
            }
        });
    }

    /// Render Tags tab
    fn render_git_tags_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button(format!("🔄 {}", self.tr("Refresh"))).clicked() {
                self.refresh_git_tags();
            }
        });

        ui.separator();

        // Create new tag
        ui.horizontal(|ui| {
            ui.label("Tag name:");
            ui.text_edit_singleline(&mut self.git_tag_state.new_tag_name);
            ui.checkbox(&mut self.git_tag_state.annotated, "Annotated");
        });

        if self.git_tag_state.annotated {
            ui.horizontal(|ui| {
                ui.label("Message:");
                ui.text_edit_singleline(&mut self.git_tag_state.new_tag_message);
            });
        }

        if ui.button("➕ Create Tag").clicked() && !self.git_tag_state.new_tag_name.is_empty() {
            self.perform_create_tag();
        }

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            if self.git_tag_state.tags.is_empty() {
                ui.label("No tags");
            } else {
                ui.heading(format!("Tags ({})", self.git_tag_state.tags.len()));
                ui.add_space(4.0);

                let tags = self.git_tag_state.tags.clone();
                for tag in &tags {
                    ui.horizontal(|ui| {
                        ui.colored_label(egui::Color32::from_rgb(255, 198, 109), "🏷");
                        ui.label(&tag.name);

                        if tag.message.is_some() {
                            ui.colored_label(egui::Color32::GRAY, "(annotated)");
                        }

                        ui.label(format!("→ {}", &tag.commit_id[..7]));

                        if ui.small_button("Delete").clicked() {
                            self.perform_delete_tag(&tag.name);
                        }
                    });

                    if let Some(message) = &tag.message {
                        ui.label(format!("  {}", message));
                    }
                }
            }
        });
    }

    /// Render Stash tab
    fn render_git_stash_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button(format!("🔄 {}", self.tr("Refresh"))).clicked() {
                self.refresh_git_stashes();
            }
        });

        ui.separator();

        // Create new stash
        ui.horizontal(|ui| {
            ui.label("Message:");
            ui.text_edit_singleline(&mut self.git_stash_state.new_stash_message);
            ui.checkbox(
                &mut self.git_stash_state.include_untracked,
                "Include untracked",
            );
        });

        if ui.button("💾 Save Stash").clicked() {
            self.perform_stash_save();
        }

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            if self.git_stash_state.stashes.is_empty() {
                ui.label("No stashes");
            } else {
                ui.heading(format!("Stashes ({})", self.git_stash_state.stashes.len()));
                ui.add_space(4.0);

                let stashes = self.git_stash_state.stashes.clone();
                for stash in &stashes {
                    ui.group(|ui| {
                        ui.heading(format!("stash@{{{}}}", stash.index));
                        ui.label(&stash.message);
                        ui.label(format!("Commit: {}", &stash.commit_id[..7]));

                        ui.horizontal(|ui| {
                            if ui.button("Apply").clicked() {
                                self.perform_stash_apply(stash.index);
                            }
                            if ui.button("Pop").clicked() {
                                self.perform_stash_pop(stash.index);
                            }
                            if ui.button("Drop").clicked() {
                                self.perform_stash_drop(stash.index);
                            }
                        });
                    });
                    ui.add_space(8.0);
                }
            }
        });
    }

    /// Helper function to format Unix timestamp
    pub(crate) fn format_timestamp(timestamp: u64) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let duration = std::time::Duration::from_secs(timestamp);
        let datetime = UNIX_EPOCH + duration;

        // Simple formatting (could use chrono crate for better formatting)
        if let Ok(elapsed) = SystemTime::now().duration_since(datetime) {
            let secs = elapsed.as_secs();
            if secs < 60 {
                return format!("{} seconds ago", secs);
            } else if secs < 3600 {
                return format!("{} minutes ago", secs / 60);
            } else if secs < 86400 {
                return format!("{} hours ago", secs / 3600);
            } else {
                return format!("{} days ago", secs / 86400);
            }
        }

        format!("Timestamp: {}", timestamp)
    }

    /// Render Git Diff Viewer in center panel (split view: Graph above, Diff below)
    pub(crate) fn render_git_diff_viewer(&mut self, ctx: &egui::Context) {
        // Top panel: Git commit graph (30% of height)
        egui::TopBottomPanel::top("git_graph_panel")
            .default_height(250.0)
            .resizable(true)
            .frame(
                egui::Frame::NONE
                    .fill(ui_colors::SIDEBAR_BG)
                    .inner_margin(egui::Margin::same(8)),
            )
            .show(ctx, |ui| {
                ui.heading("📊 Commit Graph");
                ui.separator();

                // Render commit graph (reuse existing logic from History tab)
                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        self.render_git_graph_compact(ui);
                    });
            });

        // Bottom panel: Diff viewer
        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(ui_colors::SIDEBAR_BG)
                    .inner_margin(egui::Margin::same(8)),
            )
            .show(ctx, |ui| {
                if let Some(diff) = &self.git_diff_state.diff {
                    let file_path = self
                        .git_diff_state
                        .selected_file
                        .as_ref()
                        .map(|s| s.as_str())
                        .unwrap_or("Unknown file");

                    // Header with file path and status
                    ui.horizontal(|ui| {
                        ui.heading(format!("📝 Diff: {}", file_path));
                        let status_color = match diff.status.as_str() {
                            "added" => egui::Color32::from_rgb(100, 255, 100),
                            "deleted" => egui::Color32::from_rgb(255, 100, 100),
                            "modified" => egui::Color32::from_rgb(100, 180, 255),
                            _ => ui_colors::TEXT_DEFAULT,
                        };
                        ui.label(
                            egui::RichText::new(&diff.status.to_uppercase())
                                .color(status_color)
                                .strong(),
                        );
                    });
                    ui.separator();

                    // Scroll area for diff content
                    egui::ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing.y = 0.0;
                            // Render each hunk
                            for hunk in &diff.hunks {
                                // Hunk header
                                ui.label(
                                    egui::RichText::new(&hunk.header)
                                        .color(egui::Color32::from_rgb(100, 180, 255))
                                        .family(egui::FontFamily::Monospace)
                                        .size(13.0),
                                );

                                ui.add_space(4.0);

                                // Render lines in the hunk
                                let row_height = 18.0;
                                let font = egui::FontId::new(13.0, egui::FontFamily::Monospace);

                                for line in &hunk.lines {
                                    let (bg_color, marker_color, prefix) = match line.origin {
                                        '+' => (
                                            egui::Color32::from_rgb(24, 50, 24),
                                            egui::Color32::from_rgb(80, 180, 80),
                                            "+",
                                        ),
                                        '-' => (
                                            egui::Color32::from_rgb(55, 20, 20),
                                            egui::Color32::from_rgb(200, 70, 70),
                                            "-",
                                        ),
                                        _ => (
                                            egui::Color32::TRANSPARENT,
                                            egui::Color32::from_rgb(60, 60, 60),
                                            " ",
                                        ),
                                    };

                                    let line_num = if line.origin == '-' {
                                        line.old_lineno
                                            .map(|n| format!("{:>4}", n))
                                            .unwrap_or_else(|| "    ".to_string())
                                    } else {
                                        line.new_lineno
                                            .map(|n| format!("{:>4}", n))
                                            .unwrap_or_else(|| "    ".to_string())
                                    };

                                    let content = line.content.trim_end();

                                    let (rect, _) = ui.allocate_exact_size(
                                        egui::vec2(ui.available_width(), row_height),
                                        egui::Sense::hover(),
                                    );

                                    // Full-width background
                                    if bg_color != egui::Color32::TRANSPARENT {
                                        ui.painter().rect_filled(rect, 0.0, bg_color);
                                    }

                                    let y = rect.center().y;
                                    let mut x = rect.left() + 4.0;

                                    // Line number (dim gray)
                                    ui.painter().text(
                                        egui::pos2(x, y),
                                        egui::Align2::LEFT_CENTER,
                                        &line_num,
                                        font.clone(),
                                        egui::Color32::from_rgb(70, 70, 70),
                                    );
                                    x += 40.0;

                                    // +/- marker (colored)
                                    ui.painter().text(
                                        egui::pos2(x, y),
                                        egui::Align2::LEFT_CENTER,
                                        prefix,
                                        font.clone(),
                                        marker_color,
                                    );
                                    x += 16.0;

                                    // Content
                                    ui.painter().text(
                                        egui::pos2(x, y),
                                        egui::Align2::LEFT_CENTER,
                                        content,
                                        font.clone(),
                                        ui_colors::TEXT_DEFAULT,
                                    );
                                }

                                ui.add_space(8.0);
                            }
                        });
                } else {
                    // No diff selected
                    ui.vertical_centered(|ui| {
                        ui.add_space(100.0);
                        ui.label("📝 Select a file to view diff");
                    });
                }
            });
    }

    /// Render compact Git commit graph (for split view)
    pub(crate) fn render_git_graph_compact(&mut self, ui: &mut egui::Ui) {
        if self.git_history_state.graph_nodes.is_empty() {
            ui.label("No commits. Click Refresh in Git Status tab.");
            return;
        }

        const NODE_RADIUS: f32 = 4.0;
        const COLUMN_WIDTH: f32 = 16.0;
        const ROW_HEIGHT: f32 = 24.0;

        // 8-color palette for branches (same as full graph)
        let colors = [
            egui::Color32::from_rgb(106, 180, 89),  // Green
            egui::Color32::from_rgb(100, 181, 246), // Blue
            egui::Color32::from_rgb(255, 198, 109), // Yellow
            egui::Color32::from_rgb(239, 83, 80),   // Red
            egui::Color32::from_rgb(171, 128, 255), // Purple
            egui::Color32::from_rgb(255, 138, 128), // Coral
            egui::Color32::from_rgb(128, 222, 234), // Cyan
            egui::Color32::from_rgb(255, 171, 64),  // Orange
        ];

        // Display recent 10 commits in compact form with graph
        for node in self.git_history_state.graph_nodes.iter().take(10) {
            ui.horizontal(|ui| {
                // Graph column (left side)
                let (graph_rect, _graph_response) = ui.allocate_exact_size(
                    egui::vec2(COLUMN_WIDTH * 8.0, ROW_HEIGHT),
                    egui::Sense::hover(),
                );

                if ui.is_rect_visible(graph_rect) {
                    let painter = ui.painter();

                    // Draw graph lines
                    for line in &node.graph_lines {
                        let from_pos = graph_rect.min
                            + egui::vec2(
                                line.from_column as f32 * COLUMN_WIDTH + COLUMN_WIDTH / 2.0,
                                NODE_RADIUS,
                            );
                        let to_pos = graph_rect.min
                            + egui::vec2(
                                line.to_column as f32 * COLUMN_WIDTH + COLUMN_WIDTH / 2.0,
                                ROW_HEIGHT,
                            );

                        let color = colors[line.color_index % colors.len()];

                        if line.line_type == native::git::GraphLineType::Direct {
                            // Straight line
                            painter.line_segment([from_pos, to_pos], egui::Stroke::new(2.0, color));
                        } else {
                            // Bezier curve for merge
                            painter.add(egui::Shape::CubicBezier(
                                egui::epaint::CubicBezierShape::from_points_stroke(
                                    [
                                        from_pos,
                                        from_pos + egui::vec2(0.0, ROW_HEIGHT * 0.3),
                                        to_pos - egui::vec2(0.0, ROW_HEIGHT * 0.3),
                                        to_pos,
                                    ],
                                    false,
                                    egui::Color32::TRANSPARENT,
                                    egui::Stroke::new(2.0, color),
                                ),
                            ));
                        }
                    }

                    // Draw node circle
                    let node_pos = graph_rect.min
                        + egui::vec2(
                            node.graph_column as f32 * COLUMN_WIDTH + COLUMN_WIDTH / 2.0,
                            NODE_RADIUS,
                        );
                    let node_color = colors[node.graph_column % colors.len()];
                    painter.circle_filled(node_pos, NODE_RADIUS, node_color);
                }

                // Commit message (truncated)
                let msg = node.commit.message.lines().next().unwrap_or("");
                let truncated = if msg.len() > 40 {
                    format!("{}...", &msg[..40])
                } else {
                    msg.to_string()
                };
                ui.label(
                    egui::RichText::new(truncated)
                        .family(egui::FontFamily::Monospace)
                        .size(12.0),
                );

                // Author
                ui.label(
                    egui::RichText::new(&node.commit.author)
                        .color(egui::Color32::from_rgb(150, 150, 150))
                        .size(11.0),
                );
            });
        }
    }

    // ===== Git Operations =====

    /// Refresh Git status (branch and changed files)
    pub(crate) fn refresh_git_status(&mut self) {
        tracing::info!("🔀 Refreshing Git status for {}", self.root_path);

        // Get current branch
        match native::git::get_current_branch(&self.root_path) {
            Ok(branch) => {
                self.git_current_branch = branch;
                tracing::info!("✅ Current branch: {}", self.git_current_branch);
            }
            Err(e) => {
                tracing::error!("❌ Failed to get current branch: {}", e);
                self.git_current_branch = "(error)".to_string();
            }
        }

        // Get file status
        match native::git::get_status(&self.root_path) {
            Ok(status) => {
                self.git_status = status;
                tracing::info!(
                    "✅ Git status loaded: {} files changed",
                    self.git_status.len()
                );
            }
            Err(e) => {
                tracing::error!("❌ Failed to get Git status: {}", e);
                self.git_status.clear();
            }
        }
    }

    /// Refresh Git history (load commit graph)
    pub(crate) fn refresh_git_history(&mut self) {
        tracing::info!("🔀 Refreshing Git history for {}", self.root_path);

        match native::git::get_detailed_log(
            &self.root_path,
            self.git_history_state.page_limit,
            self.git_history_state.show_all_branches,
        ) {
            Ok(nodes) => {
                self.git_history_state.graph_nodes = nodes;
                self.git_history_state.loaded_count = self.git_history_state.graph_nodes.len();
                tracing::info!(
                    "✅ Git history loaded: {} commits",
                    self.git_history_state.loaded_count
                );
            }
            Err(e) => {
                tracing::error!("❌ Failed to load Git history: {}", e);
                self.git_history_state.graph_nodes.clear();
            }
        }
    }

    /// Refresh Git branches
    pub(crate) fn refresh_git_branches(&mut self) {
        match native::git::list_branches(&self.root_path) {
            Ok(branches) => {
                self.git_branch_state.local_branches = branches;
            }
            Err(e) => {
                tracing::error!("❌ Failed to load branches: {}", e);
            }
        }

        match native::git::list_remote_branches(&self.root_path) {
            Ok(branches) => {
                self.git_branch_state.remote_branches = branches;
            }
            Err(e) => {
                tracing::error!("❌ Failed to load remote branches: {}", e);
            }
        }
    }

    /// Refresh Git remotes
    pub(crate) fn refresh_git_remotes(&mut self) {
        match native::git::list_remotes(&self.root_path) {
            Ok(remotes) => {
                self.git_remote_state.remotes = remotes;
            }
            Err(e) => {
                tracing::error!("❌ Failed to load remotes: {}", e);
            }
        }
    }

    /// Refresh Git tags
    pub(crate) fn refresh_git_tags(&mut self) {
        match native::git::list_tags(&self.root_path) {
            Ok(tags) => {
                self.git_tag_state.tags = tags;
            }
            Err(e) => {
                tracing::error!("❌ Failed to load tags: {}", e);
            }
        }
    }

    /// Refresh Git stashes
    pub(crate) fn refresh_git_stashes(&mut self) {
        match native::git::list_stashes(&self.root_path) {
            Ok(stashes) => {
                self.git_stash_state.stashes = stashes;
            }
            Err(e) => {
                tracing::error!("❌ Failed to load stashes: {}", e);
            }
        }
    }

    // ===== Branch Actions =====

    fn perform_create_branch(&mut self) {
        let branch_name = self.git_branch_state.new_branch_name.clone();
        match native::git::create_branch(&self.root_path, &branch_name) {
            Ok(_) => {
                tracing::info!("✅ Created branch: {}", branch_name);
                self.git_branch_state.new_branch_name.clear();
                self.refresh_git_branches();
            }
            Err(e) => {
                tracing::error!("❌ Failed to create branch: {}", e);
            }
        }
    }

    fn perform_checkout_branch(&mut self, branch_name: &str) {
        match native::git::checkout_branch(&self.root_path, branch_name) {
            Ok(_) => {
                tracing::info!("✅ Checked out branch: {}", branch_name);
                self.refresh_git_branches();
                self.refresh_git_status();
            }
            Err(e) => {
                tracing::error!("❌ Failed to checkout branch: {}", e);
            }
        }
    }

    fn perform_delete_branch(&mut self, branch_name: &str) {
        match native::git::delete_branch(&self.root_path, branch_name) {
            Ok(_) => {
                tracing::info!("✅ Deleted branch: {}", branch_name);
                self.refresh_git_branches();
            }
            Err(e) => {
                tracing::error!("❌ Failed to delete branch: {}", e);
            }
        }
    }

    fn perform_merge_branch(&mut self, branch_name: &str) {
        match native::git::merge_branch(&self.root_path, branch_name) {
            Ok(_) => {
                tracing::info!("✅ Merged branch: {}", branch_name);
                self.refresh_git_status();
                self.refresh_git_history();
            }
            Err(e) => {
                tracing::error!("❌ Failed to merge branch: {}", e);
            }
        }
    }

    // ===== Remote Actions =====

    fn perform_add_remote(&mut self) {
        let name = self.git_remote_state.new_remote_name.clone();
        let url = self.git_remote_state.new_remote_url.clone();

        match native::git::add_remote(&self.root_path, &name, &url) {
            Ok(_) => {
                tracing::info!("✅ Added remote: {} -> {}", name, url);
                self.git_remote_state.new_remote_name.clear();
                self.git_remote_state.new_remote_url.clear();
                self.refresh_git_remotes();
            }
            Err(e) => {
                tracing::error!("❌ Failed to add remote: {}", e);
            }
        }
    }

    fn perform_remove_remote(&mut self, name: &str) {
        match native::git::remove_remote(&self.root_path, name) {
            Ok(_) => {
                tracing::info!("✅ Removed remote: {}", name);
                self.refresh_git_remotes();
            }
            Err(e) => {
                tracing::error!("❌ Failed to remove remote: {}", e);
            }
        }
    }

    fn perform_fetch(&mut self, remote_name: &str) {
        match native::git::fetch(&self.root_path, remote_name) {
            Ok(_) => {
                tracing::info!("✅ Fetched from remote: {}", remote_name);
                self.refresh_git_branches();
            }
            Err(e) => {
                tracing::error!("❌ Failed to fetch: {}", e);
            }
        }
    }

    fn perform_pull(&mut self, remote_name: &str) {
        let branch_name = self.git_current_branch.clone();
        match native::git::pull(&self.root_path, remote_name, &branch_name) {
            Ok(_) => {
                tracing::info!("✅ Pulled from remote: {}", remote_name);
                self.refresh_git_status();
                self.refresh_git_history();
            }
            Err(e) => {
                tracing::error!("❌ Failed to pull: {}", e);
            }
        }
    }

    fn perform_push(&mut self, remote_name: &str) {
        let branch_name = self.git_current_branch.clone();
        let refspec = format!("refs/heads/{}:refs/heads/{}", branch_name, branch_name);
        match native::git::push(&self.root_path, remote_name, &[&refspec]) {
            Ok(_) => {
                tracing::info!("✅ Pushed to remote: {}", remote_name);
            }
            Err(e) => {
                tracing::error!("❌ Failed to push: {}", e);
            }
        }
    }

    // ===== Tag Actions =====

    fn perform_create_tag(&mut self) {
        let tag_name = self.git_tag_state.new_tag_name.clone();
        let result = if self.git_tag_state.annotated {
            let message = self.git_tag_state.new_tag_message.clone();
            native::git::create_annotated_tag(&self.root_path, &tag_name, &message, None)
        } else {
            native::git::create_tag(&self.root_path, &tag_name, None)
        };

        match result {
            Ok(_) => {
                tracing::info!("✅ Created tag: {}", tag_name);
                self.git_tag_state.new_tag_name.clear();
                self.git_tag_state.new_tag_message.clear();
                self.refresh_git_tags();
            }
            Err(e) => {
                tracing::error!("❌ Failed to create tag: {}", e);
            }
        }
    }

    fn perform_delete_tag(&mut self, tag_name: &str) {
        match native::git::delete_tag(&self.root_path, tag_name) {
            Ok(_) => {
                tracing::info!("✅ Deleted tag: {}", tag_name);
                self.refresh_git_tags();
            }
            Err(e) => {
                tracing::error!("❌ Failed to delete tag: {}", e);
            }
        }
    }

    // ===== Stash Actions =====

    fn perform_stash_save(&mut self) {
        let message = if self.git_stash_state.new_stash_message.is_empty() {
            None
        } else {
            Some(self.git_stash_state.new_stash_message.as_str())
        };

        match native::git::stash_save(
            &self.root_path,
            message,
            self.git_stash_state.include_untracked,
        ) {
            Ok(_) => {
                tracing::info!("✅ Saved stash");
                self.git_stash_state.new_stash_message.clear();
                self.refresh_git_stashes();
                self.refresh_git_status();
            }
            Err(e) => {
                tracing::error!("❌ Failed to save stash: {}", e);
            }
        }
    }

    fn perform_stash_apply(&mut self, index: usize) {
        match native::git::stash_apply(&self.root_path, index) {
            Ok(_) => {
                tracing::info!("✅ Applied stash@{}", index);
                self.refresh_git_status();
            }
            Err(e) => {
                tracing::error!("❌ Failed to apply stash: {}", e);
            }
        }
    }

    fn perform_stash_pop(&mut self, index: usize) {
        match native::git::stash_pop(&self.root_path, index) {
            Ok(_) => {
                tracing::info!("✅ Popped stash@{}", index);
                self.refresh_git_stashes();
                self.refresh_git_status();
            }
            Err(e) => {
                tracing::error!("❌ Failed to pop stash: {}", e);
            }
        }
    }

    fn perform_stash_drop(&mut self, index: usize) {
        match native::git::stash_drop(&self.root_path, index) {
            Ok(_) => {
                tracing::info!("✅ Dropped stash@{}", index);
                self.refresh_git_stashes();
            }
            Err(e) => {
                tracing::error!("❌ Failed to drop stash: {}", e);
            }
        }
    }

    /// Stage a file
    pub(crate) fn perform_git_stage(&mut self, file_path: &str) {
        tracing::info!("🔀 Staging file: {}", file_path);

        match native::git::stage_file(&self.root_path, file_path) {
            Ok(_) => {
                tracing::info!("✅ File staged: {}", file_path);
                self.refresh_git_status();
            }
            Err(e) => {
                tracing::error!("❌ Failed to stage file: {}", e);
            }
        }
    }

    /// Unstage a file
    pub(crate) fn perform_git_unstage(&mut self, file_path: &str) {
        tracing::info!("🔀 Unstaging file: {}", file_path);

        match native::git::unstage_file(&self.root_path, file_path) {
            Ok(_) => {
                tracing::info!("✅ File unstaged: {}", file_path);
                self.refresh_git_status();
            }
            Err(e) => {
                tracing::error!("❌ Failed to unstage file: {}", e);
            }
        }
    }

    /// Stage all files
    pub(crate) fn perform_git_stage_all(&mut self) {
        tracing::info!("🔀 Staging all files");

        match native::git::stage_all(&self.root_path) {
            Ok(_) => {
                tracing::info!("✅ All files staged");
                self.refresh_git_status();
            }
            Err(e) => {
                tracing::error!("❌ Failed to stage all: {}", e);
            }
        }
    }

    /// Create a commit
    pub(crate) fn perform_git_commit(&mut self) {
        if self.git_commit_message.trim().is_empty() {
            tracing::warn!("⚠️  Cannot commit with empty message");
            return;
        }

        tracing::info!("🔀 Creating commit: {}", self.git_commit_message);

        match native::git::commit(&self.root_path, &self.git_commit_message) {
            Ok(commit_id) => {
                tracing::info!("✅ Commit created: {}", commit_id);
                self.git_commit_message.clear();
                self.refresh_git_status();
            }
            Err(e) => {
                tracing::error!("❌ Failed to commit: {}", e);
            }
        }
    }
}
