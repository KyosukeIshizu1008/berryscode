//! Build settings and player settings panels.

use crate::app::BerryCodeApp;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildSettings {
    pub target_platform: Platform,
    pub resolution: [u32; 2],
    pub fullscreen: bool,
    pub quality: QualityLevel,
    /// Ordered list of scenes included in the build. Index 0 is the startup scene.
    #[serde(default)]
    pub scenes_in_build: Vec<SceneEntry>,
    /// iOS bundle identifier (e.g. `com.example.myapp`). Surfaced in
    /// the generated `Info.plist` and in the `xcrun altool` upload.
    #[serde(default)]
    pub ios_bundle_id: String,
    /// Apple developer Team ID for codesigning (10-char alphanumeric).
    #[serde(default)]
    pub ios_team_id: String,
    /// Android package name (e.g. `com.example.myapp`).
    #[serde(default)]
    pub android_package_name: String,
    /// Path to the keystore (`.jks`) used for AAB signing.
    #[serde(default)]
    pub android_keystore_path: String,
    /// Keystore alias inside the `.jks`.
    #[serde(default)]
    pub android_key_alias: String,
    /// Path to the Play Console service-account JSON used by
    /// `mobile::play_console::upload`.
    #[serde(default)]
    pub play_console_service_account_path: String,
}

/// A scene entry in the build order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneEntry {
    /// Path to the .bscene file (relative to project root).
    pub path: String,
    /// Whether this scene is enabled in the build.
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Platform {
    MacOS,
    Windows,
    Linux,
    Web,
    IosDevice,
    IosSimulator,
    Android,
    VisionOs,
    Quest,
}

impl Platform {
    pub const ALL: &'static [Platform] = &[
        Platform::MacOS,
        Platform::Windows,
        Platform::Linux,
        Platform::Web,
        Platform::IosDevice,
        Platform::IosSimulator,
        Platform::Android,
        Platform::VisionOs,
        Platform::Quest,
    ];

    /// Desktop + Web — the targets the existing `execute_build` path handles.
    /// Mobile / XR variants build through the `app::mobile` packagers (Phase E).
    pub const DESKTOP_AND_WEB: &'static [Platform] = &[
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
            Platform::IosDevice => "iOS Device",
            Platform::IosSimulator => "iOS Simulator",
            Platform::Android => "Android",
            Platform::VisionOs => "visionOS",
            Platform::Quest => "Meta Quest",
        }
    }

    pub fn is_mobile(&self) -> bool {
        matches!(
            self,
            Platform::IosDevice
                | Platform::IosSimulator
                | Platform::Android
                | Platform::VisionOs
                | Platform::Quest
        )
    }

    /// Apple toolchain family — uses Xcode + codesigning.
    /// API surface for the Phase E packagers; unused until then.
    #[allow(dead_code)]
    pub fn is_apple_mobile(&self) -> bool {
        matches!(
            self,
            Platform::IosDevice | Platform::IosSimulator | Platform::VisionOs
        )
    }

    /// Android toolchain family — uses SDK + NDK + gradle / keystore.
    /// API surface for the Phase E packagers; unused until then.
    #[allow(dead_code)]
    pub fn is_android_family(&self) -> bool {
        matches!(self, Platform::Android | Platform::Quest)
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
            scenes_in_build: Vec::new(),
            ios_bundle_id: String::new(),
            ios_team_id: String::new(),
            android_package_name: String::new(),
            android_keystore_path: String::new(),
            android_key_alias: String::new(),
            play_console_service_account_path: String::new(),
        }
    }
}

/// Scan project directory for all .bscene files.
pub fn scan_scene_files(root: &str) -> Vec<String> {
    let mut scenes = Vec::new();
    scan_bscene_recursive(std::path::Path::new(root), root, &mut scenes);
    scenes.sort();
    scenes
}

