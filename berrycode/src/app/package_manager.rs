//! Package Manager panel — GUI for managing Cargo.toml dependencies

use super::BerryCodeApp;

/// State for the Package Manager panel
#[derive(Debug, Clone)]
pub struct PackageManagerState {
    pub dependencies: Vec<CargoDep>,
    pub search_query: String,
    pub search_results: Vec<CrateSearchResult>,
    pub searching: bool,
    pub add_crate_name: String,
    pub add_crate_version: String,
    pub loaded: bool,
}

impl Default for PackageManagerState {
    fn default() -> Self {
        Self {
            dependencies: Vec::new(),
            search_query: String::new(),
            search_results: Vec::new(),
            searching: false,
            add_crate_name: String::new(),
            add_crate_version: String::new(),
            loaded: false,
        }
    }
}

/// A single dependency entry from Cargo.toml
#[derive(Debug, Clone)]
pub struct CargoDep {
    pub name: String,
    pub version: String,
    pub features: Vec<String>,
    pub enabled_features: Vec<String>,
    pub optional: bool,
}

/// A search result from crates.io
#[derive(Debug, Clone)]
pub struct CrateSearchResult {
    pub name: String,
    pub version: String,
    pub description: String,
    pub downloads: u64,
}

// ─── Cargo.toml Parsing ─────────────────────────────────────────────

