//! Unity-like scene editor.
//!
//! Submodules:
//! - [`model`]: in-memory scene state (`SceneModel`, `SceneEntity`, etc.)
//! - [`hierarchy`]: left-side hierarchy tree panel
//! - [`inspector`]: right-side inspector panel
//! - [`scene_view`]: central 3D viewport
//! - [`serialization`]: save scene to a RON-like file

pub(crate) mod model;
pub(crate) mod hierarchy;
pub(crate) mod history;
pub(crate) mod inspector;
pub(crate) mod scene_view;
pub(crate) mod serialization;
pub(crate) mod gizmo;
pub(crate) mod particle_preview;
pub(crate) mod prefab;
pub(crate) mod profiler;
pub(crate) mod animation;
pub(crate) mod dopesheet;
pub(crate) mod script_scan;
pub(crate) mod material_preview;
pub mod bevy_render;
pub mod bevy_sync;
pub(crate) mod asset_import;
pub(crate) mod thumbnail_cache;
pub(crate) mod scene_tabs;
pub(crate) mod asset_deps;
pub(crate) mod animator;
pub(crate) mod animator_editor;
pub(crate) mod play_mode;
pub(crate) mod physics_sim;
pub(crate) mod debug_inspector;
pub(crate) mod build_settings;
pub(crate) mod spline;
pub(crate) mod terrain;
pub(crate) mod scene_merge;
pub(crate) mod skeleton;
pub(crate) mod visual_script;
pub(crate) mod shader_graph;
pub(crate) mod navmesh;
pub(crate) mod visual_script_editor;
pub(crate) mod shader_graph_editor;
pub(crate) mod hot_reload;
