//! In-editor game view - captures spawned Bevy game window and displays it

use super::BerryCodeApp;

/// Hide an external window by moving it off-screen.
/// Platform-specific: uses AppleScript on macOS, wmctrl/xdotool on Linux,
/// and the Windows API concepts via powershell on Windows.
#[allow(unused_variables)]
fn hide_external_window(window: &xcap::Window) {
    #[cfg(target_os = "macos")]
    {
        if let Ok(app_name) = window.app_name() {
            let script = format!(
                "tell application \"System Events\" to set position of first window of (first process whose name is \"{}\") to {{-10000, -10000}}",
                app_name
            );
            std::process::Command::new("osascript")
                .arg("-e")
                .arg(&script)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .ok();
        }
    }
    #[cfg(target_os = "linux")]
    {
        // Use xdotool to move the window off-screen (X11)
        if let Ok(app_name) = window.app_name() {
            let _ = std::process::Command::new("xdotool")
                .args(&[
                    "search",
                    "--name",
                    &app_name,
                    "windowmove",
                    "--",
                    "-10000",
                    "-10000",
                ])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();
        }
    }
    #[cfg(target_os = "windows")]
    {
        // Use powershell to move the window off-screen via .NET interop
        if let Ok(app_name) = window.app_name() {
            let script = format!(
                "Add-Type -TypeDefinition 'using System; using System.Runtime.InteropServices; public class W {{ [DllImport(\"user32.dll\")] public static extern bool MoveWindow(IntPtr h,int x,int y,int w,int ht,bool r); }}'; \
                 $p = Get-Process -Name '{}' -ErrorAction SilentlyContinue | Select-Object -First 1; \
                 if ($p) {{ [W]::MoveWindow($p.MainWindowHandle, -10000, -10000, 800, 600, $true) }}",
                app_name
            );
            let _ = std::process::Command::new("powershell")
                .args(&["-WindowStyle", "Hidden", "-Command", &script])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();
        }
    }
}

impl BerryCodeApp {
    /// Capture the running game window and update the texture
    pub(crate) fn update_game_view(&mut self, ctx: &egui::Context) {
        // Only capture if game is running and game view is enabled
        if self.run_process.is_none() || !self.game_view_open {
            return;
        }

        // Throttle: only capture every 100ms
        let now = std::time::Instant::now();
        if let Some(last) = self.game_view_last_capture {
            if now.duration_since(last).as_millis() < 100 {
                return;
            }
        }
        self.game_view_last_capture = Some(now);

        // Find the game window by multiple strategies
        let project_name = self.root_path.rsplit('/').next().unwrap_or("");
        let target_pid = self.run_process.as_ref().map(|p| p.id());

        let windows = match xcap::Window::all() {
            Ok(w) => w,
            Err(_) => return,
        };

        // Strategy 1: Match by PID (most reliable)
        // Strategy 2: Match by project name in window title or app name
        // Strategy 3: Match any Bevy window (title contains "Bevy" or "App")
        // Exclude our own BerryCode window
        let target_window = windows.iter().find(|w| {
            let app_name = w.app_name().unwrap_or_default();
            let title = w.title().unwrap_or_default();

            // Skip our own editor window
            if app_name.contains("berrycode") || title.contains("BerryCode") {
                return false;
            }

            // Skip system/zero-size windows
            if let Ok(width) = w.width() {
                if width == 0 {
                    return false;
                }
            }

            // Match by PID
            if let Some(pid) = target_pid {
                if let Ok(w_pid) = w.current_monitor().map(|_| w.id()) {
                    // xcap doesn't expose PID directly, so try name matching
                    let _ = (pid, w_pid);
                }
            }

            // Match by project/binary name
            let name_lower = app_name.to_lowercase();
            let title_lower = title.to_lowercase();
            let proj_lower = project_name.to_lowercase();

            if !proj_lower.is_empty()
                && (name_lower.contains(&proj_lower) || title_lower.contains(&proj_lower))
            {
                return true;
            }

            // Match common Bevy window titles
            if title_lower.contains("bevy") || title_lower.contains("app") {
                // Only match if we have a running process
                if target_pid.is_some() {
                    return true;
                }
            }

            // Match by cargo-built binary name (often the crate name)
            if let Some(crate_name) = Self::detect_crate_name(&self.root_path) {
                let crate_lower = crate_name.to_lowercase();
                if name_lower.contains(&crate_lower) || title_lower.contains(&crate_lower) {
                    return true;
                }
            }

            false
        });

        if let Some(window) = target_window {
            if let Ok(img) = window.capture_image() {
                let width = img.width() as usize;
                let height = img.height() as usize;
                let pixels: Vec<u8> = img.into_raw();

                let color_image =
                    egui::ColorImage::from_rgba_unmultiplied([width, height], &pixels);

                // Update or create texture
                if let Some(handle) = &mut self.game_view_texture {
                    handle.set(color_image, egui::TextureOptions::LINEAR);
                } else {
                    self.game_view_texture = Some(ctx.load_texture(
                        "game_view",
                        color_image,
                        egui::TextureOptions::LINEAR,
                    ));
                }

                // Hide the external window by moving it off-screen (capture still works)
                // Retry every few captures in case the window reappears
                if !self.game_view_window_hidden {
                    hide_external_window(window);
                    // Give the OS a moment to move the window, then mark as hidden
                    self.game_view_window_hidden = true;
                }
            }
        }

        // Request repaint to keep updating
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }

