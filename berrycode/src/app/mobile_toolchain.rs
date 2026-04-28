//! Mobile Toolchain panel — Phase A's user-facing surface.
//!
//! Three rows (iOS / Android / Rust targets), each with green-tick / red-x
//! status and a copy-paste install hint when something's missing. The actual
//! probing lives in `app::mobile::probe`; this file is just the egui shell.

use super::mobile::{self, LogStream, MobileRunSession, MobileTarget, MobileToolchain};
use super::scene_editor::build_settings::Platform;
use super::BerryCodeApp;
use std::path::PathBuf;

/// Panel state. Cheap to clone; lives directly on `BerryCodeApp`.
#[derive(Debug, Default)]
pub struct MobileToolchainState {
    pub toolchain: MobileToolchain,
    /// `true` once the first probe has run *or* a cached snapshot was
    /// loaded. Until then the panel shows a "Click Refresh to probe" hint
    /// instead of a misleading all-red wall.
    pub probed: bool,
    /// Last error from a refresh attempt, surfaced inline so the user sees
    /// it next to the panel rather than buried in the status bar.
    pub last_error: Option<String>,

    // ── Run section (Phase B) ─────────────────────────────────────────
    /// Selection persists across panel opens so the user doesn't have to
    /// re-pick the simulator every refresh.
    pub run_target: RunTargetSelection,
    /// Path to the pre-built artifact (.app for iOS, .apk for Android).
    /// Phase E will populate this from a packager; for now it's set via
    /// the file picker.
    pub run_artifact: PathBuf,
    pub run_bundle_id: String,
    pub run_package_name: String,
    pub run_activity: String,
    pub run_session: Option<MobileRunSession>,
    pub run_log: LogStream,
    pub run_error: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[allow(dead_code)] // IosDevice variant lights up once devicectl device-probing lands
pub enum RunTargetSelection {
    #[default]
    None,
    IosSim {
        udid: String,
    },
    IosDevice {
        udid: String,
    },
    Android {
        serial: String,
    },
}

impl MobileToolchainState {
    /// Build the initial state: prefer the on-disk cache so a cold open is
    /// instant. The lazy probe in `render_mobile_toolchain_window` covers
    /// the case where the cache is missing or corrupted.
    pub fn from_cache_or_default() -> Self {
        match MobileToolchain::load_from_disk() {
            Some(toolchain) => Self {
                toolchain,
                probed: true,
                last_error: None,
                ..Self::default()
            },
            None => Self::default(),
        }
    }

    /// Full re-probe: Xcode + Android + Rust targets. Persists the result.
    pub fn refresh(&mut self) {
        self.toolchain = mobile::probe_all();
        self.probed = true;
        self.last_error = None;
        self.toolchain.save_to_disk();
    }

    /// Lazy probe — fills `XcodeInstall.codesign_identities`. Only runs on
    /// macOS; no-op everywhere else. Persists after refresh so the next
    /// cold open already has the identities cached.
    pub fn refresh_codesign_identities(&mut self) {
        let Some(xcode) = self.toolchain.xcode.as_mut() else {
            return;
        };
        xcode.codesign_identities = mobile::probe_codesign_identities();
        self.toolchain.save_to_disk();
    }

    /// Lazy probe — fills `AndroidInstall.devices`. Requires `adb` to have
    /// been resolved by the previous full refresh.
    pub fn refresh_adb_devices(&mut self) {
        let Some(android) = self.toolchain.android.as_mut() else {
            return;
        };
        let Some(adb) = android.adb.clone() else {
            return;
        };
        android.devices = mobile::probe_adb_devices(&adb);
        self.toolchain.save_to_disk();
    }

    /// Per-frame: drain the live run session into the log stream.
    /// Called from the main render loop so the egui pass picks up
    /// pending lines without waiting for user interaction.
    pub fn poll_run(&mut self) {
        if let Some(session) = self.run_session.as_mut() {
            session.poll_into(&mut self.run_log);
            if !session.is_running() {
                // Keep the (now-dead) session around until the user
                // dismisses it so the trailing log is preserved; just
                // drop the receiver so we stop polling.
            }
        }
    }

