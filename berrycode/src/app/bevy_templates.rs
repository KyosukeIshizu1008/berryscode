//! Bevy Templates UI panel
//!
//! Renders the template generator sidebar for creating
//! Bevy boilerplate code (Components, Resources, Systems, etc.)

use super::BerryCodeApp;
use crate::bevy_ide::templates::BevyTemplate;

/// Template selection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateType {
    Component,
    Resource,
    System,
    Plugin,
    StartupSystem,
    Event,
    State,
}

impl TemplateType {
    /// All available template types for iteration
    pub const ALL: &'static [TemplateType] = &[
        TemplateType::Component,
        TemplateType::Resource,
        TemplateType::System,
        TemplateType::Plugin,
        TemplateType::StartupSystem,
        TemplateType::Event,
        TemplateType::State,
    ];

    /// Human-readable label
    pub fn label(&self) -> &'static str {
        match self {
            TemplateType::Component => "Component",
            TemplateType::Resource => "Resource",
            TemplateType::System => "System",
            TemplateType::Plugin => "Plugin",
            TemplateType::StartupSystem => "Startup System",
            TemplateType::Event => "Event",
            TemplateType::State => "State",
        }
    }

    /// Whether this type has fields (name, type) pairs
    pub fn has_fields(&self) -> bool {
        matches!(
            self,
            TemplateType::Component | TemplateType::Resource | TemplateType::Event
        )
    }

    /// Whether this type has system parameters
    pub fn has_params(&self) -> bool {
        matches!(self, TemplateType::System)
    }

    /// Whether this type has enum variants
    pub fn has_variants(&self) -> bool {
        matches!(self, TemplateType::State)
    }
}

impl Default for TemplateType {
    fn default() -> Self {
        TemplateType::Component
    }
}

impl BerryCodeApp {
    /// Build the BevyTemplate from current UI state
    fn build_bevy_template(&self) -> BevyTemplate {
        match self.template_type {
            TemplateType::Component => BevyTemplate::Component {
                name: self.template_name.clone(),
                fields: self.template_fields.clone(),
            },
            TemplateType::Resource => BevyTemplate::Resource {
                name: self.template_name.clone(),
                fields: self.template_fields.clone(),
            },
            TemplateType::System => BevyTemplate::System {
                name: self.template_name.clone(),
                params: self.template_params.clone(),
            },
            TemplateType::Plugin => BevyTemplate::Plugin {
                name: self.template_name.clone(),
            },
            TemplateType::StartupSystem => BevyTemplate::StartupSystem {
                name: self.template_name.clone(),
            },
            TemplateType::Event => BevyTemplate::Event {
                name: self.template_name.clone(),
                fields: self.template_fields.clone(),
            },
            TemplateType::State => BevyTemplate::State {
                name: self.template_name.clone(),
                variants: self.template_variants.clone(),
            },
        }
    }

    /// Render the Bevy Templates panel in the sidebar
    pub(crate) fn render_bevy_templates_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Bevy Templates");
        ui.separator();

        // Template type selector
        ui.label("Template Type:");
        let current_label = self.template_type.label();
        egui::ComboBox::from_id_salt("template_type_selector")
            .selected_text(current_label)
            .show_ui(ui, |ui| {
                for tt in TemplateType::ALL {
                    ui.selectable_value(&mut self.template_type, *tt, tt.label());
                }
            });

        ui.add_space(8.0);

        // Name input
        ui.label("Name:");
        ui.text_edit_singleline(&mut self.template_name);

        ui.add_space(8.0);

        // Dynamic fields for Component / Resource / Event
        if self.template_type.has_fields() {
            ui.label("Fields:");
            ui.separator();

            let mut remove_idx = None;
            let field_count = self.template_fields.len();
            for i in 0..field_count {
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    let (field_name, field_type) = &mut self.template_fields[i];
                    ui.add(egui::TextEdit::singleline(field_name).desired_width(80.0));
                    ui.label("Type:");
                    ui.add(egui::TextEdit::singleline(field_type).desired_width(80.0));
                    if ui.small_button("\u{ea76}").clicked() {
                        // codicon-close
                        remove_idx = Some(i);
                    }
                });
            }
            if let Some(idx) = remove_idx {
                self.template_fields.remove(idx);
            }

            if ui.button("+ Add Field").clicked() {
                self.template_fields
                    .push(("field".to_string(), "f32".to_string()));
            }

            ui.add_space(8.0);
        }

        // System parameters
        if self.template_type.has_params() {
            ui.label("Parameters:");
            ui.separator();

            let mut remove_idx = None;
            let param_count = self.template_params.len();
            for i in 0..param_count {
                ui.horizontal(|ui| {
                    let param = &mut self.template_params[i];
                    ui.add(egui::TextEdit::singleline(param).desired_width(200.0));
                    if ui.small_button("\u{ea76}").clicked() {
                        remove_idx = Some(i);
                    }
                });
            }
            if let Some(idx) = remove_idx {
                self.template_params.remove(idx);
            }

            if ui.button("+ Add Parameter").clicked() {
                self.template_params
                    .push("query: Query<&Transform>".to_string());
            }

            ui.add_space(8.0);
        }

        // State variants
        if self.template_type.has_variants() {
            ui.label("Variants:");
            ui.separator();

            let mut remove_idx = None;
            let variant_count = self.template_variants.len();
            for i in 0..variant_count {
                ui.horizontal(|ui| {
                    let variant = &mut self.template_variants[i];
                    ui.add(egui::TextEdit::singleline(variant).desired_width(200.0));
                    if ui.small_button("\u{ea76}").clicked() {
                        remove_idx = Some(i);
                    }
                });
            }
            if let Some(idx) = remove_idx {
                self.template_variants.remove(idx);
            }

            if ui.button("+ Add Variant").clicked() {
                self.template_variants.push("NewState".to_string());
            }

            ui.add_space(8.0);
        }

        ui.separator();

        // Code preview
        let template = self.build_bevy_template();
        let generated = template.generate();

        ui.label("Preview:");
        egui::ScrollArea::vertical()
            .max_height(200.0)
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut generated.as_str())
                        .code_editor()
                        .desired_width(f32::INFINITY),
                );
            });

        ui.add_space(8.0);

        // Insert button
        if ui.button("Insert at Cursor").clicked() {
            self.insert_template_at_cursor(&generated);
        }
    }

    /// Insert generated template code at the current cursor position in the active editor tab
    fn insert_template_at_cursor(&mut self, code: &str) {
        if self.editor_tabs.is_empty() {
            self.status_message = "No file open. Open a file first.".to_string();
            self.status_message_timestamp = Some(std::time::Instant::now());
            return;
        }

        if self.active_tab_idx >= self.editor_tabs.len() {
            return;
        }

        let tab = &mut self.editor_tabs[self.active_tab_idx];
        if tab.is_readonly {
            self.status_message = "Cannot insert into a read-only file.".to_string();
            self.status_message_timestamp = Some(std::time::Instant::now());
            return;
        }

        // Insert at cursor position
        let line = tab.cursor_line;
        let col = tab.cursor_col;
        let line_start = tab.buffer.line_to_char(line);
        tab.buffer.insert(line_start + col, code);
        tab.is_dirty = true;

        self.status_message = format!("Template inserted at line {}", line + 1);
        self.status_message_timestamp = Some(std::time::Instant::now());
    }
}
