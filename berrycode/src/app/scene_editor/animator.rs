#![allow(dead_code)]
//! Animator Controller: a finite state machine for animation playback.
//!
//! Each state references an animation clip (by name or index) or a blend tree.
//! Transitions between states fire when conditions are met.
//! Serialized as .banimator files (RON format).

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Motion: what a state plays (single clip or blend tree)
// ---------------------------------------------------------------------------

/// A motion source: either a single animation clip or a blend tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Motion {
    Clip { clip_name: String },
    BlendTree(BlendTree),
}

impl Default for Motion {
    fn default() -> Self {
        Motion::Clip {
            clip_name: String::new(),
        }
    }
}

/// A blend tree that interpolates between multiple child motions based on
/// one or two float parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlendTree {
    pub name: String,
    pub blend_type: BlendType,
    /// Parameter name that drives blending on the X axis.
    pub parameter_x: String,
    /// Second parameter for 2D blend types (ignored for 1D).
    #[serde(default)]
    pub parameter_y: String,
    /// Child motions with threshold/position values.
    pub children: Vec<BlendTreeChild>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BlendType {
    Simple1D,
    SimpleDirectional2D,
    FreeformDirectional2D,
    FreeformCartesian2D,
}

impl Default for BlendType {
    fn default() -> Self {
        BlendType::Simple1D
    }
}

/// One child in a blend tree. The `motion` field is recursive, allowing nested
/// blend trees (Unity parity).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlendTreeChild {
    pub motion: Motion,
    /// Threshold for 1D blending (position on the parameter axis).
    #[serde(default)]
    pub threshold: f32,
    /// Position for 2D blending.
    #[serde(default)]
    pub position: [f32; 2],
    /// Playback speed multiplier.
    #[serde(default = "default_time_scale")]
    pub time_scale: f32,
}

fn default_time_scale() -> f32 {
    1.0
}

// ---------------------------------------------------------------------------
// Animator Controller (FSM)
// ---------------------------------------------------------------------------

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
            states: vec![
                AnimState {
                    name: "Entry".into(),
                    clip_name: String::new(),
                    motion: Motion::default(),
                    speed: 1.0,
                    looped: false,
                    position: [50.0, 150.0],
                    kind: StateKind::Entry,
                },
                AnimState {
                    name: "Idle".into(),
                    clip_name: String::new(),
                    motion: Motion::default(),
                    speed: 1.0,
                    looped: true,
                    position: [250.0, 150.0],
                    kind: StateKind::Normal,
                },
            ],
            transitions: vec![AnimTransition {
                from_state: 0,
                to_state: 1,
                condition: TransitionCondition::OnComplete,
                blend_duration: 0.0,
                has_exit_time: false,
                exit_time: 1.0,
            }],
            parameters: vec![],
            default_state: 1,
        }
    }
}

// ---------------------------------------------------------------------------
// Animation State
// ---------------------------------------------------------------------------

/// What kind of state this is in the FSM.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum StateKind {
    Normal,
    Entry,
    Exit,
    AnyState,
}

