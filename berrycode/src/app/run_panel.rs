//! Run Bevy project subprocess and display output

use super::BerryCodeApp;
use regex::Regex;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::LazyLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Severity {
    Info,
    Warning,
    Error,
}

/// Minimum log level filter for the tracing log dropdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevelFilter {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
}

impl LogLevelFilter {
    fn label(self) -> &'static str {
        match self {
            Self::Trace => "TRACE",
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
        }
    }
    fn all() -> &'static [LogLevelFilter] {
        &[
            Self::Trace,
            Self::Debug,
            Self::Info,
            Self::Warn,
            Self::Error,
        ]
    }
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "TRACE" => Some(Self::Trace),
            "DEBUG" => Some(Self::Debug),
            "INFO" => Some(Self::Info),
            "WARN" => Some(Self::Warn),
            "ERROR" => Some(Self::Error),
            _ => None,
        }
    }
}

/// A parsed tracing-format log line.
#[derive(Debug, Clone, PartialEq)]
pub struct StructuredLogEntry {
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
    pub raw: String,
}

static TRACING_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(\d{4}-\d{2}-\d{2}T[\d:.]+Z?)\s+(TRACE|DEBUG|INFO|WARN|ERROR)\s+(\S+):\s+(.*)$")
        .unwrap()
});

