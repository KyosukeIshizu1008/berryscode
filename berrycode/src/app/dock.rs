//! Dockable tool panel system.
//!
//! Provides a bottom panel area where tool windows (Console, Timeline,
//! Dopesheet, Profiler) can be displayed as tabs. Users click tabs to
//! switch between tools in the same panel area.

use crate::app::BerryCodeApp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolTab {
    Console,
    Timeline,
    Dopesheet,
    Profiler,
}

impl ToolTab {
    pub const ALL: &'static [ToolTab] = &[
        ToolTab::Console,
        ToolTab::Timeline,
        ToolTab::Dopesheet,
        ToolTab::Profiler,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            ToolTab::Console => "Console",
            ToolTab::Timeline => "Timeline",
            ToolTab::Dopesheet => "Dopesheet",
            ToolTab::Profiler => "Profiler",
        }
    }
}

impl BerryCodeApp {
    /// Render the dockable tool panel at the bottom of the screen.
    /// This provides a tabbed container for Console, Timeline, Dopesheet, and
    /// Profiler, allowing users to switch between tools without multiple
    /// floating windows.
    pub(crate) fn render_tool_panel(&mut self, ctx: &egui::Context) {
        if !self.tool_panel_open {
            return;
        }

        egui::TopBottomPanel::bottom("tool_panel")
            .resizable(true)
            .default_height(250.0)
            .min_height(100.0)
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(30, 31, 34))
                    .inner_margin(egui::Margin::same(4.0)),
            )
            .show(ctx, |ui| {
                // Tab bar (VS Code style)
                let tabs: Vec<(ToolTab, &str)> =
                    ToolTab::ALL.iter().map(|t| (*t, t.label())).collect();
                super::utils::render_tab_bar(ui, &tabs, &mut self.active_tool_tab);
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("x").clicked() {
                            self.tool_panel_open = false;
                        }
                    });
                });

                ui.separator();

                // Content based on active tab
                match self.active_tool_tab {
                    ToolTab::Console => {
                        self.render_console_content(ui);
                    }
                    ToolTab::Timeline => {
                        self.render_timeline_content(ui);
                    }
                    ToolTab::Dopesheet => {
                        self.render_dopesheet_content(ui);
                    }
                    ToolTab::Profiler => {
                        self.render_profiler_content(ui);
                    }
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_tab_labels_are_non_empty() {
        for tab in ToolTab::ALL {
            assert!(!tab.label().is_empty());
        }
    }

    #[test]
    fn tool_tab_all_has_four_entries() {
        assert_eq!(ToolTab::ALL.len(), 4);
    }

    #[test]
    fn tool_tab_equality() {
        assert_eq!(ToolTab::Console, ToolTab::Console);
        assert_ne!(ToolTab::Console, ToolTab::Profiler);
    }
}