/// Load dependencies from a project's Cargo.toml.
pub fn load_cargo_deps(root: &str) -> Option<Vec<CargoDep>> {
    let path = format!("{}/Cargo.toml", root);
    let content = std::fs::read_to_string(&path).ok()?;
    let doc: toml::Value = content.parse().ok()?;

    let deps_table = doc.get("dependencies")?.as_table()?;
    let mut deps = Vec::new();

    for (name, value) in deps_table {
        match value {
            toml::Value::String(version) => {
                deps.push(CargoDep {
                    name: name.clone(),
                    version: version.clone(),
                    features: Vec::new(),
                    enabled_features: Vec::new(),
                    optional: false,
                });
            }
            toml::Value::Table(table) => {
                let version = table
                    .get("version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("*")
                    .to_string();

                let features: Vec<String> = table
                    .get("features")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();

                let optional = table
                    .get("optional")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                deps.push(CargoDep {
                    name: name.clone(),
                    version,
                    features: features.clone(),
                    enabled_features: features,
                    optional,
                });
            }
            _ => {}
        }
    }

    deps.sort_by(|a, b| a.name.cmp(&b.name));
    Some(deps)
}

/// Save dependencies back to Cargo.toml, preserving existing content.
pub fn save_cargo_deps(root: &str, deps: &[CargoDep]) -> Result<(), String> {
    let path = format!("{}/Cargo.toml", root);
    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut doc: toml::value::Table = content
        .parse::<toml::Value>()
        .map_err(|e| e.to_string())?
        .as_table()
        .cloned()
        .ok_or_else(|| "Cargo.toml is not a table".to_string())?;

    let mut new_deps = toml::value::Table::new();
    for dep in deps {
        if dep.enabled_features.is_empty() && !dep.optional {
            // Simple string version
            new_deps.insert(dep.name.clone(), toml::Value::String(dep.version.clone()));
        } else {
            // Table form with version + features/optional
            let mut table = toml::value::Table::new();
            table.insert(
                "version".to_string(),
                toml::Value::String(dep.version.clone()),
            );
            if !dep.enabled_features.is_empty() {
                let features: Vec<toml::Value> = dep
                    .enabled_features
                    .iter()
                    .map(|f| toml::Value::String(f.clone()))
                    .collect();
                table.insert("features".to_string(), toml::Value::Array(features));
            }
            if dep.optional {
                table.insert("optional".to_string(), toml::Value::Boolean(true));
            }
            new_deps.insert(dep.name.clone(), toml::Value::Table(table));
        }
    }
    doc.insert("dependencies".to_string(), toml::Value::Table(new_deps));

    let output = toml::to_string_pretty(&toml::Value::Table(doc)).map_err(|e| e.to_string())?;
    std::fs::write(&path, output).map_err(|e| e.to_string())?;
    Ok(())
}

// ─── UI Rendering ───────────────────────────────────────────────────

impl BerryCodeApp {
    /// Render the Package Manager as a floating window.
    pub(crate) fn render_package_manager_window(&mut self, ctx: &egui::Context) {
        if !self.package_manager_open {
            return;
        }

        // Lazy-load dependencies on first open
        if !self.package_manager.loaded {
            if let Some(deps) = load_cargo_deps(&self.root_path) {
                self.package_manager.dependencies = deps;
            }
            self.package_manager.loaded = true;
        }

        let mut open = self.package_manager_open;
        egui::Window::new("Packages")
            .open(&mut open)
            .default_size([480.0, 520.0])
            .resizable(true)
            .show(ctx, |ui| {
                self.render_package_manager(ui);
            });
        self.package_manager_open = open;
    }

    /// Render Package Manager content inside a Ui region.
    pub(crate) fn render_package_manager(&mut self, ui: &mut egui::Ui) {
        // ── Header ──
        ui.horizontal(|ui| {
            ui.heading("PACKAGES");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Reload").clicked() {
                    if let Some(deps) = load_cargo_deps(&self.root_path) {
                        self.package_manager.dependencies = deps;
                    }
                    self.status_message = "Reloaded Cargo.toml".to_string();
                    self.status_message_timestamp = Some(std::time::Instant::now());
                }
            });
        });
        ui.separator();

        // ── Dependency List (scrollable) ──
        let mut remove_idx: Option<usize> = None;
        let mut deps_changed = false;

        let row_count = self.package_manager.dependencies.len();
        if row_count == 0 {
            ui.label("No dependencies found in Cargo.toml");
        } else {
            egui::ScrollArea::vertical()
                .max_height(280.0)
                .id_salt("pkg_dep_list")
                .show(ui, |ui| {
                    for (i, dep) in self.package_manager.dependencies.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            // Name
                            ui.label(
                                egui::RichText::new(&dep.name)
                                    .color(egui::Color32::from_rgb(84, 166, 224))
                                    .strong(),
                            );

                            // Version (editable)
                            let response = ui.add(
                                egui::TextEdit::singleline(&mut dep.version)
                                    .desired_width(80.0)
                                    .hint_text("version"),
                            );
                            if response.changed() {
                                deps_changed = true;
                            }

                            // Optional badge
                            if dep.optional {
                                ui.label(
                                    egui::RichText::new("[opt]")
                                        .small()
                                        .color(egui::Color32::from_rgb(200, 160, 80)),
                                );
                            }

                            // Features count
                            if !dep.enabled_features.is_empty() {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} feat",
                                        dep.enabled_features.len()
                                    ))
                                    .small()
                                    .color(egui::Color32::from_rgb(150, 150, 150)),
                                );
                            }

                            // Remove button
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui
                                        .button(
                                            egui::RichText::new("x")
                                                .color(egui::Color32::from_rgb(255, 100, 100)),
                                        )
                                        .clicked()
                                    {
                                        remove_idx = Some(i);
                                    }
                                },
                            );
                        });
                    }
                });
        }

        // Apply removals
        if let Some(idx) = remove_idx {
            self.package_manager.dependencies.remove(idx);
            deps_changed = true;
        }

        // Auto-save when changed
        if deps_changed {
            match save_cargo_deps(&self.root_path, &self.package_manager.dependencies) {
                Ok(()) => {
                    self.status_message = "Cargo.toml updated".to_string();
                    self.status_message_timestamp = Some(std::time::Instant::now());
                }
                Err(e) => {
                    self.status_message = format!("Failed to save Cargo.toml: {}", e);
                    self.status_message_timestamp = Some(std::time::Instant::now());
                }
            }
        }

        ui.separator();

        // ── Add Dependency ──
        ui.label(
            egui::RichText::new("Add Dependency")
                .strong()
                .color(egui::Color32::from_rgb(200, 200, 200)),
        );

        ui.horizontal(|ui| {
            ui.label("Name:");
            ui.add(
                egui::TextEdit::singleline(&mut self.package_manager.add_crate_name)
                    .desired_width(140.0)
                    .hint_text("e.g. serde"),
            );
            ui.label("Version:");
            ui.add(
                egui::TextEdit::singleline(&mut self.package_manager.add_crate_version)
                    .desired_width(80.0)
                    .hint_text("e.g. 1.0"),
            );

            if ui.button("Add").clicked() && !self.package_manager.add_crate_name.is_empty() {
                let version = if self.package_manager.add_crate_version.is_empty() {
                    "*".to_string()
                } else {
                    self.package_manager.add_crate_version.clone()
                };
                self.package_manager.dependencies.push(CargoDep {
                    name: self.package_manager.add_crate_name.clone(),
                    version,
                    features: Vec::new(),
                    enabled_features: Vec::new(),
                    optional: false,
                });
                self.package_manager
                    .dependencies
                    .sort_by(|a, b| a.name.cmp(&b.name));

                match save_cargo_deps(&self.root_path, &self.package_manager.dependencies) {
                    Ok(()) => {
                        self.status_message = format!(
                            "Added {} to Cargo.toml",
                            self.package_manager.add_crate_name
                        );
                        self.status_message_timestamp = Some(std::time::Instant::now());
                    }
                    Err(e) => {
                        self.status_message = format!("Failed to save: {}", e);
                        self.status_message_timestamp = Some(std::time::Instant::now());
                    }
                }
                self.package_manager.add_crate_name.clear();
                self.package_manager.add_crate_version.clear();
            }
        });

        ui.separator();

        // ── Crates.io Search ──
        ui.label(
            egui::RichText::new("Search crates.io")
                .strong()
                .color(egui::Color32::from_rgb(200, 200, 200)),
        );

        ui.horizontal(|ui| {
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.package_manager.search_query)
                    .desired_width(200.0)
                    .hint_text("Search crates..."),
            );

            let enter_pressed =
                response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

            if (ui.button("Search").clicked() || enter_pressed)
                && !self.package_manager.search_query.is_empty()
                && !self.package_manager.searching
            {
                self.package_manager.searching = true;
                self.package_manager.search_results.clear();

                let query = self.package_manager.search_query.clone();
                let query_encoded = urlencoding::encode(&query).to_string();
                let url = format!(
                    "https://crates.io/api/v1/crates?q={}&per_page=10",
                    query_encoded
                );

                // Fire off async search
                let rt = self.lsp_runtime.clone();
                let (tx, rx) = std::sync::mpsc::channel();
                rt.spawn(async move {
                    let client = reqwest::Client::builder()
                        .user_agent("BerryCode IDE (berrycode@oracleberry.co.jp)")
                        .build();
                    let result = match client {
                        Ok(c) => c.get(&url).send().await,
                        Err(e) => {
                            let _ = tx.send(Err(format!("HTTP client error: {}", e)));
                            return;
                        }
                    };
                    match result {
                        Ok(resp) => match resp.text().await {
                            Ok(body) => {
                                let _ = tx.send(Ok(body));
                            }
                            Err(e) => {
                                let _ = tx.send(Err(format!("Read error: {}", e)));
                            }
                        },
                        Err(e) => {
                            let _ = tx.send(Err(format!("Request error: {}", e)));
                        }
                    }
                });

                // Store receiver for polling (we'll check it each frame)
                self.package_manager_search_rx = Some(rx);
            }
        });

        // Poll for search results
        if let Some(rx) = &self.package_manager_search_rx {
            if let Ok(result) = rx.try_recv() {
                self.package_manager.searching = false;
                match result {
                    Ok(body) => {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                            if let Some(crates) = json.get("crates").and_then(|c| c.as_array()) {
                                self.package_manager.search_results = crates
                                    .iter()
                                    .filter_map(|c| {
                                        Some(CrateSearchResult {
                                            name: c.get("name")?.as_str()?.to_string(),
                                            version: c
                                                .get("newest_version")
                                                .or_else(|| c.get("max_version"))
                                                .and_then(|v| v.as_str())
                                                .unwrap_or("?")
                                                .to_string(),
                                            description: c
                                                .get("description")
                                                .and_then(|d| d.as_str())
                                                .unwrap_or("")
                                                .to_string(),
                                            downloads: c
                                                .get("downloads")
                                                .and_then(|d| d.as_u64())
                                                .unwrap_or(0),
                                        })
                                    })
                                    .collect();
                            }
                        }
                    }
                    Err(e) => {
                        self.status_message = format!("Search failed: {}", e);
                        self.status_message_timestamp = Some(std::time::Instant::now());
                    }
                }
                self.package_manager_search_rx = None;
            }
        }

        if self.package_manager.searching {
            ui.spinner();
            ui.label("Searching...");
        }

        // Display search results
        if !self.package_manager.search_results.is_empty() {
            ui.add_space(4.0);
            egui::ScrollArea::vertical()
                .max_height(200.0)
                .id_salt("pkg_search_results")
                .show(ui, |ui| {
                    for result in &self.package_manager.search_results {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(&result.name)
                                        .color(egui::Color32::from_rgb(84, 166, 224))
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(&result.version)
                                        .small()
                                        .color(egui::Color32::from_rgb(150, 200, 150)),
                                );
                                ui.label(
                                    egui::RichText::new(format!("({} dl)", result.downloads))
                                        .small()
                                        .color(egui::Color32::from_rgb(130, 130, 130)),
                                );

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        // Check if already added
                                        let already_exists = self
                                            .package_manager
                                            .dependencies
                                            .iter()
                                            .any(|d| d.name == result.name);
                                        if already_exists {
                                            ui.label(
                                                egui::RichText::new("installed")
                                                    .small()
                                                    .color(egui::Color32::from_rgb(100, 180, 100)),
                                            );
                                        } else {
                                            let name = result.name.clone();
                                            let version = result.version.clone();
                                            if ui.small_button("+ Add").clicked() {
                                                self.package_manager.add_crate_name = name;
                                                self.package_manager.add_crate_version = version;
                                            }
                                        }
                                    },
                                );
                            });
                            if !result.description.is_empty() {
                                ui.label(
                                    egui::RichText::new(&result.description)
                                        .small()
                                        .color(egui::Color32::from_rgb(170, 170, 170)),
                                );
                            }
                        });
                    }
                });
        }
    }
}

