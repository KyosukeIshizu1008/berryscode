//! Animator Controller: a finite state machine for animation playback.
//!
//! Each state references an animation clip (by name or index). Transitions
//! between states fire when conditions are met. Serialized as .banimator files.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimatorController {
    pub name: String,
    pub states: Vec<AnimState>,
    pub transitions: Vec<AnimTransition>,
    pub parameters: Vec<AnimParam>,
    pub default_state: usize,
}

impl Default for AnimatorController {
    fn default() -> Self {
        Self {
            name: "New Controller".into(),
            states: vec![AnimState {
                name: "Idle".into(),
                clip_name: String::new(),
                speed: 1.0,
                looped: true,
                position: [100.0, 100.0],
            }],
            transitions: vec![],
            parameters: vec![],
            default_state: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimState {
    pub name: String,
    pub clip_name: String,
    pub speed: f32,
    pub looped: bool,
    pub position: [f32; 2],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimTransition {
    pub from_state: usize,
    pub to_state: usize,
    pub condition: TransitionCondition,
    pub blend_duration: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransitionCondition {
    /// Transition when a bool parameter is true/false.
    BoolParam { name: String, value: bool },
    /// Transition when a float parameter crosses a threshold.
    FloatGreater { name: String, threshold: f32 },
    FloatLess { name: String, threshold: f32 },
    /// Transition when a trigger parameter is set.
    Trigger { name: String },
    /// Always transition (after clip finishes).
    OnComplete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnimParam {
    Bool { name: String, value: bool },
    Float { name: String, value: f32 },
    Trigger { name: String, fired: bool },
}

impl AnimParam {
    pub fn name(&self) -> &str {
        match self {
            AnimParam::Bool { name, .. } => name,
            AnimParam::Float { name, .. } => name,
            AnimParam::Trigger { name, .. } => name,
        }
    }
}

/// Save controller to .banimator file.
pub fn save_animator(controller: &AnimatorController, path: &str) -> anyhow::Result<()> {
    let s = ron::ser::to_string_pretty(controller, ron::ser::PrettyConfig::default())?;
    std::fs::write(path, s)?;
    Ok(())
}

/// Load controller from .banimator file.
pub fn load_animator(path: &str) -> anyhow::Result<AnimatorController> {
    let s = std::fs::read_to_string(path)?;
    let c: AnimatorController = ron::from_str(&s)?;
    Ok(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_one_state() {
        let c = AnimatorController::default();
        assert_eq!(c.states.len(), 1);
        assert_eq!(c.states[0].name, "Idle");
    }

    #[test]
    fn ron_roundtrip() {
        let mut c = AnimatorController::default();
        c.states.push(AnimState {
            name: "Walk".into(),
            clip_name: "walk_anim".into(),
            speed: 1.0,
            looped: true,
            position: [300.0, 100.0],
        });
        c.transitions.push(AnimTransition {
            from_state: 0,
            to_state: 1,
            condition: TransitionCondition::BoolParam {
                name: "is_walking".into(),
                value: true,
            },
            blend_duration: 0.2,
        });
        c.parameters.push(AnimParam::Bool {
            name: "is_walking".into(),
            value: false,
        });

        let s = ron::ser::to_string_pretty(&c, ron::ser::PrettyConfig::default())
            .expect("serialize");
        let loaded: AnimatorController = ron::from_str(&s).expect("deserialize");
        assert_eq!(loaded.states.len(), 2);
        assert_eq!(loaded.transitions.len(), 1);
    }
}
