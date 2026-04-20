//! In-memory scene data model for the Unity-like scene editor.
//!
//! The scene model is the editor-side source of truth for the entities the user
//! is editing. It is intentionally decoupled from Bevy's `World`/`Scene` types
//! so we can serialize, undo/redo, and freely mutate without touching the
//! preview ECS world directly.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Transform data for a scene entity (position, rotation as euler radians, scale).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformData {
    pub translation: [f32; 3],
    /// Euler angles in radians (XYZ order)
    pub rotation_euler: [f32; 3],
    pub scale: [f32; 3],
}

impl Default for TransformData {
    fn default() -> Self {
        Self {
            translation: [0.0, 0.0, 0.0],
            rotation_euler: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
        }
    }
}

/// A single keyframe in an [`ComponentData::Animation`] track. The transform is
/// a full snapshot (translation / rotation-euler / scale) at the given time in
/// seconds relative to the start of the animation.
///
/// NOTE: This is the legacy v1 keyframe type. New code should use
/// [`AnimationTrack`] / [`TrackKeyframe`] instead.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keyframe {
    pub time: f32,
    pub transform: TransformData,
}

// ---------------------------------------------------------------------------
// v2 Phase 8: Easing curves + multi-property animation tracks
// ---------------------------------------------------------------------------

/// Easing function applied between two keyframes.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum EasingType {
    #[default]
    Linear,
    EaseInQuad,
    EaseOutQuad,
    EaseInOutQuad,
    EaseInCubic,
    EaseOutCubic,
    EaseInOutCubic,
    EaseInOutSine,
}

impl EasingType {
    pub const ALL: &'static [EasingType] = &[
        EasingType::Linear,
        EasingType::EaseInQuad,
        EasingType::EaseOutQuad,
        EasingType::EaseInOutQuad,
        EasingType::EaseInCubic,
        EasingType::EaseOutCubic,
        EasingType::EaseInOutCubic,
        EasingType::EaseInOutSine,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            EasingType::Linear => "Linear",
            EasingType::EaseInQuad => "EaseIn Quad",
            EasingType::EaseOutQuad => "EaseOut Quad",
            EasingType::EaseInOutQuad => "EaseInOut Quad",
            EasingType::EaseInCubic => "EaseIn Cubic",
            EasingType::EaseOutCubic => "EaseOut Cubic",
            EasingType::EaseInOutCubic => "EaseInOut Cubic",
            EasingType::EaseInOutSine => "EaseInOut Sine",
        }
    }
}

/// Which transform property an [`AnimationTrack`] drives.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum AnimProperty {
    #[default]
    Position,
    Rotation,
    Scale,
}

impl AnimProperty {
    pub const ALL: &'static [AnimProperty] = &[
        AnimProperty::Position,
        AnimProperty::Rotation,
        AnimProperty::Scale,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            AnimProperty::Position => "Position",
            AnimProperty::Rotation => "Rotation",
            AnimProperty::Scale => "Scale",
        }
    }
}

/// An animation event fired at a specific time during playback.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationEvent {
    pub time: f32,
    pub callback_name: String,
}

/// A single animation track that drives one transform property over time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationTrack {
    pub property: AnimProperty,
    pub keyframes: Vec<TrackKeyframe>,
    #[serde(default)]
    pub events: Vec<AnimationEvent>,
}

/// A keyframe within an [`AnimationTrack`]. Stores a `[f32; 3]` value
/// (position / euler-rotation / scale) and the easing curve to use when
/// interpolating from this keyframe to the next one.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackKeyframe {
    pub time: f32,
    pub value: [f32; 3],
    #[serde(default)]
    pub easing: EasingType,
}

/// Default metallic value for newly-created or legacy mesh components (non-metal).
fn default_metallic() -> f32 {
    0.0
}

/// Default perceptual roughness for newly-created or legacy mesh components.
fn default_roughness() -> f32 {
    0.5
}

/// Default volume for newly-created or legacy AudioSource components.
fn default_audio_volume() -> f32 {
    1.0
}

/// Default autoplay flag for newly-created or legacy AudioSource components.
fn default_audio_autoplay() -> bool {
    true
}

/// Default mass for a newly-created RigidBody component.
fn default_mass() -> f32 {
    1.0
}

/// Default friction for a newly-created Collider component.
fn default_friction() -> f32 {
    0.5
}

/// Default restitution (bounciness) for a newly-created Collider component.
fn default_restitution() -> f32 {
    0.0
}

/// Default font size for a newly-created or legacy `UiText` component.
fn default_ui_font_size() -> f32 {
    16.0
}

/// Default text color (RGBA) for a newly-created or legacy `UiText` component.
fn default_ui_color() -> [f32; 4] {
    [1.0, 1.0, 1.0, 1.0]
}

/// Default background color (RGBA) for a newly-created or legacy `UiButton`.
fn default_ui_button_bg() -> [f32; 4] {
    [0.2, 0.2, 0.3, 1.0]
}

/// Default tint color (RGBA) for a newly-created or legacy `UiImage` component.
fn default_ui_tint() -> [f32; 4] {
    [1.0, 1.0, 1.0, 1.0]
}

/// Default emission rate (particles per second) for a newly-created `ParticleEmitter`.
fn default_particle_rate() -> f32 {
    30.0
}

/// Default per-particle lifetime, in seconds.
fn default_particle_lifetime() -> f32 {
    1.5
}

/// Default initial upward speed for emitted particles.
fn default_particle_speed() -> f32 {
    2.0
}

/// Default lateral spread (0..=1) applied to emitted particle velocities.
fn default_particle_spread() -> f32 {
    0.3
}

/// Default per-particle size (used for both start and end if unspecified).
fn default_particle_size() -> f32 {
    0.1
}

