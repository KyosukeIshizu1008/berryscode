//! Sync the editor `SceneModel` to real Bevy ECS entities.
//!
//! The `SceneModel` is the editor-side source of truth. This system mirrors it
//! into Bevy entities living on `RenderLayers::layer(2)` so that the scene
//! editor camera (see [`super::bevy_render`]) can render them off-screen.
//!
//! Strategy (v1, intentionally simple):
//! - Hash the model each frame; if the hash is unchanged, only refresh
//!   transforms (cheap path that handles inspector edits).
//! - If the hash changed, despawn everything and respawn from scratch. This is
//!   wasteful but correct, and good enough until we have real diffing.

use super::bevy_render::*;
use super::skeletal_animation::SceneAnimRoot;
use crate::app::scene_editor::model::*;
use crate::app::BerryCodeApp;
use bevy::camera::visibility::RenderLayers;
use bevy::gltf::Gltf;
use bevy::prelude::*;

/// Sync the editor `SceneModel` to Bevy entities (spawn / update / despawn).
pub fn sync_scene_to_bevy(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    app: bevy::ecs::system::NonSend<BerryCodeApp>,
    mut state: ResMut<SceneEditorRender>,
    mut transforms: Query<&mut Transform, With<SceneEditorObject>>,
) {
    // Compute hash of the current scene model.
    let current_hash = compute_scene_hash(&app.scene_model);

    if current_hash == state.last_sync_hash {
        // Topology unchanged: refresh transforms (inspector edits) and apply
        // any active animation sample so the Scene View previews playback.
        for (id, &bevy_entity) in &state.spawned_entities {
            if let Some(scene_entity) = app.scene_model.entities.get(id) {
                if !scene_entity.enabled {
                    continue;
                }
                if let Ok(mut t) = transforms.get_mut(bevy_entity) {
                    let effective = effective_transform(&app, *id, scene_entity);
                    apply_transform(&mut t, &effective);
                    if let Some(factor) = state.auto_fit_factors.get(id) {
                        t.scale *= *factor;
                    }
                }
            }
        }
        return;
    }

    // Topology changed: despawn everything and respawn. Inefficient but simple.
    tracing::info!(
        "Scene sync: hash changed, respawning {} entities",
        app.scene_model.entities.len()
    );
    for (id, e) in &app.scene_model.entities {
        let comp_labels: Vec<&str> = e.components.iter().map(|c| c.label()).collect();
        tracing::info!("  Entity {} '{}': {:?}", id, e.name, comp_labels);
    }
    let entities_to_despawn: Vec<Entity> = state.spawned_entities.values().copied().collect();
    for e in entities_to_despawn {
        commands.entity(e).despawn();
    }
    state.spawned_entities.clear();
    state.auto_fit_factors.clear();

    // Spawn each scene entity (skip disabled ones).
    for (id, scene_entity) in &app.scene_model.entities {
        if !scene_entity.enabled {
            continue;
        }
        let mut transform = Transform::IDENTITY;
        let effective = effective_transform(&app, *id, scene_entity);
        apply_transform(&mut transform, &effective);

        let (bevy_entity, auto_fit) = spawn_scene_entity(
            &mut commands,
            &mut meshes,
            &mut materials,
            &asset_server,
            transform,
            scene_entity,
            *id,
            &app.root_path,
        );

        state.spawned_entities.insert(*id, bevy_entity);
        if let Some(factor) = auto_fit {
            state.auto_fit_factors.insert(*id, factor);
        }
    }

    state.last_sync_hash = current_hash;

    // Detect skybox component and push path to render state.
    let mut skybox_path: Option<String> = None;
    for entity in app.scene_model.entities.values() {
        for component in &entity.components {
            if let ComponentData::Skybox { path } = component {
                if !path.is_empty() {
                    skybox_path = Some(path.clone());
                }
            }
        }
    }
    state.skybox_path = skybox_path;
}

/// Resolve the world-space transform that should be applied to a Bevy entity
/// this frame. If the entity carries an [`ComponentData::Animation`] component
/// and the app has recorded a playback time for it, the animation sample
/// (which is in local space) is composed with the parent's world transform;
/// otherwise the entity's world transform is computed from the local hierarchy.
fn effective_transform(app: &BerryCodeApp, id: u64, entity: &SceneEntity) -> TransformData {
    for component in &entity.components {
        if let ComponentData::Animation { tracks, .. } = component {
            if let Some(&t) = app.animation_playback.times.get(&id) {
                if !tracks.is_empty() {
                    let sampled = crate::app::scene_editor::animation::sample_animation_tracks(
                        tracks,
                        t,
                        &entity.transform,
                    );
                    // Animation samples are in local space; compose with parent world.
                    return compose_with_parent(&app.scene_model, id, &sampled);
                }
            }
        }
    }
    app.scene_model.compute_world_transform(id)
}

