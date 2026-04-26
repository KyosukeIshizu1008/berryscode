//! Shader Graph Editor: node-based material parameter graph editor UI.

use super::shader_graph::*;
use crate::app::BerryCodeApp;

impl BerryCodeApp {
    /// Render the Shader Graph Editor floating window.
    pub(crate) fn render_shader_graph_editor(&mut self, ctx: &egui::Context) {
        if !self.shader_graph_editor_open {
            return;
        }

        let graph = match &mut self.editing_shader_graph {
            Some(g) => g,
            None => {
                self.shader_graph_editor_open = false;
                return;
            }
        };

        let mut open = self.shader_graph_editor_open;

        // Evaluate graph and push PBR params to material preview when changed.
        let pbr = evaluate_graph(graph);
        self.material_preview_color = pbr.base_color;
        self.material_preview_metallic = pbr.metallic;
        self.material_preview_roughness = pbr.roughness;
        self.material_preview_emissive = pbr.emissive;
        self.material_preview_dirty = true;

        egui::Window::new("Shader Graph Editor")
            .open(&mut open)
            .default_size([700.0, 500.0])
            .resizable(true)
            .show(ctx, |ui| {
                // Toolbar
                ui.horizontal(|ui| {
                    ui.label(&graph.name);
                    ui.separator();
                    if ui.button("Save").clicked() {
                        let path = format!("{}/untitled.bshader", self.root_path);
                        if let Err(e) = save_shader_graph(graph, &path) {
                            self.status_message = format!("Save failed: {}", e);
                            self.status_message_timestamp = Some(std::time::Instant::now());
                        } else {
                            self.status_message = "Shader graph saved".to_string();
                            self.status_message_timestamp = Some(std::time::Instant::now());
                        }
                    }
                });

                ui.separator();

                // Canvas area
                let available = ui.available_size();
                let (response, painter) =
                    ui.allocate_painter(available, egui::Sense::click_and_drag());
                let canvas_rect = response.rect;

                // Background grid
                let grid_spacing = 20.0;
                let grid_color = egui::Color32::from_rgb(35, 35, 45);
                let mut x = canvas_rect.min.x;
                while x < canvas_rect.max.x {
                    painter.line_segment(
                        [
                            egui::Pos2::new(x, canvas_rect.min.y),
                            egui::Pos2::new(x, canvas_rect.max.y),
                        ],
                        egui::Stroke::new(1.0, grid_color),
                    );
                    x += grid_spacing;
                }
                let mut y = canvas_rect.min.y;
                while y < canvas_rect.max.y {
                    painter.line_segment(
                        [
                            egui::Pos2::new(canvas_rect.min.x, y),
                            egui::Pos2::new(canvas_rect.max.x, y),
                        ],
                        egui::Stroke::new(1.0, grid_color),
                    );
                    y += grid_spacing;
                }

                // Draw edges as bezier curves
                for edge in &graph.edges {
                    let from_node = graph.nodes.iter().find(|n| n.id == edge.from_node);
                    let to_node = graph.nodes.iter().find(|n| n.id == edge.to_node);
                    if let (Some(from), Some(to)) = (from_node, to_node) {
                        let node_w = 120.0;
                        let node_h = 40.0;
                        let from_pt = egui::Pos2::new(
                            canvas_rect.min.x + from.position[0] + node_w,
                            canvas_rect.min.y
                                + from.position[1]
                                + node_h * 0.5
                                + edge.from_pin as f32 * 15.0,
                        );
                        let to_pt = egui::Pos2::new(
                            canvas_rect.min.x + to.position[0],
                            canvas_rect.min.y
                                + to.position[1]
                                + node_h * 0.5
                                + edge.to_pin as f32 * 15.0,
                        );
                        let mid_x = (from_pt.x + to_pt.x) * 0.5;
                        let cp1 = egui::Pos2::new(mid_x, from_pt.y);
                        let cp2 = egui::Pos2::new(mid_x, to_pt.y);
                        let curve = egui::epaint::CubicBezierShape::from_points_stroke(
                            [from_pt, cp1, cp2, to_pt],
                            false,
                            egui::Color32::TRANSPARENT,
                            egui::Stroke::new(2.0, egui::Color32::from_rgb(200, 200, 160)),
                        );
                        painter.add(curve);
                    }
                }

                // Draw nodes as colored boxes
                let mut dragging_node: Option<(usize, egui::Vec2)> = None;

                for (idx, node) in graph.nodes.iter().enumerate() {
                    let node_w = 120.0;
                    let node_h = 40.0;
                    let node_pos = egui::Pos2::new(
                        canvas_rect.min.x + node.position[0],
                        canvas_rect.min.y + node.position[1],
                    );
                    let node_rect =
                        egui::Rect::from_min_size(node_pos, egui::Vec2::new(node_w, node_h));

                    // Color by shader node type
                    let color = match &node.node_type {
                        ShaderNodeType::OutputPBR => egui::Color32::from_rgb(200, 60, 60),
                        ShaderNodeType::TextureSample { .. } => {
                            egui::Color32::from_rgb(80, 140, 80)
                        }
                        ShaderNodeType::ColorConstant { .. } => {
                            egui::Color32::from_rgb(200, 150, 50)
                        }
                        ShaderNodeType::FloatConstant { .. } => {
                            egui::Color32::from_rgb(100, 100, 180)
                        }
                        ShaderNodeType::Multiply | ShaderNodeType::Add => {
                            egui::Color32::from_rgb(120, 120, 120)
                        }
                        ShaderNodeType::Lerp => egui::Color32::from_rgb(150, 100, 150),
                        ShaderNodeType::UVCoord => egui::Color32::from_rgb(50, 150, 150),
                        ShaderNodeType::Time => egui::Color32::from_rgb(150, 150, 50),
                        ShaderNodeType::Fresnel { .. } => egui::Color32::from_rgb(100, 150, 200),
                    };

                    painter.rect_filled(node_rect, 4.0, color);
                    painter.rect_stroke(
                        node_rect,
                        4.0,
                        egui::Stroke::new(1.0, egui::Color32::WHITE),
                        egui::StrokeKind::Middle,
                    );
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
                    graph.nodes[idx].position[0] += delta.x;
                    graph.nodes[idx].position[1] += delta.y;
                }

                // Right-click context menu to add nodes
                response.context_menu(|ui| {
                    ui.label("Add Shader Node");
                    ui.separator();
                    if ui.button("PBR Output").clicked() {
                        let next_id = graph.nodes.iter().map(|n| n.id).max().unwrap_or(0) + 1;
                        graph.nodes.push(ShaderNode {
                            id: next_id,
                            node_type: ShaderNodeType::OutputPBR,
                            position: [300.0, 200.0],
                        });
                        ui.close();
                    }
                    if ui.button("Color Constant").clicked() {
                        let next_id = graph.nodes.iter().map(|n| n.id).max().unwrap_or(0) + 1;
                        graph.nodes.push(ShaderNode {
                            id: next_id,
                            node_type: ShaderNodeType::ColorConstant {
                                value: [1.0, 1.0, 1.0, 1.0],
                            },
                            position: [100.0, 200.0],
                        });
                        ui.close();
                    }
                    if ui.button("Float Constant").clicked() {
                        let next_id = graph.nodes.iter().map(|n| n.id).max().unwrap_or(0) + 1;
                        graph.nodes.push(ShaderNode {
                            id: next_id,
                            node_type: ShaderNodeType::FloatConstant { value: 0.5 },
                            position: [100.0, 200.0],
                        });
                        ui.close();
                    }
                    if ui.button("Texture Sample").clicked() {
                        let next_id = graph.nodes.iter().map(|n| n.id).max().unwrap_or(0) + 1;
                        graph.nodes.push(ShaderNode {
                            id: next_id,
                            node_type: ShaderNodeType::TextureSample {
                                path: String::new(),
                            },
                            position: [100.0, 200.0],
                        });
                        ui.close();
                    }
                    if ui.button("Multiply").clicked() {
                        let next_id = graph.nodes.iter().map(|n| n.id).max().unwrap_or(0) + 1;
                        graph.nodes.push(ShaderNode {
                            id: next_id,
                            node_type: ShaderNodeType::Multiply,
                            position: [200.0, 200.0],
                        });
                        ui.close();
                    }
                    if ui.button("Lerp").clicked() {
                        let next_id = graph.nodes.iter().map(|n| n.id).max().unwrap_or(0) + 1;
                        graph.nodes.push(ShaderNode {
                            id: next_id,
                            node_type: ShaderNodeType::Lerp,
                            position: [200.0, 200.0],
                        });
                        ui.close();
                    }
                    if ui.button("UV Coord").clicked() {
                        let next_id = graph.nodes.iter().map(|n| n.id).max().unwrap_or(0) + 1;
                        graph.nodes.push(ShaderNode {
                            id: next_id,
                            node_type: ShaderNodeType::UVCoord,
                            position: [50.0, 200.0],
                        });
                        ui.close();
                    }
                    if ui.button("Time").clicked() {
                        let next_id = graph.nodes.iter().map(|n| n.id).max().unwrap_or(0) + 1;
                        graph.nodes.push(ShaderNode {
                            id: next_id,
                            node_type: ShaderNodeType::Time,
                            position: [50.0, 200.0],
                        });
                        ui.close();
                    }
                    if ui.button("Fresnel").clicked() {
                        let next_id = graph.nodes.iter().map(|n| n.id).max().unwrap_or(0) + 1;
                        graph.nodes.push(ShaderNode {
                            id: next_id,
                            node_type: ShaderNodeType::Fresnel { power: 2.0 },
                            position: [100.0, 200.0],
                        });
                        ui.close();
                    }
                });
            });

        self.shader_graph_editor_open = open;
    }
}
