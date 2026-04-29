//! Hierarchy panel: tree view of scene entities with creation toolbar,
//! drag-and-drop reparenting, right-click context menu, and inline rename.

use super::model::*;
use crate::app::BerryCodeApp;

impl BerryCodeApp {
    /// Render the Hierarchy panel showing scene entities.
    pub(crate) fn render_scene_hierarchy(&mut self, ui: &mut egui::Ui) {
        // VS Code-style panel header
        let header_rect = ui.available_rect_before_wrap();
        let header_rect =
            egui::Rect::from_min_size(header_rect.min, egui::vec2(header_rect.width(), 28.0));
        ui.painter()
            .rect_filled(header_rect, 0.0, egui::Color32::from_rgb(37, 37, 38));
        ui.painter().line_segment(
            [header_rect.left_bottom(), header_rect.right_bottom()],
            egui::Stroke::new(1.0, egui::Color32::from_rgb(54, 57, 59)),
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(header_rect), |ui| {
            ui.horizontal_centered(|ui| {
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("HIERARCHY")
                        .size(11.0)
                        .color(egui::Color32::from_rgb(187, 187, 187)),
                );
            });
        });
        ui.advance_cursor_after_rect(header_rect);
        ui.add_space(4.0);

        // Play mode banner.
        if self.play_mode.is_active() {
            ui.colored_label(
                egui::Color32::from_rgb(255, 200, 80),
                "Play Mode Active - editing disabled",
            );
            ui.separator();
        }

        // --- Scene tabs ---
        let tab_count = self.scene_tabs.len();
        let mut switch_to: Option<usize> = None;

