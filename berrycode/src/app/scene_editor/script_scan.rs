//! Scans the user's Rust source tree for `#[derive(..Component..)] struct X ...`
//! patterns so the inspector can suggest type names for `CustomScript`
//! attachments.
//!
//! This is intentionally a shallow text scan, NOT a real Rust parser. It walks
//! each `.rs` file line-by-line and, whenever it sees a `#[derive(...)]`
//! attribute that mentions `Component`, captures the identifier on the next
//! `struct`/`enum` definition it encounters. False positives are possible on
//! pathological input (e.g. macros that generate derives) but the cost of being
//! wrong is merely surfacing an irrelevant suggestion in a ComboBox.
//!
//! The scanner is decoupled from the inspector UI in v1 — a follow-up phase
//! can wire the results into a suggestions dropdown. Until that wiring lands,
//! the functions here are only exercised by unit tests, so we silence
//! `dead_code` at the module level.

#![allow(dead_code)]

use std::path::Path;

/// Return a deduplicated, sorted list of `struct` / `enum` type names in
/// `root` whose definition sits directly below a `#[derive(...)]` attribute
/// that contains `Component`.
///
/// The scan skips `target/` and any hidden directories (starting with `.`).
pub fn scan_components(root: &str) -> Vec<String> {
    let root_path = Path::new(root);
    let mut rs_files: Vec<std::path::PathBuf> = Vec::new();
    collect_rs_files(root_path, &mut rs_files);

    let mut out: Vec<String> = Vec::new();
    for path in rs_files {
        if let Ok(content) = std::fs::read_to_string(&path) {
            scan_text(&content, &mut out);
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Recursively collect every `.rs` file under `dir`, skipping `target/` and
/// hidden directories.
fn collect_rs_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            if name == "target" || name.starts_with('.') {
                continue;
            }
            collect_rs_files(&path, out);
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

/// Walk `text` line-by-line; when a `#[derive(... Component ...)]` attribute is
/// seen, look for the next `struct Name` or `enum Name` line and push the
/// identifier into `out`. Intervening attribute lines are tolerated.
fn scan_text(text: &str, out: &mut Vec<String>) {
    let mut pending_derive = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("#[derive(") && trimmed.contains("Component") {
            pending_derive = true;
            continue;
        }
        if pending_derive {
            if let Some(name) = trimmed
                .strip_prefix("pub struct ")
                .or_else(|| trimmed.strip_prefix("struct "))
                .or_else(|| trimmed.strip_prefix("pub enum "))
                .or_else(|| trimmed.strip_prefix("enum "))
            {
                let name = name
                    .split(|c: char| !c.is_alphanumeric() && c != '_')
                    .next()
                    .unwrap_or("");
                if !name.is_empty() {
                    out.push(name.to_string());
                }
                pending_derive = false;
            } else if trimmed.starts_with("#[") {
                // Another attribute line (e.g. `#[repr(C)]`) — keep pending.
            } else if !trimmed.is_empty() {
                // A non-attribute, non-struct line broke the chain — drop.
                pending_derive = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_component_struct() {
        let text = r#"
#[derive(Component, Debug)]
pub struct Health {
    pub value: i32,
}
"#;
        let mut out = Vec::new();
        scan_text(text, &mut out);
        assert_eq!(out, vec!["Health".to_string()]);
    }

    #[test]
    fn detects_enum_with_component() {
        let text = r#"
#[derive(Component)]
enum State { A, B }
"#;
        let mut out = Vec::new();
        scan_text(text, &mut out);
        assert_eq!(out, vec!["State".to_string()]);
    }

    #[test]
    fn ignores_struct_without_component_derive() {
        let text = r#"
#[derive(Debug)]
pub struct Unrelated;
"#;
        let mut out = Vec::new();
        scan_text(text, &mut out);
        assert!(out.is_empty());
    }
}
