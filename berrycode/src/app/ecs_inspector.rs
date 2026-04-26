//! ECS Inspector UI panel — VS Code style tree view + property grid

use super::BerryCodeApp;

/// Tab selection for the ECS Inspector panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EcsInspectorTab {
    #[default]
    Entities,
    Resources,
}

// VS Code-like colors
const HEADER_BG: egui::Color32 = egui::Color32::from_rgb(37, 37, 38);
const ITEM_HOVER: egui::Color32 = egui::Color32::from_rgb(45, 45, 48);
const ITEM_SELECTED: egui::Color32 = egui::Color32::from_rgb(4, 57, 94);
const LABEL_DIM: egui::Color32 = egui::Color32::from_rgb(128, 128, 128);
const LABEL_BRIGHT: egui::Color32 = egui::Color32::from_rgb(212, 212, 212);
const ACCENT_BLUE: egui::Color32 = egui::Color32::from_rgb(75, 139, 190);
const STATUS_GREEN: egui::Color32 = egui::Color32::from_rgb(80, 200, 80);
const STATUS_RED: egui::Color32 = egui::Color32::from_rgb(200, 80, 80);
const PROP_KEY: egui::Color32 = egui::Color32::from_rgb(156, 220, 254);
const PROP_VAL_NUM: egui::Color32 = egui::Color32::from_rgb(181, 206, 168);
const PROP_VAL_STR: egui::Color32 = egui::Color32::from_rgb(206, 145, 120);
const SECTION_ICON: &str = "\u{EB5F}"; // codicon: symbol-class

