//! Sidebar panel rendering

use super::types::ActivePanel;
use super::ui_colors;
use super::BerryCodeApp;

impl BerryCodeApp {
    /// Render Sidebar (file tree, chat, terminal, etc.)
    pub(crate) fn render_sidebar(&mut self, ctx: &egui::Context) {
        // Bumped id (`sidebar_v3`) discards stale persisted widths from
        // earlier builds, so the panel comes up at `default_width` and
        // stays inside `width_range` regardless of what the previous
        // session had stored. Both Explorer and Search render inside
        // this same panel, so they always share width by construction.
        egui::SidePanel::left("sidebar_v3")
            .default_width(210.0)
            .width_range(160.0..=360.0)
            .resizable(true)
            .show_separator_line(true)
            .frame(
                egui::Frame::NONE
                    .fill(ui_colors::SIDEBAR_BG)
                    .inner_margin(egui::Margin::same(8)),
            )
            .show(ctx, |ui| {
                // Track the current width for other UI code that needs it,
                // but don't feed it back into `default_width` — that would
                // create a per-frame shrink loop with the inner_margin.
                self.sidebar_width = ui.available_width();

                // Render content based on active panel
                match self.active_panel {
                    ActivePanel::Explorer => self.render_file_tree(ui),
                    ActivePanel::Search => self.render_search_panel(ui),
                    ActivePanel::Git => self.render_git_panel(ui),
                    ActivePanel::Terminal => self.render_terminal(ui),
                    ActivePanel::Settings => {
                        // Settings is rendered in the wider CentralPanel
                        // (see `mod.rs`) — the sidebar is intentionally
                        // blank in this mode so the user gets the full
                        // window width for tabs and form fields.
                    }
                    ActivePanel::EcsInspector => {
                        self.render_ecs_inspector_panel(ctx, ui);
                    }
                    ActivePanel::BevyTemplates => {}
                    ActivePanel::SceneEditor => {
                        self.render_scene_hierarchy(ui);
                    }
                    ActivePanel::Database => {
                        self.render_database_sidebar(ui);
                    }
                    ActivePanel::Docker => {
                        self.render_docker_sidebar(ui);
                    }
                    ActivePanel::OracleBerry => {
                        self.render_oracleberry_sidebar(ui);
                    }
                }
            });
    }
}