/// Parse a single line of tracing-style log output.
pub fn parse_tracing_line(line: &str) -> Option<StructuredLogEntry> {
    let caps = TRACING_RE.captures(line)?;
    Some(StructuredLogEntry {
        timestamp: caps[1].to_string(),
        level: caps[2].to_string(),
        target: caps[3].to_string(),
        message: caps[4].to_string(),
        raw: line.to_string(),
    })
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
        let log_level = self.console_log_level_filter;
        let total = self.run_output.len();
        let mut visible_indices: Vec<usize> = Vec::with_capacity(total);
        for (i, line) in self.run_output.iter().enumerate() {
            // Log level filter for tracing lines
            if let Some(entry) = parse_tracing_line(line) {
                if let Some(lvl) = LogLevelFilter::from_str(&entry.level) {
                    if lvl < log_level {
                        continue;
                    }
                }
            }
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

        // VS Code-style compact toolbar
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            let icon_color = egui::Color32::from_rgb(150, 150, 150);
            let icon_font = egui::FontId::new(14.0, egui::FontFamily::Name("codicon".into()));

            // Run/Stop button
            let is_running = self.run_process.is_some();
            if is_running {
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("\u{ead7}") // codicon: debug-stop
                                .font(icon_font.clone())
                                .color(egui::Color32::from_rgb(255, 100, 100)),
                        )
                        .frame(false),
                    )
                    .on_hover_text("Stop")
                    .clicked()
                {
                    self.stop_run();
                }
                ui.label(
                    egui::RichText::new("Running")
                        .size(11.0)
                        .color(egui::Color32::from_rgb(80, 200, 80)),
                );
            } else {
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("\u{eb2c}") // codicon: play
                                .font(icon_font.clone())
                                .color(egui::Color32::from_rgb(80, 200, 80)),
                        )
                        .frame(false),
                    )
                    .on_hover_text("Run")
                    .clicked()
                {
                    self.start_run();
                }
            }

            // Clear button
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new("\u{ea99}") // codicon: clear-all
                            .font(icon_font.clone())
                            .color(icon_color),
                    )
                    .frame(false),
                )
                .on_hover_text("Clear")
                .clicked()
            {
                self.run_output.clear();
            }

            ui.separator();

            // Filter input
            let filter_resp = ui.add_sized(
                [180.0, 22.0],
                egui::TextEdit::singleline(&mut self.console_filter_text)
                    .hint_text("Filter...")
                    .font(egui::FontId::proportional(12.0))
                    .id(egui::Id::new("console_filter_input_dock")),
            );
            if filter_resp.clicked() {
                filter_resp.request_focus();
            }

            ui.separator();

            // Severity toggles (compact)
            ui.label(
                egui::RichText::new("I")
                    .size(11.0)
                    .color(if self.console_show_info {
                        egui::Color32::from_rgb(80, 200, 80)
                    } else {
                        egui::Color32::from_rgb(80, 80, 80)
                    }),
            )
            .on_hover_text("Toggle Info");
            if ui
                .interact(
                    ui.min_rect(),
                    ui.id().with("info_toggle"),
                    egui::Sense::click(),
                )
                .clicked()
            {
                self.console_show_info = !self.console_show_info;
            }

            ui.label(
                egui::RichText::new("W")
                    .size(11.0)
                    .color(if self.console_show_warning {
                        egui::Color32::from_rgb(230, 180, 60)
                    } else {
                        egui::Color32::from_rgb(80, 80, 80)
                    }),
            )
            .on_hover_text("Toggle Warnings");
            if ui
                .interact(
                    ui.min_rect(),
                    ui.id().with("warn_toggle"),
                    egui::Sense::click(),
                )
                .clicked()
            {
                self.console_show_warning = !self.console_show_warning;
            }

            ui.label(
                egui::RichText::new("E")
                    .size(11.0)
                    .color(if self.console_show_error {
                        egui::Color32::from_rgb(255, 110, 110)
                    } else {
                        egui::Color32::from_rgb(80, 80, 80)
                    }),
            )
            .on_hover_text("Toggle Errors");
            if ui
                .interact(
                    ui.min_rect(),
                    ui.id().with("error_toggle"),
                    egui::Sense::click(),
                )
                .clicked()
            {
                self.console_show_error = !self.console_show_error;
            }

            // Right side: line count
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(format!("{} lines", shown))
                        .size(11.0)
                        .color(egui::Color32::from_rgb(100, 100, 100)),
                );
            });
        });

        // Output area.
        let scroll = egui::ScrollArea::vertical().auto_shrink([false; 2]);
        let scroll = if self.console_auto_scroll {
            scroll.stick_to_bottom(true)
        } else {
            scroll
        };
        // Build all log text for selectable display — structured tracing lines get colors
        let mut log_text = String::new();
        // Each line can have multiple colored segments: Vec<(text, color)>
        let mut line_segments: Vec<Vec<(&str, egui::Color32)>> = Vec::new();
        for &i in &visible_indices {
            let line = &self.run_output[i];
            if let Some(entry) = parse_tracing_line(line) {
                let level_color = match entry.level.as_str() {
                    "TRACE" => egui::Color32::from_rgb(128, 128, 128),
                    "DEBUG" => egui::Color32::from_rgb(80, 140, 255),
                    "INFO" => egui::Color32::from_rgb(80, 200, 80),
                    "WARN" => egui::Color32::from_rgb(230, 180, 60),
                    "ERROR" => egui::Color32::from_rgb(255, 110, 110),
                    _ => egui::Color32::from_rgb(204, 204, 204),
                };
                // We'll store segments referencing the original line
                // but since we need owned refs, we push into log_text and track offsets
                let ts_start = log_text.len();
                log_text.push_str(&entry.timestamp);
                let ts_end = log_text.len();
                log_text.push(' ');
                let lvl_start = log_text.len();
                log_text.push_str(&entry.level);
                let lvl_end = log_text.len();
                log_text.push(' ');
                let tgt_start = log_text.len();
                log_text.push_str(&entry.target);
                log_text.push(':');
                let tgt_end = log_text.len();
                log_text.push(' ');
                let msg_start = log_text.len();
                log_text.push_str(&entry.message);
                let msg_end = log_text.len();
                log_text.push('\n');
                // Store segment byte ranges + colors
                line_segments.push(vec![
                    ("ts", egui::Color32::from_rgb(100, 100, 100)), // dim gray
                    ("lvl", level_color),
                    ("tgt", egui::Color32::from_rgb(100, 100, 100)), // dim
                    ("msg", egui::Color32::from_rgb(220, 220, 220)), // white
                ]);
                // Store actual byte ranges in a separate structure below
                let _ = (
                    ts_start, ts_end, lvl_start, lvl_end, tgt_start, tgt_end, msg_start, msg_end,
                );
            } else {
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
                // Strip ANSI codes for display
                let clean: String = line
                    .chars()
                    .fold((String::new(), false), |(mut s, in_esc), c| {
                        if c == '\x1b' {
                            (s, true)
                        } else if in_esc {
                            (s, c != 'm')
                        } else {
                            s.push(c);
                            (s, false)
                        }
                    })
                    .0;
                log_text.push_str(&clean);
                log_text.push('\n');
                line_segments.push(vec![("plain", color)]);
            }
        }

        // Re-build using a LayoutJob approach that respects structured coloring
        scroll.show(ui, |ui| {
            let mut job = egui::text::LayoutJob::default();
            let font = egui::FontId::monospace(12.0);
            // Re-parse lines from log_text with their segment info
            let mut line_iter = log_text.lines();
            for (seg_idx, segments) in line_segments.iter().enumerate() {
                let Some(full_line) = line_iter.next() else {
                    break;
                };
                if segments.len() == 1 {
                    // Plain line
                    let color = segments[0].1;
                    job.append(
                        full_line,
                        0.0,
                        egui::TextFormat {
                            font_id: font.clone(),
                            color,
                            ..Default::default()
                        },
                    );
                } else {
                    // Structured tracing line: timestamp level target: message
                    // Split into parts by space
                    let parts: Vec<&str> = full_line.splitn(4, ' ').collect();
                    if parts.len() == 4 {
                        // timestamp
                        job.append(
                            parts[0],
                            0.0,
                            egui::TextFormat {
                                font_id: font.clone(),
                                color: segments[0].1,
                                ..Default::default()
                            },
                        );
                        job.append(
                            " ",
                            0.0,
                            egui::TextFormat {
                                font_id: font.clone(),
                                color: egui::Color32::TRANSPARENT,
                                ..Default::default()
                            },
                        );
                        // level
                        job.append(
                            parts[1],
                            0.0,
                            egui::TextFormat {
                                font_id: font.clone(),
                                color: segments[1].1,
                                ..Default::default()
                            },
                        );
                        job.append(
                            " ",
                            0.0,
                            egui::TextFormat {
                                font_id: font.clone(),
                                color: egui::Color32::TRANSPARENT,
                                ..Default::default()
                            },
                        );
                        // target
                        job.append(
                            parts[2],
                            0.0,
                            egui::TextFormat {
                                font_id: font.clone(),
                                color: segments[2].1,
                                ..Default::default()
                            },
                        );
                        job.append(
                            " ",
                            0.0,
                            egui::TextFormat {
                                font_id: font.clone(),
                                color: egui::Color32::TRANSPARENT,
                                ..Default::default()
                            },
                        );
                        // message
                        job.append(
                            parts[3],
                            0.0,
                            egui::TextFormat {
                                font_id: font.clone(),
                                color: segments[3].1,
                                ..Default::default()
                            },
                        );
                    } else {
                        // Fallback
                        job.append(
                            full_line,
                            0.0,
                            egui::TextFormat {
                                font_id: font.clone(),
                                color: egui::Color32::from_rgb(204, 204, 204),
                                ..Default::default()
                            },
                        );
                    }
                }
                job.append(
                    "\n",
                    0.0,
                    egui::TextFormat {
                        font_id: font.clone(),
                        color: egui::Color32::TRANSPARENT,
                        ..Default::default()
                    },
                );
            }
            job.wrap.max_width = f32::INFINITY;

            // Selectable label — allows copy
            let response = ui.add(
                egui::Label::new(job)
                    .selectable(true)
                    .sense(egui::Sense::click()),
            );

            // Handle click on file:line patterns
            if response.clicked() {
                if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                    // Find which line was clicked
                    let line_height = 15.0_f32;
                    let local_y = pos.y - response.rect.min.y;
                    let clicked_line = (local_y / line_height).floor() as usize;

                    if let Some(log_line) = log_text.lines().nth(clicked_line) {
                        // Parse "file.rs:LINE:COL" pattern
                        if let Some(file_match) =
                            log_line.split_whitespace().find(|w| w.contains(".rs:"))
                        {
                            let parts: Vec<&str> = file_match.split(':').collect();
                            if parts.len() >= 2 {
                                if let Ok(line_num) = parts[1].parse::<usize>() {
                                    // Find and open the file
                                    let file_name = parts[0];
                                    let full_path = format!("{}/src/{}", self.root_path, file_name);
                                    if std::path::Path::new(&full_path).exists() {
                                        self.open_file_from_path(&full_path);
                                        if let Some(tab) =
                                            self.editor_tabs.get_mut(self.active_tab_idx)
                                        {
                                            tab.pending_cursor_jump =
                                                Some((line_num.saturating_sub(1), 0));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
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
        let log_level = self.console_log_level_filter;
        let total = self.run_output.len();
        let mut visible_indices: Vec<usize> = Vec::with_capacity(total);
        for (i, line) in self.run_output.iter().enumerate() {
            // Log level filter for tracing lines
            if let Some(entry) = parse_tracing_line(line) {
                if let Some(lvl) = LogLevelFilter::from_str(&entry.level) {
                    if lvl < log_level {
                        continue;
                    }
                }
            }
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

                // Header row 2: severity filter chips + log level + auto-scroll + count.
                ui.horizontal(|ui| {
                    ui.label("Show:");
                    ui.checkbox(&mut self.console_show_info, "Info");
                    ui.checkbox(&mut self.console_show_warning, "Warn");
                    ui.checkbox(&mut self.console_show_error, "Error");
                    ui.separator();
                    ui.label("Level:");
                    egui::ComboBox::from_id_salt("log_level_filter_panel")
                        .selected_text(self.console_log_level_filter.label())
                        .width(70.0)
                        .show_ui(ui, |ui| {
                            for &lvl in LogLevelFilter::all() {
                                ui.selectable_value(
                                    &mut self.console_log_level_filter,
                                    lvl,
                                    lvl.label(),
                                );
                            }
                        });
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

                // Output area with structured tracing rendering.
                let scroll = egui::ScrollArea::vertical().auto_shrink([false; 2]);
                let scroll = if self.console_auto_scroll {
                    scroll.stick_to_bottom(true)
                } else {
                    scroll
                };
                scroll.show(ui, |ui| {
                    let font = egui::FontId::monospace(12.0);
                    for &i in &visible_indices {
                        let line = &self.run_output[i];
                        if let Some(entry) = parse_tracing_line(line) {
                            let level_color = match entry.level.as_str() {
                                "TRACE" => egui::Color32::from_rgb(128, 128, 128),
                                "DEBUG" => egui::Color32::from_rgb(80, 140, 255),
                                "INFO" => egui::Color32::from_rgb(80, 200, 80),
                                "WARN" => egui::Color32::from_rgb(230, 180, 60),
                                "ERROR" => egui::Color32::from_rgb(255, 110, 110),
                                _ => egui::Color32::from_rgb(204, 204, 204),
                            };
                            let dim = egui::Color32::from_rgb(100, 100, 100);
                            let white = egui::Color32::from_rgb(220, 220, 220);
                            let mut job = egui::text::LayoutJob::default();
                            job.append(
                                &entry.timestamp,
                                0.0,
                                egui::TextFormat {
                                    font_id: font.clone(),
                                    color: dim,
                                    ..Default::default()
                                },
                            );
                            job.append(
                                " ",
                                0.0,
                                egui::TextFormat {
                                    font_id: font.clone(),
                                    color: egui::Color32::TRANSPARENT,
                                    ..Default::default()
                                },
                            );
                            job.append(
                                &entry.level,
                                0.0,
                                egui::TextFormat {
                                    font_id: font.clone(),
                                    color: level_color,
                                    ..Default::default()
                                },
                            );
                            job.append(
                                " ",
                                0.0,
                                egui::TextFormat {
                                    font_id: font.clone(),
                                    color: egui::Color32::TRANSPARENT,
                                    ..Default::default()
                                },
                            );
                            job.append(
                                &format!("{}:", entry.target),
                                0.0,
                                egui::TextFormat {
                                    font_id: font.clone(),
                                    color: dim,
                                    ..Default::default()
                                },
                            );
                            job.append(
                                " ",
                                0.0,
                                egui::TextFormat {
                                    font_id: font.clone(),
                                    color: egui::Color32::TRANSPARENT,
                                    ..Default::default()
                                },
                            );
                            job.append(
                                &entry.message,
                                0.0,
                                egui::TextFormat {
                                    font_id: font.clone(),
                                    color: white,
                                    ..Default::default()
                                },
                            );
                            job.wrap.max_width = f32::INFINITY;
                            ui.add(egui::Label::new(job).selectable(true));
                        } else {
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
                    }
                });
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tracing_line_full() {
        let line = "2024-03-15T10:30:45.123Z INFO bevy_render::renderer: Initializing wgpu backend";
        let entry = parse_tracing_line(line).expect("should parse");
        assert_eq!(entry.timestamp, "2024-03-15T10:30:45.123Z");
        assert_eq!(entry.level, "INFO");
        assert_eq!(entry.target, "bevy_render::renderer");
        assert_eq!(entry.message, "Initializing wgpu backend");
        assert_eq!(entry.raw, line);
    }

    #[test]
    fn test_parse_tracing_line_no_z_suffix() {
        let line = "2024-03-15T10:30:45.123 DEBUG my_app::systems: tick 42";
        let entry = parse_tracing_line(line).expect("should parse without Z");
        assert_eq!(entry.timestamp, "2024-03-15T10:30:45.123");
        assert_eq!(entry.level, "DEBUG");
        assert_eq!(entry.target, "my_app::systems");
        assert_eq!(entry.message, "tick 42");
    }

    #[test]
    fn test_parse_tracing_line_all_levels() {
        for level in &["TRACE", "DEBUG", "INFO", "WARN", "ERROR"] {
            let line = format!("2024-01-01T00:00:00Z {} target: msg", level);
            let entry = parse_tracing_line(&line).expect("should parse");
            assert_eq!(entry.level, *level);
        }
    }

    #[test]
    fn test_parse_tracing_line_not_tracing() {
        assert!(parse_tracing_line("Compiling my_app v0.1.0").is_none());
        assert!(parse_tracing_line("error[E0308]: mismatched types").is_none());
        assert!(parse_tracing_line("").is_none());
        assert!(parse_tracing_line("just some random text").is_none());
    }

    #[test]
    fn test_parse_tracing_line_message_with_colons() {
        let line = "2024-03-15T10:30:45Z WARN bevy_ecs::world: query error: entity not found: 42";
        let entry = parse_tracing_line(line).expect("should parse");
        assert_eq!(entry.target, "bevy_ecs::world");
        assert_eq!(entry.message, "query error: entity not found: 42");
    }

    #[test]
    fn test_log_level_filter_ordering() {
        assert!(LogLevelFilter::Trace < LogLevelFilter::Debug);
        assert!(LogLevelFilter::Debug < LogLevelFilter::Info);
        assert!(LogLevelFilter::Info < LogLevelFilter::Warn);
        assert!(LogLevelFilter::Warn < LogLevelFilter::Error);
    }

    #[test]
    fn test_log_level_filter_from_str() {
        assert_eq!(
            LogLevelFilter::from_str("TRACE"),
            Some(LogLevelFilter::Trace)
        );
        assert_eq!(
            LogLevelFilter::from_str("ERROR"),
            Some(LogLevelFilter::Error)
        );
        assert_eq!(LogLevelFilter::from_str("invalid"), None);
    }
}
