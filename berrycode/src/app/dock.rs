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
                egui::Frame::NONE
                    .fill(egui::Color32::from_rgb(30, 31, 34))
                    .inner_margin(egui::Margin::same(4)),
            )
            .show(ctx, |ui| {
                // Header: Console label + close button
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Console")
                            .size(13.0)
                            .color(egui::Color32::WHITE),
                    );

                    // Close button (right-aligned)
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let icon_font =
                            egui::FontId::new(14.0, egui::FontFamily::Name("codicon".into()));
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("\u{ea76}") // codicon: close
                                        .font(icon_font)
                                        .color(egui::Color32::from_rgb(150, 150, 150)),
                                )
                                .frame(false),
                            )
                            .on_hover_text("Close Panel")
                            .clicked()
                        {
                            self.tool_panel_open = false;
                        }
                    });
                });

                // Console content
                self.render_console_content(ui);
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
