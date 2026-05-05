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

/// One dependency from the project's `Cargo.toml`, alongside what
/// `crates.io` reports as the latest published version. Populated by
/// [`scan_installed_plugins`] + [`refresh_latest_versions`]. Drives the
/// "Installed Plugins" section of the browser, where outdated entries
/// surface an Update button. v0.5 / Plugin Browser auto-update.
#[derive(Debug, Clone)]
pub struct InstalledPlugin {
    pub name: String,
    /// Version string as it appears in `Cargo.toml` (e.g. `"0.18"` or
    /// `"0.18.1"`). May include a leading `^` / `~` etc.
    pub current_version: String,
    /// Latest version reported by crates.io, or `None` if we haven't
    /// fetched it yet (or the fetch failed). Compared against
    /// `current_version` to drive the Update button.
    pub latest_version: Option<String>,
}

/// Scan `<root>/Cargo.toml` for `[dependencies]` and return one
/// [`InstalledPlugin`] per entry. We don't filter to "Bevy plugins"
/// here — surfacing all deps is more useful than guessing, and the
/// Bevy ones float to the top of the list naturally because their
/// names start with `bevy_`. Both `name = "x"` and the table form
/// (`name = { version = "x", … }`) are handled.
pub fn scan_installed_plugins(root: &str) -> Vec<InstalledPlugin> {
    let cargo_path = format!("{}/Cargo.toml", root);
    let content = match std::fs::read_to_string(&cargo_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let parsed: toml::Value = match toml::from_str(&content) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let deps = parsed.get("dependencies").and_then(|d| d.as_table());
    let Some(deps) = deps else {
        return Vec::new();
    };

    let mut out = Vec::with_capacity(deps.len());
    for (name, value) in deps {
        let current = match value {
            toml::Value::String(s) => s.clone(),
            toml::Value::Table(t) => t
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            _ => continue,
        };
        // Skip path / git deps that don't have a version string at all
        // — crates.io has nothing to compare them to, and rendering
        // them with "?" alongside real deps is just noise.
        if current.is_empty() {
            continue;
        }
        out.push(InstalledPlugin {
            name: name.clone(),
            current_version: current,
            latest_version: None,
        });
    }
    // Bevy and bevy_* plugins float to the top, alphabetical within
    // each group. Makes the section scannable for the most common
    // "is my Bevy ecosystem up to date?" question.
    out.sort_by(|a, b| {
        let a_bevy = a.name == "bevy" || a.name.starts_with("bevy_");
        let b_bevy = b.name == "bevy" || b.name.starts_with("bevy_");
        b_bevy.cmp(&a_bevy).then_with(|| a.name.cmp(&b.name))
    });
    out
}

/// Fetch the latest published version of `crate_name` from crates.io.
/// Uses the same `curl` shell-out pattern as [`search_bevy_crates`] so
/// we don't fight with reqwest's blocking-vs-async story here. Returns
/// `None` on any error (no network, crate not found, parse failure).
///
/// `--max-time 10` caps the request so a hung crates.io endpoint or
/// flaky network can't freeze the caller indefinitely. Without it,
/// curl will sit on a stalled TLS handshake for the full system TCP
/// timeout (~75s on macOS, longer on some Linuxes) — long enough for
/// the editor to feel dead.
pub fn fetch_latest_version(crate_name: &str) -> Option<String> {
    let url = format!("https://crates.io/api/v1/crates/{}", crate_name);
    let output = std::process::Command::new("curl")
        .args([
            "-s",
            "--max-time",
            "10",
            "-H",
            "User-Agent: BerryCode-Editor",
            &url,
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    json.get("crate")
        .and_then(|c| c.get("max_stable_version"))
        .or_else(|| json.get("crate").and_then(|c| c.get("newest_version")))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Refresh `latest_version` on every entry by fetching from crates.io.
///
/// **Synchronous; do not call from the egui render path.** Blocks the
/// caller for up to ~10 s per plugin (the curl `--max-time` cap). The
/// UI uses [`refresh_latest_versions_async`] instead so the editor
/// stays responsive during the network round-trip.
pub fn refresh_latest_versions(plugins: &mut [InstalledPlugin]) {
    for p in plugins {
        p.latest_version = fetch_latest_version(&p.name);
    }
}

/// Async wrapper around [`refresh_latest_versions`]. Spawns a worker
/// thread, runs the curl-based version probes off the UI thread, and
/// hands the updated `Vec<InstalledPlugin>` back through an mpsc
/// channel. The caller stores the receiver in app state and polls it
/// per-frame with `try_recv` — the egui pass never blocks on network.
pub fn refresh_latest_versions_async(
    mut plugins: Vec<InstalledPlugin>,
) -> std::sync::mpsc::Receiver<Vec<InstalledPlugin>> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        refresh_latest_versions(&mut plugins);
        // Receiver dropped before we finish? That's the user closing the
        // panel mid-refresh — silently discard the result.
        let _ = tx.send(plugins);
    });
    rx
}

/// Numeric compare of dotted version strings. `"0.18.1"` > `"0.18"`,
/// `"1.0"` > `"0.99"`. Strips a leading `^`, `~`, `>=`, etc. so
/// requirement specifiers in `Cargo.toml` compare cleanly against
/// crates.io's plain version strings. Falls back to lexicographic
/// compare for genuinely weird inputs (pre-release tags etc.).
pub fn is_outdated(current: &str, latest: &str) -> bool {
    fn normalize(s: &str) -> Vec<u32> {
        s.trim_start_matches(|c: char| !c.is_ascii_digit())
            .split(|c: char| !c.is_ascii_digit())
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse::<u32>().ok())
            .collect()
    }
    let c = normalize(current);
    let l = normalize(latest);
    if c.is_empty() || l.is_empty() {
        return current != latest;
    }
    // Pad to equal length with zeros so "1.0" vs "1.0.1" compares
    // correctly as 1.0.0 < 1.0.1.
    let n = c.len().max(l.len());
    let mut c = c;
    let mut l = l;
    c.resize(n, 0);
    l.resize(n, 0);
    l > c
}

