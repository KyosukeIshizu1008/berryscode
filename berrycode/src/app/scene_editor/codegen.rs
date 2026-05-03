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
/// Resolve a GLB path for auto-scale calculation.
/// Tries the path as-is, then assets/ relative, then common project locations.
fn resolve_glb_path(path: &str, project_root: &str) -> Option<String> {
    if std::path::Path::new(path).exists() {
        return Some(path.to_string());
    }
    if !project_root.is_empty() {
        let with_root_assets = format!("{}/assets/{}", project_root, path);
        if std::path::Path::new(&with_root_assets).exists() {
            return Some(with_root_assets);
        }
        let with_root = format!("{}/{}", project_root, path);
        if std::path::Path::new(&with_root).exists() {
            return Some(with_root);
        }
    }
    let with_assets = format!("assets/{}", path);
    if std::path::Path::new(&with_assets).exists() {
        return Some(with_assets);
    }
    None
}

/// Compute auto-scale for a GLB model to match scene editor preview size.
fn compute_glb_auto_scale(path: &str, project_root: &str) -> f32 {
    let resolved = resolve_glb_path(path, project_root).unwrap_or_else(|| path.to_string());
    crate::app::scene_editor::bevy_sync::extract_gltf_mesh_data(&resolved)
        .map(|data| {
            let mut min = [f32::MAX; 3];
            let mut max = [f32::MIN; 3];
            for p in &data.positions {
                for i in 0..3 {
                    min[i] = min[i].min(p[i]);
                    max[i] = max[i].max(p[i]);
                }
            }
            let extent = (max[0] - min[0])
                .max(max[1] - min[1])
                .max(max[2] - min[2])
                .max(0.001);
            if extent > 5.0 {
                2.0 / extent
            } else {
                1.0
            }
        })
        .unwrap_or(1.0)
}

