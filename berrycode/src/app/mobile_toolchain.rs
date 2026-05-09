//! Mobile Toolchain panel — Phase A's user-facing surface.
//!
//! Three rows (iOS / Android / Rust targets), each with green-tick / red-x
//! status and a copy-paste install hint when something's missing. The actual
//! probing lives in `app::mobile::probe`; this file is just the egui shell.

use super::mobile::one_click::{
    self, download_ios_simulator_runtime, install_cargo_mobile, install_rust_targets,
    OneClickConfig, OneClickError,
};
use super::mobile::{self, LogStream, MobileRunSession, MobileTarget, MobileToolchain};
use super::scene_editor::build_settings::Platform;
use super::BerryCodeApp;
use super::{button_style, ui_colors};
use std::path::PathBuf;
use std::process::Child;
use std::sync::mpsc::Receiver;

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

    /// Active `cargo mobile` child + log channel from the one-click flow
    /// (install / init / run). Distinct from `run_session` so the legacy
    /// pre-built-artifact path keeps working unchanged.
    pub one_click_session: Option<OneClickSession>,
    /// Cached `cargo mobile --version` probe result. `None` means not yet
    /// probed; `Some(b)` is the last result. Probe spawns a `cargo` child,
    /// so caching is critical — without it the panel re-spawns the process
    /// every frame and the UI freezes. Invalidated by the Refresh button
    /// and after a one-click session ends (install / init / run can flip
    /// the install state).
    pub one_click_installed: Option<bool>,

    // ── Doctor modal ──────────────────────────────────────────────────
    /// Last doctor report rendered as Markdown-flavoured plain text.
    /// Empty until the user clicks "Doctor"; persists so re-opening the
    /// modal shows the same report without re-probing.
    pub doctor_report: String,
    pub doctor_open: bool,
}

/// Owns the live `cargo mobile` child and its stdout/stderr channel.
/// Mirrors `MobileRunSession`'s shape so the panel polls both kinds of
/// session uniformly each frame.
pub struct OneClickSession {
    pub stage: OneClickStageLabel,
    child: Option<Child>,
    rx: Option<Receiver<String>>,
}

/// Snapshot of `OneClickStage` that survives the move into the session
/// (so the UI can show "Installing cargo-mobile2…" while the child is
/// still alive).
#[derive(Debug, Clone, Copy)]
pub enum OneClickStageLabel {
    Install,
    Init,
    RunIos,
    RunAndroid,
    InstallRustTargets,
    DownloadIosSimRuntime,
}

impl OneClickStageLabel {
    fn human(self) -> &'static str {
        match self {
            OneClickStageLabel::Install => "Installing cargo-mobile2",
            OneClickStageLabel::Init => "Initializing mobile project",
            OneClickStageLabel::RunIos => "Running on iOS Simulator",
            OneClickStageLabel::RunAndroid => "Running on Android",
            OneClickStageLabel::InstallRustTargets => "Installing rustup targets",
            OneClickStageLabel::DownloadIosSimRuntime => "Downloading iOS Simulator runtime",
        }
    }
}

impl std::fmt::Debug for OneClickSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OneClickSession")
            .field("stage", &self.stage)
            .field("running", &self.child.is_some())
            .finish()
    }
}

impl OneClickSession {
    pub fn new(stage: OneClickStageLabel, pair: (Child, Receiver<String>)) -> Self {
        let (child, rx) = pair;
        Self {
            stage,
            child: Some(child),
            rx: Some(rx),
        }
    }

    /// Drain pending lines into `stream`, then check whether the child
    /// has exited. Returns `true` while the session is still live.
    pub fn poll_into(&mut self, stream: &mut LogStream) -> bool {
        if let Some(rx) = &self.rx {
            for _ in 0..256 {
                match rx.try_recv() {
                    Ok(line) => stream.push_line(line),
                    Err(_) => break,
                }
            }
        }
        if let Some(child) = &mut self.child {
            if let Ok(Some(status)) = child.try_wait() {
                stream.push_line(format!(
                    "─── {} exited with code {:?} ───",
                    self.stage.human(),
                    status.code()
                ));
                self.child = None;
                self.rx = None;
                return false;
            }
        }
        self.child.is_some()
    }

    pub fn is_running(&self) -> bool {
        self.child.is_some()
    }

    pub fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.rx = None;
    }
}

