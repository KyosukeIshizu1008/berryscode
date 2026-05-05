//! Top header and activity bar rendering

use super::types::ActivePanel;
use super::ui_colors;
use super::BerryCodeApp;
use super::MAIN_PANELS;

impl BerryCodeApp {
    /// Render top header bar (tab bar under native title)
    pub(crate) fn render_top_header(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_header")
            .exact_height(32.0)
            .frame(
                egui::Frame::NONE
                    .fill(ui_colors::TOP_BAR_BG)
                    .inner_margin(egui::Margin::symmetric(8, 4)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(4.0);

                    // Compact project switcher, styled like VS Code's title bar command area.
                    let tab_rect_size = egui::vec2(160.0, 24.0);
                    let (tab_rect, _response) =
                        ui.allocate_exact_size(tab_rect_size, egui::Sense::click());

                    ui.painter()
                        .rect_filled(tab_rect, 3.0, egui::Color32::from_rgb(71, 71, 71));
                    ui.painter().rect_stroke(
                        tab_rect,
                        3.0,
                        egui::Stroke::new(1.0, ui_colors::CONTROL_BORDER),
                        egui::StrokeKind::Inside,
                    );

                    // Draw badge with "0"
                    let badge_center = egui::pos2(tab_rect.left() + 16.0, tab_rect.center().y);
                    ui.painter()
                        .circle_filled(badge_center, 9.0, ui_colors::ACCENT);
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
                        ui_colors::TEXT_DEFAULT,
                    );

                    // Dropdown arrow
                    let arrow_pos = egui::pos2(tab_rect.right() - 12.0, tab_rect.center().y);
                    ui.painter().text(
                        arrow_pos,
                        egui::Align2::CENTER_CENTER,
                        "▼",
                        egui::FontId::proportional(9.0),
                        ui_colors::TEXT_MUTED,
                    );

                    ui.add_space(16.0);

                    // Close Project button (return to picker)
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new(self.tr("Close Project"))
                                    .size(12.0)
                                    .color(ui_colors::TEXT_MUTED),
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
                                egui::RichText::new(self.tr("+ New Bevy Project"))
                                    .size(12.0)
                                    .color(ui_colors::TEXT_DEFAULT),
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
                            (self.tr("Stop"), egui::Color32::from_rgb(255, 100, 100))
                        } else {
                            (self.tr("Run"), egui::Color32::from_rgb(120, 220, 120))
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
                            } else {
                                self.start_run();
                                self.tool_panel_open = true;
                            }
                        }