impl Default for StateKind {
    fn default() -> Self {
        StateKind::Normal
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimState {
    pub name: String,
    /// Deprecated: kept for backward compatibility. Use `motion` instead.
    #[serde(default)]
    pub clip_name: String,
    /// The motion this state plays (clip or blend tree).
    #[serde(default)]
    pub motion: Motion,
    #[serde(default = "default_speed")]
    pub speed: f32,
    #[serde(default)]
    pub looped: bool,
    pub position: [f32; 2],
    #[serde(default)]
    pub kind: StateKind,
}

fn default_speed() -> f32 {
    1.0
}

impl AnimState {
    /// Get the effective clip name (from motion or legacy clip_name field).
    pub fn effective_clip_name(&self) -> &str {
        match &self.motion {
            Motion::Clip { clip_name } if !clip_name.is_empty() => clip_name,
            _ => &self.clip_name,
        }
    }

    /// Migrate legacy clip_name into motion field if needed.
    pub fn normalize(&mut self) {
        if !self.clip_name.is_empty() {
            if matches!(&self.motion, Motion::Clip { clip_name } if clip_name.is_empty()) {
                self.motion = Motion::Clip {
                    clip_name: self.clip_name.clone(),
                };
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Transitions
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimTransition {
    pub from_state: usize,
    pub to_state: usize,
    pub condition: TransitionCondition,
    pub blend_duration: f32,
    /// If true, the transition waits until exit_time fraction of the clip.
    #[serde(default)]
    pub has_exit_time: bool,
    /// Fraction of the clip duration at which exit occurs (0.0 to 1.0).
    #[serde(default = "default_exit_time")]
    pub exit_time: f32,
}

fn default_exit_time() -> f32 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransitionCondition {
    /// Transition when a bool parameter is true/false.
    BoolParam { name: String, value: bool },
    /// Transition when a float parameter exceeds a threshold.
    FloatGreater { name: String, threshold: f32 },
    /// Transition when a float parameter is below a threshold.
    FloatLess { name: String, threshold: f32 },
    /// Transition when an int parameter equals a value.
    IntEquals { name: String, value: i64 },
    /// Transition when a trigger parameter is set.
    Trigger { name: String },
    /// Always transition (after clip finishes).
    OnComplete,
}

// ---------------------------------------------------------------------------
// Parameters
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnimParam {
    Bool { name: String, value: bool },
    Float { name: String, value: f32 },
    Int { name: String, value: i64 },
    Trigger { name: String, fired: bool },
}

impl AnimParam {
    pub fn name(&self) -> &str {
        match self {
            AnimParam::Bool { name, .. } => name,
            AnimParam::Float { name, .. } => name,
            AnimParam::Int { name, .. } => name,
            AnimParam::Trigger { name, .. } => name,
        }
    }
}

// ---------------------------------------------------------------------------
// Blend Tree Evaluation
// ---------------------------------------------------------------------------

/// Evaluate a 1D blend tree: returns (child_index, weight) pairs.
/// Children must be sorted by threshold.
pub fn evaluate_blend_1d(tree: &BlendTree, param_value: f32) -> Vec<(usize, f32)> {
    if tree.children.is_empty() {
        return vec![];
    }
    if tree.children.len() == 1 {
        return vec![(0, 1.0)];
    }

    // Sort children by threshold conceptually (they should be pre-sorted)
    let mut sorted: Vec<(usize, f32)> = tree
        .children
        .iter()
        .enumerate()
        .map(|(i, c)| (i, c.threshold))
        .collect();
    sorted.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    // Clamp to range
    if param_value <= sorted[0].1 {
        return vec![(sorted[0].0, 1.0)];
    }
    if param_value >= sorted.last().unwrap().1 {
        return vec![(sorted.last().unwrap().0, 1.0)];
    }

    // Find bracketing pair
    for i in 0..sorted.len() - 1 {
        let (idx_a, t_a) = sorted[i];
        let (idx_b, t_b) = sorted[i + 1];
        if param_value >= t_a && param_value <= t_b {
            let range = t_b - t_a;
            if range.abs() < 0.0001 {
                return vec![(idx_a, 0.5), (idx_b, 0.5)];
            }
            let f = (param_value - t_a) / range;
            return vec![(idx_a, 1.0 - f), (idx_b, f)];
        }
    }

    vec![(0, 1.0)]
}

/// Evaluate a 2D blend tree: returns (child_index, weight) pairs using
/// inverse distance weighting.
pub fn evaluate_blend_2d(tree: &BlendTree, px: f32, py: f32) -> Vec<(usize, f32)> {
    if tree.children.is_empty() {
        return vec![];
    }
    if tree.children.len() == 1 {
        return vec![(0, 1.0)];
    }

    // Inverse distance weighting
    let mut weights: Vec<(usize, f32)> = Vec::new();
    let mut total = 0.0f32;

    for (i, child) in tree.children.iter().enumerate() {
        let dx = px - child.position[0];
        let dy = py - child.position[1];
        let dist = (dx * dx + dy * dy).sqrt().max(0.001);
        // Check if very close to a child
        if dist < 0.01 {
            return vec![(i, 1.0)];
        }
        let w = 1.0 / (dist * dist);
        weights.push((i, w));
        total += w;
    }

    if total > 0.0 {
        for w in &mut weights {
            w.1 /= total;
        }
    }

    // Filter out near-zero weights
    weights.retain(|w| w.1 > 0.001);
    weights
}

// ---------------------------------------------------------------------------
// File I/O
// ---------------------------------------------------------------------------

/// Save controller to .banimator file.
pub fn save_animator(controller: &AnimatorController, path: &str) -> anyhow::Result<()> {
    let s = ron::ser::to_string_pretty(controller, ron::ser::PrettyConfig::default())?;
    std::fs::write(path, s)?;
    Ok(())
}

/// Load controller from .banimator file. Normalizes legacy data.
pub fn load_animator(path: &str) -> anyhow::Result<AnimatorController> {
    let s = std::fs::read_to_string(path)?;
    let mut c: AnimatorController = ron::from_str(&s)?;
    // Migrate legacy clip_name → motion
    for state in &mut c.states {
        state.normalize();
    }
    Ok(c)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_entry_and_idle() {
        let c = AnimatorController::default();
        assert_eq!(c.states.len(), 2);
        assert_eq!(c.states[0].kind, StateKind::Entry);
        assert_eq!(c.states[1].kind, StateKind::Normal);
        assert_eq!(c.states[1].name, "Idle");
    }

    #[test]
    fn ron_roundtrip() {
        let mut c = AnimatorController::default();
        c.states.push(AnimState {
            name: "Walk".into(),
            clip_name: String::new(),
            motion: Motion::Clip {
                clip_name: "walk_anim".into(),
            },
            speed: 1.0,
            looped: true,
            position: [300.0, 100.0],
            kind: StateKind::Normal,
        });
        c.transitions.push(AnimTransition {
            from_state: 1,
            to_state: 2,
            condition: TransitionCondition::BoolParam {
                name: "is_walking".into(),
                value: true,
            },
            blend_duration: 0.2,
            has_exit_time: false,
            exit_time: 1.0,
        });
        c.parameters.push(AnimParam::Bool {
            name: "is_walking".into(),
            value: false,
        });

        let s =
            ron::ser::to_string_pretty(&c, ron::ser::PrettyConfig::default()).expect("serialize");
        let loaded: AnimatorController = ron::from_str(&s).expect("deserialize");
        assert_eq!(loaded.states.len(), 3);
        assert_eq!(loaded.transitions.len(), 2); // default + added
    }

    #[test]
    fn legacy_clip_name_migration() {
        // Simulate old format with clip_name but no motion
        let json = r#"(
            name: "test",
            states: [(
                name: "Walk",
                clip_name: "walk_clip",
                speed: 1.0,
                looped: true,
                position: (100.0, 100.0),
            )],
            transitions: [],
            parameters: [],
            default_state: 0,
        )"#;
        let mut c: AnimatorController = ron::from_str(json).expect("deserialize legacy");
        c.states[0].normalize();
        assert_eq!(c.states[0].effective_clip_name(), "walk_clip");
    }

    #[test]
    fn blend_1d_single_child() {
        let tree = BlendTree {
            name: "test".into(),
            blend_type: BlendType::Simple1D,
            parameter_x: "speed".into(),
            parameter_y: String::new(),
            children: vec![BlendTreeChild {
                motion: Motion::Clip {
                    clip_name: "idle".into(),
                },
                threshold: 0.0,
                position: [0.0, 0.0],
                time_scale: 1.0,
            }],
        };
        let w = evaluate_blend_1d(&tree, 0.5);
        assert_eq!(w.len(), 1);
        assert_eq!(w[0], (0, 1.0));
    }

    #[test]
    fn blend_1d_two_children() {
        let tree = BlendTree {
            name: "locomotion".into(),
            blend_type: BlendType::Simple1D,
            parameter_x: "speed".into(),
            parameter_y: String::new(),
            children: vec![
                BlendTreeChild {
                    motion: Motion::Clip {
                        clip_name: "idle".into(),
                    },
                    threshold: 0.0,
                    position: [0.0, 0.0],
                    time_scale: 1.0,
                },
                BlendTreeChild {
                    motion: Motion::Clip {
                        clip_name: "walk".into(),
                    },
                    threshold: 1.0,
                    position: [0.0, 0.0],
                    time_scale: 1.0,
                },
            ],
        };

        // At 0.0 → 100% idle
        let w = evaluate_blend_1d(&tree, 0.0);
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].0, 0);

        // At 0.5 → 50/50
        let w = evaluate_blend_1d(&tree, 0.5);
        assert_eq!(w.len(), 2);
        assert!((w[0].1 - 0.5).abs() < 0.01);
        assert!((w[1].1 - 0.5).abs() < 0.01);

        // At 1.0 → 100% walk
        let w = evaluate_blend_1d(&tree, 1.0);
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].0, 1);
    }

    #[test]
    fn blend_1d_three_children() {
        let tree = BlendTree {
            name: "locomotion".into(),
            blend_type: BlendType::Simple1D,
            parameter_x: "speed".into(),
            parameter_y: String::new(),
            children: vec![
                BlendTreeChild {
                    motion: Motion::Clip {
                        clip_name: "idle".into(),
                    },
                    threshold: 0.0,
                    position: [0.0, 0.0],
                    time_scale: 1.0,
                },
                BlendTreeChild {
                    motion: Motion::Clip {
                        clip_name: "walk".into(),
                    },
                    threshold: 0.5,
                    position: [0.0, 0.0],
                    time_scale: 1.0,
                },
                BlendTreeChild {
                    motion: Motion::Clip {
                        clip_name: "run".into(),
                    },
                    threshold: 1.0,
                    position: [0.0, 0.0],
                    time_scale: 1.0,
                },
            ],
        };

        // At 0.75 → between walk(0.5) and run(1.0)
        let w = evaluate_blend_1d(&tree, 0.75);
        assert_eq!(w.len(), 2);
        assert_eq!(w[0].0, 1); // walk
        assert_eq!(w[1].0, 2); // run
        assert!((w[0].1 - 0.5).abs() < 0.01);
        assert!((w[1].1 - 0.5).abs() < 0.01);
    }

    #[test]
    fn blend_2d_basic() {
        let tree = BlendTree {
            name: "directional".into(),
            blend_type: BlendType::SimpleDirectional2D,
            parameter_x: "vel_x".into(),
            parameter_y: "vel_y".into(),
            children: vec![
                BlendTreeChild {
                    motion: Motion::Clip {
                        clip_name: "idle".into(),
                    },
                    threshold: 0.0,
                    position: [0.0, 0.0],
                    time_scale: 1.0,
                },
                BlendTreeChild {
                    motion: Motion::Clip {
                        clip_name: "forward".into(),
                    },
                    threshold: 0.0,
                    position: [0.0, 1.0],
                    time_scale: 1.0,
                },
            ],
        };

        // At origin → closest to idle
        let w = evaluate_blend_2d(&tree, 0.0, 0.0);
        assert!(!w.is_empty());
        // Exact match with child 0
        assert_eq!(w[0].0, 0);
        assert!((w[0].1 - 1.0).abs() < 0.01);
    }

    #[test]
    fn state_kind_default_is_normal() {
        let kind = StateKind::default();
        assert_eq!(kind, StateKind::Normal);
    }

    #[test]
    fn int_param() {
        let p = AnimParam::Int {
            name: "level".into(),
            value: 5,
        };
        assert_eq!(p.name(), "level");
    }

    #[test]
    fn transition_with_exit_time() {
        let t = AnimTransition {
            from_state: 0,
            to_state: 1,
            condition: TransitionCondition::OnComplete,
            blend_duration: 0.3,
            has_exit_time: true,
            exit_time: 0.9,
        };
        assert!(t.has_exit_time);
        assert!((t.exit_time - 0.9).abs() < 0.001);
    }
}
