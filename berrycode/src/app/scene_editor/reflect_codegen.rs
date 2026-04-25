//! Generate Reflect boilerplate for user components.
//!
//! Scans `#[derive(Component)]` structs and generates a companion file with
//! `#[derive(Reflect)]` and a registration function that exposes component
//! fields to the editor via BRP (Bevy Remote Protocol).
//!
//! The user includes the generated module in their app build so the running
//! game can expose component fields to the inspector.

use super::script_scan::ScannedComponent;

/// Generate a Rust source file containing Reflect registrations for the
/// given set of scanned components. The output is meant to be written to
/// a `_reflect.rs` file alongside the scene code.
pub fn generate_reflect_code(components: &[ScannedComponent]) -> String {
    let mut code = String::new();
    code.push_str("//! Auto-generated Reflect registration for BerryCode editor integration.\n");
    code.push_str("//! Include this module in your app to enable live Inspector editing.\n");
    code.push_str("//!\n");
    code.push_str("//! Usage:\n");
    code.push_str("//!   1. Add `#[derive(Reflect)]` to each component listed below.\n");
    code.push_str("//!   2. Call `register_editor_types(&mut app)` in your plugin.\n\n");
    code.push_str("use bevy::prelude::*;\n\n");

    // Generate a documentation comment per component
    for comp in components {
        code.push_str(&format!("// --- {} ---\n", comp.name));
        if comp.fields.is_empty() {
            code.push_str(&format!(
                "// Unit / marker component. Add `#[derive(Reflect)]` to your definition.\n"
            ));
        } else {
            code.push_str("// Fields:\n");
            for f in &comp.fields {
                code.push_str(&format!("//   {}: {}\n", f.name, f.field_type));
            }
            code.push_str(&format!(
                "// To enable: add `#[derive(Reflect)]` to your `{}` definition.\n",
                comp.name
            ));
        }
        code.push('\n');
    }

    // Generate registration function
    code.push_str("/// Register all user component types for Reflect-based inspection.\n");
    code.push_str("///\n");
    code.push_str("/// Call this from your `App::build()` or plugin setup:\n");
    code.push_str("/// ```rust,ignore\n");
    code.push_str("/// app.add_plugins(register_editor_types);\n");
    code.push_str("/// ```\n");
    code.push_str("pub fn register_editor_types(app: &mut App) {\n");
    for comp in components {
        code.push_str(&format!("    app.register_type::<{}>();\n", comp.name));
    }
    code.push_str("}\n");

    code
}

/// Write the generated reflect code to a `_reflect.rs` file alongside the
/// scene file. Returns the path of the written file.
pub fn save_reflect_code(
    components: &[ScannedComponent],
    scene_path: &str,
) -> Result<String, String> {
    let code = generate_reflect_code(components);
    let rs_path = scene_path
        .strip_suffix(".bscene")
        .map(|s| format!("{}_reflect.rs", s))
        .unwrap_or_else(|| format!("{}_reflect.rs", scene_path));

    if let Some(parent) = std::path::Path::new(&rs_path).parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&rs_path, &code).map_err(|e| e.to_string())?;
    Ok(rs_path)
}

#[cfg(test)]
mod tests {
    use super::super::script_scan::{ScannedComponent, ScannedField};
    use super::*;

    #[test]
    fn generate_empty_components() {
        let code = generate_reflect_code(&[]);
        assert!(code.contains("register_editor_types"));
        assert!(code.contains("fn register_editor_types(app: &mut App)"));
    }

    #[test]
    fn generate_with_unit_component() {
        let comps = vec![ScannedComponent {
            name: "Marker".into(),
            fields: vec![],
            source_path: None,
        }];
        let code = generate_reflect_code(&comps);
        assert!(code.contains("Marker"));
        assert!(code.contains("app.register_type::<Marker>()"));
    }

    #[test]
    fn generate_with_fields() {
        let comps = vec![ScannedComponent {
            name: "Health".into(),
            fields: vec![
                ScannedField {
                    name: "value".into(),
                    field_type: "f32".into(),
                },
                ScannedField {
                    name: "max".into(),
                    field_type: "f32".into(),
                },
            ],
            source_path: None,
        }];
        let code = generate_reflect_code(&comps);
        assert!(code.contains("Health"));
        assert!(code.contains("value: f32"));
        assert!(code.contains("max: f32"));
        assert!(code.contains("app.register_type::<Health>()"));
    }

    #[test]
    fn generate_multiple_components() {
        let comps = vec![
            ScannedComponent {
                name: "Alpha".into(),
                fields: vec![],
                source_path: None,
            },
            ScannedComponent {
                name: "Beta".into(),
                fields: vec![ScannedField {
                    name: "speed".into(),
                    field_type: "f32".into(),
                }],
                source_path: None,
            },
        ];
        let code = generate_reflect_code(&comps);
        assert!(code.contains("app.register_type::<Alpha>()"));
        assert!(code.contains("app.register_type::<Beta>()"));
    }
}
