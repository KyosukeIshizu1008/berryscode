//! crates.io Bevy plugin browser.
//! Search for Bevy plugins and add them to Cargo.toml.

use crate::app::BerryCodeApp;

#[derive(Debug, Clone)]
pub struct CrateResult {
    pub name: String,
    pub version: String,
    pub description: String,
    pub downloads: u64,
}

/// Search crates.io for Bevy plugins (uses curl since reqwest may not have blocking)
pub fn search_bevy_crates(query: &str) -> Vec<CrateResult> {
    let url = format!(
        "https://crates.io/api/v1/crates?page=1&per_page=20&q=bevy+{}",
        urlencoding::encode(query)
    );

    let output = match std::process::Command::new("curl")
        .args(["-s", "-H", "User-Agent: BerryCode-Editor", &url])
        .output()
    {
        Ok(o) => o,
        Err(_) => return vec![],
    };

    if !output.status.success() {
        return vec![];
    }

    let json: serde_json::Value = match serde_json::from_slice(&output.stdout) {
        Ok(j) => j,
        Err(_) => return vec![],
    };

    let mut results = Vec::new();
    if let Some(crates) = json.get("crates").and_then(|c| c.as_array()) {
        for c in crates {
            results.push(CrateResult {
                name: c
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                version: c
                    .get("newest_version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                description: c
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                downloads: c
                    .get("downloads")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0),
            });
        }
    }
    results
}

/// Add a crate to the project's Cargo.toml
pub fn add_crate_to_cargo_toml(
    root: &str,
    crate_name: &str,
    version: &str,
) -> Result<(), String> {
    let cargo_path = format!("{}/Cargo.toml", root);
    let content = std::fs::read_to_string(&cargo_path).map_err(|e| e.to_string())?;

    // Check if already present
    if content.contains(&format!("{} ", crate_name))
        || content.contains(&format!("{}=", crate_name))
    {
        return Err(format!("{} already in Cargo.toml", crate_name));
    }

    // Find [dependencies] section and append
    let dep_line = format!("{} = \"{}\"\n", crate_name, version);
    let new_content = if let Some(pos) = content.find("[dependencies]") {
        let after_header = pos + "[dependencies]".len();
        let next_newline = content[after_header..]
            .find('\n')
            .map(|p| after_header + p + 1)
            .unwrap_or(content.len());
        format!(
            "{}{}{}",
            &content[..next_newline],
            dep_line,
            &content[next_newline..]
        )
    } else {
        format!("{}\n[dependencies]\n{}", content, dep_line)
    };

    std::fs::write(&cargo_path, new_content).map_err(|e| e.to_string())
}

impl BerryCodeApp {
    pub(crate) fn render_plugin_browser(&mut self, ctx: &egui::Context) {
        if !self.plugin_browser_open {
            return;
        }
        let mut open = self.plugin_browser_open;

        egui::Window::new("Bevy Plugin Browser")
            .open(&mut open)
            .default_size([600.0, 400.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Search crates.io:");
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.plugin_search_query)
                            .hint_text("e.g. rapier, hanabi, ui...")
                            .desired_width(300.0),
                    );
                    if ui.button("Search").clicked()
                        || (response.lost_focus()
                            && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                    {
                        self.plugin_search_results =
                            search_bevy_crates(&self.plugin_search_query);
                    }
                });

                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    for result in &self.plugin_search_results.clone() {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.strong(&result.name);
                                ui.label(format!("v{}", result.version));
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui.button("Add to Cargo.toml").clicked() {
                                            match add_crate_to_cargo_toml(
                                                &self.root_path,
                                                &result.name,
                                                &result.version,
                                            ) {
                                                Ok(_) => {
                                                    self.status_message = format!(
                                                        "Added {} v{} to Cargo.toml",
                                                        result.name, result.version
                                                    );
                                                    self.status_message_timestamp =
                                                        Some(std::time::Instant::now());
                                                }
                                                Err(e) => {
                                                    self.status_message =
                                                        format!("Failed: {}", e);
                                                    self.status_message_timestamp =
                                                        Some(std::time::Instant::now());
                                                }
                                            }
                                        }
                                        ui.label(format!(
                                            "{} downloads",
                                            result.downloads
                                        ));
                                    },
                                );
                            });
                            ui.label(
                                egui::RichText::new(&result.description)
                                    .size(11.0)
                                    .color(egui::Color32::from_gray(160)),
                            );
                        });
                    }
                    if self.plugin_search_results.is_empty()
                        && !self.plugin_search_query.is_empty()
                    {
                        ui.label("No results. Try a different search term.");
                    }
                });
            });
        self.plugin_browser_open = open;
    }
}
