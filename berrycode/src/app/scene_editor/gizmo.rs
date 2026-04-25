//! Transform gizmo and picking math for the scene editor.
//!
//! All math here mirrors the camera setup used by `bevy_render`:
//! - Camera position is reconstructed from `(yaw, pitch, distance)` orbit params
//!   around the origin, looking at `Vec3::ZERO` with `Vec3::Y` up.
//! - Projection assumes a 45° vertical FOV and a 1024x768 (4:3) aspect ratio,
//!   matching the off-screen render target.
//!
//! Public API:
//! - [`GizmoMode`]: Move / Rotate / Scale.
//! - [`project_to_screen`]: world -> screen position inside the viewport rect.
//! - [`screen_to_ray`]: viewport screen position -> world ray.
//! - [`ray_aabb_hit`]: slab-method ray vs AABB intersection.
//! - [`aabb_for_entity`]: AABB derived from a `SceneEntity`'s components.

use crate::app::scene_editor::model::*;
use bevy::math::Vec3;

/// Current transform gizmo mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GizmoMode {
    Move,
    Rotate,
    Scale,
}

impl Default for GizmoMode {
    fn default() -> Self {
        GizmoMode::Move
    }
}

/// Identifies which part of the gizmo is being dragged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GizmoDrag {
    /// Dragging along a single axis: 0=X, 1=Y, 2=Z.
    SingleAxis(usize),
    /// Dragging on a plane defined by two axes, e.g. (0,2) = XZ plane.
    Plane(usize, usize),
}

/// Camera FOV (vertical), in radians. Must match `bevy_render` setup.
const FOV_Y: f32 = std::f32::consts::FRAC_PI_4; // 45 degrees
/// Aspect ratio of the off-screen render target (1024x768 = 4:3).
const ASPECT: f32 = 1024.0 / 768.0;

/// Reconstruct the camera position from orbit parameters. This must stay in
/// sync with `update_scene_editor_camera` in `bevy_render.rs`.
pub fn camera_position(yaw: f32, pitch: f32, distance: f32, target: Vec3) -> Vec3 {
    let offset = Vec3::new(
        distance * yaw.cos() * pitch.cos(),
        distance * pitch.sin(),
        distance * yaw.sin() * pitch.cos(),
    );
    target + offset
}

/// Build the camera basis (forward, right, up) given the camera position and
/// look-at target.
fn camera_basis(cam_pos: Vec3, cam_target: Vec3) -> (Vec3, Vec3, Vec3) {
    let world_up = Vec3::Y;
    let forward = (cam_target - cam_pos).normalize();
    let right = forward.cross(world_up).normalize();
    let up = right.cross(forward);
    (forward, right, up)
}

/// Project a 3D world point to screen space within the given viewport rect.
/// Returns `None` if the point is behind the camera.
pub fn project_to_screen(
    world: Vec3,
    cam_pos: Vec3,
    cam_target: Vec3,
    rect: egui::Rect,
    ortho: bool,
    ortho_scale: f32,
) -> Option<egui::Pos2> {
    let (forward, right, up) = camera_basis(cam_pos, cam_target);

    if ortho {
        let relative = world - cam_pos;
        let x = relative.dot(right);
        let y = relative.dot(up);
        let half_h = ortho_scale;
        let half_w = half_h * ASPECT;
        let nx = x / half_w * 0.5 + 0.5;
        let ny = 0.5 - y / half_h * 0.5;
        return Some(egui::pos2(
            rect.left() + nx * rect.width(),
            rect.top() + ny * rect.height(),
        ));
    }

    // Camera-space coordinates.
    let local = world - cam_pos;
    let cx = local.dot(right);
    let cy = local.dot(up);
    let cz = local.dot(forward);

    if cz <= 0.01 {
        return None; // Behind (or too close to) the camera.
    }

    let tan_half_fov = (FOV_Y / 2.0).tan();
    let ndc_x = (cx / cz) / (tan_half_fov * ASPECT);
    let ndc_y = (cy / cz) / tan_half_fov;

    let screen_x = rect.min.x + (ndc_x + 1.0) * 0.5 * rect.width();
    // NDC y increases up; screen y increases down.
    let screen_y = rect.min.y + (1.0 - (ndc_y + 1.0) * 0.5) * rect.height();

    Some(egui::pos2(screen_x, screen_y))
}

