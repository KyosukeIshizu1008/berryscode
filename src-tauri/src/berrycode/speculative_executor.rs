//! Speculative Execution - Predictive tool execution engine
//!
//! This module implements "God Mode 2.0" - the ability to predict and execute
//! tools BEFORE the LLM finishes requesting them, dramatically reducing latency.
//!
//! ## How It Works
//!
//! 1. **Streaming Parse**: As JSON streams from the LLM, we parse it incrementally
//! 2. **Pattern Detection**: We detect keywords like "Let me search", "I'll read", "checking"
//! 3. **Predictive Spawn**: We spawn tokio tasks in the background to execute likely tools
//! 4. **Cache & Return**: When the actual tool call arrives, results are already ready!
//!
//! ## Example
//!
//! ```text
//! LLM: "Let me search for 'UserController' in the codebase..."
//! System: *detects "search" + "UserController"*
//! System: *spawns background: grep("UserController", "**/*.rs")*
//! LLM: *sends tool_call: grep(...)*
//! System: *returns cached result instantly!* ⚡
//! ```
//!
//! This reduces tool execution latency from 2-3 seconds to ~50ms!

use crate::berrycode::tools::{execute_tool, ToolCall};
use crate::berrycode::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;

/// Cache key for speculative execution results
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct CacheKey {
    tool_name: String,
    args_hash: String,
}

impl CacheKey {
    fn new(tool_name: &str, args: &Value) -> Self {
        // Create a stable hash of the arguments
        let args_str = serde_json::to_string(args).unwrap_or_default();
        let args_hash = format!("{:x}", md5::compute(&args_str));

        Self {
            tool_name: tool_name.to_string(),
            args_hash,
        }
    }
}

/// Speculative execution result - either ready or still computing
enum SpeculativeResult {
    #[allow(dead_code)]
    Ready(String),
    Computing(JoinHandle<Result<String>>),
}

/// Speculative executor - predicts and pre-executes tools
pub struct SpeculativeExecutor {
    /// Project root directory
    project_root: Arc<Path>,
    /// Cache of speculative execution results
    cache: Arc<Mutex<HashMap<CacheKey, SpeculativeResult>>>,
    /// Runtime for async execution
    runtime: Arc<tokio::runtime::Runtime>,
}

impl SpeculativeExecutor {
    /// Create a new speculative executor
    pub fn new(project_root: &Path) -> Result<Self> {
        let runtime = tokio::runtime::Runtime::new()?;

        Ok(Self {
            project_root: Arc::from(project_root),
            cache: Arc::new(Mutex::new(HashMap::new())),
            runtime: Arc::new(runtime),
        })
    }

    /// Analyze streaming text and predict likely tool calls
    pub fn analyze_stream(&self, text_chunk: &str) {
        // 🗺️ PRIORITY: Detect project structure/architecture questions
        // If user asks about design/architecture/structure, immediately spawn file_tree
        let text_lower = text_chunk.to_lowercase();
        if text_lower.contains("design")
            || text_lower.contains("設計")
            || text_lower.contains("構造")
            || text_lower.contains("architecture")
            || text_lower.contains("structure")
            || text_lower.contains("overview")
            || text_lower.contains("アーキテクチャ")
            || text_lower.contains("全体像")
        {
            self.spawn_speculative_file_tree();
        }

        // Detect search patterns
        if let Some(pattern) = Self::extract_search_pattern(text_chunk) {
            self.spawn_speculative_grep(&pattern, "**/*");
        }

        // Detect file read patterns
        if let Some(file_path) = Self::extract_file_path(text_chunk) {
            self.spawn_speculative_read(&file_path);
        }

        // Detect git diff patterns
        if text_chunk.contains("diff") || text_chunk.contains("changes") {
            self.spawn_speculative_git_diff(None);
        }
    }

