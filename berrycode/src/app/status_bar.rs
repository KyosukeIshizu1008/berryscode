//! Status bar rendering

use super::ui_colors;
use super::BerryCodeApp;
use crate::app::i18n::t;

impl BerryCodeApp {
    /// Render Status Bar (bottom)
    pub(crate) fn render_status_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status_bar")
            .exact_height(22.0)
            .frame(
                egui::Frame::none()
                    .fill(ui_colors::SIDEBAR_BG)
                    .inner_margin(egui::Margin::symmetric(8.0, 2.0)),
            )
            .show(ctx, |ui| {
                let small = egui::TextStyle::Small;
                ui.style_mut()
                    .text_styles
                    .insert(small.clone(), egui::FontId::proportional(11.0));

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;

                    // Left side
                    let lsp_color = if self.lsp_connected {
                        egui::Color32::from_rgb(80, 200, 80)
                    } else {
                        egui::Color32::from_rgb(180, 80, 80)
                    };
                    let lsp_label = if self.lsp_connected { "LSP" } else { "LSP off" };
                    ui.colored_label(lsp_color, lsp_label);

                    if !self.lsp_diagnostics.is_empty() {
                        ui.label(
                            egui::RichText::new(format!("⚠ {}", self.lsp_diagnostics.len()))
                                .small()
                                .color(egui::Color32::from_rgb(255, 200, 80)),
                        );
                    }

                    ui.separator();

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
                        } else {
                            t(self.ui_language, "Plain Text")
                        };
                        ui.label(egui::RichText::new(lang).small());

                        if tab.is_readonly {
                            ui.label(
                                egui::RichText::new(t(self.ui_language, "READ-ONLY"))
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
                        ui.label(egui::RichText::new("Bevy 0.15 + WGPU").small());
                    });
                });
            });
    }
}
