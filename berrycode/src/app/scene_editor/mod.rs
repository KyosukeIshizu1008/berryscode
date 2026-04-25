//! Unity-like scene editor.
//!
//! Submodules:
//! - [`model`]: in-memory scene state (`SceneModel`, `SceneEntity`, etc.)
//! - [`hierarchy`]: left-side hierarchy tree panel
//! - [`inspector`]: right-side inspector panel
//! - [`scene_view`]: central 3D viewport
//! - [`serialization`]: save scene to a RON-like file

pub(crate) mod animation;
pub(crate) mod animator;
pub(crate) mod animator_editor;
pub(crate) mod asset_deps;
pub(crate) mod asset_import;
pub mod bevy_render;
pub(crate) mod bevy_scene_export;
pub mod bevy_sync;
pub(crate) mod bevy_version;
pub(crate) mod blend_tree_editor;
pub(crate) mod build_settings;
pub(crate) mod code_import;
pub(crate) mod codegen;
pub(crate) mod debug_inspector;
pub(crate) mod dopesheet;
pub(crate) mod event_monitor;
pub(crate) mod gizmo;
pub(crate) mod hierarchy;
pub(crate) mod history;
pub(crate) mod hot_reload;
pub(crate) mod humanoid_avatar;
pub(crate) mod inspector;
pub(crate) mod live_sync;
pub(crate) mod material_preview;
pub(crate) mod model;
pub(crate) mod navmesh;
pub(crate) mod particle_preview;
pub(crate) mod physics_sim;
pub(crate) mod play_mode;
pub(crate) mod plugin_browser;
pub(crate) mod prefab;
pub(crate) mod profiler;
pub(crate) mod query_viz;
pub(crate) mod reflect_codegen;
pub(crate) mod resource_editor;
pub(crate) mod retargeting;
pub(crate) mod scene_merge;
pub(crate) mod scene_tabs;
pub(crate) mod scene_view;
pub(crate) mod script_scan;
pub(crate) mod serialization;
pub(crate) mod shader_graph;
pub(crate) mod shader_graph_editor;
pub(crate) mod skeleton;
pub(crate) mod spline;
pub(crate) mod state_editor;
pub(crate) mod state_editor_ui;
pub(crate) mod system_graph;
pub(crate) mod terrain;
pub(crate) mod thumbnail_cache;
pub(crate) mod visual_script;
pub(crate) mod visual_script_editor;
