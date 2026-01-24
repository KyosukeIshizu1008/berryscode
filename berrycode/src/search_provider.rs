//! Search Provider Abstraction for Command Palette
//!
//! This module provides a plugin-based architecture for search functionality.
//! Each search provider is represented as an enum variant and can be
//! dynamically enabled/disabled.
//!
//! ## Design Philosophy
//! - **Separation of Concerns**: Each provider handles one specific search domain
//! - **Extensibility**: New providers can be added as enum variants
//! - **Performance**: Enum dispatch is zero-cost (no vtable)
//! - **Testability**: Each provider can be unit-tested independently
//!
//! ## Architecture
//! Instead of using trait objects (`Box<dyn SearchProvider>`), which don't work
//! with async traits in WASM, we use an enum-based approach. This is more
//! Rust-idiomatic and has better performance.

use crate::command_palette::{PaletteItem, ActionType};
use crate::common::fuzzy::fuzzy_match_score;
use std::sync::{Arc, Mutex, OnceLock};

/// Search Provider enum
///
/// Each variant represents a different search provider.
/// Add new providers by adding new enum variants.
#[derive(Clone)]
pub enum SearchProvider {
    FileSearch(FileSearchProvider),
    GitActions,
    EditorActions,
    Settings,
    SymbolSearch,
}

impl SearchProvider {
    /// Perform a search with the given query
    pub async fn search(&self, query: &str) -> Vec<PaletteItem> {
        match self {
            Self::FileSearch(provider) => provider.search(query).await,
            Self::GitActions => GitActionProvider::search(query).await,
            Self::EditorActions => EditorActionProvider::search(query).await,
            Self::Settings => SettingsProvider::search(query).await,
            Self::SymbolSearch => SymbolSearchProvider::search(query).await,
        }
    }

    /// Get the provider name for debugging/logging
    pub fn provider_name(&self) -> &str {
        match self {
            Self::FileSearch(_) => "File Search",
            Self::GitActions => "Git Actions",
            Self::EditorActions => "Editor Actions",
            Self::Settings => "Settings",
            Self::SymbolSearch => "Symbol Search",
        }
    }

    /// Get the priority of this provider
    pub fn priority(&self) -> u32 {
        match self {
            Self::FileSearch(_) => 10,  // High priority
            Self::EditorActions => 110,  // Medium-low
            Self::GitActions => 100,     // Medium
            Self::SymbolSearch => 200,   // Medium-high
            Self::Settings => 300,       // Low
        }
    }

    /// Get the debounce delay in milliseconds
    pub fn debounce_ms(&self) -> u64 {
        match self {
            Self::SymbolSearch => 300,  // 300ms for LSP queries
            _ => 0,                     // No debounce for others
        }
    }

    /// Minimum query length to trigger this provider
    pub fn min_query_length(&self) -> usize {
        match self {
            Self::SymbolSearch => 2,  // Require 2+ chars for symbols
            _ => 0,                   // No minimum for others
        }
    }

    /// Maximum number of results to return
    pub fn max_results(&self) -> usize {
        match self {
            Self::FileSearch(_) => 100,  // Allow more file results
            Self::SymbolSearch => 50,    // Limit symbol results
            _ => 50,                     // Default limit
        }
    }

    /// Whether this provider should be enabled
    pub fn is_enabled(&self) -> bool {
        true  // All providers enabled by default
    }
}

/// Helper function to filter and sort items by fuzzy match score
pub fn fuzzy_filter_items(items: Vec<PaletteItem>, query: &str) -> Vec<PaletteItem> {
    if query.is_empty() {
        return items;
    }

    let mut scored_items: Vec<(PaletteItem, i32)> = items
        .into_iter()
        .filter_map(|item| {
            let label_score = fuzzy_match_score(&item.label, query);
            let desc_score = item
                .description
                .as_ref()
                .map_or(0, |d| fuzzy_match_score(d, query));
            let max_score = label_score.max(desc_score);

            if max_score > 0 {
                Some((item, max_score))
            } else {
                None
            }
        })
        .collect();

    // Sort by score (highest first)
    scored_items.sort_by(|a, b| b.1.cmp(&a.1));

    // Extract items (drop scores)
    scored_items.into_iter().map(|(item, _)| item).collect()
}

// ============================================================================
// File Search Provider
// ============================================================================

/// Helper function to flatten a directory tree into a list of (path, is_dir) tuples
fn flatten_dir_entries(entries: &[crate::native::fs::DirEntry]) -> Vec<(String, bool)> {
    let mut result = Vec::new();

    for entry in entries {
        result.push((entry.path.clone(), entry.is_dir));

        if let Some(children) = &entry.children {
            result.extend(flatten_dir_entries(children));
        }
    }

    result
}

#[derive(Clone)]
pub struct FileSearchProvider {
    cache: Arc<Mutex<Option<Vec<String>>>>,
}

impl FileSearchProvider {
    pub fn new() -> Self {
        static FILE_CACHE: OnceLock<Arc<Mutex<Option<Vec<String>>>>> = OnceLock::new();
        let cache = FILE_CACHE
            .get_or_init(|| Arc::new(Mutex::new(None)))
            .clone();

        Self { cache }
    }

