//! BerryCode - Bevy-based IDE for Bevy Game Engine

use bevy::prelude::*;
use berry_editor::bevy_plugin::BerryCodePlugin;

fn main() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            tracing_subscriber::EnvFilter::new("warn")
                .add_directive("berry_editor=info".parse().unwrap())
                .add_directive("berrycode=info".parse().unwrap())
        });

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .init();

    tracing::info!("Starting BerryCode - Bevy IDE");

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "BerryCode - Bevy IDE".into(),
                resolution: (1400.0, 900.0).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(BerryCodePlugin)
        .run();
}