/// Default start color (RGBA) for emitted particles — warm orange.
fn default_particle_start_color() -> [f32; 4] {
    [1.0, 0.6, 0.2, 1.0]
}

/// Default end color (RGBA) for emitted particles — fade to transparent red.
fn default_particle_end_color() -> [f32; 4] {
    [1.0, 0.0, 0.0, 0.0]
}

/// Default cap on the number of live particles for an emitter.
fn default_particle_max() -> u32 {
    200
}

/// Default gravity acceleration applied to particle Y velocity each second.
fn default_particle_gravity() -> f32 {
    -1.0
}

/// Default intensity for a newly-created SpotLight component.
fn default_spot_intensity() -> f32 {
    10000.0
}

/// Default white color for light components.
fn default_white_color() -> [f32; 3] {
    [1.0, 1.0, 1.0]
}

/// Default range for a newly-created SpotLight component.
fn default_spot_range() -> f32 {
    20.0
}

/// Default inner cone angle (radians) for a newly-created SpotLight.
fn default_spot_inner_angle() -> f32 {
    0.5
}

/// Default outer cone angle (radians) for a newly-created SpotLight.
fn default_spot_outer_angle() -> f32 {
    0.8
}

/// Default intensity for a newly-created DirectionalLight component.
fn default_dir_intensity() -> f32 {
    10000.0
}

/// Default enabled state for a newly-created or legacy entity.
fn default_true() -> bool {
    true
}

/// Default total duration (seconds) for a newly-created `Animation` component.
fn default_animation_duration() -> f32 {
    2.0
}

/// Default loop flag for a newly-created `Animation` component.
fn default_animation_loop() -> bool {
    true
}

fn default_terrain_resolution() -> u32 {
    64
}

fn default_terrain_world_size() -> [f32; 2] {
    [100.0, 100.0]
}

fn default_terrain_heights() -> Vec<f32> {
    vec![0.0; 64 * 64]
}

fn default_terrain_base_color() -> [f32; 3] {
    [0.3, 0.5, 0.3]
}

fn default_navmesh_cell_size() -> f32 {
    1.0
}

/// Rigid body simulation mode (editor-side metadata; not yet wired to a
/// physics engine — see Phase J docs).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum RigidBodyType {
    #[default]
    Dynamic,
    Static,
    Kinematic,
}

impl RigidBodyType {
    pub fn label(&self) -> &'static str {
        match self {
            RigidBodyType::Dynamic => "Dynamic",
            RigidBodyType::Static => "Static",
            RigidBodyType::Kinematic => "Kinematic",
        }
    }
    pub const ALL: &'static [RigidBodyType] = &[
        RigidBodyType::Dynamic,
        RigidBodyType::Static,
        RigidBodyType::Kinematic,
    ];
}

/// Collider shape (editor-side metadata; visualized as a wireframe in the
/// Scene View). Actual physics simulation is deferred to a follow-up phase.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ColliderShape {
    Box { half_extents: [f32; 3] },
    Sphere { radius: f32 },
    Capsule { half_height: f32, radius: f32 },
}

impl Default for ColliderShape {
    fn default() -> Self {
        ColliderShape::Box {
            half_extents: [0.5, 0.5, 0.5],
        }
    }
}

impl ColliderShape {
    pub fn label(&self) -> &'static str {
        match self {
            ColliderShape::Box { .. } => "Box",
            ColliderShape::Sphere { .. } => "Sphere",
            ColliderShape::Capsule { .. } => "Capsule",
        }
    }
}

/// One named field on a [`ComponentData::CustomScript`]. The value is a tagged
/// enum so the inspector can render the correct editor widget per field type.
///
/// This is intentionally schemaless on the editor side: the runtime game
/// interprets `(name, value)` pairs however it wants (typically by matching on
/// `type_name` and populating its own `#[derive(Component)]` struct). The
/// editor makes NO claims about type safety — it just round-trips the data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptField {
    pub name: String,
    pub value: ScriptValue,
}

/// Value carried by a [`ScriptField`]. A small fixed set of primitive types
/// covers the vast majority of custom component fields. Richer types (vectors,
/// nested structs, etc.) can be added in later phases as user demand dictates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScriptValue {
    Float(f32),
    Int(i64),
    Bool(bool),
    String(String),
    /// Variable-length list of values (e.g. `Vec<f32>`).
    Vec(Vec<ScriptValue>),
    /// Optional value (e.g. `Option<f32>`). `None` means the option is unset.
    Option(Option<Box<ScriptValue>>),
    /// Key-value map (e.g. `HashMap<String, f32>`).
    Map(Vec<(String, ScriptValue)>),
}

impl ScriptValue {
    /// Short Rust-style label for the inspector (e.g. `"f32"`, `"i64"`).
    pub fn type_label(&self) -> &'static str {
        match self {
            ScriptValue::Float(_) => "f32",
            ScriptValue::Int(_) => "i64",
            ScriptValue::Bool(_) => "bool",
            ScriptValue::String(_) => "String",
            ScriptValue::Vec(_) => "Vec",
            ScriptValue::Option(_) => "Option",
            ScriptValue::Map(_) => "Map",
        }
    }
}

/// A single level in a [`ComponentData::LodGroup`]. Each level references a mesh
/// asset and specifies the minimum screen-space percentage at which it should be
/// used. Levels are expected to be ordered by descending `screen_percentage`
/// (highest detail first).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LodLevel {
    /// Path to the mesh asset for this LOD level (relative or absolute).
    #[serde(default)]
    pub mesh_path: String,
    /// Minimum screen-space coverage (0.0 = invisible, 1.0 = full screen) at
    /// which this level should be displayed. Higher values mean more detail.
    #[serde(default)]
    pub screen_percentage: f32,
}

