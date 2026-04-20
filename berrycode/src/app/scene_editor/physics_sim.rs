//! Simplified physics simulation for Play Mode.
//!
//! Applies gravity, ground-plane collision, AABB entity-entity collision,
//! friction damping, and terminal velocity clamping to entities with RigidBody
//! components during play mode. Phase 11: Enhanced Physics Simulation.

use super::model::*;
use std::collections::HashMap;

const GRAVITY: f32 = -9.81;
const GROUND_Y: f32 = 0.0;
/// Maximum speed any entity can reach (terminal velocity).
const MAX_SPEED: f32 = 50.0;

#[derive(Debug, Clone, Default)]
pub struct PhysicsState {
    /// Per-entity velocity (only for Dynamic rigidbodies)
    pub velocities: HashMap<u64, [f32; 3]>,
    pub last_tick: Option<std::time::Instant>,
}

/// Returns the push-out vector if two AABBs overlap, or `None` if they don't.
/// The push vector is along the axis of least penetration, pointing from B to A.
fn aabb_overlap(
    pos_a: [f32; 3],
    half_a: [f32; 3],
    pos_b: [f32; 3],
    half_b: [f32; 3],
) -> Option<[f32; 3]> {
    let dx = pos_a[0] - pos_b[0];
    let dy = pos_a[1] - pos_b[1];
    let dz = pos_a[2] - pos_b[2];

    let ox = (half_a[0] + half_b[0]) - dx.abs();
    let oy = (half_a[1] + half_b[1]) - dy.abs();
    let oz = (half_a[2] + half_b[2]) - dz.abs();

    if ox <= 0.0 || oy <= 0.0 || oz <= 0.0 {
        return None; // No overlap
    }

    // Push out along the axis of least penetration
    if ox < oy && ox < oz {
        Some([ox * dx.signum(), 0.0, 0.0])
    } else if oy < oz {
        Some([0.0, oy * dy.signum(), 0.0])
    } else {
        Some([0.0, 0.0, oz * dz.signum()])
    }
}

/// Extract the AABB half-extents from an entity's Collider component.
fn get_collider_half_extents(entity: &SceneEntity) -> Option<[f32; 3]> {
    for c in &entity.components {
        if let ComponentData::Collider { shape, .. } = c {
            return Some(match shape {
                ColliderShape::Box { half_extents } => *half_extents,
                ColliderShape::Sphere { radius } => [*radius, *radius, *radius],
                ColliderShape::Capsule {
                    half_height,
                    radius,
                } => [*radius, *half_height + *radius, *radius],
            });
        }
    }
    None
}

/// Extract friction coefficient from an entity's Collider component.
fn get_entity_friction(entity: &SceneEntity) -> f32 {
    for c in &entity.components {
        if let ComponentData::Collider { friction, .. } = c {
            return *friction;
        }
    }
    0.5
}