/// Rewrite the version string for `crate_name` in `Cargo.toml` to
/// `new_version`. Handles both shorthand (`name = "0.1"`) and table
/// form (`name = { version = "0.1", … }`). Returns `Err` with a human
/// message when the dep can't be located.
pub fn update_crate_in_cargo_toml(
    root: &str,
    crate_name: &str,
    new_version: &str,
) -> Result<(), String> {
    let cargo_path = format!("{}/Cargo.toml", root);
    let content = std::fs::read_to_string(&cargo_path).map_err(|e| e.to_string())?;

    // Walk the file line-by-line and rewrite the first line that
    // starts with `<crate_name> =` inside the [dependencies] section.
    // toml-edit would preserve formatting more faithfully, but we
    // already match the existing add path's "find a line, splice it"
    // approach for symmetry and to avoid a new dependency.
    let mut out = String::with_capacity(content.len());
    let mut in_deps = false;
    let mut found = false;
    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('[') {
            in_deps = trimmed == "[dependencies]";
        }
        if in_deps && !found && line.split('=').next().map(|s| s.trim()) == Some(crate_name) {
            // Two cases:
            //   1) `name = "x"`            → replace the quoted scalar.
            //   2) `name = { version = "x", … }` → replace the version field.
            if let Some(rewritten) = rewrite_version_line(line, new_version) {
                out.push_str(&rewritten);
                out.push('\n');
                found = true;
                continue;
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    if !found {
        return Err(format!(
            "Couldn't find `{}` under [dependencies] — manual edit required",
            crate_name
        ));
    }
    std::fs::write(&cargo_path, out).map_err(|e| e.to_string())
}

fn rewrite_version_line(line: &str, new_version: &str) -> Option<String> {
    // Find the first quoted string; that's the version in both
    // shorthand and table-form lines we care about (the table form's
    // `version = "x"` is the first quoted string on that line if the
    // table is single-line, which is the common case).
    let bytes = line.as_bytes();
    let q1 = bytes.iter().position(|&b| b == b'"')?;
    let q2 = bytes[q1 + 1..]
        .iter()
        .position(|&b| b == b'"')
        .map(|p| q1 + 1 + p)?;
    let mut out = String::with_capacity(line.len() + 8);
    out.push_str(&line[..q1 + 1]);
    out.push_str(new_version);
    out.push_str(&line[q2..]);
    Some(out)
}

/// Search crates.io for Bevy plugins (uses curl since reqwest may not
/// have blocking). `--max-time 10` keeps a stalled network from
/// freezing the caller; pair with [`search_bevy_crates_async`] when
/// calling from the egui render path.
pub fn search_bevy_crates(query: &str) -> Vec<CrateResult> {
    let url = format!(
        "https://crates.io/api/v1/crates?page=1&per_page=20&q=bevy+{}",
        urlencoding::encode(query)
    );

    let output = match std::process::Command::new("curl")
        .args([
            "-s",
            "--max-time",
            "10",
            "-H",
            "User-Agent: BerryCode-Editor",
            &url,
        ])
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
                downloads: c.get("downloads").and_then(|v| v.as_u64()).unwrap_or(0),
            });
        }
    }
    results
}

