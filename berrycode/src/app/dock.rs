//! Dockable tool panel system.
//!
//! Provides a bottom panel area where tool windows (Console, Timeline,
//! Dopesheet, Profiler) can be displayed as tabs. Users click tabs to
//! switch between tools in the same panel area.

use crate::app::types::DiagnosticSeverity;
use crate::app::BerryCodeApp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolTab {
    /// LSP / cargo-check diagnostics — was a separate
    /// `diagnostics_panel` before the bottom panel was unified.
    Problems,
    /// Build / run output (cargo, run-script). Was previously labelled
    /// "Console"; renamed to match VS Code's terminology.
    Output,
    /// DAP debug REPL. Used to live as a sub-tab inside the debugger
    /// sidebar; surfaced here so all output streams are in one place.
    DebugConsole,
    Timeline,
    Dopesheet,
    Profiler,
}

impl ToolTab {
    #[allow(dead_code)]
    pub const ALL: &'static [ToolTab] = &[
        ToolTab::Problems,
        ToolTab::Output,
        ToolTab::DebugConsole,
        ToolTab::Timeline,
        ToolTab::Dopesheet,
        ToolTab::Profiler,
    ];

    /// Tabs in the "output" group — separated from "tool" tabs in the
    /// header by a visual divider (VS Code style).
    pub const OUTPUT_GROUP: &'static [ToolTab] = &[
        ToolTab::Problems,
        ToolTab::Output,
        ToolTab::DebugConsole,
    ];

    pub const TOOL_GROUP: &'static [ToolTab] =
        &[ToolTab::Timeline, ToolTab::Dopesheet, ToolTab::Profiler];

    #[allow(dead_code)]
    pub fn label(&self) -> &'static str {
        match self {
            ToolTab::Problems => "Problems",
            ToolTab::Output => "Output",
            ToolTab::DebugConsole => "Debug Console",
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
                // VS Code-style tab bar: [Problems][Output][Debug Console] |
                // [Timeline][Dopesheet][Profiler], with the active tab
                // shown via accent underline. Counts that would otherwise
                // require borrowing `self` from inside the closure are
                // pre-computed.
                let problems_count =
                    crate::app::utils::filter_rust_diagnostics(&self.lsp_diagnostics).len();
                let mut new_active = self.active_tool_tab;
                ui.horizontal(|ui| {
                    for tab in ToolTab::OUTPUT_GROUP {
                        render_tool_tab_label(ui, &mut new_active, *tab, problems_count);
                    }
                    ui.separator();
                    for tab in ToolTab::TOOL_GROUP {
                        render_tool_tab_label(ui, &mut new_active, *tab, problems_count);
                    }

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
                self.active_tool_tab = new_active;
                ui.separator();

                // Tab content
                match self.active_tool_tab {
                    ToolTab::Problems => self.render_problems_content(ui),
                    ToolTab::Output => self.render_console_content(ui),
                    ToolTab::DebugConsole => self.render_debug_console_content(ui),
                    ToolTab::Timeline => self.render_timeline_content(ui),
                    ToolTab::Dopesheet => self.render_dopesheet_content(ui),
                    ToolTab::Profiler => self.render_profiler_content(ui),
                }
            });
    }
}

impl BerryCodeApp {
    /// Renders LSP diagnostics inside the unified bottom panel. Pulled
    /// out of `render_diagnostics_panel` so the same content can live
    /// as a tab here.
    pub(crate) fn render_problems_content(&mut self, ui: &mut egui::Ui) {
        let rs_diagnostics = crate::app::utils::filter_rust_diagnostics(&self.lsp_diagnostics);
        if rs_diagnostics.is_empty() {
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new("No problems detected.")
                    .color(egui::Color32::from_rgb(140, 140, 150)),
            );
            return;
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 1.0;
            for diagnostic in &rs_diagnostics {
                let color = match diagnostic.severity {
                    DiagnosticSeverity::Error => egui::Color32::from_rgb(255, 80, 80),
                    DiagnosticSeverity::Warning => egui::Color32::from_rgb(255, 200, 100),
                    DiagnosticSeverity::Information => egui::Color32::from_rgb(100, 150, 255),
                    DiagnosticSeverity::Hint => egui::Color32::from_rgb(150, 150, 150),
                };
                let file_name = diagnostic
                    .source
                    .as_ref()
                    .and_then(|s| s.split('/').next_back())
                    .unwrap_or("unknown");
                let loc = format!(
                    "{}:{}:{}",
                    file_name,
                    diagnostic.line + 1,
                    diagnostic.column + 1
                );
                let msg = if diagnostic.message.len() > 200 {
                    format!("{}…", &diagnostic.message[..197])
                } else {
                    diagnostic.message.clone()
                };
                let mut job = egui::text::LayoutJob::default();
                let font = egui::FontId::monospace(11.5);
                job.append(
                    &loc,
                    0.0,
                    egui::TextFormat {
                        font_id: font.clone(),
                        color,
                        ..Default::default()
                    },
                );
                job.append(
                    "  ",
                    0.0,
                    egui::TextFormat {
                        font_id: font.clone(),
                        ..Default::default()
                    },
                );
                job.append(
                    &msg,
                    0.0,
                    egui::TextFormat {
                        font_id: font,
                        color: egui::Color32::from_rgb(210, 210, 210),
                        ..Default::default()
                    },
                );
                ui.label(job);
            }
        });
    }

    /// Thin wrapper that calls the existing debugger REPL renderer
    /// from inside the unified panel. Used to live in the debugger
    /// sidebar as its own tab.
    pub(crate) fn render_debug_console_content(&mut self, ui: &mut egui::Ui) {
        self.render_debug_console(ui);
    }
}

fn render_tool_tab_label(
    ui: &mut egui::Ui,
    active: &mut ToolTab,
    tab: ToolTab,
    problems_count: usize,
) {
    let is_active = *active == tab;
    // Show a small badge on Problems with the diagnostic count, so the
    // user notices new errors without expanding the tab.
    let label_text = match tab {
        ToolTab::Problems => {
            if problems_count > 0 {
                format!("Problems ({})", problems_count)
            } else {
                "Problems".to_string()
            }
        }
        other => other.label().to_string(),
    };
    let color = if is_active {
        egui::Color32::WHITE
    } else {
        egui::Color32::from_rgb(160, 162, 170)
    };
    let response = ui.add(
        egui::Button::new(egui::RichText::new(&label_text).size(12.0).color(color))
            .frame(false)
            .min_size(egui::vec2(0.0, 22.0)),
    );
    if response.clicked() {
        *active = tab;
    }
    if is_active {
        let rect = response.rect;
        ui.painter().line_segment(
            [
                egui::pos2(rect.left() + 4.0, rect.bottom() - 2.0),
                egui::pos2(rect.right() - 4.0, rect.bottom() - 2.0),
            ],
            egui::Stroke::new(2.0, egui::Color32::from_rgb(99, 139, 255)),
        );
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
    fn tool_tab_all_has_six_entries() {
        assert_eq!(ToolTab::ALL.len(), 6);
    }

    #[test]
    fn tool_tab_equality() {
        assert_eq!(ToolTab::Output, ToolTab::Output);
        assert_ne!(ToolTab::Output, ToolTab::Profiler);
    }
}