impl Drop for OneClickSession {
    fn drop(&mut self) {
        self.stop();
    }
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
        if let Some(session) = self.one_click_session.as_mut() {
            let still_running = session.poll_into(&mut self.run_log);
            if !still_running {
                // Drop the dead session so the next click can start a
                // new one. Logs already landed in `run_log` and stay
                // visible after the session goes away. Also invalidate
                // the cargo-mobile2 install cache: install obviously
                // changes it, and init/run may have run a self-update.
                self.one_click_session = None;
                self.one_click_installed = None;
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
            .frame(tool_window_frame())
            .show(ctx, |ui| {
                self.render_mobile_toolchain(ui);
            });
        self.mobile_toolchain_open = open;
    }

    fn render_mobile_toolchain(&mut self, ui: &mut egui::Ui) {
        let mut want_doctor = false;
        panel_header(ui, "MOBILE TOOLCHAIN", |ui| {
            if button_style::button_with_icon(ui, button_style::glyph::REFRESH, "Refresh").clicked()
            {
                self.mobile_toolchain.refresh();
                self.mobile_toolchain.one_click_installed = None;
            }
            if button_style::button_with_icon(ui, "\u{ea6a}", "Doctor")
                .on_hover_text("Run a full diagnostic and show what's missing + next steps")
                .clicked()
            {
                want_doctor = true;
            }
        });
        if want_doctor {
            self.run_mobile_doctor();
        }

        if let Some(err) = &self.mobile_toolchain.last_error {
            error_banner(ui, err);
            ui.add_space(8.0);
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            section_frame(ui, |ui| self.render_xcode_section(ui));
            ui.add_space(8.0);
            section_frame(ui, |ui| self.render_android_section(ui));
            ui.add_space(8.0);
            section_frame(ui, |ui| self.render_rust_targets_section(ui));
            ui.add_space(12.0);
            section_frame(ui, |ui| self.render_one_click_section(ui));
            ui.add_space(12.0);
            section_frame(ui, |ui| self.render_run_section(ui));
        });
    }

    fn render_one_click_section(&mut self, ui: &mut egui::Ui) {
        section_title(ui, "One-click mobile run");
        ui.label(
            egui::RichText::new(
                "Wraps cargo-mobile2: probe → install → init → cargo apple/android run.",
            )
            .small()
            .color(ui_colors::TEXT_MUTED),
        );
        ui.add_space(4.0);

        // Probe `cargo mobile` lazily and cache the result. Probing spawns
        // a `cargo` subprocess (~100–500ms), so doing it per-frame freezes
        // the UI. Refresh / session-end invalidate the cache.
        let installed = *self
            .mobile_toolchain
            .one_click_installed
            .get_or_insert_with(one_click::probe_cargo_mobile);
        let mobile_toml_exists = std::path::Path::new(&self.root_path)
            .join("mobile.toml")
            .exists();
        let busy = self
            .mobile_toolchain
            .one_click_session
            .as_ref()
            .map(|s| s.is_running())
            .unwrap_or(false);

        ui.horizontal_wrapped(|ui| {
            status_chip(ui, installed, "cargo-mobile2 installed");
            status_chip(ui, mobile_toml_exists, "mobile.toml present");
        });

        ui.add_space(4.0);

        ui.horizontal_wrapped(|ui| {
            let install_label = if busy
                && matches!(
                    self.mobile_toolchain
                        .one_click_session
                        .as_ref()
                        .map(|s| s.stage),
                    Some(OneClickStageLabel::Install)
                ) {
                "Installing..."
            } else {
                "Install cargo-mobile2"
            };
            if ui
                .add_enabled(!installed && !busy, egui::Button::new(install_label))
                .clicked()
            {
                self.start_one_click_install();
            }
            if ui
                .add_enabled(
                    installed && !mobile_toml_exists && !busy,
                    egui::Button::new("Initialize for Mobile"),
                )
                .clicked()
            {
                self.start_one_click_init();
            }
            #[cfg(target_os = "macos")]
            if ui
                .add_enabled(installed && !busy, egui::Button::new("Run on iOS Sim"))
                .clicked()
            {
                self.start_one_click_ios();
            }
            #[cfg(not(target_os = "macos"))]
            ui.add_enabled(false, egui::Button::new("Run on iOS Sim (macOS only)"));
            if ui
                .add_enabled(installed && !busy, egui::Button::new("Run on Android"))
                .clicked()
            {
                self.start_one_click_android();
            }
            if busy {
                if button_style::button_with_icon(ui, button_style::glyph::STOP, "Stop").clicked() {
                    if let Some(s) = self.mobile_toolchain.one_click_session.as_mut() {
                        s.stop();
                    }
                }
            }
        });
    }

