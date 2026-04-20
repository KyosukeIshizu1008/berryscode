//! Simplified Prefab system: serialize an entity subtree as a self-contained
//! `.bprefab` file and instantiate it back into a `SceneModel`.
//!
//! This is intentionally a "deep copy on instantiate" model — there is NO
//! override tracking. Once instantiated, the prefab instance is independent of
//! the source file.

use super::model::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// On-disk prefab payload: a self-contained subtree of entities.
///
/// IDs in the payload are local to the prefab (root has the lowest id), so we
/// can re-key them when instantiating to avoid collisions with the target
/// `SceneModel`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefabFile {
    pub root_id: u64,
    pub entities: HashMap<u64, SceneEntity>,
}

/// Build a `PrefabFile` from `root_id` and all of its descendants in `scene`.
/// Returns `None` if `root_id` is not present in the scene.
pub fn build_prefab_from_entity(scene: &SceneModel, root_id: u64) -> Option<PrefabFile> {
    if !scene.entities.contains_key(&root_id) {
        return None;
    }

    // Collect the subtree rooted at root_id.
    let mut subtree: Vec<u64> = Vec::new();
    let mut stack = vec![root_id];
    while let Some(current) = stack.pop() {
        subtree.push(current);
        if let Some(e) = scene.entities.get(&current) {
            for &c in &e.children {
                stack.push(c);
            }
        }
    }

    // Re-key from 1..=N so the prefab is portable and the root is always 1.
    let mut id_map: HashMap<u64, u64> = HashMap::new();
    id_map.insert(root_id, 1);
    let mut next_id: u64 = 2;
    for &id in &subtree {
        if id == root_id {
            continue;
        }
        id_map.insert(id, next_id);
        next_id += 1;
    }

    let mut entities = HashMap::new();
    for &old_id in &subtree {
        let original = match scene.entities.get(&old_id) {
            Some(e) => e,
            None => continue,
        };
        let new_id = id_map[&old_id];
        let new_parent = if old_id == root_id {
            None
        } else {
            original.parent.and_then(|pid| id_map.get(&pid).copied())
        };
        let new_children: Vec<u64> = original
            .children
            .iter()
            .filter_map(|cid| id_map.get(cid).copied())
            .collect();
        let new_entity = SceneEntity {
            id: new_id,
            name: original.name.clone(),
            transform: original.transform.clone(),
            components: original.components.clone(),
            children: new_children,
            parent: new_parent,
            enabled: original.enabled,
            // Preserve prefab_source on child entities so nested prefab
            // references survive when saving a parent prefab.
            prefab_source: original.prefab_source.clone(),
        };
        entities.insert(new_id, new_entity);
    }

    Some(PrefabFile {
        root_id: 1,
        entities,
    })
}

/// Save a `PrefabFile` to disk as RON.
pub fn save_prefab(prefab: &PrefabFile, path: &str) -> anyhow::Result<()> {
    let s = ron::ser::to_string_pretty(prefab, ron::ser::PrettyConfig::default())?;
    std::fs::write(path, s)?;
    Ok(())
}

/// Load a `PrefabFile` from disk.
pub fn load_prefab(path: &str) -> anyhow::Result<PrefabFile> {
    let s = std::fs::read_to_string(path)?;
    let p: PrefabFile = ron::from_str(&s)?;
    Ok(p)
}

/// Instantiate a prefab into the given scene. Returns the new root entity ID.
/// All prefab IDs are re-keyed using `scene.next_id` so they don't collide.
/// The new root is added to `scene.root_entities`.
///
/// The `prefab_source` field is preserved on child entities (supporting nested
/// prefab references). The root entity does NOT automatically get a
/// `prefab_source` — callers should set it if they want the inspector to
/// display prefab linkage (see [`instantiate_prefab_from_path`]).
pub fn instantiate_prefab(scene: &mut SceneModel, prefab: &PrefabFile) -> u64 {
    instantiate_prefab_inner(scene, prefab, None)
}

/// Instantiate a prefab and tag the new root entity with `prefab_source`
/// pointing to `path`. Child entities retain any `prefab_source` values that
/// were stored inside the prefab (nested prefabs).
pub fn instantiate_prefab_from_path(
    scene: &mut SceneModel,
    prefab: &PrefabFile,
    path: &str,
) -> u64 {
    instantiate_prefab_inner(scene, prefab, Some(path))
}

fn instantiate_prefab_inner(
    scene: &mut SceneModel,
    prefab: &PrefabFile,
    source_path: Option<&str>,
) -> u64 {
    // Map old prefab ids -> new scene ids.
    let mut id_map: HashMap<u64, u64> = HashMap::new();
    for &old_id in prefab.entities.keys() {
        let new_id = scene.next_id;
        scene.next_id += 1;
        id_map.insert(old_id, new_id);
    }

    // Insert each entity with re-keyed ids/parent/children.
    for (&old_id, original) in &prefab.entities {
        let new_id = id_map[&old_id];
        let new_parent = original.parent.and_then(|pid| id_map.get(&pid).copied());
        let new_children: Vec<u64> = original
            .children
            .iter()
            .filter_map(|cid| id_map.get(cid).copied())
            .collect();
        // For the root entity, use the supplied source_path. For children,
        // preserve whatever prefab_source was stored in the prefab file
        // (this is how nested prefab references survive instantiation).
        let prefab_source = if old_id == prefab.root_id {
            source_path.map(|s| s.to_string())
        } else {
            original.prefab_source.clone()
        };
        let new_entity = SceneEntity {
            id: new_id,
            name: original.name.clone(),
            transform: original.transform.clone(),
            components: original.components.clone(),
            children: new_children,
            parent: new_parent,
            enabled: original.enabled,
            prefab_source,
        };
        scene.entities.insert(new_id, new_entity);
    }

    let new_root = id_map[&prefab.root_id];
    scene.root_entities.push(new_root);
    scene.modified = true;
    new_root
}

