//! Reverse code import: parse a _scene.rs file back into SceneModel.
//!
//! Since we control the generated format, we can parse it with simple
//! line-by-line regex matching rather than full Rust AST parsing.

use super::model::*;
use regex::Regex;

/// Normalize code: strip comments, collapse whitespace, and inline
/// simple `let var = meshes.add(Cuboid::new(...))` bindings so that
/// `Mesh3d(var.clone())` blocks contain the actual mesh type.
fn normalize_code(code: &str) -> String {
    // First pass: collect let bindings for mesh/material variables
    // Handle nested parentheses: let var = meshes.add(Cuboid::new(1.5, 1.5, 1.5));
    let mut var_replacements: Vec<(String, String)> = Vec::new();
    let let_start_re = Regex::new(r"let\s+(\w+)\s*=\s*meshes\.add\(").expect("valid regex");
    for cap in let_start_re.captures_iter(code) {
        let var_name = cap[1].to_string();
        let match_end = cap.get(0).map(|m| m.end()).unwrap_or(0);
        // Find the matching closing paren by counting depth
        let remainder = &code[match_end..];
        let mut depth = 1i32;
        let mut end_pos = 0;
        for (i, ch) in remainder.chars().enumerate() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        end_pos = i;
                        break;
                    }
                }
                _ => {}
            }
        }
        let mesh_expr = &remainder[..end_pos];
        var_replacements.push((
            format!("{}.clone()", var_name),
            format!("meshes.add({})", mesh_expr),
        ));
        var_replacements.push((var_name, format!("meshes.add({})", mesh_expr)));
    }

    let mut result = String::new();
    let mut in_block_comment = false;

    for line in code.lines() {
        let trimmed = line.trim();

        if in_block_comment {
            if let Some(pos) = trimmed.find("*/") {
                in_block_comment = false;
                let after = trimmed[pos + 2..].trim();
                if !after.is_empty() {
                    result.push_str(after);
                    result.push(' ');
                }
            }
            continue;
        }

        // Check for block comment start
        let mut remaining = trimmed;
        if let Some(pos) = remaining.find("/*") {
            let before = remaining[..pos].trim();
            if !before.is_empty() {
                result.push_str(before);
                result.push(' ');
            }
            // Check if the block comment closes on the same line
            let after_open = &remaining[pos + 2..];
            if let Some(close_pos) = after_open.find("*/") {
                remaining = after_open[close_pos + 2..].trim();
                // Continue processing remaining text (fall through)
            } else {
                in_block_comment = true;
                continue;
            }
        }

        // Strip single-line comments, but preserve [BerryCode:...] markers
        if remaining.starts_with("//") {
            if remaining.contains("[BerryCode:") {
                // Preserve marker lines by emitting just the marker portion
                result.push_str(remaining);
                result.push(' ');
            }
            continue;
        }
        let no_comment = if let Some(pos) = remaining.find("//") {
            // Check if the comment contains a BerryCode marker
            let comment_part = &remaining[pos..];
            if comment_part.contains("[BerryCode:") {
                // Keep the full line including the marker comment
                remaining
            } else {
                &remaining[..pos]
            }
        } else {
            remaining
        };
        let no_comment = no_comment.trim();
        if !no_comment.is_empty() {
            result.push_str(no_comment);
            result.push(' ');
        }
    }

    // Apply variable inlining (longest match first to avoid partial replacements)
    let mut sorted_replacements = var_replacements;
    sorted_replacements.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
    for (from, to) in &sorted_replacements {
        result = result.replace(from.as_str(), to.as_str());
    }

    result
}