/// Components that an entity can carry. This is a small, editor-friendly subset
/// that mirrors common Bevy bundles.
///
/// Mesh* variants carry inline PBR material properties (metallic, roughness,
/// emissive). The `serde(default = ...)` attributes ensure existing `.bscene`
/// files written before Phase G still load — missing fields fall back to
/// non-metal / mid-roughness / no-emission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComponentData {
    MeshCube {
        size: f32,
        color: [f32; 3],
        #[serde(default = "default_metallic")]
        metallic: f32,
        #[serde(default = "default_roughness")]
        roughness: f32,
        #[serde(default)]
        emissive: [f32; 3],
        #[serde(default)]
        texture_path: Option<String>,
        #[serde(default)]
        normal_map_path: Option<String>,
    },
    MeshSphere {
        radius: f32,
        color: [f32; 3],
        #[serde(default = "default_metallic")]
        metallic: f32,
        #[serde(default = "default_roughness")]
        roughness: f32,
        #[serde(default)]
        emissive: [f32; 3],
        #[serde(default)]
        texture_path: Option<String>,
        #[serde(default)]
        normal_map_path: Option<String>,
    },
    MeshPlane {
        size: f32,
        color: [f32; 3],
        #[serde(default = "default_metallic")]
        metallic: f32,
        #[serde(default = "default_roughness")]
        roughness: f32,
        #[serde(default)]
        emissive: [f32; 3],
        #[serde(default)]
        texture_path: Option<String>,
        #[serde(default)]
        normal_map_path: Option<String>,
    },
    Light {
        intensity: f32,
        color: [f32; 3],
    },
    SpotLight {
        #[serde(default = "default_spot_intensity")]
        intensity: f32,
        #[serde(default = "default_white_color")]
        color: [f32; 3],
        #[serde(default = "default_spot_range")]
        range: f32,
        #[serde(default = "default_spot_inner_angle")]
        inner_angle: f32,
        #[serde(default = "default_spot_outer_angle")]
        outer_angle: f32,
    },
    DirectionalLight {
        #[serde(default = "default_dir_intensity")]
        intensity: f32,
        #[serde(default = "default_white_color")]
        color: [f32; 3],
        #[serde(default)]
        shadows: bool,
    },
    Camera,
    MeshFromFile {
        path: String,
        #[serde(default)]
        texture_path: Option<String>,
        #[serde(default)]
        normal_map_path: Option<String>,
    },
    AudioSource {
        /// Path to an audio file (relative to the project's `assets/` dir, or absolute).
        path: String,
        #[serde(default = "default_audio_volume")]
        volume: f32,
        #[serde(default)]
        looped: bool,
        #[serde(default = "default_audio_autoplay")]
        autoplay: bool,
    },
    AudioListener,
    RigidBody {
        #[serde(default)]
        body_type: RigidBodyType,
        #[serde(default = "default_mass")]
        mass: f32,
    },
    Collider {
        #[serde(default)]
        shape: ColliderShape,
        #[serde(default = "default_friction")]
        friction: f32,
        #[serde(default = "default_restitution")]
        restitution: f32,
    },
    /// Editor-authored UI text node. Runtime rendering is delegated to
    /// `bevy_ui` in the generated game; in the Scene View it is represented by
    /// a small placeholder gizmo so the entity remains click-selectable.
    UiText {
        text: String,
        #[serde(default = "default_ui_font_size")]
        font_size: f32,
        #[serde(default = "default_ui_color")]
        color: [f32; 4],
    },
    /// Editor-authored UI button node.
    UiButton {
        label: String,
        #[serde(default = "default_ui_button_bg")]
        background: [f32; 4],
    },
    /// Editor-authored UI image node. `path` is an asset-relative or absolute
    /// filesystem path to a texture.
    UiImage {
        path: String,
        #[serde(default = "default_ui_tint")]
        tint: [f32; 4],
    },
    /// Editor-authored particle emitter (Phase M). All parameters are
    /// authoring-time settings; the editor renders a live 2D dot preview in
    /// the Scene View. Runtime games can wire these into their own particle
    /// library (e.g. `bevy_hanabi` or a custom system).
    ParticleEmitter {
        #[serde(default = "default_particle_rate")]
        rate: f32,
        #[serde(default = "default_particle_lifetime")]
        lifetime: f32,
        #[serde(default = "default_particle_speed")]
        speed: f32,
        #[serde(default = "default_particle_spread")]
        spread: f32,
        #[serde(default = "default_particle_size")]
        start_size: f32,
        #[serde(default = "default_particle_size")]
        end_size: f32,
        #[serde(default = "default_particle_start_color")]
        start_color: [f32; 4],
        #[serde(default = "default_particle_end_color")]
        end_color: [f32; 4],
        #[serde(default = "default_particle_max")]
        max_particles: u32,
        #[serde(default = "default_particle_gravity")]
        gravity: f32,
    },
    /// Editor-authored keyframe animation (Phase K / Phase 8 v2).
    /// Multi-track: each track drives one transform property (position,
    /// rotation, scale) with per-keyframe easing curves.
    Animation {
        #[serde(default = "default_animation_duration")]
        duration: f32,
        #[serde(default)]
        tracks: Vec<AnimationTrack>,
        #[serde(default = "default_animation_loop")]
        looped: bool,
    },
    /// Editor-authored custom script attachment (Phase L). Stores a
    /// user-provided Rust type name plus a list of key/value fields. The
    /// editor renders a generic inspector (name + typed value) and makes NO
    /// static-typing guarantees: the runtime game is expected to match on
    /// `type_name`, read the fields it cares about, and insert its own
    /// `#[derive(Component)]` struct at spawn time.
    CustomScript {
        #[serde(default)]
        type_name: String,
        #[serde(default)]
        fields: Vec<ScriptField>,
    },
    /// Editor-authored skybox reference. Points to an HDR/EXR image on disk.
    /// In the Scene View the editor shows a small sky-blue sphere placeholder
    /// (actual HDR loading requires a full asset pipeline pass).
    Skybox {
        #[serde(default)]
        path: String,
    },
    /// Editor-authored animator controller reference (Phase 13). Points to
    /// a `.banimator` file on disk that defines the animation state machine.
    Animator {
        #[serde(default)]
        controller_path: String,
    },
    /// Editor-authored LOD (Level of Detail) group (Phase 61). Contains
    /// multiple mesh levels that the runtime switches between based on
    /// screen-space coverage.
    LodGroup {
        #[serde(default)]
        levels: Vec<LodLevel>,
    },
    /// Editor-authored spline / path (Phase 59). A sequence of cubic Bezier
    /// control points that can define camera rails, AI paths, or any
    /// curve-based data.
    Spline {
        #[serde(default)]
        points: Vec<super::spline::SplinePoint>,
        #[serde(default)]
        closed: bool,
    },
    /// Editor-authored terrain heightmap (Phase 65). Grid-based height data
    /// with configurable resolution and world size.
    Terrain {
        #[serde(default = "default_terrain_resolution")]
        resolution: u32,
        #[serde(default = "default_terrain_world_size")]
        world_size: [f32; 2],
        #[serde(default = "default_terrain_heights")]
        heights: Vec<f32>,
        #[serde(default = "default_terrain_base_color")]
        base_color: [f32; 3],
    },
    /// Editor-authored skinned mesh / skeletal animation (Phase 68). References
    /// a .glb/.gltf file and stores extracted bone hierarchy data.
    SkinnedMesh {
        #[serde(default)]
        path: String,
        #[serde(default)]
        bones: Vec<super::skeleton::BoneData>,
    },
    /// Editor-authored visual script reference (Phase 71). Points to a
    /// `.bvscript` file on disk that defines the node-based logic graph.
    VisualScript {
        #[serde(default)]
        path: String,
    },
    /// Editor-authored navigation mesh (Phase 70). Stores a baked grid for
    /// A* pathfinding.
    NavMesh {
        #[serde(default = "default_navmesh_cell_size")]
        cell_size: f32,
        #[serde(default)]
        grid: Vec<bool>,
        #[serde(default)]
        width: usize,
        #[serde(default)]
        height: usize,
    },
}

