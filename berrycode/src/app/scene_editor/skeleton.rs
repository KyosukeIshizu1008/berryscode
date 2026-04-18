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

/// Public wrapper for quaternion-to-euler conversion, callable from tests.
pub fn quat_to_euler_pub(q: [f32; 4]) -> [f32; 3] {
    quat_to_euler(q)
}

/// Create a test skeleton hierarchy for testing without a .glb file.
pub fn create_test_skeleton() -> Vec<BoneData> {
    vec![
        BoneData {
            name: "Root".into(),
            parent_idx: None,
            bind_pose: super::model::TransformData::default(),
        },
        BoneData {
            name: "Spine".into(),
            parent_idx: Some(0),
            bind_pose: super::model::TransformData {
                translation: [0.0, 1.0, 0.0],
                ..super::model::TransformData::default()
            },
        },
        BoneData {
            name: "Head".into(),
            parent_idx: Some(1),
            bind_pose: super::model::TransformData {
                translation: [0.0, 0.5, 0.0],
                ..super::model::TransformData::default()
            },
        },
        BoneData {
            name: "LeftArm".into(),
            parent_idx: Some(1),
            bind_pose: super::model::TransformData {
                translation: [-0.5, 0.0, 0.0],
                ..super::model::TransformData::default()
            },
        },
        BoneData {
            name: "RightArm".into(),
            parent_idx: Some(1),
            bind_pose: super::model::TransformData {
                translation: [0.5, 0.0, 0.0],
                ..super::model::TransformData::default()
            },
        },
    ]
}

/// Validate a skeleton hierarchy: checks that all parent indices are valid
/// and that there are no cycles. Returns a list of error descriptions.
pub fn validate_skeleton(bones: &[BoneData]) -> Vec<String> {
    let mut errors = Vec::new();
    for (i, bone) in bones.iter().enumerate() {
        if let Some(parent_idx) = bone.parent_idx {
            if parent_idx >= bones.len() {
                errors.push(format!(
                    "Bone '{}' (idx {}) has invalid parent_idx {}",
                    bone.name, i, parent_idx
                ));
            } else if parent_idx >= i {
                errors.push(format!(
                    "Bone '{}' (idx {}) references forward parent_idx {}",
                    bone.name, i, parent_idx
                ));
            }
        }
    }
    errors
}

/// Count root bones (bones with no parent) in a skeleton.
pub fn count_root_bones(bones: &[BoneData]) -> usize {
    bones.iter().filter(|b| b.parent_idx.is_none()).count()
}

/// Get the depth of a bone in the hierarchy (0 for root bones).
pub fn bone_depth(bones: &[BoneData], idx: usize) -> usize {
    let mut depth = 0;
    let mut current = bones[idx].parent_idx;
    while let Some(parent) = current {
        depth += 1;
        if parent < bones.len() {
            current = bones[parent].parent_idx;
        } else {
            break;
        }
    }
    depth
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
    #[test]
    fn create_test_skeleton_has_correct_structure() {
        let bones = create_test_skeleton();
        assert_eq!(bones.len(), 5);
        assert_eq!(bones[0].name, "Root");
        assert!(bones[0].parent_idx.is_none());
        assert_eq!(bones[1].parent_idx, Some(0));
        assert_eq!(bones[2].parent_idx, Some(1));
    }
    #[test]
    fn validate_valid_skeleton() {
        let bones = create_test_skeleton();
        let errors = validate_skeleton(&bones);
        assert!(errors.is_empty());
    }
    #[test]
    fn validate_invalid_parent_idx() {
        let bones = vec![
            BoneData { name: "Root".into(), parent_idx: None, bind_pose: super::super::model::TransformData::default() },
            BoneData { name: "Bad".into(), parent_idx: Some(99), bind_pose: super::super::model::TransformData::default() },
        ];
        let errors = validate_skeleton(&bones);
        assert_eq!(errors.len(), 1);
    }
    #[test]
    fn count_root_bones_works() {
        let bones = create_test_skeleton();
        assert_eq!(count_root_bones(&bones), 1);
    }
    #[test]
    fn bone_depth_works() {
        let bones = create_test_skeleton();
        assert_eq!(bone_depth(&bones, 0), 0); // Root
        assert_eq!(bone_depth(&bones, 1), 1); // Spine
        assert_eq!(bone_depth(&bones, 2), 2); // Head
    }
    #[test]
    fn quat_to_euler_pub_matches_private() {
        let q = [0.1, 0.2, 0.3, 0.9];
        let pub_result = quat_to_euler_pub(q);
        let priv_result = quat_to_euler(q);
        assert_eq!(pub_result, priv_result);
    }
}
