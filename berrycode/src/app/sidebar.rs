//! Sidebar panel rendering

use super::types::ActivePanel;
use super::ui_colors;
use super::BerryCodeApp;

impl BerryCodeApp {
    /// Render Sidebar (file tree, chat, terminal, etc.)
    pub(crate) fn render_sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("sidebar")
            .default_width(200.0)
            .width_range(150.0..=500.0)
            .resizable(true)
            .show_separator_line(false)
            .frame(
                egui::Frame::none()
                    .fill(ui_colors::SIDEBAR_BG)
                    .inner_margin(egui::Margin::same(8.0))
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(40, 40, 40))),
            )
            .show(ctx, |ui| {
                // Update sidebar width from UI
                self.sidebar_width = ui.available_width();

                // Render content based on active panel
                match self.active_panel {
                    ActivePanel::Explorer => self.render_file_tree(ui),
                    ActivePanel::Search => self.render_search_panel(ui),
                    ActivePanel::Git => self.render_git_panel(ui),
                    ActivePanel::Terminal => self.render_terminal(ui),
                    ActivePanel::Settings => {
                        self.render_settings_panel(ui);
                    }
                    ActivePanel::EcsInspector => {
                        self.render_ecs_inspector_panel(ctx, ui);
                    }
                    ActivePanel::BevyTemplates => {
                        self.render_bevy_templates_panel(ui);
                    }
                    ActivePanel::AssetBrowser => {
                        self.render_asset_browser_panel(ctx, ui);
                    }
                    ActivePanel::SceneEditor => {
                        self.render_scene_hierarchy(ui);
                    }
                }
            });
    }
}
