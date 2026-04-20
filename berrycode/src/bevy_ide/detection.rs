//! Bevy project detection
//!
//! Reads the project's Cargo.toml to determine if the current
//! workspace is a Bevy project, and extracts version and feature info.

use std::path::Path;

/// Information about a detected Bevy project.
#[derive(Debug, Clone)]
pub struct BevyProjectInfo {
    /// The Bevy version string (e.g. "0.15").
    pub version: String,
    /// Enabled Bevy features (e.g. ["dynamic_linking", "bevy_dev_tools"]).
    pub features: Vec<String>,
    /// Whether this is a Cargo workspace root.
    pub is_workspace: bool,
}

/// Detect if the given project root is a Bevy project.
///
/// Reads `Cargo.toml` and checks both `[dependencies]` and
/// `[workspace.dependencies]` for a `bevy` entry.
/// Returns `None` if the file cannot be read or bevy is not found.
pub fn detect_bevy_project(root_path: &str) -> Option<BevyProjectInfo> {
    let cargo_path = Path::new(root_path).join("Cargo.toml");
    let content = std::fs::read_to_string(&cargo_path).ok()?;

    let is_workspace = content.contains("[workspace");

    // Try workspace.dependencies first, then regular dependencies
    let bevy_line = find_bevy_dependency(&content, "[workspace.dependencies]")
        .or_else(|| find_bevy_dependency(&content, "[dependencies]"))?;

    let version = extract_version(&bevy_line)?;
    let features = extract_features(&bevy_line);

    Some(BevyProjectInfo {
        version,
        features,
        is_workspace,
    })
}

/// Simple check: returns true if bevy appears anywhere in Cargo.toml.
pub fn is_bevy_project(root_path: &str) -> bool {
    detect_bevy_project(root_path).is_some()
}

/// Search for a `bevy` dependency line within the given section of a Cargo.toml.
fn find_bevy_dependency(content: &str, section_header: &str) -> Option<String> {
    let section_start = content.find(section_header)?;
    let section_content = &content[section_start + section_header.len()..];

    // Find the end of this section (next section header starting with `[`)
    let section_end = section_content.find("\n[").unwrap_or(section_content.len());
    let section_text = &section_content[..section_end];

    for line in section_text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("bevy ") || trimmed.starts_with("bevy=") {
            return Some(trimmed.to_string());
        }
    }

    // Also handle multi-line table format:
    //   [dependencies.bevy] or [workspace.dependencies.bevy]
    let table_key = if section_header.contains("workspace") {
        "[workspace.dependencies.bevy]"
    } else {
        "[dependencies.bevy]"
    };

    if let Some(table_start) = content.find(table_key) {
        let after = &content[table_start + table_key.len()..];
        let table_end = after.find("\n[").unwrap_or(after.len());
        return Some(after[..table_end].to_string());
    }

    None
}

/// Extract the version string from a dependency value.
///
/// Handles both `bevy = "0.15"` and `bevy = { version = "0.15", ... }` forms.
fn extract_version(dep_value: &str) -> Option<String> {
    // Simple form: bevy = "0.15"
    if let Some(start) = dep_value.find('"') {
        let rest = &dep_value[start + 1..];
        if let Some(end) = rest.find('"') {
            let candidate = &rest[..end];
            // If this looks like a version (starts with digit), use it
            if candidate
                .chars()
                .next()
                .map_or(false, |c| c.is_ascii_digit())
            {
                return Some(candidate.to_string());
            }
        }
    }

    // Table form: version = "0.15"
    if let Some(ver_pos) = dep_value.find("version") {
        let after_key = &dep_value[ver_pos..];
        if let Some(eq) = after_key.find('=') {
            let after_eq = &after_key[eq + 1..];
            if let Some(q1) = after_eq.find('"') {
                let rest = &after_eq[q1 + 1..];
                if let Some(q2) = rest.find('"') {
                    return Some(rest[..q2].to_string());
                }
            }
        }
    }

    None
}

/// Extract features from a dependency value.
///
/// Handles `bevy = { version = "0.15", features = ["foo", "bar"] }`.
fn extract_features(dep_value: &str) -> Vec<String> {
    let mut features = Vec::new();

    if let Some(feat_pos) = dep_value.find("features") {
        let after_key = &dep_value[feat_pos..];
        if let Some(bracket_start) = after_key.find('[') {
            let after_bracket = &after_key[bracket_start + 1..];
            if let Some(bracket_end) = after_bracket.find(']') {
                let list = &after_bracket[..bracket_end];
                for item in list.split(',') {
                    let trimmed = item.trim().trim_matches('"').trim_matches('\'');
                    if !trimmed.is_empty() {
                        features.push(trimmed.to_string());
                    }
                }
            }
        }
    }

    features
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_version_simple() {
        assert_eq!(
            extract_version(r#"bevy = "0.15""#),
            Some("0.15".to_string())
        );
    }

    #[test]
    fn test_extract_version_table() {
        assert_eq!(
            extract_version(r#"bevy = { version = "0.15", features = ["dynamic_linking"] }"#),
            Some("0.15".to_string())
        );
    }

    #[test]
    fn test_extract_features() {
        let features = extract_features(
            r#"bevy = { version = "0.15", features = ["dynamic_linking", "bevy_dev_tools"] }"#,
        );
        assert_eq!(features, vec!["dynamic_linking", "bevy_dev_tools"]);
    }

    #[test]
    fn test_extract_features_empty() {
        let features = extract_features(r#"bevy = "0.15""#);
        assert!(features.is_empty());
    }
}
