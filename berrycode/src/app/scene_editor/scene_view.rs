//! Scene View panel: 3D viewport for the scene editor.
//!
//! The actual 3D rendering happens in Bevy via [`super::bevy_render`] and
//! [`super::bevy_sync`]. This module:
//! - Displays the resulting render-target texture inside an egui rect.
//! - Handles **orbit** (LMB drag on empty space) and **zoom** (scroll).
//! - Implements **click-to-select** via Rust-side ray vs AABB picking
//!   (see [`super::gizmo`]).
//! - Draws a yellow wireframe AABB around the selected entity.
//! - Draws an in-viewport **transform gizmo** (Move / Rotate / Scale, toggled
//!   with W / E / R) and applies axis-locked drags to the selected entity.

use crate::app::scene_editor::gizmo::{
    aabb_for_entity, camera_position, project_to_screen, ray_aabb_hit, screen_to_ray, GizmoDrag,
    GizmoMode,
};
use crate::app::BerryCodeApp;
use bevy::math::Vec3;

/// Independent camera state stored per quad-view quadrant. When the user
/// clicks a quadrant the active camera parameters are swapped to/from the
/// corresponding slot, giving the UX of four saved viewpoints.
#[derive(Debug, Clone)]
pub struct QuadCameraState {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub ortho: bool,
    pub ortho_scale: f32,
    pub target: [f32; 3],
    pub label: String,
}

impl QuadCameraState {
    /// Four default quadrants: Perspective, Front, Right, Top.
    pub fn defaults() -> [QuadCameraState; 4] {
        [
            QuadCameraState {
                yaw: std::f32::consts::FRAC_PI_4,
                pitch: 0.5,
                distance: 8.0,
                ortho: false,
                ortho_scale: 8.0,
                target: [0.0, 0.0, 0.0],
                label: "Persp".into(),
            },
            QuadCameraState {
                yaw: 0.0,
                pitch: 0.0,
                distance: 8.0,
                ortho: true,
                ortho_scale: 8.0,
                target: [0.0, 0.0, 0.0],
                label: "Front".into(),
            },
            QuadCameraState {
                yaw: std::f32::consts::FRAC_PI_2,
                pitch: 0.0,
                distance: 8.0,
                ortho: true,
                ortho_scale: 8.0,
                target: [0.0, 0.0, 0.0],
                label: "Right".into(),
            },
            QuadCameraState {
                yaw: 0.0,
                pitch: std::f32::consts::FRAC_PI_2 - 0.001,
                distance: 8.0,
                ortho: true,
                ortho_scale: 8.0,
                target: [0.0, 0.0, 0.0],
                label: "Top".into(),
            },
        ]
    }
}