impl ComponentData {
    /// Returns a list of all component types with their display names and default instances.
    pub fn default_all() -> Vec<(&'static str, ComponentData)> {
        vec![
            (
                "Mesh Cube",
                ComponentData::MeshCube {
                    size: 1.0,
                    color: [0.5, 0.5, 1.0],
                    metallic: 0.0,
                    roughness: 0.5,
                    emissive: [0.0, 0.0, 0.0],
                    texture_path: None,
                    normal_map_path: None,
                },
            ),
            (
                "Mesh Sphere",
                ComponentData::MeshSphere {
                    radius: 0.5,
                    color: [1.0, 0.5, 0.5],
                    metallic: 0.0,
                    roughness: 0.5,
                    emissive: [0.0, 0.0, 0.0],
                    texture_path: None,
                    normal_map_path: None,
                },
            ),
            (
                "Mesh Plane",
                ComponentData::MeshPlane {
                    size: 10.0,
                    color: [0.3, 0.3, 0.3],
                    metallic: 0.0,
                    roughness: 0.5,
                    emissive: [0.0, 0.0, 0.0],
                    texture_path: None,
                    normal_map_path: None,
                },
            ),
            (
                "Mesh From File",
                ComponentData::MeshFromFile {
                    path: String::new(),
                    texture_path: None,
                    normal_map_path: None,
                },
            ),
            (
                "Light",
                ComponentData::Light {
                    intensity: 10000.0,
                    color: [1.0, 1.0, 1.0],
                },
            ),
            (
                "Spot Light",
                ComponentData::SpotLight {
                    intensity: 10000.0,
                    color: [1.0, 1.0, 1.0],
                    range: 20.0,
                    inner_angle: 0.5,
                    outer_angle: 0.8,
                },
            ),
            (
                "Directional Light",
                ComponentData::DirectionalLight {
                    intensity: 10000.0,
                    color: [1.0, 1.0, 1.0],
                    shadows: false,
                },
            ),
            ("Camera", ComponentData::Camera),
            (
                "Audio Source",
                ComponentData::AudioSource {
                    path: String::new(),
                    volume: 1.0,
                    looped: false,
                    autoplay: true,
                },
            ),
            ("Audio Listener", ComponentData::AudioListener),
            (
                "Rigidbody",
                ComponentData::RigidBody {
                    body_type: RigidBodyType::Dynamic,
                    mass: 1.0,
                },
            ),
            (
                "Collider",
                ComponentData::Collider {
                    shape: ColliderShape::default(),
                    friction: 0.5,
                    restitution: 0.0,
                },
            ),
            (
                "UI Text",
                ComponentData::UiText {
                    text: "Text".into(),
                    font_size: 16.0,
                    color: [1.0, 1.0, 1.0, 1.0],
                },
            ),
            (
                "UI Button",
                ComponentData::UiButton {
                    label: "Button".into(),
                    background: [0.2, 0.2, 0.3, 1.0],
                },
            ),
            (
                "UI Image",
                ComponentData::UiImage {
                    path: String::new(),
                    tint: [1.0, 1.0, 1.0, 1.0],
                },
            ),
            (
                "Particle Emitter",
                ComponentData::ParticleEmitter {
                    rate: 30.0,
                    lifetime: 1.5,
                    speed: 2.0,
                    spread: 0.3,
                    start_size: 0.1,
                    end_size: 0.0,
                    start_color: [1.0, 0.6, 0.2, 1.0],
                    end_color: [1.0, 0.0, 0.0, 0.0],
                    max_particles: 200,
                    gravity: -1.0,
                },
            ),
            (
                "Animation",
                ComponentData::Animation {
                    duration: 2.0,
                    tracks: vec![],
                    looped: true,
                },
            ),
            (
                "Custom Script",
                ComponentData::CustomScript {
                    type_name: String::new(),
                    fields: vec![],
                },
            ),
            (
                "Skybox",
                ComponentData::Skybox {
                    path: String::new(),
                },
            ),
            (
                "Animator",
                ComponentData::Animator {
                    controller_path: String::new(),
                },
            ),
            ("LOD Group", ComponentData::LodGroup { levels: vec![] }),
            (
                "Spline",
                ComponentData::Spline {
                    points: vec![],
                    closed: false,
                },
            ),
            (
                "Terrain",
                ComponentData::Terrain {
                    resolution: 64,
                    world_size: [100.0, 100.0],
                    heights: vec![0.0; 64 * 64],
                    base_color: [0.3, 0.5, 0.3],
                },
            ),
            (
                "Skinned Mesh",
                ComponentData::SkinnedMesh {
                    path: String::new(),
                    bones: vec![],
                },
            ),
            (
                "Visual Script",
                ComponentData::VisualScript {
                    path: String::new(),
                },
            ),
            (
                "NavMesh",
                ComponentData::NavMesh {
                    cell_size: 1.0,
                    grid: vec![],
                    width: 0,
                    height: 0,
                },
            ),
        ]
    }

