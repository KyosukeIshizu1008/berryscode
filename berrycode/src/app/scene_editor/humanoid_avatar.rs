#![allow(dead_code)]
//! Humanoid avatar data model and auto-detection for humanoid rigs.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::app::BerryCodeApp;

// ---------------------------------------------------------------------------
// HumanoidBone enum
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HumanoidBone {
    Hips,
    Spine,
    Chest,
    Neck,
    Head,
    LeftShoulder,
    LeftUpperArm,
    LeftLowerArm,
    LeftHand,
    RightShoulder,
    RightUpperArm,
    RightLowerArm,
    RightHand,
    LeftUpperLeg,
    LeftLowerLeg,
    LeftFoot,
    LeftToes,
    RightUpperLeg,
    RightLowerLeg,
    RightFoot,
    RightToes,
    Spine1,
}

impl HumanoidBone {
    /// All variants in display order.
    pub const ALL: [HumanoidBone; 22] = [
        Self::Hips,
        Self::Spine,
        Self::Spine1,
        Self::Chest,
        Self::Neck,
        Self::Head,
        Self::LeftShoulder,
        Self::LeftUpperArm,
        Self::LeftLowerArm,
        Self::LeftHand,
        Self::RightShoulder,
        Self::RightUpperArm,
        Self::RightLowerArm,
        Self::RightHand,
        Self::LeftUpperLeg,
        Self::LeftLowerLeg,
        Self::LeftFoot,
        Self::LeftToes,
        Self::RightUpperLeg,
        Self::RightLowerLeg,
        Self::RightFoot,
        Self::RightToes,
    ];
}

// ---------------------------------------------------------------------------
// HumanoidAvatar
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanoidAvatar {
    pub name: String,
    pub bone_mapping: HashMap<HumanoidBone, String>,
}

