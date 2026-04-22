//! Top header and activity bar rendering

use super::types::ActivePanel;
use super::ui_colors;
use super::BerryCodeApp;
use super::MAIN_PANELS;
use crate::app::i18n::t;

impl BerryCodeApp {
    /// Render top header bar (tab bar under native title)
    pub(crate) fn render_top_header(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_header")
            .exact_height(32.0)
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(50, 51, 54)) // Dark gray background #323336
                    .inner_margin(egui::Margin::symmetric(8.0, 4.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(4.0);

                    // Purple tab with project info
                    let tab_rect_size = egui::vec2(160.0, 24.0);
                    let (tab_rect, _response) =
                        ui.allocate_exact_size(tab_rect_size, egui::Sense::click());

                    // Draw purple background
                    ui.painter().rect_filled(
                        tab_rect,
                        4.0,                                   // Rounded corners
                        egui::Color32::from_rgb(126, 89, 161), // Purple #7E59A1
                    );

                    // Draw badge with "0"
                    let badge_center = egui::pos2(tab_rect.left() + 16.0, tab_rect.center().y);
                    ui.painter().circle_filled(
                        badge_center,
                        9.0,
                        egui::Color32::from_rgba_premultiplied(255, 255, 255, 60),
                    );
                    ui.painter().text(
                        badge_center,
                        egui::Align2::CENTER_CENTER,
                        "0",
                        egui::FontId::proportional(12.0),
                        egui::Color32::WHITE,
                    );

                    // Project name
                    let project_name = self.root_path.split('/').last().unwrap_or("oracleberry");

                    let text_pos = egui::pos2(tab_rect.left() + 34.0, tab_rect.center().y);
                    ui.painter().text(
                        text_pos,
                        egui::Align2::LEFT_CENTER,
                        project_name,
                        egui::FontId::proportional(12.0),
                        egui::Color32::WHITE,
                    );

                    // Dropdown arrow
                    let arrow_pos = egui::pos2(tab_rect.right() - 12.0, tab_rect.center().y);
                    ui.painter().text(
                        arrow_pos,
                        egui::Align2::CENTER_CENTER,
                        "▼",
                        egui::FontId::proportional(9.0),
                        egui::Color32::from_rgb(200, 200, 200),
                    );

                    ui.add_space(16.0);

                    // Close Project button (return to picker)
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new(t(self.ui_language, "Close Project"))
                                    .size(12.0)
                                    .color(egui::Color32::from_rgb(180, 180, 180)),
                            )
                            .frame(false),
                        )
                        .clicked()
                    {
                        self.show_project_picker = true;
                        self.editor_tabs.clear();
                        self.active_tab_idx = 0;
                        self.file_tree_cache.clear();
                        self.root_path.clear();
                    }

                    ui.add_space(4.0);