/// Compose a local-space transform with the parent's world transform for the
/// given entity. If the entity has no parent, the local transform IS the world
/// transform.
fn compose_with_parent(scene: &SceneModel, id: u64, local: &TransformData) -> TransformData {
    let parent_id = scene.entities.get(&id).and_then(|e| e.parent);
    match parent_id {
        Some(pid) => {
            let parent_world = scene.compute_world_transform(pid);
            compose_transforms(&parent_world, local)
        }
        None => local.clone(),
    }
}

fn apply_transform(t: &mut Transform, src: &TransformData) {
    t.translation = Vec3::new(src.translation[0], src.translation[1], src.translation[2]);
    t.rotation = Quat::from_euler(
        EulerRot::XYZ,
        src.rotation_euler[0],
        src.rotation_euler[1],
        src.rotation_euler[2],
    );
    t.scale = Vec3::new(src.scale[0], src.scale[1], src.scale[2]);
}

fn spawn_scene_entity(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
    transform: Transform,
    scene_entity: &SceneEntity,
    id: u64,
    project_root: &str,
) -> (Entity, Option<f32>) {
    let mut entity = commands.spawn((
        transform,
        Visibility::default(),
        SceneEditorObject,
        SceneObjectId(id),
        RenderLayers::layer(2),
        Name::new(scene_entity.name.clone()),
    ));
    let mut auto_fit: Option<f32> = None;

    for component in &scene_entity.components {
        match component {
            ComponentData::MeshCube {
                size,
                color,
                metallic,
                roughness,
                emissive,
                texture_path,
                normal_map_path,
            } => {
                let mesh = meshes.add(Cuboid::new(*size, *size, *size));
                let mut std_mat = StandardMaterial {
                    base_color: Color::srgb(color[0], color[1], color[2]),
                    metallic: *metallic,
                    perceptual_roughness: *roughness,
                    emissive: LinearRgba::rgb(emissive[0], emissive[1], emissive[2]),
                    ..default()
                };
                if let Some(tex_path) = texture_path {
                    if !tex_path.is_empty() {
                        let ap = if tex_path.starts_with('/') || tex_path.contains(":\\") {
                            format!("file://{}", tex_path)
                        } else {
                            tex_path.clone()
                        };
                        std_mat.base_color_texture = Some(asset_server.load(&ap));
                    }
                }
                if let Some(nmap_path) = normal_map_path {
                    if !nmap_path.is_empty() {
                        let ap = if nmap_path.starts_with('/') || nmap_path.contains(":\\") {
                            format!("file://{}", nmap_path)
                        } else {
                            nmap_path.clone()
                        };
                        std_mat.normal_map_texture = Some(asset_server.load(&ap));
                    }
                }
                let mat = materials.add(std_mat);
                entity.insert((Mesh3d(mesh), MeshMaterial3d(mat)));
            }
            ComponentData::MeshSphere {
                radius,
                color,
                metallic,
                roughness,
                emissive,
                texture_path,
                normal_map_path,
            } => {
                let mesh = match Sphere::new(*radius).mesh().ico(3) {
                    Ok(m) => meshes.add(m),
                    Err(_) => meshes.add(Sphere::new(*radius).mesh().uv(32, 18)),
                };
                let mut std_mat = StandardMaterial {
                    base_color: Color::srgb(color[0], color[1], color[2]),
                    metallic: *metallic,
                    perceptual_roughness: *roughness,
                    emissive: LinearRgba::rgb(emissive[0], emissive[1], emissive[2]),
                    ..default()
                };
                if let Some(tex_path) = texture_path {
                    if !tex_path.is_empty() {
                        let ap = if tex_path.starts_with('/') || tex_path.contains(":\\") {
                            format!("file://{}", tex_path)
                        } else {
                            tex_path.clone()
                        };
                        std_mat.base_color_texture = Some(asset_server.load(&ap));
                    }
                }
                if let Some(nmap_path) = normal_map_path {
                    if !nmap_path.is_empty() {
                        let ap = if nmap_path.starts_with('/') || nmap_path.contains(":\\") {
                            format!("file://{}", nmap_path)
                        } else {
                            nmap_path.clone()
                        };
                        std_mat.normal_map_texture = Some(asset_server.load(&ap));
                    }
                }
                let mat = materials.add(std_mat);
                entity.insert((Mesh3d(mesh), MeshMaterial3d(mat)));
            }
            ComponentData::MeshPlane {
                size,
                color,
                metallic,
                roughness,
                emissive,
                texture_path,
                normal_map_path,
            } => {
                let mesh = meshes.add(Plane3d::default().mesh().size(*size, *size));
                let mut std_mat = StandardMaterial {
                    base_color: Color::srgb(color[0], color[1], color[2]),
                    metallic: *metallic,
                    perceptual_roughness: *roughness,
                    emissive: LinearRgba::rgb(emissive[0], emissive[1], emissive[2]),
                    ..default()
                };
                if let Some(tex_path) = texture_path {
                    if !tex_path.is_empty() {
                        let ap = if tex_path.starts_with('/') || tex_path.contains(":\\") {
                            format!("file://{}", tex_path)
                        } else {
                            tex_path.clone()
                        };
                        std_mat.base_color_texture = Some(asset_server.load(&ap));
                    }
                }
                if let Some(nmap_path) = normal_map_path {
                    if !nmap_path.is_empty() {
                        let ap = if nmap_path.starts_with('/') || nmap_path.contains(":\\") {
                            format!("file://{}", nmap_path)
                        } else {
                            nmap_path.clone()
                        };
                        std_mat.normal_map_texture = Some(asset_server.load(&ap));
                    }
                }
                let mat = materials.add(std_mat);
                entity.insert((Mesh3d(mesh), MeshMaterial3d(mat)));
            }
            ComponentData::Light { intensity, color } => {
                entity.insert(PointLight {
                    intensity: *intensity,
                    color: Color::srgb(color[0], color[1], color[2]),
                    range: 50.0,
                    ..default()
                });
            }
            ComponentData::SpotLight {
                intensity,
                color,
                range,
                inner_angle,
                outer_angle,
            } => {
                entity.insert(bevy::prelude::SpotLight {
                    intensity: *intensity,
                    color: Color::srgb(color[0], color[1], color[2]),
                    range: *range,
                    inner_angle: *inner_angle,
                    outer_angle: *outer_angle,
                    ..default()
                });
            }
            ComponentData::DirectionalLight {
                intensity,
                color,
                shadows,
            } => {
                entity.insert(bevy::prelude::DirectionalLight {
                    illuminance: *intensity,
                    color: Color::srgb(color[0], color[1], color[2]),
                    shadows_enabled: *shadows,
                    ..default()
                });
            }
            ComponentData::Camera => {
                // Visualize a camera with a small yellow box gizmo.
                let mesh = meshes.add(Cuboid::new(0.3, 0.3, 0.5));
                let mat = materials.add(StandardMaterial {
                    base_color: Color::srgb(1.0, 1.0, 0.0),
                    ..default()
                });
                entity.insert((Mesh3d(mesh), MeshMaterial3d(mat)));
            }
            ComponentData::MeshFromFile {
                path,
                auto_play_clip,
                ..
            } => {
                let abs_path = if path.is_empty() {
                    String::new()
                } else if path.starts_with('/') || path.contains(":\\") {
                    path.clone()
                } else {
                    // Relative paths are relative to the assets/ directory
                    let with_assets = format!("{}/assets/{}", project_root, path);
                    if std::path::Path::new(&with_assets).exists() {
                        with_assets
                    } else {
                        format!("{}/{}", project_root, path)
                    }
                };

                let staged = if abs_path.is_empty() {
                    None
                } else {
                    stage_asset_for_loading(&abs_path)
                };

                if let Some(rel_path) = staged {
                    tracing::info!("Loading GLB scene: {} (staged as {})", abs_path, rel_path);
                    let scene_handle: Handle<Scene> = asset_server
                        .load(bevy::gltf::GltfAssetLabel::Scene(0).from_asset(rel_path.clone()));
                    let gltf_handle: Handle<Gltf> = asset_server.load(rel_path);
                    entity.insert((
                        SceneRoot(scene_handle),
                        SceneAnimRoot {
                            scene_entity_id: id,
                            gltf: gltf_handle,
                            initial_clip: auto_play_clip.clone(),
                        },
                    ));
                    if let Some(factor) = compute_glb_auto_fit(&abs_path) {
                        let mut t = transform;
                        t.scale *= factor;
                        entity.insert(t);
                        auto_fit = Some(factor);
                    }
                } else {
                    if !abs_path.is_empty() {
                        tracing::warn!("Failed to stage GLB asset: {}", abs_path);
                    }
                    // Fallback placeholder
                    let mesh = meshes.add(Cuboid::new(0.5, 0.5, 0.5));
                    let mat = materials.add(StandardMaterial {
                        base_color: Color::srgb(0.6, 0.3, 0.8),
                        ..default()
                    });
                    entity.insert((Mesh3d(mesh), MeshMaterial3d(mat)));
                }
            }
            ComponentData::AudioSource {
                path,
                volume,
                looped,
                autoplay,
            } => {
                // For editor preview: spawn an AudioPlayer if autoplay is true
                // and path is non-empty. The actual file may not exist or may be
                // in a non-asset path, so we tolerate failures gracefully.
                if *autoplay && !path.is_empty() {
                    let asset_path = if path.starts_with('/') || path.contains(":\\") {
                        format!("file://{}", path)
                    } else {
                        path.clone()
                    };
                    let handle = asset_server.load::<bevy::audio::AudioSource>(&asset_path);
                    let mut settings = bevy::audio::PlaybackSettings::ONCE
                        .with_volume(bevy::audio::Volume::Linear(*volume));
                    if *looped {
                        settings = bevy::audio::PlaybackSettings::LOOP
                            .with_volume(bevy::audio::Volume::Linear(*volume));
                    }
                    entity.insert((bevy::audio::AudioPlayer(handle), settings));
                }
                // Visualize with a small purple speaker box gizmo.
                let mesh = meshes.add(Cuboid::new(0.2, 0.2, 0.2));
                let mat = materials.add(StandardMaterial {
                    base_color: Color::srgb(0.8, 0.4, 1.0),
                    emissive: LinearRgba::rgb(0.4, 0.2, 0.5),
                    ..default()
                });
                entity.insert((Mesh3d(mesh), MeshMaterial3d(mat)));
            }
            ComponentData::AudioListener => {
                // Editor preview: just a green diamond so users can see where it is.
                let mesh = meshes.add(Sphere::new(0.15).mesh().uv(16, 8));
                let mat = materials.add(StandardMaterial {
                    base_color: Color::srgb(0.4, 1.0, 0.4),
                    emissive: LinearRgba::rgb(0.2, 0.5, 0.2),
                    ..default()
                });
                entity.insert((Mesh3d(mesh), MeshMaterial3d(mat)));
            }
            ComponentData::RigidBody { .. } => {
                // No visual; rigidbody is metadata. Visual comes from companion mesh/collider.
            }
            ComponentData::Collider { .. } => {
                // Visualization is overlay-drawn from scene_view (egui Painter), not Bevy.
            }
            ComponentData::UiText { .. }
            | ComponentData::UiButton { .. }
            | ComponentData::UiImage { .. } => {
                // UI is authored metadata; rendered by the runtime game (bevy_ui) not the
                // Scene View (which shows 3D world space). We visualize UI entities as a
                // tiny cyan box so they are still click-selectable in the hierarchy/world.
                let mesh = meshes.add(Cuboid::new(0.2, 0.2, 0.05));
                let mat = materials.add(StandardMaterial {
                    base_color: Color::srgb(0.4, 0.8, 1.0),
                    emissive: LinearRgba::rgb(0.2, 0.4, 0.6),
                    ..default()
                });
                entity.insert((Mesh3d(mesh), MeshMaterial3d(mat)));
            }
            ComponentData::ParticleEmitter { .. } => {
                // Visualize emitter origin with a small magenta diamond. The
                // actual particle simulation is rendered as an egui overlay in
                // `scene_view`, not via Bevy meshes.
                let mesh = meshes.add(Sphere::new(0.1).mesh().uv(12, 8));
                let mat = materials.add(StandardMaterial {
                    base_color: Color::srgb(1.0, 0.4, 1.0),
                    emissive: LinearRgba::rgb(0.5, 0.2, 0.5),
                    ..default()
                });
                entity.insert((Mesh3d(mesh), MeshMaterial3d(mat)));
            }
            ComponentData::Animation { .. } => {
                // Animation is data-only; playback is applied in sync_scene_to_bevy
                // via `effective_transform`. No mesh / gizmo to spawn.
            }
            ComponentData::CustomScript { .. } => {
                // Data-only; runtime interprets it via user-defined Rust types.
            }
            ComponentData::Skybox { .. } => {
                // Visual placeholder: small sky-blue sphere (actual HDR loading
                // requires a full asset pipeline pass).
                let mesh = meshes.add(Sphere::new(0.15).mesh().uv(12, 8));
                let mat = materials.add(StandardMaterial {
                    base_color: Color::srgb(0.5, 0.7, 1.0),
                    emissive: LinearRgba::rgb(0.3, 0.4, 0.6),
                    ..default()
                });
                entity.insert((Mesh3d(mesh), MeshMaterial3d(mat)));
            }
            ComponentData::Animator { .. } => {
                // Data-only; the animator state machine is metadata.
            }
            ComponentData::LodGroup { levels } => {
                // Spawn the first (highest detail) level's mesh if available.
                if let Some(first) = levels.first() {
                    if !first.mesh_path.is_empty() {
                        let abs_path = if first.mesh_path.starts_with('/')
                            || first.mesh_path.contains(":\\")
                        {
                            first.mesh_path.clone()
                        } else {
                            format!("{}/assets/{}", project_root, first.mesh_path)
                        };
                        if let Some(rel_path) = stage_asset_for_loading(&abs_path) {
                            let scene_handle: Handle<Scene> = asset_server.load(
                                bevy::gltf::GltfAssetLabel::Scene(0).from_asset(rel_path.clone()),
                            );
                            let gltf_handle: Handle<Gltf> = asset_server.load(rel_path);
                            entity.insert((
                                SceneRoot(scene_handle),
                                SceneAnimRoot {
                                    scene_entity_id: id,
                                    gltf: gltf_handle,
                                    initial_clip: None,
                                },
                            ));
                            if let Some(factor) = compute_glb_auto_fit(&abs_path) {
                                let mut t = transform;
                                t.scale *= factor;
                                entity.insert(t);
                                auto_fit = Some(factor);
                            }
                        }
                    }
                }
            }
            ComponentData::Spline { .. } => {
                // Data-only; spline visualization is handled by gizmos.
            }
            ComponentData::Terrain {
                resolution,
                world_size,
                heights,
                base_color,
            } => {
                let terrain_mesh =
                    super::terrain::generate_terrain_mesh(heights, *resolution, *world_size);
                let mesh = meshes.add(terrain_mesh);
                let mat = materials.add(StandardMaterial {
                    base_color: Color::srgb(base_color[0], base_color[1], base_color[2]),
                    perceptual_roughness: 0.9,
                    ..default()
                });
                entity.insert((Mesh3d(mesh), MeshMaterial3d(mat)));
            }
            ComponentData::SkinnedMesh { path, .. } => {
                // Load the GLB/GLTF as a scene, similar to MeshFromFile.
                if !path.is_empty() {
                    let abs_path = if path.starts_with('/') || path.contains(":\\") {
                        path.clone()
                    } else {
                        format!("{}/assets/{}", project_root, path)
                    };
                    if let Some(rel_path) = stage_asset_for_loading(&abs_path) {
                        let scene_handle: Handle<Scene> = asset_server.load(
                            bevy::gltf::GltfAssetLabel::Scene(0).from_asset(rel_path.clone()),
                        );
                        let gltf_handle: Handle<Gltf> = asset_server.load(rel_path);
                        entity.insert((
                            SceneRoot(scene_handle),
                            SceneAnimRoot {
                                scene_entity_id: id,
                                gltf: gltf_handle,
                                initial_clip: None,
                            },
                        ));
                        if let Some(factor) = compute_glb_auto_fit(&abs_path) {
                            let mut t = transform;
                            t.scale *= factor;
                            entity.insert(t);
                            auto_fit = Some(factor);
                        }
                    }
                }
            }
            ComponentData::VisualScript { .. } => {
                // Data-only; visual script is metadata for the runtime.
            }
            ComponentData::NavMesh { .. } => {
                // Data-only; navmesh is baked metadata for pathfinding.
            }
            ComponentData::JointFixed { .. }
            | ComponentData::JointHinge { .. }
            | ComponentData::JointSpring { .. } => {
                // Data-only; physics joints are metadata for the runtime.
            }
            ComponentData::NavMeshAgent { .. } => {
                // Data-only; navmesh agent is metadata for the runtime.
            }
            ComponentData::PlayerController { .. } => {
                // Visualize the player as a yellow capsule so the user can see
                // it in the Scene View even before they hit Run. Sized to
                // match the runtime collider's defaults (radius 0.4, total
                // height 1.7) so what they preview is what they play.
                let mesh = meshes.add(Capsule3d::new(0.4, 0.9));
                let mat = materials.add(StandardMaterial {
                    base_color: Color::srgb(1.0, 0.85, 0.2),
                    emissive: LinearRgba::rgb(0.4, 0.34, 0.08),
                    ..default()
                });
                entity.insert((Mesh3d(mesh), MeshMaterial3d(mat)));
            }
        }
    }

    (entity.id(), auto_fit)
}