    async fn load_files(&self) -> anyhow::Result<Vec<String>> {
        // Check cache first
        {
            let cache_guard = self.cache.lock().unwrap();
            if let Some(cached_files) = cache_guard.as_ref() {
                #[cfg(debug_assertions)]
                tracing::info!("📦 File cache HIT ({} files)", cached_files.len());
                return Ok(cached_files.clone());
            }
        }

        // Cache miss - load from native fs module
        #[cfg(debug_assertions)]
        tracing::info!("📦 File cache MISS - loading from native::fs");

        // Use native::fs::read_dir_recursive to get all files
        let current_dir = crate::native::fs::get_current_dir()?;
        let dir_entries = crate::native::fs::read_dir_recursive(&current_dir)?;

        // Flatten directory tree and filter to get only file paths
        let files = flatten_dir_entries(&dir_entries)
            .into_iter()
            .filter(|(_, is_dir)| !is_dir)
            .map(|(path, _)| path)
            .collect::<Vec<String>>();

        // Update cache
        {
            let mut cache_guard = self.cache.lock().unwrap();
            *cache_guard = Some(files.clone());
        }

        Ok(files)
    }

    pub async fn search(&self, query: &str) -> Vec<PaletteItem> {
        let files = match self.load_files().await {
            Ok(files) => files,
            Err(e) => {
                tracing::error!("Failed to load files: {}", e);
                return vec![];
            }
        };

        let items: Vec<PaletteItem> = files
            .into_iter()
            .map(|file| PaletteItem {
                id: format!("file:{}", file),
                label: file.clone(),
                description: None,
                action_type: ActionType::File,
                icon: "file".to_string(),
                action: format!("open:{}", file),
            })
            .collect();

        fuzzy_filter_items(items, query)
    }

    /// Clear the file cache (call when file system changes)
    pub fn invalidate_cache(&self) {
        let mut cache_guard = self.cache.lock().unwrap();
        *cache_guard = None;
        #[cfg(debug_assertions)]
        tracing::info!("📦 File cache INVALIDATED");
    }
}

impl Default for FileSearchProvider {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Git Action Provider
// ============================================================================

pub struct GitActionProvider;

impl GitActionProvider {
    fn get_actions() -> Vec<PaletteItem> {
        vec![
            PaletteItem {
                id: "git:commit".to_string(),
                label: "Git: Commit".to_string(),
                description: Some("Create a new commit".to_string()),
                action_type: ActionType::GitAction,
                icon: "git-commit".to_string(),
                action: "git:commit".to_string(),
            },
            PaletteItem {
                id: "git:push".to_string(),
                label: "Git: Push".to_string(),
                description: Some("Push to remote".to_string()),
                action_type: ActionType::GitAction,
                icon: "repo-push".to_string(),
                action: "git:push".to_string(),
            },
            PaletteItem {
                id: "git:pull".to_string(),
                label: "Git: Pull".to_string(),
                description: Some("Pull from remote".to_string()),
                action_type: ActionType::GitAction,
                icon: "repo-pull".to_string(),
                action: "git:pull".to_string(),
            },
            PaletteItem {
                id: "git:status".to_string(),
                label: "Git: Show Status".to_string(),
                description: Some("View repository status".to_string()),
                action_type: ActionType::GitAction,
                icon: "git-branch".to_string(),
                action: "git:status".to_string(),
            },
            PaletteItem {
                id: "git:log".to_string(),
                label: "Git: View History".to_string(),
                description: Some("View commit history".to_string()),
                action_type: ActionType::GitAction,
                icon: "history".to_string(),
                action: "git:log".to_string(),
            },
        ]
    }

    pub async fn search(query: &str) -> Vec<PaletteItem> {
        fuzzy_filter_items(Self::get_actions(), query)
    }
}

// ============================================================================
// Editor Action Provider
// ============================================================================

pub struct EditorActionProvider;

impl EditorActionProvider {
    fn get_actions() -> Vec<PaletteItem> {
        vec![
            PaletteItem {
                id: "editor:save".to_string(),
                label: "File: Save".to_string(),
                description: Some("Save current file (Ctrl+S)".to_string()),
                action_type: ActionType::EditorAction,
                icon: "save".to_string(),
                action: "editor:save".to_string(),
            },
            PaletteItem {
                id: "editor:save_all".to_string(),
                label: "File: Save All".to_string(),
                description: Some("Save all open files".to_string()),
                action_type: ActionType::EditorAction,
                icon: "save-all".to_string(),
                action: "editor:save_all".to_string(),
            },
            PaletteItem {
                id: "editor:close".to_string(),
                label: "File: Close".to_string(),
                description: Some("Close current file".to_string()),
                action_type: ActionType::EditorAction,
                icon: "close".to_string(),
                action: "editor:close".to_string(),
            },
            PaletteItem {
                id: "editor:close_all".to_string(),
                label: "File: Close All".to_string(),
                description: Some("Close all open files".to_string()),
                action_type: ActionType::EditorAction,
                icon: "close-all".to_string(),
                action: "editor:close_all".to_string(),
            },
            PaletteItem {
                id: "editor:format".to_string(),
                label: "Format Document".to_string(),
                description: Some("Format the current file".to_string()),
                action_type: ActionType::EditorAction,
                icon: "symbol-color".to_string(),
                action: "editor:format".to_string(),
            },
            PaletteItem {
                id: "editor:find".to_string(),
                label: "Find in File".to_string(),
                description: Some("Search in current file (Ctrl+F)".to_string()),
                action_type: ActionType::EditorAction,
                icon: "search".to_string(),
                action: "editor:find".to_string(),
            },
            PaletteItem {
                id: "editor:replace".to_string(),
                label: "Find and Replace".to_string(),
                description: Some("Search and replace (Ctrl+H)".to_string()),
                action_type: ActionType::EditorAction,
                icon: "replace".to_string(),
                action: "editor:replace".to_string(),
            },
        ]
    }