/// Add a crate to the project's Cargo.toml
pub fn add_crate_to_cargo_toml(root: &str, crate_name: &str, version: &str) -> Result<(), String> {
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
            // Window closed; clear the loaded flag so the next open
            // re-scans Cargo.toml. Cheap and avoids stale "installed"
            // lists when the user edits deps externally.
            if self.installed_plugins_loaded {
                self.installed_plugins.clear();
                self.installed_plugins_loaded = false;
            }
            return;
        }
        let mut open = self.plugin_browser_open;

        // First render after open: scan Cargo.toml so the user sees
        // their deps without having to click Refresh. Latest-version
        // fetch stays a manual action because it hits the network.
        if !self.installed_plugins_loaded {
            self.installed_plugins = scan_installed_plugins(&self.root_path);
            self.installed_plugins_loaded = true;
        }

        egui::Window::new("Bevy Plugin Browser")
            .open(&mut open)
            .default_size([600.0, 500.0])
            .show(ctx, |ui| {
                // ── Installed Plugins (auto-update) ──
                ui.heading("Installed");
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "{} dep{} from Cargo.toml",
                        self.installed_plugins.len(),
                        if self.installed_plugins.len() == 1 {
                            ""
                        } else {
                            "s"
                        }
                    ));
                    let refresh_in_flight = self.installed_plugins_refresh_rx.is_some();
                    let refresh_label = if refresh_in_flight {
                        "Refreshing…"
                    } else {
                        "Refresh"
                    };
                    if ui
                        .add_enabled(!refresh_in_flight, egui::Button::new(refresh_label))
                        .clicked()
                    {
                        // Re-scan synchronously (file IO, sub-millisecond)
                        // so the new dep set shows up immediately, then
                        // hand the version probes to a worker thread —
                        // crates.io HTTP round-trips would otherwise
                        // freeze the egui pass for ~10 s × N plugins.
                        self.installed_plugins = scan_installed_plugins(&self.root_path);
                        self.installed_plugins_refresh_rx = Some(refresh_latest_versions_async(
                            self.installed_plugins.clone(),
                        ));
                        self.status_message =
                            format!("Refreshing {} dependencies…", self.installed_plugins.len());
                        self.status_message_timestamp = Some(std::time::Instant::now());
                    }
                });

                // Drain the background refresh result, if any. Non-
                // blocking — `try_recv` returns immediately whether the
                // worker is still running or has already finished. The
                // egui pass calls this once per frame via the panel
                // render so results arrive within a frame of completion.
                if let Some(rx) = &self.installed_plugins_refresh_rx {
                    if let Ok(updated) = rx.try_recv() {
                        self.installed_plugins = updated;
                        self.installed_plugins_refresh_rx = None;
                        self.status_message = format!(
                            "Checked {} dependencies for updates",
                            self.installed_plugins.len()
                        );
                        self.status_message_timestamp = Some(std::time::Instant::now());
                    }
                }

                let mut update_request: Option<(String, String)> = None;
                egui::ScrollArea::vertical()
                    .id_salt("plugin_browser_installed_scroll")
                    .max_height(180.0)
                    .show(ui, |ui| {
                        if self.installed_plugins.is_empty() {
                            ui.label(
                                egui::RichText::new("No dependencies found in Cargo.toml.")
                                    .color(egui::Color32::from_gray(160)),
                            );
                        }
                        for plugin in &self.installed_plugins {
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    ui.strong(&plugin.name);
                                    ui.label(format!("v{}", plugin.current_version));
                                    match plugin.latest_version.as_ref() {
                                        Some(latest)
                                            if is_outdated(&plugin.current_version, latest) =>
                                        {
                                            ui.label(
                                                egui::RichText::new(format!("→ v{}", latest))
                                                    .color(egui::Color32::from_rgb(120, 220, 140))
                                                    .strong(),
                                            );
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    if ui
                                                        .button(format!("Update to {}", latest))
                                                        .clicked()
                                                    {
                                                        update_request = Some((
                                                            plugin.name.clone(),
                                                            latest.clone(),
                                                        ));
                                                    }
                                                },
                                            );
                                        }
                                        Some(_) => {
                                            ui.label(
                                                egui::RichText::new("up to date")
                                                    .color(egui::Color32::from_gray(140))
                                                    .size(11.0),
                                            );
                                        }
                                        None => {
                                            ui.label(
                                                egui::RichText::new("(unknown — click Refresh)")
                                                    .color(egui::Color32::from_gray(140))
                                                    .size(11.0),
                                            );
                                        }
                                    }
                                });
                            });
                        }
                    });

                if let Some((name, version)) = update_request {
                    match update_crate_in_cargo_toml(&self.root_path, &name, &version) {
                        Ok(_) => {
                            self.status_message =
                                format!("Updated {} to v{} in Cargo.toml", name, version);
                            self.status_message_timestamp = Some(std::time::Instant::now());
                            // Re-scan immediately so the row reflects
                            // the new on-disk state without waiting
                            // for another Refresh.
                            self.installed_plugins = scan_installed_plugins(&self.root_path);
                        }
                        Err(e) => {
                            self.status_message = format!("Update failed: {}", e);
                            self.status_message_timestamp = Some(std::time::Instant::now());
                        }
                    }
                }

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(4.0);

                // ── Search crates.io for new plugins ──
                ui.heading("Search");
                ui.horizontal(|ui| {
                    ui.label("crates.io:");
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.plugin_search_query)
                            .hint_text("e.g. rapier, hanabi, ui...")
                            .desired_width(300.0),
                    );
                    if ui.button("Search").clicked()
                        || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                    {
                        self.plugin_search_results = search_bevy_crates(&self.plugin_search_query);
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
                                                    self.status_message = format!("Failed: {}", e);
                                                    self.status_message_timestamp =
                                                        Some(std::time::Instant::now());
                                                }
                                            }
                                        }
                                        ui.label(format!("{} downloads", result.downloads));
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
                    if self.plugin_search_results.is_empty() && !self.plugin_search_query.is_empty()
                    {
                        ui.label("No results. Try a different search term.");
                    }
                });
            });
        self.plugin_browser_open = open;
    }
}