impl BerryCodeApp {
    /// Render the Scene View (3D viewport).
    pub(crate) fn render_scene_view(&mut self, ui: &mut egui::Ui) {
        // --- VS Code-style panel header ---
        let header_rect = ui.available_rect_before_wrap();
        let header_rect =
            egui::Rect::from_min_size(header_rect.min, egui::vec2(header_rect.width(), 28.0));
        ui.painter().rect_filled(
            header_rect,
            0.0,
            egui::Color32::from_rgb(37, 37, 38), // VS Code panel header bg
        );
        // Bottom border
        ui.painter().line_segment(
            [header_rect.left_bottom(), header_rect.right_bottom()],
            egui::Stroke::new(1.0, egui::Color32::from_rgb(54, 57, 59)),
        );
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(header_rect), |ui| {
            ui.horizontal_centered(|ui| {
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("SCENE VIEW")
                        .size(11.0)
                        .color(egui::Color32::from_rgb(187, 187, 187)),
                );
            });
        });
        ui.advance_cursor_after_rect(header_rect);
        ui.add_space(2.0);

        // --- VS Code-style flat toolbar ---
        // Style overrides for the toolbar area
        let toolbar_bg = egui::Color32::from_rgb(37, 37, 38);
        let hover_bg = egui::Color32::from_rgb(50, 50, 52);
        let active_bg = egui::Color32::from_rgb(60, 60, 64);
        let text_normal = egui::Color32::from_rgb(204, 204, 204);
        let text_dim = egui::Color32::from_rgb(130, 130, 130);
        let text_active = egui::Color32::WHITE;
        let sep_color = egui::Color32::from_rgb(54, 57, 59);
        let font = egui::FontId::proportional(11.5);
        let row_h = 24.0;

        // Helper: flat toolbar button (returns true if clicked)
        let flat_btn = |ui: &mut egui::Ui, label: &str, selected: bool, enabled: bool| -> bool {
            let _text = if !enabled {
                egui::RichText::new(label)
                    .font(font.clone())
                    .color(text_dim)
            } else if selected {
                egui::RichText::new(label)
                    .font(font.clone())
                    .color(text_active)
            } else {
                egui::RichText::new(label)
                    .font(font.clone())
                    .color(text_normal)
            };
            let galley = ui
                .painter()
                .layout_no_wrap(label.to_string(), font.clone(), text_normal);
            let btn_w = galley.size().x + 12.0;
            let (rect, resp) =
                ui.allocate_exact_size(egui::vec2(btn_w, row_h), egui::Sense::click());
            if selected {
                ui.painter().rect_filled(rect, 2.0, active_bg);
            } else if resp.hovered() && enabled {
                ui.painter().rect_filled(rect, 2.0, hover_bg);
            }
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                label,
                font.clone(),
                if !enabled {
                    text_dim
                } else if selected {
                    text_active
                } else {
                    text_normal
                },
            );
            resp.clicked() && enabled
        };

        // Helper: thin vertical separator
        let thin_sep = |ui: &mut egui::Ui| {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(1.0, row_h), egui::Sense::hover());
            ui.painter().rect_filled(rect, 0.0, sep_color);
            ui.add_space(2.0);
        };

        // Helper: flat toggle (checkbox-like, but flat text)
        let flat_toggle = |ui: &mut egui::Ui, val: &mut bool, label: &str| -> bool {
            let prefix = if *val { "\u{eab4} " } else { "\u{eab6} " }; // chevron-down / right
            let display = format!("{}{}", prefix, label);
            let galley = ui
                .painter()
                .layout_no_wrap(display.clone(), font.clone(), text_normal);
            let btn_w = galley.size().x + 12.0;
            let (rect, resp) =
                ui.allocate_exact_size(egui::vec2(btn_w, row_h), egui::Sense::click());
            if *val {
                ui.painter().rect_filled(rect, 2.0, active_bg);
            } else if resp.hovered() {
                ui.painter().rect_filled(rect, 2.0, hover_bg);
            }
            // icon
            ui.painter().text(
                egui::pos2(rect.left() + 6.0, rect.center().y),
                egui::Align2::LEFT_CENTER,
                if *val { "\u{eab4}" } else { "\u{eab6}" },
                egui::FontId::new(9.0, egui::FontFamily::Name("codicon".into())),
                if *val { text_active } else { text_dim },
            );
            // label text
            ui.painter().text(
                egui::pos2(rect.left() + 18.0, rect.center().y),
                egui::Align2::LEFT_CENTER,
                label,
                font.clone(),
                if *val { text_active } else { text_normal },
            );
            if resp.clicked() {
                *val = !*val;
                true
            } else {
                false
            }
        };

        // --- Toolbar row 1: Transform + View ---
        let tb_rect = ui.available_rect_before_wrap();
        let tb_rect = egui::Rect::from_min_size(tb_rect.min, egui::vec2(tb_rect.width(), row_h));
        ui.painter().rect_filled(tb_rect, 0.0, toolbar_bg);
        ui.painter().line_segment(
            [tb_rect.left_bottom(), tb_rect.right_bottom()],
            egui::Stroke::new(1.0, sep_color),
        );
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(tb_rect), |ui| {
            ui.horizontal_centered(|ui| {
                ui.add_space(4.0);
                ui.spacing_mut().item_spacing.x = 1.0;

                let mode = self.gizmo_mode;
                if flat_btn(ui, "Move", mode == GizmoMode::Move, true) {
                    self.gizmo_mode = GizmoMode::Move;
                }
                if flat_btn(ui, "Rotate", mode == GizmoMode::Rotate, true) {
                    self.gizmo_mode = GizmoMode::Rotate;
                }
                if flat_btn(ui, "Scale", mode == GizmoMode::Scale, true) {
                    self.gizmo_mode = GizmoMode::Scale;
                }

                ui.add_space(2.0);
                thin_sep(ui);

                flat_toggle(ui, &mut self.snap_enabled, "Snap");
                if self.snap_enabled {
                    ui.add(
                        egui::DragValue::new(&mut self.snap_value)
                            .speed(0.1)
                            .range(0.1..=10.0)
                            .prefix("step:")
                            .custom_formatter(|v, _| format!("{:.1}", v)),
                    );
                }

                ui.add_space(2.0);
                thin_sep(ui);

                if flat_btn(ui, "Persp", !self.scene_ortho, true) {
                    self.scene_ortho = false;
                }
                if flat_btn(ui, "Ortho", self.scene_ortho, true) {
                    self.scene_ortho = true;
                }

                ui.add_space(2.0);
                thin_sep(ui);

                flat_toggle(ui, &mut self.scene_shadows_enabled, "Shadows");
                flat_toggle(ui, &mut self.scene_bloom_enabled, "Bloom");
                if self.scene_bloom_enabled {
                    ui.add(
                        egui::DragValue::new(&mut self.scene_bloom_intensity)
                            .speed(0.01)
                            .range(0.0..=1.0)
                            .custom_formatter(|v, _| format!("{:.2}", v)),
                    );
                }

                ui.add_space(2.0);
                thin_sep(ui);

                egui::ComboBox::from_id_salt("tonemapping_combo")
                    .selected_text(match self.scene_tonemapping {
                        0 => "None",
                        1 => "Reinhard",
                        2 => "ReinhardLum",
                        3 => "ACES",
                        4 => "AgX",
                        _ => "ACES",
                    })
                    .width(65.0)
                    .show_ui(ui, |ui| {
                        for (idx, name) in [
                            (0u8, "None"),
                            (1, "Reinhard"),
                            (2, "ReinhardLum"),
                            (3, "ACES"),
                            (4, "AgX"),
                        ] {
                            if ui
                                .selectable_label(self.scene_tonemapping == idx, name)
                                .clicked()
                            {
                                self.scene_tonemapping = idx;
                            }
                        }
                    });

                ui.add_space(2.0);
                thin_sep(ui);

                flat_toggle(ui, &mut self.scene_ssao_enabled, "SSAO");
                flat_toggle(ui, &mut self.scene_taa_enabled, "TAA");
                flat_toggle(ui, &mut self.scene_fog_enabled, "Fog");
                flat_toggle(ui, &mut self.scene_dof_enabled, "DoF");
            });
        });
        ui.advance_cursor_after_rect(tb_rect);

        // --- Toolbar row 2: Views + Play + Undo/Redo + Brush ---
        let tb2_rect = ui.available_rect_before_wrap();
        let tb2_rect = egui::Rect::from_min_size(tb2_rect.min, egui::vec2(tb2_rect.width(), row_h));
        ui.painter().rect_filled(tb2_rect, 0.0, toolbar_bg);
        ui.painter().line_segment(
            [tb2_rect.left_bottom(), tb2_rect.right_bottom()],
            egui::Stroke::new(1.0, sep_color),
        );
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(tb2_rect), |ui| {
            ui.horizontal_centered(|ui| {
                ui.add_space(4.0);
                ui.spacing_mut().item_spacing.x = 1.0;

                if flat_btn(ui, "Front", false, true) {
                    self.scene_orbit_yaw = 0.0;
                    self.scene_orbit_pitch = 0.0;
                    self.scene_ortho = true;
                }
                if flat_btn(ui, "Right", false, true) {
                    self.scene_orbit_yaw = std::f32::consts::FRAC_PI_2;
                    self.scene_orbit_pitch = 0.0;
                    self.scene_ortho = true;
                }
                if flat_btn(ui, "Top", false, true) {
                    self.scene_orbit_yaw = 0.0;
                    self.scene_orbit_pitch = std::f32::consts::FRAC_PI_2 - 0.001;
                    self.scene_ortho = true;
                }
                if flat_btn(ui, "Quad", self.quad_view_enabled, true) {
                    self.quad_view_enabled = !self.quad_view_enabled;
                }

                ui.add_space(2.0);
                thin_sep(ui);

                // Play mode controls
                match self.play_mode {
                    super::play_mode::PlayModeState::Stopped => {
                        if flat_btn(ui, "\u{eb2c} Play", false, true) {
                            self.play_mode_start();
                        }
                    }
                    super::play_mode::PlayModeState::Playing => {
                        if flat_btn(ui, "\u{eb2d} Pause", false, true) {
                            self.play_mode_pause();
                        }
                        if flat_btn(ui, "\u{eb2e} Stop", false, true) {
                            self.play_mode_stop();
                        }
                        ui.painter().text(
                            egui::pos2(
                                ui.available_rect_before_wrap().left() + 4.0,
                                tb2_rect.center().y,
                            ),
                            egui::Align2::LEFT_CENTER,
                            "Playing",
                            font.clone(),
                            egui::Color32::from_rgb(80, 200, 80),
                        );
                        ui.add_space(50.0);
                    }
                    super::play_mode::PlayModeState::Paused => {
                        if flat_btn(ui, "\u{eb2c} Resume", false, true) {
                            self.play_mode_resume();
                        }
                        if flat_btn(ui, "Step", false, true) {
                            self.play_mode_step();
                        }
                        if flat_btn(ui, "\u{eb2e} Stop", false, true) {
                            self.play_mode_stop();
                        }
                        ui.painter().text(
                            egui::pos2(
                                ui.available_rect_before_wrap().left() + 4.0,
                                tb2_rect.center().y,
                            ),
                            egui::Align2::LEFT_CENTER,
                            "Paused",
                            font.clone(),
                            egui::Color32::from_rgb(255, 200, 80),
                        );
                        ui.add_space(50.0);
                    }
                }

                ui.add_space(2.0);
                thin_sep(ui);

                let can_undo = self.command_history.can_undo();
                if flat_btn(ui, "Undo", false, can_undo) {
                    if let Some(prev) = self.command_history.undo(&self.scene_model) {
                        self.scene_model = prev;
                        self.scene_needs_sync = true;
                    }
                }
                let can_redo = self.command_history.can_redo();
                if flat_btn(ui, "Redo", false, can_redo) {
                    if let Some(next) = self.command_history.redo(&self.scene_model) {
                        self.scene_model = next;
                        self.scene_needs_sync = true;
                    }
                }

                ui.add_space(2.0);
                thin_sep(ui);

                flat_toggle(ui, &mut self.terrain_brush.active, "Brush");
                if self.terrain_brush.active {
                    egui::ComboBox::from_id_salt("terrain_brush_mode")
                        .selected_text(self.terrain_brush.mode.label())
                        .width(60.0)
                        .show_ui(ui, |ui| {
                            for &m in super::terrain::BrushMode::ALL {
                                ui.selectable_value(&mut self.terrain_brush.mode, m, m.label());
                            }
                        });
                    ui.add(
                        egui::DragValue::new(&mut self.terrain_brush.radius)
                            .prefix("R:")
                            .speed(0.5)
                            .range(0.5..=50.0),
                    );
                    ui.add(
                        egui::DragValue::new(&mut self.terrain_brush.strength)
                            .prefix("S:")
                            .speed(0.1)
                            .range(0.1..=10.0),
                    );
                }

                // Fog/DoF detail controls (shown inline when expanded)
                if self.scene_fog_enabled {
                    ui.add_space(2.0);
                    thin_sep(ui);
                    ui.color_edit_button_rgb(&mut self.scene_fog_color);
                    ui.add(
                        egui::DragValue::new(&mut self.scene_fog_start)
                            .prefix("s:")
                            .speed(1.0),
                    );
                    ui.add(
                        egui::DragValue::new(&mut self.scene_fog_end)
                            .prefix("e:")
                            .speed(1.0),
                    );
                }
                if self.scene_dof_enabled {
                    ui.add_space(2.0);
                    thin_sep(ui);
                    ui.add(
                        egui::DragValue::new(&mut self.scene_dof_focus_distance)
                            .prefix("f:")
                            .speed(0.5)
                            .range(0.1..=1000.0),
                    );
                    ui.add(
                        egui::DragValue::new(&mut self.scene_dof_aperture)
                            .prefix("f/")
                            .speed(0.01)
                            .range(0.001..=64.0),
                    );
                }
            });
        });
        ui.advance_cursor_after_rect(tb2_rect);
        ui.add_space(2.0);

        // Mark the scene dirty so the Bevy sync system runs (the hash check
        // there will short-circuit if nothing actually changed).
        self.scene_needs_sync = true;

        // Keyboard shortcuts for gizmo mode (don't fire while a text field is
        // focused or while fly mode is active, since WASD is used for movement).
        if !self.fly_mode_active {
            let keymap = self.keymap.clone();
            let (key_move, key_rotate, key_scale) = ui.input(|i| {
                (
                    keymap.is_pressed(crate::app::keymap::KeyAction::GizmoMove, i),
                    keymap.is_pressed(crate::app::keymap::KeyAction::GizmoRotate, i),
                    keymap.is_pressed(crate::app::keymap::KeyAction::GizmoScale, i),
                )
            });
            if key_move {
                self.gizmo_mode = GizmoMode::Move;
            }
            if key_rotate {
                self.gizmo_mode = GizmoMode::Rotate;
            }
            if key_scale {
                self.gizmo_mode = GizmoMode::Scale;
            }
        }

        let Some(tex_id) = self.scene_view_texture_id else {
            // Loading placeholder.
            let entity_count = self.scene_model.entities.len();
            let root_count = self.scene_model.root_entities.len();
            egui::Frame::NONE
                .fill(egui::Color32::from_rgb(18, 19, 21))
                .inner_margin(egui::Margin::same(16))
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(24.0);
                        ui.label(
                            egui::RichText::new("Initializing scene view...")
                                .size(16.0)
                                .color(egui::Color32::from_gray(200)),
                        );
                        ui.add_space(12.0);
                        ui.label(format!(
                            "Entities: {}   Roots: {}",
                            entity_count, root_count
                        ));
                    });
                });
            return;
        };

        // Allocate a rect that preserves the render target's 4:3 aspect ratio.
        let available = ui.available_size();
        let aspect = 1024.0 / 768.0;
        let display_w = available.x.min(available.y * aspect);
        let display_h = display_w / aspect;
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(display_w, display_h),
            egui::Sense::click_and_drag(),
        );

        // Draw the render-target texture.
        ui.painter().image(
            tex_id,
            rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );

        // Visual feedback while an asset is being dragged from the file tree.
        if let Some(asset) = &self.dragged_asset_path {
            let label = format!(
                "Drop to place: {}",
                std::path::Path::new(asset)
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default()
            );
            let text_pos = egui::pos2(rect.left() + 12.0, rect.top() + 12.0);
            ui.painter().rect_filled(
                egui::Rect::from_min_size(text_pos - egui::vec2(4.0, 2.0), egui::vec2(280.0, 22.0)),
                4.0,
                egui::Color32::from_rgba_premultiplied(0, 0, 0, 180),
            );
            ui.painter().text(
                text_pos + egui::vec2(4.0, 2.0),
                egui::Align2::LEFT_TOP,
                label,
                egui::FontId::proportional(13.0),
                egui::Color32::from_rgb(200, 220, 255),
            );
        }

        // Quad View: when enabled, split the viewport into 2x2 quadrants.
        // Each quadrant stores an independent camera state. Clicking a
        // quadrant saves the current camera params back to the active slot
        // and loads the clicked quadrant's state.
        if self.quad_view_enabled {
            let half_w = rect.width() * 0.5;
            let half_h = rect.height() * 0.5;
            let quad_rects: [egui::Rect; 4] = [
                egui::Rect::from_min_size(rect.min, egui::vec2(half_w, half_h)),
                egui::Rect::from_min_size(
                    rect.min + egui::vec2(half_w, 0.0),
                    egui::vec2(half_w, half_h),
                ),
                egui::Rect::from_min_size(
                    rect.min + egui::vec2(0.0, half_h),
                    egui::vec2(half_w, half_h),
                ),
                egui::Rect::from_min_size(
                    rect.min + egui::vec2(half_w, half_h),
                    egui::vec2(half_w, half_h),
                ),
            ];

            let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
            for (i, qr) in quad_rects.iter().enumerate() {
                // Draw the same texture in each quadrant.
                ui.painter().image(tex_id, *qr, uv, egui::Color32::WHITE);
                // Label overlay.
                let label = &self.quad_camera_states[i].label;
                let is_active = i == self.active_quad_idx;
                let label_color = if is_active {
                    egui::Color32::from_rgb(100, 180, 255)
                } else {
                    egui::Color32::from_rgba_premultiplied(200, 200, 200, 180)
                };
                ui.painter().text(
                    qr.left_top() + egui::vec2(6.0, 4.0),
                    egui::Align2::LEFT_TOP,
                    label,
                    egui::FontId::proportional(11.0),
                    label_color,
                );
                // Border: active quadrant gets a blue highlight, others get gray.
                let border_color = if is_active {
                    egui::Color32::from_rgb(80, 150, 255)
                } else {
                    egui::Color32::from_gray(60)
                };
                let border_width = if is_active { 2.0 } else { 1.0 };
                ui.painter().rect_stroke(
                    *qr,
                    0.0,
                    egui::Stroke::new(border_width, border_color),
                    egui::StrokeKind::Middle,
                );
            }

            // Click in a quadrant to switch the camera to that quadrant's
            // independent state.
            if response.clicked() {
                if let Some(click_pos) = response.interact_pointer_pos() {
                    for (i, qr) in quad_rects.iter().enumerate() {
                        if qr.contains(click_pos) && i != self.active_quad_idx {
                            // Save current camera state back to the active slot.
                            self.save_camera_to_quad(self.active_quad_idx);
                            // Load the clicked quadrant's state.
                            self.load_camera_from_quad(i);
                            self.active_quad_idx = i;
                            break;
                        }
                    }
                }
            }

            // Scroll to zoom even in quad view (applies to the active quadrant).
            if response.hovered() {
                let scroll = ui.input(|i| i.smooth_scroll_delta.y);
                if scroll.abs() > 0.0 {
                    if self.scene_ortho {
                        self.scene_ortho_scale *= 1.0 - scroll * 0.005;
                        self.scene_ortho_scale = self.scene_ortho_scale.clamp(0.5, 100.0);
                    } else {
                        self.scene_orbit_distance *= 1.0 - scroll * 0.005;
                        self.scene_orbit_distance = self.scene_orbit_distance.clamp(2.0, 50.0);
                    }
                }
            }

            return;
        }

        // Reconstruct the camera position from orbit params (must match
        // `update_scene_editor_camera`).
        let yaw = self.scene_orbit_yaw;
        let pitch = self.scene_orbit_pitch;
        let dist = self.scene_orbit_distance;
        let orbit_target = Vec3::from_array(self.scene_orbit_target);
        let cam_pos = camera_position(yaw, pitch, dist, orbit_target);
        let ortho = self.scene_ortho;
        let ortho_scale = self.scene_ortho_scale;

        // World-space grid overlay on the Y=0 plane.
        draw_world_grid(
            ui.painter(),
            cam_pos,
            orbit_target,
            rect,
            ortho,
            ortho_scale,
        );

        // Collider wireframe overlay (green) for all entities with a Collider component.
        for (id, entity) in &self.scene_model.entities {
            let world_t = self.scene_model.compute_world_transform(*id);
            let pos = bevy::math::Vec3::from_array(world_t.translation);
            for component in &entity.components {
                if let crate::app::scene_editor::model::ComponentData::Collider { shape, .. } =
                    component
                {
                    match shape {
                        crate::app::scene_editor::model::ColliderShape::Box { half_extents } => {
                            let h = bevy::math::Vec3::from_array(*half_extents);
                            draw_wireframe_aabb(
                                ui.painter(),
                                pos - h,
                                pos + h,
                                cam_pos,
                                orbit_target,
                                rect,
                                ortho,
                                ortho_scale,
                                egui::Color32::from_rgb(80, 220, 80),
                            );
                        }
                        crate::app::scene_editor::model::ColliderShape::Sphere { radius } => {
                            // Approximate by an axis-aligned bounding box (cheap).
                            let h = bevy::math::Vec3::splat(*radius);
                            draw_wireframe_aabb(
                                ui.painter(),
                                pos - h,
                                pos + h,
                                cam_pos,
                                orbit_target,
                                rect,
                                ortho,
                                ortho_scale,
                                egui::Color32::from_rgb(80, 220, 80),
                            );
                        }
                        crate::app::scene_editor::model::ColliderShape::Capsule {
                            half_height,
                            radius,
                        } => {
                            let h = bevy::math::Vec3::new(*radius, *half_height + *radius, *radius);
                            draw_wireframe_aabb(
                                ui.painter(),
                                pos - h,
                                pos + h,
                                cam_pos,
                                orbit_target,
                                rect,
                                ortho,
                                ortho_scale,
                                egui::Color32::from_rgb(80, 220, 80),
                            );
                        }
                    }
                }
            }
            let _ = id;
        }

        // Spline curve overlay
        for (_id, entity) in &self.scene_model.entities {
            if !entity.enabled {
                continue;
            }
            let world_t = self.scene_model.compute_world_transform(entity.id);
            let offset = Vec3::from_array(world_t.translation);
            for component in &entity.components {
                if let crate::app::scene_editor::model::ComponentData::Spline { points, closed } =
                    component
                {
                    if points.len() < 2 {
                        continue;
                    }
                    let samples =
                        crate::app::scene_editor::spline::sample_spline(points, *closed, 20);
                    let mut prev_screen: Option<egui::Pos2> = None;
                    for sample in &samples {
                        let world_pt = Vec3::new(sample[0], sample[1], sample[2]) + offset;
                        if let Some(sp) = project_to_screen(
                            world_pt,
                            cam_pos,
                            orbit_target,
                            rect,
                            ortho,
                            ortho_scale,
                        ) {
                            if let Some(pp) = prev_screen {
                                ui.painter().line_segment(
                                    [pp, sp],
                                    egui::Stroke::new(2.0, egui::Color32::from_rgb(80, 220, 220)),
                                );
                            }
                            prev_screen = Some(sp);
                        }
                    }
                    // Draw control points
                    for pt in points {
                        let world_pt = Vec3::from_array(pt.position) + offset;
                        if let Some(sp) = project_to_screen(
                            world_pt,
                            cam_pos,
                            orbit_target,
                            rect,
                            ortho,
                            ortho_scale,
                        ) {
                            ui.painter().circle_filled(sp, 4.0, egui::Color32::WHITE);
                        }
                    }
                }
            }
        }

        // Skeleton bone overlay: for entities with SkinnedMesh,
        // draw bone hierarchy as lines between joints + circles at joints.
        for (_id, entity) in &self.scene_model.entities {
            if !entity.enabled {
                continue;
            }
            let world_t = self.scene_model.compute_world_transform(entity.id);
            let entity_pos = Vec3::from_array(world_t.translation);
            for component in &entity.components {
                if let crate::app::scene_editor::model::ComponentData::SkinnedMesh {
                    bones, ..
                } = component
                {
                    if bones.is_empty() {
                        continue;
                    }
                    // Compute world positions of each bone
                    let bone_positions: Vec<Vec3> = bones
                        .iter()
                        .map(|b| entity_pos + Vec3::from_array(b.bind_pose.translation))
                        .collect();

                    // Draw lines from each bone to its parent
                    for (i, bone) in bones.iter().enumerate() {
                        let pos = bone_positions[i];
                        if let Some(sp) =
                            project_to_screen(pos, cam_pos, orbit_target, rect, ortho, ortho_scale)
                        {
                            // Draw circle at joint
                            ui.painter().circle_filled(
                                sp,
                                4.0,
                                egui::Color32::from_rgb(255, 200, 50),
                            );
                            // Draw line to parent
                            if let Some(parent_idx) = bone.parent_idx {
                                if parent_idx < bone_positions.len() {
                                    let parent_pos = bone_positions[parent_idx];
                                    if let Some(pp) = project_to_screen(
                                        parent_pos,
                                        cam_pos,
                                        orbit_target,
                                        rect,
                                        ortho,
                                        ortho_scale,
                                    ) {
                                        ui.painter().line_segment(
                                            [pp, sp],
                                            egui::Stroke::new(
                                                2.0,
                                                egui::Color32::from_rgb(255, 200, 50),
                                            ),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // NavMesh grid overlay: draw green/red cells on Y=0 plane.
        for (_id, entity) in &self.scene_model.entities {
            if !entity.enabled {
                continue;
            }
            for component in &entity.components {
                if let crate::app::scene_editor::model::ComponentData::NavMesh {
                    cell_size,
                    grid,
                    width,
                    height,
                } = component
                {
                    if grid.is_empty() || *width == 0 || *height == 0 {
                        continue;
                    }
                    let world_half = (*width as f32 * cell_size) / 2.0;
                    for gz in 0..*height {
                        for gx in 0..*width {
                            let walkable = grid[gz * width + gx];
                            let wx = gx as f32 * cell_size - world_half + cell_size * 0.5;
                            let wz = gz as f32 * cell_size - world_half + cell_size * 0.5;
                            let world_pt = Vec3::new(wx, 0.01, wz);
                            if let Some(sp) = project_to_screen(
                                world_pt,
                                cam_pos,
                                orbit_target,
                                rect,
                                ortho,
                                ortho_scale,
                            ) {
                                let color = if walkable {
                                    egui::Color32::from_rgba_premultiplied(0, 180, 0, 40)
                                } else {
                                    egui::Color32::from_rgba_premultiplied(220, 0, 0, 60)
                                };
                                // Draw a small filled square
                                let half = 2.0;
                                let cell_rect =
                                    egui::Rect::from_center_size(sp, egui::Vec2::splat(half * 2.0));
                                if rect.contains(sp) {
                                    ui.painter().rect_filled(cell_rect, 0.0, color);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Selection wireframe AABB overlay for ALL selected entities.
        for &sel_id in &self.scene_model.selected_ids.clone() {
            if let Some(entity) = self.scene_model.entities.get(&sel_id) {
                let world_t = self.scene_model.compute_world_transform(sel_id);
                if let Some((amin, amax)) = aabb_for_entity(entity, &world_t) {
                    draw_wireframe_aabb(
                        ui.painter(),
                        amin,
                        amax,
                        cam_pos,
                        orbit_target,
                        rect,
                        ortho,
                        ortho_scale,
                        egui::Color32::YELLOW,
                    );
                }
            }
        }

        // Particle preview: tick the editor-side simulation and
        // splat live particles as 2D dots on top of the scene view. Drawn
        // before the gizmo so the gizmo handles stay on top.
        self.particle_preview.tick(&self.scene_model);
        let painter = ui.painter().clone();
        self.particle_preview
            .for_each_particle(&self.scene_model, |pos, t, component| {
                if let crate::app::scene_editor::model::ComponentData::ParticleEmitter {
                    start_size,
                    end_size,
                    start_color,
                    end_color,
                    ..
                } = component
                {
                    if let Some(screen) = crate::app::scene_editor::gizmo::project_to_screen(
                        pos,
                        cam_pos,
                        orbit_target,
                        rect,
                        ortho,
                        ortho_scale,
                    ) {
                        let size = lerp(*start_size, *end_size, t);
                        // Pixel size — scale by 200 so small world sizes are
                        // visible (heuristic; not physically accurate).
                        let pixel_radius = (size * 200.0).max(1.0);
                        let color = lerp_color(*start_color, *end_color, t);
                        painter.circle_filled(
                            screen,
                            pixel_radius,
                            egui::Color32::from_rgba_premultiplied(
                                (color[0] * color[3] * 255.0) as u8,
                                (color[1] * color[3] * 255.0) as u8,
                                (color[2] * color[3] * 255.0) as u8,
                                (color[3] * 255.0) as u8,
                            ),
                        );
                    }
                }
            });

        // Keep the scene view repainting so particles animate even when no
        // input is happening.
        ui.ctx().request_repaint();

        // tick simplified physics during play mode.
        if self.play_mode == super::play_mode::PlayModeState::Playing {
            self.physics_state.tick(&mut self.scene_model, true);
            self.scene_needs_sync = true;
        }

        // When play mode is active, disable editing (gizmo, selection, etc.).
        let editing_enabled = !self.play_mode.is_active();

        // Transform gizmo overlay + interaction. This may set
        // `gizmo_dragging`, which suppresses orbit & click-select below.
        if editing_enabled {
            if let Some(sel_id) = self.primary_selected_id {
                if self.scene_model.is_selected(sel_id) {
                    self.handle_gizmo(ui, &response, rect, cam_pos, sel_id);
                }
            }
        }

        // Click-to-select: only when no axis was hit (so we don't deselect by
        // clicking the gizmo) and not while orbit-dragging.
        if editing_enabled && response.clicked() && self.gizmo_dragging.is_none() {
            if let Some(click_pos) = response.interact_pointer_pos() {
                let (origin, dir) =
                    screen_to_ray(click_pos, cam_pos, orbit_target, rect, ortho, ortho_scale);
                let mut closest: Option<(u64, f32)> = None;
                for (id, entity) in &self.scene_model.entities {
                    if !entity.enabled {
                        continue;
                    }
                    let world_t = self.scene_model.compute_world_transform(*id);
                    if let Some((amin, amax)) = aabb_for_entity(entity, &world_t) {
                        if let Some(t) = ray_aabb_hit(origin, dir, amin, amax) {
                            if closest.map_or(true, |(_, ct)| t < ct) {
                                closest = Some((*id, t));
                            }
                        }
                    }
                }
                let modifiers = ui.input(|i| i.modifiers);
                if let Some((hit_id, _)) = closest {
                    if modifiers.shift {
                        self.scene_model.select_add(hit_id);
                    } else if modifiers.command {
                        self.scene_model.select_toggle(hit_id);
                    } else {
                        self.scene_model.select_only(hit_id);
                    }
                    self.primary_selected_id = Some(hit_id);
                } else if !modifiers.shift && !modifiers.command {
                    // Clicked empty space without modifiers: deselect all.
                    self.scene_model.select_clear();
                    self.primary_selected_id = None;
                }
            }
        }

        // Terrain brush: on LMB drag when brush is active and a terrain entity
        // is selected, apply the brush at the world-space Y=0 hit point.
        if self.terrain_brush.active
            && response.dragged_by(egui::PointerButton::Primary)
            && editing_enabled
        {
            if let Some(drag_pos) = response.interact_pointer_pos() {
                let (origin, dir) =
                    screen_to_ray(drag_pos, cam_pos, orbit_target, rect, ortho, ortho_scale);
                // Intersect ray with Y=0 plane
                if dir.y.abs() > 1e-6 {
                    let t_hit = -origin.y / dir.y;
                    if t_hit > 0.0 {
                        let hit_x = origin.x + dir.x * t_hit;
                        let hit_z = origin.z + dir.z * t_hit;
                        let brush_radius = self.terrain_brush.radius;
                        let brush_strength = self.terrain_brush.strength;
                        let brush_mode = self.terrain_brush.mode;

                        // Find selected terrain entity and apply brush
                        if let Some(sel_id) = self.primary_selected_id {
                            let has_terrain = self.scene_model.entities.get(&sel_id)
                                .map(|e| e.components.iter().any(|c| {
                                    matches!(c, crate::app::scene_editor::model::ComponentData::Terrain { .. })
                                }))
                                .unwrap_or(false);
                            if has_terrain {
                                if let Some(entity) = self.scene_model.entities.get_mut(&sel_id) {
                                    for component in &mut entity.components {
                                        if let crate::app::scene_editor::model::ComponentData::Terrain {
                                            resolution, world_size, heights, ..
                                        } = component
                                        {
                                            super::terrain::apply_brush(
                                                heights,
                                                *resolution,
                                                *world_size,
                                                hit_x,
                                                hit_z,
                                                brush_radius,
                                                brush_strength,
                                                brush_mode,
                                            );
                                            self.scene_needs_sync = true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Box selection: plain LMB drag on empty space (no gizmo, no Alt).
        // Disabled during play mode.
        let is_plain_lmb_drag = editing_enabled
            && response.dragged_by(egui::PointerButton::Primary)
            && !ui.input(|i| i.modifiers.alt)
            && self.gizmo_dragging.is_none();

        if response.drag_started_by(egui::PointerButton::Primary)
            && !ui.input(|i| i.modifiers.alt)
            && self.gizmo_dragging.is_none()
        {
            if let Some(pos) = response.interact_pointer_pos() {
                self.box_select_start = Some(pos);
            }
        }

        // Draw box selection rectangle.
        if let Some(start) = self.box_select_start {
            if is_plain_lmb_drag {
                if let Some(current) = ui.input(|i| i.pointer.hover_pos()) {
                    let box_rect = egui::Rect::from_two_pos(start, current);
                    ui.painter().rect_filled(
                        box_rect,
                        0.0,
                        egui::Color32::from_rgba_premultiplied(60, 120, 220, 40),
                    );
                    ui.painter().rect_stroke(
                        box_rect,
                        0.0,
                        egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 150, 255)),
                        egui::StrokeKind::Middle,
                    );
                }
            }
        }

        // On release, select entities inside the box.
        if response.drag_stopped_by(egui::PointerButton::Primary) {
            if let Some(start) = self.box_select_start.take() {
                if let Some(end) = response.interact_pointer_pos() {
                    let box_rect = egui::Rect::from_two_pos(start, end);
                    // Only treat as box select if dragged more than a few pixels.
                    if box_rect.width() > 5.0 || box_rect.height() > 5.0 {
                        let shift = ui.input(|i| i.modifiers.shift);
                        if !shift {
                            self.scene_model.select_clear();
                            self.primary_selected_id = None;
                        }
                        let entity_ids: Vec<u64> =
                            self.scene_model.entities.keys().copied().collect();
                        for id in entity_ids {
                            let is_enabled = self
                                .scene_model
                                .entities
                                .get(&id)
                                .map_or(false, |e| e.enabled);
                            if !is_enabled {
                                continue;
                            }
                            let world_t = self.scene_model.compute_world_transform(id);
                            let world_pos = Vec3::from_array(world_t.translation);
                            if let Some(screen_pos) = project_to_screen(
                                world_pos,
                                cam_pos,
                                orbit_target,
                                rect,
                                ortho,
                                ortho_scale,
                            ) {
                                if box_rect.contains(screen_pos) {
                                    self.scene_model.select_add(id);
                                    self.primary_selected_id = Some(id);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Clear box_select_start when pointer is released without drag.
        if !response.dragged() {
            self.box_select_start = None;
        }

        // Fly camera: RMB held inside the viewport.
        let rmb_held = response.dragged_by(egui::PointerButton::Secondary);
        if rmb_held && response.hovered() {
            self.fly_mode_active = true;
            let delta = response.drag_delta();
            self.scene_orbit_yaw += delta.x * 0.003;
            self.scene_orbit_pitch += delta.y * 0.003;
            self.scene_orbit_pitch = self.scene_orbit_pitch.clamp(-1.5, 1.5);

            // WASD / Q / E movement relative to the camera facing direction.
            let forward = Vec3::new(
                self.scene_orbit_yaw.sin() * self.scene_orbit_pitch.cos(),
                -self.scene_orbit_pitch.sin(),
                self.scene_orbit_yaw.cos() * self.scene_orbit_pitch.cos(),
            ) * -1.0;
            let right_dir = Vec3::new(self.scene_orbit_yaw.cos(), 0.0, -self.scene_orbit_yaw.sin());

            let speed = self.fly_camera_speed * 0.016; // assume ~60 fps
            let mut move_delta = Vec3::ZERO;
            ui.input(|i| {
                if i.key_down(egui::Key::W) {
                    move_delta += forward;
                }
                if i.key_down(egui::Key::S) {
                    move_delta -= forward;
                }
                if i.key_down(egui::Key::D) {
                    move_delta += right_dir;
                }
                if i.key_down(egui::Key::A) {
                    move_delta -= right_dir;
                }
                if i.key_down(egui::Key::Q) {
                    move_delta -= Vec3::Y;
                }
                if i.key_down(egui::Key::E) {
                    move_delta += Vec3::Y;
                }
            });

            if move_delta.length_squared() > 0.0 {
                let d = move_delta.normalize() * speed;
                self.scene_orbit_target[0] += d.x;
                self.scene_orbit_target[1] += d.y;
                self.scene_orbit_target[2] += d.z;
            }

            // Scroll wheel adjusts fly speed while in fly mode.
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll.abs() > 0.0 {
                self.fly_camera_speed =
                    (self.fly_camera_speed * (1.0 + scroll * 0.01)).clamp(0.5, 100.0);
            }
        }
        if !rmb_held {
            self.fly_mode_active = false;
        }

        // Orbit: middle mouse button OR Alt+LMB (suppressed during fly mode).
        let orbit_dragging = !self.fly_mode_active
            && (response.dragged_by(egui::PointerButton::Middle)
                || (response.dragged_by(egui::PointerButton::Primary)
                    && ui.input(|i| i.modifiers.alt)));
        if orbit_dragging && self.gizmo_dragging.is_none() {
            let delta = response.drag_delta();
            self.scene_orbit_yaw += delta.x * 0.01;
            self.scene_orbit_pitch += delta.y * 0.01;
            self.scene_orbit_pitch = self.scene_orbit_pitch.clamp(-1.5, 1.5);
        }

        // Release the gizmo axis lock when the drag ends.
        if !response.dragged() {
            self.gizmo_dragging = None;
        }

        // Asset drop target (disabled during play mode): if the user released the primary button over this
        // scene view rect while dragging an asset from the file tree, spawn a
        // new entity at the ray-vs-ground intersection point.
        let pointer_released = ui.input(|i| i.pointer.primary_released());
        if editing_enabled && pointer_released && self.dragged_asset_path.is_some() {
            // Only handle drops that landed inside the scene view rect.
            if let Some(drop_pos) = ui.input(|i| i.pointer.interact_pos()) {
                if rect.contains(drop_pos) {
                    if let Some(asset_path) = self.dragged_asset_path.take() {
                        let (origin, dir) = screen_to_ray(
                            drop_pos,
                            cam_pos,
                            orbit_target,
                            rect,
                            ortho,
                            ortho_scale,
                        );
                        // Intersect with Y=0 plane: origin.y + t * dir.y = 0
                        // -> t = -origin.y / dir.y. Fall back to the origin if
                        // the ray is nearly parallel to the ground or points
                        // away from it.
                        let spawn_pos = if dir.y.abs() > 1e-4 {
                            let t = -origin.y / dir.y;
                            if t > 0.0 {
                                origin + dir * t
                            } else {
                                Vec3::ZERO
                            }
                        } else {
                            Vec3::ZERO
                        };

                        if asset_path.to_lowercase().ends_with(".bprefab") {
                            // Instantiate prefab.
                            match crate::app::scene_editor::prefab::load_prefab(&asset_path) {
                                Ok(prefab) => {
                                    self.scene_snapshot();
                                    let new_root = crate::app::scene_editor::prefab::instantiate_prefab_from_path(
                                        &mut self.scene_model,
                                        &prefab,
                                        &asset_path,
                                    );
                                    if let Some(entity) =
                                        self.scene_model.entities.get_mut(&new_root)
                                    {
                                        entity.transform.translation =
                                            [spawn_pos.x, spawn_pos.y, spawn_pos.z];
                                    }
                                    self.scene_model.select_only(new_root);
                                    self.primary_selected_id = Some(new_root);
                                    self.scene_needs_sync = true;
                                    self.status_message =
                                        format!("Instantiated prefab: {}", asset_path);
                                    self.status_message_timestamp = Some(std::time::Instant::now());
                                    tracing::info!(
                                        "Instantiated prefab {} at {:?}",
                                        asset_path,
                                        spawn_pos
                                    );
                                }
                                Err(e) => {
                                    self.status_message = format!("Failed to load prefab: {}", e);
                                    self.status_message_timestamp = Some(std::time::Instant::now());
                                    tracing::error!(
                                        "Failed to load prefab {}: {:#}",
                                        asset_path,
                                        e
                                    );
                                }
                            }
                        } else {
                            // Use the file's stem as the entity name.
                            let name = std::path::Path::new(&asset_path)
                                .file_stem()
                                .map(|s| s.to_string_lossy().to_string())
                                .unwrap_or_else(|| "Asset".to_string());

                            self.scene_snapshot();
                            let new_id = self.scene_model.add_entity(
                                name,
                                vec![
                                    crate::app::scene_editor::model::ComponentData::MeshFromFile {
                                        path: asset_path.clone(),
                                        texture_path: None,
                                        normal_map_path: None,
                                    },
                                ],
                            );
                            if let Some(entity) = self.scene_model.entities.get_mut(&new_id) {
                                entity.transform.translation =
                                    [spawn_pos.x, spawn_pos.y, spawn_pos.z];
                            }
                            self.scene_model.select_only(new_id);
                            self.primary_selected_id = Some(new_id);
                            self.scene_needs_sync = true;
                            self.status_message = format!("Imported asset: {}", asset_path);
                            self.status_message_timestamp = Some(std::time::Instant::now());
                            tracing::info!("Dropped asset {} at {:?}", asset_path, spawn_pos);
                        }
                    }
                }
            }
        }

        // Cancel any pending drag if the pointer was released anywhere (so a
        // drag that ended outside the scene view doesn't leak the overlay).
        if pointer_released {
            self.dragged_asset_path = None;
        }

        // Scroll over the viewport to zoom (clamped to a sane range).
        // In fly mode, scroll adjusts speed instead (handled above).
        if response.hovered() && !self.fly_mode_active {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll.abs() > 0.0 {
                if self.scene_ortho {
                    // Adjust orthographic zoom scale.
                    self.scene_ortho_scale *= 1.0 - scroll * 0.005;
                    self.scene_ortho_scale = self.scene_ortho_scale.clamp(0.5, 100.0);
                } else {
                    self.scene_orbit_distance *= 1.0 - scroll * 0.005;
                    self.scene_orbit_distance = self.scene_orbit_distance.clamp(2.0, 50.0);
                }
            }
        }
    }

    /// Draw the per-axis transform gizmo for the selected entity and apply any
    /// axis-locked (or plane-locked) drag to its transform.
    fn handle_gizmo(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        rect: egui::Rect,
        cam_pos: Vec3,
        selected_id: u64,
    ) {
        let pos = if self.scene_model.entities.contains_key(&selected_id) {
            let world_t = self.scene_model.compute_world_transform(selected_id);
            Vec3::from_array(world_t.translation)
        } else {
            return;
        };

        let orbit_target = Vec3::from_array(self.scene_orbit_target);
        let ortho = self.scene_ortho;
        let ortho_scale = self.scene_ortho_scale;
        let center_screen =
            match project_to_screen(pos, cam_pos, orbit_target, rect, ortho, ortho_scale) {
                Some(p) => p,
                None => return, // Selected entity is behind the camera.
            };

        let axes: [(Vec3, egui::Color32, usize); 3] = [
            (Vec3::X, egui::Color32::from_rgb(230, 70, 70), 0_usize),
            (Vec3::Y, egui::Color32::from_rgb(80, 200, 80), 1),
            (Vec3::Z, egui::Color32::from_rgb(80, 130, 255), 2),
        ];

        let painter = ui.painter();
        let pointer = response.hover_pos();

        // Draw axes and pick the one currently hovered (if any).
        let mut hovered_axis: Option<usize> = None;
        let mut hovered_plane: Option<(usize, usize)> = None;

        for (axis_dir, color, idx) in axes.iter().copied() {
            // Determine hot/active state for this axis.
            let active = self.gizmo_dragging == Some(GizmoDrag::SingleAxis(idx));
            let mut hot = active;
            if !active {
                if let Some(p) = pointer {
                    let world_end = pos + axis_dir;
                    if let Some(end_screen) = project_to_screen(
                        world_end,
                        cam_pos,
                        orbit_target,
                        rect,
                        ortho,
                        ortho_scale,
                    ) {
                        if distance_point_to_segment(p, center_screen, end_screen) < 8.0 {
                            hot = true;
                            hovered_axis = Some(idx);
                        }
                    }
                }
            }

            let stroke_width = if hot { 4.0 } else { 2.5 };
            let draw_color = if hot { egui::Color32::WHITE } else { color };

            if self.gizmo_mode == GizmoMode::Rotate {
                // --- Rotate mode: draw a partial arc in the plane perpendicular to the axis ---
                let arc_segments = 24;
                let arc_radius_world = 0.8_f32;
                let perp1 = if axis_dir.dot(Vec3::Y).abs() < 0.99 {
                    axis_dir.cross(Vec3::Y).normalize()
                } else {
                    axis_dir.cross(Vec3::X).normalize()
                };
                let perp2 = axis_dir.cross(perp1).normalize();

                let mut prev_pt: Option<egui::Pos2> = None;
                for seg in 0..=arc_segments {
                    let angle = (seg as f32 / arc_segments as f32) * std::f32::consts::TAU * 0.75; // 270 degrees
                    let world_pt =
                        pos + (perp1 * angle.cos() + perp2 * angle.sin()) * arc_radius_world;
                    if let Some(sp) =
                        project_to_screen(world_pt, cam_pos, orbit_target, rect, ortho, ortho_scale)
                    {
                        if let Some(pp) = prev_pt {
                            painter.line_segment(
                                [pp, sp],
                                egui::Stroke::new(stroke_width, draw_color),
                            );
                        }
                        prev_pt = Some(sp);
                    } else {
                        prev_pt = None;
                    }
                }
            } else {
                // --- Move / Scale modes: draw axis line + tip shape ---
                let world_end = pos + axis_dir;
                let end_screen = match project_to_screen(
                    world_end,
                    cam_pos,
                    orbit_target,
                    rect,
                    ortho,
                    ortho_scale,
                ) {
                    Some(p) => p,
                    None => continue,
                };

                painter.line_segment(
                    [center_screen, end_screen],
                    egui::Stroke::new(stroke_width, draw_color),
                );

                if self.gizmo_mode == GizmoMode::Move {
                    // Arrow head: triangle pointing along axis direction.
                    let dir = (end_screen - center_screen).normalized();
                    let perp = egui::vec2(-dir.y, dir.x);
                    let tip = end_screen + dir * 8.0;
                    let base_l = end_screen + perp * 5.0;
                    let base_r = end_screen - perp * 5.0;
                    painter.add(egui::Shape::convex_polygon(
                        vec![tip, base_l, base_r],
                        draw_color,
                        egui::Stroke::NONE,
                    ));
                } else {
                    // Scale mode: small square at the tip.
                    let half = 4.0;
                    painter.rect_filled(
                        egui::Rect::from_center_size(
                            end_screen,
                            egui::vec2(half * 2.0, half * 2.0),
                        ),
                        0.0,
                        draw_color,
                    );
                }
            }
        }

        // --- Plane drag handles (Move mode only) ---
        if self.gizmo_mode == GizmoMode::Move {
            let plane_pairs: [(usize, usize, egui::Color32); 3] = [
                (0, 2, egui::Color32::from_rgb(200, 200, 80)), // XZ (yellow-ish)
                (0, 1, egui::Color32::from_rgb(80, 200, 200)), // XY (cyan-ish)
                (1, 2, egui::Color32::from_rgb(200, 80, 200)), // YZ (magenta-ish)
            ];

            for (a_idx, b_idx, color) in plane_pairs {
                let offset = (axes[a_idx].0 + axes[b_idx].0) * 0.3;
                let world_pt = pos + offset;
                if let Some(sp) =
                    project_to_screen(world_pt, cam_pos, orbit_target, rect, ortho, ortho_scale)
                {
                    let half = 6.0;
                    let sq_rect =
                        egui::Rect::from_center_size(sp, egui::vec2(half * 2.0, half * 2.0));

                    let plane_hot = pointer.map_or(false, |p| sq_rect.contains(p));
                    let plane_active = self.gizmo_dragging == Some(GizmoDrag::Plane(a_idx, b_idx));
                    let draw_col = if plane_hot || plane_active {
                        egui::Color32::WHITE
                    } else {
                        color
                    };

                    painter.rect_filled(sq_rect, 0.0, draw_col);

                    if plane_hot {
                        hovered_plane = Some((a_idx, b_idx));
                    }
                }
            }
        }

        // On primary press, lock to whichever handle was hovered (plane first,
        // then axis). Snapshot the scene so the entire drag can be undone in one step.
        if response.drag_started_by(egui::PointerButton::Primary) {
            if let Some((a, b)) = hovered_plane {
                self.gizmo_dragging = Some(GizmoDrag::Plane(a, b));
                self.scene_snapshot();
            } else if let Some(axis) = hovered_axis {
                self.gizmo_dragging = Some(GizmoDrag::SingleAxis(axis));
                self.scene_snapshot();
            }
        }

        // --- Apply drag ---
        let drag_delta = response.drag_delta();
        if drag_delta != egui::Vec2::ZERO {
            if let Some(drag) = self.gizmo_dragging {
                let drag_axes: &[usize] = match drag {
                    GizmoDrag::SingleAxis(idx) => &[idx][..],
                    GizmoDrag::Plane(a, b) => &[a, b][..],
                };

                // Plane drags only apply to Move mode.
                let is_plane = matches!(drag, GizmoDrag::Plane(_, _));
                if is_plane && self.gizmo_mode != GizmoMode::Move {
                    // No-op: plane drags are not meaningful for Rotate/Scale.
                } else {
                    let snap_enabled = self.snap_enabled;
                    let snap_value = self.snap_value.max(0.0001);
                    let gizmo_mode = self.gizmo_mode;

                    // Pre-compute screen-space axis info for each drag axis.
                    let mut axis_deltas: Vec<(usize, f32)> = Vec::new();
                    for &axis_idx in drag_axes {
                        let axis_dir = match axis_idx {
                            0 => Vec3::X,
                            1 => Vec3::Y,
                            2 => Vec3::Z,
                            _ => continue,
                        };
                        let world_end = pos + axis_dir;
                        let end_screen = match project_to_screen(
                            world_end,
                            cam_pos,
                            orbit_target,
                            rect,
                            ortho,
                            ortho_scale,
                        ) {
                            Some(p) => p,
                            None => continue,
                        };
                        let axis_screen = end_screen - center_screen;
                        let axis_screen_len = axis_screen.length();
                        if axis_screen_len < 0.001 {
                            continue;
                        }
                        let axis_screen_dir = axis_screen / axis_screen_len;
                        let drag_along =
                            drag_delta.x * axis_screen_dir.x + drag_delta.y * axis_screen_dir.y;
                        let world_delta = drag_along / axis_screen_len;
                        axis_deltas.push((axis_idx, world_delta));
                    }

                    let ids: Vec<u64> = self.scene_model.selected_ids.iter().copied().collect();
                    for eid in ids {
                        if let Some(entity) = self.scene_model.entities.get_mut(&eid) {
                            for &(axis_idx, world_delta) in &axis_deltas {
                                match gizmo_mode {
                                    GizmoMode::Move => {
                                        entity.transform.translation[axis_idx] += world_delta;
                                        if snap_enabled {
                                            let v = entity.transform.translation[axis_idx];
                                            entity.transform.translation[axis_idx] =
                                                (v / snap_value).round() * snap_value;
                                        }
                                    }
                                    GizmoMode::Rotate => {
                                        entity.transform.rotation_euler[axis_idx] +=
                                            world_delta * 0.5;
                                        if snap_enabled {
                                            let step =
                                                snap_value * std::f32::consts::FRAC_PI_8 / 1.5;
                                            let v = entity.transform.rotation_euler[axis_idx];
                                            entity.transform.rotation_euler[axis_idx] =
                                                (v / step).round() * step;
                                        }
                                    }
                                    GizmoMode::Scale => {
                                        entity.transform.scale[axis_idx] =
                                            (entity.transform.scale[axis_idx] + world_delta * 0.3)
                                                .max(0.01);
                                        if snap_enabled {
                                            let v = entity.transform.scale[axis_idx];
                                            entity.transform.scale[axis_idx] =
                                                ((v / snap_value).round() * snap_value).max(0.01);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if !axis_deltas.is_empty() {
                        self.scene_needs_sync = true;
                        self.scene_model.modified = true;
                    }
                }
            }
        }
    }

    // ----- Quad View camera state helpers -----

    /// Save the current main camera parameters into the given quadrant slot.
    pub(crate) fn save_camera_to_quad(&mut self, idx: usize) {
        if idx >= 4 {
            return;
        }
        let slot = &mut self.quad_camera_states[idx];
        slot.yaw = self.scene_orbit_yaw;
        slot.pitch = self.scene_orbit_pitch;
        slot.distance = self.scene_orbit_distance;
        slot.ortho = self.scene_ortho;
        slot.ortho_scale = self.scene_ortho_scale;
        slot.target = self.scene_orbit_target;
    }

    /// Load a quadrant's camera state into the main camera parameters.
    pub(crate) fn load_camera_from_quad(&mut self, idx: usize) {
        if idx >= 4 {
            return;
        }
        let slot = &self.quad_camera_states[idx];
        self.scene_orbit_yaw = slot.yaw;
        self.scene_orbit_pitch = slot.pitch;
        self.scene_orbit_distance = slot.distance;
        self.scene_ortho = slot.ortho;
        self.scene_ortho_scale = slot.ortho_scale;
        self.scene_orbit_target = slot.target;
    }
}

/// Draw a world-space reference grid on the Y=0 plane. The X and Z axes are
/// rendered in red/blue respectively; other lines are a subdued gray.
fn draw_world_grid(
    painter: &egui::Painter,
    cam_pos: Vec3,
    cam_target: Vec3,
    rect: egui::Rect,
    ortho: bool,
    ortho_scale: f32,
) {
    use crate::app::scene_editor::gizmo::project_to_screen;
    const GRID_EXTENT: i32 = 10;
    let grid_color = egui::Color32::from_rgba_premultiplied(150, 150, 150, 60);

    for i in -GRID_EXTENT..=GRID_EXTENT {
        let f = i as f32;

        // X-aligned lines (varying Z).
        let p0 = Vec3::new(-GRID_EXTENT as f32, 0.0, f);
        let p1 = Vec3::new(GRID_EXTENT as f32, 0.0, f);
        if let (Some(s0), Some(s1)) = (
            project_to_screen(p0, cam_pos, cam_target, rect, ortho, ortho_scale),
            project_to_screen(p1, cam_pos, cam_target, rect, ortho, ortho_scale),
        ) {
            let color = if i == 0 {
                egui::Color32::from_rgb(200, 80, 80) // X axis (red)
            } else {
                grid_color
            };
            painter.line_segment([s0, s1], egui::Stroke::new(1.0, color));
        }

        // Z-aligned lines (varying X).
        let p0 = Vec3::new(f, 0.0, -GRID_EXTENT as f32);
        let p1 = Vec3::new(f, 0.0, GRID_EXTENT as f32);
        if let (Some(s0), Some(s1)) = (
            project_to_screen(p0, cam_pos, cam_target, rect, ortho, ortho_scale),
            project_to_screen(p1, cam_pos, cam_target, rect, ortho, ortho_scale),
        ) {
            let color = if i == 0 {
                egui::Color32::from_rgb(80, 80, 200) // Z axis (blue)
            } else {
                grid_color
            };
            painter.line_segment([s0, s1], egui::Stroke::new(1.0, color));
        }
    }
}

/// 2D distance from a point to a finite line segment.
fn distance_point_to_segment(p: egui::Pos2, a: egui::Pos2, b: egui::Pos2) -> f32 {
    let ab = b - a;
    let ap = p - a;
    let len_sq = ab.x * ab.x + ab.y * ab.y;
    if len_sq < 0.001 {
        return (p - a).length();
    }
    let t = ((ap.x * ab.x + ap.y * ab.y) / len_sq).clamp(0.0, 1.0);
    let proj = a + ab * t;
    (p - proj).length()
}

/// Project the 8 corners of an AABB to screen space and draw its 12 edges.
fn draw_wireframe_aabb(
    painter: &egui::Painter,
    aabb_min: Vec3,
    aabb_max: Vec3,
    cam_pos: Vec3,
    cam_target: Vec3,
    rect: egui::Rect,
    ortho: bool,
    ortho_scale: f32,
    color: egui::Color32,
) {
    let corners = [
        Vec3::new(aabb_min.x, aabb_min.y, aabb_min.z),
        Vec3::new(aabb_max.x, aabb_min.y, aabb_min.z),
        Vec3::new(aabb_max.x, aabb_max.y, aabb_min.z),
        Vec3::new(aabb_min.x, aabb_max.y, aabb_min.z),
        Vec3::new(aabb_min.x, aabb_min.y, aabb_max.z),
        Vec3::new(aabb_max.x, aabb_min.y, aabb_max.z),
        Vec3::new(aabb_max.x, aabb_max.y, aabb_max.z),
        Vec3::new(aabb_min.x, aabb_max.y, aabb_max.z),
    ];

    // Indices into `corners` for the 12 edges of the box.
    const EDGES: [(usize, usize); 12] = [
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 0), // bottom face
        (4, 5),
        (5, 6),
        (6, 7),
        (7, 4), // top face
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7), // verticals
    ];

    let projected: Vec<Option<egui::Pos2>> = corners
        .iter()
        .map(|c| project_to_screen(*c, cam_pos, cam_target, rect, ortho, ortho_scale))
        .collect();

    let stroke = egui::Stroke::new(1.5, color);
    for (a, b) in EDGES {
        if let (Some(pa), Some(pb)) = (projected[a], projected[b]) {
            painter.line_segment([pa, pb], stroke);
        }
    }
}

/// Linear interpolation of two scalars.
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Component-wise linear interpolation of two RGBA colors.
fn lerp_color(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    [
        lerp(a[0], b[0], t),
        lerp(a[1], b[1], t),
        lerp(a[2], b[2], t),
        lerp(a[3], b[3], t),
    ]
}
