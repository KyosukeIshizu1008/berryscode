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
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(header_rect), |ui| {
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

        // --- Scene tabs + project scenes dropdown ---
        let tab_count = self.scene_tabs.len();
        let mut switch_to: Option<usize> = None;
        let mut load_scene_path: Option<String> = None;

        ui.horizontal_wrapped(|ui| {
            // Existing scene tabs
            for i in 0..tab_count {
                let selected = i == self.active_scene_tab;
                let label = if self.scene_tabs[i].model.modified {
                    format!("{}*", self.scene_tabs[i].label)
                } else {
                    self.scene_tabs[i].label.clone()
                };
                if ui.selectable_label(selected, &label).clicked() && i != self.active_scene_tab {
                    switch_to = Some(i);
                }
            }

            // "Open Scene" dropdown - lists all .bscene files in project
            ui.menu_button("Open Scene...", |ui| {
                let scenes =
                    crate::app::scene_editor::build_settings::scan_scene_files(&self.root_path);
                if scenes.is_empty() {
                    ui.label(
                        egui::RichText::new("No .bscene files found")
                            .color(egui::Color32::from_gray(120))
                            .italics(),
                    );
                    ui.label(
                        egui::RichText::new("Save a scene first")
                            .color(egui::Color32::from_gray(100))
                            .size(10.0),
                    );
                } else {
                    for scene_rel in &scenes {
                        let file_name = std::path::Path::new(scene_rel)
                            .file_stem()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_else(|| scene_rel.clone());
                        // Check if already open in a tab
                        let full_path = format!("{}/{}", self.root_path, scene_rel);
                        let already_open = self.scene_tabs.iter().enumerate().find(|(_, t)| {
                            t.model.file_path.as_deref() == Some(full_path.as_str())
                        });
                        if let Some((idx, _)) = already_open {
                            if ui
                                .add(
                                    egui::Label::new(
                                        egui::RichText::new(format!("{} (open)", file_name))
                                            .color(egui::Color32::from_gray(150)),
                                    )
                                    .sense(egui::Sense::click()),
                                )
                                .clicked()
                            {
                                switch_to = Some(idx);
                                ui.close_menu();
                            }
                        } else if ui.button(&file_name).clicked() {
                            load_scene_path = Some(full_path);
                            ui.close_menu();
                        }
                    }
                }
            });
        });
        if tab_count > 0 || load_scene_path.is_some() {
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
        if let Some(path) = load_scene_path {
            self.load_scene(&path);
        }

        // Compact toolbar: essential buttons + dropdown menus
        ui.horizontal(|ui| {
            if ui.small_button("New").clicked() {
                self.scene_snapshot();
                self.scene_model = SceneModel::new();
                self.scene_needs_sync = true;
            }
            if ui.small_button("Save").clicked() {
                self.save_current_scene();
            }
            // Tools dropdown
            ui.menu_button("Tools", |ui| {
                if ui.button("Profiler").clicked() {
                    self.profiler.open = true;
                    self.tool_panel_open = true;
                    self.active_tool_tab = crate::app::dock::ToolTab::Profiler;
                    ui.close_menu();
                }
                if ui.button("Timeline").clicked() {
                    self.tool_panel_open = true;
                    self.active_tool_tab = crate::app::dock::ToolTab::Timeline;
                    ui.close_menu();
                }
                if ui.button("Dopesheet").clicked() {
                    self.tool_panel_open = true;
                    self.active_tool_tab = crate::app::dock::ToolTab::Dopesheet;
                    ui.close_menu();
                }
                if ui.button("Systems").clicked() {
                    self.system_graph_open = !self.system_graph_open;
                    ui.close_menu();
                }
                if ui.button("Events").clicked() {
                    self.event_monitor_open = !self.event_monitor_open;
                    ui.close_menu();
                }
                if ui.button("Queries").clicked() {
                    self.query_viz_open = !self.query_viz_open;
                    ui.close_menu();
                }
                if ui.button("States").clicked() {
                    self.state_editor_open = !self.state_editor_open;
                    ui.close_menu();
                }
                if ui.button("Plugins").clicked() {
                    self.plugin_browser_open = !self.plugin_browser_open;
                    ui.close_menu();
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
                    ui.close_menu();
                }
            });
            if let Some(path) = &self.scene_model.file_path {
                let file_name = std::path::Path::new(path)
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.clone());
                let dirty_marker = if self.scene_model.modified { "*" } else { "" };
                ui.label(
                    egui::RichText::new(format!("{}{}", file_name, dirty_marker))
                        .italics()
                        .color(egui::Color32::from_gray(160)),
                );
            } else if self.scene_model.modified {
                ui.label(
                    egui::RichText::new("(unsaved)")
                        .italics()
                        .color(egui::Color32::from_gray(160)),
                );
            }
        });

        ui.separator();

        // Creation toolbar (dropdown to save space)
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
                                ui.close_menu();
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
                                ui.close_menu();
                            }
                        }
                    }
                }
            });
        });

        ui.separator();

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
            });

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
                    ui.close_menu();
                }
                if ui.button("Duplicate").clicked() {
                    request_duplicate = true;
                    ui.close_menu();
                }
                if ui.button("Delete").clicked() {
                    request_delete = true;
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Copy").clicked() {
                    request_copy = true;
                    ui.close_menu();
                }
                if ui
                    .button("Paste")
                    .on_disabled_hover_text("Nothing in clipboard")
                    .clicked()
                {
                    request_paste = true;
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Add Child (Empty)").clicked() {
                    request_add_child = true;
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Save as Prefab...").clicked() {
                    request_save_prefab = true;
                    ui.close_menu();
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
}