    /// Short, human-readable name for the inspector header.
    pub fn label(&self) -> &'static str {
        match self {
            ComponentData::MeshCube { .. } => "Cube",
            ComponentData::MeshSphere { .. } => "Sphere",
            ComponentData::MeshPlane { .. } => "Plane",
            ComponentData::Light { .. } => "Light",
            ComponentData::SpotLight { .. } => "Spot Light",
            ComponentData::DirectionalLight { .. } => "Directional Light",
            ComponentData::Camera => "Camera",
            ComponentData::MeshFromFile { .. } => "Mesh",
            ComponentData::AudioSource { .. } => "Audio Source",
            ComponentData::AudioListener => "Audio Listener",
            ComponentData::RigidBody { .. } => "Rigidbody",
            ComponentData::Collider { .. } => "Collider",
            ComponentData::UiText { .. } => "UI Text",
            ComponentData::UiButton { .. } => "UI Button",
            ComponentData::UiImage { .. } => "UI Image",
            ComponentData::ParticleEmitter { .. } => "Particle Emitter",
            ComponentData::Animation { .. } => "Animation",
            // NOTE: `label()` returns a `&'static str`, so we can't embed the
            // user-typed `type_name` here. The inspector renders `type_name`
            // separately as a dedicated text field.
            ComponentData::CustomScript { .. } => "Custom Script",
            ComponentData::Skybox { .. } => "Skybox",
            ComponentData::Animator { .. } => "Animator",
            ComponentData::LodGroup { .. } => "LOD Group",
            ComponentData::Spline { .. } => "Spline",
            ComponentData::Terrain { .. } => "Terrain",
            ComponentData::SkinnedMesh { .. } => "Skinned Mesh",
            ComponentData::VisualScript { .. } => "Visual Script",
            ComponentData::NavMesh { .. } => "NavMesh",
        }
    }
}

/// A single entity in the editor scene. Children are stored as IDs so we can
/// re-parent without invalidating references.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneEntity {
    pub id: u64,
    pub name: String,
    pub transform: TransformData,
    pub components: Vec<ComponentData>,
    /// Skipped during (de)serialization: rebuilt from `parent` after load.
    #[serde(skip)]
    pub children: Vec<u64>,
    pub parent: Option<u64>,
    /// Whether the entity is enabled. Disabled entities are hidden from the
    /// Bevy sync and cannot be selected in the Scene View.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// If this entity was instantiated from a prefab, this stores the file path
    /// of the source `.bprefab` so the inspector can offer Revert / Apply.
    #[serde(default)]
    pub prefab_source: Option<String>,
}

