//! Docker panel — Docker Desktop-style layout. Sidebar holds the
//! tab selector (Containers / Images / Volumes), the central panel
//! shows the list and per-item details. All daemon access goes
//! through the `docker` CLI as a subprocess; if Docker Desktop isn't
//! installed we show an install hint instead.

use std::process::Command;
use std::time::{Duration, Instant};

use serde::Deserialize;

use super::button_style::{self, icon_button};
use super::ui_colors;
use super::BerryCodeApp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockerTab {
    Containers,
    Images,
    Volumes,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Container {
    #[serde(rename = "ID")]
    pub id: String,
    #[serde(rename = "Names")]
    pub names: String,
    #[serde(rename = "Image")]
    pub image: String,
    #[serde(rename = "Status")]
    pub status: String,
    #[serde(rename = "State")]
    pub state: String,
    #[serde(rename = "Ports")]
    pub ports: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Image {
    #[serde(rename = "ID")]
    pub id: String,
    #[serde(rename = "Repository")]
    pub repository: String,
    #[serde(rename = "Tag")]
    pub tag: String,
    #[serde(rename = "Size")]
    pub size: String,
    #[serde(rename = "CreatedSince")]
    pub created: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Volume {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Driver")]
    pub driver: String,
    #[serde(rename = "Mountpoint")]
    pub mountpoint: String,
}

pub struct DockerState {
    pub active_tab: DockerTab,
    pub cli_available: Option<bool>,
    pub daemon_reachable: Option<bool>,
    pub containers: Vec<Container>,
    pub images: Vec<Image>,
    pub volumes: Vec<Volume>,
    pub selected_container: Option<String>,
    pub selected_logs: String,
    pub error_message: Option<String>,
    pub last_refresh: Option<Instant>,
}

impl Default for DockerState {
    fn default() -> Self {
        Self {
            active_tab: DockerTab::Containers,
            cli_available: None,
            daemon_reachable: None,
            containers: Vec::new(),
            images: Vec::new(),
            volumes: Vec::new(),
            selected_container: None,
            selected_logs: String::new(),
            error_message: None,
            last_refresh: None,
        }
    }
}

impl DockerState {
    /// Check whether `docker` is on PATH. Cheap; cached.
    pub fn ensure_cli_check(&mut self) {
        if self.cli_available.is_some() {
            return;
        }
        self.cli_available = Some(which_docker().is_some());
    }

    pub fn refresh_all(&mut self) {
        self.ensure_cli_check();
        if self.cli_available != Some(true) {
            return;
        }
        self.refresh_containers();
        self.refresh_images();
        self.refresh_volumes();
        self.last_refresh = Some(Instant::now());
    }

    /// Auto-refresh if stale (every 5s). Called from render.
    pub fn maybe_refresh(&mut self) {
        let stale = match self.last_refresh {
            Some(t) => t.elapsed() > Duration::from_secs(5),
            None => true,
        };
        if stale {
            self.refresh_all();
        }
    }

    pub fn refresh_containers(&mut self) {
        match run_json_lines::<Container>(&["ps", "-a", "--format", "{{json .}}"]) {
            Ok(v) => {
                self.containers = v;
                self.daemon_reachable = Some(true);
                self.error_message = None;
            }
            Err(e) => {
                self.daemon_reachable = Some(false);
                self.error_message = Some(e);
            }
        }
    }

    pub fn refresh_images(&mut self) {
        if let Ok(v) = run_json_lines::<Image>(&["images", "--format", "{{json .}}"]) {
            self.images = v;
        }
    }

    pub fn refresh_volumes(&mut self) {
        if let Ok(v) = run_json_lines::<Volume>(&["volume", "ls", "--format", "{{json .}}"]) {
            self.volumes = v;
        }
    }

    pub fn start(&mut self, id: &str) {
        let _ = run_status(&["start", id]);
        self.refresh_containers();
    }

    pub fn stop(&mut self, id: &str) {
        let _ = run_status(&["stop", id]);
        self.refresh_containers();
    }

    pub fn restart(&mut self, id: &str) {
        let _ = run_status(&["restart", id]);
        self.refresh_containers();
    }

    pub fn remove(&mut self, id: &str) {
        let _ = run_status(&["rm", "-f", id]);
        self.refresh_containers();
    }

    pub fn fetch_logs(&mut self, id: &str) {
        match run_capture(&["logs", "--tail", "200", id]) {
            Ok(s) => self.selected_logs = s,
            Err(e) => self.selected_logs = format!("(failed: {e})"),
        }
    }
}

fn which_docker() -> Option<std::path::PathBuf> {
    // Try the user's PATH plus the common Docker Desktop install location
    // on macOS, since GUI-launched processes inherit a slim PATH.
    let mut candidates = Vec::new();
    if let Ok(p) = std::env::var("PATH") {
        for dir in std::env::split_paths(&p) {
            candidates.push(dir.join("docker"));
        }
    }
    #[cfg(target_os = "macos")]
    {
        candidates.push(std::path::PathBuf::from(
            "/Applications/Docker.app/Contents/Resources/bin/docker",
        ));
        candidates.push(std::path::PathBuf::from("/usr/local/bin/docker"));
        candidates.push(std::path::PathBuf::from("/opt/homebrew/bin/docker"));
    }
    candidates.into_iter().find(|p| p.is_file())
}

fn docker_cmd() -> Option<Command> {
    which_docker().map(Command::new)
}

fn run_capture(args: &[&str]) -> Result<String, String> {
    let mut cmd = docker_cmd().ok_or_else(|| "docker not found".to_string())?;
    cmd.args(args);
    let out = cmd.output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn run_status(args: &[&str]) -> Result<(), String> {
    run_capture(args).map(|_| ())
}

fn run_json_lines<T: serde::de::DeserializeOwned>(args: &[&str]) -> Result<Vec<T>, String> {
    let stdout = run_capture(args)?;
    let mut out = Vec::new();
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<T>(line) {
            Ok(v) => out.push(v),
            Err(_) => {} // skip malformed lines silently
        }
    }
    Ok(out)
}

impl BerryCodeApp {
    pub(crate) fn render_docker_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.label(
            egui::RichText::new("DOCKER")
                .small()
                .strong()
                .color(ui_colors::TEXT_DEFAULT),
        );
        ui.add_space(10.0);

        self.docker.ensure_cli_check();
        if self.docker.cli_available != Some(true) {
            docker_error_card(ui, "Docker CLI not found.");
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("Install Docker Desktop:")
                    .small()
                    .color(ui_colors::TEXT_MUTED),
            );
            ui.hyperlink_to(
                "docker.com/products/docker-desktop",
                "https://www.docker.com/products/docker-desktop/",
            );
            if button_style::button_with_icon(ui, button_style::glyph::REFRESH, "Re-check")
                .clicked()
            {
                self.docker.cli_available = None;
            }
            return;
        }

        ui.vertical(|ui| {
            tab_link(
                ui,
                &mut self.docker.active_tab,
                DockerTab::Containers,
                "Containers",
                self.docker.containers.len(),
            );
            tab_link(
                ui,
                &mut self.docker.active_tab,
                DockerTab::Images,
                "Images",
                self.docker.images.len(),
            );
            tab_link(
                ui,
                &mut self.docker.active_tab,
                DockerTab::Volumes,
                "Volumes",
                self.docker.volumes.len(),
            );
        });

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);

        if button_style::button_with_icon(ui, button_style::glyph::REFRESH, "Refresh").clicked() {
            self.docker.refresh_all();
        }
        if let Some(t) = self.docker.last_refresh {
            ui.label(
                egui::RichText::new(format!("updated {}s ago", t.elapsed().as_secs()))
                    .small()
                    .color(ui_colors::TEXT_MUTED),
            );
        }

        if let Some(err) = &self.docker.error_message {
            ui.add_space(6.0);
            docker_error_card(ui, err);
        }
    }

    pub(crate) fn render_docker_central(&mut self, ui: &mut egui::Ui) {
        self.docker.ensure_cli_check();
        if self.docker.cli_available != Some(true) {
            empty_state(ui, "Docker Desktop is not installed", "BerryCode talks to the local Docker daemon through the docker CLI. Install Docker Desktop and restart BerryCode to enable this panel.");
            return;
        }

        // Lazy initial load + 5s background refresh.
        self.docker.maybe_refresh();

        match self.docker.active_tab {
            DockerTab::Containers => self.render_docker_containers(ui),
            DockerTab::Images => self.render_docker_images(ui),
            DockerTab::Volumes => self.render_docker_volumes(ui),
        }
    }

    fn render_docker_containers(&mut self, ui: &mut egui::Ui) {
        if self.docker.containers.is_empty() {
            empty_state(
                ui,
                "No containers",
                "Running containers will appear here after Docker responds.",
            );
            return;
        }

        // Snapshot to avoid borrow conflicts inside the loop.
        let containers: Vec<Container> = self.docker.containers.clone();
        let selected = self.docker.selected_container.clone();

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("docker_containers_grid")
                    .num_columns(6)
                    .striped(true)
                    .min_col_width(50.0)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("").strong());
                        ui.label(
                            egui::RichText::new("Name")
                                .strong()
                                .color(ui_colors::TEXT_DEFAULT),
                        );
                        ui.label(
                            egui::RichText::new("Image")
                                .strong()
                                .color(ui_colors::TEXT_DEFAULT),
                        );
                        ui.label(
                            egui::RichText::new("Status")
                                .strong()
                                .color(ui_colors::TEXT_DEFAULT),
                        );
                        ui.label(
                            egui::RichText::new("Ports")
                                .strong()
                                .color(ui_colors::TEXT_DEFAULT),
                        );
                        ui.label(
                            egui::RichText::new("Actions")
                                .strong()
                                .color(ui_colors::TEXT_DEFAULT),
                        );
                        ui.end_row();

                        for c in &containers {
                            let running = c.state.eq_ignore_ascii_case("running")
                                || c.status.starts_with("Up");
                            ui.colored_label(
                                if running {
                                    egui::Color32::from_rgb(120, 200, 120)
                                } else {
                                    ui_colors::TEXT_MUTED
                                },
                                if running { "running" } else { "stopped" },
                            );

                            let is_selected = selected.as_deref() == Some(c.id.as_str());
                            if ui.selectable_label(is_selected, &c.names).clicked() {
                                self.docker.selected_container = Some(c.id.clone());
                                self.docker.fetch_logs(&c.id);
                            }
                            ui.label(&c.image);
                            ui.label(&c.status);
                            ui.label(if c.ports.is_empty() {
                                "—"
                            } else {
                                c.ports.as_str()
                            });
                            ui.horizontal(|ui| {
                                if running {
                                    if icon_button(ui, "\u{ead7}", "Stop").clicked() {
                                        self.docker.stop(&c.id);
                                    }
                                    if icon_button(ui, "\u{ead2}", "Restart").clicked() {
                                        self.docker.restart(&c.id);
                                    }
                                } else if icon_button(ui, "\u{eb2c}", "Start").clicked() {
                                    self.docker.start(&c.id);
                                }
                                if icon_button(ui, "\u{ea81}", "Remove").clicked() {
                                    self.docker.remove(&c.id);
                                }
                            });
                            ui.end_row();
                        }
                    });

                // Logs panel for the selected container.
                if let Some(id) = self.docker.selected_container.clone() {
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("Logs — {}", short_id(&id))).strong());
                        if button_style::button_with_icon(
                            ui,
                            button_style::glyph::REFRESH,
                            "Reload",
                        )
                        .clicked()
                        {
                            self.docker.fetch_logs(&id);
                        }
                    });
                    ui.add_space(4.0);
                    docker_inset_frame().show(ui, |ui| {
                        egui::ScrollArea::both()
                            .max_height(220.0)
                            .auto_shrink([false, false])
                            .stick_to_bottom(true)
                            .show(ui, |ui| {
                                ui.add(
                                    egui::TextEdit::multiline(
                                        &mut self.docker.selected_logs.as_str(),
                                    )
                                    .code_editor()
                                    .desired_width(f32::INFINITY)
                                    .desired_rows(12),
                                );
                            });
                    });
                }
            });
    }

    fn render_docker_images(&mut self, ui: &mut egui::Ui) {
        if self.docker.images.is_empty() {
            empty_state(
                ui,
                "No images",
                "Pulled and locally built images will appear here.",
            );
            return;
        }
        let images: Vec<Image> = self.docker.images.clone();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("docker_images_grid")
                    .num_columns(4)
                    .striped(true)
                    .min_col_width(60.0)
                    .show(ui, |ui| {
                        for c in ["Repository", "Tag", "Size", "Created"] {
                            ui.label(
                                egui::RichText::new(c)
                                    .strong()
                                    .color(ui_colors::TEXT_DEFAULT),
                            );
                        }
                        ui.end_row();
                        for img in &images {
                            ui.label(&img.repository);
                            ui.label(&img.tag);
                            ui.label(&img.size);
                            ui.label(&img.created);
                            ui.end_row();
                        }
                    });
            });
    }

    fn render_docker_volumes(&mut self, ui: &mut egui::Ui) {
        if self.docker.volumes.is_empty() {
            empty_state(ui, "No volumes", "Named volumes will appear here.");
            return;
        }
        let volumes: Vec<Volume> = self.docker.volumes.clone();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("docker_volumes_grid")
                    .num_columns(3)
                    .striped(true)
                    .min_col_width(60.0)
                    .show(ui, |ui| {
                        for c in ["Name", "Driver", "Mountpoint"] {
                            ui.label(
                                egui::RichText::new(c)
                                    .strong()
                                    .color(ui_colors::TEXT_DEFAULT),
                            );
                        }
                        ui.end_row();
                        for v in &volumes {
                            ui.label(&v.name);
                            ui.label(&v.driver);
                            ui.label(&v.mountpoint);
                            ui.end_row();
                        }
                    });
            });
    }
}

