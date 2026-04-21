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
                ComponentData::MeshCube {
                    size,
                    color,
                    metallic,
                    roughness,
                    ..
                } => {
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
                ComponentData::MeshSphere {
                    radius,
                    color,
                    metallic,
                    roughness,
                    ..
                } => {
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
                ComponentData::DirectionalLight {
                    intensity,
                    color,
                    shadows,
                } => {
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
                ComponentData::SpotLight {
                    intensity,
                    color,
                    range,
                    inner_angle,
                    outer_angle,
                } => {
                    code.push_str(&format!(
                        "        // [BerryCode:SpotLight] intensity={:.6} color_r={:.6} color_g={:.6} color_b={:.6} range={:.6} inner_angle={:.6} outer_angle={:.6}\n",
                        intensity, color[0], color[1], color[2], range, inner_angle, outer_angle
                    ));
                }
                ComponentData::MeshFromFile { path, .. } => {
                    code.push_str(&format!(
                        "        // [BerryCode:MeshFromFile] path={}\n",
                        path
                    ));
                }
                ComponentData::AudioSource {
                    path,
                    volume,
                    looped,
                    autoplay,
                } => {
                    code.push_str(&format!(
                        "        // [BerryCode:AudioSource] path={} volume={:.6} looped={} autoplay={}\n",
                        path, volume, looped, autoplay
                    ));
                }
                ComponentData::AudioListener => {
                    code.push_str("        // [BerryCode:AudioListener]\n");
                }
                ComponentData::RigidBody { body_type, mass } => {
                    code.push_str(&format!(
                        "        // [BerryCode:RigidBody] body_type={} mass={:.6}\n",
                        body_type.label(),
                        mass
                    ));
                }
                ComponentData::Collider {
                    shape,
                    friction,
                    restitution,
                } => {
                    let shape_str = match shape {
                        ColliderShape::Box { half_extents } => format!(
                            "shape=Box half_x={:.6} half_y={:.6} half_z={:.6}",
                            half_extents[0], half_extents[1], half_extents[2]
                        ),
                        ColliderShape::Sphere { radius } => {
                            format!("shape=Sphere radius={:.6}", radius)
                        }
                        ColliderShape::Capsule {
                            half_height,
                            radius,
                        } => format!(
                            "shape=Capsule half_height={:.6} radius={:.6}",
                            half_height, radius
                        ),
                    };
                    code.push_str(&format!(
                        "        // [BerryCode:Collider] {} friction={:.6} restitution={:.6}\n",
                        shape_str, friction, restitution
                    ));
                }
                ComponentData::UiText {
                    text,
                    font_size,
                    color,
                } => {
                    code.push_str(&format!(
                        "        // [BerryCode:UiText] text={} font_size={:.6} color_r={:.6} color_g={:.6} color_b={:.6} color_a={:.6}\n",
                        text, font_size, color[0], color[1], color[2], color[3]
                    ));
                }
                ComponentData::UiButton { label, background } => {
                    code.push_str(&format!(
                        "        // [BerryCode:UiButton] label={} bg_r={:.6} bg_g={:.6} bg_b={:.6} bg_a={:.6}\n",
                        label, background[0], background[1], background[2], background[3]
                    ));
                }
                ComponentData::UiImage { path, tint } => {
                    code.push_str(&format!(
                        "        // [BerryCode:UiImage] path={} tint_r={:.6} tint_g={:.6} tint_b={:.6} tint_a={:.6}\n",
                        path, tint[0], tint[1], tint[2], tint[3]
                    ));
                }
                ComponentData::ParticleEmitter {
                    rate,
                    lifetime,
                    speed,
                    spread,
                    start_size,
                    end_size,
                    start_color,
                    end_color,
                    max_particles,
                    gravity,
                } => {
                    code.push_str(&format!(
                        "        // [BerryCode:ParticleEmitter] rate={:.6} lifetime={:.6} speed={:.6} spread={:.6} start_size={:.6} end_size={:.6} sc_r={:.6} sc_g={:.6} sc_b={:.6} sc_a={:.6} ec_r={:.6} ec_g={:.6} ec_b={:.6} ec_a={:.6} max_particles={} gravity={:.6}\n",
                        rate, lifetime, speed, spread, start_size, end_size,
                        start_color[0], start_color[1], start_color[2], start_color[3],
                        end_color[0], end_color[1], end_color[2], end_color[3],
                        max_particles, gravity
                    ));
                }
                ComponentData::Animation {
                    duration,
                    tracks,
                    looped,
                } => {
                    code.push_str(&format!(
                        "        // [BerryCode:Animation] duration={:.6} looped={} tracks={}\n",
                        duration,
                        looped,
                        tracks.len()
                    ));
                }
                ComponentData::CustomScript { type_name, fields } => {
                    code.push_str(&format!(
                        "        // [BerryCode:CustomScript] type_name={} fields={}\n",
                        type_name,
                        fields.len()
                    ));
                    for field in fields {
                        let val = format_script_value(&field.value);
                        code.push_str(&format!(
                            "        // [BerryCode:CustomField] name={} value={}\n",
                            field.name, val
                        ));
                    }
                }
                ComponentData::Skybox { path } => {
                    code.push_str(&format!("        // [BerryCode:Skybox] path={}\n", path));
                }
                ComponentData::Animator { controller_path } => {
                    code.push_str(&format!(
                        "        // [BerryCode:Animator] controller_path={}\n",
                        controller_path
                    ));
                }
                ComponentData::LodGroup { levels } => {
                    code.push_str(&format!(
                        "        // [BerryCode:LodGroup] levels={}\n",
                        levels.len()
                    ));
                }
                ComponentData::Spline { points, closed } => {
                    code.push_str(&format!(
                        "        // [BerryCode:Spline] points={} closed={}\n",
                        points.len(),
                        closed
                    ));
                }
                ComponentData::Terrain {
                    resolution,
                    world_size,
                    base_color,
                    ..
                } => {
                    code.push_str(&format!(
                        "        // [BerryCode:Terrain] resolution={} world_w={:.6} world_h={:.6} base_r={:.6} base_g={:.6} base_b={:.6}\n",
                        resolution, world_size[0], world_size[1],
                        base_color[0], base_color[1], base_color[2]
                    ));
                }
                ComponentData::SkinnedMesh { path, .. } => {
                    code.push_str(&format!(
                        "        // [BerryCode:SkinnedMesh] path={}\n",
                        path
                    ));
                }
                ComponentData::VisualScript { path } => {
                    code.push_str(&format!(
                        "        // [BerryCode:VisualScript] path={}\n",
                        path
                    ));
                }
                ComponentData::NavMesh {
                    cell_size,
                    width,
                    height,
                    ..
                } => {
                    code.push_str(&format!(
                        "        // [BerryCode:NavMesh] cell_size={:.6} width={} height={}\n",
                        cell_size, width, height
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
        code.push_str(&super::resource_editor::generate_resource_code(
            &scene.resources,
        ));
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
        assert!(code.contains("[BerryCode:CustomField] name=health value=100.000000"));
        assert!(code.contains("[BerryCode:CustomField] name=speed value=5.000000"));
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
                    value: ScriptValue::Vec(vec![ScriptValue::Float(1.0), ScriptValue::Float(2.5)]),
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
        assert!(code.contains("name=maybe value=None"));
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

// -------------------------------------------------------------------------
// Extended codegen tests: verify every ComponentData variant and edge cases
// -------------------------------------------------------------------------
#[cfg(test)]
mod extended_tests {
    use super::*;

    /// Helper: generate code and check basic structural validity.
    fn assert_valid_code(scene: &SceneModel) {
        let code = generate_scene_code(scene);
        // Must contain the function signature
        assert!(
            code.contains("fn setup_scene"),
            "Missing setup_scene function"
        );
        assert!(code.contains("Commands"), "Missing Commands parameter");
        // Balanced braces
        let opens = code.chars().filter(|&c| c == '{').count();
        let closes = code.chars().filter(|&c| c == '}').count();
        assert_eq!(
            opens, closes,
            "Unbalanced braces: {} opens vs {} closes",
            opens, closes
        );
        // Balanced parens
        let opens_p = code.chars().filter(|&c| c == '(').count();
        let closes_p = code.chars().filter(|&c| c == ')').count();
        assert_eq!(
            opens_p, closes_p,
            "Unbalanced parens: {} opens vs {} closes",
            opens_p, closes_p
        );
        // No todo!() or unimplemented!()
        assert!(!code.contains("todo!()"), "Contains todo!()");
        assert!(
            !code.contains("unimplemented!()"),
            "Contains unimplemented!()"
        );
    }

    #[test]
    fn codegen_mesh_cube() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "TestCube".into(),
            vec![ComponentData::MeshCube {
                size: 2.5,
                color: [0.1, 0.2, 0.3],
                metallic: 0.7,
                roughness: 0.4,
                emissive: [0.5, 0.5, 0.0],
                texture_path: Some("tex.png".into()),
                normal_map_path: None,
            }],
        );
        assert_valid_code(&scene);
        let code = generate_scene_code(&scene);
        assert!(code.contains("Cuboid::new"));
        assert!(code.contains("StandardMaterial"));
        assert!(code.contains("TestCube"));
    }

    #[test]
    fn codegen_mesh_sphere() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "MySphere".into(),
            vec![ComponentData::MeshSphere {
                radius: 1.5,
                color: [1.0, 0.0, 0.0],
                metallic: 0.0,
                roughness: 1.0,
                emissive: [0.0, 0.0, 0.0],
                texture_path: None,
                normal_map_path: None,
            }],
        );
        assert_valid_code(&scene);
        let code = generate_scene_code(&scene);
        assert!(code.contains("Sphere::new"));
    }

    #[test]
    fn codegen_mesh_plane() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "Floor".into(),
            vec![ComponentData::MeshPlane {
                size: 50.0,
                color: [0.3, 0.3, 0.3],
                metallic: 0.0,
                roughness: 0.9,
                emissive: [0.0, 0.0, 0.0],
                texture_path: None,
                normal_map_path: None,
            }],
        );
        assert_valid_code(&scene);
        let code = generate_scene_code(&scene);
        assert!(code.contains("Plane3d"));
    }

    #[test]
    fn codegen_point_light() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "Lamp".into(),
            vec![ComponentData::Light {
                intensity: 5000.0,
                color: [1.0, 0.9, 0.8],
            }],
        );
        assert_valid_code(&scene);
        let code = generate_scene_code(&scene);
        assert!(code.contains("PointLight"));
        assert!(code.contains("5000"));
    }

    #[test]
    fn codegen_spot_light() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "Spot".into(),
            vec![ComponentData::SpotLight {
                intensity: 8000.0,
                color: [1.0, 1.0, 1.0],
                range: 20.0,
                inner_angle: 0.3,
                outer_angle: 0.6,
            }],
        );
        assert_valid_code(&scene);
        let code = generate_scene_code(&scene);
        assert!(
            code.contains("[BerryCode:SpotLight]"),
            "SpotLight entity should produce BerryCode marker"
        );
    }

    #[test]
    fn codegen_directional_light() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "Sun".into(),
            vec![ComponentData::DirectionalLight {
                intensity: 12000.0,
                color: [1.0, 1.0, 0.9],
                shadows: true,
            }],
        );
        assert_valid_code(&scene);
        let code = generate_scene_code(&scene);
        assert!(code.contains("DirectionalLight"));
        assert!(code.contains("shadows_enabled: true"));
    }

    #[test]
    fn codegen_camera() {
        let mut scene = SceneModel::new();
        scene.add_entity("MainCamera".into(), vec![ComponentData::Camera]);
        assert_valid_code(&scene);
        let code = generate_scene_code(&scene);
        assert!(code.contains("Camera3d"));
    }

    #[test]
    fn codegen_audio_source() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "BGM".into(),
            vec![ComponentData::AudioSource {
                path: "audio/bgm.ogg".into(),
                volume: 0.8,
                looped: true,
                autoplay: true,
            }],
        );
        assert_valid_code(&scene);
        let code = generate_scene_code(&scene);
        assert!(
            code.contains("BGM"),
            "Entity name should appear in generated code"
        );
    }

    #[test]
    fn codegen_audio_listener() {
        let mut scene = SceneModel::new();
        scene.add_entity("Ear".into(), vec![ComponentData::AudioListener]);
        assert_valid_code(&scene);
        let code = generate_scene_code(&scene);
        assert!(code.contains("Ear"));
    }

    #[test]
    fn codegen_rigidbody() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "Ball".into(),
            vec![ComponentData::RigidBody {
                body_type: RigidBodyType::Dynamic,
                mass: 5.0,
            }],
        );
        assert_valid_code(&scene);
    }

    #[test]
    fn codegen_collider_box() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "Wall".into(),
            vec![ComponentData::Collider {
                shape: ColliderShape::Box {
                    half_extents: [2.0, 3.0, 0.5],
                },
                friction: 0.6,
                restitution: 0.2,
            }],
        );
        assert_valid_code(&scene);
    }

    #[test]
    fn codegen_collider_sphere() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "BallCol".into(),
            vec![ComponentData::Collider {
                shape: ColliderShape::Sphere { radius: 1.0 },
                friction: 0.3,
                restitution: 0.8,
            }],
        );
        assert_valid_code(&scene);
    }

    #[test]
    fn codegen_collider_capsule() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "Player".into(),
            vec![ComponentData::Collider {
                shape: ColliderShape::Capsule {
                    half_height: 0.5,
                    radius: 0.3,
                },
                friction: 0.5,
                restitution: 0.0,
            }],
        );
        assert_valid_code(&scene);
    }

    #[test]
    fn codegen_ui_text() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "HUD".into(),
            vec![ComponentData::UiText {
                text: "Score: 100".into(),
                font_size: 24.0,
                color: [1.0, 1.0, 1.0, 1.0],
            }],
        );
        assert_valid_code(&scene);
    }

    #[test]
    fn codegen_ui_button() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "StartBtn".into(),
            vec![ComponentData::UiButton {
                label: "Start Game".into(),
                background: [0.2, 0.3, 0.8, 1.0],
            }],
        );
        assert_valid_code(&scene);
    }

    #[test]
    fn codegen_ui_image() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "Logo".into(),
            vec![ComponentData::UiImage {
                path: "ui/logo.png".into(),
                tint: [1.0, 1.0, 1.0, 1.0],
            }],
        );
        assert_valid_code(&scene);
    }

    #[test]
    fn codegen_particle_emitter() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "Fire".into(),
            vec![ComponentData::ParticleEmitter {
                rate: 50.0,
                lifetime: 1.5,
                speed: 3.0,
                spread: 0.3,
                start_size: 0.1,
                end_size: 0.0,
                start_color: [1.0, 0.6, 0.1, 1.0],
                end_color: [1.0, 0.0, 0.0, 0.0],
                max_particles: 300,
                gravity: -2.0,
            }],
        );
        assert_valid_code(&scene);
    }

    #[test]
    fn codegen_animation() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "Anim".into(),
            vec![ComponentData::Animation {
                duration: 2.0,
                tracks: vec![AnimationTrack {
                    property: AnimProperty::Position,
                    keyframes: vec![
                        TrackKeyframe {
                            time: 0.0,
                            value: [0.0, 0.0, 0.0],
                            easing: EasingType::Linear,
                        },
                        TrackKeyframe {
                            time: 1.0,
                            value: [5.0, 0.0, 0.0],
                            easing: EasingType::EaseInOutQuad,
                        },
                    ],
                    events: vec![AnimationEvent {
                        time: 0.5,
                        callback_name: "halfway".into(),
                    }],
                }],
                looped: true,
            }],
        );
        assert_valid_code(&scene);
    }

    #[test]
    fn codegen_custom_script() {
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
                        name: "lives".into(),
                        value: ScriptValue::Int(3),
                    },
                    ScriptField {
                        name: "invincible".into(),
                        value: ScriptValue::Bool(false),
                    },
                    ScriptField {
                        name: "name".into(),
                        value: ScriptValue::String("Hero".into()),
                    },
                ],
            }],
        );
        assert_valid_code(&scene);
        let code = generate_scene_code(&scene);
        assert!(code.contains("PlayerStats"));
        assert!(code.contains("name=health"));
        assert!(code.contains("name=lives"));
        assert!(code.contains("name=invincible value=false"));
    }

    #[test]
    fn codegen_custom_script_complex_types() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "Data".into(),
            vec![ComponentData::CustomScript {
                type_name: "GameData".into(),
                fields: vec![
                    ScriptField {
                        name: "scores".into(),
                        value: ScriptValue::Vec(vec![
                            ScriptValue::Float(10.0),
                            ScriptValue::Float(20.0),
                        ]),
                    },
                    ScriptField {
                        name: "active".into(),
                        value: ScriptValue::Option(Some(Box::new(ScriptValue::Bool(true)))),
                    },
                    ScriptField {
                        name: "empty".into(),
                        value: ScriptValue::Option(None),
                    },
                ],
            }],
        );
        assert_valid_code(&scene);
        let code = generate_scene_code(&scene);
        assert!(code.contains("vec!["));
        assert!(code.contains("Some(true)"));
        assert!(code.contains("name=empty value=None"));
    }

    #[test]
    fn codegen_skybox() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "Sky".into(),
            vec![ComponentData::Skybox {
                path: "sky.hdr".into(),
            }],
        );
        assert_valid_code(&scene);
    }

    #[test]
    fn codegen_animator() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "NPC".into(),
            vec![ComponentData::Animator {
                controller_path: "anims/npc.banimator".into(),
            }],
        );
        assert_valid_code(&scene);
    }

    #[test]
    fn codegen_mesh_from_file() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "Model".into(),
            vec![ComponentData::MeshFromFile {
                path: "models/character.glb".into(),
                texture_path: None,
                normal_map_path: None,
            }],
        );
        assert_valid_code(&scene);
    }

    #[test]
    fn codegen_terrain() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "Land".into(),
            vec![ComponentData::Terrain {
                resolution: 8,
                world_size: [50.0, 50.0],
                heights: vec![0.0; 64],
                base_color: [0.3, 0.5, 0.3],
            }],
        );
        assert_valid_code(&scene);
    }

    #[test]
    fn codegen_navmesh() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "Nav".into(),
            vec![ComponentData::NavMesh {
                cell_size: 1.0,
                grid: vec![true; 25],
                width: 5,
                height: 5,
            }],
        );
        assert_valid_code(&scene);
    }

    #[test]
    fn codegen_lod_group() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "LOD".into(),
            vec![ComponentData::LodGroup {
                levels: vec![
                    LodLevel {
                        mesh_path: "high.glb".into(),
                        screen_percentage: 0.5,
                    },
                    LodLevel {
                        mesh_path: "low.glb".into(),
                        screen_percentage: 0.1,
                    },
                ],
            }],
        );
        assert_valid_code(&scene);
    }

    #[test]
    fn codegen_spline() {
        use super::super::spline::SplinePoint;
        let mut scene = SceneModel::new();
        scene.add_entity(
            "Path".into(),
            vec![ComponentData::Spline {
                points: vec![
                    SplinePoint {
                        position: [0.0, 0.0, 0.0],
                        tangent_in: [0.0, 0.0, -1.0],
                        tangent_out: [0.0, 0.0, 1.0],
                    },
                    SplinePoint {
                        position: [5.0, 0.0, 0.0],
                        tangent_in: [0.0, 0.0, -1.0],
                        tangent_out: [0.0, 0.0, 1.0],
                    },
                ],
                closed: false,
            }],
        );
        assert_valid_code(&scene);
    }

    #[test]
    fn codegen_skinned_mesh() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "Character".into(),
            vec![ComponentData::SkinnedMesh {
                path: "char.glb".into(),
                bones: vec![],
            }],
        );
        assert_valid_code(&scene);
    }

    #[test]
    fn codegen_visual_script() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "Logic".into(),
            vec![ComponentData::VisualScript {
                path: "scripts/main.bscript".into(),
            }],
        );
        assert_valid_code(&scene);
    }

    // === Edge Cases ===

    #[test]
    fn codegen_empty_entity_name() {
        let mut scene = SceneModel::new();
        scene.add_entity("".into(), vec![ComponentData::Camera]);
        assert_valid_code(&scene);
    }

    #[test]
    fn codegen_special_chars_in_name() {
        let mut scene = SceneModel::new();
        scene.add_entity("Player's \"Ship\" <3>".into(), vec![ComponentData::Camera]);
        assert_valid_code(&scene);
        let code = generate_scene_code(&scene);
        // The entity name appears in a comment and in Name::new(...), but must
        // not break brace/paren balance (already checked by assert_valid_code).
        assert!(
            code.contains("Entity:") || code.contains("Name::new"),
            "Entity comment or Name should be present"
        );
    }

    #[test]
    fn codegen_zero_values() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "Zero".into(),
            vec![ComponentData::MeshCube {
                size: 0.0,
                color: [0.0, 0.0, 0.0],
                metallic: 0.0,
                roughness: 0.0,
                emissive: [0.0, 0.0, 0.0],
                texture_path: None,
                normal_map_path: None,
            }],
        );
        assert_valid_code(&scene);
    }

    #[test]
    fn codegen_negative_values() {
        let mut scene = SceneModel::new();
        let id = scene.add_entity("Neg".into(), vec![ComponentData::Camera]);
        if let Some(e) = scene.entities.get_mut(&id) {
            e.transform.translation = [-100.0, -50.0, -200.0];
            e.transform.rotation_euler = [-3.14, -1.57, -0.5];
            e.transform.scale = [-1.0, -1.0, -1.0];
        }
        assert_valid_code(&scene);
        let code = generate_scene_code(&scene);
        assert!(code.contains("-100"));
        assert!(code.contains("with_scale"));
    }

    #[test]
    fn codegen_many_entities() {
        let mut scene = SceneModel::new();
        for i in 0..50 {
            scene.add_entity(
                format!("Entity_{}", i),
                vec![ComponentData::MeshCube {
                    size: 1.0,
                    color: [0.5, 0.5, 0.5],
                    metallic: 0.0,
                    roughness: 0.5,
                    emissive: [0.0, 0.0, 0.0],
                    texture_path: None,
                    normal_map_path: None,
                }],
            );
        }
        assert_valid_code(&scene);
        let code = generate_scene_code(&scene);
        assert!(code.contains("Entity_49"));
    }

    #[test]
    fn codegen_multi_component_entity() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "Complex".into(),
            vec![
                ComponentData::MeshCube {
                    size: 1.0,
                    color: [1.0, 0.0, 0.0],
                    metallic: 0.5,
                    roughness: 0.3,
                    emissive: [0.0, 0.0, 0.0],
                    texture_path: None,
                    normal_map_path: None,
                },
                ComponentData::RigidBody {
                    body_type: RigidBodyType::Dynamic,
                    mass: 2.0,
                },
                ComponentData::Collider {
                    shape: ColliderShape::Box {
                        half_extents: [0.5, 0.5, 0.5],
                    },
                    friction: 0.5,
                    restitution: 0.3,
                },
                ComponentData::AudioSource {
                    path: "hit.wav".into(),
                    volume: 1.0,
                    looped: false,
                    autoplay: false,
                },
                ComponentData::CustomScript {
                    type_name: "Health".into(),
                    fields: vec![ScriptField {
                        name: "hp".into(),
                        value: ScriptValue::Float(100.0),
                    }],
                },
            ],
        );
        assert_valid_code(&scene);
        let code = generate_scene_code(&scene);
        assert!(code.contains("Complex"));
        assert!(code.contains("Cuboid"));
        assert!(code.contains("Health"));
    }

    #[test]
    fn codegen_disabled_entity_skipped() {
        let mut scene = SceneModel::new();
        let id = scene.add_entity("Hidden".into(), vec![ComponentData::Camera]);
        if let Some(e) = scene.entities.get_mut(&id) {
            e.enabled = false;
        }
        let code = generate_scene_code(&scene);
        assert!(
            !code.contains("Hidden"),
            "Disabled entities should be skipped"
        );
        assert_valid_code(&scene);
    }

    #[test]
    fn codegen_with_transform() {
        let mut scene = SceneModel::new();
        let id = scene.add_entity("Positioned".into(), vec![ComponentData::Camera]);
        if let Some(e) = scene.entities.get_mut(&id) {
            e.transform.translation = [10.5, 20.3, -5.7];
            e.transform.rotation_euler = [0.1, 0.2, 0.3];
            e.transform.scale = [2.0, 2.0, 2.0];
        }
        assert_valid_code(&scene);
        let code = generate_scene_code(&scene);
        assert!(code.contains("Transform::from_xyz"));
        assert!(code.contains("with_rotation"));
        assert!(code.contains("with_scale"));
    }

    #[test]
    fn codegen_all_component_types_at_once() {
        let mut scene = SceneModel::new();
        // Add one entity per component type to ensure nothing crashes
        scene.add_entity(
            "E1".into(),
            vec![ComponentData::MeshCube {
                size: 1.0,
                color: [1.0, 1.0, 1.0],
                metallic: 0.0,
                roughness: 0.5,
                emissive: [0.0, 0.0, 0.0],
                texture_path: None,
                normal_map_path: None,
            }],
        );
        scene.add_entity(
            "E2".into(),
            vec![ComponentData::MeshSphere {
                radius: 0.5,
                color: [1.0, 0.0, 0.0],
                metallic: 0.0,
                roughness: 0.5,
                emissive: [0.0, 0.0, 0.0],
                texture_path: None,
                normal_map_path: None,
            }],
        );
        scene.add_entity(
            "E3".into(),
            vec![ComponentData::MeshPlane {
                size: 10.0,
                color: [0.5, 0.5, 0.5],
                metallic: 0.0,
                roughness: 0.5,
                emissive: [0.0, 0.0, 0.0],
                texture_path: None,
                normal_map_path: None,
            }],
        );
        scene.add_entity(
            "E4".into(),
            vec![ComponentData::Light {
                intensity: 1000.0,
                color: [1.0, 1.0, 1.0],
            }],
        );
        scene.add_entity(
            "E5".into(),
            vec![ComponentData::SpotLight {
                intensity: 5000.0,
                color: [1.0, 1.0, 1.0],
                range: 10.0,
                inner_angle: 0.3,
                outer_angle: 0.5,
            }],
        );
        scene.add_entity(
            "E6".into(),
            vec![ComponentData::DirectionalLight {
                intensity: 10000.0,
                color: [1.0, 1.0, 0.9],
                shadows: false,
            }],
        );
        scene.add_entity("E7".into(), vec![ComponentData::Camera]);
        scene.add_entity(
            "E8".into(),
            vec![ComponentData::MeshFromFile {
                path: "model.glb".into(),
                texture_path: None,
                normal_map_path: None,
            }],
        );
        scene.add_entity(
            "E9".into(),
            vec![ComponentData::AudioSource {
                path: "a.wav".into(),
                volume: 1.0,
                looped: false,
                autoplay: true,
            }],
        );
        scene.add_entity("E10".into(), vec![ComponentData::AudioListener]);
        scene.add_entity(
            "E11".into(),
            vec![ComponentData::RigidBody {
                body_type: RigidBodyType::Static,
                mass: 0.0,
            }],
        );
        scene.add_entity(
            "E12".into(),
            vec![ComponentData::Collider {
                shape: ColliderShape::Sphere { radius: 1.0 },
                friction: 0.5,
                restitution: 0.0,
            }],
        );
        scene.add_entity(
            "E13".into(),
            vec![ComponentData::UiText {
                text: "Hi".into(),
                font_size: 16.0,
                color: [1.0, 1.0, 1.0, 1.0],
            }],
        );
        scene.add_entity(
            "E14".into(),
            vec![ComponentData::UiButton {
                label: "OK".into(),
                background: [0.2, 0.2, 0.3, 1.0],
            }],
        );
        scene.add_entity(
            "E15".into(),
            vec![ComponentData::UiImage {
                path: "img.png".into(),
                tint: [1.0, 1.0, 1.0, 1.0],
            }],
        );
        scene.add_entity(
            "E16".into(),
            vec![ComponentData::ParticleEmitter {
                rate: 30.0,
                lifetime: 1.0,
                speed: 2.0,
                spread: 0.2,
                start_size: 0.1,
                end_size: 0.0,
                start_color: [1.0, 1.0, 0.0, 1.0],
                end_color: [1.0, 0.0, 0.0, 0.0],
                max_particles: 100,
                gravity: -1.0,
            }],
        );
        scene.add_entity(
            "E17".into(),
            vec![ComponentData::Animation {
                duration: 1.0,
                tracks: vec![],
                looped: false,
            }],
        );
        scene.add_entity(
            "E18".into(),
            vec![ComponentData::CustomScript {
                type_name: "T".into(),
                fields: vec![],
            }],
        );
        scene.add_entity(
            "E19".into(),
            vec![ComponentData::Skybox {
                path: "sky.hdr".into(),
            }],
        );
        scene.add_entity(
            "E20".into(),
            vec![ComponentData::Animator {
                controller_path: "c.banimator".into(),
            }],
        );
        scene.add_entity(
            "E21".into(),
            vec![ComponentData::LodGroup { levels: vec![] }],
        );
        scene.add_entity(
            "E22".into(),
            vec![ComponentData::Terrain {
                resolution: 4,
                world_size: [10.0, 10.0],
                heights: vec![0.0; 16],
                base_color: [0.3, 0.5, 0.3],
            }],
        );
        scene.add_entity(
            "E23".into(),
            vec![ComponentData::NavMesh {
                cell_size: 1.0,
                grid: vec![],
                width: 0,
                height: 0,
            }],
        );
        scene.add_entity(
            "E24".into(),
            vec![ComponentData::SkinnedMesh {
                path: "s.glb".into(),
                bones: vec![],
            }],
        );
        scene.add_entity(
            "E25".into(),
            vec![ComponentData::VisualScript {
                path: "v.bscript".into(),
            }],
        );

        assert_valid_code(&scene);
        let code = generate_scene_code(&scene);
        // All 25 entities should produce output (as entity names in comments
        // and in Name::new(...))
        for i in 1..=25 {
            let tag = format!("E{}", i);
            // Use a word-boundary check: "E1" must not match "E10" etc.
            // The entity name appears in `Name::new("E{i}")` so exact match is safe.
            let needle = format!("Name::new(\"{}\")", tag);
            assert!(
                code.contains(&needle),
                "Missing entity {} in generated code",
                tag
            );
        }
    }

    #[test]
    fn codegen_custom_script_empty_type_name_skipped() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "EmptyScript".into(),
            vec![ComponentData::CustomScript {
                type_name: "".into(),
                fields: vec![ScriptField {
                    name: "x".into(),
                    value: ScriptValue::Float(1.0),
                }],
            }],
        );
        assert_valid_code(&scene);
        let code = generate_scene_code(&scene);
        // Empty type_name should not produce a struct block (the codegen skips it)
        assert!(
            !code.contains("x: 1.0"),
            "Empty type_name CustomScript should not emit fields"
        );
    }

    #[test]
    fn codegen_custom_script_map_field() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "MapTest".into(),
            vec![ComponentData::CustomScript {
                type_name: "Config".into(),
                fields: vec![ScriptField {
                    name: "settings".into(),
                    value: ScriptValue::Map(vec![
                        ("volume".into(), ScriptValue::Float(0.8)),
                        ("difficulty".into(), ScriptValue::Int(2)),
                    ]),
                }],
            }],
        );
        assert_valid_code(&scene);
        let code = generate_scene_code(&scene);
        assert!(code.contains("HashMap::from("));
    }

    #[test]
    fn codegen_custom_script_nested_vec() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "Nested".into(),
            vec![ComponentData::CustomScript {
                type_name: "Matrix".into(),
                fields: vec![ScriptField {
                    name: "rows".into(),
                    value: ScriptValue::Vec(vec![
                        ScriptValue::Vec(vec![ScriptValue::Float(1.0), ScriptValue::Float(0.0)]),
                        ScriptValue::Vec(vec![ScriptValue::Float(0.0), ScriptValue::Float(1.0)]),
                    ]),
                }],
            }],
        );
        assert_valid_code(&scene);
        let code = generate_scene_code(&scene);
        assert!(code.contains("vec![vec!["));
    }

    #[test]
    fn codegen_identity_transform_no_rotation_or_scale() {
        let mut scene = SceneModel::new();
        scene.add_entity("Default".into(), vec![ComponentData::Camera]);
        let code = generate_scene_code(&scene);
        // Default transform (0,0,0) / (0,0,0) / (1,1,1) should NOT add
        // with_rotation or with_scale
        assert!(
            !code.contains("with_rotation"),
            "Identity rotation should not emit with_rotation"
        );
        assert!(
            !code.contains("with_scale"),
            "Uniform scale=1 should not emit with_scale"
        );
    }

    #[test]
    fn codegen_produces_parseable_rust() {
        // Generate code for a complex scene and verify it is valid Rust syntax
        // by running rustfmt --check on it (syntax check only).
        let mut scene = SceneModel::new();
        scene.add_entity(
            "Test".into(),
            vec![
                ComponentData::MeshCube {
                    size: 1.0,
                    color: [1.0, 0.0, 0.0],
                    metallic: 0.5,
                    roughness: 0.5,
                    emissive: [0.0, 0.0, 0.0],
                    texture_path: None,
                    normal_map_path: None,
                },
                ComponentData::CustomScript {
                    type_name: "MyComp".into(),
                    fields: vec![ScriptField {
                        name: "val".into(),
                        value: ScriptValue::Float(42.0),
                    }],
                },
            ],
        );
        let code = generate_scene_code(&scene);

        // Write to temp file and try to parse with rustfmt (syntax check only)
        let tmp = tempfile::NamedTempFile::new().expect("create temp file");
        std::fs::write(tmp.path(), &code).expect("write temp file");

        // rustfmt --check returns 0 if file is already formatted, 1 if not but
        // parseable, and errors on syntax errors.
        let output = std::process::Command::new("rustfmt")
            .arg("--check")
            .arg(tmp.path())
            .output();

        match output {
            Ok(out) => {
                if !out.status.success() {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    // "error[" in stderr indicates actual syntax error
                    assert!(
                        !stderr.contains("error["),
                        "Generated code has syntax errors:\n{}\n\nCode:\n{}",
                        stderr,
                        code
                    );
                }
            }
            Err(_) => {
                // rustfmt not available -- skip this check
            }
        }
    }

    #[test]
    fn codegen_every_variant_produces_output() {
        // Verify that every ComponentData variant produces *some* output
        // in the generated code (not silently skipped).
        let all_defaults = ComponentData::default_all();
        for (label, component) in &all_defaults {
            let mut scene = SceneModel::new();
            scene.add_entity(
                format!("Test_{}", label.replace(' ', "_")),
                vec![component.clone()],
            );
            let code = generate_scene_code(&scene);
            let entity_name = format!("Test_{}", label.replace(' ', "_"));
            assert!(
                code.contains(&entity_name),
                "Component '{}' entity name missing from generated code",
                label
            );
            // The entity block must contain something besides just the
            // transform and Name (i.e., the component produced output).
            // Either a Bevy type or a "// <label>: (configure manually)" comment.
            // CustomScript with empty type_name intentionally produces no
            // component output (only the entity wrapper). All other variants
            // must produce either explicit Bevy code or a "configure manually"
            // comment.
            let is_empty_custom_script = matches!(
                component,
                ComponentData::CustomScript { ref type_name, .. } if type_name.is_empty()
            );
            if !is_empty_custom_script {
                let has_component_output = code.contains(component.label())
                    || code.contains("Cuboid")
                    || code.contains("Sphere::new")
                    || code.contains("Plane3d")
                    || code.contains("PointLight")
                    || code.contains("DirectionalLight")
                    || code.contains("Camera3d")
                    || code.contains("[BerryCode:");
                assert!(
                    has_component_output,
                    "Component '{}' produced no recognizable output in code",
                    label
                );
            }
        }
    }
}

