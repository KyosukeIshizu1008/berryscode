use super::BerryCodeApp;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

impl BerryCodeApp {
    /// Render the scene preview panel (shown as bottom panel when a .scn.ron file is open)
    pub(crate) fn render_scene_preview(&mut self, ctx: &egui::Context) {
        // Check if current file is a .scn.ron
        if self.editor_tabs.is_empty() || self.active_tab_idx >= self.editor_tabs.len() {
            return;
        }

        let tab = &self.editor_tabs[self.active_tab_idx];
        if !tab.file_path.ends_with(".scn.ron") && !tab.file_path.ends_with(".ron") {
            return;
        }

        // Get current content and check if we need to reparse
        let content = tab.buffer.to_string();
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        let content_hash = hasher.finish();

        if content_hash != self.scene_preview.last_content_hash {
            self.scene_preview.last_content_hash = content_hash;
            match crate::bevy_ide::scene_preview::parser::parse_scene_ron(&content) {
                Ok(entities) => {
                    self.scene_preview.entities = entities;
                    self.scene_preview.parse_error = None;
                }
                Err(err) => {
                    self.scene_preview.parse_error = Some(err);
                }
            }
        }

        // Render bottom panel
        egui::TopBottomPanel::bottom("scene_preview")
            .default_height(200.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("Scene Preview");
                    ui.label(format!(
                        "({})",
                        tab.file_path.split('/').last().unwrap_or("")
                    ));
                });
                ui.separator();

                if let Some(err) = &self.scene_preview.parse_error {
                    ui.colored_label(egui::Color32::RED, format!("Parse Error: {}", err));
                    return;
                }

                egui::ScrollArea::both().show(ui, |ui| {
                    let entities = self.scene_preview.entities.clone();
                    for (idx, entity) in entities.iter().enumerate() {
                        let name = format!(
                            "Entity {} ({} components)",
                            entity.entity_id,
                            entity.components.len()
                        );
                        let selected = self.scene_preview.selected_entity == Some(idx);

                        let header = egui::CollapsingHeader::new(&name)
                            .default_open(selected)
                            .show(ui, |ui| {
                                for comp in &entity.components {
                                    ui.collapsing(&comp.type_name, |ui| {
                                        for (prop_name, prop_value) in &comp.properties {
                                            ui.horizontal(|ui| {
                                                ui.label(prop_name);
                                                ui.monospace(prop_value);
                                            });
                                        }
                                        if comp.properties.is_empty() {
                                            ui.label("(no properties)");
                                        }
                                    });
                                }
                            });

                        if header.header_response.clicked() {
                            self.scene_preview.selected_entity = Some(idx);
                        }
                    }

                    if entities.is_empty() && self.scene_preview.parse_error.is_none() {
                        ui.label("No entities found in scene file.");
                    }
                });
            });
    }
}
