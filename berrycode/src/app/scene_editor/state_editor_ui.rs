//! State Editor window rendering (node graph for Bevy States).

use super::state_editor::{generate_states_code, GameState};
use crate::app::BerryCodeApp;

impl BerryCodeApp {
    pub(crate) fn render_state_editor(&mut self, ctx: &egui::Context) {
        if !self.state_editor_open {
            return;
        }
        let mut open = self.state_editor_open;

        egui::Window::new("Bevy States Editor")
            .open(&mut open)
            .default_size([500.0, 400.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("+ State").clicked() {
                        self.state_graph.states.push(GameState {
                            name: format!("State_{}", self.state_graph.states.len()),
                            position: [200.0, 200.0],
                        });
                    }
                    if ui.button("Generate Code").clicked() {
                        let code = generate_states_code(&self.state_graph);
                        let path = format!("{}/src/game_state.rs", self.root_path);
                        if let Err(e) = std::fs::write(&path, &code) {
                            self.status_message = format!("Failed: {}", e);
                        } else {
                            self.status_message = format!("Generated: {}", path);
                        }
                        self.status_message_timestamp = Some(std::time::Instant::now());
                    }
                });
                ui.separator();

                // Draw state graph (same pattern as animator_editor)
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), 300.0),
                    egui::Sense::click(),
                );
                ui.painter().rect_filled(
                    rect,
                    4.0,
                    egui::Color32::from_rgb(20, 22, 26),
                );

                // Draw transitions
                for t in &self.state_graph.transitions {
                    if t.from < self.state_graph.states.len()
                        && t.to < self.state_graph.states.len()
                    {
                        let from = &self.state_graph.states[t.from];
                        let to = &self.state_graph.states[t.to];
                        let fp = egui::pos2(
                            rect.left() + from.position[0] + 50.0,
                            rect.top() + from.position[1] + 15.0,
                        );
                        let tp = egui::pos2(
                            rect.left() + to.position[0] + 50.0,
                            rect.top() + to.position[1] + 15.0,
                        );
                        ui.painter().line_segment(
                            [fp, tp],
                            egui::Stroke::new(
                                1.5,
                                egui::Color32::from_rgb(200, 200, 100),
                            ),
                        );
                        // Label
                        let mid = egui::pos2(
                            (fp.x + tp.x) / 2.0,
                            (fp.y + tp.y) / 2.0 - 8.0,
                        );
                        ui.painter().text(
                            mid,
                            egui::Align2::CENTER_CENTER,
                            &t.condition,
                            egui::FontId::proportional(10.0),
                            egui::Color32::from_rgb(180, 180, 140),
                        );
                    }
                }

                // Draw state boxes
                for (i, state) in self.state_graph.states.iter().enumerate() {
                    let pos = egui::pos2(
                        rect.left() + state.position[0],
                        rect.top() + state.position[1],
                    );
                    let box_rect =
                        egui::Rect::from_min_size(pos, egui::vec2(100.0, 30.0));
                    let fill = if i == self.state_graph.initial_state {
                        egui::Color32::from_rgb(60, 100, 60)
                    } else {
                        egui::Color32::from_rgb(50, 55, 70)
                    };
                    ui.painter().rect_filled(box_rect, 4.0, fill);
                    ui.painter().rect_stroke(
                        box_rect,
                        4.0,
                        egui::Stroke::new(1.0, egui::Color32::from_gray(100)),
                    );
                    ui.painter().text(
                        box_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        &state.name,
                        egui::FontId::proportional(12.0),
                        egui::Color32::WHITE,
                    );
                }
            });
        self.state_editor_open = open;
    }
}