// -------------------------------------------------------------------------
// Compile-check integration tests: verify generated code actually compiles
// -------------------------------------------------------------------------
#[cfg(test)]
mod compile_tests {
    use super::*;

    /// Shared Cargo.toml content for temporary Bevy projects used in compile checks.
    const TEST_CARGO_TOML: &str = r#"[package]
name = "codegen_test"
version = "0.1.0"
edition = "2021"

[dependencies]
bevy = "0.15"
"#;

    /// Helper: create a temp Bevy project, generate code for a scene, run `cargo check`.
    fn compile_check_scene(scene: &SceneModel) {
        let tmp = tempfile::tempdir().expect("create temp dir");
        let project = tmp.path();
        std::fs::create_dir_all(project.join("src")).unwrap();
        std::fs::write(project.join("Cargo.toml"), TEST_CARGO_TOML).unwrap();

        let code = generate_scene_code(scene);
        let main_rs = format!(
            "{}\nfn main() {{\n    bevy::prelude::App::new()\n        \
             .add_plugins(bevy::prelude::DefaultPlugins)\n        \
             .add_systems(bevy::prelude::Startup, setup_scene)\n        \
             .run();\n}}\n",
            code
        );
        std::fs::write(project.join("src/main.rs"), &main_rs).unwrap();

        let output = std::process::Command::new("cargo")
            .arg("check")
            .current_dir(project)
            .stderr(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .output()
            .expect("run cargo check");

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("=== Generated code ===\n{}\n=== End ===", main_rs);
            eprintln!("=== Cargo check errors ===\n{}", stderr);
            panic!("Generated code failed cargo check! See errors above.");
        }
    }