fn scan_bscene_recursive(dir: &std::path::Path, root: &str, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            if name == "target" || name.starts_with('.') {
                continue;
            }
            scan_bscene_recursive(&path, root, out);
        } else if path.extension().and_then(|s| s.to_str()) == Some("bscene") {
            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();
            out.push(rel);
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
        Platform::IosDevice => "aarch64-apple-ios",
        Platform::IosSimulator => "aarch64-apple-ios-sim",
        // 64-bit ARM is the only Android triple we ship by default; armv7 / x86_64
        // simulator / x86 are added by Phase E's per-project ABI configuration.
        Platform::Android => "aarch64-linux-android",
        Platform::VisionOs => "aarch64-apple-visionos",
        // Quest runs the Android target; the OpenXR loader is layered on top in Phase F.
        Platform::Quest => "aarch64-linux-android",
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
    // Mobile / XR targets dispatch into `app::mobile::packager` (Phase E)
    // which compiles for the right `rustup` target. Codesigning / IPA
    // assembly / AAB packaging are still v0.7.1+ follow-ups; the user
    // takes the resulting binary into Xcode / Android Studio for now.
    use crate::app::mobile::packager;
    match settings.target_platform {
        Platform::IosSimulator => return packager::build_ios_simulator(root_path),
        Platform::IosDevice => return packager::build_ios_device(root_path),
        Platform::Android => return packager::build_android(root_path),
        Platform::VisionOs | Platform::Quest => {
            return Err(format!(
                "{} builds are not packaged yet — tracked for v0.9.x.",
                settings.target_platform.label()
            ));
        }
        _ => {}
    }

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

    // Capture stderr. `map_while(Result::ok)` stops the iteration on
    // the first IO error so a broken pipe doesn't spin the thread.
    if let Some(stderr) = child.stderr.take() {
        let tx_clone = tx.clone();
        std::thread::spawn(move || {
            use std::io::BufRead;
            let reader = std::io::BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                let _ = tx_clone.send(line);
            }
        });
    }

    // Capture stdout (same pattern as stderr above).
    if let Some(stdout) = child.stdout.take() {
        std::thread::spawn(move || {
            use std::io::BufRead;
            let reader = std::io::BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                let _ = tx.send(line);
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

                // Platform — desktop / web only here; mobile / XR are
                // dispatched through the Mobile Toolchain panel (Phase A) and
                // the Run / Ship pipelines (Phases B / E). Showing them in this
                // legacy dropdown would silently fall through to a broken
                // `cargo build --target …` shell-out.
                ui.horizontal(|ui| {
                    ui.label("Target Platform:");
                    egui::ComboBox::from_id_salt("build_platform")
                        .selected_text(self.build_settings.target_platform.label())
                        .show_ui(ui, |ui| {
                            for &p in Platform::DESKTOP_AND_WEB {
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

                // --- Mobile ---
                // Read by the Mobile Toolchain panel's One Click flow
                // (`cargo mobile init` requires a non-empty bundle ID).
                ui.separator();
                ui.heading("Mobile");
                ui.horizontal(|ui| {
                    ui.label("iOS Bundle ID:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.build_settings.ios_bundle_id)
                            .hint_text("com.example.MyGame")
                            .desired_width(220.0),
                    );
                });

                // --- Scenes In Build ---
                ui.separator();
                ui.heading("Scenes In Build");
                ui.separator();

                // Scan for available scenes
                if ui.small_button("Refresh Scene List").clicked() {
                    let available = scan_scene_files(&self.root_path);
                    // Add new scenes not already in list
                    for scene_path in &available {
                        if !self
                            .build_settings
                            .scenes_in_build
                            .iter()
                            .any(|s| s.path == *scene_path)
                        {
                            self.build_settings.scenes_in_build.push(SceneEntry {
                                path: scene_path.clone(),
                                enabled: true,
                            });
                        }
                    }
                    // Remove entries for deleted files
                    self.build_settings
                        .scenes_in_build
                        .retain(|s| available.contains(&s.path));
                }

                if self.build_settings.scenes_in_build.is_empty() {
                    ui.label(
                        egui::RichText::new(
                            "No scenes added. Save a scene first, then click Refresh.",
                        )
                        .color(egui::Color32::from_gray(120))
                        .italics(),
                    );
                } else {
                    // Scene list with reorder buttons
                    let mut swap: Option<(usize, usize)> = None;
                    let mut remove_idx: Option<usize> = None;
                    let mut load_path: Option<String> = None;
                    let scene_count = self.build_settings.scenes_in_build.len();
                    for i in 0..scene_count {
                        ui.horizontal(|ui| {
                            // Index label (0 = startup scene)
                            let idx_label = if i == 0 {
                                egui::RichText::new(format!("{} (Start)", i))
                                    .color(egui::Color32::from_rgb(120, 220, 120))
                            } else {
                                egui::RichText::new(format!("{}", i))
                                    .color(egui::Color32::from_gray(150))
                            };
                            ui.label(idx_label);

                            // Enable checkbox
                            ui.checkbox(&mut self.build_settings.scenes_in_build[i].enabled, "");

                            // Scene path as clickable label
                            let path = self.build_settings.scenes_in_build[i].path.clone();
                            if ui
                                .add(
                                    egui::Label::new(egui::RichText::new(&path).color(
                                        if self.build_settings.scenes_in_build[i].enabled {
                                            egui::Color32::from_rgb(212, 212, 212)
                                        } else {
                                            egui::Color32::from_gray(90)
                                        },
                                    ))
                                    .sense(egui::Sense::click()),
                                )
                                .on_hover_text("Click to load")
                                .clicked()
                            {
                                load_path = Some(path);
                            }

                            // Reorder buttons
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.small_button("x").clicked() {
                                        remove_idx = Some(i);
                                    }
                                    if i + 1 < scene_count && ui.small_button("\u{25bc}").clicked()
                                    {
                                        swap = Some((i, i + 1));
                                    }
                                    if i > 0 && ui.small_button("\u{25b2}").clicked() {
                                        swap = Some((i, i - 1));
                                    }
                                },
                            );
                        });
                    }
                    if let Some((a, b)) = swap {
                        self.build_settings.scenes_in_build.swap(a, b);
                    }
                    if let Some(idx) = remove_idx {
                        self.build_settings.scenes_in_build.remove(idx);
                    }
                    if let Some(path) = load_path {
                        let full_path = format!("{}/{}", self.root_path, path);
                        self.load_scene(&full_path);
                    }
                }

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

                // Build button and status
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
        assert_eq!(get_target_triple(Platform::IosDevice), "aarch64-apple-ios");
        assert_eq!(
            get_target_triple(Platform::IosSimulator),
            "aarch64-apple-ios-sim"
        );
        assert_eq!(
            get_target_triple(Platform::Android),
            "aarch64-linux-android"
        );
        assert_eq!(
            get_target_triple(Platform::VisionOs),
            "aarch64-apple-visionos"
        );
        assert_eq!(get_target_triple(Platform::Quest), "aarch64-linux-android");
    }

    #[test]
    fn mobile_is_classified() {
        assert!(Platform::IosDevice.is_mobile());
        assert!(Platform::Android.is_mobile());
        assert!(Platform::Quest.is_mobile());
        assert!(!Platform::MacOS.is_mobile());

        assert!(Platform::IosDevice.is_apple_mobile());
        assert!(Platform::VisionOs.is_apple_mobile());
        assert!(!Platform::Android.is_apple_mobile());

        assert!(Platform::Android.is_android_family());
        assert!(Platform::Quest.is_android_family());
        assert!(!Platform::IosDevice.is_android_family());
    }

    #[test]
    fn execute_build_rejects_visionos_until_v0_9() {
        // v0.7.0 wires iOS/Android through `mobile::packager`; visionOS /
        // Quest still hit the explicit "not packaged yet" guard.
        let bs = BuildSettings {
            target_platform: Platform::VisionOs,
            ..BuildSettings::default()
        };
        let err = execute_build(".", &bs).unwrap_err();
        assert!(
            err.contains("v0.9"),
            "expected version-gate message, got: {err}"
        );
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
            scenes_in_build: vec![],
            ..BuildSettings::default()
        };
        let s = ron::ser::to_string(&bs).unwrap();
        let loaded: BuildSettings = ron::from_str(&s).unwrap();
        assert_eq!(loaded.target_platform, Platform::Linux);
        assert_eq!(loaded.resolution, [1920, 1080]);
        assert!(loaded.fullscreen);
        assert_eq!(loaded.quality, QualityLevel::Ultra);
    }
}