// ─── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_cargo_deps_simple() {
        let dir = tempfile::tempdir().unwrap();
        let cargo_toml = dir.path().join("Cargo.toml");
        std::fs::write(
            &cargo_toml,
            r#"
[package]
name = "test"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = "1.0"
tokio = { version = "1", features = ["full"] }
anyhow = "1.0"
"#,
        )
        .unwrap();

        let deps = load_cargo_deps(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(deps.len(), 3);

        // Sorted alphabetically
        assert_eq!(deps[0].name, "anyhow");
        assert_eq!(deps[0].version, "1.0");
        assert!(deps[0].features.is_empty());

        assert_eq!(deps[1].name, "serde");
        assert_eq!(deps[1].version, "1.0");

        assert_eq!(deps[2].name, "tokio");
        assert_eq!(deps[2].version, "1");
        assert_eq!(deps[2].enabled_features, vec!["full".to_string()]);
    }

    #[test]
    fn test_load_cargo_deps_optional() {
        let dir = tempfile::tempdir().unwrap();
        let cargo_toml = dir.path().join("Cargo.toml");
        std::fs::write(
            &cargo_toml,
            r#"
[package]
name = "test"
version = "0.1.0"

[dependencies]
openssl = { version = "0.10", optional = true }
"#,
        )
        .unwrap();

        let deps = load_cargo_deps(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, "openssl");
        assert!(deps[0].optional);
    }

    #[test]
    fn test_load_cargo_deps_no_deps() {
        let dir = tempfile::tempdir().unwrap();
        let cargo_toml = dir.path().join("Cargo.toml");
        std::fs::write(
            &cargo_toml,
            r#"
[package]
name = "test"
version = "0.1.0"
"#,
        )
        .unwrap();

        // No [dependencies] section -> returns None
        let result = load_cargo_deps(dir.path().to_str().unwrap());
        assert!(result.is_none());
    }

    #[test]
    fn test_save_cargo_deps_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let cargo_toml = dir.path().join("Cargo.toml");
        std::fs::write(
            &cargo_toml,
            r#"
[package]
name = "test"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = "1.0"
"#,
        )
        .unwrap();

        let mut deps = load_cargo_deps(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(deps.len(), 1);

        // Add a new dependency
        deps.push(CargoDep {
            name: "anyhow".to_string(),
            version: "1.0".to_string(),
            features: Vec::new(),
            enabled_features: Vec::new(),
            optional: false,
        });

        save_cargo_deps(dir.path().to_str().unwrap(), &deps).unwrap();

        // Reload and verify
        let reloaded = load_cargo_deps(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(reloaded.len(), 2);
        assert!(reloaded.iter().any(|d| d.name == "anyhow"));
        assert!(reloaded.iter().any(|d| d.name == "serde"));
    }

    #[test]
    fn test_save_cargo_deps_with_features() {
        let dir = tempfile::tempdir().unwrap();
        let cargo_toml = dir.path().join("Cargo.toml");
        std::fs::write(
            &cargo_toml,
            r#"
[package]
name = "test"
version = "0.1.0"

[dependencies]
"#,
        )
        .unwrap();

        let deps = vec![CargoDep {
            name: "tokio".to_string(),
            version: "1".to_string(),
            features: vec!["full".to_string()],
            enabled_features: vec!["full".to_string()],
            optional: false,
        }];

        save_cargo_deps(dir.path().to_str().unwrap(), &deps).unwrap();

        let reloaded = load_cargo_deps(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(reloaded.len(), 1);
        assert_eq!(reloaded[0].name, "tokio");
        assert_eq!(reloaded[0].enabled_features, vec!["full".to_string()]);
    }

    #[test]
    fn test_remove_dependency() {
        let dir = tempfile::tempdir().unwrap();
        let cargo_toml = dir.path().join("Cargo.toml");
        std::fs::write(
            &cargo_toml,
            r#"
[package]
name = "test"
version = "0.1.0"

[dependencies]
serde = "1.0"
anyhow = "1.0"
regex = "1.10"
"#,
        )
        .unwrap();

        let mut deps = load_cargo_deps(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(deps.len(), 3);

        // Remove regex
        deps.retain(|d| d.name != "regex");
        save_cargo_deps(dir.path().to_str().unwrap(), &deps).unwrap();

        let reloaded = load_cargo_deps(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(reloaded.len(), 2);
        assert!(!reloaded.iter().any(|d| d.name == "regex"));
    }
}