    fn build_target(&self) -> Option<MobileTarget> {
        match &self.run_target {
            RunTargetSelection::None => None,
            RunTargetSelection::IosSim { udid } => Some(MobileTarget::IosSim {
                udid: udid.clone(),
                bundle_id: self.run_bundle_id.clone(),
            }),
            RunTargetSelection::IosDevice { udid } => Some(MobileTarget::IosDevice {
                udid: udid.clone(),
                bundle_id: self.run_bundle_id.clone(),
            }),
            RunTargetSelection::Android { serial } => Some(MobileTarget::Android {
                serial: serial.clone(),
                package_name: self.run_package_name.clone(),
                activity: self.run_activity.clone(),
            }),
        }
    }
}

// Mobile triples the panel checks Rust-target installation for. Kept in sync
// with the mobile entries in `Platform::ALL`; if the enum grows a new mobile
// triple, add it here too.
const MOBILE_TRIPLES: &[(Platform, &str)] = &[
    (Platform::IosDevice, "aarch64-apple-ios"),
    (Platform::IosSimulator, "aarch64-apple-ios-sim"),
    (Platform::Android, "aarch64-linux-android"),
    (Platform::VisionOs, "aarch64-apple-visionos"),
];

impl BerryCodeApp {
    pub(crate) fn render_mobile_toolchain_window(&mut self, ctx: &egui::Context) {
        if !self.mobile_toolchain_open {
            return;
        }

        // Lazy-probe on first open so the user sees populated rows
        // immediately. Subsequent opens reuse the cached snapshot — explicit
        // Refresh is the way to re-run the probe.
        if !self.mobile_toolchain.probed {
            self.mobile_toolchain.refresh();
        }

        let mut open = self.mobile_toolchain_open;
        egui::Window::new("Mobile Toolchain")
            .id(egui::Id::new("mobile_toolchain_v1"))
            .open(&mut open)
            .default_size([520.0, 560.0])
            .resizable(true)
            .show(ctx, |ui| {
                self.render_mobile_toolchain(ui);
            });
        self.mobile_toolchain_open = open;
    }

