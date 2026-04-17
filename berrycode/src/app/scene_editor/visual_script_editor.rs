//! Visual Script Editor: node-based logic graph editor UI.

use crate::app::BerryCodeApp;
use super::visual_script::*;

impl BerryCodeApp {
    /// Render the Visual Script Editor floating window.
    pub(crate) fn render_visual_script_editor(&mut self, ctx: &egui::Context) {
        if !self.visual_script_editor_open {
            return;
        }

        let script = match &mut self.editing_visual_script {
            Some(s) => s,
            None => {
                self.visual_script_editor_open = false;
                return;
            }
        };

        let mut open = self.visual_script_editor_open;

        egui::Window::new("Visual Script Editor")
            .open(&mut open)
            .default_size([700.0, 500.0])
            .resizable(true)
            .show(ctx, |ui| {
                // Toolbar
                ui.horizontal(|ui| {
                    ui.label(&script.name);
                    ui.separator();
                    if ui.button("Save").clicked() {
                        let path = format!("{}/untitled.bvscript", self.root_path);
                        if let Err(e) = save_visual_script(script, &path) {
                            self.status_message = format!("Save failed: {}", e);
                            self.status_message_timestamp = Some(std::time::Instant::now());
                        } else {
                            self.status_message = "Visual script saved".to_string();
                            self.status_message_timestamp = Some(std::time::Instant::now());
                        }
                    }
                });

                ui.separator();

                // Canvas area
                let available = ui.available_size();
                let (response, painter) = ui.allocate_painter(available, egui::Sense::click_and_drag());
                let canvas_rect = response.rect;

                // Draw background grid
                let grid_spacing = 20.0;
                let grid_color = egui::Color32::from_rgb(40, 40, 45);
                let min = canvas_rect.min;
                let max = canvas_rect.max;
                let mut x = min.x;
                while x < max.x {
                    painter.line_segment(
                        [egui::Pos2::new(x, min.y), egui::Pos2::new(x, max.y)],
                        egui::Stroke::new(1.0, grid_color),
                    );
                    x += grid_spacing;
                }
                let mut y = min.y;
                while y < max.y {
                    painter.line_segment(
                        [egui::Pos2::new(min.x, y), egui::Pos2::new(max.x, y)],
                        egui::Stroke::new(1.0, grid_color),
                    );
                    y += grid_spacing;
                }

                // Draw edges as bezier curves
                for edge in &script.edges {
                    let from_node = script.nodes.iter().find(|n| n.id == edge.from_node);
                    let to_node = script.nodes.iter().find(|n| n.id == edge.to_node);
                    if let (Some(from), Some(to)) = (from_node, to_node) {
                        let node_w = 120.0;
                        let node_h = 40.0;
                        let from_pt = egui::Pos2::new(
                            canvas_rect.min.x + from.position[0] + node_w,
                            canvas_rect.min.y + from.position[1] + node_h * 0.5 + edge.from_pin as f32 * 15.0,
                        );
                        let to_pt = egui::Pos2::new(
                            canvas_rect.min.x + to.position[0],
                            canvas_rect.min.y + to.position[1] + node_h * 0.5 + edge.to_pin as f32 * 15.0,
                        );
                        let mid_x = (from_pt.x + to_pt.x) * 0.5;
                        let cp1 = egui::Pos2::new(mid_x, from_pt.y);
                        let cp2 = egui::Pos2::new(mid_x, to_pt.y);
                        let curve = egui::epaint::CubicBezierShape::from_points_stroke(
                            [from_pt, cp1, cp2, to_pt],
                            false,
                            egui::Color32::TRANSPARENT,
                            egui::Stroke::new(2.0, egui::Color32::from_rgb(180, 180, 220)),
                        );
                        painter.add(curve);
                    }
                }

                // Draw nodes as colored boxes
                let mut dragging_node: Option<(usize, egui::Vec2)> = None;

                for (idx, node) in script.nodes.iter().enumerate() {
                    let node_w = 120.0;
                    let node_h = 40.0;
                    let node_pos = egui::Pos2::new(
                        canvas_rect.min.x + node.position[0],
                        canvas_rect.min.y + node.position[1],
                    );
                    let node_rect = egui::Rect::from_min_size(node_pos, egui::Vec2::new(node_w, node_h));

                    // Node color by type
                    let color = match &node.node_type {
                        NodeType::OnStart | NodeType::OnUpdate => egui::Color32::from_rgb(80, 150, 80),
                        NodeType::Branch => egui::Color32::from_rgb(200, 150, 50),
                        NodeType::Print { .. } => egui::Color32::from_rgb(100, 120, 180),
                        NodeType::SetTransform | NodeType::GetTransform => egui::Color32::from_rgb(150, 80, 150),
                        NodeType::FloatAdd | NodeType::FloatCompare { .. } => egui::Color32::from_rgb(80, 130, 150),
                        NodeType::Delay { .. } => egui::Color32::from_rgb(150, 100, 50),
                        NodeType::SpawnEntity { .. } => egui::Color32::from_rgb(180, 80, 80),
                    };

                    painter.rect_filled(node_rect, 4.0, color);
                    painter.rect_stroke(node_rect, 4.0, egui::Stroke::new(1.0, egui::Color32::WHITE));
                    painter.text(
                        node_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        node.node_type.label(),
                        egui::FontId::proportional(12.0),
                        egui::Color32::WHITE,
                    );

                    // Check if dragging this node
                    if response.dragged_by(egui::PointerButton::Primary) {
                        if let Some(pos) = response.interact_pointer_pos() {
                            if node_rect.contains(pos) {
                                dragging_node = Some((idx, response.drag_delta()));
                            }
                        }
                    }
                }

                // Apply drag
                if let Some((idx, delta)) = dragging_node {
                    script.nodes[idx].position[0] += delta.x;
                    script.nodes[idx].position[1] += delta.y;
                }

                // Right-click context menu to add nodes
                response.context_menu(|ui| {
                    ui.label("Add Node");
                    ui.separator();
                    if ui.button("On Start").clicked() {
                        let next_id = script.nodes.iter().map(|n| n.id).max().unwrap_or(0) + 1;
                        script.nodes.push(ScriptNode {
                            id: next_id,
                            node_type: NodeType::OnStart,
                            position: [200.0, 200.0],
                        });
                        ui.close_menu();
                    }
                    if ui.button("On Update").clicked() {
                        let next_id = script.nodes.iter().map(|n| n.id).max().unwrap_or(0) + 1;
                        script.nodes.push(ScriptNode {
                            id: next_id,
                            node_type: NodeType::OnUpdate,
                            position: [200.0, 200.0],
                        });
                        ui.close_menu();
                    }
                    if ui.button("Branch").clicked() {
                        let next_id = script.nodes.iter().map(|n| n.id).max().unwrap_or(0) + 1;
                        script.nodes.push(ScriptNode {
                            id: next_id,
                            node_type: NodeType::Branch,
                            position: [200.0, 200.0],
                        });
                        ui.close_menu();
                    }
                    if ui.button("Print").clicked() {
                        let next_id = script.nodes.iter().map(|n| n.id).max().unwrap_or(0) + 1;
                        script.nodes.push(ScriptNode {
                            id: next_id,
                            node_type: NodeType::Print { message: "Hello".into() },
                            position: [200.0, 200.0],
                        });
                        ui.close_menu();
                    }
                    if ui.button("Float Add").clicked() {
                        let next_id = script.nodes.iter().map(|n| n.id).max().unwrap_or(0) + 1;
                        script.nodes.push(ScriptNode {
                            id: next_id,
                            node_type: NodeType::FloatAdd,
                            position: [200.0, 200.0],
                        });
                        ui.close_menu();
                    }
                    if ui.button("Delay").clicked() {
                        let next_id = script.nodes.iter().map(|n| n.id).max().unwrap_or(0) + 1;
                        script.nodes.push(ScriptNode {
                            id: next_id,
                            node_type: NodeType::Delay { seconds: 1.0 },
                            position: [200.0, 200.0],
                        });
                        ui.close_menu();
                    }
                });
            });

        self.visual_script_editor_open = open;
    }
}
