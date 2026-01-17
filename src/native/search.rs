//! Native search operations using regex and rayon
//! Replaces tauri_bindings search commands

use anyhow::Result;
use rayon::prelude::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub file_path: String,
    pub line_number: usize,
    pub line_content: String,
    pub match_start: usize,
    pub match_end: usize,
}

/// Search for text in files (parallel)
pub fn search_in_files(
    root: impl AsRef<Path>,
    pattern: &str,
    case_sensitive: bool,
    use_regex: bool,
) -> Result<Vec<SearchResult>> {
    let root = root.as_ref();

    // Build regex pattern
    let regex_pattern = if use_regex {
        pattern.to_string()
    } else {
        regex::escape(pattern)
    };

    let regex = if case_sensitive {
        Regex::new(&regex_pattern)?
    } else {
        Regex::new(&format!("(?i){}", regex_pattern))?
    };

    // Collect all text files
    let files: Vec<PathBuf> = WalkDir::new(root)
        .max_depth(10)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| is_text_file(e.path()))
        .map(|e| e.path().to_path_buf())
        .collect();

    // Parallel search
    let results: Vec<SearchResult> = files
        .par_iter()
        .flat_map(|path| search_in_file(path, &regex))
        .collect();

    Ok(results)
}

/// Check if file is a text file (simple heuristic)
fn is_text_file(path: &Path) -> bool {
    if let Some(ext) = path.extension() {
        let ext = ext.to_string_lossy().to_lowercase();
        matches!(
            ext.as_str(),
            "rs" | "toml"
                | "txt"
                | "md"
                | "json"
                | "yaml"
                | "yml"
                | "js"
                | "ts"
                | "jsx"
                | "tsx"
                | "py"
                | "c"
                | "cpp"
                | "h"
                | "hpp"
                | "go"
                | "java"
                | "kt"
                | "swift"
                | "rb"
                | "php"
                | "html"
                | "css"
                | "scss"
                | "xml"
                | "sh"
        )
    } else {
        false
    }
}

/// Search within a single file
fn search_in_file(path: &Path, regex: &Regex) -> Vec<SearchResult> {
    let mut results = Vec::new();

    if let Ok(content) = fs::read_to_string(path) {
        for (line_num, line) in content.lines().enumerate() {
            if let Some(mat) = regex.find(line) {
                results.push(SearchResult {
                    file_path: path.to_string_lossy().to_string(),
                    line_number: line_num + 1,
                    line_content: line.to_string(),
                    match_start: mat.start(),
                    match_end: mat.end(),
                });
            }
        }
    }

    results
}

/// Replace text in files
pub fn replace_in_files(
    root: impl AsRef<Path>,
    pattern: &str,
    replacement: &str,
    case_sensitive: bool,
) -> Result<Vec<String>> {
    let root = root.as_ref();

    let regex_pattern = regex::escape(pattern);
    let regex = if case_sensitive {
        Regex::new(&regex_pattern)?
    } else {
        Regex::new(&format!("(?i){}", regex_pattern))?
    };

    let files: Vec<PathBuf> = WalkDir::new(root)
        .max_depth(10)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| is_text_file(e.path()))
        .map(|e| e.path().to_path_buf())
        .collect();

    let mut modified_files = Vec::new();

    for path in files {
        if let Ok(content) = fs::read_to_string(&path) {
            if regex.is_match(&content) {
                let new_content = regex.replace_all(&content, replacement);
                if fs::write(&path, new_content.as_bytes()).is_ok() {
                    modified_files.push(path.to_string_lossy().to_string());
                }
            }
        }
    }

    Ok(modified_files)
}