        let mut close_tab: Option<usize> = None;
        // VS-Code-style tab strip. Same colour palette as the editor
        // tabs so the two surfaces feel like one IDE: dark active bg,
        // a slightly lighter inactive bg, ACCENT-coloured 2px bottom
        // bar marking the active one.
        let tab_active_bg = egui::Color32::from_rgb(30, 30, 30);
        let tab_inactive_bg = egui::Color32::from_rgb(45, 45, 46);
        let tab_border = egui::Color32::from_rgb(37, 37, 38);
        let tab_active_indicator = egui::Color32::from_rgb(0, 122, 204);
        let tab_text_active = egui::Color32::from_rgb(255, 255, 255);
        let tab_text_inactive = egui::Color32::from_gray(180);
        // Pre-compute display labels with disambiguation: when two tabs
        // share the same base label, prefix the one with a different
        // path with its parent directory ("scenes/scene" vs
        // "other/scene"). VS Code does the same on filename collisions.
        let display_labels: Vec<String> = (0..tab_count)
            .map(|i| {
                let base = self.scene_tabs[i].label.clone();
                let collides = (0..tab_count).any(|j| j != i && self.scene_tabs[j].label == base);
                if collides {
                    let parent = self.scene_tabs[i]
                        .model
                        .file_path
                        .as_deref()
                        .and_then(|p| std::path::Path::new(p).parent())
                        .and_then(|p| p.file_name())
                        .map(|s| s.to_string_lossy().into_owned());
                    match parent {
                        Some(dir) if !dir.is_empty() => format!("{}/{}", dir, base),
                        _ => base,
                    }
                } else {
                    base
                }
            })
            .collect();

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            for i in 0..tab_count {
                let selected = i == self.active_scene_tab;
                let modified = self.scene_tabs[i].model.modified;
                let label_text = display_labels[i].clone();
                let bg = if selected {
                    tab_active_bg
                } else {
                    tab_inactive_bg
                };
                let text_color = if selected {
                    tab_text_active
                } else {
                    tab_text_inactive
                };

                let frame = egui::Frame::NONE
                    .fill(bg)
                    .inner_margin(egui::Margin::symmetric(10, 5))
                    .stroke(egui::Stroke::new(1.0, tab_border));
                let resp = frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        // Modified dot — VS Code shows a filled circle
                        // in the close-button slot when the file has
                        // unsaved edits; we emulate that with a small
                        // dot in front of the name.
                        if modified {
                            ui.label(
                                egui::RichText::new("●")
                                    .size(10.0)
                                    .color(egui::Color32::from_rgb(126, 168, 255)),
                            );
                        }
                        let label_resp = ui.add(
                            egui::Label::new(
                                egui::RichText::new(&label_text)
                                    .size(12.0)
                                    .color(text_color),
                            )
                            .sense(egui::Sense::click()),
                        );
                        if label_resp.clicked() && i != self.active_scene_tab {
                            switch_to = Some(i);
                        }
                        // Always show × so the user can close any tab.
                        // The close handler below makes sure the panel
                        // never ends up with zero tabs (it injects a
                        // fresh blank one if you close the last).
                        let close_btn = egui::Button::new(
                            egui::RichText::new("×").size(12.0).color(text_color),
                        )
                        .frame(false);
                        if ui.add(close_btn).on_hover_text("Close scene").clicked() {
                            close_tab = Some(i);
                        }
                    });
                });

                // Active-tab indicator bar
                if selected {
                    let r = resp.response.rect;
                    let bar = egui::Rect::from_min_size(
                        egui::pos2(r.left(), r.bottom() - 2.0),
                        egui::vec2(r.width(), 2.0),
                    );
                    ui.painter().rect_filled(bar, 0.0, tab_active_indicator);
                }
            }

            // The "Open Scene…" dropdown was retired — opening a
            // `.bscene` from the file tree (double-click) already
            // covers the same need without duplicating the listing.
        });
        if tab_count > 0 {
            ui.separator();
        }

        if let Some(new_idx) = switch_to {
            if new_idx < self.scene_tabs.len() {
                self.scene_tabs[self.active_scene_tab].model = self.scene_model.clone();
                self.active_scene_tab = new_idx;
                self.scene_model = self.scene_tabs[new_idx].model.clone();
                self.scene_needs_sync = true;
                self.primary_selected_id = None;
            }
        }
        if let Some(idx) = close_tab {
            // Persist the current scene back into its tab so we don't
            // close stale state; then drop the tab and adjust the active
            // index. If this was the last tab, immediately push a fresh
            // empty Untitled so the panel never ends up with zero tabs
            // (the rendering and indexing both assume at least one).
            self.scene_tabs[self.active_scene_tab].model = self.scene_model.clone();
            self.scene_tabs.remove(idx);
            if self.scene_tabs.is_empty() {
                self.scene_tabs
                    .push(crate::app::scene_editor::scene_tabs::SceneTab::new(
                        SceneModel::new(),
                        "Untitled".to_string(),
                    ));
                self.active_scene_tab = 0;
            } else if self.active_scene_tab >= self.scene_tabs.len() {
                self.active_scene_tab = self.scene_tabs.len() - 1;
            } else if idx < self.active_scene_tab {
                self.active_scene_tab -= 1;
            }
            self.scene_model = self.scene_tabs[self.active_scene_tab].model.clone();
            self.scene_needs_sync = true;
            self.primary_selected_id = None;
        }

        // Single toolbar row — all scene-level actions in one place so
        // the panel header doesn't sprawl across four rows. New / Save
        // / Add Entity / Tools all live here. Sizing is uniform to
        // match the menu_button height (otherwise `small_button` looks
        // shorter and the row reads "scattered").
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            if ui.button("New").clicked() {
                // Open the name-your-scene dialog instead of committing
                // straight to `Untitled N`. Commit happens below when
                // the user presses OK / Enter.
                if self.new_scene_dialog.is_none() {
                    self.new_scene_dialog = Some(String::new());
                }
            }
            if ui.button("Save").clicked() {
                self.save_current_scene();
            }
            // Tools dropdown
            ui.menu_button("Tools", |ui| {
                if ui.button("Profiler").clicked() {
                    self.profiler.open = true;
                    self.tool_panel_open = true;
                    self.active_tool_tab = crate::app::dock::ToolTab::Profiler;
                    ui.close();
                }
                if ui.button("Timeline").clicked() {
                    self.tool_panel_open = true;
                    self.active_tool_tab = crate::app::dock::ToolTab::Timeline;
                    ui.close();
                }
                if ui.button("Dopesheet").clicked() {
                    self.tool_panel_open = true;
                    self.active_tool_tab = crate::app::dock::ToolTab::Dopesheet;
                    ui.close();
                }
                if ui.button("Systems").clicked() {
                    self.system_graph_open = !self.system_graph_open;
                    ui.close();
                }
                if ui.button("Events").clicked() {
                    self.event_monitor_open = !self.event_monitor_open;
                    ui.close();
                }
                if ui.button("Queries").clicked() {
                    self.query_viz_open = !self.query_viz_open;
                    ui.close();
                }
                if ui.button("States").clicked() {
                    self.state_editor_open = !self.state_editor_open;
                    ui.close();
                }
                if ui.button("Plugins").clicked() {
                    self.plugin_browser_open = !self.plugin_browser_open;
                    ui.close();
                }
                if ui.button("Shader Graph").clicked() {
                    // `editing_shader_graph` is seeded with a default
                    // graph at app startup (mod.rs), so we only need
                    // to toggle the open flag here. If the user has
                    // since cleared it, fall back to a fresh default
                    // so the menu entry stays useful.
                    if self.editing_shader_graph.is_none() {
                        self.editing_shader_graph =
                            Some(crate::app::scene_editor::shader_graph::ShaderGraph::default());
                    }
                    self.shader_graph_editor_open = !self.shader_graph_editor_open;
                    ui.close();
                }
                if ui.button("Blend Tree").clicked() {
                    // Lazily seed a starter BlendTree on first open so
                    // the user can see the 1D/2D visualisation
                    // immediately. Subsequent opens preserve whatever
                    // tree the user has been editing.
                    if self.editing_blend_tree.is_none() {
                        self.editing_blend_tree =
                            Some(crate::app::scene_editor::animator::BlendTree {
                                name: "New Blend Tree".into(),
                                blend_type: crate::app::scene_editor::animator::BlendType::Simple1D,
                                parameter_x: "Speed".into(),
                                parameter_y: String::new(),
                                children: vec![
                                    crate::app::scene_editor::animator::BlendTreeChild {
                                        motion: crate::app::scene_editor::animator::Motion::Clip {
                                            clip_name: "Idle".into(),
                                        },
                                        threshold: 0.0,
                                        position: [0.0, 0.0],
                                        time_scale: 1.0,
                                    },
                                    crate::app::scene_editor::animator::BlendTreeChild {
                                        motion: crate::app::scene_editor::animator::Motion::Clip {
                                            clip_name: "Run".into(),
                                        },
                                        threshold: 1.0,
                                        position: [1.0, 0.0],
                                        time_scale: 1.0,
                                    },
                                ],
                            });
                    }
                    self.blend_tree_editor_open = !self.blend_tree_editor_open;
                    ui.close();
                }
                ui.separator();
                if ui.button("Export .scn.ron").clicked() {
                    let path = self
                        .scene_model
                        .file_path
                        .clone()
                        .unwrap_or_else(|| format!("{}/scenes/scene.bscene", self.root_path));
                    match crate::app::scene_editor::bevy_scene_export::save_bevy_scene(
                        &self.scene_model,
                        &path,
                    ) {
                        Ok(p) => {
                            self.status_message = format!("Exported: {}", p);
                            self.status_message_timestamp = Some(std::time::Instant::now());
                        }
                        Err(e) => {
                            self.status_message = format!("Export failed: {}", e);
                            self.status_message_timestamp = Some(std::time::Instant::now());
                        }
                    }
                    ui.close();
                }
            });
            // Filename label retired — the tab strip + the modified
            // dot inside each tab already convey the same info.
            ui.menu_button("+ Add Entity", |ui| {
                if ui.button("+ Cube").clicked() {
                    self.scene_snapshot();
                    self.scene_model.add_entity(
                        "Cube".into(),
                        vec![ComponentData::MeshCube {
                            size: 1.0,
                            color: [0.5, 0.5, 1.0],
                            metallic: 0.0,
                            roughness: 0.5,
                            emissive: [0.0, 0.0, 0.0],
                            texture_path: None,
                            normal_map_path: None,
                        }],
                    );
                }
                if ui.button("+ Sphere").clicked() {
                    self.scene_snapshot();
                    self.scene_model.add_entity(
                        "Sphere".into(),
                        vec![ComponentData::MeshSphere {
                            radius: 0.5,
                            color: [1.0, 0.5, 0.5],
                            metallic: 0.0,
                            roughness: 0.5,
                            emissive: [0.0, 0.0, 0.0],
                            texture_path: None,
                            normal_map_path: None,
                        }],
                    );
                }
                if ui.button("+ Plane").clicked() {
                    self.scene_snapshot();
                    self.scene_model.add_entity(
                        "Plane".into(),
                        vec![ComponentData::MeshPlane {
                            size: 10.0,
                            color: [0.3, 0.3, 0.3],
                            metallic: 0.0,
                            roughness: 0.5,
                            emissive: [0.0, 0.0, 0.0],
                            texture_path: None,
                            normal_map_path: None,
                        }],
                    );
                }
                if ui.button("+ Light").clicked() {
                    self.scene_snapshot();
                    self.scene_model.add_entity(
                        "Light".into(),
                        vec![ComponentData::Light {
                            intensity: 10_000.0,
                            color: [1.0, 1.0, 1.0],
                        }],
                    );
                }
                if ui.button("+ Camera").clicked() {
                    self.scene_snapshot();
                    self.scene_model
                        .add_entity("Camera".into(), vec![ComponentData::Camera]);
                }
                if ui.button("+ SpotLight").clicked() {
                    self.scene_snapshot();
                    self.scene_model.add_entity(
                        "Spot Light".into(),
                        vec![ComponentData::SpotLight {
                            intensity: 10_000.0,
                            color: [1.0, 1.0, 1.0],
                            range: 20.0,
                            inner_angle: 0.5,
                            outer_angle: 0.8,
                        }],
                    );
                }
                if ui.button("+ DirLight").clicked() {
                    self.scene_snapshot();
                    self.scene_model.add_entity(
                        "Directional Light".into(),
                        vec![ComponentData::DirectionalLight {
                            intensity: 10_000.0,
                            color: [1.0, 1.0, 1.0],
                            shadows: false,
                        }],
                    );
                }
                if ui.button("+ Audio Source").clicked() {
                    self.scene_snapshot();
                    self.scene_model.add_entity(
                        "Audio Source".into(),
                        vec![ComponentData::AudioSource {
                            path: String::new(),
                            volume: 1.0,
                            looped: false,
                            autoplay: true,
                        }],
                    );
                }
                if ui.button("+ Audio Listener").clicked() {
                    self.scene_snapshot();
                    self.scene_model
                        .add_entity("Audio Listener".into(), vec![ComponentData::AudioListener]);
                }
                if ui.button("+ Rigidbody").clicked() {
                    self.scene_snapshot();
                    self.scene_model.add_entity(
                        "Rigidbody".into(),
                        vec![
                            ComponentData::RigidBody {
                                body_type: RigidBodyType::Dynamic,
                                mass: 1.0,
                            },
                            ComponentData::Collider {
                                shape: ColliderShape::Box {
                                    half_extents: [0.5, 0.5, 0.5],
                                },
                                friction: 0.5,
                                restitution: 0.0,
                            },
                        ],
                    );
                }
                if ui.button("+ UI Text").clicked() {
                    self.scene_snapshot();
                    self.scene_model.add_entity(
                        "UI Text".into(),
                        vec![ComponentData::UiText {
                            text: "Hello".into(),
                            font_size: 24.0,
                            color: [1.0, 1.0, 1.0, 1.0],
                        }],
                    );
                }
                if ui.button("+ UI Button").clicked() {
                    self.scene_snapshot();
                    self.scene_model.add_entity(
                        "UI Button".into(),
                        vec![ComponentData::UiButton {
                            label: "Button".into(),
                            background: [0.2, 0.2, 0.3, 1.0],
                        }],
                    );
                }
                if ui.button("+ UI Image").clicked() {
                    self.scene_snapshot();
                    self.scene_model.add_entity(
                        "UI Image".into(),
                        vec![ComponentData::UiImage {
                            path: String::new(),
                            tint: [1.0, 1.0, 1.0, 1.0],
                        }],
                    );
                }
                if ui.button("+ Particle Emitter").clicked() {
                    self.scene_snapshot();
                    self.scene_model.add_entity(
                        "Particle Emitter".into(),
                        vec![ComponentData::ParticleEmitter {
                            rate: 30.0,
                            lifetime: 1.5,
                            speed: 2.0,
                            spread: 0.3,
                            start_size: 0.1,
                            end_size: 0.0,
                            start_color: [1.0, 0.6, 0.2, 1.0],
                            end_color: [1.0, 0.0, 0.0, 0.0],
                            max_particles: 200,
                            gravity: -1.0,
                        }],
                    );
                }
                if ui.button("+ Animation").clicked() {
                    self.scene_snapshot();
                    self.scene_model.add_entity(
                        "Animated Object".into(),
                        vec![
                            ComponentData::MeshCube {
                                size: 1.0,
                                color: [0.5, 0.8, 1.0],
                                metallic: 0.0,
                                roughness: 0.5,
                                emissive: [0.0, 0.0, 0.0],
                                texture_path: None,
                                normal_map_path: None,
                            },
                            ComponentData::Animation {
                                duration: 2.0,
                                tracks: vec![],
                                looped: true,
                            },
                        ],
                    );
                }
                if ui.button("+ Custom Script").clicked() {
                    self.scene_snapshot();
                    self.scene_model.add_entity(
                        "Script Entity".into(),
                        vec![ComponentData::CustomScript {
                            type_name: String::new(),
                            fields: vec![],
                            script_path: String::new(),
                        }],
                    );
                }
                if ui.button("+ Skybox").clicked() {
                    self.scene_snapshot();
                    self.scene_model.add_entity(
                        "Skybox".into(),
                        vec![ComponentData::Skybox {
                            path: String::new(),
                        }],
                    );
                }
                if ui.button("+ Animator").clicked() {
                    self.scene_snapshot();
                    self.scene_model.add_entity(
                        "Animated".into(),
                        vec![ComponentData::Animator {
                            controller_path: String::new(),
                        }],
                    );
                }
                if ui.button("+ LOD Group").clicked() {
                    self.scene_snapshot();
                    self.scene_model.add_entity(
                        "LOD Group".into(),
                        vec![ComponentData::LodGroup { levels: vec![] }],
                    );
                }
                if ui.button("+ Spline").clicked() {
                    self.scene_snapshot();
                    self.scene_model.add_entity(
                        "Spline".into(),
                        vec![ComponentData::Spline {
                            points: vec![],
                            closed: false,
                        }],
                    );
                }
                if ui.button("+ Terrain").clicked() {
                    self.scene_snapshot();
                    self.scene_model.add_entity(
                        "Terrain".into(),
                        vec![ComponentData::Terrain {
                            resolution: 64,
                            world_size: [100.0, 100.0],
                            heights: vec![0.0; 64 * 64],
                            base_color: [0.3, 0.5, 0.3],
                        }],
                    );
                }
                if ui.button("+ Skinned Mesh").clicked() {
                    self.scene_snapshot();
                    self.scene_model.add_entity(
                        "Skinned Mesh".into(),
                        vec![ComponentData::SkinnedMesh {
                            path: String::new(),
                            bones: vec![],
                        }],
                    );
                }
                if ui.button("+ Visual Script").clicked() {
                    self.scene_snapshot();
                    self.scene_model.add_entity(
                        "Visual Script".into(),
                        vec![ComponentData::VisualScript {
                            path: String::new(),
                        }],
                    );
                }

                // --- From Asset (3D models & prefabs in project) ---
                ui.separator();
                ui.menu_button("From Asset...", |ui| {
                    let assets_dir = std::path::Path::new(&self.root_path).join("assets");
                    let mut model_files: Vec<std::path::PathBuf> = Vec::new();
                    Self::collect_model_assets(&assets_dir, &mut model_files);
                    // Also check root for .bprefab files
                    let mut prefab_files: Vec<std::path::PathBuf> = Vec::new();
                    Self::collect_prefab_assets(
                        std::path::Path::new(&self.root_path),
                        &mut prefab_files,
                    );

                    if model_files.is_empty() && prefab_files.is_empty() {
                        ui.label(
                            egui::RichText::new("No 3D assets found")
                                .color(egui::Color32::from_gray(120))
                                .italics(),
                        );
                    } else {
                        if !model_files.is_empty() {
                            ui.label(
                                egui::RichText::new("3D Models")
                                    .size(11.0)
                                    .color(egui::Color32::from_gray(150)),
                            );
                            for path in &model_files {
                                let file_name = path
                                    .file_name()
                                    .map(|s| s.to_string_lossy().to_string())
                                    .unwrap_or_default();
                                let display = format!("\u{ea7b} {}", file_name); // codicon symbol-file
                                if ui.button(&display).clicked() {
                                    let path_str = path.to_string_lossy().to_string();
                                    let entity_name = path
                                        .file_stem()
                                        .map(|s| s.to_string_lossy().to_string())
                                        .unwrap_or_else(|| "Model".to_string());
                                    self.scene_snapshot();
                                    let new_id = self.scene_model.add_entity(
                                        entity_name,
                                        vec![ComponentData::MeshFromFile {
                                            path: path_str,
                                            texture_path: None,
                                            normal_map_path: None,
                                        }],
                                    );
                                    self.scene_model.select_only(new_id);
                                    self.primary_selected_id = Some(new_id);
                                    self.scene_needs_sync = true;
                                    ui.close();
                                }
                            }
                        }
                        if !prefab_files.is_empty() {
                            ui.separator();
                            ui.label(
                                egui::RichText::new("Prefabs")
                                    .size(11.0)
                                    .color(egui::Color32::from_gray(150)),
                            );
                            for path in &prefab_files {
                                let file_name = path
                                    .file_name()
                                    .map(|s| s.to_string_lossy().to_string())
                                    .unwrap_or_default();
                                let display = format!("\u{eb61} {}", file_name); // codicon symbol-class
                                if ui.button(&display).clicked() {
                                    let path_str = path.to_string_lossy().to_string();
                                    self.scene_snapshot();
                                    self.instantiate_prefab_from_path(&path_str);
                                    self.scene_needs_sync = true;
                                    ui.close();
                                }
                            }
                        }
                    }
                });
            });
        });

        ui.separator();

        // ── New-scene name dialog ───────────────────────────────────
        // Opens when the user clicks New. Inline (not a floating
        // window) so it stays scoped to the Hierarchy panel; commits
        // on Enter or "Create", cancels on Esc or "Cancel".
        let mut commit_new_scene: Option<String> = None;
        let mut cancel_new_scene = false;
        if let Some(buf) = self.new_scene_dialog.as_mut() {
            egui::Frame::NONE
                .fill(egui::Color32::from_rgb(38, 40, 46))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(58, 60, 68)))
                .corner_radius(egui::CornerRadius::same(5))
                .inner_margin(egui::Margin::symmetric(10, 8))
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("New scene name")
                            .size(11.0)
                            .color(egui::Color32::from_gray(180)),
                    );
                    let resp = ui.add(
                        egui::TextEdit::singleline(buf)
                            .hint_text("e.g. main_menu")
                            .desired_width(f32::INFINITY),
                    );
                    resp.request_focus();
                    let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
                    ui.horizontal(|ui| {
                        if ui.button("Create").clicked() || enter {
                            commit_new_scene = Some(buf.clone());
                        }
                        if ui.button("Cancel").clicked() || escape {
                            cancel_new_scene = true;
                        }
                    });
                });
            ui.add_space(4.0);
        }
        if cancel_new_scene {
            self.new_scene_dialog = None;
        }
        if let Some(typed) = commit_new_scene {
            // Sanitise: trim, collapse whitespace runs, strip path
            // separators so the user can't escape the scenes dir.
            // Empty falls back to the next available "Untitled N".
            let clean: String = typed
                .trim()
                .replace(['/', '\\', ':'], "_")
                .chars()
                .collect();
            self.scene_snapshot();
            if let Some(tab) = self.scene_tabs.get_mut(self.active_scene_tab) {
                tab.model = self.scene_model.clone();
            }
            let (new_label, new_path) = resolve_new_scene_path(&self.root_path, &clean);
            let mut new_model = SceneModel::new();
            new_model.file_path = Some(new_path.clone());
            self.scene_tabs
                .push(crate::app::scene_editor::scene_tabs::SceneTab::new(
                    new_model.clone(),
                    new_label.clone(),
                ));
            self.active_scene_tab = self.scene_tabs.len() - 1;
            self.scene_model = new_model;
            self.current_scene_path = Some(new_path.clone());
            self.scene_needs_sync = true;
            self.primary_selected_id = None;
            // Write the .bscene up-front so the file exists before the
            // codegen step (modular projects need the parent dir to
            // already be there). Earlier flow relied on
            // `save_current_scene` to do everything, which left a
            // narrow window where the file was reported "created" but
            // hadn't actually been flushed when the user looked.
            if let Err(e) = crate::app::scene_editor::serialization::save_scene_to_ron(
                &self.scene_model,
                &new_path,
            ) {
                tracing::warn!("🆕 New scene write failed for {}: {}", new_path, e);
                self.status_message = format!("New scene write failed: {}", e);
                self.status_message_timestamp = Some(std::time::Instant::now());
            }
            self.save_current_scene();
            self.new_scene_dialog = None;
            tracing::info!(
                "🆕 New scene tab: {} (count={}, path={}, exists={})",
                new_label,
                self.scene_tabs.len(),
                new_path,
                std::path::Path::new(&new_path).exists()
            );
        }

        // Search filter
        ui.horizontal(|ui| {
            ui.label("Filter:");
            ui.add(
                egui::TextEdit::singleline(&mut self.hierarchy_filter)
                    .hint_text("name…")
                    .desired_width(f32::INFINITY),
            );
            if !self.hierarchy_filter.is_empty() && ui.small_button("x").clicked() {
                self.hierarchy_filter.clear();
            }
        });

        ui.separator();

        // Pre-compute the set of entities that match the filter (or all of
        // them, including ancestors of matches, when a filter is active).
        let filter = self.hierarchy_filter.trim().to_lowercase();
        let visible: Option<std::collections::HashSet<u64>> = if filter.is_empty() {
            None
        } else {
            Some(self.compute_visible_entities(&filter))
        };

        // Reset drop target each frame; the per-row code re-establishes it.
        self.hierarchy_drop_target = None;

        // Entity tree
        let mut switch_to_other_tab: Option<(usize, u64)> = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let root_ids = self.scene_model.root_entities.clone();
                if root_ids.is_empty() {
                    ui.label(
                        egui::RichText::new("(empty scene)")
                            .italics()
                            .color(egui::Color32::from_gray(140)),
                    );
                } else {
                    for id in root_ids {
                        self.render_entity_tree_node(ui, id, 0, visible.as_ref());
                    }

                    // Drop zone for "make root" — a thin strip at the bottom.
                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), 12.0),
                        egui::Sense::hover(),
                    );
                    if self.hierarchy_dragged.is_some()
                        && ui.input(|i| i.pointer.any_down())
                        && response.hovered()
                    {
                        ui.painter().rect_filled(
                            rect,
                            2.0,
                            egui::Color32::from_rgba_premultiplied(80, 130, 255, 60),
                        );
                        self.hierarchy_drop_target = Some(None);
                    }
                }

                // ── Other open scenes (item 3 lite) ─────────────────
                // For every loaded tab that isn't the active one, show
                // a collapsed list of its entities. Read-only here —
                // clicking an entity row switches the active tab to
                // that scene so the standard tree UX takes over for
                // editing. This makes "which entity belongs to which
                // scene" answerable at a glance without juggling tabs.
                let other_indices: Vec<usize> = (0..self.scene_tabs.len())
                    .filter(|i| *i != self.active_scene_tab)
                    .collect();
                if !other_indices.is_empty() {
                    ui.add_space(8.0);
                    ui.separator();
                    ui.label(
                        egui::RichText::new("Other open scenes")
                            .size(11.0)
                            .color(egui::Color32::from_gray(150)),
                    );
                    for idx in other_indices {
                        let label = display_labels
                            .get(idx)
                            .cloned()
                            .unwrap_or_else(|| self.scene_tabs[idx].label.clone());
                        let entity_count = self.scene_tabs[idx].model.entities.len();
                        let header_id = ui.make_persistent_id(("other_scene", idx));
                        egui::collapsing_header::CollapsingState::load_with_default_open(
                            ui.ctx(),
                            header_id,
                            false,
                        )
                        .show_header(ui, |ui| {
                            ui.label(
                                egui::RichText::new(format!(
                                    "{}  ({} entities)",
                                    label, entity_count
                                ))
                                .color(egui::Color32::from_gray(190)),
                            );
                        })
                        .body(|ui| {
                            // Show top-level entities only; tapping
                            // one switches tabs and selects it.
                            let roots = self.scene_tabs[idx].model.root_entities.clone();
                            for ent_id in roots {
                                let name = self.scene_tabs[idx]
                                    .model
                                    .entities
                                    .get(&ent_id)
                                    .map(|e| e.name.clone())
                                    .unwrap_or_else(|| format!("Entity #{}", ent_id));
                                let resp = ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(format!("  {}", name))
                                            .color(egui::Color32::from_gray(180)),
                                    )
                                    .sense(egui::Sense::click()),
                                );
                                if resp.clicked() {
                                    switch_to_other_tab = Some((idx, ent_id));
                                }
                            }
                        });
                    }
                }
            });

        // Tab switch requested from the "Other open scenes" panel —
        // mirror the same snapshot-then-swap dance the tab strip uses
        // up top, plus pre-select the entity the user clicked.
        if let Some((new_idx, ent_id)) = switch_to_other_tab {
            if new_idx < self.scene_tabs.len() && new_idx != self.active_scene_tab {
                self.scene_tabs[self.active_scene_tab].model = self.scene_model.clone();
                self.active_scene_tab = new_idx;
                self.scene_model = self.scene_tabs[new_idx].model.clone();
                self.scene_needs_sync = true;
                self.primary_selected_id = Some(ent_id);
                self.scene_model.select_only(ent_id);
            }
        }

        // Resolve a completed drag (mouse released this frame).
        let pointer_released = ui.input(|i| i.pointer.any_released());
        if pointer_released {
            if let (Some(child_id), Some(target)) =
                (self.hierarchy_dragged, self.hierarchy_drop_target)
            {
                // `target` is the new parent (None = root). Skip if dropping on
                // self or on the existing parent (no-op).
                let current_parent = self
                    .scene_model
                    .entities
                    .get(&child_id)
                    .and_then(|e| e.parent);
                if target != Some(child_id) && current_parent != target {
                    self.command_history.execute(
                        crate::app::scene_editor::history::SceneCommand::ReparentEntity {
                            entity_id: child_id,
                            old_parent: current_parent,
                            new_parent: target,
                        },
                        &self.scene_model,
                    );
                    self.scene_model.set_parent(child_id, target);
                    self.scene_needs_sync = true;
                }
            }
            self.hierarchy_dragged = None;
            self.hierarchy_drop_target = None;
        }
    }

    /// Count entities whose name contains the filter string (case-insensitive).
    /// Testable without UI context.
    pub fn count_filtered_entities(&self, filter: &str) -> usize {
        if filter.is_empty() {
            return self.scene_model.entities.len();
        }
        let lower = filter.to_lowercase();
        self.scene_model
            .entities
            .values()
            .filter(|e| e.name.to_lowercase().contains(&lower))
            .count()
    }

    /// Get names of all entities matching a filter (case-insensitive).
    /// Testable without UI context.
    pub fn get_filtered_entity_names(&self, filter: &str) -> Vec<String> {
        if filter.is_empty() {
            return self
                .scene_model
                .entities
                .values()
                .map(|e| e.name.clone())
                .collect();
        }
        let lower = filter.to_lowercase();
        self.scene_model
            .entities
            .values()
            .filter(|e| e.name.to_lowercase().contains(&lower))
            .map(|e| e.name.clone())
            .collect()
    }

    /// Compute the set of entity IDs that should be visible given a non-empty
    /// (already lower-cased) name filter. Includes matched entities and all of
    /// their ancestors so the path to the match stays expanded.
    pub fn compute_visible_entities(&self, filter: &str) -> std::collections::HashSet<u64> {
        let mut visible = std::collections::HashSet::new();
        for (id, entity) in &self.scene_model.entities {
            if entity.name.to_lowercase().contains(filter) {
                visible.insert(*id);
                // Walk up to the root so ancestors stay visible.
                let mut current = entity.parent;
                while let Some(pid) = current {
                    if !visible.insert(pid) {
                        break; // already visited
                    }
                    current = self.scene_model.entities.get(&pid).and_then(|e| e.parent);
                }
            }
        }
        visible
    }

    /// Recursively render a single entity (and its children) in the hierarchy.
    fn render_entity_tree_node(
        &mut self,
        ui: &mut egui::Ui,
        id: u64,
        depth: usize,
        visible: Option<&std::collections::HashSet<u64>>,
    ) {
        if let Some(set) = visible {
            if !set.contains(&id) {
                return;
            }
        }

        // Snapshot name + children + enabled so we don't hold a borrow while mutating below.
        let (name, children, entity_enabled) = match self.scene_model.entities.get(&id) {
            Some(entity) => (entity.name.clone(), entity.children.clone(), entity.enabled),
            None => return,
        };

        let selected = self.scene_model.is_selected(id);
        let is_renaming = self.renaming_entity_id == Some(id);

        // Track per-row interactions so we can act after the closure returns.
        let mut request_rename = false;
        let mut request_duplicate = false;
        let mut request_delete = false;
        let mut request_add_child = false;
        let mut request_save_prefab = false;
        let mut request_copy = false;
        let mut request_paste = false;
        let mut commit_rename = false;
        let mut cancel_rename = false;
        let mut request_toggle_enabled = false;

        // --- File-tree-style row rendering ---
        let indent = depth as f32 * 16.0;
        let row_height = 22.0;
        let text_color = egui::Color32::from_rgb(229, 229, 229);
        let hover_bg = egui::Color32::from_rgb(42, 45, 46); // #2A2D2E
        let selected_bg = egui::Color32::from_rgb(4, 57, 94); // #04395E

        if is_renaming {
            // Inline rename: use a horizontal layout with text edit
            ui.horizontal(|ui| {
                ui.add_space(indent + 20.0);
                let edit = ui
                    .add(egui::TextEdit::singleline(&mut self.rename_buffer).desired_width(160.0));
                edit.request_focus();
                if edit.lost_focus() {
                    if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        commit_rename = true;
                    } else {
                        cancel_rename = true;
                    }
                }
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    cancel_rename = true;
                }
            });
        } else {
            let (rect, response) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), row_height),
                egui::Sense::click_and_drag(),
            );

            // Selection / hover highlight (full row width, VS Code style)
            if selected {
                ui.painter().rect_filled(rect, 0.0, selected_bg);
            } else if response.hovered() {
                ui.painter().rect_filled(rect, 0.0, hover_bg);
            }

            // Indent guide lines
            for d in 1..depth {
                let line_x = rect.left() + d as f32 * 16.0 + 8.0;
                ui.painter().line_segment(
                    [
                        egui::pos2(line_x, rect.top()),
                        egui::pos2(line_x, rect.bottom()),
                    ],
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(55, 55, 55)),
                );
            }

            let text_left = rect.left() + indent;

            // Visibility icon (eye)
            let eye_icon = if entity_enabled {
                "\u{eb63}" // codicon eye
            } else {
                "\u{eb64}" // codicon eye-closed
            };
            let eye_color = if entity_enabled {
                egui::Color32::from_gray(160)
            } else {
                egui::Color32::from_gray(70)
            };
            let eye_rect = egui::Rect::from_center_size(
                egui::pos2(text_left + 6.0, rect.center().y),
                egui::vec2(14.0, row_height),
            );
            ui.painter().text(
                eye_rect.center(),
                egui::Align2::CENTER_CENTER,
                eye_icon,
                egui::FontId::new(12.0, egui::FontFamily::Name("codicon".into())),
                eye_color,
            );
            // Check click on eye area
            if response.clicked() {
                if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                    if eye_rect.contains(pos) {
                        request_toggle_enabled = true;
                    }
                }
            }

            // Chevron (for entities with children)
            if !children.is_empty() {
                let chevron = "\u{eab4}"; // chevron-down
                ui.painter().text(
                    egui::pos2(text_left + 16.0, rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    chevron,
                    egui::FontId::new(10.0, egui::FontFamily::Name("codicon".into())),
                    egui::Color32::from_rgb(150, 150, 150),
                );
            }

            // Entity icon
            let entity_icon = "\u{eb5f}"; // codicon symbol-misc
            let icon_color = if !entity_enabled {
                egui::Color32::from_gray(70)
            } else {
                egui::Color32::from_rgb(120, 180, 240)
            };
            ui.painter().text(
                egui::pos2(text_left + 30.0, rect.center().y),
                egui::Align2::LEFT_CENTER,
                entity_icon,
                egui::FontId::new(14.0, egui::FontFamily::Name("codicon".into())),
                icon_color,
            );

            // Entity name
            let name_color = if !entity_enabled {
                egui::Color32::from_gray(90)
            } else if selected {
                egui::Color32::WHITE
            } else {
                text_color
            };
            ui.painter().text(
                egui::pos2(text_left + 48.0, rect.center().y),
                egui::Align2::LEFT_CENTER,
                &name,
                egui::FontId::proportional(13.0),
                name_color,
            );

            // Drop target highlight
            if self.hierarchy_dragged.is_some()
                && self.hierarchy_dragged != Some(id)
                && ui.input(|i| i.pointer.any_down())
            {
                if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
                    if rect.contains(pos) {
                        self.hierarchy_drop_target = Some(Some(id));
                        ui.painter().rect_stroke(
                            rect,
                            0.0,
                            egui::Stroke::new(1.5, egui::Color32::from_rgb(80, 200, 255)),
                            egui::StrokeKind::Middle,
                        );
                    }
                }
            }

            // Interactions
            if response.clicked() && !request_toggle_enabled {
                let modifiers = ui.input(|i| i.modifiers);
                if modifiers.shift {
                    self.scene_model.select_add(id);
                } else if modifiers.command {
                    self.scene_model.select_toggle(id);
                } else {
                    self.scene_model.select_only(id);
                }
                self.primary_selected_id = Some(id);
            }
            if response.double_clicked() {
                request_rename = true;
            }

            // Drag-and-drop source
            if response.drag_started() {
                self.hierarchy_dragged = Some(id);
                self.scene_model.select_only(id);
                self.primary_selected_id = Some(id);
            }

            response.context_menu(|ui| {
                if ui.button("Rename").clicked() {
                    request_rename = true;
                    ui.close();
                }
                if ui.button("Duplicate").clicked() {
                    request_duplicate = true;
                    ui.close();
                }
                if ui.button("Delete").clicked() {
                    request_delete = true;
                    ui.close();
                }
                ui.separator();
                if ui.button("Copy").clicked() {
                    request_copy = true;
                    ui.close();
                }
                if ui
                    .button("Paste")
                    .on_disabled_hover_text("Nothing in clipboard")
                    .clicked()
                {
                    request_paste = true;
                    ui.close();
                }
                ui.separator();
                if ui.button("Add Child (Empty)").clicked() {
                    request_add_child = true;
                    ui.close();
                }
                ui.separator();
                if ui.button("Save as Prefab...").clicked() {
                    request_save_prefab = true;
                    ui.close();
                }
            });
        }

        // Apply per-row requests.
        if request_rename {
            self.renaming_entity_id = Some(id);
            self.rename_buffer = name.clone();
        }
        if commit_rename {
            if !self.rename_buffer.trim().is_empty() {
                let new_name = self.rename_buffer.trim().to_string();
                self.command_history.execute(
                    crate::app::scene_editor::history::SceneCommand::RenameEntity {
                        entity_id: id,
                        old_name: name.clone(),
                        new_name: new_name.clone(),
                    },
                    &self.scene_model,
                );
                if let Some(entity) = self.scene_model.entities.get_mut(&id) {
                    entity.name = new_name;
                    self.scene_model.modified = true;
                }
            }
            self.renaming_entity_id = None;
            self.rename_buffer.clear();
        }
        if cancel_rename {
            self.renaming_entity_id = None;
            self.rename_buffer.clear();
        }
        if request_duplicate {
            // Record a specific DuplicateEntity command (or Batch for multi-select).
            let ids_to_dup: Vec<u64> = if self.scene_model.selected_ids.len() > 1 {
                self.scene_model.selected_ids.iter().copied().collect()
            } else {
                vec![id]
            };
            {
                use crate::app::scene_editor::history::SceneCommand;
                if ids_to_dup.len() == 1 {
                    self.command_history.execute(
                        SceneCommand::DuplicateEntity {
                            source_id: ids_to_dup[0],
                            new_id: 0,
                        },
                        &self.scene_model,
                    );
                } else {
                    let cmds: Vec<SceneCommand> = ids_to_dup
                        .iter()
                        .map(|&sid| SceneCommand::DuplicateEntity {
                            source_id: sid,
                            new_id: 0,
                        })
                        .collect();
                    self.command_history
                        .execute(SceneCommand::Batch(cmds), &self.scene_model);
                }
            }
            let mut last_new = None;
            self.scene_model.select_clear();
            for dup_id in ids_to_dup {
                if let Some(new_id) = self.scene_model.duplicate_entity(dup_id) {
                    self.scene_model.select_add(new_id);
                    last_new = Some(new_id);
                }
            }
            self.primary_selected_id = last_new;
            self.scene_needs_sync = true;
        }
        if request_delete {
            // Delete all selected entities if the right-clicked entity is in the selection.
            let ids_to_del: Vec<u64> =
                if self.scene_model.is_selected(id) && self.scene_model.selected_ids.len() > 1 {
                    self.scene_model.selected_ids.iter().copied().collect()
                } else {
                    vec![id]
                };
            {
                use crate::app::scene_editor::history::SceneCommand;
                if ids_to_del.len() == 1 {
                    self.command_history.execute(
                        SceneCommand::RemoveEntity {
                            entity_id: ids_to_del[0],
                        },
                        &self.scene_model,
                    );
                } else {
                    let cmds: Vec<SceneCommand> = ids_to_del
                        .iter()
                        .map(|&eid| SceneCommand::RemoveEntity { entity_id: eid })
                        .collect();
                    self.command_history
                        .execute(SceneCommand::Batch(cmds), &self.scene_model);
                }
            }
            for del_id in ids_to_del {
                self.scene_model.remove_entity(del_id);
            }
            self.scene_model.select_clear();
            self.primary_selected_id = None;
            self.scene_needs_sync = true;
            return; // The entity no longer exists; don't recurse.
        }
        if request_add_child {
            self.command_history.execute(
                crate::app::scene_editor::history::SceneCommand::AddEntity {
                    entity_id: 0,
                    name: "Empty".into(),
                },
                &self.scene_model,
            );
            let new_id = self.scene_model.add_entity("Empty".into(), vec![]);
            self.scene_model.set_parent(new_id, Some(id));
            self.scene_model.select_only(new_id);
            self.primary_selected_id = Some(new_id);
            self.scene_needs_sync = true;
        }
        if request_toggle_enabled {
            self.scene_snapshot();
            if let Some(entity) = self.scene_model.entities.get_mut(&id) {
                entity.enabled = !entity.enabled;
                self.scene_model.modified = true;
            }
            self.scene_needs_sync = true;
        }
        if request_save_prefab {
            if let Some(prefab) =
                crate::app::scene_editor::prefab::build_prefab_from_entity(&self.scene_model, id)
            {
                // Save under <root>/prefabs/<entity_name>.bprefab
                let dir = format!("{}/prefabs", self.root_path);
                let _ = std::fs::create_dir_all(&dir);
                let safe_name: String = name
                    .chars()
                    .map(|c| {
                        if c.is_alphanumeric() || c == '_' || c == '-' {
                            c
                        } else {
                            '_'
                        }
                    })
                    .collect();
                let path = format!("{}/{}.bprefab", dir, safe_name);
                match crate::app::scene_editor::prefab::save_prefab(&prefab, &path) {
                    Ok(_) => {
                        self.status_message = format!("Saved prefab: {}", path);
                        self.status_message_timestamp = Some(std::time::Instant::now());
                        self.file_tree_cache.clear();
                        self.file_tree_load_pending = true;
                    }
                    Err(e) => {
                        self.status_message = format!("Failed to save prefab: {}", e);
                        self.status_message_timestamp = Some(std::time::Instant::now());
                    }
                }
            }
        }

        if request_copy {
            self.entity_clipboard =
                crate::app::scene_editor::prefab::build_prefab_from_entity(&self.scene_model, id);
            if self.entity_clipboard.is_some() {
                self.status_message = format!("Copied entity: {}", name);
                self.status_message_timestamp = Some(std::time::Instant::now());
            }
        }
        if request_paste {
            if let Some(ref clipboard) = self.entity_clipboard.clone() {
                self.scene_snapshot();
                let new_root = crate::app::scene_editor::prefab::instantiate_prefab(
                    &mut self.scene_model,
                    clipboard,
                );
                // Parent the pasted entity under the right-clicked entity.
                self.scene_model.set_parent(new_root, Some(id));
                self.scene_model.select_only(new_root);
                self.primary_selected_id = Some(new_root);
                self.scene_needs_sync = true;
                self.status_message = "Pasted entity from clipboard".to_string();
                self.status_message_timestamp = Some(std::time::Instant::now());
            }
        }

        for child_id in children {
            self.render_entity_tree_node(ui, child_id, depth + 1, visible);
        }
    }

    /// Recursively collect 3D model files (.glb, .gltf, .obj, .stl, .ply) under `dir`.
    fn collect_model_assets(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = path
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();
                if name == "target" || name.starts_with('.') {
                    continue;
                }
                Self::collect_model_assets(&path, out);
            } else if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                match ext.to_lowercase().as_str() {
                    "glb" | "gltf" | "obj" | "stl" | "ply" => out.push(path),
                    _ => {}
                }
            }
        }
    }

    /// Recursively collect .bprefab files under `dir`.
    fn collect_prefab_assets(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = path
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();
                if name == "target" || name.starts_with('.') {
                    continue;
                }
                Self::collect_prefab_assets(&path, out);
            } else if path.extension().and_then(|s| s.to_str()) == Some("bprefab") {
                out.push(path);
            }
        }
    }

    /// Instantiate a prefab from a .bprefab file path into the scene.
    pub(crate) fn instantiate_prefab_from_path(&mut self, path: &str) {
        match std::fs::read_to_string(path) {
            Ok(content) => {
                if let Ok(prefab) =
                    serde_json::from_str::<crate::app::scene_editor::prefab::PrefabFile>(&content)
                {
                    let new_root = crate::app::scene_editor::prefab::instantiate_prefab(
                        &mut self.scene_model,
                        &prefab,
                    );
                    self.scene_model.select_only(new_root);
                    self.primary_selected_id = Some(new_root);
                } else {
                    tracing::warn!("Failed to parse prefab: {}", path);
                }
            }
            Err(e) => {
                tracing::warn!("Failed to read prefab {}: {}", path, e);
            }
        }
    }
}

