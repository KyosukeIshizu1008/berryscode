//! Code generation: converts a SceneModel to a Rust source file.
//!
//! When the user saves a scene, this module generates a `.rs` file alongside
//! the `.bscene` file that contains the equivalent Bevy setup code. This
//! provides bidirectional awareness: edit in the UI -> see the code; edit the
//! code -> re-import the scene.

use super::model::*;

/// Format a single ScriptValue to its Rust code representation.
fn format_script_value(val: &ScriptValue) -> String {
    match val {
        ScriptValue::Float(v) => format!("{:.6}", v),
        ScriptValue::Int(v) => format!("{}", v),
        ScriptValue::Bool(v) => format!("{}", v),
        ScriptValue::String(v) => format!("\"{}\"", v),
        ScriptValue::Vec(items) => format_vec_value(items),
        ScriptValue::Option(opt) => format_option_value(opt),
        ScriptValue::Map(entries) => format_map_value(entries),
    }
}

/// Format a `Vec<ScriptValue>` as `vec![val1, val2, ...]`.
fn format_vec_value(items: &[ScriptValue]) -> String {
    if items.is_empty() {
        return "vec![]".to_string();
    }
    let inner: Vec<String> = items.iter().map(format_script_value).collect();
    format!("vec![{}]", inner.join(", "))
}

/// Format an `Option<ScriptValue>` as `Some(val)` or `None`.
fn format_option_value(opt: &Option<Box<ScriptValue>>) -> String {
    match opt {
        Some(val) => format!("Some({})", format_script_value(val)),
        None => "None".to_string(),
    }
}

/// Format a map as `HashMap::from([("key", val), ...])`.
fn format_map_value(entries: &[(String, ScriptValue)]) -> String {
    if entries.is_empty() {
        return "HashMap::new()".to_string();
    }
    let inner: Vec<String> = entries
        .iter()
        .map(|(k, v)| format!("(\"{}\", {})", k, format_script_value(v)))
        .collect();
    format!("HashMap::from([{}])", inner.join(", "))
}

