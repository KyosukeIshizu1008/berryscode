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
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    let active_color = egui::Color32::from_rgb(255, 255, 255);
                    let inactive_color = egui::Color32::from_rgb(130, 130, 130);
                    let underline_color = egui::Color32::from_rgb(0, 122, 204);

                    for tab in ToolTab::ALL {
                        let selected = self.active_tool_tab == *tab;
                        let color = if selected {
                            active_color
                        } else {
                            inactive_color
                        };

                        let btn = egui::Button::new(
                            egui::RichText::new(tab.label()).size(12.0).color(color),
                        )
                        .frame(false)
                        .min_size(egui::vec2(70.0, 24.0));

                        let response = ui.add(btn);

                        // Active underline
                        if selected {
                            let rect = response.rect;
                            let line_rect = egui::Rect::from_min_size(
                                egui::pos2(rect.left(), rect.bottom() - 2.0),
                                egui::vec2(rect.width(), 2.0),
                            );
                            ui.painter().rect_filled(line_rect, 0.0, underline_color);
                        }

                        if response.clicked() {
                            self.active_tool_tab = *tab;
                        }
                    }

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