/// Resolve the (label, absolute path) for a brand-new scene file.
/// Sanitised name takes precedence; an empty input falls back to
/// `Untitled N`. Already-existing files are disambiguated with a
/// `-2`, `-3` … suffix. Always creates `<root>/scenes/` if missing
/// so the caller can immediately write into it.
pub fn resolve_new_scene_path(root_path: &str, sanitised_name: &str) -> (String, String) {
    let scenes_dir = format!("{}/scenes", root_path);
    let _ = std::fs::create_dir_all(&scenes_dir);
    if !sanitised_name.is_empty() {
        let path = format!("{}/{}.bscene", scenes_dir, sanitised_name);
        if !std::path::Path::new(&path).exists() {
            return (sanitised_name.to_string(), path);
        }
        let mut n = 2;
        loop {
            let alt = format!("{}-{}", sanitised_name, n);
            let alt_path = format!("{}/{}.bscene", scenes_dir, alt);
            if !std::path::Path::new(&alt_path).exists() {
                return (alt, alt_path);
            }
            n += 1;
        }
    }
    let mut n = 1;
    loop {
        let candidate = format!("Untitled {}", n);
        let path = format!("{}/{}.bscene", scenes_dir, candidate);
        if !std::path::Path::new(&path).exists() {
            return (candidate, path);
        }
        n += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn collect_model_assets_finds_glb_and_obj() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();

        // Create some model files
        fs::write(dir.join("hero.glb"), b"").unwrap();
        fs::write(dir.join("world.gltf"), b"").unwrap();
        fs::write(dir.join("rock.obj"), b"").unwrap();
        fs::write(dir.join("notes.txt"), b"").unwrap();
        fs::write(dir.join("icon.png"), b"").unwrap();

        let mut out = Vec::new();
        BerryCodeApp::collect_model_assets(dir, &mut out);

        let names: Vec<String> = out
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert!(names.contains(&"hero.glb".to_string()));
        assert!(names.contains(&"world.gltf".to_string()));
        assert!(names.contains(&"rock.obj".to_string()));
        assert!(!names.contains(&"notes.txt".to_string()));
        assert!(!names.contains(&"icon.png".to_string()));
        assert_eq!(out.len(), 3);
    }

    #[test]
    fn collect_model_assets_recurses_subdirs() {
        let tmp = tempfile::tempdir().unwrap();
        let sub = tmp.path().join("models");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join("tank.glb"), b"").unwrap();
        fs::write(tmp.path().join("base.stl"), b"").unwrap();

        let mut out = Vec::new();
        BerryCodeApp::collect_model_assets(tmp.path(), &mut out);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn collect_model_assets_skips_target_and_hidden() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("target");
        let hidden = tmp.path().join(".cache");
        fs::create_dir_all(&target).unwrap();
        fs::create_dir_all(&hidden).unwrap();
        fs::write(target.join("build.glb"), b"").unwrap();
        fs::write(hidden.join("tmp.obj"), b"").unwrap();
        fs::write(tmp.path().join("valid.ply"), b"").unwrap();

        let mut out = Vec::new();
        BerryCodeApp::collect_model_assets(tmp.path(), &mut out);
        assert_eq!(out.len(), 1);
        assert!(out[0]
            .file_name()
            .unwrap()
            .to_string_lossy()
            .contains("valid"));
    }

    #[test]
    fn collect_prefab_assets_finds_bprefab() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("enemy.bprefab"), b"{}").unwrap();
        fs::write(tmp.path().join("scene.bscene"), b"").unwrap();
        fs::write(tmp.path().join("tree.bprefab"), b"{}").unwrap();

        let mut out = Vec::new();
        BerryCodeApp::collect_prefab_assets(tmp.path(), &mut out);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn collect_model_assets_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();

        let mut out = Vec::new();
        BerryCodeApp::collect_model_assets(tmp.path(), &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn collect_model_assets_nonexistent_dir() {
        let mut out = Vec::new();
        BerryCodeApp::collect_model_assets(std::path::Path::new("/nonexistent/path"), &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn resolve_new_scene_path_uses_typed_name() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_string_lossy().to_string();
        let (label, path) = resolve_new_scene_path(&root, "scene2");
        assert_eq!(label, "scene2");
        assert!(path.ends_with("/scenes/scene2.bscene"), "got {}", path);
        // The function creates the parent dir but does not write the
        // file itself — caller is responsible for the `.bscene` body.
        assert!(std::path::Path::new(&format!("{}/scenes", root)).is_dir());
    }

    #[test]
    fn resolve_new_scene_path_falls_back_to_untitled_on_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_string_lossy().to_string();
        let (label, path) = resolve_new_scene_path(&root, "");
        assert_eq!(label, "Untitled 1");
        assert!(path.ends_with("/scenes/Untitled 1.bscene"));
    }

    #[test]
    fn resolve_new_scene_path_disambiguates_existing_file() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_string_lossy().to_string();
        let scenes_dir = format!("{}/scenes", root);
        std::fs::create_dir_all(&scenes_dir).unwrap();
        // Pretend "scene2" already exists.
        std::fs::write(format!("{}/scene2.bscene", scenes_dir), "").unwrap();

        let (label, path) = resolve_new_scene_path(&root, "scene2");
        assert_eq!(label, "scene2-2");
        assert!(path.ends_with("/scenes/scene2-2.bscene"), "got {}", path);
        assert!(!std::path::Path::new(&path).exists(), "must not exist yet");
    }

    /// End-to-end: simulate the click-flow path of the New dialog
    /// (resolve → write `.bscene`) and verify the file lands on disk.
    /// Regression test for the "scene2 を作っても scene2.bscene が
    /// できない" bug.
    #[test]
    fn new_scene_flow_writes_bscene_to_disk() {
        use crate::app::scene_editor::model::SceneModel;
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_string_lossy().to_string();

        let (label, path) = resolve_new_scene_path(&root, "scene2");
        assert_eq!(label, "scene2");

        let mut model = SceneModel::new();
        model.file_path = Some(path.clone());
        crate::app::scene_editor::serialization::save_scene_to_ron(&model, &path).expect("write");

        assert!(
            std::path::Path::new(&path).exists(),
            "{} should exist after New flow",
            path
        );
        let body = std::fs::read_to_string(&path).unwrap();
        assert!(!body.is_empty(), "should have ron payload");
    }
}
