//! Skeletal animation: bone hierarchy data model.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoneData {
    pub name: String,
    pub parent_idx: Option<usize>,
    pub bind_pose: super::model::TransformData,
}

/// Parse bone hierarchy from a .glb/.gltf file using the gltf crate.
pub fn extract_bones_from_gltf(path: &str) -> Result<Vec<BoneData>, String> {
    let (document, _buffers, _images) = gltf::import(path).map_err(|e| format!("{}", e))?;
    let mut bones = Vec::new();

    // Build a global parent map: for every node that is a child of another
    // node, record child_index -> parent_index.
    let mut parent_map: std::collections::HashMap<usize, usize> =
        std::collections::HashMap::new();
    for node in document.nodes() {
        for child in node.children() {
            parent_map.insert(child.index(), node.index());
        }
    }

    for skin in document.skins() {
        let joints: Vec<_> = skin.joints().collect();
        // Build index map: gltf node index -> our bone index
        let mut node_to_bone: std::collections::HashMap<usize, usize> =
            std::collections::HashMap::new();

        for (bone_idx, joint) in joints.iter().enumerate() {
            node_to_bone.insert(joint.index(), bone_idx);
        }

        for (bone_idx, joint) in joints.iter().enumerate() {
            let (translation, rotation, scale) = joint.transform().decomposed();
            let parent_idx = parent_map
                .get(&joint.index())
                .and_then(|parent_node_idx| node_to_bone.get(parent_node_idx).copied());

            bones.push(BoneData {
                name: joint.name().unwrap_or("Bone").to_string(),
                parent_idx,
                bind_pose: super::model::TransformData {
                    translation,
                    rotation_euler: quat_to_euler(rotation),
                    scale,
                },
            });
            let _ = bone_idx; // suppress unused warning
        }
        break; // Use first skin only
    }
    Ok(bones)
}

fn quat_to_euler(q: [f32; 4]) -> [f32; 3] {
    let [x, y, z, w] = q;
    let sinr_cosp = 2.0 * (w * x + y * z);
    let cosr_cosp = 1.0 - 2.0 * (x * x + y * y);
    let roll = sinr_cosp.atan2(cosr_cosp);
    let sinp = 2.0 * (w * y - z * x);
    let pitch = if sinp.abs() >= 1.0 {
        std::f32::consts::FRAC_PI_2.copysign(sinp)
    } else {
        sinp.asin()
    };
    let siny_cosp = 2.0 * (w * z + x * y);
    let cosy_cosp = 1.0 - 2.0 * (y * y + z * z);
    let yaw = siny_cosp.atan2(cosy_cosp);
    [roll, pitch, yaw]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bone_data_roundtrip() {
        let bone = BoneData {
            name: "Root".into(),
            parent_idx: None,
            bind_pose: super::super::model::TransformData::default(),
        };
        let s = ron::ser::to_string(&bone).unwrap();
        let loaded: BoneData = ron::from_str(&s).unwrap();
        assert_eq!(loaded.name, "Root");
    }
    #[test]
    fn quat_identity_gives_zero_euler() {
        let e = quat_to_euler([0.0, 0.0, 0.0, 1.0]);
        assert!(e[0].abs() < 0.01 && e[1].abs() < 0.01 && e[2].abs() < 0.01);
    }
}