impl Default for HumanoidAvatar {
    fn default() -> Self {
        Self {
            name: "New Avatar".into(),
            bone_mapping: HashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Auto-detection
// ---------------------------------------------------------------------------

/// Auto-detect humanoid bone mapping from skeleton bone names.
/// Supports common naming conventions including Mixamo-style names
/// (e.g. "b_Hip_01", "b_Head_05", "mixamorig:Hips").
pub fn auto_detect_humanoid(bones: &[super::skeleton::BoneData]) -> HumanoidAvatar {
    let mut avatar = HumanoidAvatar::default();
    avatar.name = "Auto-detected".into();

    for bone in bones {
        let lower = bone.name.to_lowercase();
        // Strip common prefixes: "mixamorig:", "b_", "Bip01 "
        let stripped = lower
            .trim_start_matches("mixamorig:")
            .trim_start_matches("b_")
            .trim_start_matches("bip01 ")
            .trim_start_matches("bip01_");
        // Strip trailing number suffixes like "_01", "_05"
        let clean = stripped
            .trim_end_matches(|c: char| c.is_ascii_digit())
            .trim_end_matches('_');

        if let Some(hb) = match_bone_name(clean) {
            // Only map if not already mapped (first match wins)
            avatar
                .bone_mapping
                .entry(hb)
                .or_insert_with(|| bone.name.clone());
        }
    }

    avatar
}

/// Match a cleaned bone name to a HumanoidBone variant.
fn match_bone_name(name: &str) -> Option<HumanoidBone> {
    match name {
        // Hips
        "hips" | "hip" | "pelvis" => Some(HumanoidBone::Hips),
        // Spine
        "spine" => Some(HumanoidBone::Spine),
        "spine1" | "spine_1" => Some(HumanoidBone::Spine1),
        // Chest
        "chest" | "spine2" | "spine_2" | "upperchest" | "upper_chest" => Some(HumanoidBone::Chest),
        // Neck / Head
        "neck" => Some(HumanoidBone::Neck),
        "head" => Some(HumanoidBone::Head),
        // Left arm chain
        "leftshoulder" | "left_shoulder" | "l_shoulder" | "shoulder_l" | "shoulder.l" => {
            Some(HumanoidBone::LeftShoulder)
        }
        "leftupperarm" | "leftarm" | "left_upperarm" | "left_arm" | "l_upperarm" | "arm_l"
        | "upperarm_l" | "upperarm.l" => Some(HumanoidBone::LeftUpperArm),
        "leftlowerarm" | "leftforearm" | "left_lowerarm" | "left_forearm" | "l_forearm"
        | "forearm_l" | "lowerarm_l" | "lowerarm.l" => Some(HumanoidBone::LeftLowerArm),
        "lefthand" | "left_hand" | "l_hand" | "hand_l" | "hand.l" => Some(HumanoidBone::LeftHand),
        // Right arm chain
        "rightshoulder" | "right_shoulder" | "r_shoulder" | "shoulder_r" | "shoulder.r" => {
            Some(HumanoidBone::RightShoulder)
        }
        "rightupperarm" | "rightarm" | "right_upperarm" | "right_arm" | "r_upperarm" | "arm_r"
        | "upperarm_r" | "upperarm.r" => Some(HumanoidBone::RightUpperArm),
        "rightlowerarm" | "rightforearm" | "right_lowerarm" | "right_forearm" | "r_forearm"
        | "forearm_r" | "lowerarm_r" | "lowerarm.r" => Some(HumanoidBone::RightLowerArm),
        "righthand" | "right_hand" | "r_hand" | "hand_r" | "hand.r" => {
            Some(HumanoidBone::RightHand)
        }
        // Left leg chain
        "leftupperleg" | "leftthigh" | "left_upperleg" | "left_thigh" | "l_thigh" | "thigh_l"
        | "upperleg_l" | "upperleg.l" => Some(HumanoidBone::LeftUpperLeg),
        "leftlowerleg" | "leftshin" | "leftleg" | "left_lowerleg" | "left_shin" | "left_leg"
        | "l_shin" | "shin_l" | "lowerleg_l" | "lowerleg.l" => Some(HumanoidBone::LeftLowerLeg),
        "leftfoot" | "left_foot" | "l_foot" | "foot_l" | "foot.l" => Some(HumanoidBone::LeftFoot),
        "lefttoes" | "lefttoebase" | "left_toes" | "left_toebase" | "l_toe" | "toe_l"
        | "toes.l" => Some(HumanoidBone::LeftToes),
        // Right leg chain
        "rightupperleg" | "rightthigh" | "right_upperleg" | "right_thigh" | "r_thigh"
        | "thigh_r" | "upperleg_r" | "upperleg.r" => Some(HumanoidBone::RightUpperLeg),
        "rightlowerleg" | "rightshin" | "rightleg" | "right_lowerleg" | "right_shin"
        | "right_leg" | "r_shin" | "shin_r" | "lowerleg_r" | "lowerleg.r" => {
            Some(HumanoidBone::RightLowerLeg)
        }
        "rightfoot" | "right_foot" | "r_foot" | "foot_r" | "foot.r" => {
            Some(HumanoidBone::RightFoot)
        }
        "righttoes" | "righttoebase" | "right_toes" | "right_toebase" | "r_toe" | "toe_r"
        | "toes.r" => Some(HumanoidBone::RightToes),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// File I/O (.bavatar files in RON format)
// ---------------------------------------------------------------------------

/// Save a HumanoidAvatar to a .bavatar file (RON format).
pub fn save_avatar(avatar: &HumanoidAvatar, path: &str) -> anyhow::Result<()> {
    let s = ron::ser::to_string_pretty(avatar, ron::ser::PrettyConfig::default())?;
    std::fs::write(path, s)?;
    Ok(())
}

/// Load a HumanoidAvatar from a .bavatar file (RON format).
pub fn load_avatar(path: &str) -> anyhow::Result<HumanoidAvatar> {
    let s = std::fs::read_to_string(path)?;
    let avatar: HumanoidAvatar = ron::from_str(&s)?;
    Ok(avatar)
}

// ---------------------------------------------------------------------------
// Avatar Editor UI
// ---------------------------------------------------------------------------

impl BerryCodeApp {
    /// Render the avatar editor: a simple table of HumanoidBone -> mapped bone name.
    pub(crate) fn render_avatar_editor(&mut self, ctx: &egui::Context) {
        if !self.avatar_editor_open {
            return;
        }

        let avatar = match &mut self.editing_avatar {
            Some(a) => a,
            None => {
                self.avatar_editor_open = false;
                return;
            }
        };

        let mut open = self.avatar_editor_open;

        egui::Window::new("Humanoid Avatar")
            .open(&mut open)
            .default_size([400.0, 500.0])
            .resizable(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Avatar Name:");
                    ui.text_edit_singleline(&mut avatar.name);
                });

                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::Grid::new("avatar_bone_grid")
                        .num_columns(2)
                        .striped(true)
                        .spacing([8.0, 4.0])
                        .show(ui, |ui| {
                            ui.strong("Humanoid Bone");
                            ui.strong("Mapped Name");
                            ui.end_row();

                            for hb in HumanoidBone::ALL {
                                ui.label(format!("{:?}", hb));
                                let entry =
                                    avatar.bone_mapping.entry(hb).or_insert_with(String::new);
                                ui.text_edit_singleline(entry);
                                ui.end_row();
                            }
                        });
                });
            });

        self.avatar_editor_open = open;
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::scene_editor::model::TransformData;
    use crate::app::scene_editor::skeleton::BoneData;

    fn make_bone(name: &str) -> BoneData {
        BoneData {
            name: name.into(),
            parent_idx: None,
            bind_pose: TransformData::default(),
        }
    }

    #[test]
    fn auto_detect_basic_names() {
        let bones = vec![
            make_bone("Hips"),
            make_bone("Spine"),
            make_bone("Neck"),
            make_bone("Head"),
            make_bone("LeftArm"),
            make_bone("RightArm"),
            make_bone("LeftHand"),
            make_bone("RightHand"),
            make_bone("LeftThigh"),
            make_bone("RightThigh"),
            make_bone("LeftFoot"),
            make_bone("RightFoot"),
        ];
        let avatar = auto_detect_humanoid(&bones);
        assert_eq!(
            avatar.bone_mapping.get(&HumanoidBone::Hips).unwrap(),
            "Hips"
        );
        assert_eq!(
            avatar.bone_mapping.get(&HumanoidBone::Head).unwrap(),
            "Head"
        );
        assert_eq!(
            avatar
                .bone_mapping
                .get(&HumanoidBone::LeftUpperArm)
                .unwrap(),
            "LeftArm"
        );
        assert_eq!(
            avatar.bone_mapping.get(&HumanoidBone::LeftFoot).unwrap(),
            "LeftFoot"
        );
    }

    #[test]
    fn auto_detect_mixamo_style() {
        let bones = vec![
            make_bone("b_Hip_01"),
            make_bone("b_Spine_02"),
            make_bone("b_Head_05"),
            make_bone("b_LeftHand_03"),
        ];
        let avatar = auto_detect_humanoid(&bones);
        assert_eq!(
            avatar.bone_mapping.get(&HumanoidBone::Hips).unwrap(),
            "b_Hip_01"
        );
        assert_eq!(
            avatar.bone_mapping.get(&HumanoidBone::Head).unwrap(),
            "b_Head_05"
        );
        assert_eq!(
            avatar.bone_mapping.get(&HumanoidBone::LeftHand).unwrap(),
            "b_LeftHand_03"
        );
    }

    #[test]
    fn auto_detect_mixamorig_prefix() {
        let bones = vec![
            make_bone("mixamorig:Hips"),
            make_bone("mixamorig:Spine"),
            make_bone("mixamorig:Head"),
        ];
        let avatar = auto_detect_humanoid(&bones);
        assert_eq!(
            avatar.bone_mapping.get(&HumanoidBone::Hips).unwrap(),
            "mixamorig:Hips"
        );
        assert_eq!(
            avatar.bone_mapping.get(&HumanoidBone::Head).unwrap(),
            "mixamorig:Head"
        );
    }

    #[test]
    fn ron_roundtrip() {
        let mut avatar = HumanoidAvatar::default();
        avatar.name = "TestAvatar".into();
        avatar
            .bone_mapping
            .insert(HumanoidBone::Hips, "Root_Hip".into());
        avatar
            .bone_mapping
            .insert(HumanoidBone::Head, "Head_Bone".into());

        let s = ron::ser::to_string_pretty(&avatar, ron::ser::PrettyConfig::default()).unwrap();
        let loaded: HumanoidAvatar = ron::from_str(&s).unwrap();
        assert_eq!(loaded.name, "TestAvatar");
        assert_eq!(
            loaded.bone_mapping.get(&HumanoidBone::Hips).unwrap(),
            "Root_Hip"
        );
    }

    #[test]
    fn unknown_bones_ignored() {
        let bones = vec![
            make_bone("SomeRandomBone"),
            make_bone("AnotherUnknown"),
            make_bone("Hips"),
        ];
        let avatar = auto_detect_humanoid(&bones);
        assert_eq!(avatar.bone_mapping.len(), 1);
        assert!(avatar.bone_mapping.contains_key(&HumanoidBone::Hips));
    }
}
