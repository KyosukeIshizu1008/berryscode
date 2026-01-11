//! Repository mapping with graph ranking algorithm
//!
//! This module implements a professional-grade repository map inspired by Aider,
//! using graph ranking to prioritize important files and definitions.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use std::fs;
use crate::berrycode::Result;
use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Serialize, Deserialize};

const MAX_MAP_TOKENS: usize = 2000;
const PERSONALIZATION_BOOST: f64 = 50.0;
const CACHE_VERSION: &str = "v1.0";

/// Repository map with graph-based ranking
#[derive(Serialize, Deserialize)]
pub struct RepoMap {
    root: PathBuf,
    file_map: HashMap<PathBuf, FileInfo>,
    // Graph where nodes are files and edges are dependencies
    dependency_graph: DiGraph<PathBuf, f64>,
    node_to_file: HashMap<PathBuf, NodeIndex>,
    // Persistent cache metadata
    #[serde(default)]
    cache_version: String,
    #[serde(default)]
    last_updated: HashMap<PathBuf, SystemTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: PathBuf,
    pub functions: Vec<FunctionInfo>,
    pub classes: Vec<ClassInfo>,
    pub imports: Vec<String>,
    // Identifiers defined in this file
    pub definitions: HashSet<String>,
    // Identifiers referenced in this file
    pub references: HashSet<String>,
    pub rank: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInfo {
    pub name: String,
    pub line_start: usize,
    pub line_end: usize,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassInfo {
    pub name: String,
    pub line_start: usize,
    pub line_end: usize,
    pub methods: Vec<FunctionInfo>,
}

impl RepoMap {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            file_map: HashMap::new(),
            dependency_graph: DiGraph::new(),
            node_to_file: HashMap::new(),
            cache_version: CACHE_VERSION.to_string(),
            last_updated: HashMap::new(),
        }
    }

    /// Build the repository map by analyzing all files with .gitignore support
    pub fn build(&mut self, mentioned_files: &[PathBuf]) -> Result<()> {
        use ignore::WalkBuilder;

        // Phase 1: Collect all files respecting .gitignore
        let mut files_to_analyze = Vec::new();

        for entry in WalkBuilder::new(&self.root)
            .hidden(false)
            .git_ignore(true)
            .build()
        {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_file() && self.is_source_file(path) {
                    files_to_analyze.push(path.to_path_buf());
                }
            }
        }

        tracing::info!("Found {} source files to analyze", files_to_analyze.len());

        // Phase 2: Analyze each file and extract definitions/references
        for file in &files_to_analyze {
            if let Ok(info) = self.analyze_file(file) {
                // Add node to graph
                let node = self.dependency_graph.add_node(file.clone());
                self.node_to_file.insert(file.clone(), node);
                self.file_map.insert(file.clone(), info);

                // Record mtime for differential updates
                if let Ok(metadata) = fs::metadata(file) {
                    if let Ok(mtime) = metadata.modified() {
                        self.last_updated.insert(file.clone(), mtime);
                    }
                }
            }
        }

        // Phase 3: Build dependency edges
        self.build_dependency_graph();

        // Phase 4: Compute PageRank with personalization
        self.compute_rankings(mentioned_files);

        tracing::info!("Repository map built with {} files", self.file_map.len());