/// Mesh positions and triangle indices extracted from a GLTF/GLB file. Used
/// for bounding-box / auto-scale calculations and for drawing mesh-outline
/// overlays on selected entities. `indices` is always populated: when the
/// primitive is unindexed, it is synthesized as `0,1,2,3,...` so callers can
/// treat the data uniformly as a triangle list.
pub struct GltfMeshData {
    pub positions: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
}

/// Extract vertex positions and triangle indices from the first primitive of
/// the first mesh in a GLTF/GLB file. Pure function, no Bevy dependency.
pub fn extract_gltf_mesh_data(file_path: &str) -> Option<GltfMeshData> {
    let (document, buffers, _images) = gltf::import(file_path).ok()?;

    let gltf_mesh = document.meshes().next()?;
    let primitive = gltf_mesh.primitives().next()?;

    let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
    let positions: Vec<[f32; 3]> = reader.read_positions()?.collect();
    let indices: Vec<u32> = match reader.read_indices() {
        Some(it) => it.into_u32().collect(),
        None => (0..positions.len() as u32).collect(),
    };

    Some(GltfMeshData { positions, indices })
}

/// Compute a fit-to-view scale factor from the GLB's vertex bounds. Returns
/// `Some(factor)` when the largest extent exceeds 5 units (i.e. the model is
/// authored in centimeters or similar and would otherwise render huge);
/// `None` for already-small models. The caller is expected to multiply this
/// factor into the entity's `Transform.scale` and to re-apply it on every
/// per-frame sync (since `apply_transform` overwrites scale wholesale).
pub fn compute_glb_auto_fit(abs_path: &str) -> Option<f32> {
    let data = extract_gltf_mesh_data(abs_path)?;
    let mut min = [f32::MAX; 3];
    let mut max = [f32::MIN; 3];
    for p in &data.positions {
        for i in 0..3 {
            min[i] = min[i].min(p[i]);
            max[i] = max[i].max(p[i]);
        }
    }
    let extent = (max[0] - min[0])
        .max(max[1] - min[1])
        .max(max[2] - min[2])
        .max(0.001);
    if extent <= 5.0 {
        return None;
    }
    Some(2.0 / extent)
}

