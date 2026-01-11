//! Search commands for project-wide search functionality
//! Integrates with grep/ripgrep for fast searching

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;
use tauri::State;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResult {
    pub path: String,
    pub line_number: usize,
    pub column: usize,
    pub line_text: String,
    pub match_start: usize,
    pub match_end: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchOptions {
    pub case_sensitive: bool,
    pub use_regex: bool,
    pub whole_word: bool,
    pub include_pattern: Option<String>,
    pub exclude_pattern: Option<String>,
    pub max_results: Option<usize>,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            case_sensitive: false,
            use_regex: false,
            whole_word: false,
            include_pattern: None,
            exclude_pattern: None,
            max_results: Some(1000),
        }
    }
}

/// Search for text in files using ripgrep
#[tauri::command]
pub async fn search_in_files(
    query: String,
    root_path: String,
    options: Option<SearchOptions>,
) -> Result<Vec<SearchResult>, String> {
    let opts = options.unwrap_or_default();
    let root = PathBuf::from(&root_path);

    if !root.exists() {
        return Err(format!("Root path does not exist: {}", root_path));
    }

    if query.is_empty() {
        return Ok(vec![]);
    }

    // Try to use ripgrep if available, fall back to simple grep
    match search_with_ripgrep(&query, &root, &opts) {
        Ok(results) => Ok(results),
        Err(e) => {
            eprintln!("Ripgrep failed: {}, falling back to simple search", e);
            search_with_simple_grep(&query, &root, &opts)
        }
    }
}

