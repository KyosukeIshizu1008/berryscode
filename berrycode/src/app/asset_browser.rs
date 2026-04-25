use super::scene_editor::asset_import::AssetImportSettings;
use super::BerryCodeApp;
use crate::app::i18n::t;
use crate::bevy_ide::assets::scanner::{format_size, scan_assets, AssetType, AssetViewMode};

/// Snapshot of one asset for rendering, avoiding borrow conflicts.
struct AssetRow {
    idx: usize,
    file_name: String,
    path_str: String,
    relative_path: String,
    extension: String,
    asset_type: AssetType,
    size_bytes: u64,
    is_scene_asset: bool,
}

impl BerryCodeApp {
    pub(crate) fn render_asset_browser_panel(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        // Scan assets on first render or when requested
        if self.asset_browser.scan_pending {
            self.asset_browser.assets =
                scan_assets(&self.root_path, &self.asset_browser.asset_root);
            self.asset_browser.scan_pending = false;
        }

        ui.heading(self.tr("Asset Browser"));
        ui.separator();

        // Asset root directory
        ui.horizontal(|ui| {
            ui.label(self.tr("Root:"));
            if ui
                .text_edit_singleline(&mut self.asset_browser.asset_root)
                .changed()
            {
                self.asset_browser.scan_pending = true;
            }
            if ui.button("\u{21BB}").clicked() {
                self.asset_browser.scan_pending = true;
            }
        });

        // Filter bar
        ui.horizontal(|ui| {
            ui.label(self.tr("Filter:"));
            ui.text_edit_singleline(&mut self.asset_browser.filter_query);
        });

        // Type filter buttons
        ui.horizontal_wrapped(|ui| {
            let types: [(&str, Option<AssetType>); 6] = [
                (self.tr("All"), None),
                (self.tr("Images"), Some(AssetType::Image)),
                (self.tr("Models"), Some(AssetType::Model3D)),
                (self.tr("Audio"), Some(AssetType::Audio)),
                (self.tr("Scenes"), Some(AssetType::Scene)),
                (self.tr("Shaders"), Some(AssetType::Shader)),
            ];
            for (label, filter_type) in &types {
                let selected = self.asset_browser.filter_type == *filter_type;
                if ui.selectable_label(selected, *label).clicked() {
                    self.asset_browser.filter_type = filter_type.clone();
                }
            }
        });

        // View mode toggle
        ui.horizontal(|ui| {
            if ui
                .selectable_label(self.asset_browser.view_mode == AssetViewMode::List, "List")
                .clicked()
            {
                self.asset_browser.view_mode = AssetViewMode::List;
            }
            if ui
                .selectable_label(self.asset_browser.view_mode == AssetViewMode::Grid, "Grid")
                .clicked()
            {
                self.asset_browser.view_mode = AssetViewMode::Grid;
            }
            ui.label(format!("{} assets", self.asset_browser.assets.len()));
        });

        ui.separator();

        // Pre-collect filtered asset data into owned structs to avoid borrow conflicts.
        let filter_query = self.asset_browser.filter_query.to_lowercase();
        let filter_type = self.asset_browser.filter_type.clone();
        let rows: Vec<AssetRow> = self
            .asset_browser
            .assets
            .iter()
            .enumerate()
            .filter(|(_, asset)| {
                let name_match = filter_query.is_empty()
                    || asset.file_name.to_lowercase().contains(&filter_query)
                    || asset.relative_path.to_lowercase().contains(&filter_query);
                let type_match =
                    filter_type.is_none() || filter_type.as_ref() == Some(&asset.asset_type);
                name_match && type_match
            })
            .map(|(idx, asset)| {
                let ext = asset.extension.to_lowercase();
                AssetRow {
                    idx,
                    file_name: asset.file_name.clone(),
                    path_str: asset.path.to_string_lossy().to_string(),
                    relative_path: asset.relative_path.clone(),
                    extension: asset.extension.clone(),
                    asset_type: asset.asset_type.clone(),
                    size_bytes: asset.size_bytes,
                    is_scene_asset: matches!(
                        ext.as_str(),
                        "glb" | "gltf" | "obj" | "stl" | "ply" | "bprefab"
                    ),
                }
            })
            .collect();
        let no_assets = self.asset_browser.assets.is_empty();

        // Deferred actions
        let mut clicked_idx: Option<usize> = None;
        let mut double_clicked_path: Option<(String, AssetType)> = None;
        let mut drag_started_path: Option<String> = None;
        let mut add_to_scene_path: Option<String> = None;
        let mut open_path: Option<String> = None;

        egui::ScrollArea::vertical().show(ui, |ui| {
            for row in &rows {
                let selected = self.asset_browser.selected_asset == Some(row.idx);

                let sense = if row.is_scene_asset {
                    egui::Sense::click_and_drag()
                } else {
                    egui::Sense::click()
                };

                let response = ui.horizontal(|ui| {
                    // Show thumbnail for image assets, fallback to text icon
                    if row.asset_type == AssetType::Image {
                        if let Some(tex) = self.thumbnail_cache.get_or_create(ctx, &row.path_str) {
                            let size = egui::vec2(16.0, 16.0);
                            ui.image((tex.id(), size));
                        } else {
                            ui.label(row.asset_type.icon());
                        }
                    } else {
                        ui.label(row.asset_type.icon());
                    }
                    let resp = ui.add(
                        egui::Label::new(egui::RichText::new(&row.file_name).color(if selected {
                            egui::Color32::WHITE
                        } else {
                            egui::Color32::from_rgb(212, 212, 212)
                        }))
                        .sense(sense),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(format_size(row.size_bytes));
                        ui.colored_label(egui::Color32::GRAY, row.asset_type.label());
                    });
                    resp
                });

                if response.inner.clicked() {
                    clicked_idx = Some(row.idx);
                }

                if response.inner.double_clicked() {
                    double_clicked_path = Some((row.path_str.clone(), row.asset_type.clone()));
                }

                // Drag-and-drop: start dragging for 3D assets
                if row.is_scene_asset && response.inner.drag_started() {
                    drag_started_path = Some(row.path_str.clone());
                }

                // Right-click context menu on the whole row
                let ctx_path = row.path_str.clone();
                let ctx_is_scene = row.is_scene_asset;
                response.response.context_menu(|ui| {
                    if ui.button("Open").clicked() {
                        open_path = Some(ctx_path.clone());
                        ui.close_menu();
                    }
                    if ctx_is_scene {
                        if ui.button("Add to Scene").clicked() {
                            add_to_scene_path = Some(ctx_path.clone());
                            ui.close_menu();
                        }
                    }
                });
            }

            if rows.is_empty() {
                if no_assets {
                    let assets_dir = std::path::Path::new(&self.root_path).join("assets");
                    if !assets_dir.exists() {
                        ui.add_space(20.0);
                        ui.vertical_centered(|ui| {
                            ui.label(
                                egui::RichText::new("No 'assets/' folder found")
                                    .color(egui::Color32::from_rgb(180, 180, 180))
                                    .size(13.0),
                            );
                            ui.add_space(8.0);
                            if ui.button("Create assets/ folder").clicked() {
                                if let Err(e) = std::fs::create_dir_all(&assets_dir) {
                                    tracing::warn!("Failed to create assets dir: {}", e);
                                } else {
                                    self.asset_browser.scan_pending = true;
                                    self.file_tree_cache.clear();
                                    self.file_tree_load_pending = true;
                                }
                            }
                        });
                    } else {
                        ui.label(t(self.ui_language, "No assets found in assets/ folder."));
                    }
                } else {
                    ui.label(t(self.ui_language, "No assets match the current filter."));
                }
            }
        });

