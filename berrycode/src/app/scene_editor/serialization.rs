//! Scene serialization helpers.
//!
//! Scenes are stored as RON (Rusty Object Notation). The on-disk payload is
//! the editor-side `SceneModel` minus its transient UI state (selection,
//! dirty flag, file path) — those are reconstructed on load.

use super::model::*;

/// Save a scene to a RON file. Pretty-printed for human inspection / diffing.
pub fn save_scene_to_ron(scene: &SceneModel, path: &str) -> anyhow::Result<()> {
    let ron_str = ron::ser::to_string_pretty(scene, ron::ser::PrettyConfig::default())?;
    std::fs::write(path, ron_str)?;
    Ok(())
}

/// Load a scene from a RON file. Re-derives `root_entities` and the per-entity
/// `children` lists from the persisted `parent` pointers, and re-attaches the
/// editor-only `file_path`.
pub fn load_scene_from_ron(path: &str) -> anyhow::Result<SceneModel> {
    let ron_str = std::fs::read_to_string(path)?;
    let mut scene: SceneModel = ron::from_str(&ron_str)?;
    scene.file_path = Some(path.to_string());
    scene.modified = false;

    // Rebuild children lists.
    for entity in scene.entities.values_mut() {
        entity.children.clear();
    }
    let edges: Vec<(u64, u64)> = scene
        .entities
        .iter()
        .filter_map(|(id, e)| e.parent.map(|p| (p, *id)))
        .collect();
    for (parent_id, child_id) in edges {
        if let Some(parent) = scene.entities.get_mut(&parent_id) {
            parent.children.push(child_id);
        }
    }

    // Rebuild root_entities (entities without a parent).
    scene.root_entities = scene
        .entities
        .iter()
        .filter(|(_, e)| e.parent.is_none())
        .map(|(id, _)| *id)
        .collect();

    Ok(scene)
}