fn tab_link(ui: &mut egui::Ui, current: &mut DockerTab, tab: DockerTab, label: &str, count: usize) {
    let text = if count > 0 {
        format!("{label}  ({count})")
    } else {
        label.to_string()
    };
    let selected = *current == tab;
    let response = egui::Frame::NONE
        .fill(if selected {
            ui_colors::ACTIVE_BG
        } else {
            egui::Color32::TRANSPARENT
        })
        .stroke(egui::Stroke::new(
            1.0,
            if selected {
                ui_colors::FOCUS_BORDER
            } else {
                egui::Color32::TRANSPARENT
            },
        ))
        .corner_radius(egui::CornerRadius::same(4))
        .inner_margin(egui::Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(text).strong().color(if selected {
                    ui_colors::TEXT_DEFAULT
                } else {
                    ui_colors::TEXT_MUTED
                }));
            });
        })
        .response
        .interact(egui::Sense::click());
    if response.clicked() {
        *current = tab;
    }
}

fn short_id(id: &str) -> &str {
    id.get(..12).unwrap_or(id)
}

fn docker_inset_frame() -> egui::Frame {
    egui::Frame::NONE
        .fill(ui_colors::EDITOR_BG)
        .stroke(egui::Stroke::new(1.0, ui_colors::PANEL_BORDER))
        .corner_radius(egui::CornerRadius::same(3))
        .inner_margin(egui::Margin::same(8))
}

