//! Bevy Resource Editor: edit global Resources in the Inspector.
//!
//! Resources are analogous to Bevy `Resource` types -- global singleton data
//! that is not attached to any entity. This module provides:
//! - `ResourceDef`: a serializable definition of a resource with typed fields.
//! - Inspector UI to view and edit resources.
//! - Code generation helper to produce `app.insert_resource(...)` calls.

use super::model::{ScriptField, ScriptValue};
use serde::{Deserialize, Serialize};

/// A named resource definition with typed fields, mirroring a Bevy `#[derive(Resource)]` struct.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourceDef {
    pub name: String,
    pub fields: Vec<ScriptField>,
}

/// Render the "Scene Resources" section in the Inspector panel.
/// Returns `true` if any resource was mutated this frame.
pub fn render_resource_inspector(
    ui: &mut egui::Ui,
    resources: &mut Vec<ResourceDef>,
) -> bool {
    let mut mutated = false;
    let mut remove_idx: Option<usize> = None;

    ui.separator();
    ui.heading("Scene Resources");

    if ui.button("+ Resource").clicked() {
        resources.push(ResourceDef {
            name: format!("NewResource{}", resources.len()),
            fields: Vec::new(),
        });
        mutated = true;
    }

    for (idx, res) in resources.iter_mut().enumerate() {
        let header = if res.name.is_empty() {
            format!("Resource #{}", idx)
        } else {
            res.name.clone()
        };

        let id = ui.make_persistent_id(format!("resource_{}", idx));
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true)
            .show_header(ui, |ui| {
                ui.label(egui::RichText::new(&header).strong());
                if ui.small_button("X").clicked() {
                    remove_idx = Some(idx);
                }
            })
            .body(|ui| {
                // Resource name
                ui.horizontal(|ui| {
                    ui.label("Type:");
                    if ui.text_edit_singleline(&mut res.name).changed() {
                        mutated = true;
                    }
                });

                // Fields
                let mut field_remove: Option<usize> = None;
                for (fi, field) in res.fields.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(&field.name);
                        match &mut field.value {
                            ScriptValue::Float(v) => {
                                if ui.add(egui::DragValue::new(v).speed(0.05)).changed() {
                                    mutated = true;
                                }
                            }
                            ScriptValue::Int(v) => {
                                let mut vi = *v as i32;
                                if ui.add(egui::DragValue::new(&mut vi)).changed() {
                                    *v = vi as i64;
                                    mutated = true;
                                }
                            }
                            ScriptValue::Bool(v) => {
                                if ui.checkbox(v, "").changed() {
                                    mutated = true;
                                }
                            }
                            ScriptValue::String(v) => {
                                if ui.text_edit_singleline(v).changed() {
                                    mutated = true;
                                }
                            }
                            _ => {
                                ui.label(format!("({})", field.value.type_label()));
                            }
                        }
                        if ui.small_button("x").clicked() {
                            field_remove = Some(fi);
                        }
                    });
                }
                if let Some(fi) = field_remove {
                    res.fields.remove(fi);
                    mutated = true;
                }

                // Add field button
                ui.horizontal(|ui| {
                    if ui.small_button("+ f32").clicked() {
                        res.fields.push(ScriptField {
                            name: format!("field{}", res.fields.len()),
                            value: ScriptValue::Float(0.0),
                        });
                        mutated = true;
                    }
                    if ui.small_button("+ i64").clicked() {
                        res.fields.push(ScriptField {
                            name: format!("field{}", res.fields.len()),
                            value: ScriptValue::Int(0),
                        });
                        mutated = true;
                    }
                    if ui.small_button("+ bool").clicked() {
                        res.fields.push(ScriptField {
                            name: format!("field{}", res.fields.len()),
                            value: ScriptValue::Bool(false),
                        });
                        mutated = true;
                    }
                    if ui.small_button("+ String").clicked() {
                        res.fields.push(ScriptField {
                            name: format!("field{}", res.fields.len()),
                            value: ScriptValue::String(String::new()),
                        });
                        mutated = true;
                    }
                });
            });
    }

    if let Some(idx) = remove_idx {
        resources.remove(idx);
        mutated = true;
    }

    mutated
}

/// Generate Rust code to insert all resources via `app.insert_resource(...)`.
pub fn generate_resource_code(resources: &[ResourceDef]) -> String {
    let mut code = String::new();
    for res in resources {
        if res.name.is_empty() {
            continue;
        }
        code.push_str(&format!("    app.insert_resource({} {{\n", res.name));
        for field in &res.fields {
            let val = format_value(&field.value);
            code.push_str(&format!("        {}: {},\n", field.name, val));
        }
        code.push_str("    });\n");
    }
    code
}

fn format_value(val: &ScriptValue) -> String {
    match val {
        ScriptValue::Float(v) => format!("{:.6}", v),
        ScriptValue::Int(v) => format!("{}", v),
        ScriptValue::Bool(v) => format!("{}", v),
        ScriptValue::String(v) => format!("\"{}\"", v),
        ScriptValue::Vec(items) => {
            let inner: Vec<String> = items.iter().map(format_value).collect();
            format!("vec![{}]", inner.join(", "))
        }
        ScriptValue::Option(opt) => match opt {
            Some(v) => format!("Some({})", format_value(v)),
            None => "None".to_string(),
        },
        ScriptValue::Map(entries) => {
            if entries.is_empty() {
                return "HashMap::new()".to_string();
            }
            let inner: Vec<String> = entries
                .iter()
                .map(|(k, v)| format!("(\"{}\", {})", k, format_value(v)))
                .collect();
            format!("HashMap::from([{}])", inner.join(", "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_roundtrip_serde() {
        let res = ResourceDef {
            name: "GameConfig".into(),
            fields: vec![
                ScriptField {
                    name: "gravity".into(),
                    value: ScriptValue::Float(-9.81),
                },
                ScriptField {
                    name: "max_players".into(),
                    value: ScriptValue::Int(4),
                },
            ],
        };
        let json = serde_json::to_string(&res).expect("serialize");
        let back: ResourceDef = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.name, "GameConfig");
        assert_eq!(back.fields.len(), 2);
    }

    #[test]
    fn resource_codegen() {
        let resources = vec![ResourceDef {
            name: "GameConfig".into(),
            fields: vec![
                ScriptField {
                    name: "gravity".into(),
                    value: ScriptValue::Float(-9.81),
                },
                ScriptField {
                    name: "debug".into(),
                    value: ScriptValue::Bool(true),
                },
            ],
        }];
        let code = generate_resource_code(&resources);
        assert!(code.contains("app.insert_resource(GameConfig"));
        assert!(code.contains("gravity:"));
        assert!(code.contains("debug: true"));
    }
}
