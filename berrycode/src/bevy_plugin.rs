//! BerryCode Bevy Plugin
//!
//! Integrates the BerryCode editor UI into a Bevy application
//! using bevy_egui for immediate mode UI rendering.

use bevy::prelude::*;
use bevy::winit::{WinitSettings, UpdateMode};
use bevy_egui::EguiPlugin;
use crate::app::BerryCodeApp;
use crate::app::{berry_ui_system, demo_capture_system, setup_egui_fonts_and_style};
use crate::app::preview_3d::{ModelPreviewScene, setup_preview_render_target, manage_preview_scene};
use crate::app::scene_editor::bevy_render::{
    SceneEditorRender, setup_scene_editor_render, update_scene_editor_camera,
};
use crate::app::scene_editor::bevy_sync::sync_scene_to_bevy;
use crate::app::scene_editor::material_preview::{
    MaterialPreviewRender, setup_material_preview, update_material_preview,
};
use std::time::Duration;

/// Plugin that adds the BerryCode editor to a Bevy application.
pub struct BerryCodePlugin;

impl Plugin for BerryCodePlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(EguiPlugin)
            // Reactive rendering optimized for a code editor:
            // - On user input: render immediately at up to 60fps for 1 second
            // - When idle: drop to 2fps (just enough for cursor blink, status updates)
            // This gives snappy interaction while keeping idle CPU near zero.
            .insert_resource(if std::env::var("BERRYCODE_DEMO").is_ok() {
                // Demo mode: render continuously for smooth video capture
                WinitSettings {
                    focused_mode: UpdateMode::reactive_low_power(Duration::from_millis(33)),   // ~30fps
                    unfocused_mode: UpdateMode::reactive_low_power(Duration::from_millis(33)), // ~30fps even unfocused
                }
            } else {
                WinitSettings {
                    focused_mode: UpdateMode::reactive_low_power(Duration::from_millis(16)), // ~60fps while interacting
                    unfocused_mode: UpdateMode::reactive_low_power(Duration::from_secs(1)),  // 1fps when unfocused
                }
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
            .add_systems(Update, update_scene_editor_camera.after(berry_ui_system))
            .add_systems(Update, sync_scene_to_bevy.after(berry_ui_system))
            .add_systems(Update, update_material_preview.after(berry_ui_system));
    }
}
