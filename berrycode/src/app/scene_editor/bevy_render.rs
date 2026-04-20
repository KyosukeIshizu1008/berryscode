//! Bevy render-to-texture system for the scene editor.
//!
//! Creates a dedicated off-screen camera on `RenderLayers::layer(2)` whose output
//! is displayed in the Scene View panel. The companion module
//! [`super::bevy_sync`] mirrors the `SceneModel` into real Bevy entities on that
//! render layer.

use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use bevy::render::view::RenderLayers;
use std::collections::HashMap;

/// Resource holding the scene editor's render state.
#[derive(Resource)]
pub struct SceneEditorRender {
    /// Handle to the image we draw the scene into.
    pub render_target: Option<Handle<Image>>,
    /// Cached egui texture id (assigned each frame in `berry_ui_system`).
    pub egui_texture_id: Option<egui::TextureId>,
    pub width: u32,
    pub height: u32,
    /// Camera orbit angles (radians).
    pub orbit_yaw: f32,
    pub orbit_pitch: f32,
    pub orbit_distance: f32,
    /// Look-at target for the orbit camera (world-space).
    pub orbit_target: [f32; 3],
    /// When true, use an orthographic projection instead of perspective.
    pub ortho: bool,
    /// Orthographic half-height in world units.
    pub ortho_scale: f32,
    /// Tracks which `SceneModel` entity id maps to which Bevy `Entity`
    /// (so the sync system can update or despawn them).
    pub spawned_entities: HashMap<u64, Entity>,
    /// Whether the directional light casts shadows.
    pub shadows_enabled: bool,
    /// Whether bloom post-processing is enabled.
    pub bloom_enabled: bool,
    /// Bloom intensity (0.0..=1.0).
    pub bloom_intensity: f32,
    /// Tonemapping mode: 0=None, 1=Reinhard, 2=ReinhardLuminance, 3=AcesFitted, 4=AgX.
    pub tonemapping: u8,
    /// Hash of the last synced `SceneModel` structure. Used as a cheap
    /// change-detection signal so we only rebuild the Bevy-side scene when the
    /// topology actually changed.
    pub last_sync_hash: u64,
    /// Path to the HDR/EXR skybox image, if a Skybox component exists in the
    /// scene. When `Some`, the editor camera uses a sky-tinted clear color;
    /// when `None`, it falls back to the default dark editor background.
    pub skybox_path: Option<String>,
    /// Cached handle for the loaded skybox image (loaded via AssetServer).
    pub skybox_handle: Option<Handle<Image>>,
    /// The skybox path that was last loaded into `skybox_handle`, used to
    /// detect when the path has changed and a reload is needed.
    pub skybox_path_loaded: Option<String>,
    /// Whether SSAO (Screen Space Ambient Occlusion) is enabled.
    pub ssao_enabled: bool,
    /// Whether TAA (Temporal Anti-Aliasing) is enabled.
    pub taa_enabled: bool,
    /// Whether distance fog is enabled.
    pub fog_enabled: bool,
    /// Fog color (linear RGB).
    pub fog_color: [f32; 3],
    /// Fog linear start distance.
    pub fog_start: f32,
    /// Fog linear end distance.
    pub fog_end: f32,
    /// Whether Depth of Field is enabled.
    pub dof_enabled: bool,
    /// DoF focus distance in world units.
    pub dof_focus_distance: f32,
    /// DoF aperture f-stop value.
    pub dof_aperture: f32,
}

impl Default for SceneEditorRender {
    fn default() -> Self {
        Self {
            render_target: None,
            egui_texture_id: None,
            width: 0,
            height: 0,
            orbit_yaw: 0.0,
            orbit_pitch: 0.0,
            orbit_distance: 0.0,
            orbit_target: [0.0; 3],
            ortho: false,
            ortho_scale: 0.0,
            spawned_entities: HashMap::new(),
            shadows_enabled: true,
            bloom_enabled: false,
            bloom_intensity: 0.3,
            tonemapping: 3,
            last_sync_hash: 0,
            skybox_path: None,
            skybox_handle: None,
            skybox_path_loaded: None,
            ssao_enabled: false,
            taa_enabled: false,
            fog_enabled: false,
            fog_color: [0.7, 0.8, 1.0],
            fog_start: 50.0,
            fog_end: 200.0,
            dof_enabled: false,
            dof_focus_distance: 10.0,
            dof_aperture: 0.02,
        }
    }
}

/// Marker on every Bevy entity spawned for the scene editor viewport.
#[derive(Component)]
pub struct SceneEditorObject;