    /// Extract search pattern from text
    fn extract_search_pattern(text: &str) -> Option<String> {
        // Look for patterns like:
        // - "search for 'UserController'"
        // - "grep 'function main'"
        // - "find 'class Definition'"

        let patterns = [
            (r#"(?i)search for ['"]([^'"]+)['"]"#, 1),
            (r#"(?i)grep ['"]([^'"]+)['"]"#, 1),
            (r#"(?i)find ['"]([^'"]+)['"]"#, 1),
            (r"(?i)looking for ([A-Z][a-zA-Z0-9]+)", 1), // CamelCase symbols
        ];

        for (pattern, group) in patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(caps) = re.captures(text) {
                    if let Some(matched) = caps.get(group) {
                        return Some(matched.as_str().to_string());
                    }
                }
            }
        }

        None
    }

    /// Extract file path from text
    fn extract_file_path(text: &str) -> Option<String> {
        // Look for patterns like:
        // - "read src/main.rs"
        // - "check the file src/lib.rs"
        // - "open src/tools.rs"

        let patterns = [
            r"(?i)read ([a-zA-Z0-9_/\.\-]+\.[a-z]+)",
            r"(?i)check.*file ([a-zA-Z0-9_/\.\-]+\.[a-z]+)",
            r"(?i)open ([a-zA-Z0-9_/\.\-]+\.[a-z]+)",
            r"(?i)in ([a-zA-Z0-9_/\.\-]+\.rs)", // Rust files
        ];

        for pattern in patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(caps) = re.captures(text) {
                    if let Some(matched) = caps.get(1) {
                        return Some(matched.as_str().to_string());
                    }
                }
            }
        }

