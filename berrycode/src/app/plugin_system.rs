//! Plugin system: load, manage, and execute plugins
//!
//! Plugins are loaded from ~/.berrycode/plugins/<name>/
//! Each plugin has a manifest.json describing it.
//!
//! For now, plugins are script-based (shell commands on events).
//! Future: WASM-based plugins via wasmtime.
//!
//! Manifest format:
//! ```json
//! {
//!   "name": "my-plugin",
//!   "version": "1.0.0",
//!   "description": "My awesome plugin",
//!   "author": "Name",
//!   "main": "plugin.sh",
//!   "activationEvents": ["onLanguage:rust", "onCommand:myPlugin.run"],
//!   "contributes": {
//!     "commands": [
//!       { "command": "myPlugin.run", "title": "Run My Plugin" }
//!     ],
//!     "keybindings": [
//!       { "command": "myPlugin.run", "key": "ctrl+shift+p" }
//!     ]
//!   }
//! }
//! ```

use super::BerryCodeApp;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub main: String,
    #[serde(default)]
    pub repository: String,
    #[serde(default, rename = "activationEvents")]
    pub activation_events: Vec<String>,
    #[serde(default)]
    pub contributes: PluginContributes,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct PluginContributes {
    #[serde(default)]
    pub commands: Vec<PluginCommand>,
    #[serde(default)]
    pub keybindings: Vec<PluginKeybinding>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PluginCommand {
    pub command: String,
    pub title: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PluginKeybinding {
    pub command: String,
    pub key: String,
}

/// Loaded plugin
#[derive(Debug, Clone)]
pub struct LoadedPlugin {
    pub manifest: PluginManifest,
    pub path: String,
    pub enabled: bool,
}

/// Plugin manager state
pub struct PluginManager {
    pub plugins: Vec<LoadedPlugin>,
    pub search_query: String,
    pub marketplace_plugins: Vec<MarketplacePlugin>,
    pub marketplace_loading: bool,
}

/// Plugin from the marketplace
#[derive(Debug, Clone, Deserialize)]
pub struct MarketplacePlugin {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub downloads: u64,
    pub repository: String,
    pub installed: bool,
}

impl Default for PluginManager {
    fn default() -> Self {
        Self {
            plugins: Vec::new(),
            search_query: String::new(),
            marketplace_plugins: Vec::new(),
            marketplace_loading: false,
        }
    }
}

/// Scan ~/.berrycode/plugins/ for installed plugins
pub fn scan_installed_plugins() -> Vec<LoadedPlugin> {
    let plugins_dir = dirs::home_dir()
        .map(|h| h.join(".berrycode").join("plugins"))
        .unwrap_or_default();

    if !plugins_dir.exists() {
        let _ = std::fs::create_dir_all(&plugins_dir);
        return Vec::new();
    }

    let mut plugins = Vec::new();
    let entries = match std::fs::read_dir(&plugins_dir) {
        Ok(e) => e,
        Err(_) => return plugins,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let manifest_path = path.join("manifest.json");
            if manifest_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&manifest_path) {
                    if let Ok(manifest) = serde_json::from_str::<PluginManifest>(&content) {
                        plugins.push(LoadedPlugin {
                            manifest,
                            path: path.to_string_lossy().to_string(),
                            enabled: true,
                        });
                    }
                }
            }
        }
    }

    plugins
}

impl BerryCodeApp {
    /// Load installed plugins
    pub(crate) fn load_plugins(&mut self) {
        self.plugin_manager.plugins = scan_installed_plugins();
        tracing::info!("Loaded {} plugins", self.plugin_manager.plugins.len());
    }

    /// Execute a plugin command by name
    pub(crate) fn execute_plugin_command(&mut self, command_id: &str) {
        // Find the plugin that provides this command
        let plugin = self.plugin_manager.plugins.iter().find(|p| {
            p.enabled
                && p.manifest
                    .contributes
                    .commands
                    .iter()
                    .any(|c| c.command == command_id)
        });

        let plugin = match plugin {
            Some(p) => p.clone(),
            None => {
                tracing::warn!("Plugin command not found: {}", command_id);
                return;
            }
        };

        let script_path = std::path::Path::new(&plugin.path).join(&plugin.manifest.main);
        if !script_path.exists() {
            tracing::warn!("Plugin script not found: {}", script_path.display());
            return;
        }

        // Prepare environment variables for the script
        let current_file = self
            .editor_tabs
            .get(self.active_tab_idx)
            .map(|t| t.file_path.clone())
            .unwrap_or_default();
        let cursor_line = self
            .editor_tabs
            .get(self.active_tab_idx)
            .map(|t| t.cursor_line.to_string())
            .unwrap_or_default();

        let output = std::process::Command::new("sh")
            .arg(script_path.to_str().unwrap_or(""))
            .arg(command_id)
            .current_dir(&self.root_path)
            .env("BERRYCODE_PROJECT", &self.root_path)
            .env("BERRYCODE_FILE", &current_file)
            .env("BERRYCODE_LINE", &cursor_line)
            .env("BERRYCODE_COMMAND", command_id)
            .output();

        match output {
            Ok(result) => {
                let stdout = String::from_utf8_lossy(&result.stdout).to_string();
                let stderr = String::from_utf8_lossy(&result.stderr).to_string();
                if !stdout.is_empty() {
                    self.status_message = stdout.lines().next().unwrap_or("").to_string();
                    self.status_message_timestamp = Some(std::time::Instant::now());
                }
                if !stderr.is_empty() {
                    tracing::warn!("Plugin stderr: {}", stderr);
                }
                tracing::info!(
                    "Plugin {} executed command {}",
                    plugin.manifest.name,
                    command_id
                );
            }
            Err(e) => {
                tracing::error!("Failed to execute plugin: {}", e);
                self.status_message = format!("Plugin error: {}", e);
                self.status_message_timestamp = Some(std::time::Instant::now());
            }
        }
    }

