//! ECS Inspector UI panel for browsing Bevy Entity/Component/Resource state

use super::BerryCodeApp;

/// Tab selection for the ECS Inspector panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EcsInspectorTab {
    #[default]
    Entities,
    Resources,
}

impl BerryCodeApp {
    pub(crate) fn render_ecs_inspector_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("ECS Inspector");
        ui.separator();

        // Connection section
        ui.horizontal(|ui| {
            ui.label("Endpoint:");
            ui.text_edit_singleline(&mut self.ecs_inspector.endpoint);
        });

        ui.horizontal(|ui| {
            if self.ecs_inspector.connected {
                if ui.button("Disconnect").clicked() {
                    self.ecs_inspector.connected = false;
                    self.ecs_inspector.entities.clear();
                    self.ecs_inspector.resources.clear();
                }
                ui.colored_label(egui::Color32::GREEN, "Connected");
            } else {
                if ui.button("Connect").clicked() {
                    self.connect_to_bevy_app();
                }
                ui.colored_label(egui::Color32::RED, "Disconnected");
            }
        });

        if let Some(err) = &self.ecs_inspector.error_message {
            let err = err.clone();
            ui.colored_label(egui::Color32::RED, &err);
        }

        ui.separator();

        if self.ecs_inspector.connected {
            // Auto-refresh
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.ecs_inspector.auto_refresh, "Auto-refresh");
                if ui.button("\u{27f3} Refresh").clicked() {
                    self.refresh_ecs_data();
                }
            });

            // Filter
            ui.horizontal(|ui| {
                ui.label("Filter:");
                ui.text_edit_singleline(&mut self.ecs_inspector.filter_query);
            });

            ui.separator();

            // Tab bar: Entities | Resources
            ui.horizontal(|ui| {
                let ent_selected = self.ecs_inspector_tab == EcsInspectorTab::Entities;
                if ui.selectable_label(ent_selected, "Entities").clicked() {
                    self.ecs_inspector_tab = EcsInspectorTab::Entities;
                }
                let res_selected = self.ecs_inspector_tab == EcsInspectorTab::Resources;
                if ui.selectable_label(res_selected, "Resources").clicked() {
                    self.ecs_inspector_tab = EcsInspectorTab::Resources;
                }
            });

            ui.separator();

            match self.ecs_inspector_tab {
                EcsInspectorTab::Entities => {
                    self.render_ecs_entities_tab(ui);
                }
                EcsInspectorTab::Resources => {
                    self.render_ecs_resources_tab(ui);
                }
            }
        }
    }

    /// Render the Entities tab of the ECS Inspector.
    fn render_ecs_entities_tab(&mut self, ui: &mut egui::Ui) {
        let entity_count = self.ecs_inspector.entities.len();
        ui.heading(format!("Entities ({})", entity_count));

        let filter = self.ecs_inspector.filter_query.to_lowercase();
        let entities_snapshot: Vec<_> = self.ecs_inspector.entities.iter().map(|e| {
            (e.id, e.name.clone(), e.components.len())
        }).collect();

        egui::ScrollArea::vertical()
            .id_salt("entity_list")
            .max_height(ui.available_height() * 0.6)
            .show(ui, |ui| {
                for (id, name, comp_count) in &entities_snapshot {
                    let display_name = name.as_deref().unwrap_or("(unnamed)");
                    let label = format!("Entity {} - {} ({} components)",
                        id, display_name, comp_count);

                    if !filter.is_empty() && !label.to_lowercase().contains(&filter) {
                        continue;
                    }

                    let selected = self.ecs_inspector.selected_entity == Some(*id);
                    if ui.selectable_label(selected, &label).clicked() {
                        self.ecs_inspector.selected_entity = Some(*id);
                        self.load_entity_components(*id);
                    }
                }
            });

        ui.separator();

        // Component details for selected entity
        if let Some(entity_id) = self.ecs_inspector.selected_entity {
            ui.heading(format!("Components (Entity {})", entity_id));
            egui::ScrollArea::vertical()
                .id_salt("component_details")
                .show(ui, |ui| {
                    let keys: Vec<_> = self.ecs_inspector.component_values
                        .keys()
                        .filter(|(eid, _)| *eid == entity_id)
                        .cloned()
                        .collect();

                    for (_, comp_name) in &keys {
                        if let Some(value) = self.ecs_inspector.component_values.get(&(entity_id, comp_name.clone())) {
                            let json_str = serde_json::to_string_pretty(value)
                                .unwrap_or_else(|_| "Error serializing".to_string());
                            ui.collapsing(comp_name, |ui| {
                                ui.monospace(&json_str);
                            });
                        }
                    }
                });
        }
    }

    /// Render the Resources tab of the ECS Inspector.
    /// Shows resources fetched from a running Bevy app via BRP.
    fn render_ecs_resources_tab(&mut self, ui: &mut egui::Ui) {
        let resource_count = self.ecs_inspector.resources.len();
        ui.heading(format!("Resources ({})", resource_count));

        if resource_count == 0 {
            ui.label("No resources discovered. Click Refresh to query the running Bevy app.");
            return;
        }

        let filter = self.ecs_inspector.filter_query.to_lowercase();

        egui::ScrollArea::vertical()
            .id_salt("resource_list")
            .show(ui, |ui| {
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

                    let selected = self.ecs_inspector.selected_resource.as_deref() == Some(type_name.as_str());
                    if ui.selectable_label(selected, type_name).clicked() {
                        self.ecs_inspector.selected_resource = Some(type_name.clone());
                    }
                }
            });

        ui.separator();

        // Resource value details
        if let Some(ref res_name) = self.ecs_inspector.selected_resource.clone() {
            ui.heading(format!("Resource: {}", res_name));
            if let Some(value) = self.ecs_inspector.resource_values.get(res_name) {
                let json_str = serde_json::to_string_pretty(value)
                    .unwrap_or_else(|_| "Error serializing".to_string());
                egui::ScrollArea::vertical()
                    .id_salt("resource_value")
                    .show(ui, |ui| {
                        ui.monospace(&json_str);
                    });
            } else {
                ui.label("(no data available -- refresh to fetch)");
            }
        }
    }

    fn connect_to_bevy_app(&mut self) {
        let endpoint = self.ecs_inspector.endpoint.clone();
        let runtime = self.lsp_runtime.clone();
        // Use a channel to receive connection result
        let (tx, _rx) = tokio::sync::oneshot::channel();

        runtime.spawn(async move {
            let mut client = crate::bevy_ide::inspector::brp_client::BrpClient::new(&endpoint);
            let result = client.ping().await;
            let _ = tx.send(result);
        });

        // For now, set connected optimistically - in a real implementation
        // we'd poll the channel
        self.ecs_inspector.connected = true;
        self.ecs_inspector.error_message = None;
    }

    fn refresh_ecs_data(&mut self) {
        // TODO: async refresh via channel
    }

    fn load_entity_components(&mut self, _entity_id: u64) {
        // TODO: async load via channel
    }
}