    fn run_mobile_doctor(&mut self) {
        // Make sure the snapshot we report against is current. `refresh`
        // is cheap aside from the simulator probe which it caches; the
        // user explicitly asked to see the report, so a re-probe is the
        // expected behaviour.
        self.mobile_toolchain.refresh();
        // Re-probe `cargo mobile` too so the report can't claim it's
        // installed when the user just deleted it.
        let installed = one_click::probe_cargo_mobile();
        self.mobile_toolchain.one_click_installed = Some(installed);
        let project_root = std::path::Path::new(&self.root_path);
        self.mobile_toolchain.doctor_report =
            build_doctor_report(&self.mobile_toolchain.toolchain, project_root, installed);
        self.mobile_toolchain.doctor_open = true;
    }

    pub(crate) fn render_mobile_doctor_modal(&mut self, ctx: &egui::Context) {
        if !self.mobile_toolchain.doctor_open {
            return;
        }
        let mut open = self.mobile_toolchain.doctor_open;
        egui::Window::new("Mobile Doctor")
            .id(egui::Id::new("mobile_doctor_modal_v1"))
            .open(&mut open)
            .resizable(true)
            .default_size([640.0, 520.0])
            .frame(tool_window_frame())
            .show(ctx, |ui| {
                panel_header(ui, "MOBILE DOCTOR", |ui| {
                    if button_style::button_with_icon(ui, "\u{ebcc}", "Copy").clicked() {
                        ui.ctx()
                            .copy_text(self.mobile_toolchain.doctor_report.clone());
                    }
                    if button_style::button_with_icon(ui, button_style::glyph::REFRESH, "Re-run")
                        .clicked()
                    {
                        // Trigger a fresh report next frame — can't call
                        // `run_mobile_doctor` here because we're already
                        // inside `&mut self` via the window builder.
                        self.mobile_toolchain.doctor_report.clear();
                    }
                });
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        // Plain monospace text edit so the user can
                        // select / copy any line. Read-only via the
                        // immutable scratch buffer trick.
                        let mut text = self.mobile_toolchain.doctor_report.clone();
                        ui.add(
                            egui::TextEdit::multiline(&mut text)
                                .font(egui::TextStyle::Monospace)
                                .desired_width(f32::INFINITY)
                                .desired_rows(20)
                                .code_editor(),
                        );
                    });
            });
        self.mobile_toolchain.doctor_open = open;
        // Re-run was requested from the button; recompute now that the
        // borrow on `doctor_open` is released.
        if self.mobile_toolchain.doctor_open && self.mobile_toolchain.doctor_report.is_empty() {
            self.run_mobile_doctor();
        }
    }

    fn start_one_click_install(&mut self) {
        self.mobile_toolchain.run_error = None;
        self.mobile_toolchain
            .run_log
            .push_line("─── Installing cargo-mobile2 (this can take a few minutes) ───".into());
        match install_cargo_mobile() {
            Ok(pair) => {
                self.mobile_toolchain.one_click_session =
                    Some(OneClickSession::new(OneClickStageLabel::Install, pair));
            }
            Err(e) => {
                self.mobile_toolchain.run_error = Some(format_one_click_err(&e));
            }
        }
    }

    fn start_install_rust_targets(&mut self, triples: Vec<String>) {
        self.mobile_toolchain.run_error = None;
        self.mobile_toolchain
            .run_log
            .push_line(format!("─── rustup target add {} ───", triples.join(" ")));
        let refs: Vec<&str> = triples.iter().map(String::as_str).collect();
        match install_rust_targets(&refs) {
            Ok(pair) => {
                self.mobile_toolchain.one_click_session = Some(OneClickSession::new(
                    OneClickStageLabel::InstallRustTargets,
                    pair,
                ));
            }
            Err(e) => {
                self.mobile_toolchain.run_error = Some(format_one_click_err(&e));
            }
        }
    }

    #[cfg(target_os = "macos")]
    fn start_download_ios_sim_runtime(&mut self) {
        self.mobile_toolchain.run_error = None;
        self.mobile_toolchain.run_log.push_line(
            "─── xcodebuild -downloadPlatform iOS (several GB, can take 10+ min) ───".into(),
        );
        match download_ios_simulator_runtime() {
            Ok(pair) => {
                self.mobile_toolchain.one_click_session = Some(OneClickSession::new(
                    OneClickStageLabel::DownloadIosSimRuntime,
                    pair,
                ));
            }
            Err(e) => {
                self.mobile_toolchain.run_error = Some(format_one_click_err(&e));
            }
        }
    }

    fn start_one_click_init(&mut self) {
        self.mobile_toolchain.run_error = None;
        let project_root = std::path::PathBuf::from(&self.root_path);
        let project_name = project_root
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("game")
            .to_string();
        let cfg = OneClickConfig {
            project_root: &project_root,
            project_name: &project_name,
            ios_bundle_id: &self.build_settings.ios_bundle_id,
            android_package_name: &self.build_settings.android_package_name,
        };
        match one_click::init_mobile_project_if_needed(&cfg) {
            Ok(Some(pair)) => {
                self.mobile_toolchain
                    .run_log
                    .push_line("─── cargo mobile init ───".into());
                self.mobile_toolchain.one_click_session =
                    Some(OneClickSession::new(OneClickStageLabel::Init, pair));
            }
            Ok(None) => {
                self.mobile_toolchain
                    .run_log
                    .push_line("mobile.toml already present — skipping init.".into());
            }
            Err(e) => {
                self.mobile_toolchain.run_error = Some(format_one_click_err(&e));
            }
        }
    }

    fn start_one_click_ios(&mut self) {
        self.mobile_toolchain.run_error = None;
        let project_root = std::path::PathBuf::from(&self.root_path);
        let project_name = project_root
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("game")
            .to_string();
        let cfg = OneClickConfig {
            project_root: &project_root,
            project_name: &project_name,
            ios_bundle_id: &self.build_settings.ios_bundle_id,
            android_package_name: &self.build_settings.android_package_name,
        };
        match one_click::run_on_ios_simulator(&cfg) {
            Ok(pair) => {
                self.mobile_toolchain
                    .run_log
                    .push_line("─── cargo apple run --release ───".into());
                self.mobile_toolchain.one_click_session =
                    Some(OneClickSession::new(OneClickStageLabel::RunIos, pair));
            }
            Err(e) => {
                self.mobile_toolchain.run_error = Some(format_one_click_err(&e));
            }
        }
    }

    fn start_one_click_android(&mut self) {
        self.mobile_toolchain.run_error = None;
        let project_root = std::path::PathBuf::from(&self.root_path);
        let project_name = project_root
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("game")
            .to_string();
        let cfg = OneClickConfig {
            project_root: &project_root,
            project_name: &project_name,
            ios_bundle_id: &self.build_settings.ios_bundle_id,
            android_package_name: &self.build_settings.android_package_name,
        };
        match one_click::run_on_android(&cfg) {
            Ok(pair) => {
                self.mobile_toolchain
                    .run_log
                    .push_line("─── cargo android run --release ───".into());
                self.mobile_toolchain.one_click_session =
                    Some(OneClickSession::new(OneClickStageLabel::RunAndroid, pair));
            }
            Err(e) => {
                self.mobile_toolchain.run_error = Some(format_one_click_err(&e));
            }
        }
    }

    fn render_run_section(&mut self, ui: &mut egui::Ui) {
        section_title(ui, "Run on device / simulator");

        // ── Target picker ────────────────────────────────────────────
        ui.horizontal_wrapped(|ui| {
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
            if button_style::button_with_icon(ui, button_style::glyph::FOLDER_OPENED, "Browse")
                .clicked()
            {
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
        ui.horizontal_wrapped(|ui| {
            let running = self
                .mobile_toolchain
                .run_session
                .as_ref()
                .map(|s| s.is_running())
                .unwrap_or(false);

            if running {
                if button_style::button_with_icon(ui, button_style::glyph::STOP, "Stop").clicked() {
                    if let Some(s) = self.mobile_toolchain.run_session.as_mut() {
                        s.stop();
                    }
                }
                ui.colored_label(egui::Color32::from_rgb(120, 200, 120), "running");
            } else {
                let ready = !matches!(self.mobile_toolchain.run_target, RunTargetSelection::None)
                    && self.mobile_toolchain.run_artifact.as_os_str().len() > 0;
                if ui.add_enabled(ready, egui::Button::new("Run")).clicked() {
                    self.start_mobile_run();
                }
            }
            if button_style::button_with_icon(ui, button_style::glyph::CLEAR_ALL, "Clear log")
                .clicked()
            {
                self.mobile_toolchain.run_log.clear();
            }
            ui.label(
                egui::RichText::new(format!("{} lines", self.mobile_toolchain.run_log.len()))
                    .small()
                    .color(ui_colors::TEXT_MUTED),
            );
        });

        if let Some(err) = &self.mobile_toolchain.run_error {
            error_banner(ui, err);
        }

        ui.add_space(4.0);
        // ── Log stream ───────────────────────────────────────────────
        inset_frame().show(ui, |ui| {
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
        // Pre-compute the one_click_session state outside the immutable
        // borrow on `self.mobile_toolchain.toolchain.xcode` below; we
        // can't call &mut self methods (start_download_ios_sim_runtime)
        // while that borrow is still live, and rust-analyzer also can't
        // see through nested closures to relax the rule.
        let download_busy = self
            .mobile_toolchain
            .one_click_session
            .as_ref()
            .map(|s| s.is_running())
            .unwrap_or(false);
        let download_button_label = if download_busy
            && matches!(
                self.mobile_toolchain
                    .one_click_session
                    .as_ref()
                    .map(|s| s.stage),
                Some(OneClickStageLabel::DownloadIosSimRuntime)
            ) {
            "Downloading…"
        } else {
            "Download iOS Simulator runtime"
        };
        let mut want_refresh_identities = false;
        let mut want_download_ios_sim = false;
        section_title(ui, "Xcode");
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
                    ui.horizontal(|ui| {
                        ui.label(format!("Simulators: {ios_sims} iOS, {xr_sims} visionOS"));
                        if ios_sims == 0
                            && ui
                                .add_enabled(
                                    !download_busy,
                                    egui::Button::new(download_button_label),
                                )
                                .on_hover_text(
                                    "Runs `xcodebuild -downloadPlatform iOS` (several GB)",
                                )
                                .clicked()
                        {
                            want_download_ios_sim = true;
                        }
                    });
                    if !xcode.simulators.is_empty() {
                        ui.collapsing("Show simulators", |ui| {
                            for sim in &xcode.simulators {
                                let dot = match sim.state {
                                    mobile::SimState::Booted => "online",
                                    _ => "offline",
                                };
                                ui.label(format!(
                                    "{dot}  {} — {} ({})",
                                    sim.name, sim.runtime, sim.udid
                                ));
                            }
                        });
                    }
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label("Codesign identities:");
                        if xcode.codesign_identities.is_empty() {
                            ui.colored_label(ui_colors::TEXT_MUTED, "not loaded");
                        } else {
                            ui.label(format!("{}", xcode.codesign_identities.len()));
                        }
                        if button_style::button_with_icon(
                            ui,
                            button_style::glyph::REFRESH,
                            "Refresh identities",
                        )
                        .clicked()
                        {
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
                        .color(ui_colors::TEXT_MUTED),
                    );
                }
            }
        }

        if want_refresh_identities {
            self.mobile_toolchain.refresh_codesign_identities();
        }
        if want_download_ios_sim {
            #[cfg(target_os = "macos")]
            self.start_download_ios_sim_runtime();
        }
    }

    fn render_android_section(&mut self, ui: &mut egui::Ui) {
        let mut want_refresh_devices = false;
        section_title(ui, "Android SDK");
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
                            ui.colored_label(ui_colors::TEXT_MUTED, "not loaded");
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
                                let dot = if d.authorised {
                                    "online"
                                } else {
                                    "unauthorised"
                                };
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
                                ui.label(format!("{dot}  {} — {} ({auth})", d.serial, model));
                            }
                        });
                    }
                });
            }
            None => {
                row_status(ui, "Android SDK", false, "not detected");
                setup_hint(
                    ui,
                    "Install Android Studio, then set ANDROID_HOME to the SDK root if BerryCode cannot find it automatically.",
                );
            }
        }

        if want_refresh_devices {
            self.mobile_toolchain.refresh_adb_devices();
        }
    }

    fn render_rust_targets_section(&mut self, ui: &mut egui::Ui) {
        section_title(ui, "Rust mobile targets");
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
            .color(ui_colors::TEXT_MUTED),
        );
        if !missing.is_empty() {
            ui.separator();
            install_hint(ui, &format!("rustup target add {}", missing.join(" ")));
            let busy = self
                .mobile_toolchain
                .one_click_session
                .as_ref()
                .map(|s| s.is_running())
                .unwrap_or(false);
            let button_label = if busy
                && matches!(
                    self.mobile_toolchain
                        .one_click_session
                        .as_ref()
                        .map(|s| s.stage),
                    Some(OneClickStageLabel::InstallRustTargets)
                ) {
                "Installing targets..."
            } else {
                "Install missing targets"
            };
            if ui
                .add_enabled(!busy, egui::Button::new(button_label))
                .on_hover_text("Runs `rustup target add ...` for the missing triples")
                .clicked()
            {
                let owned: Vec<String> = missing.iter().map(|s| s.to_string()).collect();
                self.start_install_rust_targets(owned);
            }
        }
    }
}