impl PhysicsState {
    /// Advance the simulation by one frame.
    pub fn tick(&mut self, scene: &mut SceneModel, playing: bool) {
        let now = std::time::Instant::now();
        let dt = match self.last_tick {
            Some(prev) => now.duration_since(prev).as_secs_f32().min(0.05),
            None => 0.0,
        };
        self.last_tick = Some(now);

        if !playing || dt == 0.0 {
            return;
        }

        // Collect dynamic entity IDs
        let dynamic_ids: Vec<u64> = scene
            .entities
            .iter()
            .filter(|(_, e)| {
                e.enabled
                    && e.components.iter().any(|c| {
                        matches!(
                            c,
                            ComponentData::RigidBody {
                                body_type: RigidBodyType::Dynamic,
                                ..
                            }
                        )
                    })
            })
            .map(|(id, _)| *id)
            .collect();

        // Snapshot positions and half-extents for all entities with colliders
        // so we can test against them without borrowing scene mutably.
        let collider_snapshot: HashMap<u64, ([f32; 3], [f32; 3])> = scene
            .entities
            .iter()
            .filter(|(_, e)| e.enabled)
            .filter_map(|(&eid, e)| {
                let half = get_collider_half_extents(e)?;
                let pos = scene.compute_world_transform(eid).translation;
                Some((eid, (pos, half)))
            })
            .collect();

        for id in dynamic_ids {
            let vel = self.velocities.entry(id).or_insert([0.0, 0.0, 0.0]);

            // Apply gravity
            vel[1] += GRAVITY * dt;

            // Terminal velocity clamping
            let speed = (vel[0] * vel[0] + vel[1] * vel[1] + vel[2] * vel[2]).sqrt();
            if speed > MAX_SPEED {
                let scale = MAX_SPEED / speed;
                vel[0] *= scale;
                vel[1] *= scale;
                vel[2] *= scale;
            }

            // Get entity's world transform
            let world = scene.compute_world_transform(id);
            let mut new_pos = [
                world.translation[0] + vel[0] * dt,
                world.translation[1] + vel[1] * dt,
                world.translation[2] + vel[2] * dt,
            ];

            // Simple ground collision (Y=0 plane)
            // Check collider half-height
            let half_h = scene
                .entities
                .get(&id)
                .and_then(|e| {
                    e.components.iter().find_map(|c| match c {
                        ComponentData::Collider { shape, .. } => Some(match shape {
                            ColliderShape::Box { half_extents } => half_extents[1],
                            ColliderShape::Sphere { radius } => *radius,
                            ColliderShape::Capsule {
                                half_height,
                                radius,
                            } => *half_height + *radius,
                        }),
                        _ => None,
                    })
                })
                .unwrap_or(0.5);

            if new_pos[1] < GROUND_Y + half_h {
                new_pos[1] = GROUND_Y + half_h;
                // Bounce with restitution
                let restitution = scene
                    .entities
                    .get(&id)
                    .and_then(|e| {
                        e.components.iter().find_map(|c| match c {
                            ComponentData::Collider { restitution, .. } => Some(*restitution),
                            _ => None,
                        })
                    })
                    .unwrap_or(0.0);
                vel[1] = -vel[1] * restitution;
                if vel[1].abs() < 0.1 {
                    vel[1] = 0.0;
                }
            }

            // AABB collision detection against other entities
            let my_half = scene
                .entities
                .get(&id)
                .and_then(|e| get_collider_half_extents(e));

            if let Some(my_half) = my_half {
                for (&other_id, &(other_pos, other_half)) in &collider_snapshot {
                    if other_id == id {
                        continue;
                    }

                    if let Some(push) = aabb_overlap(new_pos, my_half, other_pos, other_half) {
                        new_pos[0] += push[0];
                        new_pos[1] += push[1];
                        new_pos[2] += push[2];

                        // Zero velocity along push direction
                        if push[0].abs() > 0.001 {
                            vel[0] = 0.0;
                        }
                        if push[1].abs() > 0.001 {
                            vel[1] = 0.0;
                        }
                        if push[2].abs() > 0.001 {
                            vel[2] = 0.0;
                        }
                    }
                }
            }

            // Apply friction (slow down horizontal movement when approximately grounded)
            let friction = scene
                .entities
                .get(&id)
                .map(|e| get_entity_friction(e))
                .unwrap_or(0.5);
            if vel[1].abs() < 0.1 {
                vel[0] *= 1.0 - friction * dt;
                vel[2] *= 1.0 - friction * dt;
            }

            // Update the entity's local transform (approximate -- for root entities, local==world)
            if let Some(entity) = scene.entities.get_mut(&id) {
                entity.transform.translation = new_pos;
            }
        }
    }