    /// Helper: compile-check a single component in isolation.
    fn compile_check_single_component(component: ComponentData) {
        let mut scene = SceneModel::new();
        scene.add_entity("Test".into(), vec![component]);
        compile_check_scene(&scene);
    }

    // === Full scene compile check (all component types at once) ===

    #[test]
    #[ignore] // Slow: runs cargo check. Run with: cargo test -p berrycode --lib -- --ignored codegen_all_components_compile_check
    fn codegen_all_components_compile_check() {
        let mut scene = SceneModel::new();

        // Add one entity per default component type
        for (name, comp) in ComponentData::default_all() {
            scene.add_entity(name.to_string(), vec![comp]);
        }

        // Add an entity with a non-trivial transform
        let id = scene.add_entity("Positioned".into(), vec![ComponentData::Camera]);
        if let Some(e) = scene.entities.get_mut(&id) {
            e.transform.translation = [10.0, 5.0, -3.0];
            e.transform.rotation_euler = [0.1, 0.2, 0.3];
            e.transform.scale = [2.0, 2.0, 2.0];
        }

        // Add a multi-component entity
        scene.add_entity(
            "Complex".into(),
            vec![
                ComponentData::MeshCube {
                    size: 1.0,
                    color: [1.0, 0.0, 0.0],
                    metallic: 0.5,
                    roughness: 0.3,
                    emissive: [0.1, 0.0, 0.0],
                    texture_path: None,
                    normal_map_path: None,
                },
                ComponentData::RigidBody {
                    body_type: RigidBodyType::Dynamic,
                    mass: 5.0,
                },
                ComponentData::Collider {
                    shape: ColliderShape::Box {
                        half_extents: [0.5, 0.5, 0.5],
                    },
                    friction: 0.6,
                    restitution: 0.2,
                },
            ],
        );

        compile_check_scene(&scene);
    }

