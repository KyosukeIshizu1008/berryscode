//! Terminal rendering and command execution
//! iTerm2 + Oh My Zsh inspired design

use super::BerryCodeApp;
use super::types::{TerminalLine, TerminalStyle};

impl BerryCodeApp {
    /// Render Terminal panel in sidebar (compact Oh My Zsh style)
    pub(crate) fn render_terminal(&mut self, ui: &mut egui::Ui) {
        // Compact header
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Terminal")
                    .color(egui::Color32::from_rgb(200, 200, 200))
                    .size(13.0)
                    .strong(),
            );
        });

        ui.add_space(4.0);

        ui.vertical(|ui| {
            // Output area with scrolling
            let scroll_area = egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .stick_to_bottom(true)
                .max_height(ui.available_height() - 60.0);

            scroll_area.show(ui, |ui| {
                ui.style_mut().spacing.item_spacing.y = 0.0;

                if self.terminal_output.is_empty() {
                    ui.add_space(4.0);
                    self.render_prompt_compact(ui);
                } else {
                    for line in &self.terminal_output {
                        match line.style {
                            TerminalStyle::Separator => {
                                ui.add_space(2.0);
                                ui.horizontal(|ui| {
                                    ui.add_space(8.0);
                                    ui.label(
                                        egui::RichText::new(&line.text)
                                            .color(egui::Color32::from_rgb(60, 63, 65))
                                            .font(egui::FontId::monospace(10.0)),
                                    );
                                });
                                ui.add_space(2.0);
                            }
                            TerminalStyle::Command => {
                                ui.add_space(1.0);
                                ui.horizontal(|ui| {
                                    ui.add_space(8.0);
                                    // Strip the "$ " prefix and show with green bold styling
                                    let cmd_text = line.text.strip_prefix("$ ").unwrap_or(&line.text);
                                    ui.label(
                                        egui::RichText::new(format!("  {}", cmd_text))
                                            .color(egui::Color32::from_rgb(130, 220, 130))
                                            .font(egui::FontId::monospace(12.0))
                                            .strong(),
                                    );
                                });
                            }
                            TerminalStyle::Output => {
                                ui.horizontal(|ui| {
                                    ui.add_space(8.0);
                                    super::ansi::render_ansi_text(
                                        ui,
                                        &line.text,
                                        egui::Color32::from_rgb(190, 190, 190),
                                        12.0,
                                    );
                                });
                            }
                            TerminalStyle::Error => {
                                ui.horizontal(|ui| {
                                    ui.add_space(8.0);
                                    super::ansi::render_ansi_text(
                                        ui,
                                        &line.text,
                                        egui::Color32::from_rgb(255, 100, 100),
                                        12.0,
                                    );
                                });
                            }
                        }
                    }

                    ui.add_space(6.0);
                }
            });

            ui.add_space(2.0);

            // Compact prompt + input
            self.render_prompt_compact(ui);

            ui.horizontal(|ui| {
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("  ")
                        .color(egui::Color32::from_rgb(130, 220, 130))
                        .font(egui::FontId::monospace(13.0))
                        .strong(),
                );

                let text_edit = egui::TextEdit::singleline(&mut self.terminal_input)
                    .font(egui::FontId::monospace(12.0))
                    .text_color(egui::Color32::from_rgb(220, 220, 220))
                    .desired_width(ui.available_width())
                    .frame(false);

                let response = ui.add(text_edit);

                if !response.has_focus() && self.terminal_output.is_empty() {
                    response.request_focus();
                }

                let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                if enter_pressed && (response.has_focus() || response.lost_focus()) {
                    self.execute_terminal_command();
                    response.request_focus();
                }

                if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) && response.has_focus() {
                    self.navigate_history_up();
                    response.request_focus();
                }
                if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) && response.has_focus() {
                    self.navigate_history_down();
                    response.request_focus();
                }
            });
        });
    }

    /// Render compact prompt for sidebar (single line)
    fn render_prompt_compact(&self, ui: &mut egui::Ui) {
        let home = std::env::var("HOME").unwrap_or_default();
        let path = self.terminal_working_dir.replace(&home, "~");
        let git_info = self.get_git_prompt_info();

        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.spacing_mut().item_spacing.x = 0.0;

            // Directory (blue)
            ui.label(
                egui::RichText::new(format!(" {} ", path))
                    .color(egui::Color32::from_rgb(100, 160, 255))
                    .font(egui::FontId::monospace(11.0))
                    .strong(),
            );

            // Git segment
            if let Some((branch, is_clean)) = &git_info {
                let git_color = if *is_clean {
                    egui::Color32::from_rgb(130, 220, 130)
                } else {
                    egui::Color32::from_rgb(230, 180, 80)
                };
                let status_icon = if *is_clean { "\u{2713}" } else { "\u{00b1}" };

                ui.label(
                    egui::RichText::new(format!("  {} {} ", branch, status_icon))
                        .color(git_color)
                        .font(egui::FontId::monospace(11.0)),
                );
            }
        });
    }

    /// Render full-screen iTerm2 + Oh My Zsh terminal
    pub(crate) fn render_terminal_fullscreen(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(25, 26, 28))
                    .inner_margin(0.0),
            )
            .show(ctx, |ui| {
                // Terminal output area
                let scroll_area = egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .stick_to_bottom(true);

                scroll_area.show(ui, |ui| {
                    ui.style_mut().spacing.item_spacing.y = 0.0;

                    // Welcome message on empty terminal
                    if self.terminal_output.is_empty() && self.terminal_input.is_empty() {
                        ui.add_space(12.0);
                        ui.horizontal(|ui| {
                            ui.add_space(12.0);
                            ui.label(
                                egui::RichText::new("Welcome to BerryCode Terminal")
                                    .color(egui::Color32::from_rgb(100, 160, 255))
                                    .font(egui::FontId::monospace(13.0))
                                    .strong(),
                            );
                        });
                        ui.add_space(2.0);
                        ui.horizontal(|ui| {
                            ui.add_space(12.0);
                            ui.label(
                                egui::RichText::new("Type a command to get started.")
                                    .color(egui::Color32::from_rgb(100, 100, 100))
                                    .font(egui::FontId::monospace(12.0)),
                            );
                        });
                        ui.add_space(12.0);
                    } else {
                        ui.add_space(8.0);

                        // Render command blocks
                        for line in &self.terminal_output {
                            match line.style {
                                TerminalStyle::Separator => {
                                    ui.add_space(4.0);
                                    ui.horizontal(|ui| {
                                        ui.add_space(12.0);
                                        let available_w = ui.available_width() - 24.0;
                                        let dash_count =
                                            (available_w / 7.0).max(10.0) as usize;
                                        ui.label(
                                            egui::RichText::new(
                                                "\u{2500}".repeat(dash_count),
                                            )
                                            .color(egui::Color32::from_rgb(50, 53, 55))
                                            .font(egui::FontId::monospace(11.0)),
                                        );
                                    });
                                    ui.add_space(4.0);
                                }
                                TerminalStyle::Command => {
                                    // Show prompt before command
                                    let cmd_text =
                                        line.text.strip_prefix("$ ").unwrap_or(&line.text);
                                    ui.add_space(1.0);
                                    ui.horizontal(|ui| {
                                        ui.add_space(12.0);
                                        ui.spacing_mut().item_spacing.x = 0.0;
                                        ui.label(
                                            egui::RichText::new("  ")
                                                .color(egui::Color32::from_rgb(130, 220, 130))
                                                .font(egui::FontId::monospace(13.0))
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new(cmd_text)
                                                .color(egui::Color32::from_rgb(130, 220, 130))
                                                .font(egui::FontId::monospace(13.0))
                                                .strong(),
                                        );
                                    });
                                }
                                TerminalStyle::Output => {
                                    ui.add_space(1.0);
                                    ui.horizontal(|ui| {
                                        ui.add_space(12.0);
                                        super::ansi::render_ansi_text(
                                            ui,
                                            &line.text,
                                            egui::Color32::from_rgb(204, 204, 204),
                                            13.0,
                                        );
                                    });
                                }
                                TerminalStyle::Error => {
                                    ui.add_space(1.0);
                                    ui.horizontal(|ui| {
                                        ui.add_space(12.0);
                                        super::ansi::render_ansi_text(
                                            ui,
                                            &line.text,
                                            egui::Color32::from_rgb(255, 100, 100),
                                            13.0,
                                        );
                                    });
                                }
                            }
                        }

                        ui.add_space(8.0);
                    }

                    // Current input prompt
                    self.render_prompt(ui);

                    ui.horizontal(|ui| {
                        ui.add_space(12.0);
                        ui.spacing_mut().item_spacing.x = 0.0;

                        ui.label(
                            egui::RichText::new("\u{276f} ")
                                .color(egui::Color32::from_rgb(130, 220, 130))
                                .font(egui::FontId::monospace(14.0))
                                .strong(),
                        );

                        let text_edit = egui::TextEdit::singleline(&mut self.terminal_input)
                            .font(egui::FontId::monospace(13.0))
                            .text_color(egui::Color32::from_rgb(220, 220, 220))
                            .desired_width(f32::INFINITY)
                            .frame(false);

                        let response = ui.add(text_edit);

                        if !response.has_focus() {
                            response.request_focus();
                        }

                        let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                        if enter_pressed {
                            self.execute_terminal_command();
                            response.request_focus();
                        }

                        if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) && response.has_focus()
                        {
                            self.navigate_history_up();
                        }
                        if ui.input(|i| i.key_pressed(egui::Key::ArrowDown))
                            && response.has_focus()
                        {
                            self.navigate_history_down();
                        }
                    });

                    ui.add_space(16.0);
                });
            });
    }

    /// Render Oh My Zsh-style prompt (fullscreen version)
    fn render_prompt(&self, ui: &mut egui::Ui) {
        let home = std::env::var("HOME").unwrap_or_default();
        let path = self.terminal_working_dir.replace(&home, "~");
        let git_info = self.get_git_prompt_info();

        ui.horizontal(|ui| {
            ui.add_space(12.0);
            ui.spacing_mut().item_spacing.x = 0.0;

            // Directory segment (blue)
            ui.label(
                egui::RichText::new(" ")
                    .color(egui::Color32::from_rgb(100, 160, 255))
                    .font(egui::FontId::monospace(13.0)),
            );
            ui.label(
                egui::RichText::new(&path)
                    .color(egui::Color32::from_rgb(100, 160, 255))
                    .font(egui::FontId::monospace(13.0))
                    .strong(),
            );
            ui.label(
                egui::RichText::new(" ")
                    .color(egui::Color32::from_rgb(100, 160, 255))
                    .font(egui::FontId::monospace(13.0)),
            );

            // Git segment (if available)
            if let Some((branch, is_clean)) = &git_info {
                let git_color = if *is_clean {
                    egui::Color32::from_rgb(130, 220, 130) // green = clean
                } else {
                    egui::Color32::from_rgb(230, 180, 80) // yellow = dirty
                };
                let status_icon = if *is_clean { "\u{2713}" } else { "\u{00b1}" };

                ui.label(
                    egui::RichText::new(format!("  {} {} ", branch, status_icon))
                        .color(git_color)
                        .font(egui::FontId::monospace(13.0)),
                );
            }

            ui.add_space(4.0);
        });
    }

    /// Get git branch and clean/dirty status for prompt
    fn get_git_prompt_info(&self) -> Option<(String, bool)> {
        if self.git_current_branch == "(unknown)" {
            return None;
        }
        let branch = self.git_current_branch.clone();
        let is_clean = self.git_status.is_empty();
        Some((branch, is_clean))
    }

    /// Execute terminal command
    pub(crate) fn execute_terminal_command(&mut self) {
        let cmd = self.terminal_input.trim().to_string();

        if cmd.is_empty() {
            return;
        }

        // Add to history
        if !self.terminal_history.contains(&cmd) || self.terminal_history.last() != Some(&cmd) {
            self.terminal_history.push(cmd.clone());
        }
        self.terminal_history_index = None;

        // Add separator before command block (if there is prior output)
        if !self.terminal_output.is_empty() {
            self.terminal_output.push(TerminalLine {
                text: "\u{2500}".repeat(60),
                style: TerminalStyle::Separator,
            });
        }

        // Display command
        self.terminal_output.push(TerminalLine {
            text: format!("$ {}", cmd),
            style: TerminalStyle::Command,
        });

        // Handle built-in commands
        if cmd.starts_with("cd ") {
            let path = cmd[3..].trim();
            self.change_directory(path);
        } else if cmd == "clear" {
            self.terminal_output.clear();
        } else {
            // Execute external command
            self.execute_external_command(&cmd);
        }

        // Clear input
        self.terminal_input.clear();
    }

    /// Change terminal working directory
    pub(crate) fn change_directory(&mut self, path: &str) {
        use std::path::Path;

        let new_path = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("{}/{}", self.terminal_working_dir, path)
        };

        let normalized_path = Path::new(&new_path).canonicalize();

        match normalized_path {
            Ok(p) => {
                self.terminal_working_dir = p.to_string_lossy().to_string();
                tracing::info!("Changed directory to: {}", self.terminal_working_dir);
            }
            Err(e) => {
                self.terminal_output.push(TerminalLine {
                    text: format!("cd: {}: {}", path, e),
                    style: TerminalStyle::Error,
                });
            }
        }
    }

    /// Execute external command using std::process::Command (reliable, no PTY hang)
    pub(crate) fn execute_external_command(&mut self, cmd: &str) {
        use std::process::Command;

        let output = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .current_dir(&self.terminal_working_dir)
            .env("TERM", "xterm-256color")
            .env("COLORTERM", "truecolor")
            .env("CLICOLOR_FORCE", "1")
            .env("GIT_CONFIG_COUNT", "1")
            .env("GIT_CONFIG_KEY_0", "color.ui")
            .env("GIT_CONFIG_VALUE_0", "always")
            .output();

        match output {
            Ok(result) => {
                let stdout = String::from_utf8_lossy(&result.stdout);
                let stderr = String::from_utf8_lossy(&result.stderr);

                for line in stdout.lines() {
                    self.terminal_output.push(TerminalLine {
                        text: line.to_string(),
                        style: TerminalStyle::Output,
                    });
                }

                if !stderr.is_empty() {
                    for line in stderr.lines() {
                        self.terminal_output.push(TerminalLine {
                            text: line.to_string(),
                            style: TerminalStyle::Error,
                        });
                    }
                }

                if !result.status.success() {
                    if let Some(code) = result.status.code() {
                        self.terminal_output.push(TerminalLine {
                            text: format!("Exit code: {}", code),
                            style: TerminalStyle::Error,
                        });
                    }
                }
            }
            Err(e) => {
                self.terminal_output.push(TerminalLine {
                    text: format!("Error: {}", e),
                    style: TerminalStyle::Error,
                });
            }
        }
    }

    /// Navigate command history up
    pub(crate) fn navigate_history_up(&mut self) {
        if self.terminal_history.is_empty() {
            return;
        }

        let new_index = match self.terminal_history_index {
            None => Some(self.terminal_history.len() - 1),
            Some(0) => Some(0),
            Some(i) => Some(i - 1),
        };

        if let Some(idx) = new_index {
            self.terminal_history_index = Some(idx);
            self.terminal_input = self.terminal_history[idx].clone();
        }
    }

    /// Navigate command history down
    pub(crate) fn navigate_history_down(&mut self) {
        if self.terminal_history.is_empty() {
            return;
        }

        let new_index = match self.terminal_history_index {
            None => None,
            Some(i) if i >= self.terminal_history.len() - 1 => {
                self.terminal_input.clear();
                None
            }
            Some(i) => Some(i + 1),
        };

        if let Some(idx) = new_index {
            self.terminal_history_index = Some(idx);
            self.terminal_input = self.terminal_history[idx].clone();
        } else {
            self.terminal_history_index = None;
        }
    }
}
