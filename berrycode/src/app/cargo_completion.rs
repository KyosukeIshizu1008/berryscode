//! Cargo.toml crate name and version completion via crates.io API
//!
//! When editing Cargo.toml [dependencies], provides:
//! - Crate name completion (fuzzy search via crates.io API)
//! - Version completion (list available versions for a crate)

use super::BerryCodeApp;
use super::types::{LspCompletionItem, LspResponse};

/// Crate info from crates.io
#[derive(Debug, Clone)]
pub struct CrateInfo {
    pub name: String,
    pub max_version: String,
    pub description: String,
    pub downloads: u64,
}

/// Fetch crate suggestions from crates.io API
async fn search_crates(query: &str) -> Vec<CrateInfo> {
    if query.len() < 2 {
        return Vec::new();
    }

    let url = format!(
        "https://crates.io/api/v1/crates?q={}&per_page=15&sort=downloads",
        urlencoding::encode(query)
    );

    let client = match reqwest::Client::builder()
        .user_agent("BerryCode/0.2.0")
        .timeout(std::time::Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let response = match client.get(&url).send().await {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    let json: serde_json::Value = match response.json().await {
        Ok(j) => j,
        Err(_) => return Vec::new(),
    };

    let mut results = Vec::new();
    if let Some(crates) = json.get("crates").and_then(|c| c.as_array()) {
        for c in crates {
            results.push(CrateInfo {
                name: c
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("")
                    .to_string(),
                max_version: c
                    .get("max_version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("0.1.0")
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
            });
        }
    }

    results
}

/// Fetch available versions for a specific crate
async fn fetch_crate_versions(crate_name: &str) -> Vec<String> {
    let url = format!(
        "https://crates.io/api/v1/crates/{}/versions",
        urlencoding::encode(crate_name)
    );

    let client = match reqwest::Client::builder()
        .user_agent("BerryCode/0.2.0")
        .timeout(std::time::Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let response = match client.get(&url).send().await {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    let json: serde_json::Value = match response.json().await {
        Ok(j) => j,
        Err(_) => return Vec::new(),
    };

    let mut versions = Vec::new();
    if let Some(vs) = json.get("versions").and_then(|v| v.as_array()) {
        for v in vs.iter().take(20) {
            if let Some(num) = v.get("num").and_then(|n| n.as_str()) {
                let yanked = v
                    .get("yanked")
                    .and_then(|y| y.as_bool())
                    .unwrap_or(false);
                if !yanked {
                    versions.push(num.to_string());
                }
            }
        }
    }

    versions
}

impl BerryCodeApp {
    /// Trigger Cargo.toml completion for crate names or versions
    pub(crate) fn trigger_cargo_completion(&mut self) {
        let tab = match self.editor_tabs.get(self.active_tab_idx) {
            Some(t) => t,
            None => return,
        };

        // Only for Cargo.toml files
        if !tab.file_path.ends_with("Cargo.toml") {
            return;
        }

        let text = tab.buffer.to_string();
        let lines: Vec<&str> = text.lines().collect();
        let line = tab.cursor_line;
        let col = tab.cursor_col;

        if line >= lines.len() {
            return;
        }

        let current_line = lines[line];
        let trimmed = current_line.trim();

        // Detect context: are we completing a crate name or a version?
        let tx = match &self.lsp_response_tx {
            Some(t) => t.clone(),
            None => return,
        };

        let runtime = self.lsp_runtime.clone();

        // Check if we're in [dependencies] section
        let in_deps = lines[..line].iter().rev().any(|l| {
            let t = l.trim();
            t == "[dependencies]"
                || t == "[dev-dependencies]"
                || t == "[build-dependencies]"
                || t.starts_with("[dependencies.")
        });

        if !in_deps {
            return;
        }

        // If line has '=' and cursor is after '=', complete version
        if let Some(eq_pos) = trimmed.find('=') {
            let crate_name = trimmed[..eq_pos].trim().to_string();
            if col > current_line.find('=').unwrap_or(0) {
                // Version completion
                runtime.spawn(async move {
                    let versions = fetch_crate_versions(&crate_name).await;
                    let completions: Vec<LspCompletionItem> = versions
                        .iter()
                        .map(|v| LspCompletionItem {
                            label: format!("\"{}\"", v),
                            detail: Some(format!("{} v{}", crate_name, v)),
                            kind: "value".to_string(),
                            insert_text: Some(format!("\"{}\"", v)),
                            is_snippet: false,
                        })
                        .collect();
                    let _ = tx.send(LspResponse::Completions(completions));
                });
                return;
            }
        }

        // Crate name completion: get word at cursor
        let word: String = current_line[..col]
            .chars()
            .rev()
            .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .collect::<String>()
            .chars()
            .rev()
            .collect();

        if word.len() < 2 {
            return;
        }

        runtime.spawn(async move {
            let crates = search_crates(&word).await;
            let completions: Vec<LspCompletionItem> = crates
                .iter()
                .map(|c| LspCompletionItem {
                    label: c.name.clone(),
                    detail: Some(format!(
                        "v{} - {} ({} downloads)",
                        c.max_version,
                        if c.description.len() > 60 {
                            format!("{}...", &c.description[..57])
                        } else {
                            c.description.clone()
                        },
                        c.downloads
                    )),
                    kind: "module".to_string(),
                    insert_text: Some(format!("{} = \"{}\"", c.name, c.max_version)),
                    is_snippet: false,
                })
                .collect();
            let _ = tx.send(LspResponse::Completions(completions));
        });
    }
}
