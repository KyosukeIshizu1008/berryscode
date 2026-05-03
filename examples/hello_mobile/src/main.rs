//! Hello Mobile — BerryCode v0.7.5 reference sample.
//!
//! Mirrors the runtime shape BerryCode emits into a real project so
//! you can see what the editor will produce *before* dropping into
//! the editor. Demonstrates:
//!
//! - PlayerController with arrow-key + on-screen virtual button input
//! - Avian physics body + collider that the editor's Inspector can author
//! - AnimatorParams driven by both keys and a TouchInputZone
//! - Top-down follow camera that the v0.6.x display profile preview matches
//!
//! Open this folder as a BerryCode project (`File → Open Folder`) to
//! edit the scene visually. Run with `cargo run -p hello_mobile`.

use avian3d::prelude::*;
use bevy::prelude::*;
use std::collections::HashMap;

// ─── Component types BerryCode emits into `src/scenes/mod.rs` ──────────
// In a real BerryCode project these come from `use scenes::*` — duplicated
// inline here so the sample compiles standalone.

#[derive(Component, Debug, Clone)]
pub struct PlayerController {
    pub speed: f32,
    pub jump_velocity: f32,
    pub run_multiplier: f32,
    pub turn_speed: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchActionKind {
    Hold,
    Trigger,
}

#[derive(Component, Debug, Clone)]
pub struct TouchInputZone {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub parameter_name: String,
    pub action_kind: TouchActionKind,
    pub label: String,
    pub was_inside: bool,
}

#[derive(Debug, Clone)]
pub enum AnimParamValue {
    Bool(bool),
    Float(f32),
    Int(i64),
    Trigger(bool),
}

#[derive(Component, Default)]
pub struct AnimatorParams {
    pub values: HashMap<String, AnimParamValue>,
}

impl AnimatorParams {
    pub fn set_bool(&mut self, name: &str, v: bool) {
        self.values.insert(name.to_string(), AnimParamValue::Bool(v));
    }
    pub fn get_bool(&self, name: &str) -> bool {
        matches!(self.values.get(name), Some(AnimParamValue::Bool(true)))
    }
    pub fn trigger(&mut self, name: &str) {
        self.values
            .insert(name.to_string(), AnimParamValue::Trigger(true));
    }
}

#[derive(Component)]
struct FollowCamera;

const CAMERA_OFFSET: Vec3 = Vec3::new(0.0, 12.0, 8.0);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(PhysicsPlugins::default())
        .add_systems(Startup, (setup_world, setup_player, setup_camera))
        .add_systems(
            Update,
            (
                drive_animator_params,
                player_movement,
                touch_input_evaluate,
                follow_camera.after(player_movement),
            ),
        )
        .run();
}

fn setup_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        DirectionalLight {
            illuminance: 12000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -1.0, 0.5, 0.0)),
    ));
    // Ground.
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(50.0, 50.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
        RigidBody::Static,
        Collider::cuboid(50.0, 0.1, 50.0),
    ));
}

fn setup_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Player capsule.
    commands.spawn((
        Mesh3d(meshes.add(Capsule3d::new(0.4, 0.9))),
        MeshMaterial3d(materials.add(Color::srgb(0.9, 0.6, 0.2))),
        Transform::from_xyz(0.0, 1.0, 0.0),
        Name::new("Player"),
        RigidBody::Dynamic,
        Collider::capsule(0.4, 0.9),
        LockedAxes::ROTATION_LOCKED,
        Friction::new(0.0),
        PlayerController {
            speed: 5.0,
            jump_velocity: 8.0,
            run_multiplier: 2.0,
            turn_speed: 12.0,
        },
        AnimatorParams::default(),
    ));

    // On-screen virtual jump button (bottom-right) — Trigger action so
    // each tap fires once.
    commands.spawn(TouchInputZone {
        x: 0.78,
        y: 0.78,
        w: 0.18,
        h: 0.12,
        parameter_name: "jump".into(),
        action_kind: TouchActionKind::Trigger,
        label: "Jump".into(),
        was_inside: false,
    });
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(CAMERA_OFFSET).looking_at(Vec3::ZERO, Vec3::Y),
        FollowCamera,
    ));
}

