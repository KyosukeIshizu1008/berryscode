#![allow(dead_code)]
//! In-editor game view - captures spawned Bevy game window and displays it

use super::BerryCodeApp;

/// Hide an external window by making the process invisible.
/// Platform-specific: uses AppleScript on macOS, xdotool on Linux,
/// and PowerShell on Windows.
#[allow(unused_variables)]
fn hide_external_window(window: &xcap::Window) {
    #[cfg(target_os = "macos")]
    {
        if let Ok(app_name) = window.app_name() {
            // set visible to false — hides the entire process from the Dock and screen
            let script = format!(
                "tell application \"System Events\" to set visible of process \"{}\" to false",
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

        // Force continuous repaints while game is running (override reactive mode)
        ctx.request_repaint_after(std::time::Duration::from_millis(100));

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
            Err(e) => {
                tracing::warn!("xcap::Window::all() failed: {}", e);
                return;
            }
        };

        // Debug: log windows periodically to diagnose capture issues
        static LOG_COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let count = LOG_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if count % 100 == 0 {
            tracing::info!("xcap found {} windows (frame {}):", windows.len(), count);
            for w in &windows {
                let name = w.app_name().unwrap_or_default();
                let title = w.title().unwrap_or_default();
                let width = w.width().unwrap_or(0);
                if width > 0 {
                    tracing::info!("  app='{}' title='{}' w={}", name, title, width);
                }
            }
        }

        let crate_name = Self::detect_crate_name(&self.root_path);
        let target_window = windows.iter().find(|w| {
            let app_name = w.app_name().unwrap_or_default();
            let title = w.title().unwrap_or_default();

            // Skip our own editor window (match both "berrycode" and "BerryCode")
            let app_lower = app_name.to_lowercase();
            let title_lower = title.to_lowercase();
            if app_lower.contains("berrycode") || title_lower.contains("berrycode") {
                return false;
            }

            // Skip system/zero-size windows
            if let Ok(width) = w.width() {
                if width == 0 {
                    return false;
                }
            }

            // Must have a running process
            if target_pid.is_none() {
                return false;
            }

            let proj_lower = project_name.to_lowercase();

            // Match by project name in app name or title
            if !proj_lower.is_empty()
                && (app_lower.contains(&proj_lower) || title_lower.contains(&proj_lower))
            {
                return true;
            }

            // Match by crate name
            if let Some(ref cn) = crate_name {
                let cn_lower = cn.to_lowercase();
                if app_lower.contains(&cn_lower) || title_lower.contains(&cn_lower) {
                    return true;
                }
            }

            // Match common Bevy default window titles ("App", "Bevy App")
            // but exclude common system apps
            if (title == "App" || title_lower.contains("bevy"))
                && !app_lower.contains("finder")
                && !app_lower.contains("system")
                && !app_lower.contains("chrome")
                && !app_lower.contains("discord")
                && !app_lower.contains("code")
                && !app_lower.contains("iterm")
            {
                return true;
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

                // Always try to hide the external window (it may reappear after resize etc.)
                hide_external_window(window);
                self.game_view_window_hidden = true;
            }
        }

        // Request repaint to keep updating
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }

    /// Render the game view panel (floating window, only when not in GameView panel)
    pub(crate) fn render_game_view(&mut self, _ctx: &egui::Context) {
        if !self.game_view_open {
            return;
        }

        // Game view is now rendered inline in the editor area, not as a floating window
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
            ui.heading(self.tr("Game View"));
            ui.separator();

            let is_running = self.run_process.is_some();
            if is_running {
                if ui.button(self.tr("Stop")).clicked() {
                    self.stop_run();
                    self.game_view_texture = None;
                }
                ui.colored_label(egui::Color32::from_rgb(80, 200, 80), "Running");
            } else {
                if ui.button(self.tr("Play")).clicked() {
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
                ui.label(self.tr("Waiting for game window..."));
            });
        } else {
            ui.vertical_centered(|ui| {
                ui.add_space(80.0);
                ui.label(
                    egui::RichText::new(self.tr("Click Play to run your Bevy project"))
                        .size(16.0)
                        .color(egui::Color32::from_gray(160)),
                );
                ui.add_space(16.0);
                if ui
                    .button(egui::RichText::new(self.tr("Play")).size(14.0))
                    .clicked()
                {
                    self.game_view_open = true;
                    self.start_run();
                }
            });
        }
    }
}
