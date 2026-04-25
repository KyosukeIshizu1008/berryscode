//! BerryCode Bevy Plugin
//!
//! Integrates the BerryCode editor UI into a Bevy application
//! using bevy_egui for immediate mode UI rendering.

use crate::app::preview_3d::{
    manage_preview_scene, propagate_preview_render_layers, setup_preview_render_target,
    ModelPreviewScene,
};
use crate::app::scene_editor::bevy_render::{
    setup_scene_editor_render, update_scene_editor_camera, SceneEditorRender,
};
use crate::app::scene_editor::bevy_sync::sync_scene_to_bevy;
use crate::app::scene_editor::material_preview::{
    setup_material_preview, update_material_preview, MaterialPreviewRender,
};
use crate::app::BerryCodeApp;
use crate::app::{berry_ui_system, demo_capture_system, setup_egui_fonts_and_style};
use bevy::prelude::*;
use bevy::winit::{UpdateMode, WinitSettings};
use bevy_egui::EguiPlugin;
use std::time::Duration;

/// Plugin that adds the BerryCode editor to a Bevy application.
pub struct BerryCodePlugin;

impl Plugin for BerryCodePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin::default())
            .insert_resource(WinitSettings {
                focused_mode: UpdateMode::Continuous,
                unfocused_mode: UpdateMode::reactive_low_power(Duration::from_millis(100)),
            })
            .insert_non_send_resource(BerryCodeApp::new())
            .init_resource::<ModelPreviewScene>()
            .init_resource::<SceneEditorRender>()
            .init_resource::<MaterialPreviewRender>()
            .add_systems(
                Startup,
                (
                    setup_egui_fonts_and_style,
                    setup_preview_render_target,
                    setup_scene_editor_render,
                    setup_material_preview,
                ),
            )
            .add_systems(Update, berry_ui_system)
            .add_systems(Update, demo_capture_system.after(berry_ui_system))
            .add_systems(Update, manage_preview_scene.after(berry_ui_system))
            .add_systems(
                Update,
                propagate_preview_render_layers.after(manage_preview_scene),
            )
            .add_systems(Update, update_scene_editor_camera.after(berry_ui_system))
            .add_systems(Update, sync_scene_to_bevy.after(berry_ui_system))
            .add_systems(Update, update_material_preview.after(berry_ui_system));
    }
}