fn drive_animator_params(
    keys: Res<ButtonInput<KeyCode>>,
    mut player_q: Query<&mut AnimatorParams, With<PlayerController>>,
) {
    let moving = keys.pressed(KeyCode::ArrowUp)
        || keys.pressed(KeyCode::ArrowDown)
        || keys.pressed(KeyCode::ArrowLeft)
        || keys.pressed(KeyCode::ArrowRight);
    if let Ok(mut params) = player_q.single_mut() {
        params.set_bool("isMoving", moving);
    }
}

fn player_movement(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut player_q: Query<(&mut LinearVelocity, &mut Transform, &PlayerController, &AnimatorParams)>,
) {
    let Ok((mut vel, mut transform, ctrl, params)) = player_q.single_mut() else {
        return;
    };
    let mut dir = Vec3::ZERO;
    if keys.pressed(KeyCode::ArrowUp) {
        dir.z -= 1.0;
    }
    if keys.pressed(KeyCode::ArrowDown) {
        dir.z += 1.0;
    }
    if keys.pressed(KeyCode::ArrowLeft) {
        dir.x -= 1.0;
    }
    if keys.pressed(KeyCode::ArrowRight) {
        dir.x += 1.0;
    }
    if dir.length_squared() > 0.0 {
        dir = dir.normalize();
    }
    let speed = if keys.pressed(KeyCode::ShiftLeft) {
        ctrl.speed * ctrl.run_multiplier
    } else {
        ctrl.speed
    };
    vel.x = dir.x * speed;
    vel.z = dir.z * speed;
    if ctrl.turn_speed > 0.0 && dir.length_squared() > 0.0 {
        let target_yaw = dir.x.atan2(dir.z);
        let target_rot = Quat::from_axis_angle(Vec3::Y, target_yaw);
        let t = (ctrl.turn_speed * time.delta_secs()).clamp(0.0, 1.0);
        transform.rotation = transform.rotation.slerp(target_rot, t);
    }
    let on_ground = transform.translation.y < 1.0;
    let space_jump = keys.just_pressed(KeyCode::Space);
    let touch_jump = matches!(
        params.values.get("jump"),
        Some(AnimParamValue::Trigger(true))
    );
    if (space_jump || touch_jump) && on_ground {
        vel.y = ctrl.jump_velocity;
    }
}

fn touch_input_evaluate(
    touches: Res<bevy::input::touch::Touches>,
    mouse_btn: Res<ButtonInput<MouseButton>>,
    window_q: Query<&Window>,
    mut zones_q: Query<&mut TouchInputZone>,
    mut params_q: Query<&mut AnimatorParams, With<PlayerController>>,
) {
    let Ok(window) = window_q.single() else {
        return;
    };
    let win_w = window.width();
    let win_h = window.height();
    if win_w <= 0.0 || win_h <= 0.0 {
        return;
    }
    let mut points: Vec<(f32, f32)> = touches
        .iter()
        .map(|t| (t.position().x, t.position().y))
        .collect();
    if mouse_btn.pressed(MouseButton::Left) {
        if let Some(p) = window.cursor_position() {
            points.push((p.x, p.y));
        }
    }
    let Ok(mut params) = params_q.single_mut() else {
        return;
    };
    for mut zone in &mut zones_q {
        if zone.parameter_name.is_empty() {
            continue;
        }
        let zx = zone.x * win_w;
        let zy = zone.y * win_h;
        let zw = zone.w * win_w;
        let zh = zone.h * win_h;
        let inside = points
            .iter()
            .any(|(px, py)| *px >= zx && *px <= zx + zw && *py >= zy && *py <= zy + zh);
        match zone.action_kind {
            TouchActionKind::Hold => {
                let name = zone.parameter_name.clone();
                params.set_bool(&name, inside);
            }
            TouchActionKind::Trigger => {
                if inside && !zone.was_inside {
                    let name = zone.parameter_name.clone();
                    params.trigger(&name);
                }
            }
        }
        zone.was_inside = inside;
    }
}

fn follow_camera(
    player_q: Query<&Transform, (With<PlayerController>, Without<FollowCamera>)>,
    mut camera_q: Query<&mut Transform, With<FollowCamera>>,
) {
    let Ok(player_t) = player_q.single() else {
        return;
    };
    let Ok(mut cam_t) = camera_q.single_mut() else {
        return;
    };
    let target = player_t.translation;
    cam_t.translation = target + CAMERA_OFFSET;
    cam_t.look_at(target, Vec3::Y);
}
