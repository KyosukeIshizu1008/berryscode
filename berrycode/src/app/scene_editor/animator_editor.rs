//! Visual state machine editor for AnimatorController.
//!
//! Displays states as draggable boxes and transitions as arrows.
//! nodes are interactive — click-and-drag to reposition, right-click
//! for context menus (add/delete transitions).

use super::animator::*;
use crate::app::BerryCodeApp;

impl BerryCodeApp {
    /// Render the Animator Editor window.
    pub(crate) fn render_animator_editor(&mut self, ctx: &egui::Context) {
        if !self.animator_editor_open {
            return;
        }

        let controller = match &mut self.editing_animator {
            Some(c) => c,
            None => {
                self.animator_editor_open = false;
                return;
            }
        };

        let mut open = self.animator_editor_open;

        egui::Window::new("Animator Controller")
            .open(&mut open)
            .default_size([600.0, 400.0])
            .resizable(true)
            .show(ctx, |ui| {
                // Toolbar
                ui.horizontal(|ui| {
                    ui.label(&controller.name);
                    ui.separator();
                    if ui.button("+ State").clicked() {
                        controller.states.push(AnimState {
                            name: format!("State_{}", controller.states.len()),
                            clip_name: String::new(),
                            speed: 1.0,
                            looped: true,
                            position: [200.0 + controller.states.len() as f32 * 50.0, 200.0],
                        });
                    }
                    if ui.button("+ Bool Param").clicked() {
                        controller.parameters.push(AnimParam::Bool {
                            name: format!("param_{}", controller.parameters.len()),
                            value: false,
                        });
                    }
                    if ui.button("+ Float Param").clicked() {
                        controller.parameters.push(AnimParam::Float {
                            name: format!("param_{}", controller.parameters.len()),
                            value: 0.0,
                        });
                    }
                });

                ui.separator();

                // Parameters panel (left side)
                ui.columns(2, |cols| {
                    // Left: Parameters
                    cols[0].heading("Parameters");
                    for param in &mut controller.parameters {
                        cols[0].horizontal(|ui| match param {
                            AnimParam::Bool { name, value } => {
                                ui.text_edit_singleline(name);
                                ui.checkbox(value, "");
                            }
                            AnimParam::Float { name, value } => {
                                ui.text_edit_singleline(name);
                                ui.add(egui::DragValue::new(value).speed(0.05));
                            }
                            AnimParam::Trigger { name, .. } => {
                                ui.text_edit_singleline(name);
                                ui.label("[Trigger]");
                            }
                        });
                    }

                    // Right: Node graph
                    cols[1].heading("State Graph");
                    let graph_size = egui::vec2(cols[1].available_width(), 300.0);
                    let (graph_rect, _graph_resp) =
                        cols[1].allocate_exact_size(graph_size, egui::Sense::click());

                    let painter = cols[1].painter();
                    painter.rect_filled(graph_rect, 4.0, egui::Color32::from_rgb(20, 22, 26));

                    // Draw transitions as arrows
                    let mut delete_transition_idx: Option<usize> = None;
                    for (t_idx, transition) in controller.transitions.iter().enumerate() {
                        if transition.from_state < controller.states.len()
                            && transition.to_state < controller.states.len()
                        {
                            let from = &controller.states[transition.from_state];
                            let to = &controller.states[transition.to_state];
                            let from_center = egui::pos2(
                                graph_rect.left() + from.position[0] + 50.0,
                                graph_rect.top() + from.position[1] + 15.0,
                            );
                            let to_center = egui::pos2(
                                graph_rect.left() + to.position[0] + 50.0,
                                graph_rect.top() + to.position[1] + 15.0,
                            );
                            painter.line_segment(
                                [from_center, to_center],
                                egui::Stroke::new(2.0, egui::Color32::from_rgb(200, 200, 100)),
                            );
                            // Arrow head
                            let diff = to_center - from_center;
                            let len = diff.length();
                            if len > 0.01 {
                                let dir = diff / len;
                                let perp = egui::vec2(-dir.y, dir.x);
                                let tip = to_center - dir * 15.0;
                                painter.add(egui::Shape::convex_polygon(
                                    vec![to_center - dir * 5.0, tip + perp * 6.0, tip - perp * 6.0],
                                    egui::Color32::from_rgb(200, 200, 100),
                                    egui::Stroke::NONE,
                                ));
                            }

                            // Invisible hit area at the midpoint for right-click "Delete Transition"
                            let mid = egui::pos2(
                                (from_center.x + to_center.x) * 0.5,
                                (from_center.y + to_center.y) * 0.5,
                            );
                            let hit_rect =
                                egui::Rect::from_center_size(mid, egui::vec2(20.0, 20.0));
                            let hit_id = cols[1].id().with(("transition_hit", t_idx));
                            let hit_resp = cols[1].interact(hit_rect, hit_id, egui::Sense::click());
                            hit_resp.context_menu(|ui| {
                                if ui.button("Delete Transition").clicked() {
                                    delete_transition_idx = Some(t_idx);
                                    ui.close_menu();
                                }
                            });
                        }
                    }

                    // Apply deferred transition deletion
                    if let Some(idx) = delete_transition_idx {
                        if idx < controller.transitions.len() {
                            controller.transitions.remove(idx);
                        }
                    }

                    // Draw state boxes (interactive / draggable)
                    let state_count = controller.states.len();
                    // Collect drag actions to apply after iteration
                    let mut drag_deltas: Vec<(usize, egui::Vec2)> = Vec::new();
                    let mut clicked_state: Option<usize> = None;
                    let mut context_menu_state: Option<usize> = None;

                    for i in 0..state_count {
                        let state = &controller.states[i];
                        let box_pos = egui::pos2(
                            graph_rect.left() + state.position[0],
                            graph_rect.top() + state.position[1],
                        );
                        let box_size = egui::vec2(100.0, 30.0);
                        let box_rect = egui::Rect::from_min_size(box_pos, box_size);

                        let is_default = i == controller.default_state;
                        let is_dragging = self.animator_dragging_state == Some(i);
                        let is_pending_src = self.pending_transition_from == Some(i);
                        let fill = if is_dragging {
                            egui::Color32::from_rgb(80, 80, 120)
                        } else if is_pending_src {
                            egui::Color32::from_rgb(120, 100, 50)
                        } else if is_default {
                            egui::Color32::from_rgb(60, 100, 60)
                        } else {
                            egui::Color32::from_rgb(50, 55, 65)
                        };
                        painter.rect_filled(box_rect, 4.0, fill);
                        painter.rect_stroke(
                            box_rect,
                            4.0,
                            egui::Stroke::new(1.0, egui::Color32::from_gray(100)),
                        );
                        painter.text(
                            box_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            &state.name,
                            egui::FontId::proportional(12.0),
                            egui::Color32::WHITE,
                        );

                        // Interactive response for drag and context menu
                        let node_id = cols[1].id().with(("anim_state", i));
                        let resp =
                            cols[1].interact(box_rect, node_id, egui::Sense::click_and_drag());

                        if resp.drag_started() {
                            self.animator_dragging_state = Some(i);
                        }
                        if resp.dragged() && self.animator_dragging_state == Some(i) {
                            drag_deltas.push((i, resp.drag_delta()));
                        }
                        if resp.drag_stopped() && self.animator_dragging_state == Some(i) {
                            self.animator_dragging_state = None;
                        }

                        // Left click: if we have a pending_transition_from, complete the transition
                        if resp.clicked() {
                            clicked_state = Some(i);
                        }

                        // Right-click context menu
                        if resp.secondary_clicked() {
                            context_menu_state = Some(i);
                        }
                        resp.context_menu(|ui| {
                            if ui.button("Add Transition From Here").clicked() {
                                self.pending_transition_from = Some(i);
                                ui.close_menu();
                            }
                            if ui.button("Set as Default").clicked() {
                                controller.default_state = i;
                                ui.close_menu();
                            }
                            ui.separator();
                            if ui.button("Delete State").clicked() {
                                // Remove transitions referencing this state
                                controller
                                    .transitions
                                    .retain(|t| t.from_state != i && t.to_state != i);
                                // Adjust transition indices for states after the removed one
                                for t in &mut controller.transitions {
                                    if t.from_state > i {
                                        t.from_state -= 1;
                                    }
                                    if t.to_state > i {
                                        t.to_state -= 1;
                                    }
                                }
                                controller.states.remove(i);
                                if controller.default_state >= controller.states.len()
                                    && !controller.states.is_empty()
                                {
                                    controller.default_state = 0;
                                }
                                // Clear any pending transition referencing this state
                                if self.pending_transition_from == Some(i) {
                                    self.pending_transition_from = None;
                                } else if let Some(ref mut from) = self.pending_transition_from {
                                    if *from > i {
                                        *from -= 1;
                                    }
                                }
                                ui.close_menu();
                            }
                        });
                    }

                    // Apply drag position changes
                    for (idx, delta) in drag_deltas {
                        if idx < controller.states.len() {
                            controller.states[idx].position[0] += delta.x;
                            controller.states[idx].position[1] += delta.y;
                        }
                    }

                    // Complete pending transition on click
                    if let (Some(from), Some(to)) = (self.pending_transition_from, clicked_state) {
                        if from != to
                            && from < controller.states.len()
                            && to < controller.states.len()
                        {
                            controller.transitions.push(AnimTransition {
                                from_state: from,
                                to_state: to,
                                condition: TransitionCondition::OnComplete,
                                blend_duration: 0.2,
                            });
                        }
                        self.pending_transition_from = None;
                    }

                    // Status hint
                    if self.pending_transition_from.is_some() {
                        let _ = context_menu_state; // suppress unused
                        cols[1].colored_label(
                            egui::Color32::from_rgb(255, 200, 80),
                            "Click a target state to create transition (Esc to cancel)",
                        );
                    }
                });

                // Cancel pending transition on Escape
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    self.pending_transition_from = None;
                }
            });

        self.animator_editor_open = open;
    }
}
