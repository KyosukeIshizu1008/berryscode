//! Terrain system: grid-based heightmap with editing helpers.

/// Bilinear interpolation of height at world position (x, z).
pub fn height_at(heights: &[f32], resolution: u32, world_size: [f32; 2], x: f32, z: f32) -> f32 {
    if resolution == 0 || heights.is_empty() {
        return 0.0;
    }
    let res = resolution as f32;
    // Normalize x, z to grid coordinates
    let gx = ((x / world_size[0] + 0.5) * res).clamp(0.0, res - 1.001);
    let gz = ((z / world_size[1] + 0.5) * res).clamp(0.0, res - 1.001);
    let ix = gx as usize;
    let iz = gz as usize;
    let fx = gx - ix as f32;
    let fz = gz - iz as f32;
    let r = resolution as usize;
    let h00 = heights.get(iz * r + ix).copied().unwrap_or(0.0);
    let h10 = heights
        .get(iz * r + (ix + 1).min(r - 1))
        .copied()
        .unwrap_or(0.0);
    let h01 = heights
        .get((iz + 1).min(r - 1) * r + ix)
        .copied()
        .unwrap_or(0.0);
    let h11 = heights
        .get((iz + 1).min(r - 1) * r + (ix + 1).min(r - 1))
        .copied()
        .unwrap_or(0.0);
    let h0 = h00 + (h10 - h00) * fx;
    let h1 = h01 + (h11 - h01) * fx;
    h0 + (h1 - h0) * fz
}

/// Compute approximate normal at a grid position.
pub fn normal_at(
    heights: &[f32],
    resolution: u32,
    world_size: [f32; 2],
    x: f32,
    z: f32,
) -> [f32; 3] {
    let step_x = world_size[0] / resolution as f32;
    let step_z = world_size[1] / resolution as f32;
    let hx0 = height_at(heights, resolution, world_size, x - step_x, z);
    let hx1 = height_at(heights, resolution, world_size, x + step_x, z);
    let hz0 = height_at(heights, resolution, world_size, x, z - step_z);
    let hz1 = height_at(heights, resolution, world_size, x, z + step_z);
    let dx = hx1 - hx0;
    let dz = hz1 - hz0;
    let len = (dx * dx + 1.0 + dz * dz).sqrt();
    [-dx / len, 1.0 / len, -dz / len]
}

/// Generate a Bevy `Mesh` from a height grid.
pub fn generate_terrain_mesh(
    heights: &[f32],
    resolution: u32,
    world_size: [f32; 2],
) -> bevy::prelude::Mesh {
    use bevy::mesh::{Indices, PrimitiveTopology};
    use bevy::prelude::Mesh;

    let res = resolution as usize;
    let vertex_count = res * res;
    let mut positions = Vec::with_capacity(vertex_count);
    let mut normals = Vec::with_capacity(vertex_count);
    let mut uvs = Vec::with_capacity(vertex_count);

    for iz in 0..res {
        for ix in 0..res {
            let fx = ix as f32 / (res - 1).max(1) as f32;
            let fz = iz as f32 / (res - 1).max(1) as f32;
            let wx = (fx - 0.5) * world_size[0];
            let wz = (fz - 0.5) * world_size[1];
            let h = heights.get(iz * res + ix).copied().unwrap_or(0.0);
            positions.push([wx, h, wz]);
            uvs.push([fx, fz]);

            let n = normal_at(heights, resolution, world_size, wx, wz);
            normals.push(n);
        }
    }

    // Build triangle indices (two triangles per quad)
    let quad_count = (res - 1) * (res - 1);
    let mut indices = Vec::with_capacity(quad_count * 6);
    for iz in 0..(res - 1) {
        for ix in 0..(res - 1) {
            let i00 = (iz * res + ix) as u32;
            let i10 = (iz * res + ix + 1) as u32;
            let i01 = ((iz + 1) * res + ix) as u32;
            let i11 = ((iz + 1) * res + ix + 1) as u32;
            indices.push(i00);
            indices.push(i01);
            indices.push(i10);
            indices.push(i10);
            indices.push(i01);
            indices.push(i11);
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// Terrain brush editing mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrushMode {
    Raise,
    Lower,
    Smooth,
    Flatten,
}

impl BrushMode {
    pub const ALL: &'static [BrushMode] = &[
        BrushMode::Raise,
        BrushMode::Lower,
        BrushMode::Smooth,
        BrushMode::Flatten,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            BrushMode::Raise => "Raise",
            BrushMode::Lower => "Lower",
            BrushMode::Smooth => "Smooth",
            BrushMode::Flatten => "Flatten",
        }
    }
}

