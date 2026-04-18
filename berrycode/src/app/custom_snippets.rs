//! Custom snippet system: load user-defined snippets from JSON files
//!
//! VS Code-compatible snippet format:
//! ```json
//! {
//!   "Print line": {
//!     "prefix": "println",
//!     "body": ["println!(\"${1:message}\");"],
//!     "description": "Print to stdout"
//!   }
//! }
//! ```
//!
//! Files are loaded from ~/.berrycode/snippets/*.json

use super::BerryCodeApp;
use super::types::LspCompletionItem;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
struct SnippetDef {
    pub prefix: SnippetPrefix,
    pub body: SnippetBody,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum SnippetPrefix {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum SnippetBody {
    Single(String),
    Lines(Vec<String>),
}

/// Loaded snippet ready for use
#[derive(Debug, Clone)]
pub struct LoadedSnippet {
    pub name: String,
    pub prefix: String,
    pub body: String,
    pub description: String,
}

/// Load snippets from ~/.berrycode/snippets/*.json
pub fn load_user_snippets() -> Vec<LoadedSnippet> {
    let mut snippets = Vec::new();

    let snippets_dir = dirs::home_dir()
        .map(|h| h.join(".berrycode").join("snippets"))
        .unwrap_or_default();

    if !snippets_dir.exists() {
        // Create directory and default Rust snippets
        let _ = std::fs::create_dir_all(&snippets_dir);
        let default_rust = include_str!("../../assets/default_rust_snippets.json");
        let default_path = snippets_dir.join("rust.json");
        if !default_path.exists() {
            let _ = std::fs::write(&default_path, default_rust);
        }
    }

    let entries = match std::fs::read_dir(&snippets_dir) {
        Ok(e) => e,
        Err(_) => return snippets,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "json").unwrap_or(false) {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(defs) = serde_json::from_str::<HashMap<String, SnippetDef>>(&content) {
                    for (name, def) in defs {
                        let prefix = match &def.prefix {
                            SnippetPrefix::Single(s) => s.clone(),
                            SnippetPrefix::Multiple(v) => {
                                v.first().cloned().unwrap_or_default()
                            }
                        };

                        let body = match &def.body {
                            SnippetBody::Single(s) => s.clone(),
                            SnippetBody::Lines(lines) => lines.join("\n"),
                        };

                        let description = def
                            .description
                            .clone()
                            .unwrap_or_else(|| name.clone());

                        snippets.push(LoadedSnippet {
                            name,
                            prefix,
                            body,
                            description,
                        });

                        // Also add alternate prefixes
                        if let SnippetPrefix::Multiple(prefixes) = &def.prefix {
                            for alt_prefix in prefixes.iter().skip(1) {
                                snippets.push(LoadedSnippet {
                                    name: format!("{} ({})", snippets.last().unwrap().name, alt_prefix),
                                    prefix: alt_prefix.clone(),
                                    body: match &def.body {
                                        SnippetBody::Single(s) => s.clone(),
                                        SnippetBody::Lines(lines) => lines.join("\n"),
                                    },
                                    description: def
                                        .description
                                        .clone()
                                        .unwrap_or_default(),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    snippets
}

impl BerryCodeApp {
    /// Load user snippets into memory
    pub(crate) fn load_snippets(&mut self) {
        self.user_snippets = load_user_snippets();
        tracing::info!("Loaded {} user snippets", self.user_snippets.len());
    }

    /// Get snippet completions matching a prefix
    pub(crate) fn get_snippet_completions(&self, prefix: &str) -> Vec<LspCompletionItem> {
        if prefix.is_empty() {
            return Vec::new();
        }

        let lower = prefix.to_lowercase();
        self.user_snippets
            .iter()
            .filter(|s| s.prefix.to_lowercase().starts_with(&lower))
            .map(|s| LspCompletionItem {
                label: s.prefix.clone(),
                detail: Some(format!("(snippet) {}", s.description)),
                kind: "snippet".to_string(),
                insert_text: Some(s.body.clone()),
                is_snippet: true,
            })
            .collect()
    }
}