pub fn generate_scene_code(scene: &SceneModel) -> String {
    let mut code = String::new();
    code.push_str("//! Auto-generated scene setup code from BerryCode Scene Editor.\n");
    code.push_str("//! DO NOT EDIT MANUALLY -- changes will be overwritten on next save.\n");
    code.push_str("//! To modify, edit in BerryCode's Scene Editor and re-save.\n\n");
    code.push_str("use bevy::prelude::*;\n\n");

    let any_enabled = scene.entities.values().any(|e| e.enabled);
    let needs_asset_server = scene.entities.values().any(|e| {
        e.enabled
            && e.components.iter().any(|c| {
                matches!(
                    c,
                    ComponentData::MeshFromFile { .. } | ComponentData::AudioSource { .. }
                )
            })
    });
    code.push_str("pub fn setup_scene(\n");
    if any_enabled {
        code.push_str("    mut commands: Commands,\n");
        code.push_str("    mut meshes: ResMut<Assets<Mesh>>,\n");
        code.push_str("    mut materials: ResMut<Assets<StandardMaterial>>,\n");
        if needs_asset_server {
            code.push_str("    asset_server: Res<AssetServer>,\n");
        } else {
            code.push_str("    _asset_server: Res<AssetServer>,\n");
        }
    } else {
        code.push_str("    _commands: Commands,\n");
        code.push_str("    _meshes: ResMut<Assets<Mesh>>,\n");
        code.push_str("    _materials: ResMut<Assets<StandardMaterial>>,\n");
        code.push_str("    _asset_server: Res<AssetServer>,\n");
    }
    code.push_str(") {\n");

    for entity in scene.entities.values() {
        if !entity.enabled {
            continue;
        }
        code.push_str(&format!("    // Entity: {}\n", entity.name));

        // Generate transform
        let t = &entity.transform;
        code.push_str(&format!(
            "    commands.spawn((\n        Transform::from_xyz({:.3}, {:.3}, {:.3})",
            t.translation[0], t.translation[1], t.translation[2]
        ));

        // Add rotation if non-zero
        if t.rotation_euler.iter().any(|&v| v.abs() > 0.001) {
            code.push_str(&format!(
                "\n            .with_rotation(Quat::from_euler(EulerRot::XYZ, {:.3}, {:.3}, {:.3}))",
                t.rotation_euler[0], t.rotation_euler[1], t.rotation_euler[2]
            ));
        }

        // Add scale if non-uniform
        if t.scale.iter().any(|&v| (v - 1.0).abs() > 0.001) {
            code.push_str(&format!(
                "\n            .with_scale(Vec3::new({:.3}, {:.3}, {:.3}))",
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
                        "        Mesh3d(meshes.add(Cuboid::new({:.3}, {:.3}, {:.3}))),\n",
                        size, size, size
                    ));
                    code.push_str(&format!(
                        "        MeshMaterial3d(materials.add(StandardMaterial {{\n\
                         \x20           base_color: Color::srgb({:.3}, {:.3}, {:.3}),\n\
                         \x20           metallic: {:.3},\n\
                         \x20           perceptual_roughness: {:.3},\n\
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
                        "        Mesh3d(meshes.add(Sphere::new({:.3}).mesh().uv(32, 16))),\n",
                        radius
                    ));
                    code.push_str(&format!(
                        "        MeshMaterial3d(materials.add(StandardMaterial {{\n\
                         \x20           base_color: Color::srgb({:.3}, {:.3}, {:.3}),\n\
                         \x20           metallic: {:.3},\n\
                         \x20           perceptual_roughness: {:.3},\n\
                         \x20           ..default()\n\
                         \x20       }})),\n",
                        color[0], color[1], color[2], metallic, roughness
                    ));
                }
                ComponentData::MeshPlane { size, color, .. } => {
                    code.push_str(&format!(
                        "        Mesh3d(meshes.add(Plane3d::default().mesh().size({:.3}, {:.3}))),\n",
                        size, size
                    ));
                    code.push_str(&format!(
                        "        MeshMaterial3d(materials.add(Color::srgb({:.3}, {:.3}, {:.3}))),\n",
                        color[0], color[1], color[2]
                    ));
                }
                ComponentData::Light { intensity, color } => {
                    code.push_str(&format!(
                        "        PointLight {{\n\
                         \x20           intensity: {:.3},\n\
                         \x20           color: Color::srgb({:.3}, {:.3}, {:.3}),\n\
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
                         \x20           illuminance: {:.3},\n\
                         \x20           color: Color::srgb({:.3}, {:.3}, {:.3}),\n\
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
                        "        // [BerryCode:SpotLight] intensity={:.3} color_r={:.3} color_g={:.3} color_b={:.3} range={:.3} inner_angle={:.3} outer_angle={:.3}\n",
                        intensity, color[0], color[1], color[2], range, inner_angle, outer_angle
                    ));
                }
                ComponentData::MeshFromFile { path, .. } => {
                    if !path.is_empty() {
                        let asset_rel = path
                            .replace('\\', "/")
                            .split("/assets/")
                            .nth(1)
                            .unwrap_or(path)
                            .to_string();
                        code.push_str(&format!(
                            "        SceneRoot(asset_server.load(\"{}#Scene0\")),\n",
                            asset_rel
                        ));
                    }
                }
                ComponentData::AudioSource {
                    path,
                    volume,
                    looped,
                    autoplay,
                } => {
                    code.push_str(&format!(
                        "        // [BerryCode:AudioSource] path={} volume={:.3} looped={} autoplay={}\n",
                        path, volume, looped, autoplay
                    ));
                }
                ComponentData::AudioListener => {
                    code.push_str("        // [BerryCode:AudioListener]\n");
                }
                ComponentData::RigidBody { body_type, mass } => {
                    code.push_str(&format!(
                        "        // [BerryCode:RigidBody] body_type={} mass={:.3}\n",
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
                            "shape=Box half_x={:.3} half_y={:.3} half_z={:.3}",
                            half_extents[0], half_extents[1], half_extents[2]
                        ),
                        ColliderShape::Sphere { radius } => {
                            format!("shape=Sphere radius={:.3}", radius)
                        }
                        ColliderShape::Capsule {
                            half_height,
                            radius,
                        } => format!(
                            "shape=Capsule half_height={:.3} radius={:.3}",
                            half_height, radius
                        ),
                    };
                    code.push_str(&format!(
                        "        // [BerryCode:Collider] {} friction={:.3} restitution={:.3}\n",
                        shape_str, friction, restitution
                    ));
                }
                ComponentData::UiText {
                    text,
                    font_size,
                    color,
                } => {
                    code.push_str(&format!(
                        "        // [BerryCode:UiText] text={} font_size={:.3} color_r={:.3} color_g={:.3} color_b={:.3} color_a={:.3}\n",
                        text, font_size, color[0], color[1], color[2], color[3]
                    ));
                }
                ComponentData::UiButton { label, background } => {
                    code.push_str(&format!(
                        "        // [BerryCode:UiButton] label={} bg_r={:.3} bg_g={:.3} bg_b={:.3} bg_a={:.3}\n",
                        label, background[0], background[1], background[2], background[3]
                    ));
                }
                ComponentData::UiImage { path, tint } => {
                    code.push_str(&format!(
                        "        // [BerryCode:UiImage] path={} tint_r={:.3} tint_g={:.3} tint_b={:.3} tint_a={:.3}\n",
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
                        "        // [BerryCode:ParticleEmitter] rate={:.3} lifetime={:.3} speed={:.3} spread={:.3} start_size={:.3} end_size={:.3} sc_r={:.3} sc_g={:.3} sc_b={:.3} sc_a={:.3} ec_r={:.3} ec_g={:.3} ec_b={:.3} ec_a={:.3} max_particles={} gravity={:.3}\n",
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
                        "        // [BerryCode:Animation] duration={:.3} looped={} tracks={}\n",
                        duration,
                        looped,
                        tracks.len()
                    ));
                }
                ComponentData::CustomScript {
                    type_name, fields, ..
                } => {
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
                        "        // [BerryCode:Terrain] resolution={} world_w={:.3} world_h={:.3} base_r={:.3} base_g={:.3} base_b={:.3}\n",
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
                        "        // [BerryCode:NavMesh] cell_size={:.3} width={} height={}\n",
                        cell_size, width, height
                    ));
                }
                ComponentData::JointFixed { connected_entity } => {
                    code.push_str(&format!(
                        "        // [BerryCode:JointFixed] connected_entity={:?}\n",
                        connected_entity
                    ));
                }
                ComponentData::JointHinge {
                    connected_entity,
                    axis,
                    limits,
                } => {
                    code.push_str(&format!(
                        "        // [BerryCode:JointHinge] connected_entity={:?} axis=[{:.3},{:.3},{:.3}] limits={:?}\n",
                        connected_entity, axis[0], axis[1], axis[2], limits
                    ));
                }
                ComponentData::JointSpring {
                    connected_entity,
                    stiffness,
                    damping,
                    rest_length,
                } => {
                    code.push_str(&format!(
                        "        // [BerryCode:JointSpring] connected_entity={:?} stiffness={:.3} damping={:.3} rest_length={:.3}\n",
                        connected_entity, stiffness, damping, rest_length
                    ));
                }
                ComponentData::NavMeshAgent {
                    speed,
                    radius,
                    height,
                    max_slope,
                } => {
                    code.push_str(&format!(
                        "        // [BerryCode:NavMeshAgent] speed={:.3} radius={:.3} height={:.3} max_slope={:.3}\n",
                        speed, radius, height, max_slope
                    ));
                }
                ComponentData::PlayerController { .. } => {
                    // The legacy monolithic emit doesn't reference the
                    // shared `PlayerController` struct (only the modular
                    // plugin path does). Drop a comment for parity.
                    code.push_str("        // [BerryCode:PlayerController]\n");
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

/// Patch main.rs: add `asset_server` parameter to setup function if needed,
/// and append/update only MeshFromFile (GLB) spawn code. Does NOT replace
/// existing entities — only manages a marked `[BerryCode:GLB-start/end]` block.
pub fn patch_main_rs_setup(main_code: &str, scene: &SceneModel) -> String {
    // Collect only GLB entities from scene
    let glb_entities: Vec<&SceneEntity> = scene
        .entities
        .values()
        .filter(|e| {
            e.enabled
                && e.components.iter().any(
                    |c| matches!(c, ComponentData::MeshFromFile { path, .. } if !path.is_empty()),
                )
        })
        .collect();

    if glb_entities.is_empty() {
        return main_code.to_string();
    }

    let mut result = main_code.to_string();

    // Add asset_server parameter to setup function if missing
    let setup_fn_re =
        regex::Regex::new(r"(fn\s+(setup_world|setup_scene|setup)\s*\()([^)]*)\)").unwrap();

    if let Some(cap) = setup_fn_re.captures(&result.clone()) {
        let params = &cap[3];
        if !params.contains("asset_server") && !params.contains("AssetServer") {
            let full_match = cap.get(0).unwrap();
            let before_paren = &cap[1];
            let params_trimmed = params.trim_end();
            let comma = if params_trimmed.ends_with(',') {
                ""
            } else {
                ","
            };
            let new_sig = format!(
                "{}{}{}\n    asset_server: Res<AssetServer>,\n)",
                before_paren, params_trimmed, comma
            );
            result = format!(
                "{}{}{}",
                &result[..full_match.start()],
                new_sig,
                &result[full_match.end()..]
            );
        }
    }

    // Remove old BerryCode GLB block if present
    let marker_start = "    // [BerryCode:GLB-start]\n";
    let marker_end = "    // [BerryCode:GLB-end]\n";
    if let Some(start) = result.find(marker_start) {
        if let Some(end_offset) = result[start..].find(marker_end) {
            let end = start + end_offset + marker_end.len();
            result = format!("{}{}", &result[..start], &result[end..]);
        }
    }

    // Find closing brace of setup function to insert before it
    let setup_body_re =
        regex::Regex::new(r"fn\s+(setup_world|setup_scene|setup)\s*\([^)]*\)\s*\{").unwrap();

    if let Some(cap) = setup_body_re.captures(&result.clone()) {
        let fn_body_start = cap.get(0).unwrap().end();
        let bytes = result.as_bytes();
        let mut depth = 1i32;
        let mut close_pos = fn_body_start;
        for i in fn_body_start..bytes.len() {
            match bytes[i] {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        close_pos = i;
                        break;
                    }
                }
                _ => {}
            }
        }

        // Generate GLB spawn code
        let mut glb_code = String::new();
        glb_code.push_str("    // [BerryCode:GLB-start]\n");
        for entity in &glb_entities {
            let t = &entity.transform;
            for component in &entity.components {
                if let ComponentData::MeshFromFile { path, .. } = component {
                    if path.is_empty() {
                        continue;
                    }
                    let asset_rel = path
                        .replace('\\', "/")
                        .split("/assets/")
                        .nth(1)
                        .unwrap_or(path)
                        .to_string();

                    // Compute auto-scale for GLB models (match bevy_sync behavior)
                    let auto_scale = compute_glb_auto_scale(path, "");

                    let sx = t.scale[0] * auto_scale;
                    let sy = t.scale[1] * auto_scale;
                    let sz = t.scale[2] * auto_scale;

                    glb_code.push_str(&format!(
                        "    commands.spawn((\n\
                         \x20       SceneRoot(asset_server.load(\"{}#Scene0\")),\n\
                         \x20       Transform::from_xyz({:.3}, {:.3}, {:.3})\n\
                         \x20           .with_rotation(Quat::from_euler(EulerRot::XYZ, {:.3}, {:.3}, {:.3}))\n\
                         \x20           .with_scale(Vec3::new({:.3}, {:.3}, {:.3})),\n\
                         \x20       Name::new(\"{}\"),\n\
                         \x20   ));\n",
                        asset_rel,
                        t.translation[0],
                        t.translation[1],
                        t.translation[2],
                        t.rotation_euler[0],
                        t.rotation_euler[1],
                        t.rotation_euler[2],
                        sx, sy, sz,
                        entity.name,
                    ));
                }
            }
        }
        glb_code.push_str("    // [BerryCode:GLB-end]\n");

        result = format!(
            "{}{}{}",
            &result[..close_pos],
            glb_code,
            &result[close_pos..]
        );
    }

    result
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

// ---------------------------------------------------------------------------
// Modular project structure support
// ---------------------------------------------------------------------------

/// Returns true if the project at `root` has the modular `src/scenes/` structure.
pub fn has_modular_structure(root: &str) -> bool {
    std::path::Path::new(&format!("{}/src/scenes/mod.rs", root)).exists()
}

/// Ensures `pub mod <module_name>;` exists in the given mod.rs file.
/// Creates the file if it doesn't exist. Idempotent.
pub fn ensure_mod_declaration(mod_rs_path: &str, module_name: &str) -> Result<(), String> {
    let decl = format!("pub mod {};", module_name);
    let content = std::fs::read_to_string(mod_rs_path).unwrap_or_default();
    if content.contains(&decl) {
        return Ok(());
    }
    if let Some(parent) = std::path::Path::new(mod_rs_path).parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let updated = if content.is_empty() {
        format!("{}\n", decl)
    } else {
        format!("{}\n{}\n", content.trim_end(), decl)
    };
    std::fs::write(mod_rs_path, updated).map_err(|e| e.to_string())
}

/// Convert a scene name to a valid Rust module name (snake_case).
pub fn scene_name_to_module(scene_name: &str) -> String {
    let mut out = String::new();
    for (i, c) in scene_name.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            out.push('_');
        }
        out.push(c.to_lowercase().next().unwrap_or(c));
    }
    out.replace([' ', '-', '.'], "_")
}