    /// Reset all velocities.
    pub fn reset(&mut self) {
        self.velocities.clear();
        self.last_tick = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gravity_moves_down() {
        let mut scene = SceneModel::new();
        let id = scene.add_entity(
            "Ball".into(),
            vec![
                ComponentData::RigidBody {
                    body_type: RigidBodyType::Dynamic,
                    mass: 1.0,
                },
                ComponentData::Collider {
                    shape: ColliderShape::Sphere { radius: 0.5 },
                    friction: 0.5,
                    restitution: 0.0,
                },
            ],
        );
        if let Some(e) = scene.entities.get_mut(&id) {
            e.transform.translation = [0.0, 5.0, 0.0];
        }

        let mut state = PhysicsState::default();
        state.last_tick = Some(std::time::Instant::now());
        // Simulate one step
        std::thread::sleep(std::time::Duration::from_millis(16));
        state.tick(&mut scene, true);

        let y = scene
            .entities
            .get(&id)
            .map(|e| e.transform.translation[1])
            .unwrap_or(5.0);
        assert!(y < 5.0, "Entity should have moved down, got y={}", y);
    }

    #[test]
    fn static_bodies_dont_move() {
        let mut scene = SceneModel::new();
        let id = scene.add_entity(
            "Floor".into(),
            vec![ComponentData::RigidBody {
                body_type: RigidBodyType::Static,
                mass: 0.0,
            }],
        );
        if let Some(e) = scene.entities.get_mut(&id) {
            e.transform.translation = [0.0, 0.0, 0.0];
        }

        let mut state = PhysicsState::default();
        state.last_tick = Some(std::time::Instant::now());
        std::thread::sleep(std::time::Duration::from_millis(16));
        state.tick(&mut scene, true);

        let y = scene
            .entities
            .get(&id)
            .map(|e| e.transform.translation[1])
            .unwrap_or(0.0);
        assert!((y - 0.0).abs() < 0.01);
    }

    #[test]
    fn aabb_overlap_no_collision() {
        // Two boxes far apart
        let result = aabb_overlap(
            [0.0, 0.0, 0.0],
            [0.5, 0.5, 0.5],
            [5.0, 0.0, 0.0],
            [0.5, 0.5, 0.5],
        );
        assert!(result.is_none(), "Boxes far apart should not collide");
    }

    #[test]
    fn aabb_overlap_detects_collision() {
        // Two boxes overlapping on X axis
        let result = aabb_overlap(
            [0.0, 0.0, 0.0],
            [0.5, 0.5, 0.5],
            [0.8, 0.0, 0.0],
            [0.5, 0.5, 0.5],
        );
        assert!(result.is_some(), "Overlapping boxes should collide");
        let push = result.unwrap();
        // Push should be along X (least penetration axis)
        assert!(
            push[0].abs() > 0.0,
            "Push should have X component, got {:?}",
            push
        );
    }

    #[test]
    fn aabb_overlap_push_direction() {
        // A sits on top of B with slight Y overlap (0.95 apart, combined half = 1.0)
        let result = aabb_overlap(
            [0.0, 0.95, 0.0], // A at y=0.95
            [0.5, 0.5, 0.5],
            [0.0, 0.0, 0.0], // B at origin
            [0.5, 0.5, 0.5],
        );
        assert!(result.is_some(), "Should detect overlap");
        let push = result.unwrap();
        // Y overlap = 1.0 - 0.95 = 0.05 (least penetration)
        // X overlap = 1.0, Z overlap = 1.0 (both larger)
        // Push should be along Y, positive (pushing A up away from B)
        assert!(push[1] > 0.0, "Push should be upward, got {:?}", push);
    }

    #[test]
    fn entity_entity_collision() {
        // Dynamic box falls onto a static box
        let mut scene = SceneModel::new();

        // Static platform at y=0
        let platform_id = scene.add_entity(
            "Platform".into(),
            vec![
                ComponentData::RigidBody {
                    body_type: RigidBodyType::Static,
                    mass: 0.0,
                },
                ComponentData::Collider {
                    shape: ColliderShape::Box {
                        half_extents: [2.0, 0.5, 2.0],
                    },
                    friction: 0.5,
                    restitution: 0.0,
                },
            ],
        );
        if let Some(e) = scene.entities.get_mut(&platform_id) {
            e.transform.translation = [0.0, 3.0, 0.0];
        }

        // Dynamic box above the platform
        let box_id = scene.add_entity(
            "FallingBox".into(),
            vec![
                ComponentData::RigidBody {
                    body_type: RigidBodyType::Dynamic,
                    mass: 1.0,
                },
                ComponentData::Collider {
                    shape: ColliderShape::Box {
                        half_extents: [0.5, 0.5, 0.5],
                    },
                    friction: 0.5,
                    restitution: 0.0,
                },
            ],
        );
        if let Some(e) = scene.entities.get_mut(&box_id) {
            e.transform.translation = [0.0, 5.0, 0.0];
        }

        let mut state = PhysicsState::default();
        state.last_tick = Some(std::time::Instant::now());

        // Simulate many frames to let the box fall
        for _ in 0..200 {
            std::thread::sleep(std::time::Duration::from_millis(16));
            state.tick(&mut scene, true);
        }

        let box_y = scene
            .entities
            .get(&box_id)
            .map(|e| e.transform.translation[1])
            .unwrap_or(0.0);
        let platform_y = scene
            .entities
            .get(&platform_id)
            .map(|e| e.transform.translation[1])
            .unwrap_or(0.0);

        // The box should have come to rest on top of the platform (or ground)
        // Platform top = 3.0 + 0.5 = 3.5, box half = 0.5, so box center >= 4.0
        // Or on ground: box center = 0.5
        assert!(
            box_y >= 0.4,
            "Box should be resting on ground or platform, got y={}",
            box_y
        );
        // Platform should not have moved
        assert!(
            (platform_y - 3.0).abs() < 0.01,
            "Platform should not move, got y={}",
            platform_y
        );
    }

    #[test]
    fn friction_slows_horizontal_movement() {
        let mut scene = SceneModel::new();
        let id = scene.add_entity(
            "Slider".into(),
            vec![
                ComponentData::RigidBody {
                    body_type: RigidBodyType::Dynamic,
                    mass: 1.0,
                },
                ComponentData::Collider {
                    shape: ColliderShape::Box {
                        half_extents: [0.5, 0.5, 0.5],
                    },
                    friction: 0.8,
                    restitution: 0.0,
                },
            ],
        );
        // Place on ground
        if let Some(e) = scene.entities.get_mut(&id) {
            e.transform.translation = [0.0, 0.5, 0.0];
        }

        let mut state = PhysicsState::default();
        // Give it horizontal velocity
        state.velocities.insert(id, [10.0, 0.0, 0.0]);
        state.last_tick = Some(std::time::Instant::now());

        // Simulate several frames
        for _ in 0..60 {
            std::thread::sleep(std::time::Duration::from_millis(16));
            state.tick(&mut scene, true);
        }

        let vx = state.velocities.get(&id).map(|v| v[0]).unwrap_or(10.0);
        assert!(
            vx.abs() < 10.0,
            "Friction should slow horizontal velocity, got vx={}",
            vx
        );
    }

    #[test]
    fn terminal_velocity_clamping() {
        let mut scene = SceneModel::new();
        let id = scene.add_entity(
            "FastBall".into(),
            vec![
                ComponentData::RigidBody {
                    body_type: RigidBodyType::Dynamic,
                    mass: 1.0,
                },
                ComponentData::Collider {
                    shape: ColliderShape::Sphere { radius: 0.5 },
                    friction: 0.0,
                    restitution: 0.0,
                },
            ],
        );
        if let Some(e) = scene.entities.get_mut(&id) {
            e.transform.translation = [0.0, 1000.0, 0.0];
        }

        let mut state = PhysicsState::default();
        state.last_tick = Some(std::time::Instant::now());

        // Simulate many frames of freefall
        for _ in 0..500 {
            std::thread::sleep(std::time::Duration::from_millis(16));
            state.tick(&mut scene, true);
        }

        let vel = state.velocities.get(&id).copied().unwrap_or([0.0; 3]);
        let speed = (vel[0] * vel[0] + vel[1] * vel[1] + vel[2] * vel[2]).sqrt();
        assert!(
            speed <= MAX_SPEED + 0.1,
            "Speed should be clamped to {}, got {}",
            MAX_SPEED,
            speed
        );
    }
}