// ─── Doctor report ────────────────────────────────────────────────────────

/// Generate a Flutter-doctor-style diagnostic report from the current
/// toolchain snapshot + project state. Pure function (no I/O, no probing)
/// so it stays fast and easy to unit test — the caller is responsible for
/// running `MobileToolchain::refresh()` first if a fresh probe is wanted.
pub fn build_doctor_report(
    toolchain: &MobileToolchain,
    project_root: &std::path::Path,
    one_click_installed: bool,
) -> String {
    let mut out = String::new();
    let mut issues: Vec<String> = Vec::new();
    let mut next_steps: Vec<String> = Vec::new();

    out.push_str("# BerryCode Mobile Doctor\n\n");

    // ── Xcode ────────────────────────────────────────────────────────
    out.push_str("## Xcode\n");
    match &toolchain.xcode {
        Some(x) => {
            out.push_str(&format!("- ✓ Xcode {}\n", x.version));
            out.push_str(&format!(
                "- Developer dir: `{}`\n",
                x.developer_dir.display()
            ));
            out.push_str(&format!("- SDKs: {}\n", x.sdks.len()));
            let ios_sims = x
                .simulators
                .iter()
                .filter(|s| s.family == mobile::SimFamily::Ios)
                .count();
            let xr_sims = x
                .simulators
                .iter()
                .filter(|s| s.family == mobile::SimFamily::VisionOs)
                .count();
            out.push_str(&format!(
                "- Simulators: {ios_sims} iOS, {xr_sims} visionOS\n"
            ));
            if ios_sims == 0 {
                issues.push("No iOS Simulator runtime installed".into());
                next_steps.push(
                    "Download an iOS Simulator runtime: `xcodebuild -downloadPlatform iOS` \
                     (or click \"Download iOS Simulator runtime\" in this panel)"
                        .into(),
                );
            }
            out.push_str(&format!(
                "- Codesign identities: {}\n",
                x.codesign_identities.len()
            ));
            if x.codesign_identities.is_empty() {
                // Codesign is only required for real-device deploys, so
                // we surface it as advisory rather than a hard blocker.
                next_steps.push(
                    "(optional, real-device only) Sign in with your Apple ID in Xcode \
                     to populate codesign identities."
                        .into(),
                );
            }
        }
        None => {
            out.push_str("- ✗ Xcode not detected\n");
            issues.push("Xcode missing".into());
            if cfg!(target_os = "macos") {
                next_steps.push("Install Xcode: `xcode-select --install`".into());
            } else {
                next_steps.push(
                    "iOS / visionOS builds require macOS — switch host or skip these targets."
                        .into(),
                );
            }
        }
    }

    // ── Android ──────────────────────────────────────────────────────
    out.push_str("\n## Android SDK\n");
    match &toolchain.android {
        Some(a) => {
            out.push_str(&format!("- ✓ SDK root: `{}`\n", a.sdk_root.display()));
            if let Some(ndk) = &a.ndk {
                out.push_str(&format!(
                    "- NDK {} at `{}`\n",
                    ndk.version,
                    ndk.root.display()
                ));
            } else {
                out.push_str("- ✗ NDK missing\n");
                issues.push("Android NDK missing".into());
                next_steps.push(
                    "Install NDK via Android Studio (SDK Manager → SDK Tools → NDK (Side by side))."
                        .into(),
                );
            }
            if a.adb.is_none() {
                out.push_str("- ✗ adb not found in platform-tools\n");
                issues.push("`adb` missing".into());
                next_steps.push("Install platform-tools via Android Studio SDK Manager.".into());
            }
        }
        None => {
            out.push_str("- ✗ Android SDK not detected\n");
            issues.push("Android SDK missing".into());
            next_steps.push(
                "Install Android Studio (or the command-line tools) and set `$ANDROID_HOME` \
                 to the SDK root."
                    .into(),
            );
        }
    }

    // ── Rust mobile targets ──────────────────────────────────────────
    out.push_str("\n## Rust mobile targets\n");
    let mut missing: Vec<&str> = Vec::new();
    for (platform, triple) in MOBILE_TRIPLES {
        let installed = toolchain.has_rust_target(triple);
        out.push_str(&format!(
            "- {} {} ({})\n",
            if installed { "✓" } else { "✗" },
            platform.label(),
            triple
        ));
        if !installed {
            missing.push(triple);
        }
    }
    if !missing.is_empty() {
        issues.push(format!("{} Rust target(s) missing", missing.len()));
        next_steps.push(format!(
            "Install missing Rust targets: `rustup target add {}` \
             (or click \"Install missing targets\" in this panel)",
            missing.join(" ")
        ));
    }

    // ── cargo-mobile2 + project init ─────────────────────────────────
    out.push_str("\n## cargo-mobile2\n");
    out.push_str(&format!(
        "- {} cargo-mobile2 installed\n",
        if one_click_installed { "✓" } else { "✗" }
    ));
    if !one_click_installed {
        issues.push("cargo-mobile2 not installed".into());
        next_steps.push(
            "Install cargo-mobile2: `cargo install cargo-mobile2 --locked` \
             (or click \"Install cargo-mobile2\" in this panel)"
                .into(),
        );
    }
    let mobile_toml = project_root.join("mobile.toml");
    let toml_present = mobile_toml.exists();
    out.push_str(&format!(
        "- {} mobile.toml ({})\n",
        if toml_present { "✓" } else { "✗" },
        mobile_toml.display()
    ));
    if !toml_present {
        issues.push("mobile.toml missing in project".into());
        next_steps.push(
            "Initialize the project for mobile: `cargo mobile init` from the project root \
             (or click \"Initialize for Mobile\" in this panel)."
                .into(),
        );
    }

    // ── Summary + next steps ─────────────────────────────────────────
    out.push_str("\n## Summary\n");
    if issues.is_empty() {
        out.push_str(
            "✓ All checks passed — `cargo apple run --release` should work for the iOS \
             Simulator, and `cargo android run` for an Android emulator / device.\n",
        );
    } else {
        out.push_str(&format!("✗ {} issue(s) found:\n", issues.len()));
        for (i, issue) in issues.iter().enumerate() {
            out.push_str(&format!("  {}. {}\n", i + 1, issue));
        }
    }

    if !next_steps.is_empty() {
        out.push_str("\n## Next steps\n");
        for (i, step) in next_steps.iter().enumerate() {
            out.push_str(&format!("{}. {}\n", i + 1, step));
        }
    }

    out
}