/// Convert a module name to PascalCase for struct names.
fn module_to_pascal(module: &str) -> String {
    module
        .split('_')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().chain(chars).collect(),
            }
        })
        .collect()
}

/// Generate a scene as a Bevy Plugin module.
#[allow(dead_code)]
pub fn generate_scene_plugin_code(scene: &SceneModel, scene_name: &str) -> String {
    generate_scene_plugin_code_with_root(scene, scene_name, "")
}

#[allow(dead_code)]
pub fn generate_scene_plugin_code_with_root(
    scene: &SceneModel,
    scene_name: &str,
    project_root: &str,
) -> String {
    let module_name = scene_name_to_module(scene_name);
    let pascal_name = module_to_pascal(&module_name);
    // Plugin / setup names track the scene name 1:1 — `<Test>Plugin`
    // and `setup_test`, not `<Test>ScenePlugin` / `setup_test_scene`.
    // Keeps the file tree, the Hierarchy tab strip, and the generated
    // module names all reading the same.
    let plugin_name = format!("{}Plugin", pascal_name);
    let setup_fn = format!("setup_{}", module_name);
    let cleanup_fn = format!("cleanup_{}", module_name);
    // Per-scene marker component used to despawn this scene's
    // entities on `OnExit`. Naming is `<Pascal>SceneEntity` — verbose
    // but unambiguous, since the marker has to be unique across every
    // scene module that lives in `src/scenes/`.
    let marker_name = format!("{}SceneEntity", pascal_name);

    // GLB-skeletal-animation support: per-scene marker / resource / system
    // names so two scenes can each carry their own state without colliding.
    let has_glb = scene.entities.values().any(|e| {
        e.enabled
            && e.components
                .iter()
                .any(|c| matches!(c, ComponentData::MeshFromFile { path, .. } if !path.is_empty()))
    });
    let pending_marker = format!("{}PendingGlbAnim", pascal_name);
    let graphs_resource = format!("{}GlbAnimGraphs", pascal_name);
    let entry_struct = format!("{}GlbAnimEntry", pascal_name);
    let build_sys = format!("{}_build_glb_anim_graphs", module_name);
    let attach_sys = format!("{}_attach_glb_anim_players", module_name);

    let mut code = String::new();
    code.push_str("//! Auto-generated scene plugin from BerryCode Scene Editor.\n");
    code.push_str("//! DO NOT EDIT MANUALLY -- changes will be overwritten on next save.\n\n");
    code.push_str("use bevy::prelude::*;\n");
    if has_glb {
        code.push_str("use bevy::animation::graph::{AnimationGraph, AnimationGraphHandle, AnimationNodeIndex};\n");
        code.push_str("use bevy::animation::{AnimationClip, AnimationPlayer};\n");
        code.push_str("use bevy::asset::AssetId;\n");
        code.push_str("use bevy::gltf::Gltf;\n");
        code.push_str("use std::collections::HashMap;\n");
    }
    code.push_str("use super::AppScene;\n\n");

    // Marker component for cleanup. `dead_code` is allowed because
    // empty scenes never construct it (setup spawns nothing), but the
    // Plugin still needs the type to exist for the cleanup query.
    code.push_str("#[derive(Component)]\n");
    code.push_str("#[allow(dead_code)]\n");
    code.push_str(&format!("pub struct {};\n\n", marker_name));

    if has_glb {
        // Marker carrying the `Handle<Gltf>` and the optional clip name to
        // play. The build/attach systems use it to map the
        // auto-inserted `AnimationPlayer` back to the right graph + node.
        // `pub` so user code (e.g. a `switch_clip` system in `main.rs`)
        // can read the GLTF handle and trigger animation changes at
        // runtime without going through codegen.
        code.push_str("#[derive(Component)]\n");
        code.push_str(&format!(
            "pub struct {} {{\n    pub gltf: Handle<Gltf>,\n    pub clip: Option<String>,\n}}\n\n",
            pending_marker
        ));
        // Per-GLTF graph + (clip name → node) lookup so we can pick a
        // specific clip by name at attach time.
        code.push_str(&format!(
            "pub struct {} {{\n    pub graph: Handle<AnimationGraph>,\n    pub nodes: Vec<(String, AnimationNodeIndex)>,\n}}\n\n",
            entry_struct
        ));
        code.push_str("#[derive(Resource, Default)]\n");
        code.push_str(&format!(
            "pub struct {} {{\n    pub by_id: HashMap<AssetId<Gltf>, {}>,\n}}\n\n",
            graphs_resource, entry_struct
        ));
    }

    // Plugin struct — registers OnEnter/OnExit against the AppScene
    // variant matching this module. Two scenes in the same project
    // therefore no longer overlap: only the active state's entities
    // exist at any time. Switch scenes at runtime with
    // `commands.set_state(AppScene::Other)`.
    code.push_str(&format!("pub struct {};\n\n", plugin_name));
    code.push_str(&format!("impl Plugin for {} {{\n", plugin_name));
    code.push_str("    fn build(&self, app: &mut App) {\n");
    code.push_str("        app");
    if has_glb {
        code.push_str(&format!(
            "\n            .init_resource::<{}>()",
            graphs_resource
        ));
    }
    code.push_str(&format!(
        "\n            .add_systems(OnEnter(AppScene::{}), {})",
        pascal_name, setup_fn
    ));
    code.push_str(&format!(
        "\n            .add_systems(OnExit(AppScene::{}), {})",
        pascal_name, cleanup_fn
    ));
    if has_glb {
        code.push_str(&format!(
            "\n            .add_systems(Update, ({}, {}.after({})))",
            build_sys, attach_sys, build_sys
        ));
    }
    code.push_str(";\n");
    code.push_str("    }\n");
    code.push_str("}\n\n");

    // Setup function parameters are conditionally `_`-prefixed based
    // on what the body actually references. The previous heuristic
    // ("scene has any visible entity → keep all four un-prefixed")
    // still tripped `unused_variables` for scenes that only contain
    // primitives like Cube/Sphere — those don't touch
    // `asset_server`, so it has to be `_asset_server` instead.
    let visible_entities: Vec<&_> = scene
        .entities
        .values()
        .filter(|e| {
            e.enabled
                && !e
                    .components
                    .iter()
                    .any(|c| matches!(c, ComponentData::Camera))
        })
        .collect();
    let scene_has_visible_entities = !visible_entities.is_empty();
    let needs_asset_server = visible_entities.iter().any(|e| {
        e.components.iter().any(|c| {
            matches!(
                c,
                ComponentData::MeshFromFile { .. } | ComponentData::AudioSource { .. }
            )
        })
    });
    code.push_str(&format!("fn {}(\n", setup_fn));
    if scene_has_visible_entities {
        code.push_str("    mut commands: Commands,\n");
        code.push_str("    mut meshes: ResMut<Assets<Mesh>>,\n");
        code.push_str("    mut materials: ResMut<Assets<StandardMaterial>>,\n");
        if needs_asset_server {
            code.push_str("    asset_server: Res<AssetServer>,\n");
        } else {
            code.push_str("    _asset_server: Res<AssetServer>,\n");
        }
    } else {
        code.push_str("    _commands: Commands,\n");
        code.push_str("    _meshes: ResMut<Assets<Mesh>>,\n");
        code.push_str("    _materials: ResMut<Assets<StandardMaterial>>,\n");
        code.push_str("    _asset_server: Res<AssetServer>,\n");
    }
    code.push_str(") {\n");

    for entity in scene.entities.values() {
        if !entity.enabled {
            continue;
        }
        // Skip Camera entities — the main template already spawns a camera
        if entity
            .components
            .iter()
            .any(|c| matches!(c, ComponentData::Camera))
        {
            continue;
        }
        code.push_str(&format!("    // Entity: {}\n", entity.name));
        let t = &entity.transform;

        // Compute auto-scale for GLB models to match scene editor preview size
        let glb_auto_scale = entity
            .components
            .iter()
            .find_map(|c| {
                if let ComponentData::MeshFromFile { path, .. } = c {
                    if !path.is_empty() {
                        let scale = compute_glb_auto_scale(path, project_root);
                        return Some(scale);
                    }
                }
                None
            })
            .unwrap_or(1.0);

        let sx = t.scale[0] * glb_auto_scale;
        let sy = t.scale[1] * glb_auto_scale;
        let sz = t.scale[2] * glb_auto_scale;

        code.push_str(&format!(
            "    commands.spawn((\n        Transform::from_xyz({:.3}, {:.3}, {:.3})",
            t.translation[0], t.translation[1], t.translation[2]
        ));
        if t.rotation_euler.iter().any(|&v| v.abs() > 0.001) {
            code.push_str(&format!(
                "\n            .with_rotation(Quat::from_euler(EulerRot::XYZ, {:.3}, {:.3}, {:.3}))",
                t.rotation_euler[0], t.rotation_euler[1], t.rotation_euler[2]
            ));
        }
        if (sx - 1.0).abs() > 0.001 || (sy - 1.0).abs() > 0.001 || (sz - 1.0).abs() > 0.001 {
            code.push_str(&format!(
                "\n            .with_scale(Vec3::new({:.3}, {:.3}, {:.3}))",
                sx, sy, sz
            ));
        }
        code.push_str(",\n");

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
                        "        Mesh3d(meshes.add(Cuboid::new({:.3}, {:.3}, {:.3}))),\n",
                        size, size, size
                    ));
                    code.push_str(&format!(
                        "        MeshMaterial3d(materials.add(StandardMaterial {{\n\
                         \x20           base_color: Color::srgb({:.3}, {:.3}, {:.3}),\n\
                         \x20           metallic: {:.3},\n\
                         \x20           perceptual_roughness: {:.3},\n\
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
                        "        Mesh3d(meshes.add(Sphere::new({:.3}).mesh().uv(32, 16))),\n",
                        radius
                    ));
                    code.push_str(&format!(
                        "        MeshMaterial3d(materials.add(StandardMaterial {{\n\
                         \x20           base_color: Color::srgb({:.3}, {:.3}, {:.3}),\n\
                         \x20           metallic: {:.3},\n\
                         \x20           perceptual_roughness: {:.3},\n\
                         \x20           ..default()\n\
                         \x20       }})),\n",
                        color[0], color[1], color[2], metallic, roughness
                    ));
                }
                ComponentData::MeshPlane { size, color, .. } => {
                    code.push_str(&format!(
                        "        Mesh3d(meshes.add(Plane3d::default().mesh().size({:.3}, {:.3}))),\n",
                        size, size
                    ));
                    code.push_str(&format!(
                        "        MeshMaterial3d(materials.add(Color::srgb({:.3}, {:.3}, {:.3}))),\n",
                        color[0], color[1], color[2]
                    ));
                }
                ComponentData::Light { intensity, color } => {
                    code.push_str(&format!(
                        "        PointLight {{ intensity: {:.1}, color: Color::srgb({:.3}, {:.3}, {:.3}), ..default() }},\n",
                        intensity, color[0], color[1], color[2]
                    ));
                }
                ComponentData::DirectionalLight {
                    intensity,
                    color,
                    shadows,
                } => {
                    code.push_str(&format!(
                        "        DirectionalLight {{ illuminance: {:.1}, color: Color::srgb({:.3}, {:.3}, {:.3}), shadows_enabled: {}, ..default() }},\n",
                        intensity, color[0], color[1], color[2], shadows
                    ));
                }
                ComponentData::Camera => {
                    code.push_str("        Camera3d::default(),\n");
                }
                ComponentData::MeshFromFile {
                    path,
                    auto_play_clip,
                    ..
                } => {
                    if !path.is_empty() {
                        let asset_rel = path
                            .replace('\\', "/")
                            .split("/assets/")
                            .nth(1)
                            .unwrap_or(path)
                            .to_string();
                        code.push_str(&format!(
                            "        SceneRoot(asset_server.load(\"{}#Scene0\")),\n",
                            asset_rel
                        ));
                        let clip_arg = match auto_play_clip {
                            Some(name) => format!("Some(\"{}\".to_string())", name),
                            None => "None".to_string(),
                        };
                        code.push_str(&format!(
                            "        {} {{\n            gltf: asset_server.load(\"{}\"),\n            clip: {},\n        }},\n",
                            pending_marker, asset_rel, clip_arg
                        ));
                    }
                }
                ComponentData::PlayerController {
                    speed,
                    jump_velocity,
                    run_multiplier,
                    turn_speed,
                } => {
                    code.push_str(&format!(
                        "        super::PlayerController {{\n            speed: {:.3},\n            jump_velocity: {:.3},\n            run_multiplier: {:.3},\n            turn_speed: {:.3},\n        }},\n",
                        speed, jump_velocity, run_multiplier, turn_speed
                    ));
                }
                ComponentData::RigidBody { body_type, .. } => {
                    let variant = match body_type {
                        RigidBodyType::Static => "Static",
                        RigidBodyType::Dynamic => "Dynamic",
                        RigidBodyType::Kinematic => "Kinematic",
                    };
                    code.push_str(&format!(
                        "        avian3d::prelude::RigidBody::{},\n",
                        variant
                    ));
                }
                ComponentData::Collider {
                    shape,
                    friction,
                    restitution,
                } => {
                    let collider_expr = match shape {
                        ColliderShape::Box { half_extents } => format!(
                            "avian3d::prelude::Collider::cuboid({:.3}, {:.3}, {:.3})",
                            half_extents[0] * 2.0,
                            half_extents[1] * 2.0,
                            half_extents[2] * 2.0,
                        ),
                        ColliderShape::Sphere { radius } => {
                            format!("avian3d::prelude::Collider::sphere({:.3})", radius)
                        }
                        ColliderShape::Capsule {
                            half_height,
                            radius,
                        } => format!(
                            "avian3d::prelude::Collider::capsule({:.3}, {:.3})",
                            radius,
                            half_height * 2.0,
                        ),
                    };
                    code.push_str(&format!("        {},\n", collider_expr));
                    code.push_str(&format!(
                        "        avian3d::prelude::Friction::new({:.3}),\n",
                        friction
                    ));
                    code.push_str(&format!(
                        "        avian3d::prelude::Restitution::new({:.3}),\n",
                        restitution
                    ));
                }
                _ => {}
            }
        }

        // Marker last so the spawn tuple ends with the per-scene tag
        // — keeps Transform / Mesh / Material reading naturally and
        // means the cleanup query (`With<<Pascal>SceneEntity>`) hits
        // every entity this scene spawned.
        code.push_str(&format!("        {},\n", marker_name));
        code.push_str(&format!("        Name::new(\"{}\"),\n", entity.name));
        code.push_str("    ));\n\n");
    }

    code.push_str("}\n\n");

    // Cleanup runs on `OnExit(AppScene::<This>)` and despawns every
    // entity tagged with the per-scene marker. Always emitted even
    // for empty scenes — the query is just empty at runtime.
    code.push_str(&format!("fn {}(\n", cleanup_fn));
    code.push_str("    mut commands: Commands,\n");
    code.push_str(&format!(
        "    query: Query<Entity, With<{}>>,\n",
        marker_name
    ));
    code.push_str(") {\n");
    code.push_str("    for entity in &query {\n");
    code.push_str("        commands.entity(entity).despawn();\n");
    code.push_str("    }\n");
    code.push_str("}\n");

    if has_glb {
        // Helper systems mirror BerryCode's editor preview
        // (`scene_editor::skeletal_animation`): build an `AnimationGraph`
        // from the GLTF's named clips, then attach it to the
        // `AnimationPlayer` Bevy auto-inserts on the scene root and play
        // either the user-chosen clip (Inspector dropdown, persisted to
        // `.bscene` as `auto_play_clip`) or the alphabetically-first one.
        code.push_str(&format!(
            "\nfn {build}(\n\
             \x20   mut state: ResMut<{graphs}>,\n\
             \x20   pending: Query<&{marker}>,\n\
             \x20   gltfs: Res<Assets<Gltf>>,\n\
             \x20   mut graphs: ResMut<Assets<AnimationGraph>>,\n\
             ) {{\n\
             \x20   for p in &pending {{\n\
             \x20       let id = p.gltf.id();\n\
             \x20       if state.by_id.contains_key(&id) {{\n\
             \x20           continue;\n\
             \x20       }}\n\
             \x20       let Some(gltf) = gltfs.get(&p.gltf) else {{\n\
             \x20           continue;\n\
             \x20       }};\n\
             \x20       if gltf.named_animations.is_empty() {{\n\
             \x20           continue;\n\
             \x20       }}\n\
             \x20       let mut named: Vec<(&str, &Handle<AnimationClip>)> = gltf\n\
             \x20           .named_animations\n\
             \x20           .iter()\n\
             \x20           .map(|(n, h)| (n.as_ref(), h))\n\
             \x20           .collect();\n\
             \x20       named.sort_by(|a, b| a.0.cmp(b.0));\n\
             \x20       let clips: Vec<Handle<AnimationClip>> =\n\
             \x20           named.iter().map(|(_, h)| (*h).clone()).collect();\n\
             \x20       let (graph, indices) = AnimationGraph::from_clips(clips);\n\
             \x20       let graph_handle = graphs.add(graph);\n\
             \x20       let nodes: Vec<(String, AnimationNodeIndex)> = named\n\
             \x20           .into_iter()\n\
             \x20           .zip(indices)\n\
             \x20           .map(|((n, _), i)| (n.to_string(), i))\n\
             \x20           .collect();\n\
             \x20       state\n\
             \x20           .by_id\n\
             \x20           .insert(id, {entry} {{ graph: graph_handle, nodes }});\n\
             \x20   }}\n\
             }}\n\n\
             fn {attach}(\n\
             \x20   mut commands: Commands,\n\
             \x20   state: Res<{graphs}>,\n\
             \x20   mut players: Query<(Entity, &mut AnimationPlayer), Without<AnimationGraphHandle>>,\n\
             \x20   pending_q: Query<&{marker}>,\n\
             \x20   ancestors: Query<&ChildOf>,\n\
             ) {{\n\
             \x20   for (entity, mut player) in &mut players {{\n\
             \x20       let mut cur = entity;\n\
             \x20       let mut found: Option<&{marker}> = None;\n\
             \x20       for _ in 0..50 {{\n\
             \x20           if let Ok(p) = pending_q.get(cur) {{\n\
             \x20               found = Some(p);\n\
             \x20               break;\n\
             \x20           }}\n\
             \x20           match ancestors.get(cur) {{\n\
             \x20               Ok(c) => cur = c.parent(),\n\
             \x20               Err(_) => break,\n\
             \x20           }}\n\
             \x20       }}\n\
             \x20       let Some(pending) = found else {{\n\
             \x20           continue;\n\
             \x20       }};\n\
             \x20       let Some(entry) = state.by_id.get(&pending.gltf.id()) else {{\n\
             \x20           continue;\n\
             \x20       }};\n\
             \x20       let node = match &pending.clip {{\n\
             \x20           Some(name) => entry\n\
             \x20               .nodes\n\
             \x20               .iter()\n\
             \x20               .find(|(n, _)| n == name)\n\
             \x20               .or_else(|| entry.nodes.first())\n\
             \x20               .map(|(_, i)| *i),\n\
             \x20           None => entry.nodes.first().map(|(_, i)| *i),\n\
             \x20       }};\n\
             \x20       let Some(node) = node else {{\n\
             \x20           continue;\n\
             \x20       }};\n\
             \x20       player.stop_all();\n\
             \x20       let active = player.play(node);\n\
             \x20       active.repeat();\n\
             \x20       active.set_seek_time(0.0);\n\
             \x20       commands\n\
             \x20           .entity(entity)\n\
             \x20           .insert(AnimationGraphHandle(entry.graph.clone()));\n\
             \x20   }}\n\
             }}\n",
            build = build_sys,
            attach = attach_sys,
            graphs = graphs_resource,
            entry = entry_struct,
            marker = pending_marker,
        ));
    }

    code
}

