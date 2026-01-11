//! ✅ IntelliJ Pro: Background Indexing for symbol search
//! Scans files in background and builds symbol index for instant search

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// ✅ IntelliJ Pro: Symbol type (function, struct, class, etc.)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SymbolKind {
    Function,
    Struct,
    Enum,
    Trait,
    Impl,
    Const,
    Static,
    Module,
}

/// ✅ IntelliJ Pro: Symbol information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub file_path: String,
    pub line_number: usize,
    pub signature: Option<String>, // Full signature (e.g., "fn foo(x: i32) -> i32")
}

/// ✅ IntelliJ Pro: Symbol index (in-memory BTreeMap for fast lookup)
pub struct SymbolIndex {
    /// Symbol name -> Vec<Symbol> (multiple definitions possible)
    symbols: BTreeMap<String, Vec<Symbol>>,
    /// Last index time
    last_indexed: std::time::SystemTime,
}

impl SymbolIndex {
    pub fn new() -> Self {
        Self {
            symbols: BTreeMap::new(),
            last_indexed: std::time::SystemTime::now(),
        }
    }

    /// ✅ IntelliJ Pro: Index a single file
    pub fn index_file(&mut self, file_path: &str, content: &str) {
        // Regex patterns for Rust symbols
        let fn_regex = Regex::new(r"(?m)^\s*(?:pub\s+)?(?:async\s+)?fn\s+(\w+)").unwrap();
        let struct_regex = Regex::new(r"(?m)^\s*(?:pub\s+)?struct\s+(\w+)").unwrap();
        let enum_regex = Regex::new(r"(?m)^\s*(?:pub\s+)?enum\s+(\w+)").unwrap();
        let trait_regex = Regex::new(r"(?m)^\s*(?:pub\s+)?trait\s+(\w+)").unwrap();
        let const_regex = Regex::new(r"(?m)^\s*(?:pub\s+)?const\s+(\w+)").unwrap();

        // Index functions
        for cap in fn_regex.captures_iter(content) {
            let name = cap[1].to_string();
            let line_number = content[..cap.get(0).unwrap().start()]
                .chars()
                .filter(|&c| c == '\n')
                .count()
                + 1;

            self.add_symbol(Symbol {
                name,
                kind: SymbolKind::Function,
                file_path: file_path.to_string(),
                line_number,
                signature: Some(cap.get(0).unwrap().as_str().to_string()),
            });
        }

        // Index structs
        for cap in struct_regex.captures_iter(content) {
            let name = cap[1].to_string();
            let line_number = content[..cap.get(0).unwrap().start()]
                .chars()
                .filter(|&c| c == '\n')
                .count()
                + 1;

            self.add_symbol(Symbol {
                name,
                kind: SymbolKind::Struct,
                file_path: file_path.to_string(),
                line_number,
                signature: Some(cap.get(0).unwrap().as_str().to_string()),
            });
        }

        // Index enums
        for cap in enum_regex.captures_iter(content) {
            let name = cap[1].to_string();
            let line_number = content[..cap.get(0).unwrap().start()]
                .chars()
                .filter(|&c| c == '\n')
                .count()
                + 1;

            self.add_symbol(Symbol {
                name,
                kind: SymbolKind::Enum,
                file_path: file_path.to_string(),
                line_number,
                signature: Some(cap.get(0).unwrap().as_str().to_string()),
            });
        }

        // Index traits
        for cap in trait_regex.captures_iter(content) {
            let name = cap[1].to_string();
            let line_number = content[..cap.get(0).unwrap().start()]
                .chars()
                .filter(|&c| c == '\n')
                .count()
                + 1;

            self.add_symbol(Symbol {
                name,
                kind: SymbolKind::Trait,
                file_path: file_path.to_string(),
                line_number,
                signature: Some(cap.get(0).unwrap().as_str().to_string()),
            });
        }

        // Index constants
        for cap in const_regex.captures_iter(content) {
            let name = cap[1].to_string();
            let line_number = content[..cap.get(0).unwrap().start()]
                .chars()
                .filter(|&c| c == '\n')
                .count()
                + 1;

            self.add_symbol(Symbol {
                name,
                kind: SymbolKind::Const,
                file_path: file_path.to_string(),
                line_number,
                signature: Some(cap.get(0).unwrap().as_str().to_string()),
            });
        }

        self.last_indexed = std::time::SystemTime::now();
    }

    /// Add a symbol to the index
    fn add_symbol(&mut self, symbol: Symbol) {
        self.symbols
            .entry(symbol.name.clone())
            .or_insert_with(Vec::new)
            .push(symbol);
    }