/// Parse a generated _scene.rs file and create a SceneModel.
/// Also handles minor hand-edits thanks to `normalize_code()`.
pub fn import_scene_from_code(code: &str) -> SceneModel {
    let mut scene = SceneModel::new();

    // Normalize first so regex patterns tolerate whitespace / comment variations
    let normalized = normalize_code(code);

    // Pattern: commands.spawn(( followed by Transform, components, Name
    // We split on "commands.spawn((" to find entity blocks
    let entity_blocks: Vec<&str> = normalized.split("commands.spawn((").skip(1).collect();

    for block in entity_blocks {
        let mut name = String::from("Entity");
        let mut transform = TransformData::default();
        let mut components: Vec<ComponentData> = Vec::new();

        // Parse Name
        let name_re = Regex::new(r#"Name::new\("([^"]+)"\)"#).expect("valid regex");
        if let Some(cap) = name_re.captures(block) {
            name = cap[1].to_string();
        } else {
            // Auto-name based on primary component when no explicit Name
            if block.contains("Camera3d::") || block.contains("Camera2d") {
                name = "Camera".to_string();
            } else if block.contains("DirectionalLight") {
                name = "DirectionalLight".to_string();
            } else if block.contains("PointLight") {
                name = "PointLight".to_string();
            } else if block.contains("Plane3d::") {
                name = "Ground".to_string();
            } else if block.contains("Cuboid::") {
                name = "Cube".to_string();
            } else if block.contains("Sphere::") {
                name = "Sphere".to_string();
            }
        }

        // Parse Transform
        let transform_re = Regex::new(r"Transform::from_xyz\(([^,]+),\s*([^,]+),\s*([^)]+)\)")
            .expect("valid regex");
        if let Some(cap) = transform_re.captures(block) {
            transform.translation = [
                cap[1].trim().parse().unwrap_or(0.0),
                cap[2].trim().parse().unwrap_or(0.0),
                cap[3].trim().parse().unwrap_or(0.0),
            ];
        }

        // Parse rotation if present (with_rotation or from_rotation)
        let rotation_re = Regex::new(
            r"(?:with_rotation|from_rotation)\(Quat::from_euler\(EulerRot::XYZ,\s*([^,]+),\s*([^,]+),\s*([^)]+)\)\)",
        )
        .expect("valid regex");
        if let Some(cap) = rotation_re.captures(block) {
            transform.rotation_euler = [
                cap[1].trim().parse().unwrap_or(0.0),
                cap[2].trim().parse().unwrap_or(0.0),
                cap[3].trim().parse().unwrap_or(0.0),
            ];
        }

        // Parse scale if present
        let scale_re = Regex::new(r"with_scale\(Vec3::new\(([^,]+),\s*([^,]+),\s*([^)]+)\)\)")
            .expect("valid regex");
        if let Some(cap) = scale_re.captures(block) {
            transform.scale = [
                cap[1].trim().parse().unwrap_or(1.0),
                cap[2].trim().parse().unwrap_or(1.0),
                cap[3].trim().parse().unwrap_or(1.0),
            ];
        }

        // Parse Cuboid (MeshCube) - both Cuboid::new(...) and Cuboid::default()
        let cuboid_re =
            Regex::new(r"Cuboid::new\(([^,]+),\s*([^,]+),\s*([^)]+)\)").expect("valid regex");
        let has_cuboid_new = cuboid_re.is_match(block);
        if let Some(cap) = cuboid_re.captures(block) {
            let size: f32 = cap[1].trim().parse().unwrap_or(1.0);
            let color = parse_srgb_color(block);
            let (metallic, roughness) = parse_pbr_params(block);
            components.push(ComponentData::MeshCube {
                size,
                color,
                metallic,
                roughness,
                emissive: [0.0, 0.0, 0.0],
                texture_path: None,
                normal_map_path: None,
            });
        } else if block.contains("Cuboid::default()") {
            let color = parse_srgb_color(block);
            let (metallic, roughness) = parse_pbr_params(block);
            components.push(ComponentData::MeshCube {
                size: 1.0,
                color,
                metallic,
                roughness,
                emissive: [0.0, 0.0, 0.0],
                texture_path: None,
                normal_map_path: None,
            });
        }

        // Parse Sphere (only if no Cuboid to avoid false matches)
        let sphere_re = Regex::new(r"Sphere::new\(([^)]+)\)").expect("valid regex");
        if !has_cuboid_new && !block.contains("Cuboid::default()") {
            if let Some(cap) = sphere_re.captures(block) {
                let radius: f32 = cap[1].trim().parse().unwrap_or(0.5);
                let color = parse_srgb_color(block);
                let (metallic, roughness) = parse_pbr_params(block);
                components.push(ComponentData::MeshSphere {
                    radius,
                    color,
                    metallic,
                    roughness,
                    emissive: [0.0, 0.0, 0.0],
                    texture_path: None,
                    normal_map_path: None,
                });
            }
        }

        // Parse Plane
        if block.contains("Plane3d::") {
            let plane_re = Regex::new(r"\.size\(([^,]+),\s*([^)]+)\)").expect("valid regex");
            if let Some(cap) = plane_re.captures(block) {
                let size: f32 = cap[1].trim().parse().unwrap_or(10.0);
                let color = parse_srgb_color(block);
                components.push(ComponentData::MeshPlane {
                    size,
                    color,
                    metallic: 0.0,
                    roughness: 0.5,
                    emissive: [0.0, 0.0, 0.0],
                    texture_path: None,
                    normal_map_path: None,
                });
            }
        }

        // Parse PointLight
        let light_re =
            Regex::new(r"PointLight\s*\{[^}]*intensity:\s*([^,]+)").expect("valid regex");
        if let Some(cap) = light_re.captures(block) {
            let intensity: f32 = cap[1].trim().parse().unwrap_or(10000.0);
            let color = parse_light_color(block);
            components.push(ComponentData::Light { intensity, color });
        }

        // Parse DirectionalLight
        let dir_light_re =
            Regex::new(r"DirectionalLight\s*\{[^}]*illuminance:\s*([^,]+)").expect("valid regex");
        if let Some(cap) = dir_light_re.captures(block) {
            let intensity: f32 = cap[1].trim().parse().unwrap_or(10000.0);
            let shadows = block.contains("shadows_enabled: true");
            let color = parse_light_color(block);
            components.push(ComponentData::DirectionalLight {
                intensity,
                color,
                shadows,
            });
        }

        // Parse Camera3d
        if block.contains("Camera3d::") {
            components.push(ComponentData::Camera);
        }

        // Parse CustomScript blocks (TypeName { field: value, ... })
        // After normalization everything is on one line, so we match braced blocks.
        // Also works on the original multi-line format thanks to the `(?s)` flag.
        let custom_re = Regex::new(r"(\b[A-Z][a-zA-Z0-9]+)\s*\{([^}]*)\}").expect("valid regex");
        for cap in custom_re.captures_iter(block) {
            let type_name = cap[1].to_string();
            // Skip known Bevy types
            if [
                "Transform",
                "StandardMaterial",
                "PointLight",
                "DirectionalLight",
                "Name",
                "Mesh3d",
                "MeshMaterial3d",
            ]
            .contains(&type_name.as_str())
            {
                continue;
            }
            let field_block = &cap[2];
            let mut fields = Vec::new();
            // Match field: value patterns (works both newline-separated and space-separated)
            let field_re = Regex::new(r"(\w+):\s*([^,}]+)").expect("valid regex");
            for fcap in field_re.captures_iter(field_block) {
                let fname = fcap[1].to_string();
                // Skip known Bevy material/light fields and ..default()
                if [
                    "base_color",
                    "metallic",
                    "perceptual_roughness",
                    "intensity",
                    "color",
                    "illuminance",
                    "shadows_enabled",
                ]
                .contains(&fname.as_str())
                {
                    continue;
                }
                if fname == "default" {
                    continue;
                }
                let fval = fcap[2].trim().trim_end_matches(',').to_string();
                if fval.starts_with("..") || fval.contains("default()") {
                    continue;
                }
                let value = parse_script_value(&fval);
                fields.push(ScriptField { name: fname, value });
            }
            if !fields.is_empty() {
                components.push(ComponentData::CustomScript { type_name, fields });
            }
        }

        // Parse BerryCode component markers
        let marker_re = Regex::new(r"\[BerryCode:(\w+)\]\s*(.*)").expect("valid regex");
        for cap in marker_re.captures_iter(block) {
            let type_name = &cap[1];
            let params = &cap[2];
            match type_name {
                "SpotLight" => {
                    components.push(ComponentData::SpotLight {
                        intensity: parse_param_f32(params, "intensity", 10000.0),
                        color: [
                            parse_param_f32(params, "color_r", 1.0),
                            parse_param_f32(params, "color_g", 1.0),
                            parse_param_f32(params, "color_b", 1.0),
                        ],
                        range: parse_param_f32(params, "range", 20.0),
                        inner_angle: parse_param_f32(params, "inner_angle", 0.5),
                        outer_angle: parse_param_f32(params, "outer_angle", 0.8),
                    });
                }
                "MeshFromFile" => {
                    components.push(ComponentData::MeshFromFile {
                        path: parse_param_str(params, "path", ""),
                        texture_path: None,
                        normal_map_path: None,
                    });
                }
                "AudioSource" => {
                    components.push(ComponentData::AudioSource {
                        path: parse_param_str(params, "path", ""),
                        volume: parse_param_f32(params, "volume", 1.0),
                        looped: parse_param_bool(params, "looped", false),
                        autoplay: parse_param_bool(params, "autoplay", true),
                    });
                }
                "AudioListener" => {
                    components.push(ComponentData::AudioListener);
                }
                "RigidBody" => {
                    let bt_str = parse_param_str(params, "body_type", "Dynamic");
                    let body_type = match bt_str.as_str() {
                        "Static" => RigidBodyType::Static,
                        "Kinematic" => RigidBodyType::Kinematic,
                        _ => RigidBodyType::Dynamic,
                    };
                    components.push(ComponentData::RigidBody {
                        body_type,
                        mass: parse_param_f32(params, "mass", 1.0),
                    });
                }
                "Collider" => {
                    let shape_str = parse_param_str(params, "shape", "Box");
                    let shape = match shape_str.as_str() {
                        "Sphere" => ColliderShape::Sphere {
                            radius: parse_param_f32(params, "radius", 0.5),
                        },
                        "Capsule" => ColliderShape::Capsule {
                            half_height: parse_param_f32(params, "half_height", 0.5),
                            radius: parse_param_f32(params, "radius", 0.5),
                        },
                        _ => ColliderShape::Box {
                            half_extents: [
                                parse_param_f32(params, "half_x", 0.5),
                                parse_param_f32(params, "half_y", 0.5),
                                parse_param_f32(params, "half_z", 0.5),
                            ],
                        },
                    };
                    components.push(ComponentData::Collider {
                        shape,
                        friction: parse_param_f32(params, "friction", 0.5),
                        restitution: parse_param_f32(params, "restitution", 0.0),
                    });
                }
                "UiText" => {
                    components.push(ComponentData::UiText {
                        text: parse_param_str(params, "text", "Text"),
                        font_size: parse_param_f32(params, "font_size", 16.0),
                        color: [
                            parse_param_f32(params, "color_r", 1.0),
                            parse_param_f32(params, "color_g", 1.0),
                            parse_param_f32(params, "color_b", 1.0),
                            parse_param_f32(params, "color_a", 1.0),
                        ],
                    });
                }
                "UiButton" => {
                    components.push(ComponentData::UiButton {
                        label: parse_param_str(params, "label", "Button"),
                        background: [
                            parse_param_f32(params, "bg_r", 0.2),
                            parse_param_f32(params, "bg_g", 0.2),
                            parse_param_f32(params, "bg_b", 0.3),
                            parse_param_f32(params, "bg_a", 1.0),
                        ],
                    });
                }
                "UiImage" => {
                    components.push(ComponentData::UiImage {
                        path: parse_param_str(params, "path", ""),
                        tint: [
                            parse_param_f32(params, "tint_r", 1.0),
                            parse_param_f32(params, "tint_g", 1.0),
                            parse_param_f32(params, "tint_b", 1.0),
                            parse_param_f32(params, "tint_a", 1.0),
                        ],
                    });
                }
                "ParticleEmitter" => {
                    components.push(ComponentData::ParticleEmitter {
                        rate: parse_param_f32(params, "rate", 30.0),
                        lifetime: parse_param_f32(params, "lifetime", 1.5),
                        speed: parse_param_f32(params, "speed", 2.0),
                        spread: parse_param_f32(params, "spread", 0.3),
                        start_size: parse_param_f32(params, "start_size", 0.1),
                        end_size: parse_param_f32(params, "end_size", 0.0),
                        start_color: [
                            parse_param_f32(params, "sc_r", 1.0),
                            parse_param_f32(params, "sc_g", 0.6),
                            parse_param_f32(params, "sc_b", 0.2),
                            parse_param_f32(params, "sc_a", 1.0),
                        ],
                        end_color: [
                            parse_param_f32(params, "ec_r", 1.0),
                            parse_param_f32(params, "ec_g", 0.0),
                            parse_param_f32(params, "ec_b", 0.0),
                            parse_param_f32(params, "ec_a", 0.0),
                        ],
                        max_particles: parse_param_u32(params, "max_particles", 200),
                        gravity: parse_param_f32(params, "gravity", -1.0),
                    });
                }
                "Animation" => {
                    components.push(ComponentData::Animation {
                        duration: parse_param_f32(params, "duration", 2.0),
                        looped: parse_param_bool(params, "looped", true),
                        tracks: vec![],
                    });
                }
                "CustomScript" => {
                    let cs_type_name = parse_param_str(params, "type_name", "");
                    let field_count = parse_param_u32(params, "fields", 0);
                    // Parse subsequent [BerryCode:CustomField] markers for this script
                    let mut fields = Vec::new();
                    let field_re = Regex::new(
                        r"\[BerryCode:CustomField\]\s*name=(\S+)\s+value=(.+?)(?:\s*//|\s*$)",
                    )
                    .expect("valid regex");
                    // We need to re-scan for CustomField markers in the block
                    // They appear after the CustomScript marker
                    let cf_re = Regex::new(r"\[BerryCode:CustomField\]\s+name=(\S+)\s+value=(.+?)(?:\s+//\s*\[BerryCode|\s*$)").expect("valid regex");
                    // Simpler approach: find all CustomField markers sequentially
                    let cf_simple =
                        Regex::new(r"\[BerryCode:CustomField\]\s+name=(\S+)\s+value=(\S+)")
                            .expect("valid regex");
                    for fcap in cf_simple.captures_iter(block) {
                        if fields.len() >= field_count as usize {
                            break;
                        }
                        let fname = fcap[1].to_string();
                        let fval_str = fcap[2].to_string();
                        let value = parse_script_value(&fval_str);
                        fields.push(ScriptField { name: fname, value });
                    }
                    components.push(ComponentData::CustomScript {
                        type_name: cs_type_name,
                        fields,
                    });
                }
                "Skybox" => {
                    components.push(ComponentData::Skybox {
                        path: parse_param_str(params, "path", ""),
                    });
                }
                "Animator" => {
                    components.push(ComponentData::Animator {
                        controller_path: parse_param_str(params, "controller_path", ""),
                    });
                }
                "LodGroup" => {
                    components.push(ComponentData::LodGroup { levels: vec![] });
                }
                "Spline" => {
                    components.push(ComponentData::Spline {
                        points: vec![],
                        closed: parse_param_bool(params, "closed", false),
                    });
                }
                "Terrain" => {
                    let resolution = parse_param_u32(params, "resolution", 64);
                    components.push(ComponentData::Terrain {
                        resolution,
                        world_size: [
                            parse_param_f32(params, "world_w", 100.0),
                            parse_param_f32(params, "world_h", 100.0),
                        ],
                        heights: vec![0.0; (resolution * resolution) as usize],
                        base_color: [
                            parse_param_f32(params, "base_r", 0.3),
                            parse_param_f32(params, "base_g", 0.5),
                            parse_param_f32(params, "base_b", 0.3),
                        ],
                    });
                }
                "SkinnedMesh" => {
                    components.push(ComponentData::SkinnedMesh {
                        path: parse_param_str(params, "path", ""),
                        bones: vec![],
                    });
                }
                "VisualScript" => {
                    components.push(ComponentData::VisualScript {
                        path: parse_param_str(params, "path", ""),
                    });
                }
                "NavMesh" => {
                    let width = parse_param_usize(params, "width", 0);
                    let height = parse_param_usize(params, "height", 0);
                    components.push(ComponentData::NavMesh {
                        cell_size: parse_param_f32(params, "cell_size", 1.0),
                        grid: vec![],
                        width,
                        height,
                    });
                }
                "CustomField" => {
                    // Handled as part of CustomScript parsing above; skip here
                }
                _ => {}
            }
        }

        let id = scene.add_entity(name, components);
        if let Some(e) = scene.entities.get_mut(&id) {
            e.transform = transform;
        }
    }

    scene
}

/// Parse a float parameter from a `key=value` parameter string.
fn parse_param_f32(params: &str, key: &str, default: f32) -> f32 {
    let pattern = format!("{}=", key);
    params
        .split_whitespace()
        .find(|s| s.starts_with(&pattern))
        .and_then(|s| s[pattern.len()..].parse().ok())
        .unwrap_or(default)
}

/// Parse a boolean parameter from a `key=value` parameter string.
fn parse_param_bool(params: &str, key: &str, default: bool) -> bool {
    let pattern = format!("{}=", key);
    params
        .split_whitespace()
        .find(|s| s.starts_with(&pattern))
        .map(|s| &s[pattern.len()..] == "true")
        .unwrap_or(default)
}

/// Parse a string parameter from a `key=value` parameter string.
fn parse_param_str(params: &str, key: &str, default: &str) -> String {
    let pattern = format!("{}=", key);
    params
        .split_whitespace()
        .find(|s| s.starts_with(&pattern))
        .map(|s| s[pattern.len()..].to_string())
        .unwrap_or_else(|| default.to_string())
}

/// Parse a u32 parameter from a `key=value` parameter string.
fn parse_param_u32(params: &str, key: &str, default: u32) -> u32 {
    let pattern = format!("{}=", key);
    params
        .split_whitespace()
        .find(|s| s.starts_with(&pattern))
        .and_then(|s| s[pattern.len()..].parse().ok())
        .unwrap_or(default)
}

/// Parse a usize parameter from a `key=value` parameter string.
fn parse_param_usize(params: &str, key: &str, default: usize) -> usize {
    let pattern = format!("{}=", key);
    params
        .split_whitespace()
        .find(|s| s.starts_with(&pattern))
        .and_then(|s| s[pattern.len()..].parse().ok())
        .unwrap_or(default)
}

/// Parse a single value string into a [`ScriptValue`].
/// Handles primitives as well as `vec![...]`, `Some(...)`, `None`,
/// and `HashMap::from([...])` / `HashMap::new()`.
fn parse_script_value(s: &str) -> ScriptValue {
    let s = s.trim();
    if s == "true" || s == "false" {
        return ScriptValue::Bool(s == "true");
    }
    if s == "None" {
        return ScriptValue::Option(None);
    }
    if let Some(inner) = s.strip_prefix("Some(").and_then(|r| r.strip_suffix(')')) {
        return ScriptValue::Option(Some(Box::new(parse_script_value(inner))));
    }
    if let Some(inner) = s.strip_prefix("vec![").and_then(|r| r.strip_suffix(']')) {
        if inner.trim().is_empty() {
            return ScriptValue::Vec(vec![]);
        }
        let items: Vec<ScriptValue> = split_top_level(inner)
            .iter()
            .map(|item| parse_script_value(item))
            .collect();
        return ScriptValue::Vec(items);
    }
    if s == "HashMap::new()" {
        return ScriptValue::Map(vec![]);
    }
    if let Some(inner) = s
        .strip_prefix("HashMap::from([")
        .and_then(|r| r.strip_suffix("])"))
    {
        let entries: Vec<(String, ScriptValue)> = split_top_level(inner)
            .iter()
            .filter_map(|item| {
                // Each item is like ("key", val)
                let item = item.trim();
                let item = item.strip_prefix('(')?.strip_suffix(')')?;
                let parts = split_top_level(item);
                if parts.len() >= 2 {
                    let key = parts[0].trim().trim_matches('"').to_string();
                    let val = parse_script_value(parts[1].trim());
                    Some((key, val))
                } else {
                    None
                }
            })
            .collect();
        return ScriptValue::Map(entries);
    }
    if let Ok(f) = s.parse::<f64>() {
        if s.contains('.') {
            ScriptValue::Float(f as f32)
        } else if let Ok(i) = s.parse::<i64>() {
            ScriptValue::Int(i)
        } else {
            ScriptValue::Float(f as f32)
        }
    } else {
        ScriptValue::String(s.trim_matches('"').to_string())
    }
}

/// Split a string by commas, but only at the top level (respecting nested
/// parentheses, brackets, and quotes).
fn split_top_level(s: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut depth = 0i32;
    let mut start = 0;
    let mut in_string = false;
    let bytes = s.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'"' && (i == 0 || bytes[i - 1] != b'\\') {
            in_string = !in_string;
        }
        if in_string {
            continue;
        }
        match b {
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth -= 1,
            b',' if depth == 0 => {
                result.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    let remainder = s[start..].trim();
    if !remainder.is_empty() {
        result.push(&s[start..]);
    }
    result
}

fn parse_srgb_color(block: &str) -> [f32; 3] {
    let re = Regex::new(r"Color::srgb\(([^,]+),\s*([^,]+),\s*([^)]+)\)").expect("valid regex");
    if let Some(cap) = re.captures(block) {
        [
            cap[1].trim().parse().unwrap_or(0.5),
            cap[2].trim().parse().unwrap_or(0.5),
            cap[3].trim().parse().unwrap_or(0.5),
        ]
    } else {
        [0.5, 0.5, 0.5]
    }
}

fn parse_pbr_params(block: &str) -> (f32, f32) {
    let met_re = Regex::new(r"metallic:\s*([0-9.]+)").expect("valid regex");
    let rough_re = Regex::new(r"perceptual_roughness:\s*([0-9.]+)").expect("valid regex");
    let metallic = met_re
        .captures(block)
        .and_then(|c| c[1].parse().ok())
        .unwrap_or(0.0);
    let roughness = rough_re
        .captures(block)
        .and_then(|c| c[1].parse().ok())
        .unwrap_or(0.5);
    (metallic, roughness)
}

fn parse_light_color(block: &str) -> [f32; 3] {
    parse_srgb_color(block)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_empty_scene() {
        let code = "pub fn setup_scene() {}\n";
        let scene = import_scene_from_code(code);
        assert!(scene.entities.is_empty());
    }

    #[test]
    fn roundtrip_cube() {
        let mut original = SceneModel::new();
        original.add_entity(
            "TestCube".into(),
            vec![ComponentData::MeshCube {
                size: 2.0,
                color: [0.3, 0.6, 0.9],
                metallic: 0.5,
                roughness: 0.3,
                emissive: [0.0, 0.0, 0.0],
                texture_path: None,
                normal_map_path: None,
            }],
        );
        let code = super::super::codegen::generate_scene_code(&original);
        let imported = import_scene_from_code(&code);
        assert_eq!(imported.entities.len(), 1);
        let entity = imported.entities.values().next().unwrap();
        assert_eq!(entity.name, "TestCube");
        assert!(matches!(
            &entity.components[0],
            ComponentData::MeshCube { size, .. } if (*size - 2.0).abs() < 0.1
        ));
    }

    #[test]
    fn roundtrip_custom_script() {
        // CustomScript is emitted as BerryCode markers in generated code.
        // Roundtrip preserves the type name and field data.
        let mut original = SceneModel::new();
        original.add_entity(
            "Player".into(),
            vec![ComponentData::CustomScript {
                type_name: "PlayerStats".into(),
                fields: vec![
                    ScriptField {
                        name: "health".into(),
                        value: ScriptValue::Float(100.0),
                    },
                    ScriptField {
                        name: "alive".into(),
                        value: ScriptValue::Bool(true),
                    },
                ],
            }],
        );
        let code = super::super::codegen::generate_scene_code(&original);
        // Generated code should contain BerryCode markers
        assert!(code.contains("[BerryCode:CustomScript]"));
        assert!(code.contains("[BerryCode:CustomField]"));
        // Roundtrip import: entity and CustomScript component should survive
        let imported = import_scene_from_code(&code);
        assert_eq!(imported.entities.len(), 1);
        let entity = imported.entities.values().next().unwrap();
        assert!(
            entity
                .components
                .iter()
                .any(|c| matches!(c, ComponentData::CustomScript { .. })),
            "CustomScript component should survive roundtrip"
        );
    }

    #[test]
    fn roundtrip_directional_light() {
        let mut original = SceneModel::new();
        original.add_entity(
            "Sun".into(),
            vec![ComponentData::DirectionalLight {
                intensity: 15000.0,
                color: [1.0, 0.9, 0.8],
                shadows: true,
            }],
        );
        let code = super::super::codegen::generate_scene_code(&original);
        let imported = import_scene_from_code(&code);
        assert_eq!(imported.entities.len(), 1);
        let entity = imported.entities.values().next().unwrap();
        if let ComponentData::DirectionalLight {
            intensity, shadows, ..
        } = &entity.components[0]
        {
            assert!((*intensity - 15000.0).abs() < 1.0);
            assert!(*shadows);
        } else {
            panic!("Expected DirectionalLight");
        }
    }

    #[test]
    fn roundtrip_sphere() {
        let mut original = SceneModel::new();
        original.add_entity(
            "Ball".into(),
            vec![ComponentData::MeshSphere {
                radius: 1.5,
                color: [1.0, 0.0, 0.0],
                metallic: 0.8,
                roughness: 0.2,
                emissive: [0.0, 0.0, 0.0],
                texture_path: None,
                normal_map_path: None,
            }],
        );
        let code = super::super::codegen::generate_scene_code(&original);
        let imported = import_scene_from_code(&code);
        assert_eq!(imported.entities.len(), 1);
        let entity = imported.entities.values().next().unwrap();
        if let ComponentData::MeshSphere { radius, .. } = &entity.components[0] {
            assert!((*radius - 1.5).abs() < 0.1);
        } else {
            panic!("Expected MeshSphere");
        }
    }

    #[test]
    fn roundtrip_camera() {
        let mut original = SceneModel::new();
        original.add_entity("MainCamera".into(), vec![ComponentData::Camera]);
        let code = super::super::codegen::generate_scene_code(&original);
        let imported = import_scene_from_code(&code);
        assert_eq!(imported.entities.len(), 1);
        let entity = imported.entities.values().next().unwrap();
        assert!(matches!(&entity.components[0], ComponentData::Camera));
    }

    #[test]
    fn roundtrip_transform_with_rotation_and_scale() {
        let mut original = SceneModel::new();
        let id = original.add_entity("Rotated".into(), vec![ComponentData::Camera]);
        if let Some(e) = original.entities.get_mut(&id) {
            e.transform.translation = [1.0, 2.0, 3.0];
            e.transform.rotation_euler = [0.5, 1.0, 1.5];
            e.transform.scale = [2.0, 3.0, 4.0];
        }
        let code = super::super::codegen::generate_scene_code(&original);
        let imported = import_scene_from_code(&code);
        let entity = imported.entities.values().next().unwrap();
        assert!((entity.transform.translation[0] - 1.0).abs() < 0.01);
        assert!((entity.transform.rotation_euler[1] - 1.0).abs() < 0.01);
        assert!((entity.transform.scale[2] - 4.0).abs() < 0.01);
    }

    #[test]
    fn roundtrip_multiple_entities() {
        let mut original = SceneModel::new();
        original.add_entity(
            "Cube1".into(),
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
        original.add_entity(
            "Light1".into(),
            vec![ComponentData::Light {
                intensity: 5000.0,
                color: [1.0, 1.0, 1.0],
            }],
        );
        let code = super::super::codegen::generate_scene_code(&original);
        let imported = import_scene_from_code(&code);
        assert_eq!(imported.entities.len(), 2);
    }

    #[test]
    fn normalize_strips_line_comments() {
        let code = "// this is a comment\nlet x = 1;\n// another\nlet y = 2;\n";
        let norm = super::normalize_code(code);
        assert!(!norm.contains("comment"));
        assert!(norm.contains("let x = 1;"));
        assert!(norm.contains("let y = 2;"));
    }

    #[test]
    fn normalize_strips_block_comments() {
        let code = "before /* block\ncomment */ after\n";
        let norm = super::normalize_code(code);
        assert!(norm.contains("before"));
        assert!(!norm.contains("block"));
        assert!(norm.contains("after"));
    }

    #[test]
    fn import_with_comments_in_code() {
        let code = r#"
use bevy::prelude::*;

pub fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // A simple camera
    commands.spawn((
        Transform::from_xyz(0.0, 5.0, 10.0),
        Camera3d::default(),
        Name::new("MainCam"),
    ));
}
"#;
        let imported = import_scene_from_code(code);
        assert_eq!(imported.entities.len(), 1);
        let entity = imported.entities.values().next().unwrap();
        assert_eq!(entity.name, "MainCam");
    }

    #[test]
    fn parse_script_value_primitives() {
        assert!(matches!(
            super::parse_script_value("true"),
            ScriptValue::Bool(true)
        ));
        assert!(matches!(
            super::parse_script_value("false"),
            ScriptValue::Bool(false)
        ));
        assert!(matches!(
            super::parse_script_value("42"),
            ScriptValue::Int(42)
        ));
        assert!(
            matches!(super::parse_script_value("3.14"), ScriptValue::Float(f) if (f - 3.14).abs() < 0.01)
        );
        assert!(
            matches!(super::parse_script_value("\"hello\""), ScriptValue::String(s) if s == "hello")
        );
    }

    #[test]
    fn parse_script_value_option() {
        assert!(matches!(
            super::parse_script_value("None"),
            ScriptValue::Option(None)
        ));
        match super::parse_script_value("Some(42)") {
            ScriptValue::Option(Some(inner)) => {
                assert!(matches!(*inner, ScriptValue::Int(42)));
            }
            other => panic!("Expected Option(Some(42)), got {:?}", other),
        }
    }

    #[test]
    fn parse_script_value_vec() {
        match super::parse_script_value("vec![1.0, 2.0, 3.0]") {
            ScriptValue::Vec(items) => {
                assert_eq!(items.len(), 3);
            }
            other => panic!("Expected Vec, got {:?}", other),
        }
        match super::parse_script_value("vec![]") {
            ScriptValue::Vec(items) => assert!(items.is_empty()),
            other => panic!("Expected empty Vec, got {:?}", other),
        }
    }

    #[test]
    fn parse_script_value_hashmap_new() {
        match super::parse_script_value("HashMap::new()") {
            ScriptValue::Map(entries) => assert!(entries.is_empty()),
            other => panic!("Expected empty Map, got {:?}", other),
        }
    }

    #[test]
    fn import_walker_template() {
        // Use the actual Walker3D template code
        let code = crate::app::new_project::template_main_rs_for_test(
            crate::app::new_project::ProjectTemplate::Walker3D,
        );
        let scene = import_scene_from_code(&code);
        // Walker template spawns: DirectionalLight, Ground plane,
        // 15 boxes (in a for loop), 4 pillars, and a Camera (Player)
        // The for-loop boxes share one commands.spawn(( block in normalized form
        // so they may appear as one entity.
        assert!(
            scene.entities.len() >= 4,
            "Expected at least 4 entities, got {}",
            scene.entities.len()
        );

        // Verify we got a DirectionalLight
        let has_dir_light = scene.entities.values().any(|e| {
            e.components
                .iter()
                .any(|c| matches!(c, ComponentData::DirectionalLight { .. }))
        });
        assert!(has_dir_light, "Should have a DirectionalLight entity");

        // Verify we got a Camera
        let has_camera = scene.entities.values().any(|e| {
            e.components
                .iter()
                .any(|c| matches!(c, ComponentData::Camera))
        });
        assert!(has_camera, "Should have a Camera entity");

        // Verify we got a ground plane
        let has_plane = scene.entities.values().any(|e| {
            e.components
                .iter()
                .any(|c| matches!(c, ComponentData::MeshPlane { .. }))
        });
        assert!(has_plane, "Should have a Ground plane entity");

        // Verify we got at least one cube
        let has_cube = scene.entities.values().any(|e| {
            e.components
                .iter()
                .any(|c| matches!(c, ComponentData::MeshCube { .. }))
        });
        assert!(has_cube, "Should have at least one Cube entity");
    }

    #[test]
    fn import_empty3d_template() {
        let code = crate::app::new_project::template_main_rs_for_test(
            crate::app::new_project::ProjectTemplate::Empty3D,
        );
        let scene = import_scene_from_code(&code);
        // Empty3D: Camera, DirectionalLight, Cube (Cuboid::default()), Ground
        assert_eq!(
            scene.entities.len(),
            4,
            "Expected 4 entities, got {}",
            scene.entities.len()
        );

        let has_camera = scene.entities.values().any(|e| {
            e.components
                .iter()
                .any(|c| matches!(c, ComponentData::Camera))
        });
        assert!(has_camera);

        let has_cube = scene.entities.values().any(|e| {
            e.components
                .iter()
                .any(|c| matches!(c, ComponentData::MeshCube { .. }))
        });
        assert!(has_cube, "Cuboid::default() should be parsed as MeshCube");
    }

    #[test]
    fn roundtrip_all_component_types() {
        let mut scene = SceneModel::new();
        let defaults = ComponentData::default_all();

        // Add one entity per component type with a non-zero transform
        for (name, comp) in &defaults {
            let id = scene.add_entity(name.to_string(), vec![comp.clone()]);
            if let Some(e) = scene.entities.get_mut(&id) {
                e.transform.translation = [1.0, 2.0, 3.0];
            }
        }

        let original_count = scene.entities.len();
        assert_eq!(
            original_count,
            defaults.len(),
            "Should have {} entities",
            defaults.len()
        );

        // Generate code
        let code = super::super::codegen::generate_scene_code(&scene);

        // Import back
        let imported = import_scene_from_code(&code);

        // Every entity should survive the roundtrip (at minimum as a named entity)
        assert_eq!(
            imported.entities.len(),
            original_count,
            "Imported entity count ({}) != original ({})\n\nGenerated code:\n{}\n\nImported entities: {:?}",
            imported.entities.len(),
            original_count,
            &code[..code.len().min(3000)],
            imported.entities.values().map(|e| &e.name).collect::<Vec<_>>()
        );

        // Verify each entity name exists
        let imported_names: Vec<String> =
            imported.entities.values().map(|e| e.name.clone()).collect();
        for (name, _) in &defaults {
            assert!(
                imported_names.iter().any(|n| n == name),
                "Entity '{}' not found in imported scene. Found: {:?}",
                name,
                imported_names
            );
        }

        // Verify transforms survived
        for entity in imported.entities.values() {
            let t = &entity.transform.translation;
            assert!(
                (t[0] - 1.0).abs() < 0.01 && (t[1] - 2.0).abs() < 0.01 && (t[2] - 3.0).abs() < 0.01,
                "Entity '{}' transform mismatch: {:?}",
                entity.name,
                t
            );
        }

        // Verify ALL 26 component types are correctly preserved during roundtrip
        for entity in imported.entities.values() {
            assert!(
                !entity.components.is_empty(),
                "Entity '{}' lost all components during roundtrip",
                entity.name
            );
            match entity.name.as_str() {
                "Mesh Cube" => assert!(
                    entity
                        .components
                        .iter()
                        .any(|c| matches!(c, ComponentData::MeshCube { .. })),
                    "MeshCube missing"
                ),
                "Mesh Sphere" => assert!(
                    entity
                        .components
                        .iter()
                        .any(|c| matches!(c, ComponentData::MeshSphere { .. })),
                    "MeshSphere missing"
                ),
                "Mesh Plane" => assert!(
                    entity
                        .components
                        .iter()
                        .any(|c| matches!(c, ComponentData::MeshPlane { .. })),
                    "MeshPlane missing"
                ),
                "Light" => assert!(
                    entity
                        .components
                        .iter()
                        .any(|c| matches!(c, ComponentData::Light { .. })),
                    "Light missing"
                ),
                "Directional Light" => assert!(
                    entity
                        .components
                        .iter()
                        .any(|c| matches!(c, ComponentData::DirectionalLight { .. })),
                    "DirectionalLight missing"
                ),
                "Camera" => assert!(
                    entity
                        .components
                        .iter()
                        .any(|c| matches!(c, ComponentData::Camera)),
                    "Camera missing"
                ),
                "Spot Light" => assert!(
                    entity
                        .components
                        .iter()
                        .any(|c| matches!(c, ComponentData::SpotLight { .. })),
                    "SpotLight missing"
                ),
                "Mesh From File" => assert!(
                    entity
                        .components
                        .iter()
                        .any(|c| matches!(c, ComponentData::MeshFromFile { .. })),
                    "MeshFromFile missing"
                ),
                "Audio Source" => assert!(
                    entity
                        .components
                        .iter()
                        .any(|c| matches!(c, ComponentData::AudioSource { .. })),
                    "AudioSource missing"
                ),
                "Audio Listener" => assert!(
                    entity
                        .components
                        .iter()
                        .any(|c| matches!(c, ComponentData::AudioListener)),
                    "AudioListener missing"
                ),
                "Rigidbody" => assert!(
                    entity
                        .components
                        .iter()
                        .any(|c| matches!(c, ComponentData::RigidBody { .. })),
                    "RigidBody missing"
                ),
                "Collider" => assert!(
                    entity
                        .components
                        .iter()
                        .any(|c| matches!(c, ComponentData::Collider { .. })),
                    "Collider missing"
                ),
                "UI Text" => assert!(
                    entity
                        .components
                        .iter()
                        .any(|c| matches!(c, ComponentData::UiText { .. })),
                    "UiText missing"
                ),
                "UI Button" => assert!(
                    entity
                        .components
                        .iter()
                        .any(|c| matches!(c, ComponentData::UiButton { .. })),
                    "UiButton missing"
                ),
                "UI Image" => assert!(
                    entity
                        .components
                        .iter()
                        .any(|c| matches!(c, ComponentData::UiImage { .. })),
                    "UiImage missing"
                ),
                "Particle Emitter" => assert!(
                    entity
                        .components
                        .iter()
                        .any(|c| matches!(c, ComponentData::ParticleEmitter { .. })),
                    "ParticleEmitter missing"
                ),
                "Animation" => assert!(
                    entity
                        .components
                        .iter()
                        .any(|c| matches!(c, ComponentData::Animation { .. })),
                    "Animation missing"
                ),
                "Custom Script" => assert!(
                    entity
                        .components
                        .iter()
                        .any(|c| matches!(c, ComponentData::CustomScript { .. })),
                    "CustomScript missing"
                ),
                "Skybox" => assert!(
                    entity
                        .components
                        .iter()
                        .any(|c| matches!(c, ComponentData::Skybox { .. })),
                    "Skybox missing"
                ),
                "Animator" => assert!(
                    entity
                        .components
                        .iter()
                        .any(|c| matches!(c, ComponentData::Animator { .. })),
                    "Animator missing"
                ),
                "LOD Group" => assert!(
                    entity
                        .components
                        .iter()
                        .any(|c| matches!(c, ComponentData::LodGroup { .. })),
                    "LodGroup missing"
                ),
                "Spline" => assert!(
                    entity
                        .components
                        .iter()
                        .any(|c| matches!(c, ComponentData::Spline { .. })),
                    "Spline missing"
                ),
                "Terrain" => assert!(
                    entity
                        .components
                        .iter()
                        .any(|c| matches!(c, ComponentData::Terrain { .. })),
                    "Terrain missing"
                ),
                "Skinned Mesh" => assert!(
                    entity
                        .components
                        .iter()
                        .any(|c| matches!(c, ComponentData::SkinnedMesh { .. })),
                    "SkinnedMesh missing"
                ),
                "Visual Script" => assert!(
                    entity
                        .components
                        .iter()
                        .any(|c| matches!(c, ComponentData::VisualScript { .. })),
                    "VisualScript missing"
                ),
                "NavMesh" => assert!(
                    entity
                        .components
                        .iter()
                        .any(|c| matches!(c, ComponentData::NavMesh { .. })),
                    "NavMesh missing"
                ),
                other => panic!("Unexpected entity name: {}", other),
            }
        }
    }

    #[test]
    fn import_from_rotation_transform() {
        let code = r#"
fn setup(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            illuminance: 12000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -1.0, 0.5, 0.0)),
    ));
}
"#;
        let scene = import_scene_from_code(code);
        assert_eq!(scene.entities.len(), 1);
        let entity = scene.entities.values().next().unwrap();
        assert!(
            (entity.transform.rotation_euler[0] - (-1.0)).abs() < 0.01,
            "from_rotation should parse euler X"
        );
        assert!(
            (entity.transform.rotation_euler[1] - 0.5).abs() < 0.01,
            "from_rotation should parse euler Y"
        );
    }
}
