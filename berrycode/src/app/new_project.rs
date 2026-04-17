//! New Bevy project creation with templates

use super::BerryCodeApp;

/// Available project templates
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectTemplate {
    Empty2D,
    Empty3D,
    Walker3D,
    Plugin,
}

impl ProjectTemplate {
    pub const ALL: &'static [ProjectTemplate] = &[
        ProjectTemplate::Empty2D,
        ProjectTemplate::Empty3D,
        ProjectTemplate::Walker3D,
        ProjectTemplate::Plugin,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            ProjectTemplate::Empty2D => "Empty 2D",
            ProjectTemplate::Empty3D => "Empty 3D",
            ProjectTemplate::Walker3D => "3D Walker (FPS controller)",
            ProjectTemplate::Plugin => "Plugin",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            ProjectTemplate::Empty2D => "Minimal 2D project with Camera2d",
            ProjectTemplate::Empty3D => "Minimal 3D project with Camera3d, light and a cube",
            ProjectTemplate::Walker3D => "First-person walker in a 3D world (WASD + mouse look)",
            ProjectTemplate::Plugin => "A reusable Bevy Plugin",
        }
    }
}

impl BerryCodeApp {
    /// Render the "New Bevy Project" dialog
    pub(crate) fn render_new_project_dialog(&mut self, ctx: &egui::Context) {
        if !self.new_project_dialog_open {
            return;
        }

        egui::Window::new("New Bevy Project")
            .collapsible(false)
            .resizable(false)
            .default_width(500.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.label("Project name:");
                    ui.add_sized([300.0, 20.0], egui::TextEdit::singleline(&mut self.new_project_name));
                });

                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label("Location:    ");
                    ui.add_sized([300.0, 20.0], egui::TextEdit::singleline(&mut self.new_project_path));
                });

                ui.add_space(8.0);
                ui.separator();
                ui.label("Template:");
                ui.add_space(4.0);

                for tpl in ProjectTemplate::ALL {
                    let selected = self.new_project_template == *tpl;
                    let response = ui.selectable_label(
                        selected,
                        egui::RichText::new(format!("  {}", tpl.label())).strong(),
                    );
                    if response.clicked() {
                        self.new_project_template = *tpl;
                    }
                    ui.horizontal(|ui| {
                        ui.add_space(20.0);
                        ui.label(
                            egui::RichText::new(tpl.description())
                                .size(11.0)
                                .color(egui::Color32::from_rgb(150, 150, 150)),
                        );
                    });
                    ui.add_space(2.0);
                }

                ui.separator();

                let full_path = format!("{}/{}", self.new_project_path, self.new_project_name);
                ui.label(format!("Will create: {}", full_path));

                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    let name_valid = !self.new_project_name.is_empty()
                        && self.new_project_name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-');

                    if ui.add_enabled(name_valid, egui::Button::new("Create Project")).clicked() {
                        match Self::create_bevy_project(&full_path, &self.new_project_name, self.new_project_template) {
                            Ok(_) => {
                                self.status_message = format!("Bevy project created: {}", full_path);
                                self.status_message_timestamp = Some(std::time::Instant::now());

                                self.root_path = full_path;
                                self.file_tree_cache.clear();
                                self.file_tree_load_pending = true;
                                self.expanded_dirs.clear();
                                self.editor_tabs.clear();
                                self.active_tab_idx = 0;
                                self.git_initialized = false;

                                if let Ok(mut watcher) = crate::native::watcher::FileWatcher::new() {
                                    let _ = watcher.watch(&self.root_path);
                                    self.file_watcher = Some(watcher);
                                }

                                self.new_project_dialog_open = false;
                                self.new_project_name.clear();
                            }
                            Err(e) => {
                                self.status_message = format!("Error: {}", e);
                                self.status_message_timestamp = Some(std::time::Instant::now());
                            }
                        }
                    }

                    if ui.button("Cancel").clicked() {
                        self.new_project_dialog_open = false;
                        self.new_project_name.clear();
                    }
                });
            });
    }

    /// Create a new Bevy project with the chosen template
    fn create_bevy_project(project_path: &str, project_name: &str, template: ProjectTemplate) -> anyhow::Result<()> {
        use std::fs;
        use std::path::Path;

        let root = Path::new(project_path);
        if root.exists() {
            anyhow::bail!("Directory already exists: {}", project_path);
        }

        fs::create_dir_all(root.join("src"))?;
        fs::create_dir_all(root.join("assets"))?;

        // Cargo.toml
        let cargo_toml = format!(
            "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nbevy = \"0.15\"\n\n[profile.dev]\nopt-level = 1\n\n[profile.dev.package.\"*\"]\nopt-level = 3\n",
            name = project_name
        );
        fs::write(root.join("Cargo.toml"), cargo_toml)?;

        // src/main.rs based on template
        let main_rs = template_main_rs(template);
        fs::write(root.join("src/main.rs"), main_rs)?;

        // .gitignore
        fs::write(root.join(".gitignore"), "/target\n")?;

        // Initialize git
        match git2::Repository::init(root) {
            Ok(_) => tracing::info!("Git repository initialized for {}", project_path),
            Err(e) => tracing::warn!("Failed to init git: {}", e),
        }

        tracing::info!("Bevy project created at {} with template {:?}", project_path, template);
        Ok(())
    }
}

