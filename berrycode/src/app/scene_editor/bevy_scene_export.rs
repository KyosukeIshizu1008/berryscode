//! Export SceneModel to Bevy's native .scn.ron format.

use super::model::*;

/// Generate Bevy-native .scn.ron content from a SceneModel.
pub fn export_to_bevy_scene(scene: &SceneModel) -> String {
    let mut ron = String::new();
    ron.push_str("(\n  resources: {},\n  entities: {\n");

    for (idx, entity) in scene.entities.values().enumerate() {
        if !entity.enabled {
            continue;
        }
        ron.push_str(&format!("    {}: (\n      components: {{\n", idx));

        // Transform component
        let t = &entity.transform;
        ron.push_str(&format!(
            "        \"bevy_transform::components::transform::Transform\": (\n          translation: ({}, {}, {}),\n          rotation: (0.0, 0.0, 0.0, 1.0),\n          scale: ({}, {}, {}),\n        ),\n",
            t.translation[0], t.translation[1], t.translation[2],
            t.scale[0], t.scale[1], t.scale[2],
        ));

        // Name component
        ron.push_str(&format!(
            "        \"bevy_core::name::Name\": (\n          hash: 0,\n          name: \"{}\",\n        ),\n",
            entity.name
        ));

        // Map ComponentData to Bevy type paths
        for component in &entity.components {
            match component {
                ComponentData::MeshCube { .. }
                | ComponentData::MeshSphere { .. }
                | ComponentData::MeshPlane { .. } => {
                    // Mesh components need Mesh3d handle -- can't represent directly in .scn.ron
                    ron.push_str(&format!(
                        "        // {}: requires mesh asset handle\n",
                        component.label()
                    ));
                }
                ComponentData::Light { intensity, color } => {
                    ron.push_str(&format!(
                        "        \"bevy_pbr::light::point_light::PointLight\": (\n          color: Srgba(red: {}, green: {}, blue: {}, alpha: 1.0),\n          intensity: {},\n          range: 50.0,\n        ),\n",
                        color[0], color[1], color[2], intensity
                    ));
                }
                ComponentData::DirectionalLight {
                    intensity,
                    color,
                    shadows,
                } => {
                    ron.push_str(&format!(
                        "        \"bevy_pbr::light::directional_light::DirectionalLight\": (\n          color: Srgba(red: {}, green: {}, blue: {}, alpha: 1.0),\n          illuminance: {},\n          shadows_enabled: {},\n        ),\n",
                        color[0], color[1], color[2], intensity, shadows
                    ));
                }
                ComponentData::Camera => {
                    ron.push_str(
                        "        \"bevy_render::camera::camera::Camera\": (),\n",
                    );
                }
                _ => {
                    ron.push_str(&format!(
                        "        // {}: custom component\n",
                        component.label()
                    ));
                }
            }
        }

        ron.push_str("      },\n    ),\n");
    }

    ron.push_str("  },\n)\n");
    ron
}

/// Save as .scn.ron file
pub fn save_bevy_scene(scene: &SceneModel, path: &str) -> Result<String, String> {
    let content = export_to_bevy_scene(scene);
    let scn_path = if path.ends_with(".bscene") {
        path.replace(".bscene", ".scn.ron")
    } else {
        format!("{}.scn.ron", path)
    };
    std::fs::write(&scn_path, &content).map_err(|e| e.to_string())?;
    Ok(scn_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_empty_scene() {
        let scene = SceneModel::new();
        let ron = export_to_bevy_scene(&scene);
        assert!(ron.contains("entities:"));
    }

    #[test]
    fn export_with_light() {
        let mut scene = SceneModel::new();
        scene.add_entity(
            "Sun".into(),
            vec![ComponentData::DirectionalLight {
                intensity: 10000.0,
                color: [1.0, 1.0, 0.9],
                shadows: true,
            }],
        );
        let ron = export_to_bevy_scene(&scene);
        assert!(ron.contains("DirectionalLight"));
        assert!(ron.contains("shadows_enabled: true"));
    }

    #[test]
    fn export_with_transform() {
        let mut scene = SceneModel::new();
        let id = scene.add_entity("Obj".into(), vec![]);
        if let Some(e) = scene.entities.get_mut(&id) {
            e.transform.translation = [1.0, 2.0, 3.0];
        }
        let ron = export_to_bevy_scene(&scene);
        assert!(ron.contains("translation: (1, 2, 3)"));
    }
}
