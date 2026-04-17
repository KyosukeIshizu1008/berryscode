//! Inspector panel: edit transform and components of the selected entity.

use super::model::*;
use crate::app::BerryCodeApp;

impl BerryCodeApp {
    /// Render the Inspector panel for the currently selected entity.
    pub(crate) fn render_scene_inspector(&mut self, ui: &mut egui::Ui) {
        ui.heading("Inspector");
        ui.separator();

        let selected_id = match self.primary_selected_id {
            Some(id) if self.scene_model.is_selected(id) => id,
            _ => {
                if self.scene_model.selected_ids.is_empty() {
                    ui.label("No entity selected");
                } else {
                    ui.label(format!("{} entities selected (no primary)", self.scene_model.selected_ids.len()));
                }
                return;
            }
        };

        // Show multi-selection info when more than one entity is selected.
        let selection_count = self.scene_model.selected_ids.len();
        if selection_count > 1 {
            ui.label(
                egui::RichText::new(format!("{} entities selected", selection_count))
                    .color(egui::Color32::from_rgb(120, 180, 255)),
            );
            ui.separator();
        }

        // Track whether the user mutated anything this frame so we can flag dirty.
        let mut mutated = false;
        let mut delete_requested = false;
        let mut add_component: Option<ComponentData> = None;
        let mut revert_prefab_requested = false;
        let mut apply_prefab_requested = false;
        let mut unpack_prefab_requested = false;
        let mut audio_play_requested: Option<String> = None;
        let mut audio_stop_requested = false;
        let mut add_component_copy: Option<ComponentData> = None;
        let mut paste_requested = false;
        let mut open_animator_editor = false;
        let mut animator_path_for_editor = String::new();
        let mut bake_navmesh_requested: Option<f32> = None;

        // Pre-compute the world transform (and whether a parent exists) before
        // entering the mutable borrow on `entities`, so we can display a
        // read-only world position for child entities.
        let has_parent = self
            .scene_model
            .entities
            .get(&selected_id)
            .and_then(|e| e.parent)
            .is_some();
        let world_transform_readout = if has_parent {
            Some(self.scene_model.compute_world_transform(selected_id))
        } else {
            None
        };

        // Capture GPU material preview texture id before entering the mutable
        // entity borrow (Phase 8).
        let mat_preview_tex = self.material_preview_texture_id;

        {
            let entity = match self.scene_model.entities.get_mut(&selected_id) {
                Some(e) => e,
                None => {
                    ui.label("Entity not found");
                    return;
                }
            };

            // Prefab source info + Revert / Apply buttons
            if let Some(ref prefab_path) = entity.prefab_source {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("Prefab: {}",
                            std::path::Path::new(prefab_path)
                                .file_name()
                                .map(|s| s.to_string_lossy().to_string())
                                .unwrap_or_else(|| prefab_path.clone())
                        )).color(egui::Color32::from_rgb(120, 200, 255))
                    );
                });
                ui.horizontal(|ui| {
                    if ui.button("Revert to Prefab").clicked() {
                        revert_prefab_requested = true;
                    }
                    if ui.button("Apply to Prefab").clicked() {
                        apply_prefab_requested = true;
                    }
                    if ui.button("Unpack Prefab").clicked() {
                        unpack_prefab_requested = true;
                    }
                });
                ui.separator();
            }

            // ID + Name
            ui.horizontal(|ui| {
                ui.label("ID:");
                ui.monospace(entity.id.to_string());
            });
            ui.horizontal(|ui| {
                ui.label("Name:");
                if ui.text_edit_singleline(&mut entity.name).changed() {
                    mutated = true;
                }
            });

            ui.separator();

            // Transform (local space — editable)
            egui::CollapsingHeader::new(if has_parent {
                "Local Transform"
            } else {
                "Transform"
            })
                .default_open(true)
                .show(ui, |ui| {
                    egui::Grid::new("scene_transform_grid")
                        .num_columns(4)
                        .spacing([6.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("Position");
                            for axis in 0..3 {
                                if ui
                                    .add(
                                        egui::DragValue::new(&mut entity.transform.translation[axis])
                                            .speed(0.1)
                                            .prefix(["x: ", "y: ", "z: "][axis]),
                                    )
                                    .changed()
                                {
                                    mutated = true;
                                }
                            }
                            ui.end_row();

                            ui.label("Rotation");
                            for axis in 0..3 {
                                if ui
                                    .add(
                                        egui::DragValue::new(
                                            &mut entity.transform.rotation_euler[axis],
                                        )
                                        .speed(0.01)
                                        .prefix(["x: ", "y: ", "z: "][axis]),
                                    )
                                    .changed()
                                {
                                    mutated = true;
                                }
                            }
                            ui.end_row();

                            ui.label("Scale");
                            for axis in 0..3 {
                                if ui
                                    .add(
                                        egui::DragValue::new(&mut entity.transform.scale[axis])
                                            .speed(0.01)
                                            .prefix(["x: ", "y: ", "z: "][axis]),
                                    )
                                    .changed()
                                {
                                    mutated = true;
                                }
                            }
                            ui.end_row();
                        });
                });

            // World transform (read-only) for child entities.
            if let Some(ref world) = world_transform_readout {
                ui.separator();
                ui.label(
                    egui::RichText::new("World Transform (read-only)")
                        .small()
                        .color(egui::Color32::from_gray(140)),
                );
                ui.horizontal(|ui| {
                    ui.label("World Pos:");
                    ui.monospace(format!(
                        "[{:.2}, {:.2}, {:.2}]",
                        world.translation[0], world.translation[1], world.translation[2]
                    ));
                });
                ui.horizontal(|ui| {
                    ui.label("World Rot:");
                    ui.monospace(format!(
                        "[{:.3}, {:.3}, {:.3}]",
                        world.rotation_euler[0], world.rotation_euler[1], world.rotation_euler[2]
                    ));
                });
                ui.horizontal(|ui| {
                    ui.label("World Scale:");
                    ui.monospace(format!(
                        "[{:.2}, {:.2}, {:.2}]",
                        world.scale[0], world.scale[1], world.scale[2]
                    ));
                });
            }

            ui.separator();
            ui.label("Components");

            // Components
            let mut component_to_remove: Option<usize> = None;
            let mut move_up_idx: Option<usize> = None;
            let mut move_down_idx: Option<usize> = None;
            let component_count = entity.components.len();
            for (idx, component) in entity.components.iter_mut().enumerate() {
                ui.group(|ui| {
                    // Header row: component label + reorder/remove buttons.
                    ui.horizontal(|ui| {
                        ui.strong(component.label());
                        ui.add_enabled_ui(idx > 0, |ui| {
                            if ui.small_button("^").clicked() {
                                move_up_idx = Some(idx);
                            }
                        });
                        ui.add_enabled_ui(idx + 1 < component_count, |ui| {
                            if ui.small_button("v").clicked() {
                                move_down_idx = Some(idx);
                            }
                        });
                        if ui.small_button("C").on_hover_text("Copy component").clicked() {
                            add_component_copy = Some(component.clone());
                        }
                        if ui.small_button("x").clicked() {
                            component_to_remove = Some(idx);
                        }
                    });

                    match component {
                        ComponentData::MeshCube {
                            size,
                            color,
                            metallic,
                            roughness,
                            emissive,
                            texture_path,
                            normal_map_path,
                        } => {
                            ui.horizontal(|ui| {
                                ui.label("Size:");
                                if ui.add(egui::DragValue::new(size).speed(0.1)).changed() {
                                    mutated = true;
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("Color:");
                                if ui.color_edit_button_rgb(color).changed() {
                                    mutated = true;
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("Metallic:");
                                if ui
                                    .add(egui::Slider::new(metallic, 0.0..=1.0))
                                    .changed()
                                {
                                    mutated = true;
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("Roughness:");
                                if ui
                                    .add(egui::Slider::new(roughness, 0.0..=1.0))
                                    .changed()
                                {
                                    mutated = true;
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("Emissive:");
                                if ui.color_edit_button_rgb(emissive).changed() {
                                    mutated = true;
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("Texture:");
                                let mut path_str = texture_path.clone().unwrap_or_default();
                                if ui.add(egui::TextEdit::singleline(&mut path_str).hint_text("image path").desired_width(180.0))
                                    .changed() {
                                    *texture_path = if path_str.is_empty() { None } else { Some(path_str) };
                                    mutated = true;
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("Normal Map:");
                                let mut path_str = normal_map_path.clone().unwrap_or_default();
                                if ui.add(egui::TextEdit::singleline(&mut path_str).hint_text("normal map path").desired_width(180.0))
                                    .changed() {
                                    *normal_map_path = if path_str.is_empty() { None } else { Some(path_str) };
                                    mutated = true;
                                }
                            });
                            ui.add_space(4.0);
                            crate::app::scene_editor::material_preview::draw_material_preview_gpu_or_fallback(
                                ui,
                                mat_preview_tex,
                                *color,
                                *metallic,
                                *roughness,
                                *emissive,
                            );
                        }
                        ComponentData::MeshSphere {
                            radius,
                            color,
                            metallic,
                            roughness,
                            emissive,
                            texture_path,
                            normal_map_path,
                        } => {
                            ui.horizontal(|ui| {
                                ui.label("Radius:");
                                if ui.add(egui::DragValue::new(radius).speed(0.1)).changed() {
                                    mutated = true;
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("Color:");
                                if ui.color_edit_button_rgb(color).changed() {
                                    mutated = true;
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("Metallic:");
                                if ui
                                    .add(egui::Slider::new(metallic, 0.0..=1.0))
                                    .changed()
                                {
                                    mutated = true;
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("Roughness:");
                                if ui
                                    .add(egui::Slider::new(roughness, 0.0..=1.0))
                                    .changed()
                                {
                                    mutated = true;
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("Emissive:");
                                if ui.color_edit_button_rgb(emissive).changed() {
                                    mutated = true;
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("Texture:");
                                let mut path_str = texture_path.clone().unwrap_or_default();
                                if ui.add(egui::TextEdit::singleline(&mut path_str).hint_text("image path").desired_width(180.0))
                                    .changed() {
                                    *texture_path = if path_str.is_empty() { None } else { Some(path_str) };
                                    mutated = true;
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("Normal Map:");
                                let mut path_str = normal_map_path.clone().unwrap_or_default();
                                if ui.add(egui::TextEdit::singleline(&mut path_str).hint_text("normal map path").desired_width(180.0))
                                    .changed() {
                                    *normal_map_path = if path_str.is_empty() { None } else { Some(path_str) };
                                    mutated = true;
                                }
                            });
                            ui.add_space(4.0);
                            crate::app::scene_editor::material_preview::draw_material_preview_gpu_or_fallback(
                                ui,
                                mat_preview_tex,
                                *color,
                                *metallic,
                                *roughness,
                                *emissive,
                            );
                        }
                        ComponentData::MeshPlane {
                            size,
                            color,
                            metallic,
                            roughness,
                            emissive,
                            texture_path,
                            normal_map_path,
                        } => {
                            ui.horizontal(|ui| {
                                ui.label("Size:");
                                if ui.add(egui::DragValue::new(size).speed(0.1)).changed() {
                                    mutated = true;
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("Color:");
                                if ui.color_edit_button_rgb(color).changed() {
                                    mutated = true;
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("Metallic:");
                                if ui
                                    .add(egui::Slider::new(metallic, 0.0..=1.0))
                                    .changed()
                                {
                                    mutated = true;
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("Roughness:");
                                if ui
                                    .add(egui::Slider::new(roughness, 0.0..=1.0))
                                    .changed()
                                {
                                    mutated = true;
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("Emissive:");
                                if ui.color_edit_button_rgb(emissive).changed() {
                                    mutated = true;
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("Texture:");
                                let mut path_str = texture_path.clone().unwrap_or_default();
                                if ui.add(egui::TextEdit::singleline(&mut path_str).hint_text("image path").desired_width(180.0))
                                    .changed() {
                                    *texture_path = if path_str.is_empty() { None } else { Some(path_str) };
                                    mutated = true;
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("Normal Map:");
                                let mut path_str = normal_map_path.clone().unwrap_or_default();
                                if ui.add(egui::TextEdit::singleline(&mut path_str).hint_text("normal map path").desired_width(180.0))
                                    .changed() {
                                    *normal_map_path = if path_str.is_empty() { None } else { Some(path_str) };
                                    mutated = true;
                                }
                            });
                            ui.add_space(4.0);
                            crate::app::scene_editor::material_preview::draw_material_preview_gpu_or_fallback(
                                ui,
                                mat_preview_tex,
                                *color,
                                *metallic,
                                *roughness,
                                *emissive,
                            );
                        }
                        ComponentData::Light { intensity, color } => {
                            ui.horizontal(|ui| {
                                ui.label("Intensity:");
                                if ui
                                    .add(egui::DragValue::new(intensity).speed(100.0))
                                    .changed()
                                {
                                    mutated = true;
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("Color:");
                                if ui.color_edit_button_rgb(color).changed() {
                                    mutated = true;
                                }
                            });
                        }
                        ComponentData::SpotLight {
                            intensity,
                            color,
                            range,
                            inner_angle,
                            outer_angle,
                        } => {
                            ui.horizontal(|ui| {
                                ui.label("Intensity:");
                                if ui
                                    .add(egui::DragValue::new(intensity).speed(100.0))
                                    .changed()
                                {
                                    mutated = true;
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("Color:");
                                if ui.color_edit_button_rgb(color).changed() {
                                    mutated = true;
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("Range:");
                                if ui
                                    .add(egui::DragValue::new(range).speed(0.5).range(0.1..=200.0))
                                    .changed()
                                {
                                    mutated = true;
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("Inner Angle:");
                                if ui
                                    .add(egui::Slider::new(inner_angle, 0.0..=1.5))
                                    .changed()
                                {
                                    mutated = true;
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("Outer Angle:");
                                if ui
                                    .add(egui::Slider::new(outer_angle, 0.0..=1.5))
                                    .changed()
                                {
                                    mutated = true;
                                }
                            });
                        }
                        ComponentData::DirectionalLight {
                            intensity,
                            color,
                            shadows,
                        } => {
                            ui.horizontal(|ui| {
                                ui.label("Intensity:");
                                if ui
                                    .add(egui::DragValue::new(intensity).speed(100.0))
                                    .changed()
                                {
                                    mutated = true;
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("Color:");
                                if ui.color_edit_button_rgb(color).changed() {
                                    mutated = true;
                                }
                            });
                            ui.horizontal(|ui| {
                                if ui.checkbox(shadows, "Shadows").changed() {
                                    mutated = true;
                                }
                            });
                        }
                        ComponentData::Camera => {
                            ui.label("(no editable properties)");
                        }
                        ComponentData::MeshFromFile { path, texture_path, normal_map_path } => {
                            ui.horizontal(|ui| {
                                ui.label("Path:");
                                ui.label(path.as_str());
                            });
                            ui.horizontal(|ui| {
                                ui.label("Texture:");
                                let mut path_str = texture_path.clone().unwrap_or_default();
                                if ui.add(egui::TextEdit::singleline(&mut path_str).hint_text("image path").desired_width(180.0))
                                    .changed() {
                                    *texture_path = if path_str.is_empty() { None } else { Some(path_str) };
                                    mutated = true;
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("Normal Map:");
                                let mut path_str = normal_map_path.clone().unwrap_or_default();
                                if ui.add(egui::TextEdit::singleline(&mut path_str).hint_text("normal map path").desired_width(180.0))
                                    .changed() {
                                    *normal_map_path = if path_str.is_empty() { None } else { Some(path_str) };
                                    mutated = true;
                                }
                            });
                        }
                        ComponentData::AudioSource {
                            path,
                            volume,
                            looped,
                            autoplay,
                        } => {
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label("Path:");
                                    if ui.text_edit_singleline(path).changed() {
                                        mutated = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Volume:");
                                    if ui
                                        .add(egui::Slider::new(volume, 0.0..=2.0))
                                        .changed()
                                    {
                                        mutated = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    if ui.checkbox(looped, "Loop").changed() {
                                        mutated = true;
                                    }
                                    if ui.checkbox(autoplay, "Auto Play").changed() {
                                        mutated = true;
                                    }
                                });
                                // Audio preview Play / Stop controls
                                ui.horizontal(|ui| {
                                    let is_playing_this = self.audio_preview_playing
                                        && self.audio_preview_path == *path;
                                    if !is_playing_this {
                                        if ui.button("\u{25B6} Play").clicked() && !path.is_empty() {
                                            audio_play_requested = Some(path.clone());
                                        }
                                    } else {
                                        if ui.button("\u{25A0} Stop").clicked() {
                                            audio_stop_requested = true;
                                        }
                                        ui.colored_label(
                                            egui::Color32::from_rgb(80, 200, 80),
                                            "Playing...",
                                        );
                                    }
                                });
                            });
                        }
                        ComponentData::AudioListener => {
                            ui.label("(receives audio at this entity's transform)");
                        }
                        ComponentData::RigidBody { body_type, mass } => {
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label("Body Type:");
                                    egui::ComboBox::from_id_salt(format!("rb_type_{}", idx))
                                        .selected_text(body_type.label())
                                        .show_ui(ui, |ui| {
                                            for option in RigidBodyType::ALL.iter().copied() {
                                                if ui
                                                    .selectable_label(
                                                        *body_type == option,
                                                        option.label(),
                                                    )
                                                    .clicked()
                                                    && *body_type != option
                                                {
                                                    *body_type = option;
                                                    mutated = true;
                                                }
                                            }
                                        });
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Mass:");
                                    if ui
                                        .add(
                                            egui::DragValue::new(mass)
                                                .speed(0.1)
                                                .range(0.001..=10000.0),
                                        )
                                        .changed()
                                    {
                                        mutated = true;
                                    }
                                });
                            });
                        }
                        ComponentData::Collider {
                            shape,
                            friction,
                            restitution,
                        } => {
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label("Shape:");
                                    egui::ComboBox::from_id_salt(format!("col_shape_{}", idx))
                                        .selected_text(shape.label())
                                        .show_ui(ui, |ui| {
                                            if ui
                                                .selectable_label(
                                                    matches!(shape, ColliderShape::Box { .. }),
                                                    "Box",
                                                )
                                                .clicked()
                                                && !matches!(shape, ColliderShape::Box { .. })
                                            {
                                                *shape = ColliderShape::Box {
                                                    half_extents: [0.5, 0.5, 0.5],
                                                };
                                                mutated = true;
                                            }
                                            if ui
                                                .selectable_label(
                                                    matches!(shape, ColliderShape::Sphere { .. }),
                                                    "Sphere",
                                                )
                                                .clicked()
                                                && !matches!(shape, ColliderShape::Sphere { .. })
                                            {
                                                *shape = ColliderShape::Sphere { radius: 0.5 };
                                                mutated = true;
                                            }
                                            if ui
                                                .selectable_label(
                                                    matches!(shape, ColliderShape::Capsule { .. }),
                                                    "Capsule",
                                                )
                                                .clicked()
                                                && !matches!(shape, ColliderShape::Capsule { .. })
                                            {
                                                *shape = ColliderShape::Capsule {
                                                    half_height: 0.5,
                                                    radius: 0.25,
                                                };
                                                mutated = true;
                                            }
                                        });
                                });
                                match shape {
                                    ColliderShape::Box { half_extents } => {
                                        ui.horizontal(|ui| {
                                            ui.label("Half Extents:");
                                            for axis in 0..3 {
                                                if ui
                                                    .add(
                                                        egui::DragValue::new(
                                                            &mut half_extents[axis],
                                                        )
                                                        .speed(0.05)
                                                        .range(0.01..=100.0),
                                                    )
                                                    .changed()
                                                {
                                                    mutated = true;
                                                }
                                            }
                                        });
                                    }
                                    ColliderShape::Sphere { radius } => {
                                        ui.horizontal(|ui| {
                                            ui.label("Radius:");
                                            if ui
                                                .add(
                                                    egui::DragValue::new(radius)
                                                        .speed(0.05)
                                                        .range(0.01..=100.0),
                                                )
                                                .changed()
                                            {
                                                mutated = true;
                                            }
                                        });
                                    }
                                    ColliderShape::Capsule {
                                        half_height,
                                        radius,
                                    } => {
                                        ui.horizontal(|ui| {
                                            ui.label("Half Height:");
                                            if ui
                                                .add(
                                                    egui::DragValue::new(half_height)
                                                        .speed(0.05)
                                                        .range(0.01..=100.0),
                                                )
                                                .changed()
                                            {
                                                mutated = true;
                                            }
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("Radius:");
                                            if ui
                                                .add(
                                                    egui::DragValue::new(radius)
                                                        .speed(0.05)
                                                        .range(0.01..=100.0),
                                                )
                                                .changed()
                                            {
                                                mutated = true;
                                            }
                                        });
                                    }
                                }
                                ui.horizontal(|ui| {
                                    ui.label("Friction:");
                                    if ui
                                        .add(egui::Slider::new(friction, 0.0..=2.0))
                                        .changed()
                                    {
                                        mutated = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Restitution:");
                                    if ui
                                        .add(egui::Slider::new(restitution, 0.0..=1.0))
                                        .changed()
                                    {
                                        mutated = true;
                                    }
                                });
                            });
                        }
                        ComponentData::UiText {
                            text,
                            font_size,
                            color,
                        } => {
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label("Text:");
                                    if ui.text_edit_singleline(text).changed() {
                                        mutated = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Font Size:");
                                    if ui
                                        .add(
                                            egui::DragValue::new(font_size)
                                                .speed(0.5)
                                                .range(4.0..=200.0),
                                        )
                                        .changed()
                                    {
                                        mutated = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Color:");
                                    let mut rgb = [color[0], color[1], color[2]];
                                    if ui.color_edit_button_rgb(&mut rgb).changed() {
                                        color[0] = rgb[0];
                                        color[1] = rgb[1];
                                        color[2] = rgb[2];
                                        mutated = true;
                                    }
                                    if ui
                                        .add(
                                            egui::Slider::new(&mut color[3], 0.0..=1.0)
                                                .text("alpha"),
                                        )
                                        .changed()
                                    {
                                        mutated = true;
                                    }
                                });
                            });
                        }
                        ComponentData::UiButton { label, background } => {
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label("Label:");
                                    if ui.text_edit_singleline(label).changed() {
                                        mutated = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Background:");
                                    let mut rgb = [background[0], background[1], background[2]];
                                    if ui.color_edit_button_rgb(&mut rgb).changed() {
                                        background[0] = rgb[0];
                                        background[1] = rgb[1];
                                        background[2] = rgb[2];
                                        mutated = true;
                                    }
                                    if ui
                                        .add(
                                            egui::Slider::new(&mut background[3], 0.0..=1.0)
                                                .text("alpha"),
                                        )
                                        .changed()
                                    {
                                        mutated = true;
                                    }
                                });
                            });
                        }
                        ComponentData::UiImage { path, tint } => {
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label("Path:");
                                    if ui.text_edit_singleline(path).changed() {
                                        mutated = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Tint:");
                                    let mut rgb = [tint[0], tint[1], tint[2]];
                                    if ui.color_edit_button_rgb(&mut rgb).changed() {
                                        tint[0] = rgb[0];
                                        tint[1] = rgb[1];
                                        tint[2] = rgb[2];
                                        mutated = true;
                                    }
                                    if ui
                                        .add(
                                            egui::Slider::new(&mut tint[3], 0.0..=1.0)
                                                .text("alpha"),
                                        )
                                        .changed()
                                    {
                                        mutated = true;
                                    }
                                });
                            });
                        }
                        ComponentData::ParticleEmitter {
                            rate,
                            lifetime,
                            speed,
                            spread,
                            start_size,
                            end_size,
                            start_color,
                            end_color,
                            max_particles,
                            gravity,
                        } => {
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label("Rate (per sec):");
                                    if ui
                                        .add(
                                            egui::DragValue::new(rate)
                                                .speed(0.5)
                                                .range(0.0..=1000.0),
                                        )
                                        .changed()
                                    {
                                        mutated = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Max Particles:");
                                    if ui
                                        .add(
                                            egui::DragValue::new(max_particles)
                                                .speed(1.0)
                                                .range(0u32..=10_000u32),
                                        )
                                        .changed()
                                    {
                                        mutated = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Lifetime (s):");
                                    if ui
                                        .add(
                                            egui::DragValue::new(lifetime)
                                                .speed(0.05)
                                                .range(0.05..=20.0),
                                        )
                                        .changed()
                                    {
                                        mutated = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Speed:");
                                    if ui
                                        .add(egui::DragValue::new(speed).speed(0.05))
                                        .changed()
                                    {
                                        mutated = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Spread:");
                                    if ui
                                        .add(egui::Slider::new(spread, 0.0..=1.0))
                                        .changed()
                                    {
                                        mutated = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Gravity:");
                                    if ui
                                        .add(egui::DragValue::new(gravity).speed(0.05))
                                        .changed()
                                    {
                                        mutated = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Start Size:");
                                    if ui
                                        .add(
                                            egui::DragValue::new(start_size)
                                                .speed(0.005)
                                                .range(0.001..=10.0),
                                        )
                                        .changed()
                                    {
                                        mutated = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("End Size:");
                                    if ui
                                        .add(
                                            egui::DragValue::new(end_size)
                                                .speed(0.005)
                                                .range(0.001..=10.0),
                                        )
                                        .changed()
                                    {
                                        mutated = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Start Color:");
                                    let mut rgb =
                                        [start_color[0], start_color[1], start_color[2]];
                                    if ui.color_edit_button_rgb(&mut rgb).changed() {
                                        start_color[0] = rgb[0];
                                        start_color[1] = rgb[1];
                                        start_color[2] = rgb[2];
                                        mutated = true;
                                    }
                                    if ui
                                        .add(
                                            egui::Slider::new(&mut start_color[3], 0.0..=1.0)
                                                .text("a"),
                                        )
                                        .changed()
                                    {
                                        mutated = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("End Color:");
                                    let mut rgb = [end_color[0], end_color[1], end_color[2]];
                                    if ui.color_edit_button_rgb(&mut rgb).changed() {
                                        end_color[0] = rgb[0];
                                        end_color[1] = rgb[1];
                                        end_color[2] = rgb[2];
                                        mutated = true;
                                    }
                                    if ui
                                        .add(
                                            egui::Slider::new(&mut end_color[3], 0.0..=1.0)
                                                .text("a"),
                                        )
                                        .changed()
                                    {
                                        mutated = true;
                                    }
                                });
                            });
                        }
                        ComponentData::CustomScript { type_name, fields } => {
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label("Type:");
                                    if ui
                                        .add(
                                            egui::TextEdit::singleline(type_name)
                                                .hint_text("e.g. MyGameState")
                                                .desired_width(200.0),
                                        )
                                        .changed()
                                    {
                                        mutated = true;
                                    }
                                });
                                ui.separator();
                                let mut remove_idx: Option<usize> = None;
                                for (f_idx, field) in fields.iter_mut().enumerate() {
                                    ui.horizontal(|ui| {
                                        if ui
                                            .add(
                                                egui::TextEdit::singleline(&mut field.name)
                                                    .hint_text("name")
                                                    .desired_width(100.0),
                                            )
                                            .changed()
                                        {
                                            mutated = true;
                                        }
                                        ui.label(field.value.type_label());
                                        match &mut field.value {
                                            ScriptValue::Float(v) => {
                                                if ui
                                                    .add(egui::DragValue::new(v).speed(0.05))
                                                    .changed()
                                                {
                                                    mutated = true;
                                                }
                                            }
                                            ScriptValue::Int(v) => {
                                                if ui
                                                    .add(egui::DragValue::new(v).speed(1.0))
                                                    .changed()
                                                {
                                                    mutated = true;
                                                }
                                            }
                                            ScriptValue::Bool(v) => {
                                                if ui.checkbox(v, "").changed() {
                                                    mutated = true;
                                                }
                                            }
                                            ScriptValue::String(v) => {
                                                if ui
                                                    .add(
                                                        egui::TextEdit::singleline(v)
                                                            .desired_width(160.0),
                                                    )
                                                    .changed()
                                                {
                                                    mutated = true;
                                                }
                                            }
                                        }
                                        if ui.small_button("x").clicked() {
                                            remove_idx = Some(f_idx);
                                        }
                                    });
                                }
                                if let Some(i) = remove_idx {
                                    fields.remove(i);
                                    mutated = true;
                                }
                                ui.separator();
                                ui.horizontal(|ui| {
                                    ui.label("Add field:");
                                    if ui.small_button("+ f32").clicked() {
                                        fields.push(ScriptField {
                                            name: format!("field_{}", fields.len()),
                                            value: ScriptValue::Float(0.0),
                                        });
                                        mutated = true;
                                    }
                                    if ui.small_button("+ i64").clicked() {
                                        fields.push(ScriptField {
                                            name: format!("field_{}", fields.len()),
                                            value: ScriptValue::Int(0),
                                        });
                                        mutated = true;
                                    }
                                    if ui.small_button("+ bool").clicked() {
                                        fields.push(ScriptField {
                                            name: format!("field_{}", fields.len()),
                                            value: ScriptValue::Bool(false),
                                        });
                                        mutated = true;
                                    }
                                    if ui.small_button("+ String").clicked() {
                                        fields.push(ScriptField {
                                            name: format!("field_{}", fields.len()),
                                            value: ScriptValue::String(String::new()),
                                        });
                                        mutated = true;
                                    }
                                });
                            });
                        }
                        ComponentData::Skybox { path } => {
                            ui.horizontal(|ui| {
                                ui.label("HDR Path:");
                                if ui.add(egui::TextEdit::singleline(path).hint_text("path to .hdr/.exr").desired_width(200.0))
                                    .changed() { mutated = true; }
                            });
                        }
                        ComponentData::Animation {
                            duration,
                            tracks,
                            looped,
                        } => {
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label("Duration (s):");
                                    if ui
                                        .add(
                                            egui::DragValue::new(duration)
                                                .speed(0.05)
                                                .range(0.05..=60.0),
                                        )
                                        .changed()
                                    {
                                        mutated = true;
                                    }
                                });
                                if ui.checkbox(looped, "Loop").changed() {
                                    mutated = true;
                                }
                                ui.separator();

                                // Per-track editor
                                ui.label(format!("Tracks: {}", tracks.len()));
                                let mut track_remove: Option<usize> = None;
                                for (t_idx, track) in tracks.iter_mut().enumerate() {
                                    ui.group(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.strong(track.property.label());
                                            ui.label(format!(
                                                "({} keyframes)",
                                                track.keyframes.len()
                                            ));
                                            if ui.small_button("x").clicked() {
                                                track_remove = Some(t_idx);
                                            }
                                        });
                                        let mut kf_remove: Option<usize> = None;
                                        for (k_idx, kf) in
                                            track.keyframes.iter_mut().enumerate()
                                        {
                                            ui.horizontal(|ui| {
                                                if ui
                                                    .add(
                                                        egui::DragValue::new(&mut kf.time)
                                                            .speed(0.02)
                                                            .range(0.0..=60.0)
                                                            .prefix("t: "),
                                                    )
                                                    .changed()
                                                {
                                                    mutated = true;
                                                }
                                                for i in 0..3 {
                                                    if ui
                                                        .add(
                                                            egui::DragValue::new(
                                                                &mut kf.value[i],
                                                            )
                                                            .speed(0.05)
                                                            .max_decimals(2),
                                                        )
                                                        .changed()
                                                    {
                                                        mutated = true;
                                                    }
                                                }
                                                // Easing dropdown
                                                egui::ComboBox::from_id_salt(format!(
                                                    "ease_{}_{}",
                                                    t_idx, k_idx
                                                ))
                                                .selected_text(kf.easing.label())
                                                .width(100.0)
                                                .show_ui(ui, |ui| {
                                                    for &e in EasingType::ALL {
                                                        if ui
                                                            .selectable_label(
                                                                kf.easing == e,
                                                                e.label(),
                                                            )
                                                            .clicked()
                                                            && kf.easing != e
                                                        {
                                                            kf.easing = e;
                                                            mutated = true;
                                                        }
                                                    }
                                                });
                                                if ui.small_button("x").clicked() {
                                                    kf_remove = Some(k_idx);
                                                }
                                            });
                                        }
                                        if let Some(ri) = kf_remove {
                                            track.keyframes.remove(ri);
                                            mutated = true;
                                        }
                                    });
                                }
                                if let Some(ri) = track_remove {
                                    tracks.remove(ri);
                                    mutated = true;
                                }

                                // Add track buttons
                                ui.separator();
                                ui.horizontal(|ui| {
                                    ui.label("Add Track:");
                                    for &prop in AnimProperty::ALL {
                                        if ui.small_button(prop.label()).clicked() {
                                            tracks.push(AnimationTrack {
                                                property: prop,
                                                keyframes: vec![],
                                                events: vec![],
                                            });
                                            mutated = true;
                                        }
                                    }
                                });
                            });
                        }
                        ComponentData::Animator { controller_path } => {
                            ui.horizontal(|ui| {
                                ui.label("Controller:");
                                if ui.add(egui::TextEdit::singleline(controller_path).hint_text(".banimator path").desired_width(200.0))
                                    .changed() { mutated = true; }
                            });
                            if ui.button("Open Editor").clicked() {
                                open_animator_editor = true;
                                animator_path_for_editor = controller_path.clone();
                            }
                        }
                        ComponentData::LodGroup { levels } => {
                            ui.vertical(|ui| {
                                ui.label(format!("Levels: {}", levels.len()));
                                // Sort levels by screen_percentage descending for display.
                                levels.sort_by(|a, b| b.screen_percentage.partial_cmp(&a.screen_percentage).unwrap_or(std::cmp::Ordering::Equal));
                                let mut level_remove: Option<usize> = None;
                                for (l_idx, level) in levels.iter_mut().enumerate() {
                                    ui.group(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.strong(format!("LOD {}", l_idx));
                                            if ui.small_button("x").clicked() {
                                                level_remove = Some(l_idx);
                                            }
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("Mesh:");
                                            if ui.add(
                                                egui::TextEdit::singleline(&mut level.mesh_path)
                                                    .hint_text("mesh asset path")
                                                    .desired_width(200.0),
                                            ).changed() {
                                                mutated = true;
                                            }
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("Screen %:");
                                            if ui.add(
                                                egui::Slider::new(&mut level.screen_percentage, 0.0..=1.0),
                                            ).changed() {
                                                mutated = true;
                                            }
                                        });
                                    });
                                }
                                if let Some(ri) = level_remove {
                                    levels.remove(ri);
                                    mutated = true;
                                }
                                ui.separator();
                                if ui.button("+ Add LOD Level").clicked() {
                                    levels.push(LodLevel {
                                        mesh_path: String::new(),
                                        screen_percentage: 0.0,
                                    });
                                    mutated = true;
                                }
                            });
                        }
                        ComponentData::Spline { points, closed } => {
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label("Closed:");
                                    if ui.checkbox(closed, "").changed() {
                                        mutated = true;
                                    }
                                });
                                ui.label(format!("Points: {}", points.len()));
                                let mut point_remove: Option<usize> = None;
                                for (p_idx, point) in points.iter_mut().enumerate() {
                                    ui.group(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.strong(format!("Point {}", p_idx));
                                            if ui.small_button("x").clicked() {
                                                point_remove = Some(p_idx);
                                            }
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("Pos:");
                                            for axis in 0..3 {
                                                if ui.add(egui::DragValue::new(&mut point.position[axis]).speed(0.05)).changed() {
                                                    mutated = true;
                                                }
                                            }
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("Tan In:");
                                            for axis in 0..3 {
                                                if ui.add(egui::DragValue::new(&mut point.tangent_in[axis]).speed(0.05)).changed() {
                                                    mutated = true;
                                                }
                                            }
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("Tan Out:");
                                            for axis in 0..3 {
                                                if ui.add(egui::DragValue::new(&mut point.tangent_out[axis]).speed(0.05)).changed() {
                                                    mutated = true;
                                                }
                                            }
                                        });
                                    });
                                }
                                if let Some(ri) = point_remove {
                                    points.remove(ri);
                                    mutated = true;
                                }
                                ui.separator();
                                if ui.button("+ Add Point").clicked() {
                                    points.push(super::spline::SplinePoint::default());
                                    mutated = true;
                                }
                            });
                        }
                        ComponentData::Terrain { resolution, world_size, heights, base_color } => {
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label("Resolution:");
                                    if ui.add(egui::DragValue::new(resolution).range(2..=512).speed(1.0)).changed() {
                                        let r = *resolution as usize;
                                        heights.resize(r * r, 0.0);
                                        mutated = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("World Size:");
                                    for axis in 0..2 {
                                        if ui.add(egui::DragValue::new(&mut world_size[axis]).speed(0.5).prefix(if axis == 0 { "X: " } else { "Z: " })).changed() {
                                            mutated = true;
                                        }
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Base Color:");
                                    let mut color = egui::Color32::from_rgb(
                                        (base_color[0] * 255.0) as u8,
                                        (base_color[1] * 255.0) as u8,
                                        (base_color[2] * 255.0) as u8,
                                    );
                                    if ui.color_edit_button_srgba(&mut color).changed() {
                                        base_color[0] = color.r() as f32 / 255.0;
                                        base_color[1] = color.g() as f32 / 255.0;
                                        base_color[2] = color.b() as f32 / 255.0;
                                        mutated = true;
                                    }
                                });
                                if !heights.is_empty() {
                                    let min_h = heights.iter().cloned().fold(f32::INFINITY, f32::min);
                                    let max_h = heights.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                                    ui.label(format!("Height range: {:.2} .. {:.2}", min_h, max_h));
                                }
                            });
                        }
                        ComponentData::SkinnedMesh { path, bones } => {
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label("GLB/GLTF Path:");
                                    if ui.text_edit_singleline(path).changed() {
                                        mutated = true;
                                    }
                                });
                                if ui.button("Load Bones").clicked() && !path.is_empty() {
                                    match super::skeleton::extract_bones_from_gltf(path) {
                                        Ok(loaded) => {
                                            *bones = loaded;
                                            mutated = true;
                                        }
                                        Err(_e) => {
                                            // Silently ignore load errors in inspector.
                                        }
                                    }
                                }
                                if !bones.is_empty() {
                                    ui.label(format!("{} bones:", bones.len()));
                                    egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
                                        for (i, bone) in bones.iter().enumerate() {
                                            let parent_label = match bone.parent_idx {
                                                Some(p) => format!(" (parent: {})", p),
                                                None => " (root)".to_string(),
                                            };
                                            ui.label(format!("  [{}] {}{}", i, bone.name, parent_label));
                                        }
                                    });
                                }
                            });
                        }
                        ComponentData::VisualScript { path } => {
                            ui.horizontal(|ui| {
                                ui.label("Script Path:");
                                if ui.text_edit_singleline(path).changed() {
                                    mutated = true;
                                }
                            });
                        }
                        ComponentData::NavMesh { cell_size, grid, width, height } => {
                            ui.horizontal(|ui| {
                                ui.label("Cell Size:");
                                if ui.add(egui::DragValue::new(cell_size).speed(0.1).range(0.1..=10.0)).changed() {
                                    mutated = true;
                                }
                            });
                            ui.label(format!("Grid: {}x{} ({} cells)", width, height, grid.len()));
                            let walkable = grid.iter().filter(|&&c| c).count();
                            ui.label(format!("Walkable: {}, Blocked: {}", walkable, grid.len() - walkable));
                            if ui.button("Bake NavMesh").clicked() {
                                bake_navmesh_requested = Some(*cell_size);
                            }
                        }
                    }
                });
            }

            if let Some(idx) = component_to_remove {
                entity.components.remove(idx);
                mutated = true;
            }
            if let Some(idx) = move_up_idx {
                entity.components.swap(idx, idx - 1);
                mutated = true;
            }
            if let Some(idx) = move_down_idx {
                entity.components.swap(idx, idx + 1);
                mutated = true;
            }

            ui.separator();

            // Delete button
            if ui
                .add(egui::Button::new(
                    egui::RichText::new("Delete Entity")
                        .color(egui::Color32::from_rgb(255, 120, 120)),
                ))
                .clicked()
            {
                delete_requested = true;
            }

            // Add Component searchable dropdown
            ui.separator();
            ui.label("Add Component:");
            if ui.button(if self.add_component_popup_open { "Cancel" } else { "Add Component..." }).clicked() {
                self.add_component_popup_open = !self.add_component_popup_open;
                if self.add_component_popup_open {
                    self.add_component_filter.clear();
                }
            }
            if self.add_component_popup_open {
                ui.horizontal(|ui| {
                    ui.label("\u{1f50d}");
                    ui.text_edit_singleline(&mut self.add_component_filter);
                });
                let filter_lower = self.add_component_filter.to_lowercase();
                let mut add_label = String::new();
                egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                    for (name, _) in ComponentData::default_all() {
                        if !filter_lower.is_empty() && !name.to_lowercase().contains(&filter_lower) {
                            continue;
                        }
                        if ui.selectable_label(false, name).clicked() {
                            add_label = name.to_string();
                        }
                    }
                });
                if !add_label.is_empty() {
                    for (name, default) in ComponentData::default_all() {
                        if name == add_label {
                            add_component = Some(default);
                            break;
                        }
                    }
                    self.add_component_popup_open = false;
                    self.add_component_filter.clear();
                }
            }

            // Paste Component button (enabled when clipboard has data)
            if self.component_clipboard.is_some() {
                if ui.button("Paste Component").on_hover_text("Paste copied component").clicked() {
                    paste_requested = true;
                }
            }
        }

        // Apply deferred component copy to clipboard.
        if let Some(c) = add_component_copy {
            self.component_clipboard = Some(c);
        }

        // Apply deferred paste: push clipboard component onto the entity.
        if paste_requested {
            if let Some(ref clip) = self.component_clipboard {
                let cloned = clip.clone();
                self.scene_snapshot();
                if let Some(entity) = self.scene_model.entities.get_mut(&selected_id) {
                    entity.components.push(cloned);
                }
                self.scene_model.modified = true;
                self.scene_needs_sync = true;
            }
        }

        // Deferred NavMesh bake (Phase 70): runs after the mutable entity borrow is released.
        if let Some(bake_cell_size) = bake_navmesh_requested {
            let nav = super::navmesh::bake_nav_grid(&self.scene_model, bake_cell_size);
            if let Some(entity) = self.scene_model.entities.get_mut(&selected_id) {
                for component in &mut entity.components {
                    if let ComponentData::NavMesh { grid, width, height, .. } = component {
                        *grid = nav.cells.clone();
                        *width = nav.width;
                        *height = nav.height;
                    }
                }
            }
            self.scene_model.modified = true;
            self.scene_needs_sync = true;
        }

        if let Some(component) = add_component {
            self.scene_snapshot();
            if let Some(entity) = self.scene_model.entities.get_mut(&selected_id) {
                entity.components.push(component);
            }
            self.scene_model.modified = true;
            self.scene_needs_sync = true;
        } else if delete_requested {
            self.scene_snapshot();
            // Delete all selected entities, not just the primary.
            let to_delete: Vec<u64> = self.scene_model.selected_ids.iter().copied().collect();
            for id in to_delete {
                self.scene_model.remove_entity(id);
            }
            self.scene_model.select_clear();
            self.primary_selected_id = None;
            self.scene_needs_sync = true;
        } else if mutated {
            self.scene_model.modified = true;
            self.scene_needs_sync = true;
        }

        // Push current PBR values to the material preview GPU sphere (Phase 8).
        // We read the selected entity's first mesh component immutably now that
        // the mutable entity borrow has been released.
        if let Some(entity) = self.scene_model.entities.get(&selected_id) {
            for component in &entity.components {
                let pbr = match component {
                    ComponentData::MeshCube { color, metallic, roughness, emissive, .. } => {
                        Some((*color, *metallic, *roughness, *emissive))
                    }
                    ComponentData::MeshSphere { color, metallic, roughness, emissive, .. } => {
                        Some((*color, *metallic, *roughness, *emissive))
                    }
                    ComponentData::MeshPlane { color, metallic, roughness, emissive, .. } => {
                        Some((*color, *metallic, *roughness, *emissive))
                    }
                    _ => None,
                };
                if let Some((color, met, rough, emis)) = pbr {
                    if color != self.material_preview_color
                        || met != self.material_preview_metallic
                        || rough != self.material_preview_roughness
                        || emis != self.material_preview_emissive
                    {
                        self.material_preview_color = color;
                        self.material_preview_metallic = met;
                        self.material_preview_roughness = rough;
                        self.material_preview_emissive = emis;
                        self.material_preview_dirty = true;
                    }
                    break;
                }
            }
        }

        // Handle prefab Revert / Apply requests (after the entity borrow is released).
        if revert_prefab_requested {
            if let Some(entity) = self.scene_model.entities.get(&selected_id) {
                if let Some(ref path) = entity.prefab_source {
                    let path = path.clone();
                    if let Ok(prefab) = crate::app::scene_editor::prefab::load_prefab(&path) {
                        self.scene_snapshot();
                        if let Some(prefab_root) = prefab.entities.get(&prefab.root_id) {
                            if let Some(entity) = self.scene_model.entities.get_mut(&selected_id) {
                                entity.components = prefab_root.components.clone();
                                entity.transform = prefab_root.transform.clone();
                                entity.name = prefab_root.name.clone();
                            }
                            self.scene_model.modified = true;
                            self.scene_needs_sync = true;
                        }
                    }
                }
            }
        }

        if apply_prefab_requested {
            if let Some(entity) = self.scene_model.entities.get(&selected_id) {
                if let Some(ref path) = entity.prefab_source {
                    let path = path.clone();
                    if let Some(prefab) = crate::app::scene_editor::prefab::build_prefab_from_entity(&self.scene_model, selected_id) {
                        match crate::app::scene_editor::prefab::save_prefab(&prefab, &path) {
                            Ok(_) => {
                                self.status_message = format!("Applied to prefab: {}", path);
                                self.status_message_timestamp = Some(std::time::Instant::now());
                            }
                            Err(e) => {
                                self.status_message = format!("Failed: {}", e);
                                self.status_message_timestamp = Some(std::time::Instant::now());
                            }
                        }
                    }
                }
            }
        }

        // Handle Unpack Prefab request (after entity borrow is released).
        if unpack_prefab_requested {
            self.scene_snapshot();
            crate::app::scene_editor::prefab::unpack_prefab(&mut self.scene_model, selected_id);
            self.scene_needs_sync = true;
        }

        // Handle audio preview Play / Stop requests (after entity borrow is released).
        if let Some(path) = audio_play_requested {
            // Stop any previously playing preview first.
            #[cfg(target_os = "macos")]
            {
                let _ = std::process::Command::new("killall")
                    .arg("afplay")
                    .stderr(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .spawn();
            }
            #[cfg(target_os = "linux")]
            {
                let _ = std::process::Command::new("killall")
                    .arg("aplay")
                    .stderr(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .spawn();
            }

            // Spawn background process to play the audio file.
            #[cfg(target_os = "macos")]
            {
                let _ = std::process::Command::new("afplay")
                    .arg(&path)
                    .stderr(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .spawn();
            }
            #[cfg(target_os = "linux")]
            {
                let _ = std::process::Command::new("aplay")
                    .arg(&path)
                    .stderr(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .spawn();
            }

            self.audio_preview_playing = true;
            self.audio_preview_path = path;
        }
        if audio_stop_requested {
            #[cfg(target_os = "macos")]
            {
                let _ = std::process::Command::new("killall")
                    .arg("afplay")
                    .stderr(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .spawn();
            }
            #[cfg(target_os = "linux")]
            {
                let _ = std::process::Command::new("killall")
                    .arg("aplay")
                    .stderr(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .spawn();
            }
            self.audio_preview_playing = false;
            self.audio_preview_path.clear();
        }

        // Handle Animator editor open request.
        if open_animator_editor {
            if !animator_path_for_editor.is_empty() {
                match crate::app::scene_editor::animator::load_animator(&animator_path_for_editor) {
                    Ok(ctrl) => {
                        self.editing_animator = Some(ctrl);
                    }
                    Err(_) => {
                        // File doesn't exist or is invalid; create a new controller.
                        self.editing_animator = Some(crate::app::scene_editor::animator::AnimatorController::default());
                    }
                }
            } else {
                self.editing_animator = Some(crate::app::scene_editor::animator::AnimatorController::default());
            }
            self.editing_animator_path = animator_path_for_editor;
            self.animator_editor_open = true;
        }

        // Phase 17: debug overlay for live values during play mode.
        if self.play_mode.is_active() {
            self.render_debug_overlay(ui);
        }
    }
}
