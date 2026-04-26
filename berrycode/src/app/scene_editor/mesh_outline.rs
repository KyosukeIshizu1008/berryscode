//! Per-triangle mesh-outline rendering for selected entities.
//!
//! Both the Scene Editor (selection wireframe) and the ECS Inspector (3D
//! entity overview) use this to project a mesh's triangles into screen space
//! and draw their edges, instead of falling back to a generic AABB cube.

use crate::app::scene_editor::gizmo::project_to_screen;
use crate::app::scene_editor::model::{ColliderShape, ComponentData, SceneEntity, TransformData};
use bevy::math::{Quat, Vec3};

/// Build the list of local-space triangles that represent an entity's first
/// renderable component. Returns `None` for non-renderable entities (lights,
/// data-only components, etc.) so the caller can fall back to a default
/// gizmo. Paths inside `MeshFromFile` are resolved relative to
/// `<project_root>/assets/`.
pub fn triangles_for_entity(
    entity: &SceneEntity,
    project_root: &str,
) -> Option<Vec<[Vec3; 3]>> {
    for c in &entity.components {
        match c {
            ComponentData::MeshCube { size, .. } => {
                return Some(cube_triangles(*size));
            }
            ComponentData::MeshSphere { radius, .. } => {
                return Some(sphere_triangles(*radius, 12, 8));
            }
            ComponentData::MeshPlane { size, .. } => {
                return Some(plane_triangles(*size));
            }
            ComponentData::MeshFromFile { path, .. } => {
                if path.is_empty() {
                    continue;
                }
                let abs = resolve_asset_path(path, project_root);
                let data = crate::app::scene_editor::bevy_sync::extract_gltf_mesh_data(&abs)?;
                let auto_fit = compute_auto_fit_factor(&data.positions);
                let mut tris = Vec::with_capacity(data.indices.len() / 3);
                for chunk in data.indices.chunks_exact(3) {
                    let a = data.positions.get(chunk[0] as usize)?;
                    let b = data.positions.get(chunk[1] as usize)?;
                    let c = data.positions.get(chunk[2] as usize)?;
                    tris.push([
                        Vec3::from_array(*a) * auto_fit,
                        Vec3::from_array(*b) * auto_fit,
                        Vec3::from_array(*c) * auto_fit,
                    ]);
                }
                return Some(tris);
            }
            ComponentData::Collider { shape, .. } => match shape {
                ColliderShape::Box { half_extents } => {
                    return Some(cube_triangles(half_extents[0].max(half_extents[1]).max(half_extents[2]) * 2.0));
                }
                ColliderShape::Sphere { radius } => {
                    return Some(sphere_triangles(*radius, 12, 8));
                }
                ColliderShape::Capsule { radius, .. } => {
                    return Some(sphere_triangles(*radius, 12, 8));
                }
            },
            _ => continue,
        }
    }
    None
}

/// Project triangle vertices to screen space and draw their three edges.
/// Triangles whose vertices project off-screen (behind the camera) are
/// skipped. The world transform composes with each triangle vertex.
pub fn draw_mesh_outline(
    painter: &egui::Painter,
    triangles: &[[Vec3; 3]],
    world_transform: &TransformData,
    cam_pos: Vec3,
    cam_target: Vec3,
    rect: egui::Rect,
    ortho: bool,
    ortho_scale: f32,
    color: egui::Color32,
    width: f32,
) {
    let translation = Vec3::from_array(world_transform.translation);
    let rotation = Quat::from_euler(
        bevy::math::EulerRot::XYZ,
        world_transform.rotation_euler[0],
        world_transform.rotation_euler[1],
        world_transform.rotation_euler[2],
    );
    let scale = Vec3::from_array(world_transform.scale);

    let stroke = egui::Stroke::new(width, color);

    for tri in triangles {
        let world: [Vec3; 3] = [
            translation + rotation * (tri[0] * scale),
            translation + rotation * (tri[1] * scale),
            translation + rotation * (tri[2] * scale),
        ];
        let projected: [Option<egui::Pos2>; 3] = [
            project_to_screen(world[0], cam_pos, cam_target, rect, ortho, ortho_scale),
            project_to_screen(world[1], cam_pos, cam_target, rect, ortho, ortho_scale),
            project_to_screen(world[2], cam_pos, cam_target, rect, ortho, ortho_scale),
        ];
        for &(a, b) in &[(0usize, 1usize), (1, 2), (2, 0)] {
            if let (Some(pa), Some(pb)) = (projected[a], projected[b]) {
                painter.line_segment([pa, pb], stroke);
            }
        }
    }
}

