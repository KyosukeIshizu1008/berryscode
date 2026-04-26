#![allow(dead_code)]
//! NavMesh: grid-based navigation with A* pathfinding.

use super::model::*;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BinaryHeap;

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
                    ColliderShape::Box { half_extents } => (
                        half_extents[0] * world_t.scale[0],
                        half_extents[2] * world_t.scale[2],
                    ),
                    ColliderShape::Sphere { radius } => {
                        let r = radius * world_t.scale[0].max(world_t.scale[2]);
                        (r, r)
                    }
                    ColliderShape::Capsule {
                        half_height,
                        radius,
                    } => {
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
        other_total
            .partial_cmp(&self_total)
            .unwrap_or(Ordering::Equal)
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

/// Configuration for a navigation mesh agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavMeshAgent {
    pub speed: f32,
    pub radius: f32,
    pub height: f32,
    pub max_slope: f32,
    pub auto_path: bool,
}

impl Default for NavMeshAgent {
    fn default() -> Self {
        Self {
            speed: 3.5,
            radius: 0.5,
            height: 2.0,
            max_slope: 0.785,
            auto_path: true,
        }
    }
}

/// A colored line segment for path visualization.
#[derive(Debug, Clone)]
pub struct PathSegment {
    pub start: [f32; 2],
    pub end: [f32; 2],
    pub color: [f32; 4],
}

/// Render an agent's path on the navmesh grid as colored line segments.
/// Given start and end positions (world XZ), compute the shortest A* path and
/// return a list of line segments for visualization.
///
/// The path is colored with a gradient from green (start) to red (end).
pub fn render_navmesh_agent_preview(
    grid: &NavGrid,
    start: [f32; 2],
    end: [f32; 2],
) -> Vec<PathSegment> {
    let path = match find_path(grid, start, end) {
        Some(p) => p,
        None => return vec![],
    };

    if path.len() < 2 {
        return vec![];
    }

    let total = (path.len() - 1) as f32;
    let mut segments = Vec::with_capacity(path.len() - 1);

    for i in 0..path.len() - 1 {
        let t = i as f32 / total;
        // Gradient: green -> red
        let color = [t, 1.0 - t, 0.0, 1.0];
        segments.push(PathSegment {
            start: path[i],
            end: path[i + 1],
            color,
        });
    }

    segments
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

    #[test]
    fn astar_straight_line_path() {
        let grid = NavGrid::new(1.0, 10, 10);
        let path = find_path(&grid, [-4.5, 0.5], [4.5, 0.5]).unwrap();
        // Path should go from left to right, all y values should be the same row
        assert!(path.len() >= 2);
        // First waypoint near start, last near goal
        assert!((path[0][0] - (-4.5)).abs() < 1.0);
        assert!((path.last().unwrap()[0] - 4.5).abs() < 1.0);
    }

    #[test]
    fn astar_path_around_obstacle() {
        let mut grid = NavGrid::new(1.0, 10, 10);
        // Block a vertical wall in the middle (column 5), leaving a gap at row 0
        for row in 1..10 {
            grid.cells[row * 10 + 5] = false;
        }
        // Start at column 0, row 5; goal at column 6, row 5 (past the wall)
        let path = find_path(&grid, [-4.5, 0.5], [1.5, 0.5]);
        assert!(path.is_some(), "Should find a path around the wall");
        let path = path.unwrap();
        assert!(
            path.len() > 2,
            "Path around obstacle should have multiple waypoints"
        );
    }

    #[test]
    fn astar_no_path_fully_blocked() {
        let mut grid = NavGrid::new(1.0, 10, 10);
        // Block entire column 5, completely splitting the grid
        for row in 0..10 {
            grid.cells[row * 10 + 5] = false;
        }
        let path = find_path(&grid, [-4.5, 0.5], [0.5, 0.5]);
        assert!(path.is_none(), "Should return None when fully blocked");
    }

    #[test]
    fn navmesh_agent_default() {
        let agent = NavMeshAgent::default();
        assert!((agent.speed - 3.5).abs() < 1e-6);
        assert!((agent.radius - 0.5).abs() < 1e-6);
        assert!((agent.height - 2.0).abs() < 1e-6);
        assert!(agent.auto_path);
    }

    #[test]
    fn render_preview_empty_on_no_path() {
        let mut grid = NavGrid::new(1.0, 10, 10);
        grid.cells[0] = false; // block start
        let segments = render_navmesh_agent_preview(&grid, [-4.5, -4.5], [4.0, 4.0]);
        assert!(segments.is_empty());
    }

    #[test]
    fn render_preview_produces_segments() {
        let grid = NavGrid::new(1.0, 10, 10);
        let segments = render_navmesh_agent_preview(&grid, [-4.0, -4.0], [4.0, 4.0]);
        assert!(!segments.is_empty());
        // First segment should be green-ish, last should be red-ish
        let first = &segments[0];
        assert!(
            first.color[1] > first.color[0],
            "First segment should be more green"
        );
        let last = segments.last().unwrap();
        assert!(
            last.color[0] > last.color[1],
            "Last segment should be more red"
        );
    }

    #[test]
    fn astar_start_equals_goal() {
        let grid = NavGrid::new(1.0, 10, 10);
        let path = find_path(&grid, [0.0, 0.0], [0.0, 0.0]);
        assert!(path.is_some());
        let path = path.unwrap();
        assert_eq!(path.len(), 1, "Start==goal should return a single waypoint");
    }
}