        None
    }

    /// Spawn speculative grep in background
    fn spawn_speculative_grep(&self, pattern: &str, file_pattern: &str) {
        let key = CacheKey::new("grep", &serde_json::json!({
            "pattern": pattern,
            "file_pattern": file_pattern,
            "context_lines": 3
        }));

        // Check if already cached or computing
        {
            let cache = self.cache.lock().unwrap();
            if cache.contains_key(&key) {
                return; // Already in progress or done
            }
        }

        let project_root = self.project_root.clone();
        let pattern = pattern.to_string();
        let file_pattern = file_pattern.to_string();

        let handle = self.runtime.spawn(async move {
            let tool_call = ToolCall {
                id: "speculative_grep".to_string(),
                tool_type: "function".to_string(),
                function: crate::berrycode::tools::FunctionCall {
                    name: "grep".to_string(),
                    arguments: serde_json::to_string(&serde_json::json!({
                        "pattern": pattern,
                        "file_pattern": file_pattern,
                        "context_lines": 3
                    }))?,
                },
            };

            execute_tool(&tool_call, &project_root)
        });

        self.cache.lock().unwrap().insert(key, SpeculativeResult::Computing(handle));
    }

    /// Spawn speculative file read in background
    fn spawn_speculative_read(&self, file_path: &str) {
        let key = CacheKey::new("read_file", &serde_json::json!({
            "file_path": file_path
        }));

        // Check if already cached or computing
        {
            let cache = self.cache.lock().unwrap();
            if cache.contains_key(&key) {
                return;
            }
        }

        let project_root = self.project_root.clone();
        let file_path = file_path.to_string();

        let handle = self.runtime.spawn(async move {
            let tool_call = ToolCall {
                id: "speculative_read".to_string(),
                tool_type: "function".to_string(),
                function: crate::berrycode::tools::FunctionCall {
                    name: "read_file".to_string(),
                    arguments: serde_json::to_string(&serde_json::json!({
                        "file_path": file_path
                    }))?,
                },
            };

            execute_tool(&tool_call, &project_root)
        });

        self.cache.lock().unwrap().insert(key, SpeculativeResult::Computing(handle));
    }

    /// Spawn speculative git diff in background
    fn spawn_speculative_git_diff(&self, file_path: Option<&str>) {
        let key = CacheKey::new("git_diff", &serde_json::json!({
            "file_path": file_path
        }));

        // Check if already cached or computing
        {
            let cache = self.cache.lock().unwrap();
            if cache.contains_key(&key) {
                return;
            }
        }

        let project_root = self.project_root.clone();
        let file_path = file_path.map(|s| s.to_string());

        let handle = self.runtime.spawn(async move {
            let tool_call = ToolCall {
                id: "speculative_git_diff".to_string(),
                tool_type: "function".to_string(),
                function: crate::berrycode::tools::FunctionCall {
                    name: "git_diff".to_string(),
                    arguments: serde_json::to_string(&serde_json::json!({
                        "file_path": file_path
                    }))?,
                },
            };

            execute_tool(&tool_call, &project_root)
        });

        self.cache.lock().unwrap().insert(key, SpeculativeResult::Computing(handle));
    }

    /// Spawn speculative file_tree in background (for architecture/design queries)
    fn spawn_speculative_file_tree(&self) {
        let key = CacheKey::new("file_tree", &serde_json::json!({
            "max_depth": 3
        }));

        // Check if already cached or computing
        {
            let cache = self.cache.lock().unwrap();
            if cache.contains_key(&key) {
                return;
            }
        }

        let project_root = self.project_root.clone();

        let handle = self.runtime.spawn(async move {
            let tool_call = ToolCall {
                id: "speculative_file_tree".to_string(),
                tool_type: "function".to_string(),
                function: crate::berrycode::tools::FunctionCall {
                    name: "file_tree".to_string(),
                    arguments: serde_json::to_string(&serde_json::json!({
                        "max_depth": 3
                    }))?,
                },
            };

            execute_tool(&tool_call, &project_root)
        });

        self.cache.lock().unwrap().insert(key, SpeculativeResult::Computing(handle));
    }

    /// Try to get cached result for a tool call
    pub fn try_get_cached(&self, tool_call: &ToolCall) -> Option<Result<String>> {
        let args: Value = match serde_json::from_str(&tool_call.function.arguments) {
            Ok(v) => v,
            Err(_) => return None,
        };

        let key = CacheKey::new(&tool_call.function.name, &args);

        let mut cache = self.cache.lock().unwrap();

        if let Some(result) = cache.remove(&key) {
            match result {
                SpeculativeResult::Ready(output) => {
                    tracing::info!("⚡ Speculative cache HIT for {}", tool_call.function.name);
                    return Some(Ok(output));
                }
                SpeculativeResult::Computing(handle) => {
                    // Block and wait for result
                    drop(cache); // Release lock while waiting

                    match self.runtime.block_on(handle) {
                        Ok(result) => {
                            tracing::info!("⚡ Speculative execution completed for {}", tool_call.function.name);
                            return Some(result);
                        }
                        Err(e) => {
                            tracing::warn!("⚠️  Speculative execution failed: {}", e);
                            return None;
                        }
                    }
                }
            }
        }

        None
    }

    /// Clear the cache
    pub fn clear_cache(&self) {
        self.cache.lock().unwrap().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_extract_search_pattern() {
        assert_eq!(
            SpeculativeExecutor::extract_search_pattern("Let me search for 'UserController' in the codebase"),
            Some("UserController".to_string())
        );

        assert_eq!(
            SpeculativeExecutor::extract_search_pattern("I'll grep 'fn main' to find it"),
            Some("fn main".to_string())
        );

        assert_eq!(
            SpeculativeExecutor::extract_search_pattern("Looking for MyClass definition"),
            Some("MyClass".to_string())
        );
    }

    #[test]
    fn test_extract_file_path() {
        assert_eq!(
            SpeculativeExecutor::extract_file_path("Let me read src/main.rs to check"),
            Some("src/main.rs".to_string())
        );

        assert_eq!(
            SpeculativeExecutor::extract_file_path("Check the file src/tools.rs"),
            Some("src/tools.rs".to_string())
        );
    }

    #[test]
    fn test_speculative_executor_creation() {
        let temp_dir = TempDir::new().unwrap();
        let executor = SpeculativeExecutor::new(temp_dir.path());
        assert!(executor.is_ok());
    }
}