fn template_main_rs(template: ProjectTemplate) -> String {
    match template {
        ProjectTemplate::Empty2D => {
            "use bevy::prelude::*;\n\nfn main() {\n    App::new()\n        .add_plugins(DefaultPlugins)\n        .add_systems(Startup, setup)\n        .run();\n}\n\nfn setup(mut commands: Commands) {\n    commands.spawn(Camera2d);\n}\n".to_string()
        }
        ProjectTemplate::Empty3D => {
            r#"use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(3.0, 3.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Light
    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.5, 0.5, 0.0)),
    ));

    // Cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(materials.add(Color::srgb(0.4, 0.6, 1.0))),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));

    // Ground
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(10.0, 10.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.3, 0.3))),
    ));
}
"#.to_string()
        }
        ProjectTemplate::Walker3D => {
            r#"//! 3D Walker - First-person walker in a 3D world
//!
//! Controls:
//!   WASD     - Move
//!   Mouse    - Look around
//!   Space    - Jump
//!   Shift    - Run faster
//!   Esc      - Release/grab cursor

use bevy::prelude::*;
use bevy::input::mouse::MouseMotion;
use bevy::window::{CursorGrabMode, PrimaryWindow};

const PLAYER_SPEED: f32 = 5.0;
const RUN_MULTIPLIER: f32 = 2.0;
const MOUSE_SENSITIVITY: f32 = 0.002;
const GRAVITY: f32 = -25.0;
const JUMP_VELOCITY: f32 = 8.0;
const PLAYER_HEIGHT: f32 = 1.7;

#[derive(Component)]
struct Player {
    yaw: f32,
    pitch: f32,
    velocity_y: f32,
    on_ground: bool,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, (setup_world, setup_player, grab_cursor))
        .add_systems(Update, (
            mouse_look,
            player_movement,
            apply_gravity,
            toggle_cursor_grab,
        ))
        .run();
}

fn setup_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Sun
    commands.spawn((
        DirectionalLight {
            illuminance: 12000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -1.0, 0.5, 0.0)),
    ));

    // Ground
    let ground_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.3, 0.5, 0.3),
        perceptual_roughness: 0.95,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(100.0, 100.0))),
        MeshMaterial3d(ground_mat),
    ));

    // Some boxes scattered around
    let box_mesh = meshes.add(Cuboid::new(1.5, 1.5, 1.5));
    let colors = [
        Color::srgb(0.8, 0.3, 0.3),
        Color::srgb(0.3, 0.8, 0.3),
        Color::srgb(0.3, 0.3, 0.8),
        Color::srgb(0.8, 0.8, 0.3),
        Color::srgb(0.8, 0.3, 0.8),
    ];
    for i in 0..15 {
        let angle = (i as f32) * 0.6;
        let radius = 4.0 + (i as f32) * 0.8;
        let x = angle.cos() * radius;
        let z = angle.sin() * radius;
        let color = colors[i % colors.len()];
        commands.spawn((
            Mesh3d(box_mesh.clone()),
            MeshMaterial3d(materials.add(color)),
            Transform::from_xyz(x, 0.75, z),
        ));
    }

    // A few tall pillars
    let pillar_mesh = meshes.add(Cuboid::new(1.0, 5.0, 1.0));
    let pillar_mat = materials.add(Color::srgb(0.6, 0.6, 0.7));
    for (x, z) in [(8.0, 0.0), (-8.0, 0.0), (0.0, 8.0), (0.0, -8.0)] {
        commands.spawn((
            Mesh3d(pillar_mesh.clone()),
            MeshMaterial3d(pillar_mat.clone()),
            Transform::from_xyz(x, 2.5, z),
        ));
    }
}