        // Apply deferred actions
        if let Some(idx) = clicked_idx {
            self.asset_browser.selected_asset = Some(idx);
        }

        if let Some((path, asset_type)) = double_clicked_path {
            match asset_type {
                AssetType::Scene | AssetType::Shader | AssetType::Data => {
                    self.open_file_from_path(&path);
                }
                _ => {}
            }
        }

        if let Some(path) = drag_started_path {
            self.dragged_asset_path = Some(path);
        }

        if let Some(path) = open_path {
            self.open_file_from_path(&path);
        }

        if let Some(path) = add_to_scene_path {
            let entity_name = std::path::Path::new(&path)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "Asset".to_string());
            if path.ends_with(".bprefab") {
                self.scene_snapshot();
                self.instantiate_prefab_from_path(&path);
                self.scene_needs_sync = true;
            } else {
                self.scene_snapshot();
                let new_id = self.scene_model.add_entity(
                    entity_name,
                    vec![
                        crate::app::scene_editor::model::ComponentData::MeshFromFile {
                            path,
                            texture_path: None,
                            normal_map_path: None,
                        },
                    ],
                );
                self.scene_model.select_only(new_id);
                self.primary_selected_id = Some(new_id);
                self.scene_needs_sync = true;
            }
        }

        // Selected asset details + import settings
        if let Some(idx) = self.asset_browser.selected_asset {
            if let Some(asset) = self.asset_browser.assets.get(idx) {
                ui.separator();
                ui.label(format!("Path: {}", asset.relative_path));
                ui.label(format!(
                    "Type: {} (.{})",
                    asset.asset_type.label(),
                    asset.extension
                ));
                ui.label(format!("Size: {}", format_size(asset.size_bytes)));

                // Import settings
                let asset_path_str = asset.path.to_string_lossy().to_string();
                let mut settings = AssetImportSettings::load(&asset_path_str);
                let mut changed = false;

                match &mut settings {
                    AssetImportSettings::Texture {
                        max_size,
                        generate_mipmaps,
                        filter_mode,
                    } => {
                        ui.separator();
                        ui.strong("Import Settings (Texture)");
                        if ui
                            .add(
                                egui::DragValue::new(max_size)
                                    .prefix("Max Size: ")
                                    .range(64..=8192),
                            )
                            .changed()
                        {
                            changed = true;
                        }
                        if ui.checkbox(generate_mipmaps, "Generate Mipmaps").changed() {
                            changed = true;
                        }
                        egui::ComboBox::from_label("Filter Mode")
                            .selected_text(filter_mode.as_str())
                            .show_ui(ui, |ui| {
                                for mode in &["Nearest", "Linear"] {
                                    if ui
                                        .selectable_value(filter_mode, mode.to_string(), *mode)
                                        .changed()
                                    {
                                        changed = true;
                                    }
                                }
                            });
                    }
                    AssetImportSettings::Model {
                        scale_factor,
                        flip_uvs,
                        import_materials,
                    } => {
                        ui.separator();
                        ui.strong("Import Settings (Model)");
                        if ui
                            .add(
                                egui::DragValue::new(scale_factor)
                                    .prefix("Scale: ")
                                    .speed(0.01)
                                    .range(0.001..=1000.0),
                            )
                            .changed()
                        {
                            changed = true;
                        }
                        if ui.checkbox(flip_uvs, "Flip UVs").changed() {
                            changed = true;
                        }
                        if ui.checkbox(import_materials, "Import Materials").changed() {
                            changed = true;
                        }
                    }
                    AssetImportSettings::Audio {
                        sample_rate,
                        force_mono,
                    } => {
                        ui.separator();
                        ui.strong("Import Settings (Audio)");
                        egui::ComboBox::from_label("Sample Rate")
                            .selected_text(format!("{} Hz", sample_rate))
                            .show_ui(ui, |ui| {
                                for rate in &[22050u32, 44100, 48000, 96000] {
                                    if ui
                                        .selectable_value(
                                            sample_rate,
                                            *rate,
                                            format!("{} Hz", rate),
                                        )
                                        .changed()
                                    {
                                        changed = true;
                                    }
                                }
                            });
                        if ui.checkbox(force_mono, "Force Mono").changed() {
                            changed = true;
                        }
                    }
                    AssetImportSettings::Unknown => {}
                }

                // Process button (applies import settings to the asset)
                if !matches!(settings, AssetImportSettings::Unknown) {
                    if ui.button(self.tr("Process")).clicked() {
                        match settings.process(&asset_path_str) {
                            Ok(msg) => {
                                tracing::info!("Asset processed: {}", msg);
                                self.asset_browser.scan_pending = true;
                            }
                            Err(e) => {
                                tracing::warn!("Asset processing failed: {}", e);
                            }
                        }
                    }
                }

                if changed {
                    if let Err(e) = settings.save(&asset_path_str) {
                        tracing::warn!("Failed to save import settings: {}", e);
                    }
                }
            }
        }
    }

    /// Render asset preview in the central panel (3D wireframe for models).
    pub(crate) fn render_asset_preview(&mut self, ui: &mut egui::Ui) {
        // Get selected asset path
        let selected_path = self
            .asset_browser
            .selected_asset
            .and_then(|idx| self.asset_browser.assets.get(idx))
            .map(|a| a.path.to_string_lossy().to_string());

        let Some(path) = selected_path else {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("Select an asset to preview")
                        .color(egui::Color32::from_gray(100))
                        .size(14.0),
                );
            });
            return;
        };

        let ext = path.rsplit('.').next().unwrap_or("").to_lowercase();
        let is_model = matches!(ext.as_str(), "glb" | "gltf" | "obj" | "stl" | "ply");

        if !is_model {
            // Non-model: show basic info
            let file_name = std::path::Path::new(&path)
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            ui.heading(&file_name);
            ui.separator();
            ui.label(format!("Type: {}", ext.to_uppercase()));
            if let Ok(meta) = std::fs::metadata(&path) {
                let size = meta.len();
                if size < 1024 * 1024 {
                    ui.label(format!("Size: {:.1} KB", size as f64 / 1024.0));
                } else {
                    ui.label(format!("Size: {:.1} MB", size as f64 / (1024.0 * 1024.0)));
                }
            }
            return;
        }

        // Load/cache model data
        if self.asset_preview_path != path {
            self.asset_preview_data = Self::load_model_data(&path);
            self.asset_preview_path = path.clone();
            self.asset_preview_rot_x = 0.3;
            self.asset_preview_rot_y = std::f32::consts::FRAC_PI_4;
            self.asset_preview_zoom = 1.0;
        }

        let file_name = std::path::Path::new(&path)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        // Header with file info + Add to Scene button
        ui.horizontal(|ui| {
            ui.heading(&file_name);
            ui.separator();
            if let Ok(meta) = std::fs::metadata(&path) {
                let size = meta.len();
                if size < 1024 * 1024 {
                    ui.label(format!("{:.1} KB", size as f64 / 1024.0));
                } else {
                    ui.label(format!("{:.1} MB", size as f64 / (1024.0 * 1024.0)));
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("+ Add to Scene")
                                .color(egui::Color32::from_rgb(220, 220, 220)),
                        )
                        .fill(egui::Color32::from_rgb(0, 100, 180)),
                    )
                    .clicked()
                {
                    let entity_name = std::path::Path::new(&path)
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| "Model".to_string());
                    self.scene_snapshot();
                    let new_id = self.scene_model.add_entity(
                        entity_name,
                        vec![
                            crate::app::scene_editor::model::ComponentData::MeshFromFile {
                                path: path.clone(),
                                texture_path: None,
                                normal_map_path: None,
                            },
                        ],
                    );
                    self.scene_model.select_only(new_id);
                    self.primary_selected_id = Some(new_id);
                    self.scene_needs_sync = true;
                    self.status_message = format!("Added {} to scene", file_name);
                    self.status_message_timestamp = Some(std::time::Instant::now());
                }
            });
        });
        ui.separator();

        // Stats
        if let Some(data) = &self.asset_preview_data {
            ui.horizontal(|ui| {
                ui.label(format!("Meshes: {}", data.meshes.len()));
                ui.separator();
                ui.label(format!("Materials: {}", data.materials_count));
                ui.separator();
                ui.label(format!("Animations: {}", data.animations_count));
                ui.separator();
                let total_verts: usize = data.meshes.iter().map(|m| m.vertex_count).sum();
                let total_tris: usize = data.meshes.iter().map(|m| m.triangle_count).sum();
                ui.label(format!("Verts: {}", total_verts));
                ui.separator();
                ui.label(format!("Tris: {}", total_tris));
            });
            ui.separator();
        }

        // Animation controls
        let Some(data) = &self.asset_preview_data else {
            ui.label("Failed to load model data");
            return;
        };
        if !data.anim_clips.is_empty() {
            ui.horizontal(|ui| {
                // Play/Pause
                if ui
                    .button(if self.asset_preview_anim_playing {
                        "\u{23f8}" // pause
                    } else {
                        "\u{25b6}" // play
                    })
                    .clicked()
                {
                    self.asset_preview_anim_playing = !self.asset_preview_anim_playing;
                    if self.asset_preview_anim_playing {
                        self.asset_preview_last_instant = Some(std::time::Instant::now());
                    }
                }

                // Clip selector
                let clip_name = data
                    .anim_clips
                    .get(self.asset_preview_anim_idx)
                    .map(|c| c.name.as_str())
                    .unwrap_or("None");
                egui::ComboBox::from_id_salt("anim_clip_select")
                    .selected_text(clip_name)
                    .width(100.0)
                    .show_ui(ui, |ui| {
                        for (i, clip) in data.anim_clips.iter().enumerate() {
                            if ui
                                .selectable_label(i == self.asset_preview_anim_idx, &clip.name)
                                .clicked()
                            {
                                self.asset_preview_anim_idx = i;
                                self.asset_preview_anim_time = 0.0;
                            }
                        }
                    });

                // Time slider
                if let Some(clip) = data.anim_clips.get(self.asset_preview_anim_idx) {
                    let duration = clip.duration;
                    ui.add(
                        egui::Slider::new(&mut self.asset_preview_anim_time, 0.0..=duration)
                            .text("s")
                            .max_decimals(2),
                    );
                }
            });
            ui.separator();
        }

        // Advance animation time
        if self.asset_preview_anim_playing && !data.anim_clips.is_empty() {
            let now = std::time::Instant::now();
            if let Some(last) = self.asset_preview_last_instant {
                let dt = now.duration_since(last).as_secs_f32();
                if let Some(clip) = data.anim_clips.get(self.asset_preview_anim_idx) {
                    self.asset_preview_anim_time += dt;
                    if clip.duration > 0.0 {
                        self.asset_preview_anim_time %= clip.duration;
                    }
                }
            }
            self.asset_preview_last_instant = Some(now);
            ui.ctx().request_repaint();
        }

        // Compute skinned vertices
        let data = self.asset_preview_data.as_ref().unwrap();
        let skinned_verts: Vec<[f32; 3]> = if !data.joint_node_indices.is_empty()
            && !data.skin_vertices.is_empty()
            && !data.anim_clips.is_empty()
        {
            Self::compute_skinned_vertices(
                data,
                self.asset_preview_anim_idx,
                self.asset_preview_anim_time,
            )
        } else {
            data.vertices.clone()
        };

        // 3D preview

        let available = ui.available_size();
        let preview_size = available.x.min(available.y - 20.0).max(200.0);
        let (response, painter) = ui.allocate_painter(
            egui::vec2(available.x, preview_size),
            egui::Sense::click_and_drag(),
        );
        let rect = response.rect;

        // Orbit camera
        if response.dragged_by(egui::PointerButton::Primary) {
            let delta = response.drag_delta();
            self.asset_preview_rot_y += delta.x * 0.01;
            self.asset_preview_rot_x += delta.y * 0.01;
            self.asset_preview_rot_x = self.asset_preview_rot_x.clamp(
                -std::f32::consts::FRAC_PI_2 + 0.1,
                std::f32::consts::FRAC_PI_2 - 0.1,
            );
        }

        let scroll_delta = ui.input(|i| {
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
        if scroll_delta != 0.0 {
            self.asset_preview_zoom *= 1.0 + scroll_delta * 0.002;
            self.asset_preview_zoom = self.asset_preview_zoom.clamp(0.1, 20.0);
        }

        if response.double_clicked() {
            self.asset_preview_rot_y = std::f32::consts::FRAC_PI_4;
            self.asset_preview_rot_x = 0.3;
            self.asset_preview_zoom = 1.0;
        }

        // Dark background
        painter.rect_filled(rect, 4.0, egui::Color32::from_rgb(20, 20, 25));

        // Grid
        let grid_color = egui::Color32::from_rgba_premultiplied(255, 255, 255, 15);
        let grid_count = 8;
        for i in 0..=grid_count {
            let t = i as f32 / grid_count as f32;
            let x = rect.min.x + t * rect.width();
            let y = rect.min.y + t * rect.height();
            painter.line_segment(
                [egui::pos2(x, rect.min.y), egui::pos2(x, rect.max.y)],
                egui::Stroke::new(0.5, grid_color),
            );
            painter.line_segment(
                [egui::pos2(rect.min.x, y), egui::pos2(rect.max.x, y)],
                egui::Stroke::new(0.5, grid_color),
            );
        }

        // Hint
        painter.text(
            egui::pos2(rect.max.x - 8.0, rect.min.y + 12.0),
            egui::Align2::RIGHT_TOP,
            "Drag: rotate | Scroll: zoom | Double-click: reset",
            egui::FontId::proportional(10.0),
            egui::Color32::from_rgb(80, 80, 80),
        );

        if !skinned_verts.is_empty() {
            let angle_y = self.asset_preview_rot_y;
            let angle_x = self.asset_preview_rot_x;
            let zoom = self.asset_preview_zoom;

            let center = [
                (data.bounds_min[0] + data.bounds_max[0]) / 2.0,
                (data.bounds_min[1] + data.bounds_max[1]) / 2.0,
                (data.bounds_min[2] + data.bounds_max[2]) / 2.0,
            ];
            let extent = [
                data.bounds_max[0] - data.bounds_min[0],
                data.bounds_max[1] - data.bounds_min[1],
                data.bounds_max[2] - data.bounds_min[2],
            ];
            let max_extent = extent[0].max(extent[1]).max(extent[2]).max(0.001);
            let scale = (preview_size * 0.35) / max_extent * zoom;

            let cos_y = angle_y.cos();
            let sin_y = angle_y.sin();
            let cos_x = angle_x.cos();
            let sin_x = angle_x.sin();

            // Project 3D -> 2D; also return rotated Z for depth sorting
            let project_3d = |v: &[f32; 3]| -> (egui::Pos2, f32) {
                let x = v[0] - center[0];
                let y = v[1] - center[1];
                let z = v[2] - center[2];
                let rx = x * cos_y - z * sin_y;
                let rz = x * sin_y + z * cos_y;
                let ry = y;
                let fy = ry * cos_x - rz * sin_x;
                let fz = ry * sin_x + rz * cos_x;
                let fx = rx;
                (
                    egui::pos2(rect.center().x + fx * scale, rect.center().y - fy * scale),
                    fz,
                )
            };

            if !data.triangles.is_empty() {
                // Filled triangles with simple directional lighting
                let light_dir = [0.3_f32, 0.7, 0.6]; // normalized-ish
                let light_len = (light_dir[0] * light_dir[0]
                    + light_dir[1] * light_dir[1]
                    + light_dir[2] * light_dir[2])
                    .sqrt();

                // Build sorted triangle list (back-to-front)
                let mut sorted_tris: Vec<(f32, &crate::app::model_preview::TriFace)> = data
                    .triangles
                    .iter()
                    .filter_map(|tri| {
                        let v0 = skinned_verts.get(tri.idx[0])?;
                        let v1 = skinned_verts.get(tri.idx[1])?;
                        let v2 = skinned_verts.get(tri.idx[2])?;
                        let (_, z0) = project_3d(v0);
                        let (_, z1) = project_3d(v1);
                        let (_, z2) = project_3d(v2);
                        Some(((z0 + z1 + z2) / 3.0, tri))
                    })
                    .collect();
                sorted_tris
                    .sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

                for (_, tri) in &sorted_tris {
                    let v0 = &skinned_verts[tri.idx[0]];
                    let v1 = &skinned_verts[tri.idx[1]];
                    let v2 = &skinned_verts[tri.idx[2]];

                    // Face normal for lighting
                    let e1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
                    let e2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];
                    let nx = e1[1] * e2[2] - e1[2] * e2[1];
                    let ny = e1[2] * e2[0] - e1[0] * e2[2];
                    let nz = e1[0] * e2[1] - e1[1] * e2[0];
                    let nl = (nx * nx + ny * ny + nz * nz).sqrt().max(0.0001);

                    // Rotate normal same as vertices
                    let rnx = (nx / nl) * cos_y - (nz / nl) * sin_y;
                    let rnz = (nx / nl) * sin_y + (nz / nl) * cos_y;
                    let rny = ny / nl;
                    let fny = rny * cos_x - rnz * sin_x;
                    let fnz = rny * sin_x + rnz * cos_x;
                    let fnx = rnx;

                    let dot =
                        (fnx * light_dir[0] + fny * light_dir[1] + fnz * light_dir[2]) / light_len;
                    let brightness = 0.3 + 0.7 * dot.abs(); // ambient + diffuse

                    let r = (tri.color[0] as f32 * brightness).min(255.0) as u8;
                    let g = (tri.color[1] as f32 * brightness).min(255.0) as u8;
                    let b = (tri.color[2] as f32 * brightness).min(255.0) as u8;

                    let (p0, _) = project_3d(v0);
                    let (p1, _) = project_3d(v1);
                    let (p2, _) = project_3d(v2);

                    // Clip: skip if all points outside rect
                    if !rect.contains(p0) && !rect.contains(p1) && !rect.contains(p2) {
                        continue;
                    }

                    let mesh = egui::Mesh {
                        indices: vec![0, 1, 2],
                        vertices: vec![
                            egui::epaint::Vertex {
                                pos: p0,
                                uv: egui::pos2(0.0, 0.0),
                                color: egui::Color32::from_rgb(r, g, b),
                            },
                            egui::epaint::Vertex {
                                pos: p1,
                                uv: egui::pos2(0.0, 0.0),
                                color: egui::Color32::from_rgb(r, g, b),
                            },
                            egui::epaint::Vertex {
                                pos: p2,
                                uv: egui::pos2(0.0, 0.0),
                                color: egui::Color32::from_rgb(r, g, b),
                            },
                        ],
                        texture_id: egui::TextureId::default(),
                    };
                    painter.add(egui::Shape::mesh(mesh));
                }
            } else {
                // Fallback: wireframe only
                let edge_color = egui::Color32::from_rgb(100, 180, 255);
                for &(a, b) in &data.edges {
                    if a < skinned_verts.len() && b < skinned_verts.len() {
                        let (p1, _) = project_3d(&skinned_verts[a]);
                        let (p2, _) = project_3d(&skinned_verts[b]);
                        if rect.contains(p1) || rect.contains(p2) {
                            painter.line_segment([p1, p2], egui::Stroke::new(1.0, edge_color));
                        }
                    }
                }
            }
        }
    }

    /// Compute skinned vertex positions by applying skeletal animation.
    fn compute_skinned_vertices(
        data: &crate::app::model_preview::ModelPreviewData,
        clip_idx: usize,
        time: f32,
    ) -> Vec<[f32; 3]> {
        use crate::app::model_preview::AnimProperty;

        let clip = match data.anim_clips.get(clip_idx) {
            Some(c) => c,
            None => return data.vertices.clone(),
        };

        let node_count = data.node_transforms.len();
        if node_count == 0 {
            return data.vertices.clone();
        }

        // Start with rest-pose node transforms
        let mut local_transforms = data.node_transforms.clone();

        // Apply animation channels
        for ch in &clip.channels {
            if ch.node_index >= node_count || ch.times.is_empty() || ch.values.is_empty() {
                continue;
            }
            let val = Self::sample_channel(ch, time);
            let m = &mut local_transforms[ch.node_index];
            match ch.property {
                AnimProperty::Translation => {
                    m[3][0] = val[0];
                    m[3][1] = val[1];
                    m[3][2] = val[2];
                }
                AnimProperty::Rotation => {
                    // Convert quaternion to rotation matrix, keep existing translation/scale
                    let tx = m[3][0];
                    let ty = m[3][1];
                    let tz = m[3][2];
                    let sx = (m[0][0] * m[0][0] + m[0][1] * m[0][1] + m[0][2] * m[0][2]).sqrt();
                    let sy = (m[1][0] * m[1][0] + m[1][1] * m[1][1] + m[1][2] * m[1][2]).sqrt();
                    let sz = (m[2][0] * m[2][0] + m[2][1] * m[2][1] + m[2][2] * m[2][2]).sqrt();
                    let rot = Self::quat_to_mat3(val);
                    m[0] = [rot[0][0] * sx, rot[0][1] * sx, rot[0][2] * sx, 0.0];
                    m[1] = [rot[1][0] * sy, rot[1][1] * sy, rot[1][2] * sy, 0.0];
                    m[2] = [rot[2][0] * sz, rot[2][1] * sz, rot[2][2] * sz, 0.0];
                    m[3] = [tx, ty, tz, 1.0];
                }
                AnimProperty::Scale => {
                    // Normalize existing rotation columns then rescale
                    for col in 0..3 {
                        let len =
                            (m[col][0] * m[col][0] + m[col][1] * m[col][1] + m[col][2] * m[col][2])
                                .sqrt()
                                .max(0.0001);
                        let s = val[col];
                        m[col][0] = m[col][0] / len * s;
                        m[col][1] = m[col][1] / len * s;
                        m[col][2] = m[col][2] / len * s;
                    }
                }
            }
        }

        // Compute world transforms (parent chain)
        let mut world_transforms = vec![[[0.0f32; 4]; 4]; node_count];
        for i in 0..node_count {
            world_transforms[i] = match data.node_parents[i] {
                Some(parent) if parent < i => {
                    Self::mat4_mul(&world_transforms[parent], &local_transforms[i])
                }
                _ => local_transforms[i],
            };
        }

        // Compute joint matrices: world_transform[joint_node] * inverse_bind_matrix
        let joint_matrices: Vec<[[f32; 4]; 4]> = data
            .joint_node_indices
            .iter()
            .enumerate()
            .map(|(ji, &node_idx)| {
                let world = if node_idx < world_transforms.len() {
                    &world_transforms[node_idx]
                } else {
                    return [
                        [1.0, 0.0, 0.0, 0.0],
                        [0.0, 1.0, 0.0, 0.0],
                        [0.0, 0.0, 1.0, 0.0],
                        [0.0, 0.0, 0.0, 1.0],
                    ];
                };
                let ibm = data.inverse_bind_matrices.get(ji).copied().unwrap_or([
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [0.0, 0.0, 0.0, 1.0],
                ]);
                Self::mat4_mul(world, &ibm)
            })
            .collect();

        // Skin each vertex
        data.vertices
            .iter()
            .enumerate()
            .map(|(vi, v)| {
                let skin = data.skin_vertices.get(vi);
                if let Some(skin) = skin {
                    let w_sum =
                        skin.weights[0] + skin.weights[1] + skin.weights[2] + skin.weights[3];
                    if w_sum < 0.001 {
                        return *v;
                    }
                    let mut out = [0.0f32; 3];
                    for k in 0..4 {
                        let w = skin.weights[k];
                        if w < 0.001 {
                            continue;
                        }
                        let ji = skin.joints[k] as usize;
                        if ji >= joint_matrices.len() {
                            continue;
                        }
                        let m = &joint_matrices[ji];
                        out[0] += w * (m[0][0] * v[0] + m[1][0] * v[1] + m[2][0] * v[2] + m[3][0]);
                        out[1] += w * (m[0][1] * v[0] + m[1][1] * v[1] + m[2][1] * v[2] + m[3][1]);
                        out[2] += w * (m[0][2] * v[0] + m[1][2] * v[1] + m[2][2] * v[2] + m[3][2]);
                    }
                    out
                } else {
                    *v
                }
            })
            .collect()
    }

    /// Linearly interpolate an animation channel at the given time.
    fn sample_channel(ch: &crate::app::model_preview::AnimChannel, time: f32) -> [f32; 4] {
        if ch.times.len() == 1 || time <= ch.times[0] {
            return ch.values[0];
        }
        if time >= *ch.times.last().unwrap() {
            return *ch.values.last().unwrap();
        }
        // Find bracketing keyframes
        let mut i = 0;
        while i + 1 < ch.times.len() && ch.times[i + 1] < time {
            i += 1;
        }
        let t0 = ch.times[i];
        let t1 = ch.times[i + 1];
        let f = if (t1 - t0).abs() > 0.0001 {
            (time - t0) / (t1 - t0)
        } else {
            0.0
        };
        let a = &ch.values[i];
        let b = &ch.values.get(i + 1).unwrap_or(a);
        if ch.property == crate::app::model_preview::AnimProperty::Rotation {
            Self::quat_slerp(a, b, f)
        } else {
            [
                a[0] + (b[0] - a[0]) * f,
                a[1] + (b[1] - a[1]) * f,
                a[2] + (b[2] - a[2]) * f,
                0.0,
            ]
        }
    }

    /// Quaternion spherical linear interpolation.
    fn quat_slerp(a: &[f32; 4], b: &[f32; 4], t: f32) -> [f32; 4] {
        let mut dot = a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3];
        let mut b2 = *b;
        if dot < 0.0 {
            dot = -dot;
            b2 = [-b[0], -b[1], -b[2], -b[3]];
        }
        if dot > 0.9995 {
            // Linear interpolation for near-equal quaternions
            let r = [
                a[0] + (b2[0] - a[0]) * t,
                a[1] + (b2[1] - a[1]) * t,
                a[2] + (b2[2] - a[2]) * t,
                a[3] + (b2[3] - a[3]) * t,
            ];
            let len = (r[0] * r[0] + r[1] * r[1] + r[2] * r[2] + r[3] * r[3]).sqrt();
            return [r[0] / len, r[1] / len, r[2] / len, r[3] / len];
        }
        let theta = dot.acos();
        let sin_theta = theta.sin();
        let wa = ((1.0 - t) * theta).sin() / sin_theta;
        let wb = (t * theta).sin() / sin_theta;
        [
            a[0] * wa + b2[0] * wb,
            a[1] * wa + b2[1] * wb,
            a[2] * wa + b2[2] * wb,
            a[3] * wa + b2[3] * wb,
        ]
    }

    /// Convert quaternion [x, y, z, w] to 3x3 rotation matrix.
    fn quat_to_mat3(q: [f32; 4]) -> [[f32; 3]; 3] {
        let (x, y, z, w) = (q[0], q[1], q[2], q[3]);
        [
            [
                1.0 - 2.0 * (y * y + z * z),
                2.0 * (x * y + z * w),
                2.0 * (x * z - y * w),
            ],
            [
                2.0 * (x * y - z * w),
                1.0 - 2.0 * (x * x + z * z),
                2.0 * (y * z + x * w),
            ],
            [
                2.0 * (x * z + y * w),
                2.0 * (y * z - x * w),
                1.0 - 2.0 * (x * x + y * y),
            ],
        ]
    }

    /// Multiply two column-major 4x4 matrices.
    fn mat4_mul(a: &[[f32; 4]; 4], b: &[[f32; 4]; 4]) -> [[f32; 4]; 4] {
        let mut out = [[0.0f32; 4]; 4];
        for col in 0..4 {
            for row in 0..4 {
                out[col][row] = a[0][row] * b[col][0]
                    + a[1][row] * b[col][1]
                    + a[2][row] * b[col][2]
                    + a[3][row] * b[col][3];
            }
        }
        out
    }
}