impl SceneEntity {
    pub fn new(id: u64, name: String, components: Vec<ComponentData>) -> Self {
        Self {
            id,
            name,
            transform: TransformData::default(),
            components,
            children: Vec::new(),
            parent: None,
            enabled: true,
            prefab_source: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Quaternion helpers for local/world transform composition
// ---------------------------------------------------------------------------

/// Quaternion as [x, y, z, w].
type Quat4 = [f32; 4];

fn quat_from_euler(euler: [f32; 3]) -> Quat4 {
    let (sx, cx) = (euler[0] * 0.5).sin_cos();
    let (sy, cy) = (euler[1] * 0.5).sin_cos();
    let (sz, cz) = (euler[2] * 0.5).sin_cos();
    [
        sx * cy * cz + cx * sy * sz,
        cx * sy * cz - sx * cy * sz,
        cx * cy * sz + sx * sy * cz,
        cx * cy * cz - sx * sy * sz,
    ]
}

fn euler_from_quat(q: Quat4) -> [f32; 3] {
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

fn quat_mul(a: Quat4, b: Quat4) -> Quat4 {
    [
        a[3] * b[0] + a[0] * b[3] + a[1] * b[2] - a[2] * b[1],
        a[3] * b[1] - a[0] * b[2] + a[1] * b[3] + a[2] * b[0],
        a[3] * b[2] + a[0] * b[1] - a[1] * b[0] + a[2] * b[3],
        a[3] * b[3] - a[0] * b[0] - a[1] * b[1] - a[2] * b[2],
    ]
}

fn quat_rotate(q: Quat4, v: [f32; 3]) -> [f32; 3] {
    let [qx, qy, qz, qw] = q;
    let t = [
        2.0 * (qy * v[2] - qz * v[1]),
        2.0 * (qz * v[0] - qx * v[2]),
        2.0 * (qx * v[1] - qy * v[0]),
    ];
    [
        v[0] + qw * t[0] + qy * t[2] - qz * t[1],
        v[1] + qw * t[1] + qz * t[0] - qx * t[2],
        v[2] + qw * t[2] + qx * t[1] - qy * t[0],
    ]
}

fn quat_inverse(q: Quat4) -> Quat4 {
    let len_sq = q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3];
    if len_sq < 1e-10 {
        return [0.0, 0.0, 0.0, 1.0];
    }
    let inv = 1.0 / len_sq;
    [-q[0] * inv, -q[1] * inv, -q[2] * inv, q[3] * inv]
}

/// Compose two transforms: result = parent * child (in transform space).
/// This gives the world transform of a child given its parent's world transform
/// and the child's local transform.
pub fn compose_transforms(parent: &TransformData, child: &TransformData) -> TransformData {
    let pr = quat_from_euler(parent.rotation_euler);
    let cr = quat_from_euler(child.rotation_euler);

    // Combined rotation
    let combined_rot = quat_mul(pr, cr);

    // Scale child's translation by parent's scale, rotate by parent's rotation,
    // then add parent's translation.
    let scaled_child_pos = [
        child.translation[0] * parent.scale[0],
        child.translation[1] * parent.scale[1],
        child.translation[2] * parent.scale[2],
    ];
    let rotated_pos = quat_rotate(pr, scaled_child_pos);

    TransformData {
        translation: [
            parent.translation[0] + rotated_pos[0],
            parent.translation[1] + rotated_pos[1],
            parent.translation[2] + rotated_pos[2],
        ],
        rotation_euler: euler_from_quat(combined_rot),
        scale: [
            parent.scale[0] * child.scale[0],
            parent.scale[1] * child.scale[1],
            parent.scale[2] * child.scale[2],
        ],
    }
}

/// Compute the local transform of a child given its world transform and its
/// parent's world transform: local = inverse(parent_world) * child_world.
pub fn compute_local_from_world(
    parent_world: &TransformData,
    child_world: &TransformData,
) -> TransformData {
    let pr = quat_from_euler(parent_world.rotation_euler);
    let inv_pr = quat_inverse(pr);

    // Inverse scale
    let inv_scale = [
        if parent_world.scale[0].abs() > 1e-6 {
            1.0 / parent_world.scale[0]
        } else {
            1.0
        },
        if parent_world.scale[1].abs() > 1e-6 {
            1.0 / parent_world.scale[1]
        } else {
            1.0
        },
        if parent_world.scale[2].abs() > 1e-6 {
            1.0 / parent_world.scale[2]
        } else {
            1.0
        },
    ];

    // Local translation: unrotate(child_world_pos - parent_world_pos) * inv_scale
    let diff = [
        child_world.translation[0] - parent_world.translation[0],
        child_world.translation[1] - parent_world.translation[1],
        child_world.translation[2] - parent_world.translation[2],
    ];
    let unrotated = quat_rotate(inv_pr, diff);
    let local_translation = [
        unrotated[0] * inv_scale[0],
        unrotated[1] * inv_scale[1],
        unrotated[2] * inv_scale[2],
    ];

    // Local rotation: inverse(parent_rot) * child_rot
    let cr = quat_from_euler(child_world.rotation_euler);
    let local_rot = quat_mul(inv_pr, cr);

    TransformData {
        translation: local_translation,
        rotation_euler: euler_from_quat(local_rot),
        scale: [
            child_world.scale[0] * inv_scale[0],
            child_world.scale[1] * inv_scale[1],
            child_world.scale[2] * inv_scale[2],
        ],
    }
}

/// The full editor scene state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SceneModel {
    pub entities: HashMap<u64, SceneEntity>,
    /// Entities without a parent, in display order. Rebuilt on load.
    #[serde(skip)]
    pub root_entities: Vec<u64>,
    pub next_id: u64,
    /// Editor-only selection state; not persisted. Supports multi-select.
    #[serde(skip)]
    pub selected_ids: HashSet<u64>,
    /// Editor-only path of the on-disk scene; not persisted.
    #[serde(skip)]
    pub file_path: Option<String>,
    /// Editor-only dirty flag; not persisted.
    #[serde(skip)]
    pub modified: bool,
    /// Global Bevy Resources defined for this scene.
    #[serde(default)]
    pub resources: Vec<super::resource_editor::ResourceDef>,
}

impl SceneModel {
    /// Create an empty scene model.
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            root_entities: Vec::new(),
            next_id: 1,
            selected_ids: HashSet::new(),
            file_path: None,
            modified: false,
            resources: Vec::new(),
        }
    }

    /// Compute the world-space transform for an entity by composing the local
    /// transforms along the parent chain (root -> ... -> parent -> self).
    pub fn compute_world_transform(&self, id: u64) -> TransformData {
        let entity = match self.entities.get(&id) {
            Some(e) => e,
            None => return TransformData::default(),
        };

        match entity.parent {
            None => entity.transform.clone(),
            Some(parent_id) => {
                let parent_world = self.compute_world_transform(parent_id);
                compose_transforms(&parent_world, &entity.transform)
            }
        }
    }