    pub async fn search(query: &str) -> Vec<PaletteItem> {
        fuzzy_filter_items(Self::get_actions(), query)
    }
}

// ============================================================================
// Settings Provider
// ============================================================================

pub struct SettingsProvider;

impl SettingsProvider {
    fn get_items() -> Vec<PaletteItem> {
        vec![
            PaletteItem {
                id: "settings:open".to_string(),
                label: "Settings".to_string(),
                description: Some("Open settings panel".to_string()),
                action_type: ActionType::Settings,
                icon: "settings-gear".to_string(),
                action: "settings:open".to_string(),
            },
            PaletteItem {
                id: "settings:theme".to_string(),
                label: "Preferences: Color Theme".to_string(),
                description: Some("Change color theme".to_string()),
                action_type: ActionType::Settings,
                icon: "symbol-color".to_string(),
                action: "settings:theme".to_string(),
            },
            PaletteItem {
                id: "settings:keyboard".to_string(),
                label: "Preferences: Keyboard Shortcuts".to_string(),
                description: Some("Customize keyboard shortcuts".to_string()),
                action_type: ActionType::Settings,
                icon: "keyboard".to_string(),
                action: "settings:keyboard".to_string(),
            },
        ]
    }

    pub async fn search(query: &str) -> Vec<PaletteItem> {
        fuzzy_filter_items(Self::get_items(), query)
    }
}

// ============================================================================
// Symbol Search Provider
// ============================================================================

pub struct SymbolSearchProvider;

impl SymbolSearchProvider {
    pub async fn search(_query: &str) -> Vec<PaletteItem> {
        // TODO: Symbol search requires LSP (Language Server Protocol) or tree-sitter integration
        // to properly parse source code and extract symbols (functions, structs, traits, etc.).
        // This stub returns empty results until LSP integration is complete.
        //
        // Future implementation should:
        // 1. Query LSP server for workspace symbols matching the query
        // 2. Parse responses and convert to PaletteItem format
        // 3. Support multiple languages via their respective language servers
        // 4. Cache symbol tables for better performance
        //
        // Reference: native::search module for potential future implementation

        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzy_filter_items_empty_query() {
        let items = vec![
            PaletteItem {
                id: "1".to_string(),
                label: "Test".to_string(),
                description: None,
                action_type: ActionType::File,
                icon: "file".to_string(),
                action: "open:test".to_string(),
            },
        ];

        let result = fuzzy_filter_items(items.clone(), "");
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_fuzzy_filter_items_with_match() {
        let items = vec![
            PaletteItem {
                id: "1".to_string(),
                label: "main.rs".to_string(),
                description: None,
                action_type: ActionType::File,
                icon: "file".to_string(),
                action: "open:main.rs".to_string(),
            },
            PaletteItem {
                id: "2".to_string(),
                label: "test.rs".to_string(),
                description: None,
                action_type: ActionType::File,
                icon: "file".to_string(),
                action: "open:test.rs".to_string(),
            },
        ];

        let result = fuzzy_filter_items(items, "main");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].label, "main.rs");
    }

    #[test]
    fn test_fuzzy_filter_items_no_match() {
        let items = vec![
            PaletteItem {
                id: "1".to_string(),
                label: "main.rs".to_string(),
                description: None,
                action_type: ActionType::File,
                icon: "file".to_string(),
                action: "open:main.rs".to_string(),
            },
        ];

        let result = fuzzy_filter_items(items, "xyz");
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_fuzzy_filter_items_sorting() {
        let items = vec![
            PaletteItem {
                id: "1".to_string(),
                label: "test_file.rs".to_string(),
                description: None,
                action_type: ActionType::File,
                icon: "file".to_string(),
                action: "open:test_file.rs".to_string(),
            },
            PaletteItem {
                id: "2".to_string(),
                label: "test.rs".to_string(),
                description: None,
                action_type: ActionType::File,
                icon: "file".to_string(),
                action: "open:test.rs".to_string(),
            },
        ];

        let result = fuzzy_filter_items(items, "test");
        // "test.rs" should score higher than "test_file.rs"
        assert_eq!(result[0].label, "test.rs");
    }
}