/// Scan `src/scenes/` for scene plugin files and generate `mod.rs` with
/// `AppScene` (a `States` enum, one variant per scene) and
/// `ScenesPlugin`. Picks up any `*.rs` (except `mod.rs`) since scene
/// files are now named to match their `.bscene` (e.g. `test.rs`).
///
/// The alphabetically-first module gets `#[default]`, mirroring the
/// editor's tab strip which makes that same scene the active tab on
/// project open. Switch scenes at runtime with
/// `commands.set_state(AppScene::<Other>)`.
///
/// Empty case (no modules): emit a stub `ScenesPlugin` with no
/// `AppScene` enum, since an empty enum can't derive `Default`. New
/// projects start in this state and get rewritten on first scene save.
pub fn generate_scenes_mod_rs(scenes_dir: &str) -> String {
    let mut modules: Vec<String> = Vec::new();

    if let Ok(entries) = std::fs::read_dir(scenes_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".rs") && name != "mod.rs" {
                let module = name.strip_suffix(".rs").unwrap_or(&name).to_string();
                modules.push(module);
            }
        }
    }
    modules.sort();

    let mut code = String::new();
    for m in &modules {
        code.push_str(&format!("pub mod {};\n", m));
    }
    code.push_str("\nuse bevy::prelude::*;\n\n");

    // Project-wide `PlayerController` — single shared definition so the
    // user's movement system can `use scenes::PlayerController` regardless
    // of which scene the player lives in. Scenes that don't use the
    // component just don't reference it; the unused-struct warning is
    // suppressed below.
    code.push_str("#[derive(Component, Debug, Clone)]\n");
    code.push_str("#[allow(dead_code)]\n");
    code.push_str("pub struct PlayerController {\n");
    code.push_str("    pub speed: f32,\n");
    code.push_str("    pub jump_velocity: f32,\n");
    code.push_str("    pub run_multiplier: f32,\n");
    code.push_str("    /// Slerp factor (per second) used to face the\n");
    code.push_str("    /// movement direction. `0.0` keeps rotation locked.\n");
    code.push_str("    pub turn_speed: f32,\n");
    code.push_str("}\n\n");

    if modules.is_empty() {
        code.push_str("pub struct ScenesPlugin;\n\n");
        code.push_str("impl Plugin for ScenesPlugin {\n");
        code.push_str("    fn build(&self, _app: &mut App) {\n");
        code.push_str("    }\n");
        code.push_str("}\n");
        return code;
    }

    code.push_str("#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]\n");
    code.push_str("pub enum AppScene {\n");
    for (i, m) in modules.iter().enumerate() {
        let pascal = module_to_pascal(m);
        if i == 0 {
            code.push_str("    #[default]\n");
        }
        code.push_str(&format!("    {},\n", pascal));
    }
    code.push_str("}\n\n");

    code.push_str("pub struct ScenesPlugin;\n\n");
    code.push_str("impl Plugin for ScenesPlugin {\n");
    code.push_str("    fn build(&self, app: &mut App) {\n");
    code.push_str("        app.init_state::<AppScene>();\n");
    for m in &modules {
        let pascal = module_to_pascal(m);
        code.push_str(&format!(
            "        app.add_plugins({}::{}Plugin);\n",
            m, pascal
        ));
    }
    code.push_str("    }\n");
    code.push_str("}\n");
    code
}

