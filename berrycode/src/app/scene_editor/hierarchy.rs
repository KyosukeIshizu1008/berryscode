//! Hierarchy panel: tree view of scene entities with creation toolbar,
//! drag-and-drop reparenting, right-click context menu, and inline rename.

use super::model::*;
use crate::app::BerryCodeApp;

impl BerryCodeApp {
    /// Render the Hierarchy panel showing scene entities.
    pub(crate) fn render_scene_hierarchy(&mut self, ui: &mut egui::Ui) {
        ui.heading("Hierarchy");
        ui.separator();

        // Play mode banner (Phase 15).
        if self.play_mode.is_active() {
            ui.colored_label(
                egui::Color32::from_rgb(255, 200, 80),
                "Play Mode Active - editing disabled",
            );
            ui.separator();
        }

        // --- Scene tabs (Phase 8) ---
        let tab_count = self.scene_tabs.len();
        let mut switch_to: Option<usize> = None;
        if tab_count > 0 {
            ui.horizontal_wrapped(|ui| {
                for i in 0..tab_count {
                    let selected = i == self.active_scene_tab;
                    let label = if self.scene_tabs[i].model.modified {
                        format!("{}*", self.scene_tabs[i].label)
                    } else {
                        self.scene_tabs[i].label.clone()
                    };
                    if ui.selectable_label(selected, &label).clicked() && i != self.active_scene_tab
                    {
                        switch_to = Some(i);
                    }
                }
            });
            ui.separator();
        }
        if let Some(new_idx) = switch_to {
            self.scene_tabs[self.active_scene_tab].model = self.scene_model.clone();
            self.active_scene_tab = new_idx;
            self.scene_model = self.scene_tabs[new_idx].model.clone();
            self.scene_needs_sync = true;
            self.primary_selected_id = None;
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

        let row_response = ui
            .horizontal(|ui| {
                ui.add_space((depth as f32) * 16.0);

                // Visibility toggle (eye icon).
                let eye_label = if entity_enabled { "V" } else { "-" };
                let eye_color = if entity_enabled {
                    egui::Color32::from_gray(200)
                } else {
                    egui::Color32::from_gray(80)
                };
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new(eye_label).size(10.0).color(eye_color),
                        )
                        .frame(false)
                        .min_size(egui::vec2(14.0, 14.0)),
                    )
                    .clicked()
                {
                    request_toggle_enabled = true;
                }

                let prefix = if children.is_empty() { "• " } else { "▸ " };

                if is_renaming {
                    let edit = ui.add(
                        egui::TextEdit::singleline(&mut self.rename_buffer).desired_width(160.0),
                    );
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
                    // Build a placeholder response so we still get drag/context
                    // semantics on the row label area.
                    ui.allocate_response(egui::Vec2::ZERO, egui::Sense::hover())
                } else {
                    let label = format!("{}{}", prefix, name);
                    let label_color = if !entity_enabled {
                        egui::Color32::from_gray(90)
                    } else if selected {
                        egui::Color32::from_rgb(255, 255, 255)
                    } else {
                        egui::Color32::from_rgb(212, 212, 212)
                    };
                    let resp = ui.add(
                        egui::Label::new(egui::RichText::new(label).color(label_color))
                            .sense(egui::Sense::click_and_drag()),
                    );

                    // Selection background highlight.
                    if selected {
                        ui.painter().rect_filled(
                            resp.rect.expand(2.0),
                            2.0,
                            egui::Color32::from_rgba_premultiplied(80, 130, 255, 50),
                        );
                    }

                    if resp.clicked() {
                        let modifiers = ui.input(|i| i.modifiers);
                        if modifiers.shift {
                            // Shift+Click: add to selection.
                            self.scene_model.select_add(id);
                        } else if modifiers.command {
                            // Cmd/Ctrl+Click: toggle in selection.
                            self.scene_model.select_toggle(id);
                        } else {
                            // Plain click: replace selection.
                            self.scene_model.select_only(id);
                        }
                        self.primary_selected_id = Some(id);
                    }
                    if resp.double_clicked() {
                        request_rename = true;
                    }

                    // Drag-and-drop source.
                    if resp.drag_started() {
                        self.hierarchy_dragged = Some(id);
                        self.scene_model.select_only(id);
                        self.primary_selected_id = Some(id);
                    }

                    // Drop target detection: if a drag is in progress and the
                    // pointer is hovering this row's rect, mark this entity as
                    // the drop target.
                    if self.hierarchy_dragged.is_some()
                        && self.hierarchy_dragged != Some(id)
                        && ui.input(|i| i.pointer.any_down())
                    {
                        if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
                            if resp.rect.contains(pos) {
                                self.hierarchy_drop_target = Some(Some(id));
                                ui.painter().rect_stroke(
                                    resp.rect,
                                    2.0,
                                    egui::Stroke::new(1.5, egui::Color32::from_rgb(80, 200, 255)),
                                );
                            }
                        }
                    }

                    resp.context_menu(|ui| {
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

                    resp
                }
            })
            .inner;
        let _ = row_response;

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
}
