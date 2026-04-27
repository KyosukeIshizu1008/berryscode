//! BerryCode Bevy Plugin
//!
//! Integrates the BerryCode editor UI into a Bevy application
//! using bevy_egui for immediate mode UI rendering.

use crate::app::preview_3d::{
    manage_preview_scene, propagate_preview_render_layers, setup_preview_render_target,
    ModelPreviewScene,
};
use crate::app::scene_editor::bevy_render::{
    propagate_scene_editor_render_layers, setup_scene_editor_render, update_scene_editor_camera,
    SceneEditorRender,
};
use crate::app::scene_editor::bevy_sync::sync_scene_to_bevy;
use crate::app::scene_editor::material_preview::{
    setup_material_preview, update_material_preview, MaterialPreviewRender,
};
use crate::app::BerryCodeApp;
use crate::app::{berry_ui_system, demo_capture_system, setup_egui_fonts_and_style};
use bevy::prelude::*;
use bevy::winit::{UpdateMode, WinitSettings};
use bevy_egui::{EguiGlobalSettings, EguiPlugin, PrimaryEguiContext};
use std::time::Duration;

/// Spawn a `Camera2d` for the primary window, marked as the host of the
/// primary egui context. Without this, `bevy_egui`'s auto-create logic
/// attaches the primary context to the *first* camera it sees — which in
/// our app is the offscreen scene-editor `Camera3d` — and the main
/// window receives no draws (entirely black window).
///
/// `RenderLayers::none()` keeps this camera UI-only (it doesn't pull in
/// any of the offscreen scene/preview meshes), and `order: 100` makes
/// it draw on top of everything else. We clear to the same dark UI
/// color as the panel background so any unpainted splitter pixels don't
/// appear as black bands.
fn setup_primary_camera(mut commands: Commands) {
    commands.spawn((
        PrimaryEguiContext,
        Camera2d,
        bevy::camera::visibility::RenderLayers::none(),
        Camera {
            order: 100,
            clear_color: ClearColorConfig::Custom(Color::srgb(
                25.0 / 255.0,
                26.0 / 255.0,
                28.0 / 255.0,
            )),
            ..default()
        },
    ));
}

fn disable_auto_primary_context(mut settings: ResMut<EguiGlobalSettings>) {
    // We attach the `PrimaryEguiContext` marker ourselves on a dedicated
    // window-targeted Camera2d (see `setup_primary_camera`), so let
    // bevy_egui know not to pick a camera at random.
    settings.auto_create_primary_context = false;
}

/// Plugin that adds the BerryCode editor to a Bevy application.
pub struct BerryCodePlugin;

impl Plugin for BerryCodePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin::default())
            // Match Bevy's frame clear to the editor / sidebar fill (#191A1C).
            // The egui camera uses `ClearColorConfig::None`, so any pixel
            // that egui doesn't paint shows whatever the previous render
            // pipeline wrote — by default Bevy clears to pure black, which
            // appeared as a thin dark seam on the boundary between
            // `SidePanel`s. Matching the clear color to the panel fill
            // hides any subpixel/rounding gap.
            .insert_resource(ClearColor(Color::srgb(
                25.0 / 255.0,
                26.0 / 255.0,
                28.0 / 255.0,
            )))
            .insert_resource(WinitSettings {
                focused_mode: UpdateMode::Continuous,
                unfocused_mode: UpdateMode::reactive_low_power(Duration::from_millis(100)),
            })
            .insert_non_send_resource(BerryCodeApp::new())
            .init_resource::<ModelPreviewScene>()
            .init_resource::<SceneEditorRender>()
            .init_resource::<MaterialPreviewRender>()
            .init_resource::<crate::app::EguiFontsConfigured>()
            .add_systems(
                PreStartup,
                (
                    disable_auto_primary_context,
                    setup_primary_camera.before(bevy_egui::EguiStartupSet::InitContexts),
                ),
            )
            .add_systems(
                Startup,
                (
                    setup_preview_render_target,
                    setup_scene_editor_render,
                    setup_material_preview,
                ),
            )
            .add_systems(
                bevy_egui::EguiPrimaryContextPass,
                (
                    setup_egui_fonts_and_style,
                    berry_ui_system.after(setup_egui_fonts_and_style),
                ),
            )
            .add_systems(Update, demo_capture_system)
            .add_systems(Update, manage_preview_scene)
            .add_systems(
                Update,
                propagate_preview_render_layers.after(manage_preview_scene),
            )
            .add_systems(Update, update_scene_editor_camera)
            .add_systems(Update, sync_scene_to_bevy)
            .add_systems(
                Update,
                propagate_scene_editor_render_layers.after(sync_scene_to_bevy),
            )
            .add_systems(Update, update_material_preview);
    }
}
