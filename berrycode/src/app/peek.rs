//! Peek Definition
//! Shows definition inline instead of jumping to it

use super::BerryCodeApp;
use super::types::PeekDefinition;
use super::ui_colors;

/// Render the peek definition inline window below the current cursor line.
/// Standalone function to avoid borrow conflicts with BerryCodeApp.
pub(crate) fn render_peek_standalone(
    ui: &mut egui::Ui,
    peek: &PeekDefinition,
    editor_rect: egui::Rect,
) {
    let line_height = 19.5_f32;
    let peek_y = editor_rect.min.y + (peek.anchor_line + 1) as f32 * line_height;

    // Don't render if below visible area
    if peek_y > editor_rect.max.y {
        return;
    }

    let peek_height = 160.0_f32;
    let peek_width = (editor_rect.width() * 0.8).min(600.0);
    let peek_x = editor_rect.min.x + 40.0;

    let peek_rect = egui::Rect::from_min_size(
        egui::pos2(peek_x, peek_y),
        egui::vec2(peek_width, peek_height),
    );

    // Background with border
    ui.painter().rect_filled(
        peek_rect,
        4.0,
        egui::Color32::from_rgb(35, 36, 40),
    );
    ui.painter().rect_stroke(
        peek_rect,
        4.0,
        egui::Stroke::new(1.0, egui::Color32::from_rgb(75, 78, 82)),
    );

    // Header with file path
    let header_rect = egui::Rect::from_min_size(
        peek_rect.min,
        egui::vec2(peek_width, 22.0),
    );
    ui.painter().rect_filled(
        header_rect,
        egui::Rounding {
            nw: 4.0,
            ne: 4.0,
            sw: 0.0,
            se: 0.0,
        },
        egui::Color32::from_rgb(45, 46, 50),
    );

    // File path text
    let path_text = format!("{}:{}", peek.file_path, peek.line + 1);
    ui.painter().text(
        egui::pos2(peek_rect.min.x + 8.0, header_rect.center().y),
        egui::Align2::LEFT_CENTER,
        &path_text,
        egui::FontId::proportional(11.0),
        egui::Color32::from_rgb(180, 180, 180),
    );

    // Close hint [Esc]
    ui.painter().text(
        egui::pos2(peek_rect.max.x - 30.0, header_rect.center().y),
        egui::Align2::CENTER_CENTER,
        "[Esc]",
        egui::FontId::proportional(10.0),
        egui::Color32::from_rgb(128, 128, 128),
    );

    // Content preview
    let content_y = peek_rect.min.y + 24.0;
    let preview_line_height = 16.0_f32;
    for (idx, line) in peek.content_preview.lines().enumerate() {
        let y = content_y + idx as f32 * preview_line_height;
        if y + preview_line_height > peek_rect.max.y {
            break;
        }

        ui.painter().text(
            egui::pos2(peek_rect.min.x + 8.0, y),
            egui::Align2::LEFT_TOP,
            line,
            egui::FontId::monospace(12.0),
            ui_colors::TEXT_DEFAULT,
        );
    }
}

impl BerryCodeApp {
    /// Open peek definition at the current cursor position
    pub(crate) fn open_peek_definition(&mut self) {
        // Use the last go-to-definition result if available
        if let Some(location) = self.definition_picker_locations.first() {
            let file_path = location.file_path.clone();
            let line = location.line;

            // Read a few lines around the definition
            let content_preview = match crate::native::fs::read_file(&file_path) {
                Ok(content) => {
                    let lines: Vec<&str> = content.lines().collect();
                    let start = line.saturating_sub(2);
                    let end = (line + 8).min(lines.len());
                    lines[start..end].join("\n")
                }
                Err(_) => "(Could not read file)".to_string(),
            };

            // Get current cursor line for anchor
            let anchor_line = self
                .editor_tabs
                .get(self.active_tab_idx)
                .map(|t| t.cursor_line)
                .unwrap_or(0);

            self.peek_definition = Some(PeekDefinition {
                file_path,
                line,
                content_preview,
                anchor_line,
            });
        }
    }

    /// Close the peek definition view
    pub(crate) fn close_peek_definition(&mut self) {
        self.peek_definition = None;
    }
}