/// Search using ripgrep (preferred)
fn search_with_ripgrep(
    query: &str,
    root: &PathBuf,
    opts: &SearchOptions,
) -> Result<Vec<SearchResult>, String> {
    let mut cmd = Command::new("rg");

    // Basic options
    cmd.arg("--json") // JSON output for parsing
        .arg("--line-number")
        .arg("--column");

    // Case sensitivity
    if !opts.case_sensitive {
        cmd.arg("--ignore-case");
    }

    // Regex mode
    if !opts.use_regex {
        cmd.arg("--fixed-strings");
    }

    // Whole word
    if opts.whole_word {
        cmd.arg("--word-regexp");
    }

    // Include/exclude patterns
    if let Some(ref include) = opts.include_pattern {
        cmd.arg("--glob").arg(include);
    }

    if let Some(ref exclude) = opts.exclude_pattern {
        cmd.arg("--glob").arg(format!("!{}", exclude));
    }

    // Max results (ripgrep uses --max-count per file)
    if let Some(max) = opts.max_results {
        cmd.arg("--max-count").arg(max.to_string());
    }

    // Query and path
    cmd.arg(query).arg(root);

    // Execute
    let output = cmd
        .output()
        .map_err(|e| format!("Failed to execute ripgrep: {}", e))?;

    if !output.status.success() {
        // Exit code 1 means no matches, which is OK
        if output.status.code() == Some(1) {
            return Ok(vec![]);
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Ripgrep failed: {}", stderr));
    }

    // Parse JSON output
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_ripgrep_json(&stdout)
}

/// Parse ripgrep's JSON output
fn parse_ripgrep_json(output: &str) -> Result<Vec<SearchResult>, String> {
    let mut results = Vec::new();

    for line in output.lines() {
        if line.trim().is_empty() {
            continue;
        }

        // Parse each line as JSON
        let json: serde_json::Value =
            serde_json::from_str(line).map_err(|e| format!("Failed to parse JSON: {}", e))?;

        // Look for "match" type entries
        if json["type"].as_str() == Some("match") {
            if let Some(data) = json["data"].as_object() {
                let path = data["path"]["text"].as_str().unwrap_or("").to_string();
                let line_number = data["line_number"].as_u64().unwrap_or(0) as usize;

                if let Some(submatches) = data["submatches"].as_array() {
                    for submatch in submatches {
                        let start = submatch["start"].as_u64().unwrap_or(0) as usize;
                        let end = submatch["end"].as_u64().unwrap_or(0) as usize;
                        let line_text = data["lines"]["text"]
                            .as_str()
                            .unwrap_or("")
                            .trim_end()
                            .to_string();

                        results.push(SearchResult {
                            path: path.clone(),
                            line_number,
                            column: start + 1, // 1-indexed
                            line_text,
                            match_start: start,
                            match_end: end,
                        });
                    }
                }
            }
        }
    }

    Ok(results)
}

/// Fallback: Simple recursive grep using Rust
fn search_with_simple_grep(
    query: &str,
    root: &PathBuf,
    opts: &SearchOptions,
) -> Result<Vec<SearchResult>, String> {
    use std::fs;
    use std::io::{BufRead, BufReader};

    let mut results = Vec::new();
    let max_results = opts.max_results.unwrap_or(1000);

    let query_lower = if opts.case_sensitive {
        query.to_string()
    } else {
        query.to_lowercase()
    };

    fn search_dir(
        dir: &PathBuf,
        query: &str,
        opts: &SearchOptions,
        results: &mut Vec<SearchResult>,
        max_results: usize,
    ) -> Result<(), String> {
        if results.len() >= max_results {
            return Ok(());
        }

        let entries = fs::read_dir(dir).map_err(|e| format!("Failed to read directory: {}", e))?;

        for entry in entries.flatten() {
            if results.len() >= max_results {
                break;
            }

            let path = entry.path();

            // Skip hidden files
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with('.') {
                    continue;
                }
            }

            if path.is_dir() {
                let _ = search_dir(&path, query, opts, results, max_results);
            } else if path.is_file() {
                // Check file extension if include pattern specified
                if let Some(ref include) = opts.include_pattern {
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        if !include.contains(ext) {
                            continue;
                        }
                    }
                }

                let _ = search_file(&path, query, opts, results, max_results);
            }
        }

        Ok(())
    }

    fn search_file(
        path: &PathBuf,
        query: &str,
        opts: &SearchOptions,
        results: &mut Vec<SearchResult>,
        max_results: usize,
    ) -> Result<(), String> {
        let file = fs::File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;

        let reader = BufReader::new(file);
        let path_str = path.to_string_lossy().to_string();

        for (line_num, line_result) in reader.lines().enumerate() {
            if results.len() >= max_results {
                break;
            }

            if let Ok(line) = line_result {
                let search_line = if opts.case_sensitive {
                    line.clone()
                } else {
                    line.to_lowercase()
                };

                if let Some(pos) = search_line.find(query) {
                    results.push(SearchResult {
                        path: path_str.clone(),
                        line_number: line_num + 1,
                        column: pos + 1,
                        line_text: line.clone(),
                        match_start: pos,
                        match_end: pos + query.len(),
                    });
                }
            }
        }

        Ok(())
    }

    search_dir(root, &query_lower, opts, &mut results, max_results)?;
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_search_in_files_empty_query() {
        let temp_dir = TempDir::new().unwrap();
        let result = search_in_files(
            "".to_string(),
            temp_dir.path().to_string_lossy().to_string(),
            None,
        )
        .await
        .unwrap();

        assert_eq!(result.len(), 0);
    }

    #[tokio::test]
    async fn test_search_in_files_nonexistent_path() {
        let result =
            search_in_files("test".to_string(), "/nonexistent/path".to_string(), None).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_simple_grep_basic_search() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "Hello World\nTest Line\nAnother Test").unwrap();

        let opts = SearchOptions::default();
        let results =
            search_with_simple_grep("test", &temp_dir.path().to_path_buf(), &opts).unwrap();

        assert!(results.len() >= 2); // Should find "Test" in lines 2 and 3
    }

    #[tokio::test]
    async fn test_simple_grep_case_sensitive() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "Hello World\ntest line\nTest Line").unwrap();

        let mut opts = SearchOptions::default();
        opts.case_sensitive = true;

        let results =
            search_with_simple_grep("Test", &temp_dir.path().to_path_buf(), &opts).unwrap();

        assert_eq!(results.len(), 1); // Should only find "Test" (capital T)
    }

    #[tokio::test]
    async fn test_search_result_structure() {
        let result = SearchResult {
            path: "/path/to/file.rs".to_string(),
            line_number: 42,
            column: 10,
            line_text: "let x = test();".to_string(),
            match_start: 8,
            match_end: 12,
        };

        assert_eq!(result.path, "/path/to/file.rs");
        assert_eq!(result.line_number, 42);
        assert_eq!(result.match_start, 8);
    }
}