// ─── Helpers ──────────────────────────────────────────────────────────────

fn format_one_click_err(e: &OneClickError) -> String {
    format!("[{}] {}", e.stage.label(), e.message)
}

fn short_id(s: &str) -> String {
    if s.len() > 8 {
        format!("{}…", &s[..8])
    } else {
        s.to_string()
    }
}

fn row_status(ui: &mut egui::Ui, label: &str, ok: bool, detail: &str) {
    ui.horizontal(|ui| {
        let bg = if ok {
            egui::Color32::from_rgb(60, 140, 80)
        } else {
            egui::Color32::from_rgb(190, 60, 60)
        };
        let mark = if ok { "OK" } else { "MISS" };
        egui::Frame::NONE
            .fill(bg)
            .corner_radius(egui::CornerRadius::same(4))
            .inner_margin(egui::Margin::symmetric(8, 3))
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new(mark)
                        .size(11.0)
                        .strong()
                        .color(egui::Color32::WHITE),
                );
            });
        ui.label(egui::RichText::new(label).strong());
        ui.label(egui::RichText::new(detail).color(ui_colors::TEXT_MUTED));
    });
}

fn install_hint(ui: &mut egui::Ui, command: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.label("Command:");
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
        if button_style::button_with_icon(ui, "\u{ebcc}", "Copy").clicked() {
            ui.ctx().copy_text(command.to_string());
        }
    });
}