                    // New Project button
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new(t(self.ui_language, "+ New Bevy Project"))
                                    .size(12.0)
                                    .color(egui::Color32::from_rgb(200, 200, 200)),
                            )
                            .frame(false),
                        )
                        .clicked()
                    {
                        self.new_project_dialog_open = true;
                    }

                    ui.add_space(8.0);

                    // Run Bevy Project button + Release toggle
                    let is_bevy_project = std::path::Path::new(&self.root_path)
                        .join("Cargo.toml")
                        .exists();
                    if is_bevy_project {
                        let is_running = self.run_process.is_some();
                        let (label, color) = if is_running {
                            (
                                t(self.ui_language, "Stop"),
                                egui::Color32::from_rgb(255, 100, 100),
                            )
                        } else {
                            (
                                t(self.ui_language, "Run"),
                                egui::Color32::from_rgb(120, 220, 120),
                            )
                        };

                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new(label).size(12.0).color(color),
                                )
                                .frame(false),
                            )
                            .clicked()
                        {
                            if is_running {
                                self.stop_run();
                                self.game_view_open = false;
                            } else {
                                self.start_run();
                                self.open_game_view();
                                self.tool_panel_open = true;
                            }
                        }

                        // Release mode toggle
                        let mode_label = if self.run_release_mode {
                            t(self.ui_language, "Release")
                        } else {
                            t(self.ui_language, "Debug")
                        };
                        let mode_color = if self.run_release_mode {
                            egui::Color32::from_rgb(255, 180, 80)
                        } else {
                            egui::Color32::from_rgb(150, 150, 150)
                        };
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new(mode_label).size(10.0).color(mode_color),
                                )
                                .frame(false),
                            )
                            .clicked()
                        {
                            self.run_release_mode = !self.run_release_mode;
                        }
                    }

                    ui.add_space(8.0);

                    // Build Settings button (Phase 18)
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new(t(self.ui_language, "Build Settings"))
                                    .size(12.0)
                                    .color(egui::Color32::from_rgb(200, 200, 200)),
                            )
                            .frame(false),
                        )
                        .clicked()
                    {
                        self.build_settings_open = true;
                    }
                });
            });
    }

    /// Render Activity Bar (left-most 48px panel with icons)
    pub(crate) fn render_activity_bar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("activity_bar")
            .exact_width(48.0)
            .resizable(false)
            .frame(
                egui::Frame::none()
                    .fill(ui_colors::SIDEBAR_BG) // #191A1C
                    .inner_margin(egui::Margin::same(4.0)),
            )
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(8.0);

                    // Increase icon size for Activity Bar
                    ui.style_mut().text_styles.insert(
                        egui::TextStyle::Button,
                        egui::FontId::proportional(20.0), // Increased from default
                    );

                    let icon_size = 20.0;
                    let btn_size = egui::vec2(40.0, 36.0);
                    let active_bar_color = egui::Color32::from_rgb(255, 255, 255);
                    let icon_active = egui::Color32::from_rgb(255, 255, 255);
                    let icon_inactive = egui::Color32::from_rgb(120, 120, 120);
                    let hover_bg = egui::Color32::from_rgb(45, 47, 50);

                    for panel in MAIN_PANELS {
                        let is_selected = self.active_panel == panel.variant;

                        let (rect, response) =
                            ui.allocate_exact_size(btn_size, egui::Sense::click());

                        // Hover background
                        if response.hovered() && !is_selected {
                            ui.painter().rect_filled(rect, 0.0, hover_bg);
                        }

                        // Active indicator (left white bar, VS Code style)
                        if is_selected {
                            let bar = egui::Rect::from_min_size(
                                egui::pos2(rect.left(), rect.top() + 6.0),
                                egui::vec2(2.0, rect.height() - 12.0),
                            );
                            ui.painter().rect_filled(bar, 1.0, active_bar_color);
                        }

                        // Icon
                        let color = if is_selected {
                            icon_active
                        } else {
                            icon_inactive
                        };
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            panel.icon,
                            egui::FontId::new(icon_size, egui::FontFamily::Name("codicon".into())),
                            color,
                        );

                        if response.clicked() {
                            self.active_panel = panel.variant;
                        }

                        ui.add_space(2.0);
                    }

                    // Push settings icon to bottom
                    let remaining = ui.available_height() - 40.0;
                    if remaining > 0.0 {
                        ui.add_space(remaining);
                    }

                    // Settings gear icon at bottom
                    let is_settings = self.active_panel == ActivePanel::Settings;
                    let (rect, response) = ui.allocate_exact_size(btn_size, egui::Sense::click());
                    if response.hovered() && !is_settings {
                        ui.painter().rect_filled(rect, 0.0, hover_bg);
                    }
                    if is_settings {
                        let bar = egui::Rect::from_min_size(
                            egui::pos2(rect.left(), rect.top() + 6.0),
                            egui::vec2(2.0, rect.height() - 12.0),
                        );
                        ui.painter().rect_filled(bar, 1.0, active_bar_color);
                    }
                    let gear_color = if is_settings {
                        icon_active
                    } else {
                        icon_inactive
                    };
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "\u{eb51}", // codicon-gear
                        egui::FontId::new(icon_size, egui::FontFamily::Name("codicon".into())),
                        gear_color,
                    );
                    if response.clicked() {
                        self.active_panel = ActivePanel::Settings;
                    }
                });
            });
    }
}
