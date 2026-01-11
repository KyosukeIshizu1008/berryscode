//! Git Integration - Diff Display
//! 100% Rust - No JavaScript!

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeType {
    Added,
    Modified,
    Deleted,
}

impl ChangeType {
    /// Get the gutter color for this change type
    pub fn gutter_color(&self) -> &'static str {
        match self {
            ChangeType::Added => "#587c0c",      // Green
            ChangeType::Modified => "#0c7d9d",   // Blue
            ChangeType::Deleted => "#94151b",    // Red
        }
    }

    /// Get the gutter indicator character
    pub fn gutter_indicator(&self) -> &'static str {
        match self {
            ChangeType::Added => "▎",
            ChangeType::Modified => "▎",
            ChangeType::Deleted => "▼",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineChange {
    pub line_number: usize,
    pub change_type: ChangeType,
    pub old_content: Option<String>,
}

impl LineChange {
    pub fn new(line_number: usize, change_type: ChangeType) -> Self {
        Self {
            line_number,
            change_type,
            old_content: None,
        }
    }

    pub fn with_old_content(line_number: usize, change_type: ChangeType, old_content: String) -> Self {
        Self {
            line_number,
            change_type,
            old_content: Some(old_content),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    pub file_path: String,
    pub changes: Vec<LineChange>,
}

impl FileDiff {
    pub fn new(file_path: String) -> Self {
        Self {
            file_path,
            changes: Vec::new(),
        }
    }

    pub fn add_change(&mut self, change: LineChange) {
        self.changes.push(change);
    }

    pub fn get_change_at_line(&self, line: usize) -> Option<&LineChange> {
        self.changes.iter().find(|c| c.line_number == line)
    }

    pub fn has_changes(&self) -> bool {
        !self.changes.is_empty()
    }
}

pub struct GitDiffTracker {
    current_file: Option<String>,
    diff_cache: HashMap<String, FileDiff>,
}

impl GitDiffTracker {
    pub fn new() -> Self {
        Self {
            current_file: None,
            diff_cache: HashMap::new(),
        }
    }

    pub async fn load_diff(&mut self, file_path: String) -> Result<FileDiff, String> {
        // Check cache first
        if let Some(cached) = self.diff_cache.get(&file_path) {
            return Ok(cached.clone());
        }

        // Call Tauri backend to get git diff
        #[wasm_bindgen]
        extern "C" {
            #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
            async fn tauri_invoke(cmd: &str, args: JsValue) -> JsValue;
        }

        let args = serde_wasm_bindgen::to_value(&serde_json::json!({
            "filePath": file_path
        }))
        .map_err(|e| format!("Failed to serialize args: {}", e))?;

        let result = tauri_invoke("git_diff", args).await;

        let diff: FileDiff = serde_wasm_bindgen::from_value(result)
            .map_err(|e| format!("Failed to deserialize git diff: {}", e))?;

        // Cache the result
        self.diff_cache.insert(file_path.clone(), diff.clone());
        self.current_file = Some(file_path);

        Ok(diff)
    }

    pub async fn refresh_current(&mut self) -> Result<Option<FileDiff>, String> {
        if let Some(file_path) = self.current_file.clone() {
            // Remove from cache to force refresh
            self.diff_cache.remove(&file_path);
            Ok(Some(self.load_diff(file_path).await?))
        } else {
            Ok(None)
        }
    }

    pub fn get_cached_diff(&self, file_path: &str) -> Option<&FileDiff> {
        self.diff_cache.get(file_path)
    }

    pub fn clear_cache(&mut self) {
        self.diff_cache.clear();
    }

    /// Parse unified diff format
    pub fn parse_unified_diff(diff_text: &str, file_path: String) -> FileDiff {
        let mut file_diff = FileDiff::new(file_path);
        let lines: Vec<&str> = diff_text.lines().collect();

        let mut current_new_line = 0;
        let mut in_hunk = false;

        for line in lines {
            // Parse hunk header: @@ -old_start,old_count +new_start,new_count @@
            if line.starts_with("@@") {
                if let Some(new_info) = line.split('+').nth(1) {
                    if let Some(start) = new_info.split(',').next() {
                        if let Ok(line_num) = start.trim().parse::<usize>() {
                            current_new_line = line_num;
                            in_hunk = true;
                        }
                    }
                }
                continue;
            }

            if !in_hunk {
                continue;
            }

            if line.starts_with('+') && !line.starts_with("+++") {
                // Added line
                file_diff.add_change(LineChange::new(current_new_line, ChangeType::Added));
                current_new_line += 1;
            } else if line.starts_with('-') && !line.starts_with("---") {
                // Deleted line (mark the next line as deleted)
                let deleted_content = line[1..].to_string();
                file_diff.add_change(LineChange::with_old_content(
                    current_new_line,
                    ChangeType::Deleted,
                    deleted_content,
                ));
            } else if line.starts_with(' ') {
                // Check if we need to mark as modified
                // (This is simplified - real implementation would track context)
                current_new_line += 1;
            }
        }

        file_diff
    }

    /// Get all files with changes
    pub fn get_changed_files(&self) -> Vec<String> {
        self.diff_cache.keys().cloned().collect()
    }

    /// Check if a file has unsaved changes
    pub async fn has_unstaged_changes(&self, file_path: String) -> Result<bool, String> {
        #[wasm_bindgen]
        extern "C" {
            #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
            async fn tauri_invoke(cmd: &str, args: JsValue) -> JsValue;
        }

        let args = serde_wasm_bindgen::to_value(&serde_json::json!({
            "filePath": file_path
        }))
        .map_err(|e| format!("Failed to serialize args: {}", e))?;

        let result = tauri_invoke("git_has_unstaged_changes", args).await;

        serde_wasm_bindgen::from_value(result)
            .map_err(|e| format!("Failed to check unstaged changes: {}", e))
    }

    /// Stage current file
    pub async fn stage_file(&self, file_path: String) -> Result<(), String> {
        #[wasm_bindgen]
        extern "C" {
            #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
            async fn tauri_invoke(cmd: &str, args: JsValue) -> JsValue;
        }

        let args = serde_wasm_bindgen::to_value(&serde_json::json!({
            "filePath": file_path
        }))
        .map_err(|e| format!("Failed to serialize args: {}", e))?;

        let result = tauri_invoke("git_stage_file", args).await;

        serde_wasm_bindgen::from_value::<bool>(result)
            .map(|_| ())
            .map_err(|e| format!("Failed to stage file: {}", e))
    }

    /// Unstage current file
    pub async fn unstage_file(&self, file_path: String) -> Result<(), String> {
        #[wasm_bindgen]
        extern "C" {
            #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
            async fn tauri_invoke(cmd: &str, args: JsValue) -> JsValue;
        }

        let args = serde_wasm_bindgen::to_value(&serde_json::json!({
            "filePath": file_path
        }))
        .map_err(|e| format!("Failed to serialize args: {}", e))?;

        let result = tauri_invoke("git_unstage_file", args).await;

        serde_wasm_bindgen::from_value::<bool>(result)
            .map(|_| ())
            .map_err(|e| format!("Failed to unstage file: {}", e))
    }

    /// Discard changes in file
    pub async fn discard_changes(&mut self, file_path: String) -> Result<(), String> {
        #[wasm_bindgen]
        extern "C" {
            #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
            async fn tauri_invoke(cmd: &str, args: JsValue) -> JsValue;
        }

        let args = serde_wasm_bindgen::to_value(&serde_json::json!({
            "filePath": file_path
        }))
        .map_err(|e| format!("Failed to serialize args: {}", e))?;

        let result = tauri_invoke("git_discard_changes", args).await;

        serde_wasm_bindgen::from_value::<bool>(result)
            .map(|_| {
                // Clear cache for this file
                self.diff_cache.remove(&file_path);
            })
            .map_err(|e| format!("Failed to discard changes: {}", e))
    }
}

impl Default for GitDiffTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_parse_unified_diff() {
        let diff = r#"@@ -1,3 +1,4 @@
 unchanged line
-old line
+new line
+added line
 another unchanged"#;

        let file_diff = GitDiffTracker::parse_unified_diff(diff, "test.rs".to_string());

        assert!(file_diff.has_changes());
        assert_eq!(file_diff.changes.len(), 3);
    }

    #[wasm_bindgen_test]
    fn test_change_type_colors() {
        assert_eq!(ChangeType::Added.gutter_color(), "#587c0c");
        assert_eq!(ChangeType::Modified.gutter_color(), "#0c7d9d");
        assert_eq!(ChangeType::Deleted.gutter_color(), "#94151b");
    }

    #[wasm_bindgen_test]
    fn test_line_change_creation() {
        let change = LineChange::new(42, ChangeType::Modified);
        assert_eq!(change.line_number, 42);
        assert_eq!(change.change_type, ChangeType::Modified);
        assert!(change.old_content.is_none());

        let change_with_content = LineChange::with_old_content(
            42,
            ChangeType::Deleted,
            "old content".to_string(),
        );
        assert!(change_with_content.old_content.is_some());
    }

    #[wasm_bindgen_test]
    fn test_file_diff_operations() {
        let mut diff = FileDiff::new("test.rs".to_string());
        assert!(!diff.has_changes());

        diff.add_change(LineChange::new(10, ChangeType::Added));
        assert!(diff.has_changes());

        let change = diff.get_change_at_line(10);
        assert!(change.is_some());
        assert_eq!(change.unwrap().change_type, ChangeType::Added);

        let no_change = diff.get_change_at_line(20);
        assert!(no_change.is_none());
    }
}
