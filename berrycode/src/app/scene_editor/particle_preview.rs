//! Live editor-side preview of particle emitters.
//!
//! Maintains per-entity particle state (positions, velocities, ages) and
//! advances it each frame. Renders particles as filled dots overlay on top of
//! the Scene View using egui's painter.
//!
//! This is intentionally decoupled from any runtime particle library (e.g.
//! `bevy_hanabi`): the simulation here exists solely so the editor can give
//! the author immediate visual feedback on the emitter parameters they are
//! tweaking. A runtime game is expected to read the same `ComponentData` and
//! feed it to its own simulation.

use super::model::*;
use bevy::math::Vec3;
use std::collections::HashMap;

/// One live particle in the preview simulation.
#[derive(Debug, Clone, Copy)]
struct Particle {
    pos: Vec3,
    vel: Vec3,
    age: f32,
    lifetime: f32,
}

/// Per-emitter simulation state (live particles + fractional spawn carry).
#[derive(Debug, Clone, Default)]
pub struct EmitterRuntime {
    particles: Vec<Particle>,
    spawn_accumulator: f32,
}

/// Root container: one `EmitterRuntime` per scene entity that has a
/// `ParticleEmitter` component. Keyed by scene entity id.
#[derive(Debug, Clone, Default)]
pub struct ParticlePreview {
    pub emitters: HashMap<u64, EmitterRuntime>,
    pub last_update: Option<std::time::Instant>,
}

impl ParticlePreview {
    /// Advance simulation for all emitters in `scene` using elapsed time since
    /// the last call (capped to a reasonable max so paused frames don't burst).
    pub fn tick(&mut self, scene: &SceneModel) {
        let now = std::time::Instant::now();
        let dt = match self.last_update {
            Some(prev) => now.duration_since(prev).as_secs_f32().min(0.1),
            None => 0.0,
        };
        self.last_update = Some(now);

        // Drop emitters whose entity no longer exists.
        self.emitters.retain(|id, _| scene.entities.contains_key(id));

        for (id, entity) in &scene.entities {
            for component in &entity.components {
                if let ComponentData::ParticleEmitter {
                    rate,
                    lifetime,
                    speed,
                    spread,
                    max_particles,
                    gravity,
                    ..
                } = component
                {
                    let runtime = self.emitters.entry(*id).or_default();

                    // Advance existing particles.
                    for p in &mut runtime.particles {
                        p.vel.y += gravity * dt;
                        p.pos += p.vel * dt;
                        p.age += dt;
                    }
                    runtime.particles.retain(|p| p.age < p.lifetime);

                    // Spawn new particles.
                    runtime.spawn_accumulator += rate * dt;
                    let max = *max_particles as usize;
                    let world_t = scene.compute_world_transform(*id);
                    let origin = Vec3::from_array(world_t.translation);
                    while runtime.spawn_accumulator >= 1.0 && runtime.particles.len() < max {
                        runtime.spawn_accumulator -= 1.0;
                        let (rx, ry, rz) =
                            pseudo_random_dir(runtime.particles.len() as u32, *id);
                        let lateral = Vec3::new(rx, 0.0, rz) * *spread;
                        let vel = Vec3::new(0.0, *speed, 0.0)
                            + lateral * *speed
                            + Vec3::new(0.0, ry * *spread, 0.0);
                        runtime.particles.push(Particle {
                            pos: origin,
                            vel,
                            age: 0.0,
                            lifetime: *lifetime,
                        });
                    }
                }
            }
        }
    }

    /// Iterate live particles for an entity along with the emitter's params, so
    /// the renderer can compute per-particle color/size based on age.
    pub fn for_each_particle<F: FnMut(Vec3, f32, &ComponentData)>(
        &self,
        scene: &SceneModel,
        mut f: F,
    ) {
        for (id, runtime) in &self.emitters {
            let entity = match scene.entities.get(id) {
                Some(e) => e,
                None => continue,
            };
            let component = entity
                .components
                .iter()
                .find(|c| matches!(c, ComponentData::ParticleEmitter { .. }));
            let component = match component {
                Some(c) => c,
                None => continue,
            };
            for p in &runtime.particles {
                let t = (p.age / p.lifetime).clamp(0.0, 1.0);
                f(p.pos, t, component);
            }
        }
    }
}

/// Cheap, stateless pseudo-random vector in approx unit cube — good enough for
/// jittering particle spawn directions. Not crypto-anything; just visual noise.
fn pseudo_random_dir(seed_a: u32, seed_b: u64) -> (f32, f32, f32) {
    fn hash(mut x: u64) -> u64 {
        x ^= x >> 33;
        x = x.wrapping_mul(0xff51afd7ed558ccd);
        x ^= x >> 33;
        x = x.wrapping_mul(0xc4ceb9fe1a85ec53);
        x ^= x >> 33;
        x
    }
    let a = hash((seed_a as u64).wrapping_add(seed_b.wrapping_mul(0x9e3779b97f4a7c15)));
    let b = hash(a.wrapping_add(0xdeadbeef));
    let c = hash(b.wrapping_add(0xfeedface));
    let to_unit = |v: u64| ((v as u32) as f32 / u32::MAX as f32) * 2.0 - 1.0;
    (to_unit(a), to_unit(b).abs() * 0.5, to_unit(c)) // y bias upward
}