        Ok(())
    }

    /// Check if file is a source code file we should analyze
    fn is_source_file(&self, path: &Path) -> bool {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            matches!(ext, "py" | "rs" | "js" | "ts" | "jsx" | "tsx" | "c" | "h" | "cpp" | "cc" | "cxx" | "hpp" | "go" | "java" | "rb")
        } else {
            false
        }
    }

    /// Build dependency graph based on imports/references
    fn build_dependency_graph(&mut self) {
        let files: Vec<PathBuf> = self.file_map.keys().cloned().collect();

        for from_file in &files {
            let from_node = match self.node_to_file.get(from_file) {
                Some(n) => *n,
                None => continue,
            };

            let from_info = match self.file_map.get(from_file) {
                Some(i) => i,
                None => continue,
            };

            // For each definition referenced in this file
            for referenced in &from_info.references {
                // Find which file defines this identifier
                for (to_file, to_info) in &self.file_map {
                    if to_file == from_file {
                        continue;
                    }

                    if to_info.definitions.contains(referenced) {
                        let to_node = match self.node_to_file.get(to_file) {
                            Some(n) => *n,
                            None => continue,
                        };

                        // Calculate edge weight based on identifier importance
                        let weight = self.calculate_edge_weight(referenced);

                        // Add or update edge
                        self.dependency_graph.add_edge(from_node, to_node, weight);
                    }
                }
            }
        }

        // Add small self-edges for files with no outgoing edges
        // This ensures graph connectivity for PageRank
        for node in self.dependency_graph.node_indices() {
            if self.dependency_graph.edges(node).count() == 0 {
                self.dependency_graph.add_edge(node, node, 0.1);
            }
        }
    }

    /// Calculate edge weight based on identifier characteristics
    /// Inspired by Aider's weighting strategy
    fn calculate_edge_weight(&self, identifier: &str) -> f64 {
        let mut weight = 1.0;

        // Boost snake_case and camelCase identifiers (more likely to be important)
        if identifier.contains('_') || identifier.chars().any(|c| c.is_uppercase()) {
            weight *= 1.5;
        }

        // Boost longer identifiers (more specific)
        if identifier.len() > 15 {
            weight *= 1.3;
        }

        weight
    }

    /// Compute PageRank scores for all files with personalization
    fn compute_rankings(&mut self, mentioned_files: &[PathBuf]) {
        use petgraph::algo::page_rank;

        // Build personalization vector (boost mentioned files)
        let mut personalization = HashMap::new();
        for node in self.dependency_graph.node_indices() {
            let file = &self.dependency_graph[node];

            let score = if mentioned_files.contains(file) {
                PERSONALIZATION_BOOST
            } else {
                1.0
            };

            personalization.insert(node, score);
        }

        // Compute PageRank with personalization
        let damping = 0.85;
        let max_iterations = 100;

        let ranks = page_rank(
            &self.dependency_graph,
            damping,
            max_iterations,
        );

        // Apply personalization boost
        for (node_idx, rank) in ranks.into_iter().enumerate() {
            let node = petgraph::graph::NodeIndex::new(node_idx);
            if let Some(file) = self.dependency_graph.node_weight(node) {
                let boost = personalization.get(&node).unwrap_or(&1.0);

                if let Some(info) = self.file_map.get_mut(file) {
                    info.rank = rank * boost;
                }
            }
        }

        tracing::debug!("PageRank computed for {} nodes", self.dependency_graph.node_count());
    }

    /// Analyze a single file using regex-based parsing
    /// TODO: Migrate to tree-sitter for more accurate parsing
    fn analyze_file(&self, file: &Path) -> Result<FileInfo> {
        use std::fs;

        let content = fs::read_to_string(file)?;
        let extension = file.extension().and_then(|e| e.to_str()).unwrap_or("");

        match extension {
            "py" => self.analyze_python(&content, file),
            "rs" => self.analyze_rust(&content, file),
            "js" | "ts" | "jsx" | "tsx" => self.analyze_javascript(&content, file),
            "c" | "h" => self.analyze_c(&content, file),
            "cpp" | "cc" | "cxx" | "hpp" => self.analyze_cpp(&content, file),
            _ => Ok(FileInfo {
                path: file.to_path_buf(),
                functions: Vec::new(),
                classes: Vec::new(),
                imports: Vec::new(),
                definitions: HashSet::new(),
                references: HashSet::new(),
                rank: 0.0,
            }),
        }
    }

    /// Analyze Python file using tree-sitter
    /// Analyze Python file using simple regex-based parsing
    fn analyze_python(&self, content: &str, file_path: &Path) -> Result<FileInfo> {
        let mut functions = Vec::new();
        let mut classes = Vec::new();
        let mut imports = Vec::new();
        let mut definitions = HashSet::new();
        let references = HashSet::new();

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();

            if trimmed.starts_with("def ") {
                if let Some(name) = trimmed
                    .strip_prefix("def ")
                    .and_then(|s| s.split('(').next())
                {
                    functions.push(FunctionInfo {
                        name: name.to_string(),
                        line_start: line_num + 1,
                        line_end: line_num + 1,
                        signature: line.to_string(),
                    });
                    definitions.insert(name.to_string());
                }
            } else if trimmed.starts_with("class ") {
                if let Some(name) = trimmed
                    .strip_prefix("class ")
                    .and_then(|s| s.split_whitespace().next())
                    .map(|s| s.trim_end_matches(':'))
                {
                    classes.push(ClassInfo {
                        name: name.to_string(),
                        line_start: line_num + 1,
                        line_end: line_num + 1,
                        methods: Vec::new(),
                    });
                    definitions.insert(name.to_string());
                }
            } else if trimmed.starts_with("import ") || trimmed.starts_with("from ") {
                imports.push(trimmed.to_string());
            }
        }

        Ok(FileInfo {
            path: file_path.to_path_buf(),
            functions,
            classes,
            imports,
            definitions,
            references,
            rank: 0.0,
        })
    }

    /// Analyze Rust file using simple regex-based parsing
    fn analyze_rust(&self, content: &str, file_path: &Path) -> Result<FileInfo> {
        let mut functions = Vec::new();
        let mut classes = Vec::new();
        let mut imports = Vec::new();
        let mut definitions = HashSet::new();
        let references = HashSet::new();

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();

            if trimmed.starts_with("fn ") || trimmed.contains(" fn ") {
                if let Some(name) = trimmed
                    .split_whitespace()
                    .skip_while(|&s| s != "fn")
                    .nth(1)
                    .and_then(|s| s.split('(').next())
                    .or_else(|| trimmed.split("fn ").nth(1).and_then(|s| s.split('(').next()))
                {
                    functions.push(FunctionInfo {
                        name: name.to_string(),
                        line_start: line_num + 1,
                        line_end: line_num + 1,
                        signature: line.to_string(),
                    });
                    definitions.insert(name.to_string());
                }
            } else if trimmed.starts_with("struct ") || trimmed.starts_with("enum ") || trimmed.starts_with("trait ") {
                if let Some(name) = trimmed
                    .split_whitespace()
                    .nth(1)
                    .and_then(|s| s.split(&['<', '{', '(', ';'][..]).next())
                {
                    classes.push(ClassInfo {
                        name: name.to_string(),
                        line_start: line_num + 1,
                        line_end: line_num + 1,
                        methods: Vec::new(),
                    });
                    definitions.insert(name.to_string());
                }
            } else if trimmed.starts_with("use ") {
                imports.push(trimmed.to_string());
            }
        }

        Ok(FileInfo {
            path: file_path.to_path_buf(),
            functions,
            classes,
            imports,
            definitions,
            references,
            rank: 0.0,
        })
    }

    /// Analyze JavaScript/TypeScript file using simple regex-based parsing
    fn analyze_javascript(&self, content: &str, file_path: &Path) -> Result<FileInfo> {
        let mut functions = Vec::new();
        let mut classes = Vec::new();
        let mut imports = Vec::new();
        let mut definitions = HashSet::new();
        let references = HashSet::new();

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();

            if trimmed.starts_with("function ") || trimmed.contains("function ") {
                if let Some(name) = trimmed
                    .split("function ")
                    .nth(1)
                    .and_then(|s| s.split('(').next())
                {
                    if !name.is_empty() {
                        functions.push(FunctionInfo {
                            name: name.to_string(),
                            line_start: line_num + 1,
                            line_end: line_num + 1,
                            signature: line.to_string(),
                        });
                        definitions.insert(name.to_string());
                    }
                }
            } else if trimmed.starts_with("class ") {
                if let Some(name) = trimmed
                    .strip_prefix("class ")
                    .and_then(|s| s.split_whitespace().next())
                    .map(|s| s.trim_end_matches('{'))
                {
                    classes.push(ClassInfo {
                        name: name.to_string(),
                        line_start: line_num + 1,
                        line_end: line_num + 1,
                        methods: Vec::new(),
                    });
                    definitions.insert(name.to_string());
                }
            } else if trimmed.starts_with("import ") || trimmed.starts_with("export ") {
                imports.push(trimmed.to_string());
            }
        }

        Ok(FileInfo {
            path: file_path.to_path_buf(),
            functions,
            classes,
            imports,
            definitions,
            references,
            rank: 0.0,
        })
    }

    /// Analyze C file using simple regex-based parsing
    fn analyze_c(&self, content: &str, file_path: &Path) -> Result<FileInfo> {
        let mut functions = Vec::new();
        let mut imports = Vec::new();
        let mut definitions = HashSet::new();
        let references = HashSet::new();

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();

            if trimmed.starts_with("#include") {
                imports.push(trimmed.to_string());
            }

            // Simple C function detection
            if line.contains('(') && line.contains(')') && !trimmed.starts_with("//") && !trimmed.starts_with("/*") {
                if let Some(potential_func) = line.split('(').next() {
                    if let Some(name) = potential_func.split_whitespace().last() {
                        if name.chars().all(|c| c.is_alphanumeric() || c == '_') && name.len() > 0 {
                            functions.push(FunctionInfo {
                                name: name.to_string(),
                                line_start: line_num + 1,
                                line_end: line_num + 1,
                                signature: line.to_string(),
                            });
                            definitions.insert(name.to_string());
                        }
                    }
                }
            }
        }

        Ok(FileInfo {
            path: file_path.to_path_buf(),
            functions,
            classes: Vec::new(),
            imports,
            definitions,
            references,
            rank: 0.0,
        })
    }

    /// Analyze C++ file using simple regex-based parsing
    fn analyze_cpp(&self, content: &str, file_path: &Path) -> Result<FileInfo> {
        // For now, use the same logic as C with additional class detection
        let mut result = self.analyze_c(content, file_path)?;

        // Add class detection for C++
        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("class ") {
                if let Some(name) = trimmed
                    .strip_prefix("class ")
                    .and_then(|s| s.split_whitespace().next())
                    .map(|s| s.trim_end_matches(&['{', ':', ';'][..]))
                {
                    result.classes.push(ClassInfo {
                        name: name.to_string(),
                        line_start: line_num + 1,
                        line_end: line_num + 1,
                        methods: Vec::new(),
                    });
                    result.definitions.insert(name.to_string());
                }
            }
        }

        Ok(result)
    }

    pub fn get_map_string(&self, max_tokens: usize) -> String {
        // Sort files by rank (descending)
        let mut ranked_files: Vec<_> = self.file_map.values().collect();
        ranked_files.sort_by(|a, b| b.rank.partial_cmp(&a.rank).unwrap());

        let mut result = String::from("# Repository Map (ranked by importance)\n\n");
        let mut current_tokens = 0;
        let max_tokens = if max_tokens > 0 { max_tokens } else { MAX_MAP_TOKENS };

        let total_files = ranked_files.len();
        for (idx, info) in ranked_files.iter().enumerate() {
            // Estimate tokens (rough: 4 chars = 1 token)
            let file_header = format!("## {} (rank: {:.3})\n", info.path.display(), info.rank);
            let estimated_tokens = file_header.len() / 4;

            if current_tokens + estimated_tokens > max_tokens {
                result.push_str(&format!("\n... ({} more files omitted due to token limit)\n",
                    total_files - idx));
                break;
            }

            result.push_str(&file_header);
            current_tokens += estimated_tokens;

            // Add definitions
            for class in &info.classes {
                let line = format!("  class {}\n", class.name);
                let tokens = line.len() / 4;
                if current_tokens + tokens > max_tokens { break; }
                result.push_str(&line);
                current_tokens += tokens;

                for method in &class.methods {
                    let line = format!("    fn {}()\n", method.name);
                    let tokens = line.len() / 4;
                    if current_tokens + tokens > max_tokens { break; }
                    result.push_str(&line);
                    current_tokens += tokens;
                }
            }

            for func in &info.functions {
                let line = format!("  fn {}()\n", func.name);
                let tokens = line.len() / 4;
                if current_tokens + tokens > max_tokens { break; }
                result.push_str(&line);
                current_tokens += tokens;
            }

            result.push('\n');
        }

        tracing::info!("Generated repo map with ~{} tokens", current_tokens);

        result
    }

    /// Get relevant context for a query with mentioned files
    pub fn get_context(&self, _mentioned_files: &[PathBuf], max_tokens: usize) -> String {
        // TODO: Use mentioned_files to boost relevance of specific files
        self.get_map_string(max_tokens)
    }

    // ========================================================================
    // 🚀 Persistent Cache (永続化キャッシュ)
    // ========================================================================

    /// Update only files that have been modified since last cache
    ///
    /// 🎯 DIFFERENTIAL UPDATE:
    /// - Scan all files in project
    /// - Compare mtime with cached mtime
    /// - Re-analyze only changed/new files
    /// - Rebuild graph edges
    /// - Recompute PageRank
    fn update_changed_files(&mut self) -> Result<()> {
        use ignore::WalkBuilder;

        let mut current_files = Vec::new();
        let mut changed_files = Vec::new();

        // Scan all source files
        for entry in WalkBuilder::new(&self.root)
            .hidden(false)
            .git_ignore(true)
            .build()
        {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_file() && self.is_source_file(path) {
                    let path_buf = path.to_path_buf();
                    current_files.push(path_buf.clone());

                    // Check if file is new or modified
                    if let Ok(metadata) = fs::metadata(path) {
                        if let Ok(mtime) = metadata.modified() {
                            let needs_update = self.last_updated
                                .get(&path_buf)
                                .map(|&last| mtime > last)
                                .unwrap_or(true); // New file

                            if needs_update {
                                changed_files.push(path_buf.clone());
                                self.last_updated.insert(path_buf, mtime);
                            }
                        }
                    }
                }
            }
        }

        if changed_files.is_empty() {
            tracing::debug!("✅ No files changed - cache is up to date");
            return Ok(());
        }

        tracing::info!("📝 Updating {} changed files (out of {} total)",
            changed_files.len(), current_files.len());

        // Re-analyze changed files
        for file in &changed_files {
            if let Ok(info) = self.analyze_file(file) {
                // Update or add node
                if !self.node_to_file.contains_key(file) {
                    let node = self.dependency_graph.add_node(file.clone());
                    self.node_to_file.insert(file.clone(), node);
                }
                self.file_map.insert(file.clone(), info);
            }
        }

        // Rebuild dependency graph (edges may have changed)
        self.build_dependency_graph();

        // NOTE: Rankings will be recomputed with mentioned files in load_or_build()

        Ok(())
    }

    /// Save RepoMap to binary cache file (.berrycode/repomap.bin)
    pub fn save_to_cache(&self) -> Result<()> {
        let cache_dir = self.root.join(".berrycode");
        fs::create_dir_all(&cache_dir)?;

        let cache_path = cache_dir.join("repomap.bin");
        let encoded = bincode::serialize(self)
            .map_err(|e| anyhow::anyhow!("Failed to serialize RepoMap: {}", e))?;

        fs::write(&cache_path, encoded)?;
        tracing::info!("✅ RepoMap saved to cache: {:?}", cache_path);

        Ok(())
    }

    /// Load RepoMap from binary cache file
    pub fn load_from_cache(root: PathBuf) -> Result<Self> {
        let cache_path = root.join(".berrycode/repomap.bin");

        if !cache_path.exists() {
            return Err(anyhow::anyhow!("Cache file does not exist"));
        }

        let data = fs::read(&cache_path)?;
        let mut repomap: RepoMap = bincode::deserialize(&data)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize RepoMap: {}", e))?;

        // Verify cache version
        if repomap.cache_version != CACHE_VERSION {
            tracing::warn!("Cache version mismatch. Expected {}, got {}", CACHE_VERSION, repomap.cache_version);
            return Err(anyhow::anyhow!("Cache version mismatch"));
        }

        // Update root path (in case project was moved)
        repomap.root = root;

        tracing::info!("✅ RepoMap loaded from cache: {} files", repomap.file_map.len());

        Ok(repomap)
    }

    /// Check if any files have been modified since last cache
    pub fn needs_rebuild(&self) -> Result<bool> {
        use ignore::WalkBuilder;

        for entry in WalkBuilder::new(&self.root)
            .hidden(false)
            .git_ignore(true)
            .build()
        {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_file() && self.is_source_file(path) {
                    if let Ok(metadata) = fs::metadata(path) {
                        if let Ok(modified) = metadata.modified() {
                            if let Some(last_updated) = self.last_updated.get(path) {
                                if modified > *last_updated {
                                    tracing::debug!("File modified: {:?}", path);
                                    return Ok(true);
                                }
                            } else {
                                // New file not in cache
                                return Ok(true);
                            }
                        }
                    }
                }
            }
        }

        Ok(false)
    }

    /// Smart load: use cache if valid, otherwise rebuild
    pub fn load_or_build(root: PathBuf, mentioned_files: &[PathBuf]) -> Result<Self> {
        let start = std::time::Instant::now();

        // Try to load from cache
        match Self::load_from_cache(root.clone()) {
            Ok(mut repomap) => {
                // Check if rebuild is needed
                match repomap.needs_rebuild() {
                    Ok(false) => {
                        // Cache is still valid, just recompute rankings
                        repomap.compute_rankings(mentioned_files);
                        tracing::info!("🚀 RepoMap loaded from cache in {:?}", start.elapsed());
                        return Ok(repomap);
                    }
                    Ok(true) => {
                        tracing::info!("🔄 Cache outdated, rebuilding...");
                        // Fall through to rebuild
                    }
                    Err(e) => {
                        tracing::warn!("Failed to check cache validity: {}", e);
                        // Fall through to rebuild
                    }
                }
            }
            Err(e) => {
                tracing::info!("No cache found ({}), building from scratch...", e);
            }
        }

        // Build from scratch
        let mut repomap = Self::new(root);
        repomap.build(mentioned_files)?;

        // Update last_updated timestamps
        use ignore::WalkBuilder;
        for entry in WalkBuilder::new(&repomap.root)
            .hidden(false)
            .git_ignore(true)
            .build()
        {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_file() && repomap.is_source_file(path) {
                    if let Ok(metadata) = fs::metadata(path) {
                        if let Ok(modified) = metadata.modified() {
                            repomap.last_updated.insert(path.to_path_buf(), modified);
                        }
                    }
                }
            }
        }

        // Save to cache for next time
        if let Err(e) = repomap.save_to_cache() {
            tracing::warn!("Failed to save cache: {}", e);
        }

        tracing::info!("🔨 RepoMap built from scratch in {:?}", start.elapsed());
        Ok(repomap)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_weight_calculation() {
        let repo = RepoMap::new(PathBuf::from("."));

        // Simple identifier
        let w1 = repo.calculate_edge_weight("foo");
        assert_eq!(w1, 1.0);

        // snake_case identifier
        let w2 = repo.calculate_edge_weight("my_function");
        assert!(w2 > 1.0);

        // Long identifier
        let w3 = repo.calculate_edge_weight("very_long_identifier_name");
        assert!(w3 > w2);
    }

    #[test]
    fn test_is_source_file() {
        let repo = RepoMap::new(PathBuf::from("."));

        assert!(repo.is_source_file(Path::new("test.py")));
        assert!(repo.is_source_file(Path::new("test.rs")));
        assert!(repo.is_source_file(Path::new("test.js")));
        assert!(!repo.is_source_file(Path::new("test.txt")));
        assert!(!repo.is_source_file(Path::new("README.md")));
    }
}
