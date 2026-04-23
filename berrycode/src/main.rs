//! BerryCode - Bevy-based IDE for Bevy Game Engine

use berrycode::bevy_plugin::BerryCodePlugin;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::winit::WinitWindows;

fn main() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        tracing_subscriber::EnvFilter::new("warn").add_directive("berrycode=info".parse().unwrap())
    });

    tracing_subscriber::fmt().with_env_filter(filter).init();

    tracing::info!("Starting BerryCode - Bevy IDE");

    App::new()
        .add_plugins(
            DefaultPlugins
                .build()
                .disable::<LogPlugin>()
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "BerryCode - Bevy IDE".into(),
                        resolution: (1400.0, 900.0).into(),
                        ..default()
                    }),
                    exit_condition: bevy::window::ExitCondition::DontExit,
                    ..default()
                }),
        )
        .add_plugins(BerryCodePlugin)
        .add_systems(Startup, set_window_icon)
        .run();
}

fn set_window_icon(windows: NonSend<WinitWindows>) {
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