/// Cast a world-space ray from a screen-space position inside the viewport.
/// Returns `(origin, direction)` where direction is normalized.
pub fn screen_to_ray(
    screen: egui::Pos2,
    cam_pos: Vec3,
    cam_target: Vec3,
    rect: egui::Rect,
    ortho: bool,
    ortho_scale: f32,
) -> (Vec3, Vec3) {
    let ndc_x = (screen.x - rect.min.x) / rect.width() * 2.0 - 1.0;
    let ndc_y = -((screen.y - rect.min.y) / rect.height() * 2.0 - 1.0);

    let (forward, right, up) = camera_basis(cam_pos, cam_target);

    if ortho {
        let half_h = ortho_scale;
        let half_w = half_h * ASPECT;
        let origin = cam_pos + right * ndc_x * half_w + up * ndc_y * half_h;
        return (origin, forward);
    }

    let tan_half_fov = (FOV_Y / 2.0).tan();

    let ray_dir = (forward + right * (ndc_x * tan_half_fov * ASPECT) + up * (ndc_y * tan_half_fov))
        .normalize();

    (cam_pos, ray_dir)
}

/// Slab method ray-AABB intersection. Returns `Some(t_enter)` of the entry
/// distance (clamped to 0 if the ray starts inside the box), or `None` if no
/// hit.
pub fn ray_aabb_hit(
    ray_origin: Vec3,
    ray_dir: Vec3,
    aabb_min: Vec3,
    aabb_max: Vec3,
) -> Option<f32> {
    // Avoid division by zero by treating zero components as a very small value.
    let safe = |v: f32| {
        if v.abs() < 1e-8 {
            1e-8_f32.copysign(v.max(0.0))
        } else {
            v
        }
    };
    let inv_d = Vec3::new(
        1.0 / safe(ray_dir.x),
        1.0 / safe(ray_dir.y),
        1.0 / safe(ray_dir.z),
    );

    let t1 = (aabb_min - ray_origin) * inv_d;
    let t2 = (aabb_max - ray_origin) * inv_d;
    let tmin = t1.min(t2);
    let tmax = t1.max(t2);
    let t_enter = tmin.x.max(tmin.y).max(tmin.z);
    let t_exit = tmax.x.min(tmax.y).min(tmax.z);

    if t_enter <= t_exit && t_exit >= 0.0 {
        Some(t_enter.max(0.0))
    } else {
        None
    }
}