    /// Get all registered plugin commands
    pub(crate) fn get_plugin_commands(&self) -> Vec<(&str, &str)> {
        self.plugin_manager
            .plugins
            .iter()
            .filter(|p| p.enabled)
            .flat_map(|p| {
                p.manifest
                    .contributes
                    .commands
                    .iter()
                    .map(|c| (c.command.as_str(), c.title.as_str()))
            })
            .collect()
    }

    /// Render plugin manager panel (extensions view)
    pub(crate) fn render_plugin_manager(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Extensions")
                    .size(13.0)
                    .color(egui::Color32::from_rgb(200, 200, 200))
                    .strong(),
            );
        });

        ui.add_space(4.0);

        // Search bar
        ui.add(
            egui::TextEdit::singleline(&mut self.plugin_manager.search_query)
                .hint_text("Search extensions...")
                .font(egui::FontId::proportional(12.0))
                .desired_width(f32::INFINITY),
        );

        ui.add_space(8.0);

        // Installed plugins
        ui.label(
            egui::RichText::new("INSTALLED")
                .size(10.0)
                .color(egui::Color32::from_rgb(120, 120, 120)),
        );
        ui.add_space(4.0);

        let filter = self.plugin_manager.search_query.to_lowercase();
        let mut toggle_idx: Option<usize> = None;

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                if self.plugin_manager.plugins.is_empty() {
                    ui.colored_label(
                        egui::Color32::from_rgb(100, 100, 100),
                        "No extensions installed",
                    );
                    ui.add_space(4.0);
                    ui.colored_label(
                        egui::Color32::from_rgb(80, 80, 80),
                        "Install from ~/.berrycode/plugins/",
                    );
                } else {
                    for (idx, plugin) in self.plugin_manager.plugins.iter().enumerate() {
                        if !filter.is_empty()
                            && !plugin.manifest.name.to_lowercase().contains(&filter)
                        {
                            continue;
                        }

                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                // Plugin icon
                                ui.label(
                                    egui::RichText::new("\u{ea8c}") // codicon: extensions
                                        .size(16.0)
                                        .color(if plugin.enabled {
                                            egui::Color32::from_rgb(100, 180, 255)
                                        } else {
                                            egui::Color32::from_rgb(80, 80, 80)
                                        }),
                                );

                                ui.vertical(|ui| {
                                    // Name + version
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new(&plugin.manifest.name)
                                                .size(12.0)
                                                .color(egui::Color32::from_rgb(200, 200, 200))
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "v{}",
                                                plugin.manifest.version
                                            ))
                                            .size(10.0)
                                            .color(egui::Color32::from_rgb(100, 100, 100)),
                                        );
                                    });

                                    // Description
                                    if !plugin.manifest.description.is_empty() {
                                        ui.label(
                                            egui::RichText::new(&plugin.manifest.description)
                                                .size(11.0)
                                                .color(egui::Color32::from_rgb(140, 140, 140)),
                                        );
                                    }

                                    // Author
                                    if !plugin.manifest.author.is_empty() {
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "by {}",
                                                plugin.manifest.author
                                            ))
                                            .size(10.0)
                                            .color(egui::Color32::from_rgb(100, 100, 100)),
                                        );
                                    }
                                });

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        let btn_text =
                                            if plugin.enabled { "Disable" } else { "Enable" };
                                        if ui.small_button(btn_text).clicked() {
                                            toggle_idx = Some(idx);
                                        }
                                    },
                                );
                            });
                        });

                        ui.add_space(2.0);
                    }
                }

                // Marketplace section
                ui.add_space(12.0);
                ui.label(
                    egui::RichText::new("MARKETPLACE")
                        .size(10.0)
                        .color(egui::Color32::from_rgb(120, 120, 120)),
                );
                ui.add_space(4.0);

                if self.plugin_manager.marketplace_plugins.is_empty() {
                    ui.colored_label(
                        egui::Color32::from_rgb(100, 100, 100),
                        "Search for extensions to install",
                    );
                }

                for plugin in &self.plugin_manager.marketplace_plugins {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("\u{ea8c}")
                                    .size(16.0)
                                    .color(egui::Color32::from_rgb(140, 140, 140)),
                            );
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(&plugin.name).size(12.0).strong());
                                    ui.label(
                                        egui::RichText::new(format!("v{}", plugin.version))
                                            .size(10.0)
                                            .color(egui::Color32::GRAY),
                                    );
                                });
                                ui.label(
                                    egui::RichText::new(&plugin.description)
                                        .size(11.0)
                                        .color(egui::Color32::from_rgb(140, 140, 140)),
                                );
                            });
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if plugin.installed {
                                        ui.colored_label(
                                            egui::Color32::from_rgb(80, 200, 80),
                                            "Installed",
                                        );
                                    } else {
                                        if ui.small_button("Install").clicked() {
                                            // Would download and install the plugin
                                        }
                                    }
                                },
                            );
                        });
                    });
                    ui.add_space(2.0);
                }
            });

        // Process deferred
        if let Some(idx) = toggle_idx {
            if let Some(p) = self.plugin_manager.plugins.get_mut(idx) {
                p.enabled = !p.enabled;
            }
        }
    }
}