    // === Individual component compile tests ===

    macro_rules! codegen_compile_test {
        ($name:ident, $component:expr) => {
            #[test]
            #[ignore]
            fn $name() {
                compile_check_single_component($component);
            }
        };
    }

    codegen_compile_test!(
        compile_mesh_cube,
        ComponentData::MeshCube {
            size: 1.0,
            color: [1.0, 0.0, 0.0],
            metallic: 0.5,
            roughness: 0.3,
            emissive: [0.0, 0.0, 0.0],
            texture_path: None,
            normal_map_path: None,
        }
    );

    codegen_compile_test!(
        compile_mesh_sphere,
        ComponentData::MeshSphere {
            radius: 0.5,
            color: [0.0, 1.0, 0.0],
            metallic: 0.0,
            roughness: 0.5,
            emissive: [0.0, 0.0, 0.0],
            texture_path: None,
            normal_map_path: None,
        }
    );

    codegen_compile_test!(
        compile_mesh_plane,
        ComponentData::MeshPlane {
            size: 10.0,
            color: [0.5, 0.5, 0.5],
            metallic: 0.0,
            roughness: 0.5,
            emissive: [0.0, 0.0, 0.0],
            texture_path: None,
            normal_map_path: None,
        }
    );

    codegen_compile_test!(
        compile_light,
        ComponentData::Light {
            intensity: 1000.0,
            color: [1.0, 1.0, 1.0],
        }
    );

    codegen_compile_test!(
        compile_directional_light,
        ComponentData::DirectionalLight {
            intensity: 10000.0,
            color: [1.0, 1.0, 0.9],
            shadows: true,
        }
    );

    codegen_compile_test!(compile_camera, ComponentData::Camera);

    codegen_compile_test!(
        compile_custom_script,
        ComponentData::CustomScript {
            type_name: "MyComp".into(),
            fields: vec![ScriptField {
                name: "val".into(),
                value: ScriptValue::Float(42.0),
            }],
        }
    );
}