/// Same as [`draw_mesh_outline`], but uses the simpler isometric-ish
/// projection that the ECS Inspector's mini 3D view uses (it has its own
/// camera math, no shared `project_to_screen`). The caller supplies a closure
/// that maps a world-space point to a screen position.
pub fn draw_mesh_outline_with<P>(
    painter: &egui::Painter,
    triangles: &[[Vec3; 3]],
    world_transform: &TransformData,
    project: P,
    color: egui::Color32,
    width: f32,
) where
    P: Fn(Vec3) -> egui::Pos2,
{
    let translation = Vec3::from_array(world_transform.translation);
    let rotation = Quat::from_euler(
        bevy::math::EulerRot::XYZ,
        world_transform.rotation_euler[0],
        world_transform.rotation_euler[1],
        world_transform.rotation_euler[2],
    );
    let scale = Vec3::from_array(world_transform.scale);

    let stroke = egui::Stroke::new(width, color);

    for tri in triangles {
        let pa = project(translation + rotation * (tri[0] * scale));
        let pb = project(translation + rotation * (tri[1] * scale));
        let pc = project(translation + rotation * (tri[2] * scale));
        painter.line_segment([pa, pb], stroke);
        painter.line_segment([pb, pc], stroke);
        painter.line_segment([pc, pa], stroke);
    }
}

fn resolve_asset_path(path: &str, project_root: &str) -> String {
    if path.starts_with('/') || path.contains(":\\") {
        return path.to_string();
    }
    let with_assets = format!("{}/assets/{}", project_root, path);
    if std::path::Path::new(&with_assets).exists() {
        with_assets
    } else {
        format!("{}/{}", project_root, path)
    }
}

fn compute_auto_fit_factor(positions: &[[f32; 3]]) -> f32 {
    let mut min = [f32::MAX; 3];
    let mut max = [f32::MIN; 3];
    for p in positions {
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
        1.0
    } else {
        2.0 / extent
    }
}

/// 12 triangles forming a centered cube of side `size`.
fn cube_triangles(size: f32) -> Vec<[Vec3; 3]> {
    let h = size * 0.5;
    let v = [
        Vec3::new(-h, -h, -h),
        Vec3::new(h, -h, -h),
        Vec3::new(h, h, -h),
        Vec3::new(-h, h, -h),
        Vec3::new(-h, -h, h),
        Vec3::new(h, -h, h),
        Vec3::new(h, h, h),
        Vec3::new(-h, h, h),
    ];
    let faces = [
        [0, 1, 2], [0, 2, 3],
        [4, 6, 5], [4, 7, 6],
        [0, 4, 5], [0, 5, 1],
        [2, 6, 7], [2, 7, 3],
        [0, 3, 7], [0, 7, 4],
        [1, 5, 6], [1, 6, 2],
    ];
    faces
        .iter()
        .map(|f| [v[f[0]], v[f[1]], v[f[2]]])
        .collect()
}

fn plane_triangles(size: f32) -> Vec<[Vec3; 3]> {
    let h = size * 0.5;
    let a = Vec3::new(-h, 0.0, -h);
    let b = Vec3::new(h, 0.0, -h);
    let c = Vec3::new(h, 0.0, h);
    let d = Vec3::new(-h, 0.0, h);
    vec![[a, b, c], [a, c, d]]
}

fn sphere_triangles(radius: f32, sectors: usize, stacks: usize) -> Vec<[Vec3; 3]> {
    use std::f32::consts::PI;
    let mut verts = Vec::with_capacity((stacks + 1) * (sectors + 1));
    for i in 0..=stacks {
        let phi = PI / 2.0 - (i as f32) * PI / stacks as f32;
        let xy = radius * phi.cos();
        let z = radius * phi.sin();
        for j in 0..=sectors {
            let theta = (j as f32) * 2.0 * PI / sectors as f32;
            verts.push(Vec3::new(xy * theta.cos(), z, xy * theta.sin()));
        }
    }
    let row = sectors + 1;
    let mut tris = Vec::new();
    for i in 0..stacks {
        for j in 0..sectors {
            let a = i * row + j;
            let b = a + row;
            tris.push([verts[a], verts[a + 1], verts[b]]);
            tris.push([verts[a + 1], verts[b + 1], verts[b]]);
        }
    }
    tris
}
