//! Status bar rendering

use super::ui_colors;
use super::BerryCodeApp;

impl BerryCodeApp {
    /// Render Status Bar (bottom)
    pub(crate) fn render_status_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status_bar")
            .exact_height(22.0)
            .frame(
                egui::Frame::NONE
                    .fill(ui_colors::STATUS_BAR_BG)
                    .inner_margin(egui::Margin::symmetric(8, 2)),
            )
            .show(ctx, |ui| {
                let small = egui::TextStyle::Small;
                ui.style_mut()
                    .text_styles
                    .insert(small.clone(), egui::FontId::proportional(11.0));

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;

                    // Left side
                    let lsp_label = if self.lsp_connected { "LSP" } else { "LSP off" };
                    ui.label(egui::RichText::new(lsp_label).color(ui_colors::TEXT_DEFAULT));

                    let rs_diag_count =
                        super::utils::filter_rust_diagnostics(&self.lsp_diagnostics).len();
                    if rs_diag_count > 0 {
                        ui.label(
                            egui::RichText::new(format!("⚠ {}", rs_diag_count))
                                .small()
                                .color(egui::Color32::from_rgb(255, 200, 80)),
                        );
                    }

                    ui.separator();

                    // Godot project badge — shows whenever the workspace
                    // root contains a `project.godot`. Cheap to recompute
                    // each frame (`is_file()` syscall) and gives migrating
                    // users an instant visual confirmation that BerryCode
                    // recognises their project type. (v0.8.x Migration & interop)
                    if !self.root_path.is_empty()
                        && crate::godot_import::is_godot_project(std::path::Path::new(
                            &self.root_path,
                        ))
                    {
                        ui.label(
                            egui::RichText::new("● Godot Project")
                                .small()
                                .color(super::file_icon_colors::GODOT_SCRIPT_BLUE),
                        );
                        ui.separator();
                    }

                    // Current file language
                    if let Some(tab) = self.editor_tabs.get(self.active_tab_idx) {
                        let lang = if tab.file_path.ends_with(".rs") {
                            "Rust"
                        } else if tab.file_path.ends_with(".toml") {
                            "TOML"
                        } else if tab.file_path.ends_with(".md") {
                            "Markdown"
                        } else if tab.file_path.ends_with(".json") {
                            "JSON"
                        } else if tab.file_path.ends_with(".gd") {
                            "GDScript"
                        } else if tab.file_path.ends_with(".tscn") {
                            "Godot Scene"
                        } else if tab.file_path.ends_with(".tres")
                            || tab.file_path.ends_with(".res")
                        {
                            "Godot Resource"
                        } else if tab.file_path.ends_with("project.godot") {
                            "Godot Project"
                        } else {
                            self.tr("Plain Text")
                        };
                        ui.label(egui::RichText::new(lang).small());

                        if tab.is_readonly {
                            ui.label(
                                egui::RichText::new(self.tr("READ-ONLY"))
                                    .small()
                                    .color(egui::Color32::from_rgb(255, 200, 0)),
                            );
                        }
                    }

                    // Status message (auto-clear after 3 seconds)
                    if !self.status_message.is_empty() {
                        if let Some(timestamp) = self.status_message_timestamp {
                            if timestamp.elapsed().as_secs() < 3 {
                                ui.separator();
                                ui.label(egui::RichText::new(&self.status_message).small());
                            } else {
                                self.status_message.clear();
                                self.status_message_timestamp = None;
                            }
                        }
                    }

                    // Right side
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new("Bevy 0.18 + WGPU").small());
                    });
                });
            });
    }
}
