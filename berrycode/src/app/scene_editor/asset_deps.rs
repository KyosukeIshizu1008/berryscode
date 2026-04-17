//! Asset dependency tracking.
//!
//! Scans all `.bscene` and `.bprefab` files in the project to find which assets
//! (textures, models, audio) are referenced. Provides a reverse lookup:
//! "which scenes use this asset?"

use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct AssetDependencies {
    /// asset_path -> list of scene/prefab files that reference it
    pub reverse_index: HashMap<String, Vec<String>>,
}

impl AssetDependencies {
    /// Scan all `.bscene` and `.bprefab` files under `root` and build the reverse index.
    pub fn scan(root: &str) -> Self {
        let mut deps = Self::default();
        let mut scene_files = Vec::new();
        collect_scene_files(std::path::Path::new(root), &mut scene_files);

        for scene_path in &scene_files {
            if let Ok(content) = std::fs::read_to_string(scene_path) {
                let paths = extract_asset_paths(&content);
                for asset_path in paths {
                    deps.reverse_index
                        .entry(asset_path)
                        .or_default()
                        .push(scene_path.clone());
                }
            }
        }
        deps
    }

    /// Get scenes that use a given asset path.
    pub fn used_by(&self, asset_path: &str) -> &[String] {
        self.reverse_index
            .get(asset_path)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Check if an asset is unused.
    pub fn is_unused(&self, asset_path: &str) -> bool {
        self.used_by(asset_path).is_empty()
    }
}

fn collect_scene_files(dir: &std::path::Path, out: &mut Vec<String>) {
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
            collect_scene_files(&path, out);
        } else {
            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
            if ext == "bscene" || ext == "bprefab" {
                out.push(path.to_string_lossy().to_string());
            }
        }
    }
}

/// Extract asset path strings from RON content.
/// Looks for common patterns: `path: "..."`, `texture_path: Some("...")`,
/// `normal_map_path: Some("...")`.
fn extract_asset_paths(content: &str) -> Vec<String> {
    let mut paths = Vec::new();
    // Simple scan for quoted paths after known field names.
    for line in content.lines() {
        let trimmed = line.trim();
        for prefix in &["path:", "texture_path:", "normal_map_path:"] {
            if let Some(rest) = trimmed.strip_prefix(prefix) {
                if let Some(start) = rest.find('"') {
                    if let Some(end) = rest[start + 1..].find('"') {
                        let path = &rest[start + 1..start + 1 + end];
                        if !path.is_empty() {
                            paths.push(path.to_string());
                        }
                    }
                }
            }
        }
    }
    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_simple_path() {
        let content = r#"path: "assets/model.glb""#;
        let paths = extract_asset_paths(content);
        assert_eq!(paths, vec!["assets/model.glb"]);
    }

    #[test]
    fn extract_texture_path() {
        let content = r#"texture_path: Some("textures/brick.png")"#;
        let paths = extract_asset_paths(content);
        assert_eq!(paths, vec!["textures/brick.png"]);
    }

    #[test]
    fn extract_empty_returns_nothing() {
        let content = r#"path: """#;
        let paths = extract_asset_paths(content);
        assert!(paths.is_empty());
    }
}