    fn render_mobile_toolchain(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("MOBILE TOOLCHAIN");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Refresh").clicked() {
                    self.mobile_toolchain.refresh();
                }
            });
        });
        ui.separator();

        if let Some(err) = &self.mobile_toolchain.last_error {
            ui.colored_label(egui::Color32::from_rgb(220, 80, 80), err);
            ui.separator();
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            self.render_xcode_section(ui);
            ui.add_space(8.0);
            self.render_android_section(ui);
            ui.add_space(8.0);
            self.render_rust_targets_section(ui);
            ui.add_space(12.0);
            ui.separator();
            self.render_run_section(ui);
        });
    }

    fn render_run_section(&mut self, ui: &mut egui::Ui) {
        ui.heading("Run on device / simulator");
        ui.separator();

        // ── Target picker ────────────────────────────────────────────
        ui.horizontal(|ui| {
            ui.label("Target:");
            let label = match &self.mobile_toolchain.run_target {
                RunTargetSelection::None => "(none)".to_string(),
                RunTargetSelection::IosSim { udid } => format!("iOS Sim {}", short_id(udid)),
                RunTargetSelection::IosDevice { udid } => {
                    format!("iOS Device {}", short_id(udid))
                }
                RunTargetSelection::Android { serial } => format!("Android {}", serial),
            };
            egui::ComboBox::from_id_salt("mobile_run_target")
                .selected_text(label)
                .width(280.0)
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.mobile_toolchain.run_target,
                        RunTargetSelection::None,
                        "(none)",
                    );
                    if let Some(xcode) = &self.mobile_toolchain.toolchain.xcode {
                        for sim in xcode
                            .simulators
                            .iter()
                            .filter(|s| s.family == mobile::SimFamily::Ios)
                        {
                            ui.selectable_value(
                                &mut self.mobile_toolchain.run_target,
                                RunTargetSelection::IosSim {
                                    udid: sim.udid.clone(),
                                },
                                format!("iOS Sim — {} ({})", sim.name, sim.runtime),
                            );
                        }
                    }
                    if let Some(android) = &self.mobile_toolchain.toolchain.android {
                        for d in &android.devices {
                            let auth = if d.authorised { "" } else { " (unauthorised)" };
                            let label = if d.model.is_empty() {
                                format!("Android — {}{}", d.serial, auth)
                            } else {
                                format!("Android — {} {}{}", d.model, d.serial, auth)
                            };
                            ui.selectable_value(
                                &mut self.mobile_toolchain.run_target,
                                RunTargetSelection::Android {
                                    serial: d.serial.clone(),
                                },
                                label,
                            );
                        }
                    }
                });
        });

        // ── Artifact picker ──────────────────────────────────────────
        ui.horizontal(|ui| {
            ui.label("Artifact:");
            let mut s = self.mobile_toolchain.run_artifact.display().to_string();
            if ui
                .add(
                    egui::TextEdit::singleline(&mut s)
                        .desired_width(360.0)
                        .hint_text("path to .app / .apk"),
                )
                .changed()
            {
                self.mobile_toolchain.run_artifact = PathBuf::from(s);
            }
            if ui.small_button("Browse…").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Mobile bundle", &["app", "apk", "ipa"])
                    .pick_file()
                {
                    self.mobile_toolchain.run_artifact = path;
                }
            }
        });

        // ── Bundle / package metadata ─────────────────────────────────
        let is_android = matches!(
            self.mobile_toolchain.run_target,
            RunTargetSelection::Android { .. }
        );
        if is_android {
            ui.horizontal(|ui| {
                ui.label("Package:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.mobile_toolchain.run_package_name)
                        .desired_width(220.0)
                        .hint_text("com.example.game"),
                );
                ui.label("Activity:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.mobile_toolchain.run_activity)
                        .desired_width(180.0)
                        .hint_text("MainActivity"),
                );
            });
        } else {
            ui.horizontal(|ui| {
                ui.label("Bundle ID:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.mobile_toolchain.run_bundle_id)
                        .desired_width(360.0)
                        .hint_text("com.example.Game"),
                );
            });
        }

        // ── Run / Stop / Clear ───────────────────────────────────────
        ui.horizontal(|ui| {
            let running = self
                .mobile_toolchain
                .run_session
                .as_ref()
                .map(|s| s.is_running())
                .unwrap_or(false);

            if running {
                if ui.button("Stop").clicked() {
                    if let Some(s) = self.mobile_toolchain.run_session.as_mut() {
                        s.stop();
                    }
                }
                ui.colored_label(egui::Color32::from_rgb(120, 200, 120), "● running");
            } else {
                let ready = !matches!(self.mobile_toolchain.run_target, RunTargetSelection::None)
                    && self.mobile_toolchain.run_artifact.as_os_str().len() > 0;
                if ui.add_enabled(ready, egui::Button::new("Run")).clicked() {
                    self.start_mobile_run();
                }
            }
            if ui.button("Clear log").clicked() {
                self.mobile_toolchain.run_log.clear();
            }
            ui.label(format!("{} lines", self.mobile_toolchain.run_log.len()));
        });

        if let Some(err) = &self.mobile_toolchain.run_error {
            ui.colored_label(egui::Color32::from_rgb(220, 80, 80), err);
        }

        ui.add_space(4.0);
        // ── Log stream ───────────────────────────────────────────────
        egui::ScrollArea::vertical()
            .id_salt("mobile_run_log")
            .max_height(220.0)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for line in self.mobile_toolchain.run_log.iter() {
                    let color = match line.severity {
                        mobile::LogSeverity::Panic => egui::Color32::from_rgb(255, 100, 200),
                        mobile::LogSeverity::Error => egui::Color32::from_rgb(220, 80, 80),
                        mobile::LogSeverity::Warn => egui::Color32::from_rgb(220, 180, 80),
                        mobile::LogSeverity::Info => egui::Color32::from_rgb(180, 200, 220),
                        mobile::LogSeverity::Debug => egui::Color32::from_gray(170),
                        mobile::LogSeverity::Trace => egui::Color32::from_gray(140),
                        mobile::LogSeverity::Unknown => egui::Color32::from_gray(200),
                    };
                    ui.label(
                        egui::RichText::new(&line.text)
                            .color(color)
                            .font(egui::FontId::monospace(11.0)),
                    );
                }
            });
    }

    fn start_mobile_run(&mut self) {
        self.mobile_toolchain.run_error = None;
        let Some(target) = self.mobile_toolchain.build_target() else {
            self.mobile_toolchain.run_error = Some("Pick a target first.".into());
            return;
        };
        let artifact = self.mobile_toolchain.run_artifact.clone();
        match mobile::start_run(target, artifact) {
            Ok(session) => {
                self.mobile_toolchain
                    .run_log
                    .push_line("─── Launching ───".into());
                self.mobile_toolchain.run_session = Some(session);
            }
            Err(e) => {
                self.mobile_toolchain.run_error = Some(e);
            }
        }
    }

    fn render_xcode_section(&mut self, ui: &mut egui::Ui) {
        // Step 1: figure out what to render and what action (if any) the user
        // requested. We can't both borrow `self.mobile_toolchain.toolchain`
        // immutably for rendering and call `self.refresh_codesign_identities()`
        // (which needs `&mut self`), so we scope the read borrow tightly and
        // perform the refresh after the closure returns.
        let mut want_refresh_identities = false;
        match &self.mobile_toolchain.toolchain.xcode {
            Some(xcode) => {
                row_status(ui, "Xcode", true, &xcode.version);
                ui.indent("xcode_detail", |ui| {
                    ui.label(format!("Developer dir: {}", xcode.developer_dir.display()));
                    ui.label(format!("SDKs: {}", xcode.sdks.len()));
                    if !xcode.sdks.is_empty() {
                        ui.collapsing("Show SDKs", |ui| {
                            for s in &xcode.sdks {
                                ui.label(s);
                            }
                        });
                    }
                    let ios_sims = xcode
                        .simulators
                        .iter()
                        .filter(|s| s.family == mobile::SimFamily::Ios)
                        .count();
                    let xr_sims = xcode
                        .simulators
                        .iter()
                        .filter(|s| s.family == mobile::SimFamily::VisionOs)
                        .count();
                    ui.label(format!("Simulators: {ios_sims} iOS, {xr_sims} visionOS"));
                    if !xcode.simulators.is_empty() {
                        ui.collapsing("Show simulators", |ui| {
                            for sim in &xcode.simulators {
                                let dot = match sim.state {
                                    mobile::SimState::Booted => "🟢",
                                    _ => "⚪",
                                };
                                ui.label(format!(
                                    "{dot} {} — {} ({})",
                                    sim.name, sim.runtime, sim.udid
                                ));
                            }
                        });
                    }

                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label("Codesign identities:");
                        if xcode.codesign_identities.is_empty() {
                            ui.colored_label(egui::Color32::from_gray(170), "not loaded");
                        } else {
                            ui.label(format!("{}", xcode.codesign_identities.len()));
                        }
                        if ui.small_button("Refresh identities").clicked() {
                            want_refresh_identities = true;
                        }
                    });
                    if !xcode.codesign_identities.is_empty() {
                        ui.collapsing("Show identities", |ui| {
                            for id in &xcode.codesign_identities {
                                ui.label(format!(
                                    "{} — {}",
                                    &id.id[..id.id.len().min(10)],
                                    id.common_name
                                ));
                            }
                        });
                    }
                });
            }
            None => {
                row_status(ui, "Xcode", false, "not detected");
                if cfg!(target_os = "macos") {
                    install_hint(ui, "xcode-select --install");
                } else {
                    ui.label(
                        egui::RichText::new(
                            "iOS / visionOS builds require macOS — \
                             this section stays disabled on other hosts.",
                        )
                        .italics()
                        .color(egui::Color32::from_gray(150)),
                    );
                }
            }
        }

        if want_refresh_identities {
            self.mobile_toolchain.refresh_codesign_identities();
        }
    }

    fn render_android_section(&mut self, ui: &mut egui::Ui) {
        let mut want_refresh_devices = false;
        match &self.mobile_toolchain.toolchain.android {
            Some(a) => {
                row_status(ui, "Android SDK", true, &a.sdk_root.display().to_string());
                ui.indent("android_detail", |ui| {
                    ui.label(format!(
                        "Platforms: {}",
                        if a.platforms.is_empty() {
                            "none".into()
                        } else {
                            a.platforms
                                .iter()
                                .map(|v| format!("android-{v}"))
                                .collect::<Vec<_>>()
                                .join(", ")
                        }
                    ));
                    ui.label(format!(
                        "Build tools: {}",
                        if a.build_tools.is_empty() {
                            "none".into()
                        } else {
                            a.build_tools.join(", ")
                        }
                    ));
                    match &a.ndk {
                        Some(ndk) => {
                            ui.label(format!("NDK: {} ({})", ndk.version, ndk.root.display()))
                        }
                        None => ui.colored_label(
                            egui::Color32::from_rgb(220, 160, 80),
                            "NDK: not installed (sdkmanager 'ndk;<version>')",
                        ),
                    };
                    match &a.adb {
                        Some(adb) => ui.label(format!("adb: {}", adb.display())),
                        None => ui.colored_label(
                            egui::Color32::from_rgb(220, 160, 80),
                            "adb: missing (sdkmanager 'platform-tools')",
                        ),
                    };

                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label("Devices:");
                        if a.devices.is_empty() {
                            ui.colored_label(egui::Color32::from_gray(170), "not loaded");
                        } else {
                            ui.label(format!("{}", a.devices.len()));
                        }
                        // The button is wired even when adb is missing so the
                        // user gets a clear "no adb → nothing to refresh"
                        // experience rather than a phantom-clicked-but-nothing-
                        // happened.
                        let enabled = a.adb.is_some();
                        if ui
                            .add_enabled(enabled, egui::Button::new("Refresh devices").small())
                            .clicked()
                        {
                            want_refresh_devices = true;
                        }
                    });
                    if !a.devices.is_empty() {
                        ui.collapsing("Show devices", |ui| {
                            for d in &a.devices {
                                let dot = if d.authorised { "🟢" } else { "🟠" };
                                let model = if d.model.is_empty() {
                                    "(unknown model)"
                                } else {
                                    d.model.as_str()
                                };
                                let auth = if d.authorised {
                                    "device"
                                } else {
                                    "unauthorised"
                                };
                                ui.label(format!("{dot} {} — {} ({auth})", d.serial, model));
                            }
                        });
                    }
                });
            }
            None => {
                row_status(ui, "Android SDK", false, "not detected");
                install_hint(
                    ui,
                    "Install Android Studio or set $ANDROID_HOME to the SDK root.",
                );
            }
        }

        if want_refresh_devices {
            self.mobile_toolchain.refresh_adb_devices();
        }
    }

    fn render_rust_targets_section(&mut self, ui: &mut egui::Ui) {
        ui.heading("Rust mobile targets");
        ui.separator();
        let mut missing: Vec<&str> = Vec::new();
        for (platform, triple) in MOBILE_TRIPLES {
            let installed = self.mobile_toolchain.toolchain.has_rust_target(triple);
            if !installed {
                missing.push(triple);
            }
            row_status(
                ui,
                platform.label(),
                installed,
                if installed { triple } else { triple },
            );
        }
        // Quest reuses the Android target; flag it with a clarifying note
        // rather than a separate row that can disagree.
        ui.label(
            egui::RichText::new(
                "Meta Quest reuses aarch64-linux-android — no extra rustup target needed.",
            )
            .italics()
            .color(egui::Color32::from_gray(150)),
        );
        if !missing.is_empty() {
            ui.separator();
            install_hint(ui, &format!("rustup target add {}", missing.join(" ")));
        }
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────

fn short_id(s: &str) -> String {
    if s.len() > 8 {
        format!("{}…", &s[..8])
    } else {
        s.to_string()
    }
}

fn row_status(ui: &mut egui::Ui, label: &str, ok: bool, detail: &str) {
    ui.horizontal(|ui| {
        let (mark, color) = if ok {
            ("✔", egui::Color32::from_rgb(120, 200, 120))
        } else {
            ("✖", egui::Color32::from_rgb(220, 80, 80))
        };
        ui.colored_label(color, mark);
        ui.label(egui::RichText::new(label).strong());
        ui.label(egui::RichText::new(detail).color(egui::Color32::from_gray(170)));
    });
}

fn install_hint(ui: &mut egui::Ui, command: &str) {
    ui.horizontal(|ui| {
        ui.label("Install:");
        let mut text = command.to_string();
        // Read-only single-line display + copy button. The copy uses egui's
        // built-in clipboard so the user can paste into a terminal without
        // hand-typing the line.
        ui.add(
            egui::TextEdit::singleline(&mut text)
                .desired_width(f32::INFINITY)
                .interactive(false)
                .font(egui::TextStyle::Monospace),
        );
        if ui.small_button("Copy").clicked() {
            ui.ctx().copy_text(command.to_string());
        }
    });
}
