//! GPU-accelerated 3D model preview using Bevy's renderer
//! Renders GLB/GLTF models to an off-screen texture displayed in egui

use bevy::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy::image::{TextureFormatPixelInfo as _, ImageSampler}; use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use bevy::camera::visibility::RenderLayers;

/// Resource tracking the 3D preview state
#[derive(Resource, Default)]
pub struct ModelPreviewScene {
    /// Path of the model currently loaded (None = no model)
    pub loaded_model_path: Option<String>,
    /// Path of the model requested to load
    pub requested_model_path: Option<String>,
    /// Handle to the render target image
    pub render_target: Option<Handle<Image>>,
    /// egui texture ID for the rendered image (cached each frame)
    pub egui_texture_id: Option<egui::TextureId>,
    /// Preview image dimensions
    pub preview_width: u32,
    pub preview_height: u32,
    /// Camera orbit angles
    pub orbit_yaw: f32,
    pub orbit_pitch: f32,
    pub orbit_distance: f32,
}

/// Component to mark preview scene entities for cleanup
#[derive(Component)]
pub struct PreviewSceneEntity;

/// Component for the preview camera
#[derive(Component)]
pub struct PreviewCamera;

/// System: Create the render target image on startup
pub fn setup_preview_render_target(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut preview: ResMut<ModelPreviewScene>,
) {
    let width = 512;
    let height = 512;

    let size = Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };

    let mut image = Image::new_fill(
        size,
        TextureDimension::D2,
        &[25, 26, 28, 255], // dark background matching editor
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;

    let image_handle = images.add(image);

    // Spawn preview camera targeting the render texture
    // Matches Scene Editor pattern (bevy_render.rs) which is known to work.
    commands.spawn((
        Camera3d::default(),
        Camera {
            target: bevy::camera::RenderTarget::Image(image_handle.clone().into()),
            clear_color: ClearColorConfig::Custom(Color::srgba(0.098, 0.102, 0.11, 1.0)),
            order: -2,
            ..default()
        },
        Projection::Perspective(PerspectiveProjection {
            fov: std::f32::consts::FRAC_PI_4,
            aspect_ratio: width as f32 / height as f32,
            near: 0.1,
            far: 1000.0,
        }),
        Transform::from_xyz(0.0, 50.0, 150.0).looking_at(Vec3::ZERO, Vec3::Y),
        RenderLayers::layer(1),
        PreviewCamera,
    ));

    // Add a directional light for the preview scene (no PreviewSceneEntity — must persist)
    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.5, 0.5, 0.0)),
        RenderLayers::layer(1),
    ));

    // Ambient light
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 300.0,
        affects_lightmapped_meshes: false,
    });

    preview.render_target = Some(image_handle);
    preview.preview_width = width;
    preview.preview_height = height;
    preview.orbit_yaw = std::f32::consts::FRAC_PI_4;
    preview.orbit_pitch = 0.3;
    preview.orbit_distance = 150.0;
}

/// System: Load/unload models based on requested_model_path
pub fn manage_preview_scene(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut preview: ResMut<ModelPreviewScene>,
    preview_entities: Query<Entity, (With<PreviewSceneEntity>, Without<PreviewCamera>)>,
    mut camera_query: Query<&mut Transform, With<PreviewCamera>>,
) {
    // Check if we need to load a new model
    let needs_load = match (&preview.requested_model_path, &preview.loaded_model_path) {
        (Some(requested), Some(loaded)) => requested != loaded,
        (Some(_), None) => true,
        (None, Some(_)) => true, // unload
        (None, None) => false,
    };

    if !needs_load {
        // Update camera orbit position each frame
        if let Ok(mut camera_transform) = camera_query.single_mut() {
            let yaw = preview.orbit_yaw;
            let pitch = preview.orbit_pitch;
            let distance = preview.orbit_distance;

            let x = distance * yaw.cos() * pitch.cos();
            let y = distance * pitch.sin();
            let z = distance * yaw.sin() * pitch.cos();

            *camera_transform = Transform::from_xyz(x, y, z).looking_at(Vec3::ZERO, Vec3::Y);
        }
        return;
    }

    // Despawn old model entities (not camera or light -- those are excluded by the query filter)
    for entity in preview_entities.iter() {
        commands.entity(entity).despawn();
    }

    if let Some(path) = preview.requested_model_path.clone() {
        // Bevy's AssetServer uses a fixed assets/ folder determined at startup.
        // We find it by checking the Cargo manifest directory (set during `cargo run`).
        let file_path = std::path::Path::new(&path);
        let file_name = file_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // Bevy uses CARGO_MANIFEST_DIR/assets/ during `cargo run`
        let bevy_assets_dir = option_env!("CARGO_MANIFEST_DIR")
            .map(|d| std::path::PathBuf::from(d).join("assets"))
            .unwrap_or_else(|| {
                std::env::current_exe()
                    .ok()
                    .and_then(|p| p.parent().map(|p| p.join("assets")))
                    .unwrap_or_else(|| std::path::PathBuf::from("assets"))
            });

        let preview_dir = bevy_assets_dir.join("_preview");
        let _ = std::fs::create_dir_all(&preview_dir);
        let dest = preview_dir.join(&file_name);

        // Copy file if needed
        let src_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let dst_size = std::fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
        if src_size != dst_size {
            let _ = std::fs::copy(&path, &dest);
        }

        let asset_path = format!("_preview/{}", file_name);
        tracing::info!(
            "3D Preview: loading '{}' from {:?}",
            asset_path,
            bevy_assets_dir
        );
        let label = bevy::gltf::GltfAssetLabel::Scene(0).from_asset(asset_path);
        let scene_handle: Handle<Scene> = asset_server.load(label);

        commands.spawn((
            SceneRoot(scene_handle),
            RenderLayers::layer(1),
            PreviewSceneEntity,
        ));

        tracing::info!("3D Preview: Loading model {}", path);
        preview.loaded_model_path = Some(path);
    } else {
        preview.loaded_model_path = None;
        tracing::info!("3D Preview: Unloaded model");
    }

    preview.requested_model_path = preview.loaded_model_path.clone();
}

/// Propagate RenderLayers::layer(1) to newly added children of PreviewSceneEntity.
/// GLTF scenes spawn children asynchronously, so we detect them via `Added<ChildOf>`.
pub fn propagate_preview_render_layers(
    new_children: Query<(Entity, &ChildOf), Added<ChildOf>>,
    preview_markers: Query<&PreviewSceneEntity>,
    ancestors: Query<&ChildOf>,
    mut commands: Commands,
) {
    let target = RenderLayers::layer(1);
    for (entity, _parent) in new_children.iter() {
        // Walk up the ancestor chain to see if this entity is under a PreviewSceneEntity
        let mut current = entity;
        let mut is_preview = false;
        for _ in 0..50 {
            // depth limit
            if preview_markers.get(current).is_ok() {
                is_preview = true;
                break;
            }
            match ancestors.get(current) {
                Ok(p) => current = p.parent(),
                Err(_) => break,
            }
        }
        if is_preview {
            commands.entity(entity).insert(target.clone());
        }
    }
}