/// Generate a Rust source file from a SceneModel.
pub fn generate_scene_code(scene: &SceneModel) -> String {
    let mut code = String::new();
    code.push_str("//! Auto-generated scene setup code from BerryCode Scene Editor.\n");
    code.push_str("//! DO NOT EDIT MANUALLY -- changes will be overwritten on next save.\n");
    code.push_str("//! To modify, edit in BerryCode's Scene Editor and re-save.\n\n");
    code.push_str("use bevy::prelude::*;\n\n");

    code.push_str("pub fn setup_scene(\n");
    code.push_str("    mut commands: Commands,\n");
    code.push_str("    mut meshes: ResMut<Assets<Mesh>>,\n");
    code.push_str("    mut materials: ResMut<Assets<StandardMaterial>>,\n");
    code.push_str(") {\n");

    for entity in scene.entities.values() {
        if !entity.enabled {
            continue;
        }
        code.push_str(&format!("    // Entity: {}\n", entity.name));

        // Generate transform
        let t = &entity.transform;
        code.push_str(&format!(
            "    commands.spawn((\n        Transform::from_xyz({:.6}, {:.6}, {:.6})",
            t.translation[0], t.translation[1], t.translation[2]
        ));

        // Add rotation if non-zero
        if t.rotation_euler.iter().any(|&v| v.abs() > 0.001) {
            code.push_str(&format!(
                "\n            .with_rotation(Quat::from_euler(EulerRot::XYZ, {:.6}, {:.6}, {:.6}))",
                t.rotation_euler[0], t.rotation_euler[1], t.rotation_euler[2]
            ));
        }

        // Add scale if non-uniform
        if t.scale.iter().any(|&v| (v - 1.0).abs() > 0.001) {
            code.push_str(&format!(
                "\n            .with_scale(Vec3::new({:.6}, {:.6}, {:.6}))",
                t.scale[0], t.scale[1], t.scale[2]
            ));
        }
        code.push_str(",\n");

        // Generate components
        for component in &entity.components {
            match component {
                ComponentData::MeshCube { size, color, metallic, roughness, .. } => {
                    code.push_str(&format!(
                        "        Mesh3d(meshes.add(Cuboid::new({:.6}, {:.6}, {:.6}))),\n",
                        size, size, size
                    ));
                    code.push_str(&format!(
                        "        MeshMaterial3d(materials.add(StandardMaterial {{\n\
                         \x20           base_color: Color::srgb({:.6}, {:.6}, {:.6}),\n\
                         \x20           metallic: {:.6},\n\
                         \x20           perceptual_roughness: {:.6},\n\
                         \x20           ..default()\n\
                         \x20       }})),\n",
                        color[0], color[1], color[2], metallic, roughness
                    ));
                }
                ComponentData::MeshSphere { radius, color, metallic, roughness, .. } => {
                    code.push_str(&format!(
                        "        Mesh3d(meshes.add(Sphere::new({:.6}).mesh().uv(32, 16))),\n",
                        radius
                    ));
                    code.push_str(&format!(
                        "        MeshMaterial3d(materials.add(StandardMaterial {{\n\
                         \x20           base_color: Color::srgb({:.6}, {:.6}, {:.6}),\n\
                         \x20           metallic: {:.6},\n\
                         \x20           perceptual_roughness: {:.6},\n\
                         \x20           ..default()\n\
                         \x20       }})),\n",
                        color[0], color[1], color[2], metallic, roughness
                    ));
                }
                ComponentData::MeshPlane { size, color, .. } => {
                    code.push_str(&format!(
                        "        Mesh3d(meshes.add(Plane3d::default().mesh().size({:.6}, {:.6}))),\n",
                        size, size
                    ));
                    code.push_str(&format!(
                        "        MeshMaterial3d(materials.add(Color::srgb({:.6}, {:.6}, {:.6}))),\n",
                        color[0], color[1], color[2]
                    ));
                }
                ComponentData::Light { intensity, color } => {
                    code.push_str(&format!(
                        "        PointLight {{\n\
                         \x20           intensity: {:.6},\n\
                         \x20           color: Color::srgb({:.6}, {:.6}, {:.6}),\n\
                         \x20           ..default()\n\
                         \x20       }},\n",
                        intensity, color[0], color[1], color[2]
                    ));
                }
                ComponentData::DirectionalLight { intensity, color, shadows } => {
                    code.push_str(&format!(
                        "        DirectionalLight {{\n\
                         \x20           illuminance: {:.6},\n\
                         \x20           color: Color::srgb({:.6}, {:.6}, {:.6}),\n\
                         \x20           shadows_enabled: {},\n\
                         \x20           ..default()\n\
                         \x20       }},\n",
                        intensity, color[0], color[1], color[2], shadows
                    ));
                }
                ComponentData::Camera => {
                    code.push_str("        Camera3d::default(),\n");
                }
                ComponentData::CustomScript { type_name, fields } => {
                    if !type_name.is_empty() {
                        code.push_str(&format!("        {} {{\n", type_name));
                        for field in fields {
                            let val = match &field.value {
                                ScriptValue::Float(v) => format!("{:.6}", v),
                                ScriptValue::Int(v) => format!("{}", v),
                                ScriptValue::Bool(v) => format!("{}", v),
                                ScriptValue::String(v) => format!("\"{}\"", v),
                                ScriptValue::Vec(items) => format_vec_value(items),
                                ScriptValue::Option(opt) => format_option_value(opt),
                                ScriptValue::Map(entries) => format_map_value(entries),
                            };
                            code.push_str(&format!("            {}: {},\n", field.name, val));
                        }
                        code.push_str("        },\n");
                    }
                }
                _ => {
                    // Other components: add a comment
                    code.push_str(&format!(
                        "        // {}: (configure manually)\n",
                        component.label()
                    ));
                }
            }
        }

        code.push_str(&format!("        Name::new(\"{}\"),\n", entity.name));
        code.push_str("    ));\n\n");
    }

    code.push_str("}\n\n");

    // Generate resource insertion function if any resources are defined.
    if !scene.resources.is_empty() {
        code.push_str("pub fn insert_scene_resources(app: &mut App) {\n");
        code.push_str(&super::resource_editor::generate_resource_code(&scene.resources));
        code.push_str("}\n");
    }

    code
}

