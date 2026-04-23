use super::scene_editor::asset_import::AssetImportSettings;
use super::BerryCodeApp;
use crate::app::i18n::t;
use crate::bevy_ide::assets::scanner::{format_size, scan_assets, AssetType, AssetViewMode};

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

        // Asset list
        let filter_query = self.asset_browser.filter_query.to_lowercase();
        let filter_type = self.asset_browser.filter_type.clone();

        let filtered_assets: Vec<_> = self
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
            .collect();

        // Collect data needed for post-scroll-area actions
        let mut clicked_idx: Option<usize> = None;
        let mut double_clicked_path: Option<(String, AssetType)> = None;

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (idx, asset) in &filtered_assets {
                let selected = self.asset_browser.selected_asset == Some(*idx);
                let asset_path_str = asset.path.to_string_lossy().to_string();

                let response = ui.horizontal(|ui| {
                    // Show thumbnail for image assets, fallback to text icon
                    if asset.asset_type == AssetType::Image {
                        if let Some(tex) = self.thumbnail_cache.get_or_create(ctx, &asset_path_str) {
                            let size = egui::vec2(16.0, 16.0);
                            ui.image((tex.id(), size));
                        } else {
                            ui.label(asset.asset_type.icon());
                        }
                    } else {
                        ui.label(asset.asset_type.icon());
                    }
                    let resp = ui.selectable_label(selected, &asset.file_name);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(format_size(asset.size_bytes));
                        ui.colored_label(egui::Color32::GRAY, asset.asset_type.label());
                    });
                    resp
                });

                if response.inner.clicked() {
                    clicked_idx = Some(*idx);
                }

                if response.inner.double_clicked() {
                    double_clicked_path = Some((
                        asset.path.to_string_lossy().to_string(),
                        asset.asset_type.clone(),
                    ));
                }
            }

            if filtered_assets.is_empty() {
                if self.asset_browser.assets.is_empty() {
                    ui.label(t(self.ui_language, "No assets directory found. Create an 'assets/' folder in your project root."));
                } else {
                    ui.label(t(self.ui_language, "No assets match the current filter."));
                }
            }
        });

        // Apply click actions
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
}
