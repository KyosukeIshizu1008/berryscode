//! Minimap (code overview) rendering
//! Shows a small code overview on the right side of the editor

/// Render a minimap (code overview) on the right side of the editor.
/// Standalone function to avoid borrow conflicts with BerryCodeApp.
pub(crate) fn render_minimap_standalone(ui: &mut egui::Ui, text: &str, editor_rect: egui::Rect) {
    let minimap_width = 60.0_f32;
    let minimap_rect = egui::Rect::from_min_size(
        egui::pos2(editor_rect.max.x - minimap_width, editor_rect.min.y),
        egui::vec2(minimap_width, editor_rect.height()),
    );

    // Background - match editor background (#191A1C)
    ui.painter()
        .rect_filled(minimap_rect, 0.0, egui::Color32::from_rgb(25, 26, 28));

    // Left border separator
    ui.painter().line_segment(
        [minimap_rect.left_top(), minimap_rect.left_bottom()],
        egui::Stroke::new(
            0.5,
            egui::Color32::from_rgba_premultiplied(255, 255, 255, 15),
        ),
    );

    let lines: Vec<&str> = text.lines().collect();
    let total_lines = lines.len().max(1);
    let scale = (minimap_rect.height() / (total_lines as f32 * 2.0)).min(2.0);

    // Draw minimap lines
    for (idx, line) in lines.iter().enumerate() {
        let y = minimap_rect.min.y + idx as f32 * scale;
        if y > minimap_rect.max.y {
            break;
        }

        let indent = line.chars().take_while(|c| c.is_whitespace()).count();
        let content_len = line.trim().len().min(50);

        if content_len == 0 {
            continue;
        }

        let x_start = minimap_rect.min.x + 3.0 + indent as f32 * 0.5;
        let x_end = x_start + content_len as f32 * 0.6;

        let color = if line.trim_start().starts_with("//") {
            egui::Color32::from_rgba_premultiplied(80, 130, 80, 50)
        } else if line.contains("fn ") || line.contains("pub ") || line.contains("struct ") {
            egui::Color32::from_rgba_premultiplied(100, 140, 220, 60)
        } else {
            egui::Color32::from_rgba_premultiplied(140, 140, 140, 35)
        };

        ui.painter().line_segment(
            [
                egui::pos2(x_start, y),
                egui::pos2(x_end.min(minimap_rect.max.x - 3.0), y),
            ],
            egui::Stroke::new(scale.max(1.0), color),
        );
    }

    // Viewport indicator
    let clip = ui.clip_rect();
    let visible_start = ((clip.min.y - editor_rect.min.y) / 19.5).max(0.0) as usize;
    let visible_end = ((clip.max.y - editor_rect.min.y) / 19.5).min(total_lines as f32) as usize;

    let vy_start = minimap_rect.min.y + visible_start as f32 * scale;
    let vy_end = minimap_rect.min.y + visible_end as f32 * scale;
    let viewport_rect = egui::Rect::from_min_max(
        egui::pos2(minimap_rect.min.x, vy_start),
        egui::pos2(minimap_rect.max.x, vy_end),
    );

    ui.painter().rect_filled(
        viewport_rect,
        0.0,
        egui::Color32::from_rgba_premultiplied(255, 255, 255, 8),
    );
}
