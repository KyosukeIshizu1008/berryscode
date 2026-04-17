//! Status bar rendering

use super::BerryCodeApp;
use super::ui_colors;

impl BerryCodeApp {
    /// Render Status Bar (bottom)
    pub(crate) fn render_status_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status_bar")
            .exact_height(24.0)
            .frame(
                egui::Frame::none()
                    .fill(ui_colors::SIDEBAR_BG) // #191A1C
                    .inner_margin(egui::Margin::symmetric(8.0, 2.0))
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("BerryEditor");
                    ui.separator();
                    ui.label(format!("📁 {}", self.root_path));
                    ui.separator();
                    ui.label(format!("ファイル数: {}", self.file_tree_cache.len()));

                    // LSP connection status
                    ui.separator();
                    let status_text = if self.lsp_connected {
                        "🟢 LSP: Connected | F12: Definition | Shift+F12: References | Cmd+Click: Jump"
                    } else {
                        "🔴 LSP: Disconnected | Regex search only"
                    };
                    ui.label(status_text);

                    // Diagnostics count
                    if !self.lsp_diagnostics.is_empty() {
                        ui.separator();
                        ui.label(format!("⚠️ {}", self.lsp_diagnostics.len()));
                    }

                    // Status message display (auto-clear after 3 seconds)
                    if !self.status_message.is_empty() {
                        if let Some(timestamp) = self.status_message_timestamp {
                            if timestamp.elapsed().as_secs() < 3 {
                                ui.separator();
                                ui.label(&self.status_message);
                            } else {
                                self.status_message.clear();
                                self.status_message_timestamp = None;
                            }
                        }
                    }

                    // Read-only warning
                    if let Some(tab) = self.editor_tabs.get(self.active_tab_idx) {
                        if tab.is_readonly {
                            ui.separator();
                            ui.label(egui::RichText::new("📖 READ-ONLY")
                                .color(egui::Color32::from_rgb(255, 200, 0)));
                        }

                        ui.separator();

                        // Language indicator
                        let lang = if tab.file_path.ends_with(".rs") {
                            "Rust"
                        } else if tab.file_path.ends_with(".toml") {
                            "TOML"
                        } else if tab.file_path.ends_with(".md") {
                            "Markdown"
                        } else {
                            "Plain Text"
                        };
                        ui.label(format!("言語: {}", lang));

                        // Format button (only for supported languages)
                        if tab.file_path.ends_with(".rs") {
                            ui.separator();
                            if ui.button("Format (Cmd+Shift+F)").clicked() {
                                self.format_current_file();
                            }
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label("egui 0.29 + Native");
                    });
                });
            });
    }
}
