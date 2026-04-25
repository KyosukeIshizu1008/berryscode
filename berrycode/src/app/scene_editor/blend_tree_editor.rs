#![allow(dead_code)]
//! Visual editor for BlendTree (1D and 2D blend visualization).

use super::animator::*;
use crate::app::BerryCodeApp;

impl BerryCodeApp {
    /// Render the BlendTree visual editor window.
    pub(crate) fn render_blend_tree_editor(&mut self, ctx: &egui::Context) {
        if !self.blend_tree_editor_open {
            return;
        }

        let tree = match &mut self.editing_blend_tree {
            Some(t) => t,
            None => {
                self.blend_tree_editor_open = false;
                return;
            }
        };

        let mut open = self.blend_tree_editor_open;

        egui::Window::new("Blend Tree Editor")
            .open(&mut open)
            .default_size([600.0, 400.0])
            .resizable(true)
            .show(ctx, |ui| {
                // --- Header ---
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut tree.name);
                    ui.separator();
                    egui::ComboBox::from_label("Type")
                        .selected_text(format!("{:?}", tree.blend_type))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut tree.blend_type,
                                BlendType::Simple1D,
                                "Simple 1D",
                            );
                            ui.selectable_value(
                                &mut tree.blend_type,
                                BlendType::SimpleDirectional2D,
                                "Simple Directional 2D",
                            );
                            ui.selectable_value(
                                &mut tree.blend_type,
                                BlendType::FreeformDirectional2D,
                                "Freeform Directional 2D",
                            );
                            ui.selectable_value(
                                &mut tree.blend_type,
                                BlendType::FreeformCartesian2D,
                                "Freeform Cartesian 2D",
                            );
                        });
                });

                ui.horizontal(|ui| {
                    ui.label("Param X:");
                    ui.text_edit_singleline(&mut tree.parameter_x);
                    if tree.blend_type != BlendType::Simple1D {
                        ui.label("Param Y:");
                        ui.text_edit_singleline(&mut tree.parameter_y);
                    }
                });

                ui.separator();

                if tree.blend_type == BlendType::Simple1D {
                    Self::render_blend_1d_view(ui, tree);
                } else {
                    Self::render_blend_2d_view(ui, tree);
                }

                ui.separator();

                // --- Children list ---
                ui.label("Children:");
                let mut remove_idx = None;
                for (i, child) in tree.children.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(format!("[{}]", i));
                        let clip_name = match &mut child.motion {
                            Motion::Clip { clip_name } => clip_name,
                            _ => return,
                        };
                        ui.label("Clip:");
                        ui.text_edit_singleline(clip_name);
                        if tree.blend_type == BlendType::Simple1D {
                            ui.label("Threshold:");
                            ui.add(egui::DragValue::new(&mut child.threshold).speed(0.01));
                        } else {
                            ui.label("Pos:");
                            ui.add(
                                egui::DragValue::new(&mut child.position[0])
                                    .speed(0.01)
                                    .prefix("x:"),
                            );
                            ui.add(
                                egui::DragValue::new(&mut child.position[1])
                                    .speed(0.01)
                                    .prefix("y:"),
                            );
                        }
                        ui.label("Speed:");
                        ui.add(egui::DragValue::new(&mut child.time_scale).speed(0.01));
                        if ui.button("X").clicked() {
                            remove_idx = Some(i);
                        }
                    });
                }
                if let Some(idx) = remove_idx {
                    tree.children.remove(idx);
                }
                if ui.button("+ Add Child").clicked() {
                    tree.children.push(BlendTreeChild {
                        motion: Motion::Clip {
                            clip_name: String::new(),
                        },
                        threshold: 0.0,
                        position: [0.0, 0.0],
                        time_scale: 1.0,
                    });
                }
            });

        self.blend_tree_editor_open = open;
    }

    /// Draw a 1D blend visualization: horizontal bar with children at threshold positions.
    fn render_blend_1d_view(ui: &mut egui::Ui, tree: &BlendTree) {
        let (response, painter) =
            ui.allocate_painter(egui::vec2(ui.available_width(), 60.0), egui::Sense::hover());
        let rect = response.rect;
        let bg = egui::Color32::from_rgb(40, 42, 46);
        painter.rect_filled(rect, 4.0, bg);

        if tree.children.is_empty() {
            return;
        }

        let min_t = tree
            .children
            .iter()
            .map(|c| c.threshold)
            .fold(f32::INFINITY, f32::min)
            - 0.1;
        let max_t = tree
            .children
            .iter()
            .map(|c| c.threshold)
            .fold(f32::NEG_INFINITY, f32::max)
            + 0.1;
        let range = (max_t - min_t).max(0.01);

        // Axis line
        let y_mid = rect.center().y;
        painter.line_segment(
            [
                egui::pos2(rect.left() + 10.0, y_mid),
                egui::pos2(rect.right() - 10.0, y_mid),
            ],
            egui::Stroke::new(1.0, egui::Color32::GRAY),
        );

        for (i, child) in tree.children.iter().enumerate() {
            let frac = (child.threshold - min_t) / range;
            let x = rect.left() + 10.0 + frac * (rect.width() - 20.0);
            painter.circle_filled(
                egui::pos2(x, y_mid),
                6.0,
                egui::Color32::from_rgb(100, 180, 255),
            );
            let label = match &child.motion {
                Motion::Clip { clip_name } if !clip_name.is_empty() => clip_name.as_str(),
                _ => "?",
            };
            painter.text(
                egui::pos2(x, y_mid - 14.0),
                egui::Align2::CENTER_BOTTOM,
                format!("{} ({})", label, i),
                egui::FontId::proportional(10.0),
                egui::Color32::WHITE,
            );
        }
    }

    /// Draw a 2D blend visualization: scatter plot with children at positions.
    fn render_blend_2d_view(ui: &mut egui::Ui, tree: &BlendTree) {
        let size = ui.available_width().min(300.0);
        let (response, painter) = ui.allocate_painter(egui::vec2(size, size), egui::Sense::hover());
        let rect = response.rect;
        let bg = egui::Color32::from_rgb(40, 42, 46);
        painter.rect_filled(rect, 4.0, bg);

        // Crosshair at center (current param = 0,0)
        let center = rect.center();
        painter.line_segment(
            [
                egui::pos2(rect.left(), center.y),
                egui::pos2(rect.right(), center.y),
            ],
            egui::Stroke::new(0.5, egui::Color32::DARK_GRAY),
        );
        painter.line_segment(
            [
                egui::pos2(center.x, rect.top()),
                egui::pos2(center.x, rect.bottom()),
            ],
            egui::Stroke::new(0.5, egui::Color32::DARK_GRAY),
        );

        if tree.children.is_empty() {
            return;
        }

        // Compute extent
        let mut extent = 1.0f32;
        for child in &tree.children {
            extent = extent
                .max(child.position[0].abs())
                .max(child.position[1].abs());
        }
        extent += 0.1;

        for (i, child) in tree.children.iter().enumerate() {
            let fx = child.position[0] / extent * 0.45 + 0.5;
            let fy = 0.5 - child.position[1] / extent * 0.45; // y up
            let px = rect.left() + fx * rect.width();
            let py = rect.top() + fy * rect.height();
            painter.circle_filled(
                egui::pos2(px, py),
                6.0,
                egui::Color32::from_rgb(100, 220, 140),
            );
            let label = match &child.motion {
                Motion::Clip { clip_name } if !clip_name.is_empty() => clip_name.as_str(),
                _ => "?",
            };
            painter.text(
                egui::pos2(px, py - 10.0),
                egui::Align2::CENTER_BOTTOM,
                format!("{} ({})", label, i),
                egui::FontId::proportional(10.0),
                egui::Color32::WHITE,
            );
        }
    }
}
