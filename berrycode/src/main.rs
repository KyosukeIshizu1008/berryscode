//! BerryCode - Bevy-based IDE for Bevy Game Engine

use berrycode::bevy_plugin::BerryCodePlugin;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::winit::WinitWindows;
use std::fs;
use std::io::Write;

fn main() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        tracing_subscriber::EnvFilter::new("warn").add_directive("berrycode=info".parse().unwrap())
    });

    tracing_subscriber::fmt().with_env_filter(filter).init();

    // Single instance check
    let lock_path = dirs::cache_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
        .join("berrycode.lock");

    if let Ok(content) = fs::read_to_string(&lock_path) {
        if let Ok(pid) = content.trim().parse::<u32>() {
            // Check if the process is still alive using kill -0
            let alive = std::process::Command::new("kill")
                .args(["-0", &pid.to_string()])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            if alive {
                eprintln!("BerryCode is already running (pid {})", pid);
                std::process::exit(0);
            }
        }
    }

    // Write our PID to lock file
    if let Ok(mut f) = fs::File::create(&lock_path) {
        let _ = write!(f, "{}", std::process::id());
    }

    tracing::info!("Starting BerryCode - Bevy IDE");

    App::new()
        .add_plugins(
            DefaultPlugins
                .build()
                .disable::<LogPlugin>()
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "BerryCode - Bevy IDE".into(),
                        resolution: (1400, 900).into(),
                        ..default()
                    }),
                    exit_condition: bevy::window::ExitCondition::DontExit,
                    ..default()
                }),
        )
        .add_plugins(BerryCodePlugin)
        .add_systems(PreStartup, setup_camera.before(bevy_egui::EguiStartupSet::InitContexts))
        .add_systems(Startup, set_window_icon)
        .run();

    // Clean up lock file on exit
    let _ = fs::remove_file(&lock_path);
}

fn setup_camera(
    mut commands: Commands,
    mut egui_settings: ResMut<bevy_egui::EguiGlobalSettings>,
) {
    egui_settings.auto_create_primary_context = false;
    // Egui camera - renders UI only
    commands.spawn((
        bevy_egui::PrimaryEguiContext,
        Camera2d,
        bevy::camera::visibility::RenderLayers::none(),
        Camera {
            order: 100, // render last, on top of everything
            clear_color: ClearColorConfig::None,
            ..default()
        },
    ));
}

fn set_window_icon(windows: Option<NonSend<WinitWindows>>) {
    let Some(windows) = windows else { return; };
    let icon_bytes = include_bytes!("../assets/icon_256.png");
    let img = match image::load_from_memory(icon_bytes) {
        Ok(img) => img.into_rgba8(),
        Err(e) => {
            tracing::warn!("Failed to load window icon: {}", e);
            return;
        }
    };
    let (width, height) = img.dimensions();
    let icon = match winit::window::Icon::from_rgba(img.into_raw(), width, height) {
        Ok(icon) => icon,
        Err(e) => {
            tracing::warn!("Failed to create window icon: {}", e);
            return;
        }
    };
    for window in windows.windows.values() {
        window.set_window_icon(Some(icon.clone()));
    }
}