fn setup_hint(ui: &mut egui::Ui, text: &str) {
    egui::Frame::NONE
        .fill(egui::Color32::from_rgba_premultiplied(190, 130, 60, 34))
        .stroke(egui::Stroke::new(
            1.0,
            egui::Color32::from_rgb(150, 105, 55),
        ))
        .corner_radius(egui::CornerRadius::same(4))
        .inner_margin(egui::Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                egui::Frame::NONE
                    .fill(egui::Color32::from_rgb(200, 140, 50))
                    .corner_radius(egui::CornerRadius::same(4))
                    .inner_margin(egui::Margin::symmetric(8, 3))
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new("FIX")
                                .size(11.0)
                                .strong()
                                .color(egui::Color32::WHITE),
                        );
                    });
                ui.label(egui::RichText::new(text).color(ui_colors::TEXT_DEFAULT));
            });
        });
}

fn tool_window_frame() -> egui::Frame {
    egui::Frame::window(&egui::Style::default())
        .fill(ui_colors::SIDEBAR_BG)
        .stroke(egui::Stroke::new(1.0, ui_colors::PANEL_BORDER))
        .corner_radius(egui::CornerRadius::same(4))
        .inner_margin(egui::Margin::same(10))
}

fn panel_header(ui: &mut egui::Ui, title: &str, add_actions: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::NONE
        .fill(ui_colors::TOP_BAR_BG)
        .inner_margin(egui::Margin::symmetric(10, 7))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(title)
                        .small()
                        .strong()
                        .color(ui_colors::TEXT_DEFAULT),
                );
                ui.with_layout(
                    egui::Layout::right_to_left(egui::Align::Center),
                    add_actions,
                );
            });
        });
    ui.add_space(10.0);
}