/// Compute a world-space AABB for a scene entity based on its first renderable
/// component. The caller must supply the entity's world-space transform (since
/// `entity.transform` is now local-space).
pub fn aabb_for_entity(
    entity: &SceneEntity,
    world_transform: &TransformData,
) -> Option<(Vec3, Vec3)> {
    let pos = Vec3::from_array(world_transform.translation);
    let scale = Vec3::from_array(world_transform.scale);

    for c in &entity.components {
        match c {
            ComponentData::MeshCube { size, .. } => {
                let h = (*size * 0.5) * scale.abs();
                return Some((pos - h, pos + h));
            }
            ComponentData::MeshSphere { radius, .. } => {
                let h = Vec3::splat(*radius) * scale.abs();
                return Some((pos - h, pos + h));
            }
            ComponentData::MeshPlane { size, .. } => {
                let h = Vec3::new(*size * 0.5, 0.05, *size * 0.5) * scale.abs();
                return Some((pos - h, pos + h));
            }
            ComponentData::Light { .. }
            | ComponentData::SpotLight { .. }
            | ComponentData::DirectionalLight { .. }
            | ComponentData::Camera => {
                let h = Vec3::splat(0.3);
                return Some((pos - h, pos + h));
            }
            ComponentData::MeshFromFile { path, .. } => {
                // Try to get real bounds from GLTF data
                if !path.is_empty() {
                    if let Some(data) =
                        crate::app::scene_editor::bevy_sync::extract_gltf_mesh_data(path)
                    {
                        let mut bmin = [f32::MAX; 3];
                        let mut bmax = [f32::MIN; 3];
                        for p in &data.positions {
                            for i in 0..3 {
                                bmin[i] = bmin[i].min(p[i]);
                                bmax[i] = bmax[i].max(p[i]);
                            }
                        }
                        let extent_max = (bmax[0] - bmin[0])
                            .max(bmax[1] - bmin[1])
                            .max(bmax[2] - bmin[2])
                            .max(0.001);
                        let auto_s = if extent_max > 5.0 {
                            2.0 / extent_max
                        } else {
                            1.0
                        };
                        let h = Vec3::new(
                            (bmax[0] - bmin[0]) * 0.5 * auto_s,
                            (bmax[1] - bmin[1]) * 0.5 * auto_s,
                            (bmax[2] - bmin[2]) * 0.5 * auto_s,
                        ) * scale.abs();
                        let center_y = (bmax[1] - bmin[1]) * 0.5 * auto_s;
                        return Some((
                            pos - h + Vec3::new(0.0, center_y, 0.0),
                            pos + h + Vec3::new(0.0, center_y, 0.0),
                        ));
                    }
                }
                let h = Vec3::splat(0.5) * scale.abs();
                return Some((pos - h, pos + h));
            }
            ComponentData::AudioSource { .. } => {
                return Some((pos - Vec3::splat(0.15), pos + Vec3::splat(0.15)));
            }
            ComponentData::AudioListener => {
                return Some((pos - Vec3::splat(0.15), pos + Vec3::splat(0.15)));
            }
            ComponentData::RigidBody { .. } => {
                // RigidBody has no visual; fall through to the next component.
                continue;
            }
            ComponentData::Collider { shape, .. } => {
                // Use the collider bounds as the pickable AABB so entities with
                // only RigidBody + Collider (no mesh) are still click-selectable.
                let h = match shape {
                    ColliderShape::Box { half_extents } => Vec3::from_array(*half_extents),
                    ColliderShape::Sphere { radius } => Vec3::splat(*radius),
                    ColliderShape::Capsule {
                        half_height,
                        radius,
                    } => Vec3::new(*radius, *half_height + *radius, *radius),
                };
                return Some((pos - h * scale.abs(), pos + h * scale.abs()));
            }
            ComponentData::UiText { .. }
            | ComponentData::UiButton { .. }
            | ComponentData::UiImage { .. } => {
                // UI entities are 2D by nature; give them a tiny 3D AABB so the
                // placeholder gizmo cuboid is still click-selectable in the
                // Scene View.
                return Some((
                    pos - Vec3::new(0.1, 0.1, 0.025),
                    pos + Vec3::new(0.1, 0.1, 0.025),
                ));
            }
            ComponentData::ParticleEmitter { .. } => {
                return Some((pos - Vec3::splat(0.15), pos + Vec3::splat(0.15)));
            }
            ComponentData::Animation { .. } => {
                // Animation has no visual footprint of its own; fall through to
                // the next component (if any) and ultimately `None`.
                continue;
            }
            ComponentData::CustomScript { .. } => {
                // Data-only, no footprint.
                continue;
            }
            ComponentData::Skybox { .. } => {
                // Small AABB matching the 0.15 sphere placeholder.
                return Some((pos - Vec3::splat(0.15), pos + Vec3::splat(0.15)));
            }
            ComponentData::Animator { .. } => {
                // Data-only, no footprint.
                continue;
            }
            ComponentData::LodGroup { .. } => {
                // Use a conservative 1m AABB (similar to MeshFromFile).
                let h = Vec3::splat(0.5) * scale.abs();
                return Some((pos - h, pos + h));
            }
            ComponentData::Spline { points, .. } => {
                // AABB from the first point (small marker), or fall back to
                // entity position if the point list is empty.
                if let Some(first) = points.first() {
                    let p =
                        Vec3::new(first.position[0], first.position[1], first.position[2]) + pos;
                    return Some((p - Vec3::splat(0.15), p + Vec3::splat(0.15)));
                }
                return Some((pos - Vec3::splat(0.15), pos + Vec3::splat(0.15)));
            }
            ComponentData::Terrain { world_size, .. } => {
                let half_w = world_size[0] * 0.5;
                let half_d = world_size[1] * 0.5;
                let h = Vec3::new(half_w, 1.0, half_d);
                return Some((pos - h, pos + h));
            }
            ComponentData::SkinnedMesh { .. } => {
                // Conservative 1m AABB (similar to MeshFromFile).
                let h = Vec3::splat(0.5) * scale.abs();
                return Some((pos - h, pos + h));
            }
            ComponentData::VisualScript { .. } => {
                // Data-only, no footprint.
                continue;
            }
            ComponentData::NavMesh { .. } => {
                // Data-only, no footprint.
                continue;
            }
        }
    }
    None
}