/// Save generated code alongside a scene file.
/// For "scenes/level1.bscene", generates "scenes/level1_scene.rs"
pub fn save_scene_code(scene: &SceneModel, scene_path: &str) -> Result<String, String> {
    let code = generate_scene_code(scene);
    let rs_path = scene_path
        .strip_suffix(".bscene")
        .map(|s| format!("{}_scene.rs", s))
        .unwrap_or_else(|| format!("{}.rs", scene_path));

    // Ensure the parent directory exists.
    if let Some(parent) = std::path::Path::new(&rs_path).parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    std::fs::write(&rs_path, &code).map_err(|e| e.to_string())?;
    Ok(rs_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_empty_scene() {
        let scene = SceneModel::new();
        let code = generate_scene_code(&scene);
        assert!(code.contains("fn setup_scene"));
        assert!(code.contains("Commands"));
    }

    #[test]
    fn generate_scene_with_cube() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "MyCube".into(),
            vec![ComponentData::MeshCube {
                size: 1.0,
                color: [0.5, 0.5, 1.0],
                metallic: 0.3,
                roughness: 0.5,
                emissive: [0.0, 0.0, 0.0],
                texture_path: None,
                normal_map_path: None,
            }],
        );
        let code = generate_scene_code(&scene);
        assert!(code.contains("Cuboid::new"));
        assert!(code.contains("MyCube"));
        assert!(code.contains("StandardMaterial"));
    }

    #[test]
    fn generate_scene_with_custom_script() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "Player".into(),
            vec![ComponentData::CustomScript {
                type_name: "PlayerStats".into(),
                fields: vec![
                    ScriptField {
                        name: "health".into(),
                        value: ScriptValue::Float(100.0),
                    },
                    ScriptField {
                        name: "speed".into(),
                        value: ScriptValue::Float(5.0),
                    },
                ],
            }],
        );
        let code = generate_scene_code(&scene);
        assert!(code.contains("PlayerStats"));
        assert!(code.contains("health: 100.000000"));
        assert!(code.contains("speed: 5.000000"));
    }

    #[test]
    fn generate_scene_with_light() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "Sun".into(),
            vec![ComponentData::DirectionalLight {
                intensity: 10000.0,
                color: [1.0, 1.0, 0.9],
                shadows: true,
            }],
        );
        let code = generate_scene_code(&scene);
        assert!(code.contains("DirectionalLight"));
        assert!(code.contains("shadows_enabled: true"));
    }

    #[test]
    fn disabled_entities_skipped() {
        let mut scene = SceneModel::new();
        let id = scene.add_entity("Hidden".into(), vec![]);
        if let Some(e) = scene.entities.get_mut(&id) {
            e.enabled = false;
        }
        let code = generate_scene_code(&scene);
        assert!(!code.contains("Hidden"));
    }

    #[test]
    fn generate_vec_field() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "VecEntity".into(),
            vec![ComponentData::CustomScript {
                type_name: "Inventory".into(),
                fields: vec![ScriptField {
                    name: "items".into(),
                    value: ScriptValue::Vec(vec![
                        ScriptValue::Float(1.0),
                        ScriptValue::Float(2.5),
                    ]),
                }],
            }],
        );
        let code = generate_scene_code(&scene);
        assert!(code.contains("vec![1.000000, 2.500000]"));
    }

    #[test]
    fn generate_option_some_field() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "OptEntity".into(),
            vec![ComponentData::CustomScript {
                type_name: "Config".into(),
                fields: vec![ScriptField {
                    name: "maybe".into(),
                    value: ScriptValue::Option(Some(Box::new(ScriptValue::Int(42)))),
                }],
            }],
        );
        let code = generate_scene_code(&scene);
        assert!(code.contains("Some(42)"));
    }

    #[test]
    fn generate_option_none_field() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "NoneEntity".into(),
            vec![ComponentData::CustomScript {
                type_name: "Config".into(),
                fields: vec![ScriptField {
                    name: "maybe".into(),
                    value: ScriptValue::Option(None),
                }],
            }],
        );
        let code = generate_scene_code(&scene);
        assert!(code.contains("maybe: None"));
    }

    #[test]
    fn generate_map_field() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "MapEntity".into(),
            vec![ComponentData::CustomScript {
                type_name: "Stats".into(),
                fields: vec![ScriptField {
                    name: "values".into(),
                    value: ScriptValue::Map(vec![
                        ("hp".into(), ScriptValue::Float(100.0)),
                        ("mp".into(), ScriptValue::Float(50.0)),
                    ]),
                }],
            }],
        );
        let code = generate_scene_code(&scene);
        assert!(code.contains("HashMap::from("));
        assert!(code.contains(r#""hp""#));
        assert!(code.contains(r#""mp""#));
    }

    #[test]
    fn precision_six_decimal_places() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "Precise".into(),
            vec![ComponentData::MeshCube {
                size: 1.123456,
                color: [0.123456, 0.654321, 0.999999],
                metallic: 0.111111,
                roughness: 0.222222,
                emissive: [0.0, 0.0, 0.0],
                texture_path: None,
                normal_map_path: None,
            }],
        );
        let code = generate_scene_code(&scene);
        assert!(code.contains("1.123456"));
        assert!(code.contains("0.123456"));
        assert!(code.contains("0.654321"));
    }
}
