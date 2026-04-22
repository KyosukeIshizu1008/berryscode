//! Run Bevy project subprocess and display output

use super::BerryCodeApp;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Severity {
    Info,
    Warning,
    Error,
}

fn classify_severity(line: &str) -> Severity {
    let lower = line.to_lowercase();
    // Cargo outputs compilation progress to stderr — these are NOT errors
    if lower.contains("compiling ")
        || lower.contains("downloading ")
        || lower.contains("finished ")
        || lower.contains("building ")
        || lower.contains("checking ")
        || lower.contains("running ")
        || lower.contains("linking ")
        || lower.contains("fresh ")
    {
        Severity::Info
    } else if lower.contains("error") || lower.contains("panic") || lower.contains("failed") {
        Severity::Error
    } else if lower.contains("warning") || lower.contains("warn:") {
        Severity::Warning
    } else {
        Severity::Info
    }
}

impl BerryCodeApp {
    /// Start the Bevy project as a subprocess (cargo run)
    pub(crate) fn start_run(&mut self) {
        // Stop any existing process
        self.stop_run();

        self.run_output.clear();
        self.run_output
            .push("─── Starting cargo run ───".to_string());
        self.run_panel_open = true;
        self.game_view_open = true;

        let project_path = self.root_path.clone();

        // Resolve cargo path — .app bundles may not inherit shell PATH
        let cargo_bin = dirs::home_dir()
            .map(|h| h.join(".cargo/bin/cargo"))
            .filter(|p| p.exists())
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "cargo".to_string());
        let mut cmd = Command::new(&cargo_bin);
        cmd.arg("run");
        if self.run_release_mode {
            cmd.arg("--release");
        }
        // Note: Game View captures the external window via xcap.
        // On macOS, granting Accessibility permission in System Settings
        // allows BerryCode to auto-hide the game window.
        let mut child = match cmd
            .current_dir(&project_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                self.run_output.push(format!("Failed to start: {}", e));
                return;
            }
        };

        let (tx, rx) = std::sync::mpsc::channel::<String>();

        // Spawn thread to read stdout
        if let Some(stdout) = child.stdout.take() {
            let tx_clone = tx.clone();
            std::thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines() {
                    if let Ok(line) = line {
                        if tx_clone.send(line).is_err() {
                            break;
                        }
                    }
                }
            });
        }

        // Spawn thread to read stderr
        if let Some(stderr) = child.stderr.take() {
            std::thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines() {
                    if let Ok(line) = line {
                        if tx.send(format!("[stderr] {}", line)).is_err() {
                            break;
                        }
                    }
                }
            });
        }

        self.run_process = Some(child);
        self.run_output_rx = Some(rx);
    }

    /// Stop the running process
    pub(crate) fn stop_run(&mut self) {
        if let Some(mut child) = self.run_process.take() {
            let _ = child.kill();
            let _ = child.wait();
            self.run_output
                .push("─── Process terminated ───".to_string());
        }
        self.run_output_rx = None;
        self.game_view_window_hidden = false;
    }

    /// Poll output from the running process
    pub(crate) fn poll_run_output(&mut self) {
        if let Some(rx) = &self.run_output_rx {
            // Drain available output
            for _ in 0..100 {
                match rx.try_recv() {
                    Ok(line) => self.run_output.push(line),
                    Err(_) => break,
                }
            }
        }

        // Check if process has exited
        let exited = if let Some(child) = &mut self.run_process {
            match child.try_wait() {
                Ok(Some(status)) => {
                    self.run_output
                        .push(format!("─── Exited with code: {:?} ───", status.code()));
                    true
                }
                _ => false,
            }
        } else {
            false
        };

        if exited {
            self.run_process = None;
            self.run_output_rx = None;
        }
    }

    /// Render console content into a provided `Ui` region (used by the tool panel).
    pub(crate) fn render_console_content(&mut self, ui: &mut egui::Ui) {
        // Pre-compute filtered lines.
        let filter = self.console_filter_text.trim().to_lowercase();
        let total = self.run_output.len();
        let mut visible_indices: Vec<usize> = Vec::with_capacity(total);
        for (i, line) in self.run_output.iter().enumerate() {
            let sev = classify_severity(line);
            let sev_visible = match sev {
                Severity::Info => self.console_show_info,
                Severity::Warning => self.console_show_warning,
                Severity::Error => self.console_show_error,
            };
            if !sev_visible {
                continue;
            }
            if !filter.is_empty() && !line.to_lowercase().contains(&filter) {
                continue;
            }
            visible_indices.push(i);
        }
        let shown = visible_indices.len();
        let hidden = total - shown;

        // Header row 1: title + run/stop controls.
        ui.horizontal(|ui| {
            ui.heading("Console");
            ui.separator();

            let is_running = self.run_process.is_some();
            if is_running {
                if ui.button("Stop").clicked() {
                    self.stop_run();
                }
                ui.colored_label(egui::Color32::from_rgb(80, 200, 80), "Running");
            } else {
                if ui.button("Run").clicked() {
                    self.start_run();
                }
                ui.colored_label(egui::Color32::from_rgb(150, 150, 150), "Stopped");
            }

            ui.separator();
            if ui.button("Clear").clicked() {
                self.run_output.clear();
            }
        });

        // Header row 2: severity filter chips + auto-scroll + count.
        ui.horizontal(|ui| {
            ui.label("Show:");
            ui.checkbox(&mut self.console_show_info, "Info");
            ui.checkbox(&mut self.console_show_warning, "Warn");
            ui.checkbox(&mut self.console_show_error, "Error");
            ui.separator();
            ui.checkbox(&mut self.console_auto_scroll, "Auto-scroll");
            ui.separator();
            ui.label("Filter:");
            ui.add(
                egui::TextEdit::singleline(&mut self.console_filter_text)
                    .hint_text("substring (case-insensitive)")
                    .desired_width(220.0),
            );
            if !self.console_filter_text.is_empty() && ui.small_button("x").clicked() {
                self.console_filter_text.clear();
            }
            ui.separator();
            ui.label(
                egui::RichText::new(format!("{} lines / {} hidden", shown, hidden))
                    .color(egui::Color32::from_gray(160))
                    .size(11.0),
            );
        });

        ui.separator();

        // Output area.
        let scroll = egui::ScrollArea::vertical().auto_shrink([false; 2]);
        let scroll = if self.console_auto_scroll {
            scroll.stick_to_bottom(true)
        } else {
            scroll
        };
        scroll.show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 1.0; // Compact log lines
            for &i in &visible_indices {
                let line = &self.run_output[i];
                let color = match classify_severity(line) {
                    Severity::Error => egui::Color32::from_rgb(255, 110, 110),
                    Severity::Warning => egui::Color32::from_rgb(230, 180, 60),
                    Severity::Info => {
                        if line.starts_with("───") {
                            egui::Color32::from_rgb(100, 180, 255)
                        } else {
                            egui::Color32::from_rgb(204, 204, 204)
                        }
                    }
                };
                ui.horizontal(|ui| {
                    super::ansi::render_ansi_text(ui, line, color, 11.5);
                });
            }
        });
    }

    /// Render the run output panel (bottom panel)
    pub(crate) fn render_run_panel(&mut self, ctx: &egui::Context) {
        if !self.run_panel_open {
            return;
        }

        // Pre-compute filtered lines so we can show "N lines, M filtered".
        let filter = self.console_filter_text.trim().to_lowercase();
        let total = self.run_output.len();
        let mut visible_indices: Vec<usize> = Vec::with_capacity(total);
        for (i, line) in self.run_output.iter().enumerate() {
            let sev = classify_severity(line);
            let sev_visible = match sev {
                Severity::Info => self.console_show_info,
                Severity::Warning => self.console_show_warning,
                Severity::Error => self.console_show_error,
            };
            if !sev_visible {
                continue;
            }
            if !filter.is_empty() && !line.to_lowercase().contains(&filter) {
                continue;
            }
            visible_indices.push(i);
        }
        let shown = visible_indices.len();
        let hidden = total - shown;

        egui::TopBottomPanel::bottom("run_output_panel")
            .resizable(true)
            .default_height(280.0)
            .show(ctx, |ui| {
                // Header row 1: title + run/stop controls.
                ui.horizontal(|ui| {
                    ui.heading("Console");
                    ui.separator();

                    let is_running = self.run_process.is_some();
                    if is_running {
                        if ui.button("Stop").clicked() {
                            self.stop_run();
                        }
                        ui.colored_label(egui::Color32::from_rgb(80, 200, 80), "Running");
                    } else {
                        if ui.button("Run").clicked() {
                            self.start_run();
                        }
                        ui.colored_label(egui::Color32::from_rgb(150, 150, 150), "Stopped");
                    }

                    ui.separator();
                    if ui.button("Clear").clicked() {
                        self.run_output.clear();
                    }
                    if ui.button("Copy All").clicked() {
                        let text: String = visible_indices
                            .iter()
                            .map(|&i| self.run_output[i].as_str())
                            .collect::<Vec<_>>()
                            .join("\n");
                        ctx.copy_text(text);
                    }
                    if ui.button("Save Log...").clicked() {
                        let now = chrono::Local::now();
                        let stamp = now.format("%Y%m%d-%H%M%S").to_string();
                        let path = format!("{}/console-{}.log", self.root_path, stamp);
                        let body: String = self.run_output.join("\n");
                        match std::fs::write(&path, body) {
                            Ok(_) => {
                                self.status_message = format!("Saved log: {}", path);
                                self.status_message_timestamp = Some(std::time::Instant::now());
                            }
                            Err(e) => {
                                self.status_message = format!("Save failed: {}", e);
                                self.status_message_timestamp = Some(std::time::Instant::now());
                            }
                        }
                    }
                    if ui.button("Close").clicked() {
                        self.run_panel_open = false;
                    }
                });

                // Header row 2: severity filter chips + auto-scroll + count.
                ui.horizontal(|ui| {
                    ui.label("Show:");
                    ui.checkbox(&mut self.console_show_info, "Info");
                    ui.checkbox(&mut self.console_show_warning, "Warn");
                    ui.checkbox(&mut self.console_show_error, "Error");
                    ui.separator();
                    ui.checkbox(&mut self.console_auto_scroll, "Auto-scroll");
                    ui.separator();
                    ui.label("Filter:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.console_filter_text)
                            .hint_text("substring (case-insensitive)")
                            .desired_width(220.0),
                    );
                    if !self.console_filter_text.is_empty() && ui.small_button("x").clicked() {
                        self.console_filter_text.clear();
                    }
                    ui.separator();
                    ui.label(
                        egui::RichText::new(format!("{} lines · {} hidden", shown, hidden))
                            .color(egui::Color32::from_gray(160))
                            .size(11.0),
                    );
                });

                ui.separator();

                // Output area.
                let scroll = egui::ScrollArea::vertical().auto_shrink([false; 2]);
                let scroll = if self.console_auto_scroll {
                    scroll.stick_to_bottom(true)
                } else {
                    scroll
                };
                scroll.show(ui, |ui| {
                    for &i in &visible_indices {
                        let line = &self.run_output[i];
                        let color = match classify_severity(line) {
                            Severity::Error => egui::Color32::from_rgb(255, 110, 110),
                            Severity::Warning => egui::Color32::from_rgb(230, 180, 60),
                            Severity::Info => {
                                if line.starts_with("───") {
                                    egui::Color32::from_rgb(100, 180, 255)
                                } else {
                                    egui::Color32::from_rgb(204, 204, 204)
                                }
                            }
                        };
                        ui.horizontal(|ui| {
                            super::ansi::render_ansi_text(ui, line, color, 12.0);
                        });
                    }
                });
            });
    }
}