fn section_frame(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::NONE
        .fill(egui::Color32::from_rgb(30, 31, 34))
        .stroke(egui::Stroke::new(1.0, ui_colors::PANEL_BORDER))
        .corner_radius(egui::CornerRadius::same(4))
        .inner_margin(egui::Margin::same(10))
        .show(ui, add_contents);
}

fn inset_frame() -> egui::Frame {
    egui::Frame::NONE
        .fill(ui_colors::EDITOR_BG)
        .stroke(egui::Stroke::new(1.0, ui_colors::PANEL_BORDER))
        .corner_radius(egui::CornerRadius::same(3))
        .inner_margin(egui::Margin::same(8))
}

fn section_title(ui: &mut egui::Ui, title: &str) {
    ui.label(
        egui::RichText::new(title)
            .strong()
            .color(ui_colors::TEXT_DEFAULT),
    );
    ui.add_space(4.0);
}

fn status_chip(ui: &mut egui::Ui, ok: bool, label: &str) {
    let (text, color, bg) = if ok {
        (
            label.to_string(),
            egui::Color32::from_rgb(120, 200, 120),
            egui::Color32::from_rgba_premultiplied(70, 150, 90, 34),
        )
    } else {
        (
            label
                .replace(" installed", " missing")
                .replace(" present", " missing"),
            egui::Color32::from_rgb(220, 160, 80),
            egui::Color32::from_rgba_premultiplied(190, 130, 60, 34),
        )
    };
    egui::Frame::NONE
        .fill(bg)
        .stroke(egui::Stroke::new(1.0, color.gamma_multiply(0.45)))
        .corner_radius(egui::CornerRadius::same(3))
        .inner_margin(egui::Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).small().color(color));
        });
}