fn docker_error_card(ui: &mut egui::Ui, text: &str) {
    egui::Frame::NONE
        .fill(egui::Color32::from_rgba_premultiplied(190, 70, 70, 38))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(150, 70, 70)))
        .corner_radius(egui::CornerRadius::same(3))
        .inner_margin(egui::Margin::symmetric(8, 5))
        .show(ui, |ui| {
            ui.colored_label(egui::Color32::from_rgb(230, 110, 110), text);
        });
}

fn empty_state(ui: &mut egui::Ui, title: &str, body: &str) {
    ui.add_space(16.0);
    egui::Frame::NONE
        .fill(egui::Color32::from_rgb(30, 31, 34))
        .stroke(egui::Stroke::new(1.0, ui_colors::PANEL_BORDER))
        .corner_radius(egui::CornerRadius::same(4))
        .inner_margin(egui::Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(title)
                    .strong()
                    .color(ui_colors::TEXT_DEFAULT),
            );
            ui.add_space(4.0);
            ui.label(egui::RichText::new(body).color(ui_colors::TEXT_MUTED));
            if title.contains("Docker Desktop") {
                ui.add_space(8.0);
                ui.hyperlink_to(
                    "Download Docker Desktop",
                    "https://www.docker.com/products/docker-desktop/",
                );
            }
        });
}