/// Remove the prefab link from an entity, making it a regular entity.
/// This "unpacks" the prefab — the entity's components and children remain
/// but it is no longer associated with a `.bprefab` file.
pub fn unpack_prefab(scene: &mut SceneModel, entity_id: u64) {
    if let Some(entity) = scene.entities.get_mut(&entity_id) {
        entity.prefab_source = None;
        scene.modified = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube_component() -> ComponentData {
        ComponentData::MeshCube {
            size: 1.0,
            color: [0.5, 0.5, 0.5],
            metallic: 0.0,
            roughness: 0.5,
            emissive: [0.0, 0.0, 0.0],
            texture_path: None,
            normal_map_path: None,
        }
    }

    #[test]
    fn build_and_instantiate_roundtrip() {
        let mut scene = SceneModel::new();
        let parent = scene.add_entity("P".into(), vec![cube_component()]);
        let child = scene.add_entity("C".into(), vec![cube_component()]);
        scene.set_parent(child, Some(parent));

        let prefab = build_prefab_from_entity(&scene, parent).expect("prefab");
        assert_eq!(prefab.entities.len(), 2);
        assert_eq!(prefab.root_id, 1);

        let mut target = SceneModel::new();
        let new_root = instantiate_prefab(&mut target, &prefab);
        assert_eq!(target.entities.len(), 2);
        assert!(target.root_entities.contains(&new_root));
        let root_entity = target.entities.get(&new_root).unwrap();
        assert_eq!(root_entity.children.len(), 1);
        assert_eq!(root_entity.parent, None);
    }

    #[test]
    fn instantiate_twice_does_not_collide() {
        let mut scene = SceneModel::new();
        let id = scene.add_entity("E".into(), vec![cube_component()]);
        let prefab = build_prefab_from_entity(&scene, id).unwrap();

        let mut target = SceneModel::new();
        let r1 = instantiate_prefab(&mut target, &prefab);
        let r2 = instantiate_prefab(&mut target, &prefab);
        assert_ne!(r1, r2);
        assert_eq!(target.entities.len(), 2);
    }

    #[test]
    fn build_returns_none_for_missing_id() {
        let scene = SceneModel::new();
        assert!(build_prefab_from_entity(&scene, 999).is_none());
    }

    #[test]
    fn nested_prefab_source_preserved_on_save_and_instantiate() {
        // Build a scene where entity A has a child B with prefab_source set.
        let mut scene = SceneModel::new();
        let a = scene.add_entity("A".into(), vec![cube_component()]);
        let b = scene.add_entity("B".into(), vec![cube_component()]);
        scene.set_parent(b, Some(a));
        // Simulate B being a prefab instance from "x.bprefab".
        scene.entities.get_mut(&b).unwrap().prefab_source = Some("x.bprefab".into());

        // Save A (including its child B) as a prefab.
        let prefab = build_prefab_from_entity(&scene, a).expect("prefab");
        assert_eq!(prefab.entities.len(), 2);

        // The child in the prefab file should retain its prefab_source.
        let child_in_prefab = prefab
            .entities
            .values()
            .find(|e| e.name == "B")
            .expect("child B in prefab");
        assert_eq!(child_in_prefab.prefab_source.as_deref(), Some("x.bprefab"));

        // Instantiate the prefab into a new scene.
        let mut target = SceneModel::new();
        let new_root = instantiate_prefab(&mut target, &prefab);
        // Root should have no prefab_source (not called with _from_path).
        assert!(target
            .entities
            .get(&new_root)
            .unwrap()
            .prefab_source
            .is_none());

        // The child entity should still carry the nested prefab_source.
        let child_id = target.entities.get(&new_root).unwrap().children[0];
        assert_eq!(
            target
                .entities
                .get(&child_id)
                .unwrap()
                .prefab_source
                .as_deref(),
            Some("x.bprefab")
        );
    }

    #[test]
    fn instantiate_from_path_sets_root_prefab_source() {
        let mut scene = SceneModel::new();
        let id = scene.add_entity("E".into(), vec![cube_component()]);
        let prefab = build_prefab_from_entity(&scene, id).unwrap();

        let mut target = SceneModel::new();
        let new_root = instantiate_prefab_from_path(&mut target, &prefab, "my.bprefab");
        let root_entity = target.entities.get(&new_root).unwrap();
        assert_eq!(root_entity.prefab_source.as_deref(), Some("my.bprefab"));
    }

    #[test]
    fn unpack_prefab_clears_source() {
        let mut scene = SceneModel::new();
        let id = scene.add_entity("E".into(), vec![cube_component()]);
        scene.entities.get_mut(&id).unwrap().prefab_source = Some("test.bprefab".into());

        unpack_prefab(&mut scene, id);
        assert!(scene.entities.get(&id).unwrap().prefab_source.is_none());
        assert!(scene.modified);
    }
}