    /// ✅ IntelliJ Pro: Search symbols by name (fuzzy match)
    pub fn search(&self, query: &str) -> Vec<Symbol> {
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        for (name, symbols) in &self.symbols {
            if name.to_lowercase().contains(&query_lower) {
                results.extend(symbols.clone());
            }
        }

        // Sort by relevance (exact matches first)
        results.sort_by(|a, b| {
            let a_exact = a.name.to_lowercase() == query_lower;
            let b_exact = b.name.to_lowercase() == query_lower;

            match (a_exact, b_exact) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });

        results
    }

    /// Get all symbols
    pub fn all_symbols(&self) -> Vec<Symbol> {
        self.symbols.values().flat_map(|v| v.clone()).collect()
    }

    /// Clear the index
    pub fn clear(&mut self) {
        self.symbols.clear();
    }

    /// Get symbol count
    pub fn symbol_count(&self) -> usize {
        self.symbols.values().map(|v| v.len()).sum()
    }
}

impl Default for SymbolIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// ✅ IntelliJ Pro: Tauri command module for background indexing
pub mod commands {
    use super::*;
    use std::fs;
    use std::path::Path;
    use std::sync::{Arc, Mutex};

    /// ✅ IntelliJ Pro: Index all Rust files in a workspace
    /// Runs in background, scans recursively for .rs files
    #[tauri::command]
    pub async fn index_workspace(
        path: String,
        state: tauri::State<'_, Arc<Mutex<SymbolIndex>>>,
    ) -> Result<usize, String> {
        let mut index = state.lock().map_err(|e| format!("Lock error: {}", e))?;

        // Clear existing index before reindexing
        index.clear();

        // Recursively scan for .rs files
        let rust_files = scan_rust_files(&path)?;

        for file_path in rust_files {
            if let Ok(content) = fs::read_to_string(&file_path) {
                index.index_file(&file_path, &content);
            }
        }

        Ok(index.symbol_count())
    }

    /// ✅ IntelliJ Pro: Search symbols by query string
    /// Returns matching symbols sorted by relevance
    #[tauri::command]
    pub async fn search_symbols(
        query: String,
        state: tauri::State<'_, Arc<Mutex<SymbolIndex>>>,
    ) -> Result<Vec<Symbol>, String> {
        let index = state.lock().map_err(|e| format!("Lock error: {}", e))?;
        Ok(index.search(&query))
    }

    /// ✅ IntelliJ Pro: Update index for a single file (incremental)
    /// Called after file edits to keep index fresh
    #[tauri::command]
    pub async fn index_file(
        path: String,
        content: String,
        state: tauri::State<'_, Arc<Mutex<SymbolIndex>>>,
    ) -> Result<(), String> {
        let mut index = state.lock().map_err(|e| format!("Lock error: {}", e))?;
        index.index_file(&path, &content);
        Ok(())
    }

    /// ✅ IntelliJ Pro: Get total symbol count (for UI status)
    #[tauri::command]
    pub async fn get_symbol_count(
        state: tauri::State<'_, Arc<Mutex<SymbolIndex>>>,
    ) -> Result<usize, String> {
        let index = state.lock().map_err(|e| format!("Lock error: {}", e))?;
        Ok(index.symbol_count())
    }

    /// Helper: Recursively scan directory for .rs files
    fn scan_rust_files(dir: &str) -> Result<Vec<String>, String> {
        let mut rust_files = Vec::new();
        scan_rust_files_recursive(Path::new(dir), &mut rust_files)?;
        Ok(rust_files)
    }

    fn scan_rust_files_recursive(dir: &Path, files: &mut Vec<String>) -> Result<(), String> {
        if !dir.is_dir() {
            return Ok(());
        }

        let entries = fs::read_dir(dir).map_err(|e| format!("Failed to read dir: {}", e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let path = entry.path();

            // Skip hidden directories and target/node_modules
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with('.') || name == "target" || name == "node_modules" {
                    continue;
                }
            }

            if path.is_dir() {
                scan_rust_files_recursive(&path, files)?;
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                if let Some(path_str) = path.to_str() {
                    files.push(path_str.to_string());
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_rust_functions() {
        let mut index = SymbolIndex::new();

        let content = r#"
            pub fn hello_world() {
                println!("Hello");
            }

            fn private_function() -> i32 {
                42
            }
        "#;

        index.index_file("test.rs", content);

        let results = index.search("hello");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "hello_world");
        assert_eq!(results[0].kind, SymbolKind::Function);
    }

    #[test]
    fn test_index_structs() {
        let mut index = SymbolIndex::new();

        let content = r#"
            pub struct MyStruct {
                field: i32,
            }

            struct PrivateStruct;
        "#;

        index.index_file("test.rs", content);

        let results = index.search("Struct");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_fuzzy_search() {
        let mut index = SymbolIndex::new();

        let content = r#"
            fn create_user() {}
            fn delete_user() {}
            fn user_login() {}
        "#;

        index.index_file("test.rs", content);

        let results = index.search("user");
        assert_eq!(results.len(), 3);
    }
}