    /// Render the game view panel (floating window, only when not in GameView panel)
    pub(crate) fn render_game_view(&mut self, ctx: &egui::Context) {
        if !self.game_view_open {
            return;
        }

        // Skip floating window when GameView is the active central panel
        if self.active_panel == super::types::ActivePanel::GameView {
            return;
        }

        egui::Window::new("Game View")
            .default_size([800.0, 600.0])
            .resizable(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let is_running = self.run_process.is_some();
                    if !is_running {
                        if ui.button("Play").clicked() {
                            self.start_run();
                        }
                        ui.label("Game not running. Click Play to start.");
                    } else {
                        if ui.button("Stop").clicked() {
                            self.stop_run();
                            self.game_view_texture = None;
                        }
                        ui.colored_label(egui::Color32::from_rgb(80, 200, 80), "Playing");
                    }

                    ui.separator();
                    if ui.button("Close").clicked() {
                        self.game_view_open = false;
                    }
                });

                ui.separator();

                // Display captured frame
                if let Some(texture) = &self.game_view_texture {
                    let available = ui.available_size();
                    let tex_size = texture.size_vec2();
                    let scale = (available.x / tex_size.x).min(available.y / tex_size.y);
                    let display_size = egui::vec2(tex_size.x * scale, tex_size.y * scale);

                    ui.centered_and_justified(|ui| {
                        ui.image(egui::load::SizedTexture::new(texture.id(), display_size));
                    });
                } else if self.run_process.is_some() {
                    ui.centered_and_justified(|ui| {
                        ui.label("Waiting for game window...");
                    });
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.label("Game not running.");
                    });
                }
            });
    }

    /// Detect the crate name from Cargo.toml in the project
    fn detect_crate_name(project_path: &str) -> Option<String> {
        let cargo_toml = std::path::Path::new(project_path).join("Cargo.toml");
        let content = std::fs::read_to_string(cargo_toml).ok()?;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("name") && trimmed.contains('=') {
                let name = trimmed
                    .split('=')
                    .nth(1)?
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'')
                    .to_string();
                if !name.is_empty() {
                    return Some(name);
                }
            }
        }
        None
    }

    /// Open the game view panel (auto-starts game if not running)
    pub(crate) fn open_game_view(&mut self) {
        self.game_view_open = true;
        if self.run_process.is_none() {
            self.start_run();
        }
    }

    /// Render the Game View as the main central panel (used when ActivePanel::GameView)
    pub(crate) fn render_game_view_central(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Game View");
            ui.separator();

            let is_running = self.run_process.is_some();
            if is_running {
                if ui.button("Stop").clicked() {
                    self.stop_run();
                    self.game_view_texture = None;
                }
                ui.colored_label(egui::Color32::from_rgb(80, 200, 80), "Running");
            } else {
                if ui.button("Play").clicked() {
                    self.game_view_open = true;
                    self.start_run();
                }
                ui.colored_label(egui::Color32::from_rgb(150, 150, 150), "Stopped");
            }
        });

        ui.separator();

        // Enable capture while this panel is visible
        if self.run_process.is_some() {
            self.game_view_open = true;
        }

        if let Some(texture) = &self.game_view_texture {
            let available = ui.available_size();
            let tex_size = texture.size_vec2();
            let scale = (available.x / tex_size.x)
                .min(available.y / tex_size.y)
                .min(1.0);
            let display_size = egui::vec2(tex_size.x * scale, tex_size.y * scale);

            ui.centered_and_justified(|ui| {
                ui.image(egui::load::SizedTexture::new(texture.id(), display_size));
            });
        } else if self.run_process.is_some() {
            ui.centered_and_justified(|ui| {
                ui.label("Waiting for game window...");
            });
        } else {
            ui.vertical_centered(|ui| {
                ui.add_space(80.0);
                ui.label(
                    egui::RichText::new("Click Play to run your Bevy project")
                        .size(16.0)
                        .color(egui::Color32::from_gray(160)),
                );
                ui.add_space(16.0);
                if ui.button(egui::RichText::new("Play").size(14.0)).clicked() {
                    self.game_view_open = true;
                    self.start_run();
                }
            });
        }
    }
}