                        // Release mode toggle
                        let mode_label = if self.run_release_mode {
                            self.tr("Release")
                        } else {
                            self.tr("Debug")
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

                    // Build Settings button
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new(self.tr("Build Settings"))
                                    .size(12.0)
                                    .color(ui_colors::TEXT_DEFAULT),
                            )
                            .frame(false),
                        )
                        .clicked()
                    {
                        self.build_settings_open = true;
                    }

                    ui.add_space(4.0);

                    // Packages button
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new(self.tr("Packages"))
                                    .size(12.0)
                                    .color(ui_colors::TEXT_DEFAULT),
                            )
                            .frame(false),
                        )
                        .clicked()
                    {
                        self.package_manager_open = !self.package_manager_open;
                    }

                    ui.add_space(4.0);

                    // Mobile Toolchain button (v0.8 Phase A)
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new(self.tr("Mobile"))
                                    .size(12.0)
                                    .color(ui_colors::TEXT_DEFAULT),
                            )
                            .frame(false),
                        )
                        .clicked()
                    {
                        self.mobile_toolchain_open = !self.mobile_toolchain_open;
                    }
                });
            });
    }

    /// Lazily rasterise the Scene View dove SVG and upload it to an
    /// egui texture. The handle is cached on `self` so we only pay the
    /// SVG decode + raster cost once per session. The SVG uses
    /// `currentColor` for its fill so the activity bar's tint
    /// (icon_active / icon_inactive) applies directly.
    pub(crate) fn scene_view_icon_texture(
        &mut self,
        ctx: &egui::Context,
        size_px: u32,
    ) -> Option<egui::TextureHandle> {
        if let Some(tex) = &self.scene_view_icon {
            return Some(tex.clone());
        }
        const SVG_BYTES: &str = include_str!("../../assets/icons/scene_view.svg");
        let handle = rasterise_svg(ctx, "scene_view_icon", SVG_BYTES, size_px)?;
        self.scene_view_icon = Some(handle.clone());
        Some(handle)
    }

    pub(crate) fn database_icon_texture(
        &mut self,
        ctx: &egui::Context,
        size_px: u32,
    ) -> Option<egui::TextureHandle> {
        if let Some(tex) = &self.database_icon {
            return Some(tex.clone());
        }
        const SVG_BYTES: &str = include_str!("../../assets/icons/database.svg");
        let handle = rasterise_svg(ctx, "database_icon", SVG_BYTES, size_px)?;
        self.database_icon = Some(handle.clone());
        Some(handle)
    }

    pub(crate) fn docker_icon_texture(
        &mut self,
        ctx: &egui::Context,
        size_px: u32,
    ) -> Option<egui::TextureHandle> {
        if let Some(tex) = &self.docker_icon {
            return Some(tex.clone());
        }
        const SVG_BYTES: &str = include_str!("../../assets/icons/whale.svg");
        let handle = rasterise_svg(ctx, "docker_icon", SVG_BYTES, size_px)?;
        self.docker_icon = Some(handle.clone());
        Some(handle)
    }

    pub(crate) fn oracleberry_icon_texture(
        &mut self,
        ctx: &egui::Context,
        size_px: u32,
    ) -> Option<egui::TextureHandle> {
        if let Some(tex) = &self.oracleberry_icon {
            return Some(tex.clone());
        }
        const SVG_BYTES: &str = include_str!("../../assets/icons/oracleberry.svg");
        let handle = rasterise_svg(ctx, "oracleberry_icon", SVG_BYTES, size_px)?;
        self.oracleberry_icon = Some(handle.clone());
        Some(handle)
    }

    /// Render Activity Bar (left-most 48px panel with icons)
    pub(crate) fn render_activity_bar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("activity_bar")
            .exact_width(48.0)
            .resizable(false)
            .frame(
                egui::Frame::NONE
                    .fill(ui_colors::ACTIVITY_BAR_BG)
                    .inner_margin(egui::Margin::same(4)),
            )
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(8.0);

                    let icon_size = 24.0;
                    let btn_size = egui::vec2(40.0, 40.0);
                    let active_bar_color = ui_colors::TEXT_DEFAULT;
                    let icon_active = ui_colors::TEXT_DEFAULT;
                    let icon_inactive = ui_colors::TEXT_MUTED;
                    let hover_bg = ui_colors::HOVER_BG;

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
                        // Custom-SVG icons rasterised once and tinted, vs.
                        // codicon glyphs drawn from the font. Each SVG entry
                        // has its own visual scale: codicon glyphs include
                        // their own EM padding, but SVG paths fill the
                        // viewBox edge-to-edge, so 1.0 is small.
                        let svg_tex = match panel.variant {
                            ActivePanel::SceneEditor => self
                                .scene_view_icon_texture(ctx, icon_size as u32)
                                .map(|t| (t, 1.15_f32)),
                            ActivePanel::Database => self
                                .database_icon_texture(ctx, icon_size as u32)
                                .map(|t| (t, 1.15_f32)),
                            ActivePanel::Docker => self
                                .docker_icon_texture(ctx, icon_size as u32)
                                .map(|t| (t, 1.15_f32)),
                            ActivePanel::OracleBerry => self
                                .oracleberry_icon_texture(ctx, icon_size as u32)
                                .map(|t| (t, 1.15_f32)),
                            _ => None,
                        };
                        if let Some((tex, scale)) = svg_tex {
                            let visual = icon_size * scale;
                            let img_rect = egui::Rect::from_center_size(
                                rect.center(),
                                egui::vec2(visual, visual),
                            );
                            egui::Image::new(&tex).tint(color).paint_at(ui, img_rect);
                        } else {
                            ui.painter().text(
                                rect.center(),
                                egui::Align2::CENTER_CENTER,
                                panel.icon,
                                egui::FontId::new(
                                    icon_size,
                                    egui::FontFamily::Name("codicon".into()),
                                ),
                                color,
                            );
                        }

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

/// Rasterise an embedded SVG to an `egui::TextureHandle`. The SVG is
/// rendered ×3 the requested pixel size for hidpi crispness, then
/// scaled down by egui at draw time. Used for the custom activity-bar
/// icons that aren't part of the codicon glyph set.
fn rasterise_svg(
    ctx: &egui::Context,
    texture_name: &str,
    svg_bytes: &str,
    size_px: u32,
) -> Option<egui::TextureHandle> {
    // Activity-bar icons are tinted multiplicatively, so the SVG must
    // rasterise to white. Codicons use `fill="currentColor"` (default
    // black with no context); Simple-Icons paths declare no fill at
    // all (also black). Force both to white before parsing.
    let mut prepared = svg_bytes.replace("currentColor", "#ffffff");
    if !prepared.contains("fill=") {
        prepared = prepared.replacen("<svg", "<svg fill=\"#ffffff\"", 1);
    }
    let opts = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_str(&prepared, &opts).ok()?;
    let render_size = size_px.max(8) * 3;
    let scale = render_size as f32 / tree.size().width().max(1.0);
    let mut pixmap = resvg::tiny_skia::Pixmap::new(render_size, render_size)?;
    let transform = resvg::tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    let image = egui::ColorImage::from_rgba_unmultiplied(
        [render_size as usize, render_size as usize],
        pixmap.data(),
    );
    Some(ctx.load_texture(texture_name, image, egui::TextureOptions::LINEAR))
}