/// Stage a model file for loading by Bevy's `AssetServer`.
///
/// Bevy's default `AssetPlugin` only resolves paths under the `assets/`
/// directory configured at startup. To load models the user picked from
/// arbitrary disk locations, we copy the file into `assets/_preview/` and
/// return a relative asset path.
///
/// Returns `None` if the source file is missing, empty, or the copy fails.
fn stage_asset_for_loading(abs_path: &str) -> Option<String> {
    let file_path = std::path::Path::new(abs_path);
    let file_name = file_path.file_name()?.to_string_lossy().to_string();

    let bevy_assets_dir = option_env!("CARGO_MANIFEST_DIR")
        .map(|d| std::path::PathBuf::from(d).join("assets"))
        .unwrap_or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.join("assets")))
                .unwrap_or_else(|| std::path::PathBuf::from("assets"))
        });

    let preview_dir = bevy_assets_dir.join("_preview");
    std::fs::create_dir_all(&preview_dir).ok()?;
    let dest = preview_dir.join(&file_name);

    let src_size = std::fs::metadata(abs_path).map(|m| m.len()).ok()?;
    if src_size == 0 {
        return None;
    }
    let dst_size = std::fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
    if src_size != dst_size {
        std::fs::copy(abs_path, &dest).ok()?;
    }

    Some(format!("_preview/{}", file_name))
}