impl BerryCodeApp {
    pub(crate) fn render_ecs_inspector_panel(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) {
        let small = egui::FontId::proportional(11.0);
        let normal = egui::FontId::proportional(12.0);

        // ── Header bar ──
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("ECS INSPECTOR")
                    .font(small.clone())
                    .color(LABEL_DIM)
                    .strong(),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Refresh button
                if self.ecs_inspector.connected {
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("\u{eb37}")
                                    .font(normal.clone())
                                    .color(LABEL_DIM),
                            )
                            .frame(false),
                        )
                        .on_hover_text("Refresh")
                        .clicked()
                    {
                        self.refresh_ecs_data();
                    }
                }
            });
        });

        ui.add_space(2.0);

        // ── Connection bar (compact) ──
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;

            let (btn_text, btn_color) = if self.ecs_inspector.connected {
                ("\u{eadf}", STATUS_GREEN) // codicon: debug-disconnect
            } else {
                ("\u{eade}", LABEL_DIM) // codicon: debug-start
            };

            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new(btn_text)
                            .font(normal.clone())
                            .color(btn_color),
                    )
                    .frame(false),
                )
                .on_hover_text(if self.ecs_inspector.connected {
                    "Disconnect"
                } else {
                    "Connect"
                })
                .clicked()
            {
                if self.ecs_inspector.connected {
                    self.ecs_inspector.connected = false;
                    self.ecs_inspector.entities.clear();
                    self.ecs_inspector.resources.clear();
                    self.ecs_inspector.component_values.clear();
                } else {
                    self.connect_to_bevy_app();
                }
            }

            // Status dot
            let status_color = if self.ecs_inspector.connected {
                STATUS_GREEN
            } else {
                STATUS_RED
            };
            ui.label(
                egui::RichText::new("\u{25CF}")
                    .font(small.clone())
                    .color(status_color),
            );

            // Endpoint (compact)
            let ep_resp = ui.add(
                egui::TextEdit::singleline(&mut self.ecs_inspector.endpoint)
                    .font(small.clone())
                    .desired_width(ui.available_width())
                    .text_color(LABEL_DIM),
            );
            if ep_resp.lost_focus()
                && ui.input(|i| i.key_pressed(egui::Key::Enter))
                && !self.ecs_inspector.connected
            {
                self.connect_to_bevy_app();
            }
        });

        // Error message
        if let Some(err) = &self.ecs_inspector.error_message {
            let err = err.clone();
            ui.label(
                egui::RichText::new(&err)
                    .font(small.clone())
                    .color(STATUS_RED),
            );
        }

        // ── Performance stats (compact) ──
        if self.ecs_inspector.connected {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                let entity_count = self.ecs_inspector.perf_entity_count;
                let latency = self.ecs_inspector.perf_poll_latency_ms;
                ui.label(
                    egui::RichText::new(format!("{} entities | {:.0}ms", entity_count, latency))
                        .font(small.clone())
                        .color(LABEL_DIM),
                );

                // Sparkline of latency history
                let history = &self.ecs_inspector.perf_latency_history;
                if !history.is_empty() {
                    let sparkline_w = 60.0_f32;
                    let sparkline_h = 16.0_f32;
                    let (spark_rect, _) = ui.allocate_exact_size(
                        egui::vec2(sparkline_w, sparkline_h),
                        egui::Sense::hover(),
                    );

                    // Background
                    ui.painter()
                        .rect_filled(spark_rect, 2.0, egui::Color32::from_rgb(30, 30, 35));

                    let max_val = history.iter().cloned().fold(1.0_f64, f64::max);
                    let n = history.len();
                    if n >= 2 {
                        let points: Vec<egui::Pos2> = history
                            .iter()
                            .enumerate()
                            .map(|(i, &v)| {
                                let x = spark_rect.min.x
                                    + (i as f32 / (n - 1).max(1) as f32) * sparkline_w;
                                let y = spark_rect.max.y
                                    - ((v / max_val) as f32 * (sparkline_h - 2.0) + 1.0);
                                egui::pos2(x, y)
                            })
                            .collect();
                        for pair in points.windows(2) {
                            ui.painter().line_segment(
                                [pair[0], pair[1]],
                                egui::Stroke::new(1.0, ACCENT_BLUE),
                            );
                        }
                    }
                }
            });
        }

        ui.add_space(2.0);

        if !self.ecs_inspector.connected {
            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new("Connect to a running Bevy app")
                        .font(normal.clone())
                        .color(LABEL_DIM),
                );
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("Add RemotePlugin to your Bevy app")
                        .font(small.clone())
                        .color(egui::Color32::from_rgb(80, 80, 80)),
                );
            });
            return;
        }

        // ── Tab bar (VS Code style underline tabs) ──
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let tabs = [
                (EcsInspectorTab::Entities, "Entities"),
                (EcsInspectorTab::Resources, "Resources"),
            ];
            for (tab, label) in &tabs {
                let selected = self.ecs_inspector_tab == *tab;
                let color = if selected { LABEL_BRIGHT } else { LABEL_DIM };
                let btn =
                    egui::Button::new(egui::RichText::new(*label).font(small.clone()).color(color))
                        .frame(false)
                        .min_size(egui::vec2(0.0, 20.0));
                let resp = ui.add(btn);
                if selected {
                    let r = resp.rect;
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(
                            egui::pos2(r.left(), r.bottom() - 2.0),
                            egui::vec2(r.width(), 2.0),
                        ),
                        0.0,
                        ACCENT_BLUE,
                    );
                }
                if resp.clicked() {
                    self.ecs_inspector_tab = *tab;
                }
                ui.add_space(12.0);
            }
        });

        // Thin separator
        ui.painter().line_segment(
            [
                egui::pos2(ui.min_rect().left(), ui.cursor().top()),
                egui::pos2(ui.min_rect().right(), ui.cursor().top()),
            ],
            egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 45, 45)),
        );
        ui.add_space(2.0);

        // ── Filter ──
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.label(
                egui::RichText::new("\u{eb51}")
                    .font(small.clone())
                    .color(LABEL_DIM),
            ); // codicon: search
            ui.add(
                egui::TextEdit::singleline(&mut self.ecs_inspector.filter_query)
                    .font(small.clone())
                    .hint_text("Filter...")
                    .desired_width(ui.available_width()),
            );
        });

        ui.add_space(2.0);

        match self.ecs_inspector_tab {
            EcsInspectorTab::Entities => self.render_ecs_entities_tab_vscode(ui),
            EcsInspectorTab::Resources => self.render_ecs_resources_tab(ui),
        }
    }

    /// VS Code style entity tree view + inline property grid
    fn render_ecs_entities_tab_vscode(&mut self, ui: &mut egui::Ui) {
        let small = egui::FontId::proportional(11.0);
        let filter = self.ecs_inspector.filter_query.to_lowercase();

        let entities_snapshot: Vec<_> = self
            .ecs_inspector
            .entities
            .iter()
            .map(|e| (e.id, e.name.clone()))
            .collect();

        let entity_count = entities_snapshot.len();

        // Section header
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("ENTITIES ({})", entity_count))
                    .font(small.clone())
                    .color(LABEL_DIM)
                    .strong(),
            );
        });

        ui.add_space(1.0);

        egui::ScrollArea::vertical()
            .id_salt("ecs_entity_tree")
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;

                for (id, name) in &entities_snapshot {
                    let display_name = name.as_deref().unwrap_or("(unnamed)");

                    if !filter.is_empty() && !display_name.to_lowercase().contains(&filter) {
                        continue;
                    }

                    let selected = self.ecs_inspector.selected_entity == Some(*id);

                    // Row background
                    let row_rect = ui.available_rect_before_wrap();
                    let row_rect = egui::Rect::from_min_size(
                        row_rect.left_top(),
                        egui::vec2(row_rect.width(), 22.0),
                    );

                    let row_response = ui.allocate_rect(row_rect, egui::Sense::click());

                    let bg = if selected {
                        ITEM_SELECTED
                    } else if row_response.hovered() {
                        ITEM_HOVER
                    } else {
                        egui::Color32::TRANSPARENT
                    };

                    if bg != egui::Color32::TRANSPARENT {
                        ui.painter().rect_filled(row_rect, 0.0, bg);
                    }

                    // Icon + name
                    let icon_pos = row_rect.left_center() + egui::vec2(8.0, 0.0);
                    ui.painter().text(
                        icon_pos,
                        egui::Align2::LEFT_CENTER,
                        SECTION_ICON,
                        small.clone(),
                        ACCENT_BLUE,
                    );

                    let name_pos = row_rect.left_center() + egui::vec2(24.0, 0.0);
                    let name_color = if selected {
                        egui::Color32::WHITE
                    } else {
                        LABEL_BRIGHT
                    };
                    ui.painter().text(
                        name_pos,
                        egui::Align2::LEFT_CENTER,
                        display_name,
                        small.clone(),
                        name_color,
                    );

                    if row_response.clicked() {
                        self.ecs_inspector.selected_entity = Some(*id);
                        self.load_entity_components(*id);
                    }
                }
            });
    }

    /// Helper: get selected entity id and name
    fn selected_entity_info(&self) -> Option<(u64, String)> {
        let id = self.ecs_inspector.selected_entity?;
        let name = self
            .ecs_inspector
            .entities
            .iter()
            .find(|e| e.id == id)
            .and_then(|e| e.name.clone())
            .unwrap_or_else(|| format!("Entity {}", id));
        Some((id, name))
    }

    /// Render 3D view (center panel)
    pub(crate) fn render_ecs_3d_view(&mut self, ui: &mut egui::Ui) {
        if !self.ecs_inspector.connected {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("Connect to a Bevy app to inspect ECS data")
                        .font(egui::FontId::proportional(14.0))
                        .color(LABEL_DIM),
                );
            });
            return;
        }

        let Some((entity_id, entity_name)) = self.selected_entity_info() else {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("Select an entity to view in 3D")
                        .font(egui::FontId::proportional(14.0))
                        .color(LABEL_DIM),
                );
            });
            return;
        };

        let small = egui::FontId::proportional(11.0);

        // ── 3D View ──
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("3D VIEW — {}", entity_name))
                    .font(small.clone())
                    .color(LABEL_DIM)
                    .strong(),
            );
        });
        ui.add_space(2.0);

        // Collect transforms for all entities
        let mut entity_positions: Vec<(u64, String, [f32; 3], bool)> = Vec::new();
        for e in &self.ecs_inspector.entities {
            let transform_key = (
                e.id,
                "bevy_transform::components::transform::Transform".to_string(),
            );
            if let Some(val) = self.ecs_inspector.component_values.get(&transform_key) {
                let translation = val
                    .get("translation")
                    .and_then(|t| t.as_array())
                    .map(|a| {
                        [
                            a.get(0).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                            a.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                            a.get(2).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                        ]
                    })
                    .unwrap_or([0.0, 0.0, 0.0]);
                let name = e.name.clone().unwrap_or_default();
                entity_positions.push((e.id, name, translation, e.id == entity_id));
            }
        }

        // If selected entity has no cached transform yet, still show the view
        let selected_pos = entity_positions
            .iter()
            .find(|(id, _, _, _)| *id == entity_id)
            .map(|(_, _, pos, _)| *pos)
            .unwrap_or([0.0, 0.0, 0.0]);

        let available_w = ui.available_width();
        let view_h = ui.available_height();
        let (response, painter) = ui.allocate_painter(
            egui::vec2(available_w, view_h),
            egui::Sense::click_and_drag(),
        );
        let rect = response.rect;

        // Camera interaction
        if response.dragged_by(egui::PointerButton::Primary) {
            let delta = response.drag_delta();
            self.ecs_inspector.view_yaw += delta.x * 0.01;
            self.ecs_inspector.view_pitch += delta.y * 0.01;
            self.ecs_inspector.view_pitch = self.ecs_inspector.view_pitch.clamp(-1.4, 1.4);
        }
        let scroll = ui.input(|i| {
            if let Some(pos) = i.pointer.hover_pos() {
                if rect.contains(pos) {
                    i.smooth_scroll_delta.y
                } else {
                    0.0
                }
            } else {
                0.0
            }
        });
        if scroll != 0.0 {
            self.ecs_inspector.view_zoom *= 1.0 + scroll * 0.003;
            self.ecs_inspector.view_zoom = self.ecs_inspector.view_zoom.clamp(0.1, 10.0);
        }
        if response.double_clicked() {
            self.ecs_inspector.view_yaw = std::f32::consts::PI * 0.25;
            self.ecs_inspector.view_pitch = std::f32::consts::PI * 0.15;
            self.ecs_inspector.view_zoom = 1.0;
        }

        // Background
        painter.rect_filled(rect, 4.0, egui::Color32::from_rgb(20, 20, 25));

        // Grid
        let grid_color = egui::Color32::from_rgba_premultiplied(255, 255, 255, 12);
        for i in 0..=8 {
            let t = i as f32 / 8.0;
            painter.line_segment(
                [
                    egui::pos2(rect.min.x + t * rect.width(), rect.min.y),
                    egui::pos2(rect.min.x + t * rect.width(), rect.max.y),
                ],
                egui::Stroke::new(0.5, grid_color),
            );
            painter.line_segment(
                [
                    egui::pos2(rect.min.x, rect.min.y + t * rect.height()),
                    egui::pos2(rect.max.x, rect.min.y + t * rect.height()),
                ],
                egui::Stroke::new(0.5, grid_color),
            );
        }

        // 3D projection
        let cy = self.ecs_inspector.view_yaw.cos();
        let sy = self.ecs_inspector.view_yaw.sin();
        let cx = self.ecs_inspector.view_pitch.cos();
        let sx = self.ecs_inspector.view_pitch.sin();
        let zoom = self.ecs_inspector.view_zoom;
        let center_x = rect.center().x;
        let center_y = rect.center().y;
        let scale = (rect.width().min(rect.height()) * 0.03) * zoom;

        let project = |pos: [f32; 3]| -> egui::Pos2 {
            let x = pos[0] - selected_pos[0];
            let y = pos[1] - selected_pos[1];
            let z = pos[2] - selected_pos[2];
            let rx = x * cy - z * sy;
            let rz = x * sy + z * cy;
            let ry = y * cx - rz * sx;
            egui::pos2(center_x + rx * scale, center_y - ry * scale)
        };

        // Draw ground plane
        let ground_color = egui::Color32::from_rgba_premultiplied(100, 100, 100, 30);
        let g = 20.0;
        for i in -5..=5 {
            let f = i as f32 * (g / 5.0);
            let p1 = project([f + selected_pos[0], 0.0, -g + selected_pos[2]]);
            let p2 = project([f + selected_pos[0], 0.0, g + selected_pos[2]]);
            painter.line_segment([p1, p2], egui::Stroke::new(0.5, ground_color));
            let p3 = project([-g + selected_pos[0], 0.0, f + selected_pos[2]]);
            let p4 = project([g + selected_pos[0], 0.0, f + selected_pos[2]]);
            painter.line_segment([p3, p4], egui::Stroke::new(0.5, ground_color));
        }

        // Draw axes at origin
        let origin = project(selected_pos);
        let ax = project([selected_pos[0] + 2.0, selected_pos[1], selected_pos[2]]);
        let ay = project([selected_pos[0], selected_pos[1] + 2.0, selected_pos[2]]);
        let az = project([selected_pos[0], selected_pos[1], selected_pos[2] + 2.0]);
        painter.line_segment(
            [origin, ax],
            egui::Stroke::new(1.5, egui::Color32::from_rgb(220, 60, 60)),
        );
        painter.line_segment(
            [origin, ay],
            egui::Stroke::new(1.5, egui::Color32::from_rgb(60, 220, 60)),
        );
        painter.line_segment(
            [origin, az],
            egui::Stroke::new(1.5, egui::Color32::from_rgb(60, 60, 220)),
        );

        // Draw entities as boxes
        for (_eid, name, pos, is_selected) in &entity_positions {
            let s = 0.5_f32; // box half-size
            let corners = [
                [pos[0] - s, pos[1] - s, pos[2] - s],
                [pos[0] + s, pos[1] - s, pos[2] - s],
                [pos[0] + s, pos[1] + s, pos[2] - s],
                [pos[0] - s, pos[1] + s, pos[2] - s],
                [pos[0] - s, pos[1] - s, pos[2] + s],
                [pos[0] + s, pos[1] - s, pos[2] + s],
                [pos[0] + s, pos[1] + s, pos[2] + s],
                [pos[0] - s, pos[1] + s, pos[2] + s],
            ];
            let edges = [
                (0, 1),
                (1, 2),
                (2, 3),
                (3, 0),
                (4, 5),
                (5, 6),
                (6, 7),
                (7, 4),
                (0, 4),
                (1, 5),
                (2, 6),
                (3, 7),
            ];

            let (color, width) = if *is_selected {
                (egui::Color32::from_rgb(75, 180, 255), 2.0)
            } else {
                (
                    egui::Color32::from_rgba_premultiplied(150, 150, 150, 80),
                    1.0,
                )
            };

            let projected: Vec<egui::Pos2> = corners.iter().map(|c| project(*c)).collect();
            for (a, b) in &edges {
                painter.line_segment(
                    [projected[*a], projected[*b]],
                    egui::Stroke::new(width, color),
                );
            }

            // Label
            let label_pos = project([pos[0], pos[1] + s + 0.3, pos[2]]);
            let label_color = if *is_selected {
                egui::Color32::WHITE
            } else {
                egui::Color32::from_rgb(120, 120, 120)
            };
            if !name.is_empty() {
                painter.text(
                    label_pos,
                    egui::Align2::CENTER_BOTTOM,
                    name,
                    egui::FontId::proportional(if *is_selected { 11.0 } else { 9.0 }),
                    label_color,
                );
            }
        }

        // Controls hint
        painter.text(
            egui::pos2(rect.max.x - 4.0, rect.max.y - 4.0),
            egui::Align2::RIGHT_BOTTOM,
            "Drag: rotate | Scroll: zoom",
            egui::FontId::proportional(9.0),
            egui::Color32::from_rgb(60, 60, 60),
        );
    }

    /// Render properties only (right panel)
    pub(crate) fn render_ecs_properties_only(&mut self, ui: &mut egui::Ui) {
        let small = egui::FontId::proportional(11.0);

        if !self.ecs_inspector.connected {
            return;
        }

        let Some((entity_id, entity_name)) = self.selected_entity_info() else {
            ui.label(
                egui::RichText::new("No entity selected")
                    .font(small.clone())
                    .color(LABEL_DIM),
            );
            return;
        };

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("PROPERTIES — {}", entity_name))
                    .font(small.clone())
                    .color(LABEL_DIM)
                    .strong(),
            );
        });
        ui.add_space(2.0);
        ui.painter().line_segment(
            [
                egui::pos2(ui.min_rect().left(), ui.cursor().top()),
                egui::pos2(ui.min_rect().right(), ui.cursor().top()),
            ],
            egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 45, 45)),
        );
        ui.add_space(4.0);

        let keys: Vec<_> = self
            .ecs_inspector
            .component_values
            .keys()
            .filter(|(_eid, _)| *_eid == entity_id)
            .cloned()
            .collect();

        if keys.is_empty() {
            ui.label(
                egui::RichText::new("Loading...")
                    .font(small.clone())
                    .color(LABEL_DIM),
            );
            return;
        }

        egui::ScrollArea::vertical()
            .id_salt("ecs_properties_right")
            .show(ui, |ui| {
                for (_, comp_name) in &keys {
                    if let Some(_value) = self
                        .ecs_inspector
                        .component_values
                        .get(&(entity_id, comp_name.clone()))
                    {
                        let short_name = comp_name.rsplit("::").next().unwrap_or(comp_name);
                        let header_id = ui.make_persistent_id(format!("rprop_{}", comp_name));
                        egui::collapsing_header::CollapsingState::load_with_default_open(
                            ui.ctx(),
                            header_id,
                            true,
                        )
                        .show_header(ui, |ui| {
                            ui.label(
                                egui::RichText::new(short_name)
                                    .font(small.clone())
                                    .color(ACCENT_BLUE)
                                    .strong(),
                            );
                        })
                        .body(|ui| {
                            // Extract value, render editable, reinsert
                            let key = (entity_id, comp_name.clone());
                            if let Some(mut val) = self.ecs_inspector.component_values.remove(&key)
                            {
                                let did_change = Self::render_editable_properties(ui, &mut val, 0);
                                if did_change && self.ecs_inspector.connected {
                                    // Schedule BRP write-back with debouncing
                                    self.ecs_inspector.pending_write =
                                        Some((entity_id, comp_name.clone(), val.clone()));
                                    self.ecs_inspector.write_debounce_timer =
                                        Some(std::time::Instant::now());
                                }
                                self.ecs_inspector.component_values.insert(key, val);
                            }
                        });
                    }
                }
            });
    }

    /// Render JSON value as editable property grid. Returns true if any value changed.
    fn render_editable_properties(
        ui: &mut egui::Ui,
        value: &mut serde_json::Value,
        depth: usize,
    ) -> bool {
        let small = egui::FontId::proportional(11.0);
        let indent = depth as f32 * 12.0;
        let mut changed = false;

        match value {
            serde_json::Value::Object(map) => {
                let keys: Vec<String> = map.keys().cloned().collect();
                for key in keys {
                    let val = map.get_mut(&key).unwrap();

                    // Special handling for Vec3-like arrays (translation, scale, etc.)
                    if matches!(&key as &str, "translation" | "scale")
                        && val.is_array()
                        && val.as_array().map_or(false, |a| a.len() == 3)
                    {
                        ui.horizontal(|ui| {
                            ui.add_space(indent + 8.0);
                            ui.label(
                                egui::RichText::new(&key)
                                    .font(small.clone())
                                    .color(PROP_KEY),
                            );
                            if let Some(arr) = val.as_array_mut() {
                                for (i, label) in ["x:", "y:", "z:"].iter().enumerate() {
                                    if let Some(num) = arr[i].as_f64() {
                                        let mut v = num as f32;
                                        if ui
                                            .add(
                                                egui::DragValue::new(&mut v)
                                                    .speed(0.05)
                                                    .prefix(*label),
                                            )
                                            .changed()
                                        {
                                            arr[i] = serde_json::Value::from(v as f64);
                                            changed = true;
                                        }
                                    }
                                }
                            }
                        });
                        continue;
                    }

                    // Special: rotation (quaternion, 4 values)
                    if key == "rotation"
                        && val.is_array()
                        && val.as_array().map_or(false, |a| a.len() == 4)
                    {
                        ui.horizontal(|ui| {
                            ui.add_space(indent + 8.0);
                            ui.label(
                                egui::RichText::new(&key)
                                    .font(small.clone())
                                    .color(PROP_KEY),
                            );
                            if let Some(arr) = val.as_array_mut() {
                                for (i, label) in ["x:", "y:", "z:", "w:"].iter().enumerate() {
                                    if let Some(num) = arr[i].as_f64() {
                                        let mut v = num as f32;
                                        if ui
                                            .add(
                                                egui::DragValue::new(&mut v)
                                                    .speed(0.01)
                                                    .prefix(*label),
                                            )
                                            .changed()
                                        {
                                            arr[i] = serde_json::Value::from(v as f64);
                                            changed = true;
                                        }
                                    }
                                }
                            }
                        });
                        continue;
                    }

                    // Nested object or array → recurse
                    if val.is_object() || val.is_array() {
                        ui.horizontal(|ui| {
                            ui.add_space(indent + 8.0);
                            ui.label(
                                egui::RichText::new(&key)
                                    .font(small.clone())
                                    .color(PROP_KEY),
                            );
                        });
                        if Self::render_editable_properties(ui, val, depth + 1) {
                            changed = true;
                        }
                        continue;
                    }

                    // Scalar values with editable widgets
                    ui.horizontal(|ui| {
                        ui.add_space(indent + 8.0);
                        ui.label(
                            egui::RichText::new(&key)
                                .font(small.clone())
                                .color(PROP_KEY),
                        );

                        match val {
                            serde_json::Value::Number(n) => {
                                if let Some(f) = n.as_f64() {
                                    let mut v = f as f32;
                                    if ui.add(egui::DragValue::new(&mut v).speed(0.05)).changed() {
                                        *val = serde_json::Value::from(v as f64);
                                        changed = true;
                                    }
                                } else if let Some(i) = n.as_i64() {
                                    let mut v = i;
                                    if ui.add(egui::DragValue::new(&mut v).speed(1.0)).changed() {
                                        *val = serde_json::Value::from(v);
                                        changed = true;
                                    }
                                }
                            }
                            serde_json::Value::Bool(b) => {
                                if ui.checkbox(b, "").changed() {
                                    changed = true;
                                }
                            }
                            serde_json::Value::String(s) => {
                                if ui
                                    .add(egui::TextEdit::singleline(s).desired_width(120.0))
                                    .changed()
                                {
                                    changed = true;
                                }
                            }
                            serde_json::Value::Null => {
                                ui.label(
                                    egui::RichText::new("null")
                                        .font(small.clone())
                                        .color(LABEL_DIM),
                                );
                            }
                            _ => {}
                        }
                    });
                }
            }
            serde_json::Value::Array(arr) => {
                for (i, val) in arr.iter_mut().enumerate() {
                    if val.is_number() {
                        ui.horizontal(|ui| {
                            ui.add_space(indent + 8.0);
                            ui.label(
                                egui::RichText::new(format!("[{}]", i))
                                    .font(small.clone())
                                    .color(LABEL_DIM),
                            );
                            if let Some(f) = val.as_f64() {
                                let mut v = f as f32;
                                if ui.add(egui::DragValue::new(&mut v).speed(0.05)).changed() {
                                    *val = serde_json::Value::from(v as f64);
                                    changed = true;
                                }
                            }
                        });
                    } else {
                        if Self::render_editable_properties(ui, val, depth + 1) {
                            changed = true;
                        }
                    }
                }
            }
            _ => {
                ui.horizontal(|ui| {
                    ui.add_space(indent + 8.0);
                    match value {
                        serde_json::Value::Number(n) => {
                            if let Some(f) = n.as_f64() {
                                let mut v = f as f32;
                                if ui.add(egui::DragValue::new(&mut v).speed(0.05)).changed() {
                                    *value = serde_json::Value::from(v as f64);
                                    changed = true;
                                }
                            }
                        }
                        serde_json::Value::Bool(b) => {
                            if ui.checkbox(b, "").changed() {
                                changed = true;
                            }
                        }
                        _ => {
                            let text = value.to_string();
                            ui.label(
                                egui::RichText::new(text)
                                    .font(small.clone())
                                    .color(LABEL_BRIGHT),
                            );
                        }
                    }
                });
            }
        }

        changed
    }

    /// Legacy read-only render for backward compat
    fn render_json_properties(&self, ui: &mut egui::Ui, value: &serde_json::Value, depth: usize) {
        let small = egui::FontId::proportional(11.0);
        let indent = depth as f32 * 12.0;
        match value {
            serde_json::Value::Object(map) => {
                for (key, val) in map {
                    ui.horizontal(|ui| {
                        ui.add_space(indent + 8.0);
                        ui.label(egui::RichText::new(key).font(small.clone()).color(PROP_KEY));
                        if !val.is_object() && !val.is_array() {
                            let (text, color) = self.format_json_value(val);
                            ui.label(egui::RichText::new(text).font(small.clone()).color(color));
                        }
                    });
                    if val.is_object() || val.is_array() {
                        self.render_json_properties(ui, val, depth + 1);
                    }
                }
            }
            serde_json::Value::Array(arr) => {
                for (i, val) in arr.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.add_space(indent + 8.0);
                        ui.label(
                            egui::RichText::new(format!("[{}]", i))
                                .font(small.clone())
                                .color(LABEL_DIM),
                        );
                        let (text, color) = self.format_json_value(val);
                        ui.label(egui::RichText::new(text).font(small.clone()).color(color));
                    });
                }
            }
            _ => {
                ui.horizontal(|ui| {
                    ui.add_space(indent + 8.0);
                    let (text, color) = self.format_json_value(value);
                    ui.label(egui::RichText::new(text).font(small.clone()).color(color));
                });
            }
        }
    }

    /// Format a JSON scalar value with appropriate color
    fn format_json_value(&self, value: &serde_json::Value) -> (String, egui::Color32) {
        match value {
            serde_json::Value::Number(n) => {
                if let Some(f) = n.as_f64() {
                    (format!("{:.3}", f), PROP_VAL_NUM)
                } else {
                    (n.to_string(), PROP_VAL_NUM)
                }
            }
            serde_json::Value::String(s) => (format!("\"{}\"", s), PROP_VAL_STR),
            serde_json::Value::Bool(b) => (b.to_string(), ACCENT_BLUE),
            serde_json::Value::Null => ("null".to_string(), LABEL_DIM),
            _ => (value.to_string(), LABEL_BRIGHT),
        }
    }

    /// Render the Resources tab of the ECS Inspector.
    fn render_ecs_resources_tab(&mut self, ui: &mut egui::Ui) {
        let small = egui::FontId::proportional(11.0);
        let resource_count = self.ecs_inspector.resources.len();

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("RESOURCES ({})", resource_count))
                    .font(small.clone())
                    .color(LABEL_DIM)
                    .strong(),
            );
        });

        if resource_count == 0 {
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new("No resources discovered")
                    .font(small.clone())
                    .color(LABEL_DIM),
            );
            return;
        }

        let filter = self.ecs_inspector.filter_query.to_lowercase();

        egui::ScrollArea::vertical()
            .id_salt("resource_list")
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let resources_snapshot: Vec<_> = self
                    .ecs_inspector
                    .resources
                    .iter()
                    .map(|r| r.type_name.clone())
                    .collect();

                for type_name in &resources_snapshot {
                    if !filter.is_empty() && !type_name.to_lowercase().contains(&filter) {
                        continue;
                    }

                    let short_name = type_name.rsplit("::").next().unwrap_or(type_name);
                    let selected =
                        self.ecs_inspector.selected_resource.as_deref() == Some(type_name.as_str());

                    let row_rect = ui.available_rect_before_wrap();
                    let row_rect = egui::Rect::from_min_size(
                        row_rect.left_top(),
                        egui::vec2(row_rect.width(), 22.0),
                    );
                    let row_response = ui.allocate_rect(row_rect, egui::Sense::click());

                    let bg = if selected {
                        ITEM_SELECTED
                    } else if row_response.hovered() {
                        ITEM_HOVER
                    } else {
                        egui::Color32::TRANSPARENT
                    };
                    if bg != egui::Color32::TRANSPARENT {
                        ui.painter().rect_filled(row_rect, 0.0, bg);
                    }

                    let name_pos = row_rect.left_center() + egui::vec2(8.0, 0.0);
                    ui.painter().text(
                        name_pos,
                        egui::Align2::LEFT_CENTER,
                        short_name,
                        small.clone(),
                        if selected {
                            egui::Color32::WHITE
                        } else {
                            LABEL_BRIGHT
                        },
                    );

                    if row_response.clicked() {
                        self.ecs_inspector.selected_resource = Some(type_name.clone());
                    }
                }
            });
    }

    fn connect_to_bevy_app(&mut self) {
        let endpoint = self.ecs_inspector.endpoint.clone();
        let runtime = self.lsp_runtime.clone();
        let (tx, rx) = std::sync::mpsc::channel();

        runtime.spawn(async move {
            let mut client = crate::bevy_ide::inspector::brp_client::BrpClient::new(&endpoint);
            let result = client.ping().await;
            let _ = tx.send(result);
        });

        self.ecs_inspector.pending_connect = Some(rx);
        self.ecs_inspector.error_message = None;
    }

    fn refresh_ecs_data(&mut self) {
        if self.ecs_inspector.pending_entities.is_some() {
            return;
        }

        let endpoint = self.ecs_inspector.endpoint.clone();
        let runtime = self.lsp_runtime.clone();
        let (tx, rx) = std::sync::mpsc::channel();

        runtime.spawn(async move {
            let mut client = crate::bevy_ide::inspector::brp_client::BrpClient::new(&endpoint);
            let result = client.list_entities().await;
            let _ = tx.send(result);
        });

        self.ecs_inspector.pending_entities = Some(rx);
        self.ecs_inspector.poll_start = Some(std::time::Instant::now());
    }

    fn load_entity_components(&mut self, entity_id: u64) {
        if self.ecs_inspector.pending_components.is_some() {
            return;
        }

        let component_names: Vec<String> = self
            .ecs_inspector
            .entities
            .iter()
            .find(|e| e.id == entity_id)
            .map(|e| e.components.clone())
            .unwrap_or_default();

        let endpoint = self.ecs_inspector.endpoint.clone();
        let runtime = self.lsp_runtime.clone();
        let (tx, rx) = std::sync::mpsc::channel();

        runtime.spawn(async move {
            let mut client = crate::bevy_ide::inspector::brp_client::BrpClient::new(&endpoint);
            let result = client
                .get_entity_components(entity_id, &component_names)
                .await;
            let _ = tx.send(result);
        });

        self.ecs_inspector.pending_components = Some(rx);
    }

    /// Poll pending async results — call this every frame
    pub(crate) fn poll_ecs_inspector(&mut self) {
        if let Some(rx) = &self.ecs_inspector.pending_connect {
            match rx.try_recv() {
                Ok(true) => {
                    self.ecs_inspector.connected = true;
                    self.ecs_inspector.error_message = None;
                    self.ecs_inspector.pending_connect = None;
                    self.refresh_ecs_data();
                }
                Ok(false) => {
                    self.ecs_inspector.connected = false;
                    self.ecs_inspector.error_message = Some(
                        "Connection failed — is the Bevy app running with RemotePlugin?"
                            .to_string(),
                    );
                    self.ecs_inspector.pending_connect = None;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.ecs_inspector.connected = false;
                    self.ecs_inspector.error_message = Some("Connection failed".to_string());
                    self.ecs_inspector.pending_connect = None;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
            }
        }

        if let Some(rx) = &self.ecs_inspector.pending_entities {
            match rx.try_recv() {
                Ok(Ok(entities)) => {
                    // Measure poll latency
                    if let Some(start) = self.ecs_inspector.poll_start.take() {
                        let latency = start.elapsed().as_secs_f64() * 1000.0;
                        self.ecs_inspector.perf_poll_latency_ms = latency;
                        self.ecs_inspector.perf_latency_history.push_back(latency);
                        if self.ecs_inspector.perf_latency_history.len() > 60 {
                            self.ecs_inspector.perf_latency_history.pop_front();
                        }
                    }
                    self.ecs_inspector.perf_entity_count = entities.len();
                    self.ecs_inspector.entities = entities;
                    self.ecs_inspector.error_message = None;
                    self.ecs_inspector.last_poll = Some(std::time::Instant::now());
                    self.ecs_inspector.pending_entities = None;
                }
                Ok(Err(e)) => {
                    self.ecs_inspector.error_message = Some(format!("Refresh failed: {}", e));
                    self.ecs_inspector.pending_entities = None;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.ecs_inspector.pending_entities = None;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
            }
        }

        if let Some(rx) = &self.ecs_inspector.pending_components {
            match rx.try_recv() {
                Ok(Ok(components)) => {
                    if let Some(entity_id) = self.ecs_inspector.selected_entity {
                        for (name, value) in components {
                            self.ecs_inspector
                                .component_values
                                .insert((entity_id, name), value);
                        }
                    }
                    self.ecs_inspector.pending_components = None;
                }
                Ok(Err(e)) => {
                    self.ecs_inspector.error_message =
                        Some(format!("Failed to load components: {}", e));
                    self.ecs_inspector.pending_components = None;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.ecs_inspector.pending_components = None;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
            }
        }

        if self.ecs_inspector.connected
            && self.ecs_inspector.auto_refresh
            && self.ecs_inspector.pending_entities.is_none()
        {
            let should_refresh = self.ecs_inspector.last_poll.map_or(true, |t| {
                t.elapsed().as_millis() >= self.ecs_inspector.poll_interval_ms as u128
            });
            if should_refresh {
                self.refresh_ecs_data();
            }
        }

        // Poll write-back result
        if let Some(rx) = &self.ecs_inspector.pending_write_result {
            match rx.try_recv() {
                Ok(Ok(())) => {
                    self.ecs_inspector.write_error = None;
                    self.ecs_inspector.pending_write_result = None;
                }
                Ok(Err(e)) => {
                    self.ecs_inspector.write_error = Some(format!("Write failed: {}", e));
                    self.ecs_inspector.pending_write_result = None;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.ecs_inspector.pending_write_result = None;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
            }
        }

        // Debounced write-back: send BRP insert after 100ms of no changes
        if let Some(timer) = self.ecs_inspector.write_debounce_timer {
            if timer.elapsed().as_millis() >= 100
                && self.ecs_inspector.pending_write_result.is_none()
            {
                if let Some((entity_id, comp_name, value)) = self.ecs_inspector.pending_write.take()
                {
                    self.ecs_inspector.write_debounce_timer = None;
                    let endpoint = self.ecs_inspector.endpoint.clone();
                    let (tx, rx) = std::sync::mpsc::channel();
                    {
                        let rt_handle = std::sync::Arc::clone(&self.lsp_runtime);
                        std::thread::spawn(move || {
                            rt_handle.block_on(async {
                                let mut client =
                                    crate::bevy_ide::inspector::brp_client::BrpClient::new(
                                        &endpoint,
                                    );
                                let components = serde_json::json!({ comp_name: value });
                                let result = client.insert_component(entity_id, components).await;
                                let _ = tx.send(result);
                            });
                        });
                        self.ecs_inspector.pending_write_result = Some(rx);
                    }
                }
            }
        }
    }
}