    /// Add a new top-level entity and return its id.
    pub fn add_entity(&mut self, name: String, components: Vec<ComponentData>) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let entity = SceneEntity::new(id, name, components);
        self.entities.insert(id, entity);
        self.root_entities.push(id);
        self.modified = true;
        id
    }

    /// Remove an entity and all of its descendants. If the removed entity was
    /// selected, selection is cleared.
    pub fn remove_entity(&mut self, id: u64) {
        // Collect the full subtree first so we can mutate the map afterwards.
        let mut to_remove: Vec<u64> = Vec::new();
        self.collect_subtree(id, &mut to_remove);

        // Detach from parent or root list
        if let Some(entity) = self.entities.get(&id) {
            if let Some(parent_id) = entity.parent {
                if let Some(parent) = self.entities.get_mut(&parent_id) {
                    parent.children.retain(|c| *c != id);
                }
            } else {
                self.root_entities.retain(|c| *c != id);
            }
        }

        for victim in to_remove {
            self.entities.remove(&victim);
            self.selected_ids.remove(&victim);
        }

        self.modified = true;
    }

    /// Re-parent `child_id` to `parent_id`. Pass `None` for the parent to make
    /// the entity a root entity. Cycles are rejected silently.
    ///
    /// The child's local transform is recomputed so that its world-space
    /// position/rotation/scale is preserved across the reparent.
    pub fn set_parent(&mut self, child_id: u64, parent_id: Option<u64>) {
        if Some(child_id) == parent_id {
            return;
        }

        // Reject cycles (parent must not be a descendant of child).
        if let Some(pid) = parent_id {
            if self.is_descendant(pid, child_id) {
                return;
            }
        }

        // Snapshot the child's current world transform BEFORE reparenting.
        let child_world_before = self.compute_world_transform(child_id);

        // Detach from current parent / root list
        let old_parent = self.entities.get(&child_id).and_then(|e| e.parent);
        match old_parent {
            Some(old) => {
                if let Some(parent) = self.entities.get_mut(&old) {
                    parent.children.retain(|c| *c != child_id);
                }
            }
            None => {
                self.root_entities.retain(|c| *c != child_id);
            }
        }

        // Attach to new parent / root list
        if let Some(new_parent_id) = parent_id {
            if let Some(parent) = self.entities.get_mut(&new_parent_id) {
                parent.children.push(child_id);
            } else {
                // Parent doesn't exist; fall back to root.
                self.root_entities.push(child_id);
            }
        } else {
            self.root_entities.push(child_id);
        }

        if let Some(child) = self.entities.get_mut(&child_id) {
            child.parent = parent_id;
        }

        // Preserve world position: recompute local transform relative to new parent.
        if let Some(new_parent_id) = parent_id {
            let parent_world = self.compute_world_transform(new_parent_id);
            let new_local = compute_local_from_world(&parent_world, &child_world_before);
            if let Some(child) = self.entities.get_mut(&child_id) {
                child.transform = new_local;
            }
        } else {
            // Becoming root: local IS world.
            if let Some(child) = self.entities.get_mut(&child_id) {
                child.transform = child_world_before;
            }
        }

        self.modified = true;
    }

    /// Returns true if `candidate` is `ancestor` itself or a descendant of it.
    fn is_descendant(&self, candidate: u64, ancestor: u64) -> bool {
        if candidate == ancestor {
            return true;
        }
        let mut stack = vec![ancestor];
        while let Some(current) = stack.pop() {
            if let Some(entity) = self.entities.get(&current) {
                for &child in &entity.children {
                    if child == candidate {
                        return true;
                    }
                    stack.push(child);
                }
            }
        }
        false
    }

    /// Collect `root` and all of its descendants (depth-first) into `out`.
    fn collect_subtree(&self, root: u64, out: &mut Vec<u64>) {
        let mut stack = vec![root];
        while let Some(current) = stack.pop() {
            out.push(current);
            if let Some(entity) = self.entities.get(&current) {
                for &child in &entity.children {
                    stack.push(child);
                }
            }
        }
    }

    /// Deep clone an entity and all its descendants. Returns the new root entity ID.
    /// The duplicate is inserted directly after the source under the same parent.
    pub fn duplicate_entity(&mut self, source_id: u64) -> Option<u64> {
        let source = self.entities.get(&source_id)?.clone();

        let new_id = self.next_id;
        self.next_id += 1;

        let mut cloned = source.clone();
        cloned.id = new_id;
        cloned.name = format!("{} (Copy)", source.name);
        cloned.children = Vec::new(); // we'll re-add cloned children below

        // Same parent as source (so it appears next to it)
        let parent = source.parent;
        cloned.parent = parent;

        // Recursively clone children
        let original_children = source.children.clone();
        self.entities.insert(new_id, cloned);

        if parent.is_none() {
            // Insert after source in root_entities
            if let Some(idx) = self.root_entities.iter().position(|&id| id == source_id) {
                self.root_entities.insert(idx + 1, new_id);
            } else {
                self.root_entities.push(new_id);
            }
        } else if let Some(pid) = parent {
            if let Some(parent_entity) = self.entities.get_mut(&pid) {
                if let Some(idx) = parent_entity
                    .children
                    .iter()
                    .position(|&id| id == source_id)
                {
                    parent_entity.children.insert(idx + 1, new_id);
                } else {
                    parent_entity.children.push(new_id);
                }
            }
        }

        // Recursively duplicate children
        let mut new_children = Vec::new();
        for child_id in original_children {
            if let Some(new_child_id) = self.duplicate_entity_with_parent(child_id, new_id) {
                new_children.push(new_child_id);
            }
        }
        if let Some(new_entity) = self.entities.get_mut(&new_id) {
            new_entity.children = new_children;
        }

        self.modified = true;
        Some(new_id)
    }

    /// Internal: deep clone a subtree under a specific (already cloned) parent.
    fn duplicate_entity_with_parent(&mut self, source_id: u64, new_parent: u64) -> Option<u64> {
        let source = self.entities.get(&source_id)?.clone();

        let new_id = self.next_id;
        self.next_id += 1;

        let mut cloned = source.clone();
        cloned.id = new_id;
        cloned.parent = Some(new_parent);
        cloned.children = Vec::new();

        let original_children = source.children.clone();
        self.entities.insert(new_id, cloned);

        let mut new_children = Vec::new();
        for child_id in original_children {
            if let Some(new_child_id) = self.duplicate_entity_with_parent(child_id, new_id) {
                new_children.push(new_child_id);
            }
        }
        if let Some(new_entity) = self.entities.get_mut(&new_id) {
            new_entity.children = new_children;
        }

        Some(new_id)
    }

    // --- Multi-select helpers ---

    /// Returns true if the given entity is in the current selection set.
    pub fn is_selected(&self, id: u64) -> bool {
        self.selected_ids.contains(&id)
    }

    /// Replace the entire selection with a single entity.
    pub fn select_only(&mut self, id: u64) {
        self.selected_ids.clear();
        self.selected_ids.insert(id);
    }

    /// Add an entity to the current selection (Shift+Click).
    pub fn select_add(&mut self, id: u64) {
        self.selected_ids.insert(id);
    }

    /// Toggle an entity in/out of the selection (Ctrl/Cmd+Click).
    pub fn select_toggle(&mut self, id: u64) {
        if self.selected_ids.contains(&id) {
            self.selected_ids.remove(&id);
        } else {
            self.selected_ids.insert(id);
        }
    }

    /// Clear the entire selection.
    pub fn select_clear(&mut self) {
        self.selected_ids.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compose_identity() {
        let parent = TransformData::default();
        let child = TransformData {
            translation: [1.0, 2.0, 3.0],
            ..TransformData::default()
        };
        let world = compose_transforms(&parent, &child);
        assert!(
            (world.translation[0] - 1.0).abs() < 1e-4,
            "x: {}",
            world.translation[0]
        );
        assert!(
            (world.translation[1] - 2.0).abs() < 1e-4,
            "y: {}",
            world.translation[1]
        );
        assert!(
            (world.translation[2] - 3.0).abs() < 1e-4,
            "z: {}",
            world.translation[2]
        );
    }

    #[test]
    fn compute_world_transform_with_parent() {
        let mut scene = SceneModel::new();
        let parent = scene.add_entity("P".into(), vec![]);
        if let Some(p) = scene.entities.get_mut(&parent) {
            p.transform.translation = [10.0, 0.0, 0.0];
        }
        let child = scene.add_entity("C".into(), vec![]);
        // Child starts at world [0,0,0]. After reparenting under P at [10,0,0],
        // set_parent preserves world position, so child.local becomes [-10,0,0].
        scene.set_parent(child, Some(parent));
        let world = scene.compute_world_transform(child);
        assert!(
            world.translation[0].abs() < 1e-3,
            "world x should be ~0, got {}",
            world.translation[0]
        );
    }

    #[test]
    fn lod_level_serialize_deserialize() {
        let level = LodLevel {
            mesh_path: "models/tree_lod0.gltf".into(),
            screen_percentage: 0.75,
        };
        let json = serde_json::to_string(&level).unwrap();
        let parsed: LodLevel = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.mesh_path, "models/tree_lod0.gltf");
        assert!((parsed.screen_percentage - 0.75).abs() < 1e-6);
    }

    #[test]
    fn lod_group_component_roundtrip() {
        let comp = ComponentData::LodGroup {
            levels: vec![
                LodLevel {
                    mesh_path: "high.glb".into(),
                    screen_percentage: 0.5,
                },
                LodLevel {
                    mesh_path: "low.glb".into(),
                    screen_percentage: 0.1,
                },
            ],
        };
        let json = serde_json::to_string(&comp).unwrap();
        let parsed: ComponentData = serde_json::from_str(&json).unwrap();
        if let ComponentData::LodGroup { levels } = parsed {
            assert_eq!(levels.len(), 2);
            assert_eq!(levels[0].mesh_path, "high.glb");
            assert!((levels[1].screen_percentage - 0.1).abs() < 1e-6);
        } else {
            panic!("Expected LodGroup variant");
        }
    }

    #[test]
    fn lod_group_empty_levels_default() {
        let json = r#"{"LodGroup":{}}"#;
        let parsed: ComponentData = serde_json::from_str(json).unwrap();
        if let ComponentData::LodGroup { levels } = parsed {
            assert!(levels.is_empty());
        } else {
            panic!("Expected LodGroup variant");
        }
    }

    #[test]
    fn lod_group_label() {
        let comp = ComponentData::LodGroup { levels: vec![] };
        assert_eq!(comp.label(), "LOD Group");
    }

    #[test]
    fn reparent_preserves_world_position() {
        let mut scene = SceneModel::new();
        let parent = scene.add_entity("P".into(), vec![]);
        if let Some(p) = scene.entities.get_mut(&parent) {
            p.transform.translation = [5.0, 0.0, 0.0];
        }
        let child = scene.add_entity("C".into(), vec![]);
        if let Some(c) = scene.entities.get_mut(&child) {
            c.transform.translation = [3.0, 0.0, 0.0]; // world pos = [3,0,0]
        }

        let world_before = scene.compute_world_transform(child);
        scene.set_parent(child, Some(parent));
        let world_after = scene.compute_world_transform(child);

        assert!(
            (world_before.translation[0] - world_after.translation[0]).abs() < 1e-3,
            "x: before={}, after={}",
            world_before.translation[0],
            world_after.translation[0]
        );
        assert!(
            (world_before.translation[1] - world_after.translation[1]).abs() < 1e-3,
            "y: before={}, after={}",
            world_before.translation[1],
            world_after.translation[1]
        );
        assert!(
            (world_before.translation[2] - world_after.translation[2]).abs() < 1e-3,
            "z: before={}, after={}",
            world_before.translation[2],
            world_after.translation[2]
        );
    }
}
