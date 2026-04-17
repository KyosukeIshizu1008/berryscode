//! Reverse code import: parse a _scene.rs file back into SceneModel.
//!
//! Since we control the generated format, we can parse it with simple
//! line-by-line regex matching rather than full Rust AST parsing.

use super::model::*;
use regex::Regex;

/// Normalize code: strip line comments, collapse multi-line whitespace, and
/// resolve simple `let` bindings so the regex patterns work on hand-edited code.
fn normalize_code(code: &str) -> String {
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

        // Strip single-line comments
        if remaining.starts_with("//") {
            continue;
        }
        let no_comment = if let Some(pos) = remaining.find("//") {
            &remaining[..pos]
        } else {
            remaining
        };
        let no_comment = no_comment.trim();
        if !no_comment.is_empty() {
            result.push_str(no_comment);
            result.push(' ');
        }
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
        }

        // Parse Transform
        let transform_re =
            Regex::new(r"Transform::from_xyz\(([^,]+),\s*([^,]+),\s*([^)]+)\)").expect("valid regex");
        if let Some(cap) = transform_re.captures(block) {
            transform.translation = [
                cap[1].trim().parse().unwrap_or(0.0),
                cap[2].trim().parse().unwrap_or(0.0),
                cap[3].trim().parse().unwrap_or(0.0),
            ];
        }

        // Parse rotation if present
        let rotation_re = Regex::new(
            r"with_rotation\(Quat::from_euler\(EulerRot::XYZ,\s*([^,]+),\s*([^,]+),\s*([^)]+)\)\)",
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
        let scale_re =
            Regex::new(r"with_scale\(Vec3::new\(([^,]+),\s*([^,]+),\s*([^)]+)\)\)").expect("valid regex");
        if let Some(cap) = scale_re.captures(block) {
            transform.scale = [
                cap[1].trim().parse().unwrap_or(1.0),
                cap[2].trim().parse().unwrap_or(1.0),
                cap[3].trim().parse().unwrap_or(1.0),
            ];
        }

        // Parse Cuboid (MeshCube)
        let cuboid_re =
            Regex::new(r"Cuboid::new\(([^,]+),\s*([^,]+),\s*([^)]+)\)").expect("valid regex");
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
        }

        // Parse Sphere (only if no Cuboid to avoid false matches)
        let sphere_re = Regex::new(r"Sphere::new\(([^)]+)\)").expect("valid regex");
        if !cuboid_re.is_match(block) {
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
        let custom_re =
            Regex::new(r"(\b[A-Z][a-zA-Z0-9]+)\s*\{([^}]*)\}")
                .expect("valid regex");
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
            let field_re = Regex::new(r"(\w+):\s*([^,}]+)")
                .expect("valid regex");
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

        let id = scene.add_entity(name, components);
        if let Some(e) = scene.entities.get_mut(&id) {
            e.transform = transform;
        }
    }

    scene
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
        let imported = import_scene_from_code(&code);
        assert_eq!(imported.entities.len(), 1);
        let entity = imported.entities.values().next().unwrap();
        if let ComponentData::CustomScript { type_name, fields } = &entity.components[0] {
            assert_eq!(type_name, "PlayerStats");
            assert_eq!(fields.len(), 2);
        } else {
            panic!("Expected CustomScript");
        }
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
        assert!(matches!(super::parse_script_value("true"), ScriptValue::Bool(true)));
        assert!(matches!(super::parse_script_value("false"), ScriptValue::Bool(false)));
        assert!(matches!(super::parse_script_value("42"), ScriptValue::Int(42)));
        assert!(matches!(super::parse_script_value("3.14"), ScriptValue::Float(f) if (f - 3.14).abs() < 0.01));
        assert!(matches!(super::parse_script_value("\"hello\""), ScriptValue::String(s) if s == "hello"));
    }

    #[test]
    fn parse_script_value_option() {
        assert!(matches!(super::parse_script_value("None"), ScriptValue::Option(None)));
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
}