/// Save scene code in modular structure: `src/scenes/{name}.rs` + update mod.rs.
/// Filename matches the `.bscene` stem so the file tree and the
/// Hierarchy tab strip read the same way (a scene called `test` lives
/// at `src/scenes/test.rs`, not `test_scene.rs`).
pub fn save_scene_code_modular(
    scene: &SceneModel,
    scene_path: &str,
    project_root: &str,
) -> Result<String, String> {
    let scene_name = std::path::Path::new(scene_path)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "scene".to_string());
    let module_name = scene_name_to_module(&scene_name);

    let scenes_dir = format!("{}/src/scenes", project_root);
    std::fs::create_dir_all(&scenes_dir).map_err(|e| e.to_string())?;

    let code = generate_scene_plugin_code_with_root(scene, &scene_name, project_root);
    let rs_path = format!("{}/{}.rs", scenes_dir, module_name);
    // Only touch the file when the content actually changes —
    // `cargo check` keys off mtime, and a no-op rewrite triggers an
    // unnecessary rebuild + LSP reanalysis.
    let needs_write = std::fs::read_to_string(&rs_path)
        .map(|existing| existing != code)
        .unwrap_or(true);
    if needs_write {
        std::fs::write(&rs_path, &code).map_err(|e| e.to_string())?;
    }

    // Migrate from the old `<name>_scene.rs` convention. If a file
    // with the legacy suffix still exists, delete it so cargo doesn't
    // pick up two scene plugins for the same `.bscene`.
    let legacy_path = format!("{}/{}_scene.rs", scenes_dir, module_name);
    if std::path::Path::new(&legacy_path).exists() && legacy_path != rs_path {
        let _ = std::fs::remove_file(&legacy_path);
    }

    let mod_rs = generate_scenes_mod_rs(&scenes_dir);
    std::fs::write(format!("{}/mod.rs", scenes_dir), &mod_rs).map_err(|e| e.to_string())?;

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
                script_path: String::new(),
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
                script_path: String::new(),
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
                script_path: String::new(),
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
                script_path: String::new(),
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
                script_path: String::new(),
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
        assert!(code.contains("1.123"));
        assert!(code.contains("0.123"));
        assert!(code.contains("0.654"));
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
                script_path: String::new(),
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
                script_path: String::new(),
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
                auto_play_clip: None,
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
                    script_path: String::new(),
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
                auto_play_clip: None,
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
                script_path: String::new(),
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
                script_path: String::new(),
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
                script_path: String::new(),
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
                script_path: String::new(),
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
                    script_path: String::new(),
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
            script_path: String::new(),
            fields: vec![ScriptField {
                name: "val".into(),
                value: ScriptValue::Float(42.0),
            }],
        }
    );

    #[test]
    fn scene_name_to_module_converts() {
        assert_eq!(scene_name_to_module("GameLevel"), "game_level");
        assert_eq!(scene_name_to_module("my-scene"), "my_scene");
        assert_eq!(scene_name_to_module("simple"), "simple");
    }

    #[test]
    fn module_to_pascal_converts() {
        assert_eq!(module_to_pascal("game_level"), "GameLevel");
        assert_eq!(module_to_pascal("scene"), "Scene");
        assert_eq!(module_to_pascal("my_cool_scene"), "MyCoolScene");
    }

    #[test]
    fn generate_scene_plugin_has_plugin_struct() {
        let scene = SceneModel::new();
        let code = generate_scene_plugin_code(&scene, "game");
        // Plugin / setup names track the scene name 1:1 — no `_scene`
        // suffix on either, so the file tree, the Hierarchy tab, and
        // the generated module all read the same.
        assert!(code.contains("pub struct GamePlugin;"));
        assert!(code.contains("impl Plugin for GamePlugin"));
        assert!(code.contains("fn setup_game("));
        // Scenes are state-driven: setup runs on `OnEnter`, cleanup
        // (despawning everything tagged with the per-scene marker)
        // runs on `OnExit`. The old `Startup` registration would
        // overlap every scene's entities into the same world.
        assert!(code.contains("OnEnter(AppScene::Game)"));
        assert!(code.contains("OnExit(AppScene::Game)"));
        assert!(code.contains("fn cleanup_game("));
        assert!(code.contains("pub struct GameSceneEntity;"));
        assert!(code.contains("use super::AppScene;"));
    }

    #[test]
    fn generate_scene_plugin_with_entities() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "TestCube".into(),
            vec![ComponentData::MeshCube {
                size: 1.0,
                color: [1.0, 0.0, 0.0],
                metallic: 0.0,
                roughness: 0.5,
                emissive: [0.0, 0.0, 0.0],
                texture_path: None,
                normal_map_path: None,
            }],
        );
        let code = generate_scene_plugin_code(&scene, "level1");
        assert!(code.contains("pub struct Level1Plugin;"));
        assert!(code.contains("Cuboid::new(1.000"));
        assert!(code.contains("Name::new(\"TestCube\")"));
        // With at least one visible entity the params stay un-prefixed
        // and `mut` so the body can spawn into them without warnings.
        assert!(code.contains("mut commands: Commands"));
        assert!(code.contains("mut meshes: ResMut<Assets<Mesh>>"));
        // Spawned entity must include the per-scene marker so the
        // OnExit cleanup query picks it up.
        assert!(code.contains("Level1SceneEntity,"));
    }

    #[test]
    fn primitives_only_scene_underscores_asset_server() {
        // Regression: a scene with only Cube/Sphere/Plane primitives
        // doesn't reference `asset_server`, so the param must be
        // `_asset_server` to silence `unused_variables`. `mut commands`
        // / `mut meshes` / `mut materials` stay un-prefixed because
        // the spawn calls do use them.
        let mut scene = SceneModel::new();
        scene.add_entity(
            "Cube".into(),
            vec![ComponentData::MeshCube {
                size: 1.0,
                color: [1.0, 0.0, 0.0],
                metallic: 0.0,
                roughness: 0.5,
                emissive: [0.0, 0.0, 0.0],
                texture_path: None,
                normal_map_path: None,
            }],
        );
        let code = generate_scene_plugin_code(&scene, "primitives");
        assert!(code.contains("_asset_server: Res<AssetServer>"));
        assert!(!code.contains("    asset_server: Res<AssetServer>"));
        assert!(code.contains("mut commands: Commands"));
        assert!(code.contains("mut meshes: ResMut<Assets<Mesh>>"));
    }

    #[test]
    fn glb_loading_scene_keeps_asset_server_un_prefixed() {
        // Sanity check the inverse: a scene with a `MeshFromFile`
        // entity needs the `asset_server` to actually be in scope
        // without the leading underscore.
        let mut scene = SceneModel::new();
        scene.add_entity(
            "fox".into(),
            vec![ComponentData::MeshFromFile {
                path: "fox.glb".into(),
                texture_path: None,
                normal_map_path: None,
                auto_play_clip: None,
            }],
        );
        let code = generate_scene_plugin_code(&scene, "world");
        assert!(code.contains("    asset_server: Res<AssetServer>"));
        assert!(!code.contains("_asset_server: Res<AssetServer>"));
    }

    #[test]
    fn empty_scene_uses_underscore_prefix_to_avoid_warnings() {
        // Empty scenes don't reference commands / meshes / materials /
        // asset_server in `setup_*`, so the codegen must emit
        // `_`-prefixed names there — otherwise every freshly-`New`'d
        // scene generates a wave of `unused_variables` / `unused_mut`
        // warnings on `cargo check` until the user adds an entity.
        // The cleanup function legitimately uses `mut commands`
        // (despawn loop), so we scope the assertion to setup only.
        let scene = SceneModel::new();
        let code = generate_scene_plugin_code(&scene, "empty");
        let setup_block = code
            .split("fn cleanup_")
            .next()
            .expect("setup half must exist");
        assert!(setup_block.contains("_commands: Commands"));
        assert!(setup_block.contains("_meshes: ResMut<Assets<Mesh>>"));
        assert!(setup_block.contains("_materials: ResMut<Assets<StandardMaterial>>"));
        assert!(setup_block.contains("_asset_server: Res<AssetServer>"));
        assert!(!setup_block.contains("mut commands: Commands"));
        assert!(!setup_block.contains("mut meshes:"));
    }

    #[test]
    fn generate_scene_plugin_with_glb() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "fox".into(),
            vec![ComponentData::MeshFromFile {
                path: "/Users/test/project/assets/fox.glb".into(),
                texture_path: None,
                normal_map_path: None,
                auto_play_clip: None,
            }],
        );
        let code = generate_scene_plugin_code(&scene, "world");
        assert!(code.contains("SceneRoot(asset_server.load(\"fox.glb#Scene0\"))"));
        assert!(code.contains("asset_server: Res<AssetServer>"));
    }

    #[test]
    fn ensure_mod_declaration_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let mod_path = tmp.path().join("mod.rs");
        let mod_str = mod_path.to_string_lossy().to_string();

        ensure_mod_declaration(&mod_str, "player").unwrap();
        let content1 = std::fs::read_to_string(&mod_path).unwrap();
        assert!(content1.contains("pub mod player;"));

        // Second call should not duplicate
        ensure_mod_declaration(&mod_str, "player").unwrap();
        let content2 = std::fs::read_to_string(&mod_path).unwrap();
        assert_eq!(
            content2.matches("pub mod player;").count(),
            1,
            "Should not duplicate"
        );

        // Add another module
        ensure_mod_declaration(&mod_str, "enemy").unwrap();
        let content3 = std::fs::read_to_string(&mod_path).unwrap();
        assert!(content3.contains("pub mod player;"));
        assert!(content3.contains("pub mod enemy;"));
    }

    #[test]
    fn has_modular_structure_false_for_nonexistent() {
        assert!(!has_modular_structure("/nonexistent/path"));
    }

    #[test]
    fn has_modular_structure_true_when_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let scenes_dir = tmp.path().join("src/scenes");
        std::fs::create_dir_all(&scenes_dir).unwrap();
        std::fs::write(scenes_dir.join("mod.rs"), "").unwrap();
        assert!(has_modular_structure(&tmp.path().to_string_lossy()));
    }

    #[test]
    fn generate_scenes_mod_rs_aggregates_plugins() {
        // Scene plugin files now live at `<name>.rs` (no `_scene`
        // suffix). The scanner picks up every `.rs` except `mod.rs`.
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("game.rs"), "").unwrap();
        std::fs::write(tmp.path().join("title.rs"), "").unwrap();
        std::fs::write(tmp.path().join("mod.rs"), "").unwrap(); // ignored

        let code = generate_scenes_mod_rs(&tmp.path().to_string_lossy());
        assert!(code.contains("pub mod game;"));
        assert!(code.contains("pub mod title;"));
        assert!(!code.contains("pub mod mod;"));
        assert!(code.contains("pub struct ScenesPlugin;"));
        assert!(code.contains("game::GamePlugin"));
        assert!(code.contains("title::TitlePlugin"));
        // States-based scene switching: the enum has one variant per
        // module, and the alphabetically-first one is the default
        // (matches the editor's "first tab is active" behaviour).
        assert!(code.contains("pub enum AppScene"));
        assert!(code.contains("#[default]\n    Game,"));
        assert!(code.contains("Title,"));
        assert!(code.contains("init_state::<AppScene>()"));
    }

    #[test]
    fn generate_scenes_mod_rs_empty_emits_stub() {
        // No scene files yet → emit the no-op `ScenesPlugin` stub
        // without an `AppScene` enum (an empty enum can't derive
        // `Default`, and there's nothing for the user to switch to).
        let tmp = tempfile::tempdir().unwrap();
        let code = generate_scenes_mod_rs(&tmp.path().to_string_lossy());
        assert!(code.contains("pub struct ScenesPlugin;"));
        assert!(!code.contains("enum AppScene"));
        assert!(!code.contains("init_state"));
    }

    #[test]
    fn save_scene_code_modular_creates_files() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_string_lossy().to_string();
        std::fs::create_dir_all(format!("{}/src/scenes", root)).unwrap();

        let mut scene = SceneModel::new();
        scene.add_entity(
            "Cube".into(),
            vec![ComponentData::MeshCube {
                size: 1.0,
                color: [1.0, 0.0, 0.0],
                metallic: 0.0,
                roughness: 0.5,
                emissive: [0.0, 0.0, 0.0],
                texture_path: None,
                normal_map_path: None,
            }],
        );

        let result = save_scene_code_modular(&scene, "scenes/game.bscene", &root);
        assert!(result.is_ok());

        let rs_path = result.unwrap();
        // No `_scene` suffix — file matches `.bscene` stem 1:1.
        assert!(rs_path.ends_with("/game.rs"), "got {}", rs_path);
        assert!(std::path::Path::new(&rs_path).exists());

        let mod_rs = std::fs::read_to_string(format!("{}/src/scenes/mod.rs", root)).unwrap();
        assert!(mod_rs.contains("pub mod game;"));
        assert!(mod_rs.contains("game::GamePlugin"));
        assert!(mod_rs.contains("ScenesPlugin"));
        // Single-scene project still gets the AppScene enum so the
        // generated `<name>.rs` (which `use super::AppScene`) compiles.
        assert!(mod_rs.contains("enum AppScene"));
        assert!(mod_rs.contains("#[default]\n    Game,"));
    }

    #[test]
    fn save_scene_code_modular_removes_legacy_suffixed_file() {
        // Existing projects may have `<name>_scene.rs` from before the
        // naming convention changed; saving the same scene must drop
        // the legacy file so cargo doesn't see two scene plugins for
        // the same `.bscene` (which would compile into duplicate
        // module entries in `mod.rs`).
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_string_lossy().to_string();
        let scenes_dir = format!("{}/src/scenes", root);
        std::fs::create_dir_all(&scenes_dir).unwrap();
        let legacy = format!("{}/main_scene.rs", scenes_dir);
        std::fs::write(&legacy, "// stale codegen").unwrap();

        let scene = SceneModel::new();
        let result = save_scene_code_modular(&scene, "scenes/main.bscene", &root);
        assert!(result.is_ok());

        let rs_path = result.unwrap();
        assert!(rs_path.ends_with("/main.rs"));
        assert!(std::path::Path::new(&rs_path).exists(), "new file written");
        assert!(
            !std::path::Path::new(&legacy).exists(),
            "legacy {} should have been deleted",
            legacy
        );
    }

    // ===== Bevy 0.18 migration regression tests =====

    #[test]
    fn camera_entity_skipped_in_scene_plugin_codegen() {
        let mut scene = SceneModel::new();
        scene.add_entity("MainCamera".into(), vec![ComponentData::Camera]);
        scene.add_entity(
            "MyCube".into(),
            vec![ComponentData::MeshCube {
                size: 1.0,
                color: [1.0, 0.0, 0.0],
                metallic: 0.0,
                roughness: 0.5,
                emissive: [0.0, 0.0, 0.0],
                texture_path: None,
                normal_map_path: None,
            }],
        );
        let code = generate_scene_plugin_code(&scene, "test");
        // Camera entity should NOT appear in codegen (template already has one)
        assert!(
            !code.contains("Camera3d"),
            "Camera entity should be skipped in scene plugin codegen"
        );
        // Cube should still be generated
        assert!(
            code.contains("MyCube"),
            "Non-camera entities should still be generated"
        );
        assert!(code.contains("Cuboid"));
    }

    #[test]
    fn camera_entity_skipped_in_scene_plugin_code() {
        let mut scene = SceneModel::new();
        scene.add_entity("PlayerCam".into(), vec![ComponentData::Camera]);
        scene.add_entity(
            "Floor".into(),
            vec![ComponentData::MeshPlane {
                size: 10.0,
                color: [0.3, 0.3, 0.3],
                metallic: 0.0,
                roughness: 0.9,
                emissive: [0.0, 0.0, 0.0],
                texture_path: None,
                normal_map_path: None,
            }],
        );
        let code = generate_scene_plugin_code(&scene, "test");
        assert!(
            !code.contains("Camera3d"),
            "Camera should be skipped in generate_scene_plugin_code"
        );
        assert!(code.contains("Floor"));
        assert!(code.contains("Plane3d"));
    }

    #[test]
    fn glb_codegen_includes_scale_and_rotation() {
        let mut scene = SceneModel::new();
        let id = scene.add_entity(
            "MyModel".into(),
            vec![ComponentData::MeshFromFile {
                path: "/fake/path/assets/model.glb".into(),
                texture_path: None,
                normal_map_path: None,
                auto_play_clip: None,
            }],
        );
        // Set non-default transform
        if let Some(entity) = scene.entities.get_mut(&id) {
            entity.transform.scale = [0.5, 0.5, 0.5];
            entity.transform.rotation_euler = [0.0, 1.57, 0.0];
        }
        let code = generate_scene_plugin_code(&scene, "test");
        // Should contain SceneRoot with scale and rotation
        assert!(
            code.contains("with_scale"),
            "GLB codegen must include scale: {}",
            code
        );
        assert!(
            code.contains("with_rotation"),
            "GLB codegen must include rotation: {}",
            code
        );
        assert!(code.contains("SceneRoot"));
        assert!(code.contains("model.glb#Scene0"));
    }

    #[test]
    fn multiple_cameras_not_generated() {
        let mut scene = SceneModel::new();
        scene.add_entity("Cam1".into(), vec![ComponentData::Camera]);
        scene.add_entity("Cam2".into(), vec![ComponentData::Camera]);
        scene.add_entity(
            "Light".into(),
            vec![ComponentData::DirectionalLight {
                intensity: 10000.0,
                color: [1.0, 1.0, 1.0],
                shadows: true,
            }],
        );
        let code = generate_scene_plugin_code(&scene, "test");
        // No cameras should be in the output
        assert!(
            !code.contains("Camera3d"),
            "Multiple cameras must all be skipped"
        );
        // But light should be present
        assert!(code.contains("DirectionalLight"));
    }

    #[test]
    fn bevy_018_api_in_generated_code() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "Box".into(),
            vec![ComponentData::MeshCube {
                size: 2.0,
                color: [1.0, 1.0, 1.0],
                metallic: 0.0,
                roughness: 0.5,
                emissive: [0.0, 0.0, 0.0],
                texture_path: None,
                normal_map_path: None,
            }],
        );
        let code = generate_scene_plugin_code(&scene, "test");
        // Verify Bevy 0.18 API usage
        assert!(
            code.contains("Mesh3d("),
            "Should use Bevy 0.18 Mesh3d component"
        );
        assert!(
            code.contains("MeshMaterial3d("),
            "Should use Bevy 0.18 MeshMaterial3d component"
        );
        assert!(code.contains("use bevy::prelude::*;"));
    }

    // ──────────────────────────────────────────────────────────────────────
    // GLB skeletal animation (v0.5.4)
    // ──────────────────────────────────────────────────────────────────────

    /// Scene with no GLB → none of the animation scaffolding leaks in.
    /// Guards against the helper module bloating every scene plugin.
    #[test]
    fn no_glb_scene_omits_animation_scaffolding() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "Box".into(),
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
        let code = generate_scene_plugin_code_with_root(&scene, "scene", "");

        assert!(!code.contains("PendingGlbAnim"), "marker should not appear");
        assert!(
            !code.contains("GlbAnimGraphs"),
            "resource should not appear"
        );
        assert!(
            !code.contains("AnimationGraph"),
            "anim import should not appear"
        );
        assert!(
            !code.contains("init_resource::"),
            "no resource init for non-GLB scenes"
        );
        assert!(!code.contains("use bevy::gltf::Gltf;"), "no gltf import");
    }

    /// Scene with a GLB and no clip choice emits the helper scaffolding and
    /// passes `clip: None` so the runtime falls back to the first clip
    /// alphabetically — same behaviour the editor preview shows by default.
    #[test]
    fn glb_scene_without_clip_choice_emits_clip_none() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "fox".into(),
            vec![ComponentData::MeshFromFile {
                path: "/proj/assets/fox.glb".into(),
                texture_path: None,
                normal_map_path: None,
                auto_play_clip: None,
            }],
        );
        let code = generate_scene_plugin_code_with_root(&scene, "scene", "");

        // Helper scaffolding emitted
        assert!(
            code.contains("ScenePendingGlbAnim"),
            "marker struct emitted"
        );
        assert!(
            code.contains("SceneGlbAnimGraphs"),
            "graphs resource emitted"
        );
        assert!(code.contains("init_resource::<SceneGlbAnimGraphs>()"));
        assert!(code.contains("scene_build_glb_anim_graphs"));
        assert!(code.contains("scene_attach_glb_anim_players"));
        assert!(
            code.contains(".add_systems(Update, (scene_build_glb_anim_graphs, scene_attach_glb_anim_players.after(scene_build_glb_anim_graphs)))"),
            "systems registered with build→attach ordering"
        );

        // Anim-related imports appear
        assert!(code.contains("use bevy::animation::graph::{AnimationGraph"));
        assert!(code.contains("use bevy::animation::{AnimationClip, AnimationPlayer};"));
        assert!(code.contains("use bevy::gltf::Gltf;"));
        assert!(code.contains("use bevy::asset::AssetId;"));

        // Spawn site embeds the marker with clip: None
        assert!(
            code.contains("ScenePendingGlbAnim {")
                && code.contains("gltf: asset_server.load(\"fox.glb\")")
                && code.contains("clip: None,"),
            "MeshFromFile spawn carries marker with clip: None\n\n{}",
            code
        );
    }

    /// User picked "Walk" in the Inspector → that exact name shows up in
    /// the generated `clip` field so `cargo run` plays it (not the
    /// alphabetical default).
    #[test]
    fn glb_scene_with_clip_choice_persists_to_codegen() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "fox".into(),
            vec![ComponentData::MeshFromFile {
                path: "/proj/assets/fox.glb".into(),
                texture_path: None,
                normal_map_path: None,
                auto_play_clip: Some("Walk".into()),
            }],
        );
        let code = generate_scene_plugin_code_with_root(&scene, "scene", "");
        assert!(
            code.contains("clip: Some(\"Walk\".to_string()),"),
            "clip name must round-trip into runtime spawn\n\n{}",
            code
        );
    }

    /// Two scenes in the same project must each get their own resource /
    /// marker / system names so the runtime app can register both plugins
    /// without colliding on a single `<X>GlbAnimGraphs` type.
    #[test]
    fn glb_helper_names_are_per_scene_pascal_prefixed() {
        let mut scene_a = SceneModel::new();
        scene_a.add_entity(
            "fox".into(),
            vec![ComponentData::MeshFromFile {
                path: "/proj/assets/fox.glb".into(),
                texture_path: None,
                normal_map_path: None,
                auto_play_clip: None,
            }],
        );
        let scene_b = scene_a.clone();
        let code_a = generate_scene_plugin_code_with_root(&scene_a, "scene", "");
        let code_b = generate_scene_plugin_code_with_root(&scene_b, "level_two", "");

        assert!(code_a.contains("SceneGlbAnimGraphs"));
        assert!(code_a.contains("ScenePendingGlbAnim"));
        assert!(code_a.contains("scene_build_glb_anim_graphs"));

        assert!(code_b.contains("LevelTwoGlbAnimGraphs"));
        assert!(code_b.contains("LevelTwoPendingGlbAnim"));
        assert!(code_b.contains("level_two_build_glb_anim_graphs"));

        // No cross-contamination
        assert!(
            !code_a.contains("LevelTwo"),
            "scene plugin must not reference another scene's prefix"
        );
        assert!(!code_b.contains("ScenePendingGlbAnim"));
    }

    // ──────────────────────────────────────────────────────────────────────
    // Physics codegen (avian3d) — Inspector Collider/RigidBody must
    // round-trip into runnable physics components.
    // ──────────────────────────────────────────────────────────────────────

    #[test]
    fn collider_box_emits_avian_cuboid_with_full_extents() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "Wall".into(),
            vec![ComponentData::Collider {
                shape: ColliderShape::Box {
                    // half_extents → cuboid takes full side lengths.
                    half_extents: [0.75, 1.0, 0.5],
                },
                friction: 0.6,
                restitution: 0.1,
            }],
        );
        let code = generate_scene_plugin_code_with_root(&scene, "scene", "");
        assert!(
            code.contains("avian3d::prelude::Collider::cuboid(1.500, 2.000, 1.000)"),
            "Box collider must double half_extents to full lengths\n\n{}",
            code
        );
        assert!(code.contains("avian3d::prelude::Friction::new(0.600)"));
        assert!(code.contains("avian3d::prelude::Restitution::new(0.100)"));
    }

    #[test]
    fn collider_sphere_emits_avian_sphere() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "Ball".into(),
            vec![ComponentData::Collider {
                shape: ColliderShape::Sphere { radius: 0.42 },
                friction: 0.5,
                restitution: 0.0,
            }],
        );
        let code = generate_scene_plugin_code_with_root(&scene, "scene", "");
        assert!(code.contains("avian3d::prelude::Collider::sphere(0.420)"));
    }

    #[test]
    fn collider_capsule_emits_avian_capsule_with_full_length() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "Person".into(),
            vec![ComponentData::Collider {
                // half_height → capsule takes full segment length.
                shape: ColliderShape::Capsule {
                    half_height: 0.45,
                    radius: 0.4,
                },
                friction: 0.5,
                restitution: 0.0,
            }],
        );
        let code = generate_scene_plugin_code_with_root(&scene, "scene", "");
        assert!(
            code.contains("avian3d::prelude::Collider::capsule(0.400, 0.900)"),
            "Capsule expects (radius, length=2*half_height)\n\n{}",
            code
        );
    }

    #[test]
    fn rigidbody_variants_each_map_to_avian_variant() {
        for (variant, expected) in [
            (RigidBodyType::Static, "RigidBody::Static"),
            (RigidBodyType::Dynamic, "RigidBody::Dynamic"),
            (RigidBodyType::Kinematic, "RigidBody::Kinematic"),
        ] {
            let mut scene = SceneModel::new();
            scene.add_entity(
                "X".into(),
                vec![ComponentData::RigidBody {
                    body_type: variant,
                    mass: 1.0,
                }],
            );
            let code = generate_scene_plugin_code_with_root(&scene, "scene", "");
            assert!(
                code.contains(&format!("avian3d::prelude::{},", expected)),
                "{:?} should emit avian3d::prelude::{}\n\n{}",
                variant,
                expected,
                code
            );
        }
    }

    // ──────────────────────────────────────────────────────────────────────
    // PlayerController (v0.5.6)
    // ──────────────────────────────────────────────────────────────────────

    #[test]
    fn player_controller_emits_super_struct_with_inspector_values() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "Hero".into(),
            vec![ComponentData::PlayerController {
                speed: 7.5,
                jump_velocity: 12.0,
                run_multiplier: 1.8,
                turn_speed: 9.0,
            }],
        );
        let code = generate_scene_plugin_code_with_root(&scene, "scene", "");
        assert!(
            code.contains("super::PlayerController {")
                && code.contains("speed: 7.500,")
                && code.contains("jump_velocity: 12.000,")
                && code.contains("run_multiplier: 1.800,"),
            "PlayerController must emit `super::PlayerController` with inspector values\n\n{}",
            code
        );
    }

    #[test]
    fn shared_player_controller_struct_emitted_in_mod_rs() {
        // Empty scenes dir → still emit the shared struct so the user's
        // movement system can reference `scenes::PlayerController` even
        // before they add a player to a scene.
        let tmp = tempfile::tempdir().unwrap();
        let code = generate_scenes_mod_rs(&tmp.path().to_string_lossy());
        assert!(
            code.contains("pub struct PlayerController {")
                && code.contains("pub speed: f32,")
                && code.contains("pub jump_velocity: f32,")
                && code.contains("pub run_multiplier: f32,"),
            "scenes/mod.rs must always declare the shared PlayerController\n\n{}",
            code
        );
    }

    /// Scene without any Collider/RigidBody must not pull in avian
    /// references — keeps non-physics scenes cheap and prevents accidental
    /// `use avian3d` requirements on projects that haven't added the dep.
    #[test]
    fn no_physics_scene_does_not_reference_avian() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "Box".into(),
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
        let code = generate_scene_plugin_code_with_root(&scene, "scene", "");
        assert!(
            !code.contains("avian3d"),
            "non-physics scene leaked an avian reference"
        );
    }

    /// The attach helper must use the direct `player.stop_all()` +
    /// `player.play()` path. We learned the hard way that
    /// `AnimationTransitions::play` looked like it succeeded but didn't
    /// actually swap the active clip — leaving the runtime stuck on the
    /// initial animation forever.
    #[test]
    fn attach_helper_uses_direct_play_not_transitions() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "fox".into(),
            vec![ComponentData::MeshFromFile {
                path: "/proj/assets/fox.glb".into(),
                texture_path: None,
                normal_map_path: None,
                auto_play_clip: None,
            }],
        );
        let code = generate_scene_plugin_code_with_root(&scene, "scene", "");
        assert!(code.contains("player.stop_all();"));
        assert!(code.contains("player.play(node)"));
        assert!(code.contains(".repeat();"));
        assert!(
            !code.contains("AnimationTransitions"),
            "transitions layer must be bypassed in the runtime helper"
        );
    }
}
