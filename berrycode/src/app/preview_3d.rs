//! GPU-accelerated 3D model preview using Bevy's renderer
//! Renders GLB/GLTF models to an off-screen texture displayed in egui

use bevy::animation::graph::{AnimationGraph, AnimationGraphHandle, AnimationNodeIndex};
use bevy::animation::transition::AnimationTransitions;
use bevy::animation::{AnimationClip, AnimationPlayer};
use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::gltf::Gltf;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use std::time::Duration;

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

    /// Handle to the loaded GLTF asset (used to enumerate animations).
    pub gltf_handle: Option<Handle<Gltf>>,
    /// Built animation graph for the current model.
    pub animation_graph: Option<Handle<AnimationGraph>>,
    /// Available clips for the current model (name → graph node).
    /// Sorted alphabetically so the UI order is deterministic.
    pub animation_clips: Vec<(String, AnimationNodeIndex)>,
    /// Clip the UI is asking the player to switch to (name).
    pub requested_clip: Option<String>,
    /// Clip currently driving the AnimationPlayer (name).
    pub current_clip: Option<String>,
    /// Entity that holds the AnimationPlayer for the current model.
    pub animation_player_entity: Option<Entity>,
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

    // Spawn preview camera targeting the render texture.
    // `order: -1` is intentionally distinct from the scene editor camera
    // (`order: -2` in `bevy_render.rs`) and the material preview camera
    // (`order: -3`). When two off-screen cameras share an order Bevy's
    // render order is undefined, which manifested as the preview render
    // target staying black (#1).
    commands.spawn((
        Camera3d::default(),
        Camera {
            clear_color: ClearColorConfig::Custom(Color::srgba(0.098, 0.102, 0.11, 1.0)),
            order: -1,
            ..default()
        },
        bevy::camera::RenderTarget::Image(image_handle.clone().into()),
        Projection::Perspective(PerspectiveProjection {
            fov: std::f32::consts::FRAC_PI_4,
            aspect_ratio: width as f32 / height as f32,
            near: 0.1,
            far: 1000.0,
            ..default()
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

    // Ambient light, scoped to layer 1 so it lifts the preview scene without
    // bleeding into the scene editor or material preview viewports.
    commands.spawn((
        AmbientLight {
            color: Color::WHITE,
            brightness: 600.0,
            affects_lightmapped_meshes: false,
        },
        RenderLayers::layer(1),
    ));

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

    // Reset animation state — the new (or empty) model will rebuild it.
    preview.gltf_handle = None;
    preview.animation_graph = None;
    preview.animation_clips.clear();
    preview.requested_clip = None;
    preview.current_clip = None;
    preview.animation_player_entity = None;

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
        let label = bevy::gltf::GltfAssetLabel::Scene(0).from_asset(asset_path.clone());
        let scene_handle: Handle<Scene> = asset_server.load(label);
        // Also load the GLTF root so we can enumerate named animation clips.
        let gltf_handle: Handle<Gltf> = asset_server.load(asset_path);

        commands.spawn((
            SceneRoot(scene_handle),
            RenderLayers::layer(1),
            PreviewSceneEntity,
        ));

        tracing::info!("3D Preview: Loading model {}", path);
        preview.loaded_model_path = Some(path);
        preview.gltf_handle = Some(gltf_handle);
    } else {
        preview.loaded_model_path = None;
        tracing::info!("3D Preview: Unloaded model");
    }

    preview.requested_model_path = preview.loaded_model_path.clone();
}

/// Build an `AnimationGraph` once the GLTF asset has finished loading.
/// Stores the graph + (clip name, node index) list on `ModelPreviewScene`
/// so the UI can offer them as a dropdown.
pub fn setup_preview_animation_graph(
    mut preview: ResMut<ModelPreviewScene>,
    gltfs: Res<Assets<Gltf>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    if preview.animation_graph.is_some() {
        return;
    }
    let Some(gltf_h) = preview.gltf_handle.clone() else {
        return;
    };
    let Some(gltf) = gltfs.get(&gltf_h) else {
        return;
    };
    if gltf.named_animations.is_empty() {
        return;
    }

    let mut named: Vec<(String, Handle<AnimationClip>)> = gltf
        .named_animations
        .iter()
        .map(|(n, h)| (n.to_string(), h.clone()))
        .collect();
    named.sort_by(|a, b| a.0.cmp(&b.0));

    let clips: Vec<Handle<AnimationClip>> = named.iter().map(|(_, h)| h.clone()).collect();
    let (graph, indices) = AnimationGraph::from_clips(clips);
    let graph_handle = graphs.add(graph);

    preview.animation_clips = named
        .into_iter()
        .zip(indices)
        .map(|((n, _), i)| (n, i))
        .collect();
    if preview.requested_clip.is_none() {
        if let Some((first, _)) = preview.animation_clips.first() {
            preview.requested_clip = Some(first.clone());
        }
    }
    tracing::info!(
        "3D Preview: built AnimationGraph with {} clips: {:?}",
        preview.animation_clips.len(),
        preview
            .animation_clips
            .iter()
            .map(|(n, _)| n.as_str())
            .collect::<Vec<_>>()
    );
    preview.animation_graph = Some(graph_handle);
}

/// When Bevy spawns a GLTF scene that contains animations it inserts an
/// `AnimationPlayer` on the scene root. We catch that the frame it appears,
/// attach our `AnimationGraphHandle` + `AnimationTransitions`, and start the
/// requested (or first) clip on a loop.
pub fn attach_preview_animation_player(
    mut commands: Commands,
    mut preview: ResMut<ModelPreviewScene>,
    mut players: Query<(Entity, &mut AnimationPlayer), Added<AnimationPlayer>>,
    preview_markers: Query<&PreviewSceneEntity>,
    ancestors: Query<&ChildOf>,
) {
    let Some(graph_handle) = preview.animation_graph.clone() else {
        return;
    };
    if preview.animation_clips.is_empty() {
        return;
    }

    for (entity, mut player) in &mut players {
        // Walk up the ancestor chain to confirm this player belongs to the preview scene.
        let mut current = entity;
        let mut is_preview = false;
        for _ in 0..50 {
            if preview_markers.get(current).is_ok() {
                is_preview = true;
                break;
            }
            match ancestors.get(current) {
                Ok(p) => current = p.parent(),
                Err(_) => break,
            }
        }
        if !is_preview {
            continue;
        }

        let target_name = preview
            .requested_clip
            .clone()
            .or_else(|| preview.animation_clips.first().map(|(n, _)| n.clone()));
        let Some(target_name) = target_name else {
            continue;
        };
        let Some(node) = preview
            .animation_clips
            .iter()
            .find(|(n, _)| n == &target_name)
            .map(|(_, i)| *i)
        else {
            continue;
        };

        let mut transitions = AnimationTransitions::new();
        transitions.play(&mut player, node, Duration::ZERO).repeat();
        commands
            .entity(entity)
            .insert((AnimationGraphHandle(graph_handle.clone()), transitions));
        preview.animation_player_entity = Some(entity);
        preview.current_clip = Some(target_name);
    }
}

/// Apply UI-driven clip changes by transitioning the `AnimationPlayer` to the
/// requested clip with a short cross-fade.
pub fn apply_preview_clip_change(
    mut preview: ResMut<ModelPreviewScene>,
    mut players: Query<(&mut AnimationPlayer, &mut AnimationTransitions)>,
) {
    let Some(entity) = preview.animation_player_entity else {
        return;
    };
    let Some(requested) = preview.requested_clip.clone() else {
        return;
    };
    if preview.current_clip.as_ref() == Some(&requested) {
        return;
    }
    let Some(node) = preview
        .animation_clips
        .iter()
        .find(|(n, _)| n == &requested)
        .map(|(_, i)| *i)
    else {
        return;
    };
    let Ok((mut player, mut transitions)) = players.get_mut(entity) else {
        return;
    };
    transitions
        .play(&mut player, node, Duration::from_millis(250))
        .repeat();
    preview.current_clip = Some(requested);
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