/// Marker on the scene editor's off-screen camera.
#[derive(Component)]
pub struct SceneEditorCamera;

/// Marker on the scene editor's permanent directional light.
#[derive(Component)]
pub struct SceneEditorLight;

/// Component tagging a Bevy entity with the corresponding `SceneModel`
/// entity id (used for picking in later phases).
#[derive(Component)]
pub struct SceneObjectId(pub u64);

/// Startup: create the render target image, the editor camera, and a baseline
/// directional light.
pub fn setup_scene_editor_render(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut state: ResMut<SceneEditorRender>,
) {
    let width = 1024;
    let height = 768;
    let size = Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };

    let mut image = Image::new_fill(
        size,
        TextureDimension::D2,
        &[35, 38, 45, 255], // dark blue-gray scene editor background
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;

    let image_handle = images.add(image);

    // Camera on RenderLayers::layer(2) for the scene editor.
    // We pin FOV to 45° (PI/4) so the picking / gizmo math in `gizmo.rs` matches
    // exactly what the GPU renders.
    commands.spawn((
        Camera3d::default(),
        Camera {
            target: bevy::render::camera::RenderTarget::Image(image_handle.clone()),
            clear_color: ClearColorConfig::Custom(Color::srgba(0.137, 0.149, 0.176, 1.0)),
            order: -2,
            ..default()
        },
        Projection::Perspective(PerspectiveProjection {
            fov: std::f32::consts::FRAC_PI_4, // 45 degrees vertical FOV
            aspect_ratio: width as f32 / height as f32,
            near: 0.1,
            far: 1000.0,
        }),
        Transform::from_xyz(5.0, 5.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        RenderLayers::layer(2),
        SceneEditorCamera,
    ));

    // Editor light (always present so the scene isn't pitch black when empty).
    commands.spawn((
        DirectionalLight {
            illuminance: 8000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.7, 0.5, 0.0)),
        RenderLayers::layer(2),
        SceneEditorLight,
    ));

    state.render_target = Some(image_handle);
    state.width = width;
    state.height = height;
    state.orbit_yaw = std::f32::consts::FRAC_PI_4;
    state.orbit_pitch = 0.5;
    state.orbit_distance = 8.0;
}