fn setup_player(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, PLAYER_HEIGHT, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        Player {
            yaw: 0.0,
            pitch: 0.0,
            velocity_y: 0.0,
            on_ground: true,
        },
    ));
}

fn grab_cursor(mut windows: Query<&mut Window, With<PrimaryWindow>>) {
    if let Ok(mut window) = windows.get_single_mut() {
        window.cursor_options.grab_mode = CursorGrabMode::Locked;
        window.cursor_options.visible = false;
    }
}

fn toggle_cursor_grab(
    keys: Res<ButtonInput<KeyCode>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        if let Ok(mut window) = windows.get_single_mut() {
            let grabbed = window.cursor_options.grab_mode != CursorGrabMode::None;
            window.cursor_options.grab_mode = if grabbed { CursorGrabMode::None } else { CursorGrabMode::Locked };
            window.cursor_options.visible = grabbed;
        }
    }
}

fn mouse_look(
    mut motion_events: EventReader<MouseMotion>,
    mut player_q: Query<(&mut Transform, &mut Player)>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let Ok(window) = windows.get_single() else { return; };
    if window.cursor_options.grab_mode == CursorGrabMode::None {
        motion_events.clear();
        return;
    }

    let mut delta = Vec2::ZERO;
    for ev in motion_events.read() {
        delta += ev.delta;
    }

    if let Ok((mut transform, mut player)) = player_q.get_single_mut() {
        player.yaw -= delta.x * MOUSE_SENSITIVITY;
        player.pitch -= delta.y * MOUSE_SENSITIVITY;
        player.pitch = player.pitch.clamp(-1.5, 1.5);

        transform.rotation = Quat::from_axis_angle(Vec3::Y, player.yaw)
            * Quat::from_axis_angle(Vec3::X, player.pitch);
    }
}

fn player_movement(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut player_q: Query<(&mut Transform, &mut Player)>,
) {
    let Ok((mut transform, mut player)) = player_q.get_single_mut() else { return; };

    let forward = Vec3::new(player.yaw.sin(), 0.0, player.yaw.cos()) * -1.0;
    let right = Vec3::new(player.yaw.cos(), 0.0, -player.yaw.sin());

    let mut direction = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) { direction += forward; }
    if keys.pressed(KeyCode::KeyS) { direction -= forward; }
    if keys.pressed(KeyCode::KeyD) { direction += right; }
    if keys.pressed(KeyCode::KeyA) { direction -= right; }

    if direction.length_squared() > 0.0 {
        direction = direction.normalize();
    }

    let speed = if keys.pressed(KeyCode::ShiftLeft) {
        PLAYER_SPEED * RUN_MULTIPLIER
    } else {
        PLAYER_SPEED
    };
    transform.translation += direction * speed * time.delta_secs();

    // Jump
    if keys.just_pressed(KeyCode::Space) && player.on_ground {
        player.velocity_y = JUMP_VELOCITY;
        player.on_ground = false;
    }
}

fn apply_gravity(time: Res<Time>, mut player_q: Query<(&mut Transform, &mut Player)>) {
    let Ok((mut transform, mut player)) = player_q.get_single_mut() else { return; };

    player.velocity_y += GRAVITY * time.delta_secs();
    transform.translation.y += player.velocity_y * time.delta_secs();

    // Simple ground collision (y = 0 plane, player feet at y = 0, eyes at PLAYER_HEIGHT)
    if transform.translation.y < PLAYER_HEIGHT {
        transform.translation.y = PLAYER_HEIGHT;
        player.velocity_y = 0.0;
        player.on_ground = true;
    }
}
"#.to_string()
        }
        ProjectTemplate::Plugin => {
            r#"use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MyPlugin)
        .run();
}

pub struct MyPlugin;

impl Plugin for MyPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, setup)
            .add_systems(Update, update);
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn update() {
    // TODO
}
"#.to_string()
        }
    }
}
