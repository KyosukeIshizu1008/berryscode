//! Visual state machine editor for AnimatorController.
//!
//! Displays states as draggable boxes and transitions as arrows.
//! States are color-coded by kind: Entry=green, Exit=red, AnyState=cyan, Normal=gray.
//! Includes inline inspector for selected state/transition.

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
            .default_size([800.0, 550.0])
            .resizable(true)
            .show(ctx, |ui| {
                // --- Toolbar ---
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(&controller.name)
                            .size(14.0)
                            .color(egui::Color32::WHITE),
                    );
                    ui.separator();
                    if ui.button("+ State").clicked() {
                        controller.states.push(AnimState {
                            name: format!("State_{}", controller.states.len()),
                            clip_name: String::new(),
                            motion: Motion::default(),
                            speed: 1.0,
                            looped: true,
                            position: [200.0 + controller.states.len() as f32 * 60.0, 200.0],
                            kind: StateKind::Normal,
                        });
                    }
                    ui.menu_button("+ Parameter", |ui| {
                        if ui.button("Bool").clicked() {
                            controller.parameters.push(AnimParam::Bool {
                                name: format!("param_{}", controller.parameters.len()),
                                value: false,
                            });
                            ui.close_menu();
                        }
                        if ui.button("Float").clicked() {
                            controller.parameters.push(AnimParam::Float {
                                name: format!("param_{}", controller.parameters.len()),
                                value: 0.0,
                            });
                            ui.close_menu();
                        }
                        if ui.button("Int").clicked() {
                            controller.parameters.push(AnimParam::Int {
                                name: format!("param_{}", controller.parameters.len()),
                                value: 0,
                            });
                            ui.close_menu();
                        }
                        if ui.button("Trigger").clicked() {
                            controller.parameters.push(AnimParam::Trigger {
                                name: format!("trigger_{}", controller.parameters.len()),
                                fired: false,
                            });
                            ui.close_menu();
                        }
                    });
                    if ui.button("+ Exit State").clicked() {
                        controller.states.push(AnimState {
                            name: "Exit".into(),
                            clip_name: String::new(),
                            motion: Motion::default(),
                            speed: 1.0,
                            looped: false,
                            position: [400.0, 300.0],
                            kind: StateKind::Exit,
                        });
                    }
                    if ui.button("+ AnyState").clicked() {
                        controller.states.push(AnimState {
                            name: "AnyState".into(),
                            clip_name: String::new(),
                            motion: Motion::default(),
                            speed: 1.0,
                            looped: false,
                            position: [50.0, 50.0],
                            kind: StateKind::AnyState,
                        });
                    }
                    if ui.button("Save").clicked() {
                        if !self.editing_animator_path.is_empty() {
                            let _ = save_animator(controller, &self.editing_animator_path);
                            self.status_message = "Animator saved".into();
                            self.status_message_timestamp = Some(std::time::Instant::now());
                        }
                    }
                });

                ui.separator();

                // --- Main layout: Parameters + Graph + Inspector ---
                let available_h = ui.available_height();

                // Parameters panel (left, narrow)
                ui.horizontal(|ui| {
                    // Left: Parameters
                    ui.vertical(|ui| {
                        ui.set_width(160.0);
                        ui.label(
                            egui::RichText::new("PARAMETERS")
                                .size(10.0)
                                .color(egui::Color32::from_gray(150)),
                        );
                        ui.separator();
                        let mut remove_param: Option<usize> = None;
                        for (pi, param) in controller.parameters.iter_mut().enumerate() {
                            ui.horizontal(|ui| {
                                match param {
                                    AnimParam::Bool { name, value } => {
                                        ui.checkbox(value, "");
                                        ui.add(
                                            egui::TextEdit::singleline(name).desired_width(80.0),
                                        );
                                    }
                                    AnimParam::Float { name, value } => {
                                        ui.add(egui::DragValue::new(value).speed(0.05));
                                        ui.add(
                                            egui::TextEdit::singleline(name).desired_width(80.0),
                                        );
                                    }
                                    AnimParam::Int { name, value } => {
                                        ui.add(egui::DragValue::new(value).speed(1.0));
                                        ui.add(
                                            egui::TextEdit::singleline(name).desired_width(80.0),
                                        );
                                    }
                                    AnimParam::Trigger { name, .. } => {
                                        ui.label("\u{26a1}");
                                        ui.add(
                                            egui::TextEdit::singleline(name).desired_width(80.0),
                                        );
                                    }
                                }
                                if ui.small_button("x").clicked() {
                                    remove_param = Some(pi);
                                }
                            });
                        }
                        if let Some(idx) = remove_param {
                            controller.parameters.remove(idx);
                        }
                    });

                    ui.separator();

                    // Center: Node graph
                    ui.vertical(|ui| {
                        let graph_size =
                            egui::vec2(ui.available_width(), (available_h - 40.0).max(200.0));
                        let (graph_rect, _) =
                            ui.allocate_exact_size(graph_size, egui::Sense::click());
                        let painter = ui.painter();

                        // Dark background with grid
                        painter.rect_filled(graph_rect, 4.0, egui::Color32::from_rgb(20, 22, 26));
                        let grid_color = egui::Color32::from_rgba_premultiplied(255, 255, 255, 10);
                        let grid_step = 30.0;
                        let mut gx = graph_rect.left();
                        while gx < graph_rect.right() {
                            painter.line_segment(
                                [
                                    egui::pos2(gx, graph_rect.top()),
                                    egui::pos2(gx, graph_rect.bottom()),
                                ],
                                egui::Stroke::new(0.5, grid_color),
                            );
                            gx += grid_step;
                        }
                        let mut gy = graph_rect.top();
                        while gy < graph_rect.bottom() {
                            painter.line_segment(
                                [
                                    egui::pos2(graph_rect.left(), gy),
                                    egui::pos2(graph_rect.right(), gy),
                                ],
                                egui::Stroke::new(0.5, grid_color),
                            );
                            gy += grid_step;
                        }

                        // --- Draw transitions ---
                        let mut delete_transition_idx: Option<usize> = None;
                        let mut select_transition: Option<usize> = None;
                        for (t_idx, transition) in controller.transitions.iter().enumerate() {
                            if transition.from_state >= controller.states.len()
                                || transition.to_state >= controller.states.len()
                            {
                                continue;
                            }
                            let from = &controller.states[transition.from_state];
                            let to = &controller.states[transition.to_state];
                            let from_c = egui::pos2(
                                graph_rect.left() + from.position[0] + 75.0,
                                graph_rect.top() + from.position[1] + 25.0,
                            );
                            let to_c = egui::pos2(
                                graph_rect.left() + to.position[0] + 75.0,
                                graph_rect.top() + to.position[1] + 25.0,
                            );

                            let is_selected = self.animator_selected_transition == Some(t_idx);
                            let arrow_color = if is_selected {
                                egui::Color32::from_rgb(255, 220, 80)
                            } else {
                                egui::Color32::from_rgb(180, 180, 80)
                            };
                            let stroke_w = if is_selected { 3.0 } else { 2.0 };

                            // Bezier curve
                            let mid_x = (from_c.x + to_c.x) * 0.5;
                            let ctrl1 = egui::pos2(mid_x, from_c.y);
                            let ctrl2 = egui::pos2(mid_x, to_c.y);
                            let points: Vec<egui::Pos2> = (0..=20)
                                .map(|i| {
                                    let t = i as f32 / 20.0;
                                    let mt = 1.0 - t;
                                    egui::pos2(
                                        mt * mt * mt * from_c.x
                                            + 3.0 * mt * mt * t * ctrl1.x
                                            + 3.0 * mt * t * t * ctrl2.x
                                            + t * t * t * to_c.x,
                                        mt * mt * mt * from_c.y
                                            + 3.0 * mt * mt * t * ctrl1.y
                                            + 3.0 * mt * t * t * ctrl2.y
                                            + t * t * t * to_c.y,
                                    )
                                })
                                .collect();
                            for w in points.windows(2) {
                                painter.line_segment(
                                    [w[0], w[1]],
                                    egui::Stroke::new(stroke_w, arrow_color),
                                );
                            }

                            // Arrow head
                            let diff = to_c - from_c;
                            let len = diff.length();
                            if len > 0.01 {
                                let dir = diff / len;
                                let perp = egui::vec2(-dir.y, dir.x);
                                let tip = to_c - dir * 18.0;
                                painter.add(egui::Shape::convex_polygon(
                                    vec![to_c - dir * 8.0, tip + perp * 6.0, tip - perp * 6.0],
                                    arrow_color,
                                    egui::Stroke::NONE,
                                ));
                            }

                            // Hit area at midpoint
                            let mid =
                                egui::pos2((from_c.x + to_c.x) * 0.5, (from_c.y + to_c.y) * 0.5);
                            let hit_rect =
                                egui::Rect::from_center_size(mid, egui::vec2(24.0, 24.0));
                            let hit_id = ui.id().with(("transition_hit", t_idx));
                            let hit_resp = ui.interact(hit_rect, hit_id, egui::Sense::click());
                            if hit_resp.clicked() {
                                select_transition = Some(t_idx);
                            }
                            hit_resp.context_menu(|ui| {
                                if ui.button("Delete Transition").clicked() {
                                    delete_transition_idx = Some(t_idx);
                                    ui.close_menu();
                                }
                            });
                        }

                        if let Some(idx) = delete_transition_idx {
                            if idx < controller.transitions.len() {
                                controller.transitions.remove(idx);
                                self.animator_selected_transition = None;
                            }
                        }
                        if let Some(idx) = select_transition {
                            self.animator_selected_transition = Some(idx);
                            self.animator_selected_state = None;
                        }

                        // --- Draw state nodes ---
                        let state_count = controller.states.len();
                        let mut drag_deltas: Vec<(usize, egui::Vec2)> = Vec::new();
                        let mut clicked_state: Option<usize> = None;

                        for i in 0..state_count {
                            let state = &controller.states[i];
                            let box_pos = egui::pos2(
                                graph_rect.left() + state.position[0],
                                graph_rect.top() + state.position[1],
                            );
                            let box_size = egui::vec2(150.0, 50.0);
                            let box_rect = egui::Rect::from_min_size(box_pos, box_size);

                            let is_default = i == controller.default_state;
                            let is_selected = self.animator_selected_state == Some(i);
                            let is_dragging = self.animator_dragging_state == Some(i);
                            let is_pending = self.pending_transition_from == Some(i);

                            let fill = match state.kind {
                                StateKind::Entry => egui::Color32::from_rgb(40, 100, 50),
                                StateKind::Exit => egui::Color32::from_rgb(120, 40, 40),
                                StateKind::AnyState => egui::Color32::from_rgb(40, 90, 110),
                                StateKind::Normal => {
                                    if is_dragging {
                                        egui::Color32::from_rgb(80, 80, 120)
                                    } else if is_pending {
                                        egui::Color32::from_rgb(120, 100, 50)
                                    } else if is_default {
                                        egui::Color32::from_rgb(60, 85, 60)
                                    } else {
                                        egui::Color32::from_rgb(50, 55, 65)
                                    }
                                }
                            };

                            let border_color = if is_selected {
                                egui::Color32::from_rgb(0, 150, 255)
                            } else {
                                egui::Color32::from_gray(80)
                            };

                            painter.rect_filled(box_rect, 6.0, fill);
                            painter.rect_stroke(
                                box_rect,
                                6.0,
                                egui::Stroke::new(
                                    if is_selected { 2.0 } else { 1.0 },
                                    border_color,
                                ),
                            );

                            // State name
                            painter.text(
                                egui::pos2(box_rect.center().x, box_rect.top() + 15.0),
                                egui::Align2::CENTER_CENTER,
                                &state.name,
                                egui::FontId::proportional(12.0),
                                egui::Color32::WHITE,
                            );

                            // Motion subtitle
                            let subtitle = match &state.motion {
                                Motion::Clip { clip_name } if !clip_name.is_empty() => {
                                    clip_name.as_str()
                                }
                                Motion::BlendTree(bt) => &bt.name,
                                _ => {
                                    if !state.clip_name.is_empty() {
                                        state.clip_name.as_str()
                                    } else {
                                        match state.kind {
                                            StateKind::Entry => "Entry Point",
                                            StateKind::Exit => "Exit Point",
                                            StateKind::AnyState => "Any State",
                                            StateKind::Normal => "(no clip)",
                                        }
                                    }
                                }
                            };
                            painter.text(
                                egui::pos2(box_rect.center().x, box_rect.top() + 34.0),
                                egui::Align2::CENTER_CENTER,
                                subtitle,
                                egui::FontId::proportional(10.0),
                                egui::Color32::from_gray(160),
                            );

                            // Default state arrow indicator
                            if is_default && state.kind == StateKind::Normal {
                                let arrow_x = box_rect.left() - 12.0;
                                let arrow_y = box_rect.center().y;
                                painter.add(egui::Shape::convex_polygon(
                                    vec![
                                        egui::pos2(arrow_x - 8.0, arrow_y - 5.0),
                                        egui::pos2(arrow_x, arrow_y),
                                        egui::pos2(arrow_x - 8.0, arrow_y + 5.0),
                                    ],
                                    egui::Color32::from_rgb(255, 160, 50),
                                    egui::Stroke::NONE,
                                ));
                            }

                            // Interaction
                            let node_id = ui.id().with(("anim_state", i));
                            let resp =
                                ui.interact(box_rect, node_id, egui::Sense::click_and_drag());

                            if resp.drag_started() {
                                self.animator_dragging_state = Some(i);
                            }
                            if resp.dragged() && self.animator_dragging_state == Some(i) {
                                drag_deltas.push((i, resp.drag_delta()));
                            }
                            if resp.drag_stopped() && self.animator_dragging_state == Some(i) {
                                self.animator_dragging_state = None;
                            }
                            if resp.clicked() {
                                clicked_state = Some(i);
                                self.animator_selected_state = Some(i);
                                self.animator_selected_transition = None;
                            }

                            let state_kind = state.kind;
                            resp.context_menu(|ui| {
                                if ui.button("Add Transition From Here").clicked() {
                                    self.pending_transition_from = Some(i);
                                    ui.close_menu();
                                }
                                if state_kind == StateKind::Normal {
                                    if ui.button("Set as Default").clicked() {
                                        controller.default_state = i;
                                        ui.close_menu();
                                    }
                                }
                                ui.separator();
                                if ui.button("Delete State").clicked() {
                                    controller
                                        .transitions
                                        .retain(|t| t.from_state != i && t.to_state != i);
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
                                    if self.pending_transition_from == Some(i) {
                                        self.pending_transition_from = None;
                                    } else if let Some(ref mut from) = self.pending_transition_from
                                    {
                                        if *from > i {
                                            *from -= 1;
                                        }
                                    }
                                    if self.animator_selected_state == Some(i) {
                                        self.animator_selected_state = None;
                                    }
                                    ui.close_menu();
                                }
                            });
                        }

                        // Apply drags
                        for (idx, delta) in drag_deltas {
                            if idx < controller.states.len() {
                                controller.states[idx].position[0] += delta.x;
                                controller.states[idx].position[1] += delta.y;
                            }
                        }

                        // Complete pending transition
                        if let (Some(from), Some(to)) =
                            (self.pending_transition_from, clicked_state)
                        {
                            if from != to
                                && from < controller.states.len()
                                && to < controller.states.len()
                            {
                                controller.transitions.push(AnimTransition {
                                    from_state: from,
                                    to_state: to,
                                    condition: TransitionCondition::OnComplete,
                                    blend_duration: 0.2,
                                    has_exit_time: false,
                                    exit_time: 1.0,
                                });
                            }
                            self.pending_transition_from = None;
                        }

                        // Status hint
                        if self.pending_transition_from.is_some() {
                            painter.text(
                                egui::pos2(graph_rect.left() + 8.0, graph_rect.bottom() - 20.0),
                                egui::Align2::LEFT_CENTER,
                                "Click target state to create transition (Esc to cancel)",
                                egui::FontId::proportional(11.0),
                                egui::Color32::from_rgb(255, 200, 80),
                            );
                        }
                    });
                });

                // --- Inline Inspector (bottom) ---
                ui.separator();
                if let Some(si) = self.animator_selected_state {
                    if si < controller.states.len() {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("State:")
                                    .size(11.0)
                                    .color(egui::Color32::from_gray(150)),
                            );
                            ui.add(
                                egui::TextEdit::singleline(&mut controller.states[si].name)
                                    .desired_width(100.0),
                            );
                            ui.separator();
                            ui.label("Motion:");
                            let is_clip =
                                matches!(&controller.states[si].motion, Motion::Clip { .. });
                            if ui.selectable_label(is_clip, "Clip").clicked() && !is_clip {
                                controller.states[si].motion = Motion::Clip {
                                    clip_name: controller.states[si].clip_name.clone(),
                                };
                            }
                            if ui.selectable_label(!is_clip, "BlendTree").clicked() && is_clip {
                                controller.states[si].motion = Motion::BlendTree(BlendTree {
                                    name: format!("{}_blend", controller.states[si].name),
                                    blend_type: BlendType::Simple1D,
                                    parameter_x: controller
                                        .parameters
                                        .first()
                                        .map(|p| p.name().to_string())
                                        .unwrap_or_default(),
                                    parameter_y: String::new(),
                                    children: vec![],
                                });
                            }
                            ui.separator();
                            match &mut controller.states[si].motion {
                                Motion::Clip { clip_name } => {
                                    ui.label("Clip:");
                                    ui.add(
                                        egui::TextEdit::singleline(clip_name)
                                            .desired_width(100.0)
                                            .hint_text("clip name"),
                                    );
                                }
                                Motion::BlendTree(bt) => {
                                    ui.label(format!("BlendTree: {}", bt.name));
                                }
                            }
                            ui.separator();
                            ui.label("Speed:");
                            ui.add(
                                egui::DragValue::new(&mut controller.states[si].speed)
                                    .speed(0.05)
                                    .range(0.0..=10.0),
                            );
                            ui.checkbox(&mut controller.states[si].looped, "Loop");
                        });
                    }
                } else if let Some(ti) = self.animator_selected_transition {
                    if ti < controller.transitions.len() {
                        let from_name = controller
                            .states
                            .get(controller.transitions[ti].from_state)
                            .map(|s| s.name.clone())
                            .unwrap_or_else(|| "?".into());
                        let to_name = controller
                            .states
                            .get(controller.transitions[ti].to_state)
                            .map(|s| s.name.clone())
                            .unwrap_or_else(|| "?".into());
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(format!(
                                    "Transition: {} -> {}",
                                    from_name, to_name
                                ))
                                .size(11.0)
                                .color(egui::Color32::from_gray(150)),
                            );
                            ui.separator();
                            ui.label("Blend:");
                            ui.add(
                                egui::DragValue::new(
                                    &mut controller.transitions[ti].blend_duration,
                                )
                                .speed(0.01)
                                .range(0.0..=5.0)
                                .suffix("s"),
                            );
                            ui.separator();
                            ui.checkbox(&mut controller.transitions[ti].has_exit_time, "Exit Time");
                            if controller.transitions[ti].has_exit_time {
                                ui.add(
                                    egui::DragValue::new(&mut controller.transitions[ti].exit_time)
                                        .speed(0.01)
                                        .range(0.0..=1.0),
                                );
                            }
                            ui.separator();
                            if ui
                                .button(
                                    egui::RichText::new("Delete")
                                        .color(egui::Color32::from_rgb(255, 100, 100)),
                                )
                                .clicked()
                            {
                                controller.transitions.remove(ti);
                                self.animator_selected_transition = None;
                            }
                        });
                    }
                }

                // Cancel pending transition on Escape
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    self.pending_transition_from = None;
                }
            });

        self.animator_editor_open = open;
    }
}