fn compute_scene_hash(scene: &SceneModel) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    scene.entities.len().hash(&mut hasher);
    // `scene.entities` is a `HashMap`, whose iteration order is unstable
    // across `clone()` calls (the cloned table can reorder buckets). Without
    // sorting, the hash flips spuriously every time a tab switch / play-mode
    // snapshot / hot-reload clones `scene_model`, which fires a full
    // despawn+respawn of every entity (and reloads any GLB scenes from disk)
    // about once a second. Sort by id so the hash is content-addressed.
    let mut sorted: Vec<(&u64, &SceneEntity)> = scene.entities.iter().collect();
    sorted.sort_by_key(|(id, _)| **id);
    for (id, e) in sorted {
        id.hash(&mut hasher);
        e.name.hash(&mut hasher);
        e.enabled.hash(&mut hasher);
        e.components.len().hash(&mut hasher);
        for c in &e.components {
            std::mem::discriminant(c).hash(&mut hasher);
            // Include mesh material/PBR fields so inspector edits trigger a
            // re-sync (the fast transform-only path is gated on this hash).
            match c {
                ComponentData::MeshCube {
                    size,
                    color,
                    metallic,
                    roughness,
                    emissive,
                    texture_path,
                    normal_map_path,
                } => {
                    size.to_bits().hash(&mut hasher);
                    for v in color {
                        v.to_bits().hash(&mut hasher);
                    }
                    metallic.to_bits().hash(&mut hasher);
                    roughness.to_bits().hash(&mut hasher);
                    for v in emissive {
                        v.to_bits().hash(&mut hasher);
                    }
                    texture_path.hash(&mut hasher);
                    normal_map_path.hash(&mut hasher);
                }
                ComponentData::MeshSphere {
                    radius,
                    color,
                    metallic,
                    roughness,
                    emissive,
                    texture_path,
                    normal_map_path,
                } => {
                    radius.to_bits().hash(&mut hasher);
                    for v in color {
                        v.to_bits().hash(&mut hasher);
                    }
                    metallic.to_bits().hash(&mut hasher);
                    roughness.to_bits().hash(&mut hasher);
                    for v in emissive {
                        v.to_bits().hash(&mut hasher);
                    }
                    texture_path.hash(&mut hasher);
                    normal_map_path.hash(&mut hasher);
                }
                ComponentData::MeshPlane {
                    size,
                    color,
                    metallic,
                    roughness,
                    emissive,
                    texture_path,
                    normal_map_path,
                } => {
                    size.to_bits().hash(&mut hasher);
                    for v in color {
                        v.to_bits().hash(&mut hasher);
                    }
                    metallic.to_bits().hash(&mut hasher);
                    roughness.to_bits().hash(&mut hasher);
                    for v in emissive {
                        v.to_bits().hash(&mut hasher);
                    }
                    texture_path.hash(&mut hasher);
                    normal_map_path.hash(&mut hasher);
                }
                ComponentData::Light { intensity, color } => {
                    intensity.to_bits().hash(&mut hasher);
                    for v in color {
                        v.to_bits().hash(&mut hasher);
                    }
                }
                ComponentData::SpotLight {
                    intensity,
                    color,
                    range,
                    inner_angle,
                    outer_angle,
                } => {
                    intensity.to_bits().hash(&mut hasher);
                    for v in color {
                        v.to_bits().hash(&mut hasher);
                    }
                    range.to_bits().hash(&mut hasher);
                    inner_angle.to_bits().hash(&mut hasher);
                    outer_angle.to_bits().hash(&mut hasher);
                }
                ComponentData::DirectionalLight {
                    intensity,
                    color,
                    shadows,
                } => {
                    intensity.to_bits().hash(&mut hasher);
                    for v in color {
                        v.to_bits().hash(&mut hasher);
                    }
                    shadows.hash(&mut hasher);
                }
                ComponentData::Camera => {}
                ComponentData::MeshFromFile {
                    path,
                    texture_path,
                    normal_map_path,
                    auto_play_clip,
                } => {
                    path.hash(&mut hasher);
                    texture_path.hash(&mut hasher);
                    normal_map_path.hash(&mut hasher);
                    auto_play_clip.hash(&mut hasher);
                }
                ComponentData::AudioSource {
                    path,
                    volume,
                    looped,
                    autoplay,
                } => {
                    path.hash(&mut hasher);
                    volume.to_bits().hash(&mut hasher);
                    looped.hash(&mut hasher);
                    autoplay.hash(&mut hasher);
                }
                ComponentData::AudioListener => {}
                ComponentData::RigidBody { body_type, mass } => {
                    std::mem::discriminant(body_type).hash(&mut hasher);
                    mass.to_bits().hash(&mut hasher);
                }
                ComponentData::Collider {
                    shape,
                    friction,
                    restitution,
                    physic_material_path,
                } => {
                    physic_material_path.hash(&mut hasher);
                    match shape {
                        ColliderShape::Box { half_extents } => {
                            "box".hash(&mut hasher);
                            for v in half_extents {
                                v.to_bits().hash(&mut hasher);
                            }
                        }
                        ColliderShape::Sphere { radius } => {
                            "sphere".hash(&mut hasher);
                            radius.to_bits().hash(&mut hasher);
                        }
                        ColliderShape::Capsule {
                            half_height,
                            radius,
                        } => {
                            "capsule".hash(&mut hasher);
                            half_height.to_bits().hash(&mut hasher);
                            radius.to_bits().hash(&mut hasher);
                        }
                    }
                    friction.to_bits().hash(&mut hasher);
                    restitution.to_bits().hash(&mut hasher);
                }
                ComponentData::UiText {
                    text,
                    font_size,
                    color,
                } => {
                    text.hash(&mut hasher);
                    font_size.to_bits().hash(&mut hasher);
                    for v in color {
                        v.to_bits().hash(&mut hasher);
                    }
                }
                ComponentData::UiButton { label, background } => {
                    label.hash(&mut hasher);
                    for v in background {
                        v.to_bits().hash(&mut hasher);
                    }
                }
                ComponentData::UiImage { path, tint } => {
                    path.hash(&mut hasher);
                    for v in tint {
                        v.to_bits().hash(&mut hasher);
                    }
                }
                ComponentData::ParticleEmitter {
                    rate,
                    lifetime,
                    speed,
                    spread,
                    start_size,
                    end_size,
                    start_color,
                    end_color,
                    max_particles,
                    gravity,
                } => {
                    rate.to_bits().hash(&mut hasher);
                    lifetime.to_bits().hash(&mut hasher);
                    speed.to_bits().hash(&mut hasher);
                    spread.to_bits().hash(&mut hasher);
                    start_size.to_bits().hash(&mut hasher);
                    end_size.to_bits().hash(&mut hasher);
                    for v in start_color {
                        v.to_bits().hash(&mut hasher);
                    }
                    for v in end_color {
                        v.to_bits().hash(&mut hasher);
                    }
                    max_particles.hash(&mut hasher);
                    gravity.to_bits().hash(&mut hasher);
                }
                ComponentData::Animation {
                    duration,
                    tracks,
                    looped,
                } => {
                    duration.to_bits().hash(&mut hasher);
                    tracks.len().hash(&mut hasher);
                    for track in tracks {
                        (track.property as u8).hash(&mut hasher);
                        track.keyframes.len().hash(&mut hasher);
                        for kf in &track.keyframes {
                            kf.time.to_bits().hash(&mut hasher);
                            for v in kf.value {
                                v.to_bits().hash(&mut hasher);
                            }
                            (kf.easing as u8).hash(&mut hasher);
                        }
                    }
                    looped.hash(&mut hasher);
                }
                ComponentData::CustomScript {
                    type_name, fields, ..
                } => {
                    type_name.hash(&mut hasher);
                    fields.len().hash(&mut hasher);
                    for f in fields {
                        f.name.hash(&mut hasher);
                        match &f.value {
                            ScriptValue::Float(v) => {
                                "f".hash(&mut hasher);
                                v.to_bits().hash(&mut hasher);
                            }
                            ScriptValue::Int(v) => {
                                "i".hash(&mut hasher);
                                v.hash(&mut hasher);
                            }
                            ScriptValue::Bool(v) => {
                                "b".hash(&mut hasher);
                                v.hash(&mut hasher);
                            }
                            ScriptValue::String(v) => {
                                "s".hash(&mut hasher);
                                v.hash(&mut hasher);
                            }
                            ScriptValue::Vec(items) => {
                                "v".hash(&mut hasher);
                                items.len().hash(&mut hasher);
                            }
                            ScriptValue::Option(opt) => {
                                "o".hash(&mut hasher);
                                opt.is_some().hash(&mut hasher);
                            }
                            ScriptValue::Map(entries) => {
                                "m".hash(&mut hasher);
                                entries.len().hash(&mut hasher);
                            }
                        }
                    }
                }
                ComponentData::Skybox { path } => {
                    path.hash(&mut hasher);
                }
                ComponentData::Animator { controller_path } => {
                    controller_path.hash(&mut hasher);
                }
                ComponentData::LodGroup { levels } => {
                    levels.len().hash(&mut hasher);
                    for level in levels {
                        level.mesh_path.hash(&mut hasher);
                        level.screen_percentage.to_bits().hash(&mut hasher);
                    }
                }
                ComponentData::Spline { points, closed } => {
                    points.len().hash(&mut hasher);
                    closed.hash(&mut hasher);
                    for p in points {
                        for v in &p.position {
                            v.to_bits().hash(&mut hasher);
                        }
                        for v in &p.tangent_in {
                            v.to_bits().hash(&mut hasher);
                        }
                        for v in &p.tangent_out {
                            v.to_bits().hash(&mut hasher);
                        }
                    }
                }
                ComponentData::Terrain {
                    resolution,
                    world_size,
                    heights,
                    base_color,
                } => {
                    resolution.hash(&mut hasher);
                    for v in world_size {
                        v.to_bits().hash(&mut hasher);
                    }
                    heights.len().hash(&mut hasher);
                    for h in heights {
                        h.to_bits().hash(&mut hasher);
                    }
                    for v in base_color {
                        v.to_bits().hash(&mut hasher);
                    }
                }
                ComponentData::SkinnedMesh { path, bones } => {
                    path.hash(&mut hasher);
                    bones.len().hash(&mut hasher);
                    for b in bones {
                        b.name.hash(&mut hasher);
                        b.parent_idx.hash(&mut hasher);
                    }
                }
                ComponentData::VisualScript { path } => {
                    path.hash(&mut hasher);
                }
                ComponentData::NavMesh {
                    cell_size,
                    grid,
                    width,
                    height,
                } => {
                    cell_size.to_bits().hash(&mut hasher);
                    grid.len().hash(&mut hasher);
                    width.hash(&mut hasher);
                    height.hash(&mut hasher);
                }
                ComponentData::JointFixed { connected_entity } => {
                    connected_entity.hash(&mut hasher);
                }
                ComponentData::JointHinge {
                    connected_entity,
                    axis,
                    limits,
                } => {
                    connected_entity.hash(&mut hasher);
                    for v in axis {
                        v.to_bits().hash(&mut hasher);
                    }
                    if let Some(lim) = limits {
                        true.hash(&mut hasher);
                        lim[0].to_bits().hash(&mut hasher);
                        lim[1].to_bits().hash(&mut hasher);
                    } else {
                        false.hash(&mut hasher);
                    }
                }
                ComponentData::JointSpring {
                    connected_entity,
                    stiffness,
                    damping,
                    rest_length,
                } => {
                    connected_entity.hash(&mut hasher);
                    stiffness.to_bits().hash(&mut hasher);
                    damping.to_bits().hash(&mut hasher);
                    rest_length.to_bits().hash(&mut hasher);
                }
                ComponentData::NavMeshAgent {
                    speed,
                    radius,
                    height,
                    max_slope,
                } => {
                    speed.to_bits().hash(&mut hasher);
                    radius.to_bits().hash(&mut hasher);
                    height.to_bits().hash(&mut hasher);
                    max_slope.to_bits().hash(&mut hasher);
                }
                ComponentData::PlayerController {
                    speed,
                    jump_velocity,
                    run_multiplier,
                    turn_speed,
                } => {
                    speed.to_bits().hash(&mut hasher);
                    jump_velocity.to_bits().hash(&mut hasher);
                    run_multiplier.to_bits().hash(&mut hasher);
                    turn_speed.to_bits().hash(&mut hasher);
                }
            }
        }
    }
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_gltf_mesh_data_from_fox() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../berrycode/tests/fixtures/fox_test.glb"
        );
        // If test fixture doesn't exist, try the user's project
        let path = if std::path::Path::new(path).exists() {
            path.to_string()
        } else {
            "/Users/Kyosuke/Test2/assets/fox.glb".to_string()
        };
        if !std::path::Path::new(&path).exists() {
            eprintln!("Skipping test: fox.glb not found at {}", path);
            return;
        }

        let data = extract_gltf_mesh_data(&path);
        assert!(data.is_some(), "extract_gltf_mesh_data returned None");
        let data = data.unwrap();
        assert!(!data.positions.is_empty(), "No positions extracted");
        println!("fox.glb: {} positions", data.positions.len());
    }

    #[test]
    fn extract_gltf_mesh_data_bounds() {
        let path = "/Users/Kyosuke/Test2/assets/fox.glb";
        if !std::path::Path::new(path).exists() {
            return;
        }
        let data = extract_gltf_mesh_data(path).unwrap();
        let mut min = [f32::MAX; 3];
        let mut max = [f32::MIN; 3];
        for p in &data.positions {
            for i in 0..3 {
                min[i] = min[i].min(p[i]);
                max[i] = max[i].max(p[i]);
            }
        }
        println!("Bounds: min={:?} max={:?}", min, max);
        println!(
            "Size: [{}, {}, {}]",
            max[0] - min[0],
            max[1] - min[1],
            max[2] - min[2]
        );
    }

    #[test]
    fn extract_gltf_nonexistent_returns_none() {
        let result = extract_gltf_mesh_data("/nonexistent/file.glb");
        assert!(result.is_none());
    }
}
