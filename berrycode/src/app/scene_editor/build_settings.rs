//! Build settings and player settings panels.

use crate::app::BerryCodeApp;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildSettings {
    pub target_platform: Platform,
    pub resolution: [u32; 2],
    pub fullscreen: bool,
    pub quality: QualityLevel,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Platform {
    MacOS,
    Windows,
    Linux,
    Web,
}

impl Platform {
    pub const ALL: &'static [Platform] = &[
        Platform::MacOS,
        Platform::Windows,
        Platform::Linux,
        Platform::Web,
    ];
    pub fn label(&self) -> &'static str {
        match self {
            Platform::MacOS => "macOS",
            Platform::Windows => "Windows",
            Platform::Linux => "Linux",
            Platform::Web => "Web (WASM)",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum QualityLevel {
    Low,
    Medium,
    High,
    Ultra,
}

impl QualityLevel {
    pub const ALL: &'static [QualityLevel] = &[
        QualityLevel::Low,
        QualityLevel::Medium,
        QualityLevel::High,
        QualityLevel::Ultra,
    ];
    pub fn label(&self) -> &'static str {
        match self {
            QualityLevel::Low => "Low",
            QualityLevel::Medium => "Medium",
            QualityLevel::High => "High",
            QualityLevel::Ultra => "Ultra",
        }
    }
}

impl Default for BuildSettings {
    fn default() -> Self {
        Self {
            target_platform: Platform::MacOS,
            resolution: [1280, 720],
            fullscreen: false,
            quality: QualityLevel::High,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerSettings {
    pub window_title: String,
    pub icon_path: String,
    pub splash_image_path: String,
}

impl Default for PlayerSettings {
    fn default() -> Self {
        Self {
            window_title: "My Bevy Game".into(),
            icon_path: String::new(),
            splash_image_path: String::new(),
        }
    }
}

impl BuildSettings {
    pub fn load(root: &str) -> Self {
        let path = format!("{}/build_settings.ron", root);
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| ron::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, root: &str) {
        let path = format!("{}/build_settings.ron", root);
        if let Ok(s) = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default()) {
            let _ = std::fs::write(&path, s);
        }
    }
}

impl PlayerSettings {
    pub fn load(root: &str) -> Self {
        let path = format!("{}/player_settings.ron", root);
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| ron::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, root: &str) {
        let path = format!("{}/player_settings.ron", root);
        if let Ok(s) = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default()) {
            let _ = std::fs::write(&path, s);
        }
    }
}

impl Platform {
    /// Map platform to Rust target triple.
    pub fn target_triple(&self) -> &'static str {
        get_target_triple(*self)
    }
}

/// Standalone function to get target triple for a platform (testable without self).
pub fn get_target_triple(platform: Platform) -> &'static str {
    match platform {
        Platform::MacOS => "aarch64-apple-darwin",
        Platform::Windows => "x86_64-pc-windows-msvc",
        Platform::Linux => "x86_64-unknown-linux-gnu",
        Platform::Web => "wasm32-unknown-unknown",
    }
}

/// Validate build settings: check that resolution is within reasonable bounds.
pub fn validate_build_settings(settings: &BuildSettings) -> Vec<String> {
    let mut errors = Vec::new();
    if settings.resolution[0] < 320 || settings.resolution[0] > 7680 {
        errors.push(format!("Invalid width: {}", settings.resolution[0]));
    }
    if settings.resolution[1] < 240 || settings.resolution[1] > 4320 {
        errors.push(format!("Invalid height: {}", settings.resolution[1]));
    }
    errors
}

/// Generate the cargo build command arguments for a given build settings config.
pub fn build_command_args(settings: &BuildSettings) -> Vec<String> {
    let triple = get_target_triple(settings.target_platform);
    vec![
        "build".into(),
        "--release".into(),
        "--target".into(),
        triple.into(),
    ]
}

