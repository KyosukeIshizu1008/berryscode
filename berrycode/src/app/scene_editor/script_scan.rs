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

use std::path::Path;

/// A scanned component with its name and extracted fields.
#[derive(Debug, Clone)]
pub struct ScannedComponent {
    pub name: String,
    pub fields: Vec<ScannedField>,
    /// Path to the source file where this component was found.
    pub source_path: Option<String>,
}

/// A single field extracted from a scanned component struct.
#[derive(Debug, Clone)]
pub struct ScannedField {
    pub name: String,
    pub field_type: String,
}

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

/// Return a list of [`ScannedComponent`] entries found under `root`, each
/// containing the struct name and its public fields with types. The scan
/// skips `target/` and hidden directories.
pub fn scan_components_with_fields(root: &str) -> Vec<ScannedComponent> {
    let root_path = Path::new(root);
    let mut rs_files: Vec<std::path::PathBuf> = Vec::new();
    collect_rs_files(root_path, &mut rs_files);

    let mut out: Vec<ScannedComponent> = Vec::new();
    for path in &rs_files {
        if let Ok(content) = std::fs::read_to_string(path) {
            let path_str = path.to_string_lossy().to_string();
            scan_text_with_fields(&content, &path_str, &mut out);
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out.dedup_by(|a, b| a.name == b.name);
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
                // Another attribute line (e.g. `#[repr(C)]`) -- keep pending.
            } else if !trimmed.is_empty() {
                // A non-attribute, non-struct line broke the chain -- drop.
                pending_derive = false;
            }
        }
    }
}

/// Walk `text` line-by-line; when a `#[derive(... Component ...)]` attribute is
/// seen, capture the struct name and parse any `pub` fields from the body block.
fn scan_text_with_fields(text: &str, file_path: &str, out: &mut Vec<ScannedComponent>) {
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let trimmed = lines[i].trim();
        if trimmed.starts_with("#[derive(") && trimmed.contains("Component") {
            // Found a derive(Component) -- look for the struct definition.
            i += 1;
            while i < lines.len() {
                let line = lines[i].trim();
                if let Some(rest) = line
                    .strip_prefix("pub struct ")
                    .or_else(|| line.strip_prefix("struct "))
                {
                    let name = rest
                        .split(|c: char| !c.is_alphanumeric() && c != '_')
                        .next()
                        .unwrap_or("")
                        .to_string();
                    if name.is_empty() {
                        break;
                    }

                    // Parse fields if there is a { block on this line or subsequent lines.
                    let mut fields = Vec::new();
                    if line.contains('{') {
                        i += 1;
                        while i < lines.len() {
                            let field_line = lines[i].trim();
                            if field_line.starts_with('}') {
                                break;
                            }
                            // Parse "pub field_name: FieldType," or "pub field_name: FieldType"
                            if let Some(rest) = field_line.strip_prefix("pub ") {
                                if let Some(colon_pos) = rest.find(':') {
                                    let fname = rest[..colon_pos].trim().to_string();
                                    let ftype = rest[colon_pos + 1..]
                                        .trim()
                                        .trim_end_matches(',')
                                        .trim()
                                        .to_string();
                                    if !fname.is_empty() && !ftype.is_empty() {
                                        fields.push(ScannedField {
                                            name: fname,
                                            field_type: ftype,
                                        });
                                    }
                                }
                            }
                            i += 1;
                        }
                    }

                    out.push(ScannedComponent {
                        name,
                        fields,
                        source_path: Some(file_path.to_string()),
                    });
                    break;
                }
                // Tolerate intervening attribute lines.
                if !line.starts_with("#[") && !line.is_empty() {
                    break;
                }
                i += 1;
            }
        }
        i += 1;
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

    #[test]
    fn scan_with_fields_extracts_pub_fields() {
        let text = r#"
#[derive(Component, Debug)]
pub struct PlayerStats {
    pub health: f32,
    pub speed: f32,
    pub name: String,
    secret: i32,
}
"#;
        let mut out = Vec::new();
        scan_text_with_fields(text, "", &mut out);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "PlayerStats");
        assert_eq!(out[0].fields.len(), 3);
        assert_eq!(out[0].fields[0].name, "health");
        assert_eq!(out[0].fields[0].field_type, "f32");
        assert_eq!(out[0].fields[1].name, "speed");
        assert_eq!(out[0].fields[1].field_type, "f32");
        assert_eq!(out[0].fields[2].name, "name");
        assert_eq!(out[0].fields[2].field_type, "String");
    }

    #[test]
    fn scan_with_fields_unit_struct() {
        let text = r#"
#[derive(Component)]
pub struct Marker;
"#;
        let mut out = Vec::new();
        scan_text_with_fields(text, "", &mut out);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "Marker");
        assert!(out[0].fields.is_empty());
    }

    #[test]
    fn scan_with_fields_multiple_components() {
        let text = r#"
#[derive(Component)]
pub struct Health {
    pub value: f32,
}

#[derive(Debug)]
pub struct NotAComponent;

#[derive(Component)]
pub struct Velocity {
    pub x: f32,
    pub y: f32,
}
"#;
        let mut out = Vec::new();
        scan_text_with_fields(text, "", &mut out);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].name, "Health");
        assert_eq!(out[0].fields.len(), 1);
        assert_eq!(out[1].name, "Velocity");
        assert_eq!(out[1].fields.len(), 2);
    }

    #[test]
    fn scan_with_fields_various_types() {
        let text = r#"
#[derive(Component)]
pub struct Config {
    pub enabled: bool,
    pub count: i32,
    pub label: String,
    pub ratio: f64,
}
"#;
        let mut out = Vec::new();
        scan_text_with_fields(text, "", &mut out);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].fields.len(), 4);
        assert_eq!(out[0].fields[0].field_type, "bool");
        assert_eq!(out[0].fields[1].field_type, "i32");
        assert_eq!(out[0].fields[2].field_type, "String");
        assert_eq!(out[0].fields[3].field_type, "f64");
    }

    #[test]
    fn scan_with_fields_preserves_source_path() {
        let text = r#"
#[derive(Component)]
pub struct Enemy {
    pub hp: f32,
}
"#;
        let mut out = Vec::new();
        scan_text_with_fields(text, "/project/src/enemy.rs", &mut out);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "Enemy");
        assert_eq!(out[0].source_path.as_deref(), Some("/project/src/enemy.rs"));
    }

    #[test]
    fn scan_with_fields_empty_path_gives_some_empty() {
        let text = r#"
#[derive(Component)]
pub struct Marker;
"#;
        let mut out = Vec::new();
        scan_text_with_fields(text, "", &mut out);
        assert_eq!(out[0].source_path.as_deref(), Some(""));
    }
}
