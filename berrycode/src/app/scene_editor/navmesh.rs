//! NavMesh: grid-based navigation with A* pathfinding.

use super::model::*;
use std::collections::BinaryHeap;
use std::cmp::Ordering;

/// Grid-based navigation mesh.
#[derive(Debug, Clone)]
pub struct NavGrid {
    pub cell_size: f32,
    pub width: usize,
    pub height: usize,
    /// True = walkable, false = blocked.
    pub cells: Vec<bool>,
}

impl NavGrid {
    pub fn new(cell_size: f32, width: usize, height: usize) -> Self {
        Self {
            cell_size,
            width,
            height,
            cells: vec![true; width * height],
        }
    }
}

/// Bake a navigation grid from the scene. Cells overlapping with collider
/// entities are marked as blocked.
pub fn bake_nav_grid(scene: &SceneModel, cell_size: f32) -> NavGrid {
    // Determine grid extents from scene bounds. Use a fixed 100x100 world
    // range centered at origin for simplicity.
    let world_half = 50.0;
    let width = (world_half * 2.0 / cell_size).ceil() as usize;
    let height = width;
    let mut grid = NavGrid::new(cell_size, width, height);

    for (_id, entity) in &scene.entities {
        if !entity.enabled {
            continue;
        }
        let world_t = scene.compute_world_transform(entity.id);
        let pos = world_t.translation;

        for component in &entity.components {
            if let ComponentData::Collider { shape, .. } = component {
                // Compute AABB of the collider in world space
                let (half_x, half_z) = match shape {
                    ColliderShape::Box { half_extents } => {
                        (half_extents[0] * world_t.scale[0], half_extents[2] * world_t.scale[2])
                    }
                    ColliderShape::Sphere { radius } => {
                        let r = radius * world_t.scale[0].max(world_t.scale[2]);
                        (r, r)
                    }
                    ColliderShape::Capsule { half_height, radius } => {
                        let r = radius * world_t.scale[0].max(world_t.scale[2]);
                        let h = half_height * world_t.scale[1];
                        (r, r.max(h))
                    }
                };

                let min_x = pos[0] - half_x;
                let max_x = pos[0] + half_x;
                let min_z = pos[2] - half_z;
                let max_z = pos[2] + half_z;

                // Mark overlapping grid cells as blocked
                let gx_min = ((min_x + world_half) / cell_size).floor().max(0.0) as usize;
                let gx_max = ((max_x + world_half) / cell_size).ceil().min(width as f32) as usize;
                let gz_min = ((min_z + world_half) / cell_size).floor().max(0.0) as usize;
                let gz_max = ((max_z + world_half) / cell_size).ceil().min(height as f32) as usize;

                for gz in gz_min..gz_max {
                    for gx in gx_min..gx_max {
                        if gz < height && gx < width {
                            grid.cells[gz * width + gx] = false;
                        }
                    }
                }
            }
        }
    }

    grid
}

// A* pathfinding

#[derive(Clone, PartialEq)]
struct AStarNode {
    cost: f32,
    heuristic: f32,
    x: usize,
    y: usize,
}

impl Eq for AStarNode {}

impl Ord for AStarNode {
    fn cmp(&self, other: &Self) -> Ordering {
        let self_total = self.cost + self.heuristic;
        let other_total = other.cost + other.heuristic;
        other_total.partial_cmp(&self_total).unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for AStarNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Find a path on the nav grid using A*. Returns world-space waypoints.
pub fn find_path(grid: &NavGrid, start: [f32; 2], goal: [f32; 2]) -> Option<Vec<[f32; 2]>> {
    let world_half = (grid.width as f32 * grid.cell_size) / 2.0;

    // Convert world coords to grid coords
    let sx = ((start[0] + world_half) / grid.cell_size) as usize;
    let sy = ((start[1] + world_half) / grid.cell_size) as usize;
    let gx = ((goal[0] + world_half) / grid.cell_size) as usize;
    let gy = ((goal[1] + world_half) / grid.cell_size) as usize;

    if sx >= grid.width || sy >= grid.height || gx >= grid.width || gy >= grid.height {
        return None;
    }

    if !grid.cells[sy * grid.width + sx] || !grid.cells[gy * grid.width + gx] {
        return None;
    }

    let w = grid.width;
    let h = grid.height;
    let mut costs = vec![f32::MAX; w * h];
    let mut came_from = vec![usize::MAX; w * h];

    let heuristic = |x: usize, y: usize| -> f32 {
        let dx = (x as f32 - gx as f32).abs();
        let dy = (y as f32 - gy as f32).abs();
        dx + dy
    };

    let mut open = BinaryHeap::new();
    costs[sy * w + sx] = 0.0;
    open.push(AStarNode {
        cost: 0.0,
        heuristic: heuristic(sx, sy),
        x: sx,
        y: sy,
    });

    let directions: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

    while let Some(current) = open.pop() {
        if current.x == gx && current.y == gy {
            // Reconstruct path
            let mut path = Vec::new();
            let mut idx = gy * w + gx;
            while idx != usize::MAX {
                let py = idx / w;
                let px = idx % w;
                let wx = px as f32 * grid.cell_size - world_half + grid.cell_size * 0.5;
                let wz = py as f32 * grid.cell_size - world_half + grid.cell_size * 0.5;
                path.push([wx, wz]);
                idx = came_from[idx];
            }
            path.reverse();
            return Some(path);
        }

        let current_cost = current.cost;
        if current_cost > costs[current.y * w + current.x] {
            continue;
        }

        for (dx, dy) in &directions {
            let nx = current.x as i32 + dx;
            let ny = current.y as i32 + dy;
            if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                continue;
            }
            let nx = nx as usize;
            let ny = ny as usize;
            if !grid.cells[ny * w + nx] {
                continue;
            }
            let new_cost = current_cost + 1.0;
            if new_cost < costs[ny * w + nx] {
                costs[ny * w + nx] = new_cost;
                came_from[ny * w + nx] = current.y * w + current.x;
                open.push(AStarNode {
                    cost: new_cost,
                    heuristic: heuristic(nx, ny),
                    x: nx,
                    y: ny,
                });
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_grid_has_path() {
        let grid = NavGrid::new(1.0, 10, 10);
        let path = find_path(&grid, [-4.0, -4.0], [4.0, 4.0]);
        assert!(path.is_some());
    }

    #[test]
    fn blocked_start_returns_none() {
        let mut grid = NavGrid::new(1.0, 10, 10);
        grid.cells[0] = false;
        let path = find_path(&grid, [-4.5, -4.5], [4.0, 4.0]);
        assert!(path.is_none());
    }

    #[test]
    fn bake_empty_scene() {
        let scene = SceneModel::new();
        let grid = bake_nav_grid(&scene, 1.0);
        assert!(grid.cells.iter().all(|&c| c));
    }
}