/// Execute a release build for the configured platform. Returns a channel
/// receiver for build output lines. The caller is responsible for polling it.
pub fn execute_build(
    root_path: &str,
    settings: &BuildSettings,
) -> Result<(std::process::Child, std::sync::mpsc::Receiver<String>), String> {
    let triple = settings.target_platform.target_triple();
    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("build")
        .arg("--release")
        .arg("--target")
        .arg(triple)
        .current_dir(root_path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to start build: {}", e))?;

    let (tx, rx) = std::sync::mpsc::channel();

    // Capture stderr
    if let Some(stderr) = child.stderr.take() {
        let tx_clone = tx.clone();
        std::thread::spawn(move || {
            use std::io::BufRead;
            let reader = std::io::BufReader::new(stderr);
            for line in reader.lines() {
                if let Ok(line) = line {
                    let _ = tx_clone.send(line);
                }
            }
        });
    }

    // Capture stdout
    if let Some(stdout) = child.stdout.take() {
        std::thread::spawn(move || {
            use std::io::BufRead;
            let reader = std::io::BufReader::new(stdout);
            for line in reader.lines() {
                if let Ok(line) = line {
                    let _ = tx.send(line);
                }
            }
        });
    }

    Ok((child, rx))
}

impl BerryCodeApp {
    /// Render Build Settings window.
    pub(crate) fn render_build_settings(&mut self, ctx: &egui::Context) {
        if !self.build_settings_open {
            return;
        }

        let mut open = self.build_settings_open;
        egui::Window::new("Build Settings")
            .open(&mut open)
            .default_size([400.0, 350.0])
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading("Build Configuration");
                ui.separator();

                // Platform
                ui.horizontal(|ui| {
                    ui.label("Target Platform:");
                    egui::ComboBox::from_id_salt("build_platform")
                        .selected_text(self.build_settings.target_platform.label())
                        .show_ui(ui, |ui| {
                            for &p in Platform::ALL {
                                ui.selectable_value(
                                    &mut self.build_settings.target_platform,
                                    p,
                                    p.label(),
                                );
                            }
                        });
                });

                // Resolution
                ui.horizontal(|ui| {
                    ui.label("Resolution:");
                    ui.add(
                        egui::DragValue::new(&mut self.build_settings.resolution[0])
                            .prefix("W: ")
                            .range(320u32..=7680u32),
                    );
                    ui.label("x");
                    ui.add(
                        egui::DragValue::new(&mut self.build_settings.resolution[1])
                            .prefix("H: ")
                            .range(240u32..=4320u32),
                    );
                });

                ui.checkbox(&mut self.build_settings.fullscreen, "Fullscreen");

                // Quality
                ui.horizontal(|ui| {
                    ui.label("Quality:");
                    egui::ComboBox::from_id_salt("build_quality")
                        .selected_text(self.build_settings.quality.label())
                        .show_ui(ui, |ui| {
                            for &q in QualityLevel::ALL {
                                ui.selectable_value(&mut self.build_settings.quality, q, q.label());
                            }
                        });
                });

                ui.separator();
                ui.heading("Player Settings");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Window Title:");
                    ui.text_edit_singleline(&mut self.player_settings.window_title);
                });
                ui.horizontal(|ui| {
                    ui.label("Icon Path:");
                    ui.text_edit_singleline(&mut self.player_settings.icon_path);
                });
                ui.horizontal(|ui| {
                    ui.label("Splash Image:");
                    ui.text_edit_singleline(&mut self.player_settings.splash_image_path);
                });

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Save Settings").clicked() {
                        self.build_settings.save(&self.root_path);
                        self.player_settings.save(&self.root_path);
                        self.status_message = "Build settings saved".to_string();
                        self.status_message_timestamp = Some(std::time::Instant::now());
                    }
                    if ui.button("Reset Defaults").clicked() {
                        self.build_settings = BuildSettings::default();
                        self.player_settings = PlayerSettings::default();
                    }
                });

                ui.separator();

                // Build button and status (Phase 76)
                let is_building = self.build_process.is_some();
                ui.add_enabled_ui(!is_building, |ui| {
                    if ui.button("Build").clicked() {
                        match execute_build(&self.root_path, &self.build_settings) {
                            Ok((child, rx)) => {
                                self.build_process = Some(child);
                                self.build_output_rx = Some(rx);
                                self.build_output.clear();
                                self.status_message = format!(
                                    "Building for {}...",
                                    self.build_settings.target_platform.label()
                                );
                                self.status_message_timestamp = Some(std::time::Instant::now());
                            }
                            Err(e) => {
                                self.status_message = e;
                                self.status_message_timestamp = Some(std::time::Instant::now());
                            }
                        }
                    }
                });

                if is_building {
                    ui.colored_label(egui::Color32::YELLOW, "Building...");
                }

                // Poll build output
                if let Some(ref rx) = self.build_output_rx {
                    while let Ok(line) = rx.try_recv() {
                        self.build_output.push(line);
                    }
                }

                // Check if build finished
                if let Some(ref mut child) = self.build_process {
                    if let Ok(Some(status)) = child.try_wait() {
                        let msg = if status.success() {
                            "Build succeeded".to_string()
                        } else {
                            format!("Build failed (exit {})", status.code().unwrap_or(-1))
                        };
                        self.status_message = msg;
                        self.status_message_timestamp = Some(std::time::Instant::now());
                        // Will be cleaned up below
                    }
                }

                // Clean up finished process
                let finished = self
                    .build_process
                    .as_mut()
                    .and_then(|c| c.try_wait().ok())
                    .flatten()
                    .is_some();
                if finished {
                    self.build_process = None;
                    self.build_output_rx = None;
                }

                // Show build output
                if !self.build_output.is_empty() {
                    ui.separator();
                    ui.label("Build Output:");
                    egui::ScrollArea::vertical()
                        .max_height(150.0)
                        .show(ui, |ui| {
                            for line in &self.build_output {
                                ui.monospace(line);
                            }
                        });
                }
            });
        self.build_settings_open = open;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_settings_default() {
        let bs = BuildSettings::default();
        assert_eq!(bs.resolution, [1280, 720]);
        assert_eq!(bs.target_platform, Platform::MacOS);
    }

    #[test]
    fn player_settings_default() {
        let ps = PlayerSettings::default();
        assert_eq!(ps.window_title, "My Bevy Game");
    }

    #[test]
    fn get_target_triple_all_platforms() {
        assert_eq!(get_target_triple(Platform::MacOS), "aarch64-apple-darwin");
        assert_eq!(
            get_target_triple(Platform::Windows),
            "x86_64-pc-windows-msvc"
        );
        assert_eq!(
            get_target_triple(Platform::Linux),
            "x86_64-unknown-linux-gnu"
        );
        assert_eq!(get_target_triple(Platform::Web), "wasm32-unknown-unknown");
    }

    #[test]
    fn target_triple_method_matches_function() {
        for &p in Platform::ALL {
            assert_eq!(p.target_triple(), get_target_triple(p));
        }
    }

    #[test]
    fn validate_build_settings_valid() {
        let bs = BuildSettings::default();
        let errors = validate_build_settings(&bs);
        assert!(errors.is_empty());
    }

    #[test]
    fn validate_build_settings_invalid_resolution() {
        let bs = BuildSettings {
            resolution: [100, 100],
            ..BuildSettings::default()
        };
        let errors = validate_build_settings(&bs);
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn build_command_args_contains_target() {
        let bs = BuildSettings::default();
        let args = build_command_args(&bs);
        assert!(args.contains(&"--release".to_string()));
        assert!(args.contains(&"--target".to_string()));
        assert!(args.contains(&"aarch64-apple-darwin".to_string()));
    }

    #[test]
    fn build_command_args_web() {
        let bs = BuildSettings {
            target_platform: Platform::Web,
            ..BuildSettings::default()
        };
        let args = build_command_args(&bs);
        assert!(args.contains(&"wasm32-unknown-unknown".to_string()));
    }

    #[test]
    fn platform_labels() {
        for &p in Platform::ALL {
            assert!(!p.label().is_empty());
        }
    }

    #[test]
    fn quality_labels() {
        for &q in QualityLevel::ALL {
            assert!(!q.label().is_empty());
        }
    }

    #[test]
    fn build_settings_roundtrip() {
        let bs = BuildSettings {
            target_platform: Platform::Linux,
            resolution: [1920, 1080],
            fullscreen: true,
            quality: QualityLevel::Ultra,
        };
        let s = ron::ser::to_string(&bs).unwrap();
        let loaded: BuildSettings = ron::from_str(&s).unwrap();
        assert_eq!(loaded.target_platform, Platform::Linux);
        assert_eq!(loaded.resolution, [1920, 1080]);
        assert!(loaded.fullscreen);
        assert_eq!(loaded.quality, QualityLevel::Ultra);
    }
}