fn error_banner(ui: &mut egui::Ui, text: &str) {
    egui::Frame::NONE
        .fill(egui::Color32::from_rgba_premultiplied(190, 70, 70, 38))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(150, 70, 70)))
        .corner_radius(egui::CornerRadius::same(3))
        .inner_margin(egui::Margin::symmetric(8, 5))
        .show(ui, |ui| {
            ui.colored_label(egui::Color32::from_rgb(230, 110, 110), text);
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_click_installed_starts_unprobed() {
        // Regression guard: the panel relies on `None` to mean "probe on
        // first paint" via `get_or_insert_with`. If this default ever
        // becomes `Some(false)`, the panel will skip probing and the
        // status row gets stuck on `cargo-mobile2 missing`.
        let state = MobileToolchainState::default();
        assert_eq!(state.one_click_installed, None);
    }

    #[test]
    fn refresh_keeps_one_click_cache_untouched() {
        // `refresh()` re-probes Xcode/Android/Rust targets but must not
        // touch the cargo-mobile2 cache — that invalidation lives at the
        // call site (Refresh button + session end) so unit refreshes
        // (e.g. codesign/adb) don't trigger a stray `cargo` subprocess.
        let mut state = MobileToolchainState::default();
        state.one_click_installed = Some(true);
        state.refresh();
        assert_eq!(state.one_click_installed, Some(true));
    }
}