/// Update the camera transform and projection each frame from the orbit
/// parameters pushed in by the UI layer.
pub fn update_scene_editor_camera(
    mut commands: Commands,
    mut state: ResMut<SceneEditorRender>,
    asset_server: Res<AssetServer>,
    mut q: Query<(&mut Transform, &mut Projection), With<SceneEditorCamera>>,
    mut light_q: Query<&mut DirectionalLight, With<SceneEditorLight>>,
    camera_entity_q: Query<Entity, With<SceneEditorCamera>>,
) {
    // Keep the editor light's shadow flag in sync with the resource.
    if let Ok(mut light) = light_q.get_single_mut() {
        light.shadows_enabled = state.shadows_enabled;
    }

    if let Ok((mut t, mut proj)) = q.get_single_mut() {
        let yaw = state.orbit_yaw;
        let pitch = state.orbit_pitch;
        let d = state.orbit_distance;
        let target = Vec3::from_slice(&state.orbit_target);
        let x = d * yaw.cos() * pitch.cos();
        let y = d * pitch.sin();
        let z = d * yaw.sin() * pitch.cos();
        *t = Transform::from_xyz(target.x + x, target.y + y, target.z + z)
            .looking_at(target, Vec3::Y);

        // Swap projection based on ortho flag.
        if state.ortho {
            *proj = Projection::Orthographic(OrthographicProjection {
                scaling_mode: bevy::render::camera::ScalingMode::FixedVertical {
                    viewport_height: state.ortho_scale * 2.0,
                },
                ..OrthographicProjection::default_3d()
            });
        } else {
            *proj = Projection::Perspective(PerspectiveProjection {
                fov: std::f32::consts::FRAC_PI_4,
                aspect_ratio: state.width as f32 / state.height.max(1) as f32,
                near: 0.1,
                far: 1000.0,
            });
        }
    }

    // Update clear color and ambient light based on skybox presence.
    // When a skybox HDR path is set, attempt to load it via AssetServer and
    // insert a Skybox component on the camera. Also boost ambient light to
    // approximate image-based lighting (IBL).
    if let Ok(cam_entity) = camera_entity_q.get_single() {
        let skybox_path_cloned = state.skybox_path.clone();
        if let Some(ref path) = skybox_path_cloned {
            // Load skybox image if path changed or not yet loaded.
            let needs_load = match state.skybox_path_loaded.as_deref() {
                Some(loaded) => loaded != path.as_str(),
                None => true,
            };
            if needs_load {
                let asset_path = if path.starts_with('/') || path.contains(":\\") {
                    format!("file://{}", path)
                } else {
                    path.clone()
                };
                let handle: Handle<Image> = asset_server.load(&asset_path);
                state.skybox_handle = Some(handle);
                state.skybox_path_loaded = Some(path.clone());
            }
            // Sky-tinted background and elevated ambient light for IBL feel.
            let clear_color = ClearColorConfig::Custom(Color::srgb(0.05, 0.1, 0.2));
            commands.entity(cam_entity).insert(Camera {
                target: state
                    .render_target
                    .as_ref()
                    .map(|h| bevy::render::camera::RenderTarget::Image(h.clone()))
                    .unwrap_or(bevy::render::camera::RenderTarget::default()),
                clear_color,
                order: -2,
                ..default()
            });
            commands.insert_resource(AmbientLight {
                color: Color::WHITE,
                brightness: 500.0,
            });
        } else {
            // No skybox: clean up cached state and revert to default background.
            if state.skybox_handle.is_some() {
                state.skybox_handle = None;
                state.skybox_path_loaded = None;
            }
            let clear_color = ClearColorConfig::Custom(Color::srgba(0.098, 0.102, 0.11, 1.0));
            commands.entity(cam_entity).insert(Camera {
                target: state
                    .render_target
                    .as_ref()
                    .map(|h| bevy::render::camera::RenderTarget::Image(h.clone()))
                    .unwrap_or(bevy::render::camera::RenderTarget::default()),
                clear_color,
                order: -2,
                ..default()
            });
            commands.insert_resource(AmbientLight {
                color: Color::WHITE,
                brightness: 300.0,
            });
        }
    }

    // Apply bloom and tonemapping post-processing to the scene editor camera.
    if let Ok(cam_entity) = camera_entity_q.get_single() {
        if state.bloom_enabled {
            commands
                .entity(cam_entity)
                .insert(bevy::core_pipeline::bloom::Bloom {
                    intensity: state.bloom_intensity,
                    ..default()
                });
        } else {
            commands
                .entity(cam_entity)
                .remove::<bevy::core_pipeline::bloom::Bloom>();
        }

        let tm = match state.tonemapping {
            0 => bevy::core_pipeline::tonemapping::Tonemapping::None,
            1 => bevy::core_pipeline::tonemapping::Tonemapping::Reinhard,
            2 => bevy::core_pipeline::tonemapping::Tonemapping::ReinhardLuminance,
            3 => bevy::core_pipeline::tonemapping::Tonemapping::AcesFitted,
            4 => bevy::core_pipeline::tonemapping::Tonemapping::AgX,
            _ => bevy::core_pipeline::tonemapping::Tonemapping::AcesFitted,
        };
        commands.entity(cam_entity).insert(tm);

        // --- SSAO ---
        if state.ssao_enabled {
            commands
                .entity(cam_entity)
                .insert(bevy::pbr::ScreenSpaceAmbientOcclusion::default());
        } else {
            commands
                .entity(cam_entity)
                .remove::<bevy::pbr::ScreenSpaceAmbientOcclusion>();
        }

        // --- TAA (Temporal Anti-Aliasing) ---
        {
            use bevy::core_pipeline::experimental::taa::TemporalAntiAliasing;
            if state.taa_enabled {
                commands
                    .entity(cam_entity)
                    .insert(TemporalAntiAliasing::default());
            } else {
                commands.entity(cam_entity).remove::<TemporalAntiAliasing>();
            }
        }

        // --- Fog ---
        if state.fog_enabled {
            commands.entity(cam_entity).insert(bevy::pbr::DistanceFog {
                color: Color::srgb(state.fog_color[0], state.fog_color[1], state.fog_color[2]),
                falloff: bevy::pbr::FogFalloff::Linear {
                    start: state.fog_start,
                    end: state.fog_end,
                },
                ..default()
            });
        } else {
            commands
                .entity(cam_entity)
                .remove::<bevy::pbr::DistanceFog>();
        }

        // --- Depth of Field ---
        {
            use bevy::core_pipeline::dof::{DepthOfField, DepthOfFieldMode};
            if state.dof_enabled {
                commands.entity(cam_entity).insert(DepthOfField {
                    focal_distance: state.dof_focus_distance,
                    aperture_f_stops: state.dof_aperture,
                    mode: DepthOfFieldMode::Bokeh,
                    ..default()
                });
            } else {
                commands.entity(cam_entity).remove::<DepthOfField>();
            }
        }
    }
}