/// Editor-side terrain brush state.
#[derive(Debug, Clone)]
pub struct TerrainBrushState {
    pub active: bool,
    pub mode: BrushMode,
    pub radius: f32,
    pub strength: f32,
}

impl Default for TerrainBrushState {
    fn default() -> Self {
        Self {
            active: false,
            mode: BrushMode::Raise,
            radius: 5.0,
            strength: 1.0,
        }
    }
}

/// Apply a terrain brush stroke at (center_x, center_z) in world space.
pub fn apply_brush(
    heights: &mut [f32],
    resolution: u32,
    world_size: [f32; 2],
    center_x: f32,
    center_z: f32,
    radius: f32,
    strength: f32,
    mode: BrushMode,
) {
    if resolution == 0 || heights.is_empty() {
        return;
    }
    let res = resolution as usize;

    // Flatten target: sample height at center before modification.
    let flatten_target = match mode {
        BrushMode::Flatten => height_at(heights, resolution, world_size, center_x, center_z),
        _ => 0.0,
    };

    for iz in 0..res {
        for ix in 0..res {
            // Convert grid coords to world coords
            let wx = (ix as f32 / resolution as f32 - 0.5) * world_size[0];
            let wz = (iz as f32 / resolution as f32 - 0.5) * world_size[1];

            let dx = wx - center_x;
            let dz = wz - center_z;
            let dist = (dx * dx + dz * dz).sqrt();

            if dist > radius {
                continue;
            }

            // Falloff: 1 at center, 0 at edge
            let falloff = 1.0 - (dist / radius);
            let idx = iz * res + ix;

            match mode {
                BrushMode::Raise => {
                    heights[idx] += strength * falloff * 0.1;
                }
                BrushMode::Lower => {
                    heights[idx] -= strength * falloff * 0.1;
                }
                BrushMode::Smooth => {
                    // Average with neighbors
                    let mut sum = 0.0f32;
                    let mut count = 0u32;
                    for diz in -1i32..=1 {
                        for dix in -1i32..=1 {
                            let nz = iz as i32 + diz;
                            let nx = ix as i32 + dix;
                            if nz >= 0 && nz < res as i32 && nx >= 0 && nx < res as i32 {
                                sum += heights[nz as usize * res + nx as usize];
                                count += 1;
                            }
                        }
                    }
                    if count > 0 {
                        let avg = sum / count as f32;
                        heights[idx] += (avg - heights[idx]) * falloff * strength * 0.1;
                    }
                }
                BrushMode::Flatten => {
                    let diff = flatten_target - heights[idx];
                    heights[idx] += diff * falloff * strength * 0.1;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_terrain_height_zero() {
        let heights = vec![0.0; 16];
        assert!((height_at(&heights, 4, [10.0, 10.0], 0.0, 0.0) - 0.0).abs() < 0.01);
    }

    #[test]
    fn elevated_terrain() {
        let mut heights = vec![0.0; 16];
        heights[0] = 5.0; // corner
        let h = height_at(&heights, 4, [10.0, 10.0], -5.0, -5.0);
        assert!((h - 5.0).abs() < 0.1);
    }

    #[test]
    fn normal_on_flat_points_up() {
        let heights = vec![0.0; 16];
        let n = normal_at(&heights, 4, [10.0, 10.0], 0.0, 0.0);
        assert!(n[1] > 0.99); // should point mostly up
    }
}
