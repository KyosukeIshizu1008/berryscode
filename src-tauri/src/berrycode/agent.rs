//! Smart agent system with planning and optimization

use std::collections::HashSet;

/// Agent planning strategy
#[derive(Debug, Clone)]
pub enum AgentStrategy {
    /// Explore project structure efficiently
    ExploreProject,
    /// Find specific code/functionality
    FindCode,
    /// Make targeted changes
    ModifyCode,
    /// Debug and fix issues
    Debug,
    /// General conversation
    General,
}

/// Smart agent with planning capabilities
pub struct SmartAgent {
    strategy: AgentStrategy,
    files_read: HashSet<String>,
    max_files_per_query: usize,
}

impl SmartAgent {
    pub fn new() -> Self {
        Self {
            strategy: AgentStrategy::General,
            files_read: HashSet::new(),
            max_files_per_query: 5,
        }
    }

    /// Detect strategy from user query
    pub fn detect_strategy(&mut self, query: &str) {
        let query_lower = query.to_lowercase();

        self.strategy = if query_lower.contains("設計") ||
            query_lower.contains("structure") ||
            query_lower.contains("architecture") ||
            query_lower.contains("overview") {
            AgentStrategy::ExploreProject
        } else if query_lower.contains("find") ||
            query_lower.contains("where") ||
            query_lower.contains("どこ") {
            AgentStrategy::FindCode
        } else if query_lower.contains("fix") ||
            query_lower.contains("debug") ||
            query_lower.contains("error") ||
            query_lower.contains("bug") {
            AgentStrategy::Debug
        } else if query_lower.contains("add") ||
            query_lower.contains("modify") ||
            query_lower.contains("change") ||
            query_lower.contains("実装") {
            AgentStrategy::ModifyCode
        } else {
            AgentStrategy::General
        };
    }

    /// Get recommended tools for current strategy
    pub fn recommend_tools(&self) -> Vec<String> {
        match self.strategy {
            AgentStrategy::ExploreProject => {
                vec![
                    "list_files".to_string(),
                    "read_file".to_string(), // Only key files: README, lib.rs, main.rs
                ]
            }
            AgentStrategy::FindCode => {
                vec![
                    "grep".to_string(),
                    "search_files".to_string(),
                ]
            }
            AgentStrategy::ModifyCode => {
                vec![
                    "read_file".to_string(),
                    "edit_file".to_string(),
                    "bash".to_string(), // For testing
                ]
            }
            AgentStrategy::Debug => {
                vec![
                    "grep".to_string(),
                    "read_file".to_string(),
                    "bash".to_string(),
                ]
            }
            AgentStrategy::General => {
                vec![]
            }
        }
    }

    /// Get key files to read for project exploration
    pub fn get_key_files(&self) -> Vec<String> {
        vec![
            "README.md".to_string(),
            "Cargo.toml".to_string(),
            "package.json".to_string(),
            "src/lib.rs".to_string(),
            "src/main.rs".to_string(),
            "pyproject.toml".to_string(),
            "setup.py".to_string(),
        ]
    }

    /// Check if we should read this file
    pub fn should_read_file(&mut self, file_path: &str) -> bool {
        // Already read?
        if self.files_read.contains(file_path) {
            return false;
        }

        // Too many files read?
        if self.files_read.len() >= self.max_files_per_query {
            return false;
        }

        // For ExploreProject strategy, only read key files
        if matches!(self.strategy, AgentStrategy::ExploreProject) {
            let key_files = self.get_key_files();
            let should_read = key_files.iter().any(|key| file_path.contains(key));

            if should_read {
                self.files_read.insert(file_path.to_string());
            }

            return should_read;
        }

        // For other strategies, allow but track
        self.files_read.insert(file_path.to_string());
        true
    }

    /// Generate planning guidance for LLM
    pub fn generate_plan_guidance(&self) -> String {
        match self.strategy {
            AgentStrategy::ExploreProject => {
                format!(
                    "\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\
                    🎯 ACTIVE STRATEGY: Project Exploration\n\
                    ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\
                    \n\
                    📋 MANDATORY STEPS (DO NOT DEVIATE - 逸脱禁止):\n\
                    \n\
                    Step 1: list_files on root directory → (1 tool call)\n\
                    Step 2: Read EXACTLY 2-3 files in priority order:\n\
                       ✓ Priority 1: README.md or similar documentation\n\
                       ✓ Priority 2: Cargo.toml/package.json (project config)\n\
                       ✓ Priority 3: src/main.rs OR src/lib.rs (entry point)\n\
                    Step 3: STOP and provide comprehensive answer based on these files\n\
                    \n\
                    ⛔ STRICTLY FORBIDDEN:\n\
                    ✗ DO NOT read: tests/*, examples/*, docs/*, detailed implementations\n\
                    ✗ DO NOT read more than 4 files total (including list_files)\n\
                    ✗ DO NOT explore every directory and subdirectory\n\
                    \n\
                    💡 Remember: Core files (README + config + entry) give 80% understanding!\n\
                    \n\
                    📊 Progress:\n\
                    Files read: {}/{} | Efficiency score: {}/100\n\
                    \n\
                    🎯 Target: 3-4 tool calls total for exploration\n\
                    ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n",
                    self.files_read.len(),
                    self.max_files_per_query,
                    self.get_efficiency_score()
                )
            }
            AgentStrategy::FindCode => {
                "\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\
                🎯 ACTIVE STRATEGY: Code Search\n\
                ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\
                \n\
                📋 MANDATORY STEPS:\n\
                \n\
                Step 1: Use grep with specific pattern → (1 tool call)\n\
                   Example: grep \"fn function_name\" or grep \"struct MyStruct\"\n\
                Step 2: Based on grep results, read ONLY the specific file(s) → (1-2 tool calls)\n\
                Step 3: Provide answer with file:line references\n\
                \n\
                ⛔ STRICTLY FORBIDDEN:\n\
                ✗ DO NOT use list_files before grep\n\
                ✗ DO NOT read files randomly hoping to find the code\n\
                ✗ DO NOT read multiple files without grep first\n\
                \n\
                💡 grep is fast and searches ALL files at once!\n\
                \n\
                🎯 Target: 2-3 tool calls total for code search\n\
                ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n".to_string()
            }
            AgentStrategy::ModifyCode => {
                "\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\
                🎯 ACTIVE STRATEGY: Code Modification\n\
                ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\
                \n\
                📋 MANDATORY STEPS:\n\
                \n\
                Step 1: Read the specific file that needs modification → (1 tool call)\n\
                Step 2: Make edits using 'edit_file' with SEARCH/REPLACE → (1 tool call)\n\
                Step 3: (Optional) Run tests: bash \"cargo test\" or bash \"npm test\" → (1 tool call)\n\
                Step 4: Provide summary of changes made\n\
                \n\
                ⛔ OPTIMIZATION TIPS:\n\
                ✓ If making multiple changes to same file, use multiple SEARCH/REPLACE blocks\n\
                ✓ If running multiple commands, combine: bash \"cargo build && cargo test\"\n\
                ✗ DO NOT read the file multiple times\n\
                ✗ DO NOT use write_file unless creating entirely new file\n\
                \n\
                🎯 Target: 2-3 tool calls total for modifications\n\
                ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n".to_string()
            }
            AgentStrategy::Debug => {
                "\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\
                🎯 ACTIVE STRATEGY: Debugging\n\
                ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\
                \n\
                📋 MANDATORY STEPS:\n\
                \n\
                Step 1: Use grep to find error messages or relevant code → (1 tool call)\n\
                   Example: grep \"error_text\" or grep \"panic\" or grep \"unwrap\"\n\
                Step 2: Read specific files that grep identified → (1-2 tool calls)\n\
                Step 3: Analyze the issue and identify root cause\n\
                Step 4: (Optional) Run tests to verify: bash \"cargo test\" → (1 tool call)\n\
                Step 5: Suggest fixes with code examples\n\
                \n\
                ⛔ DEBUGGING TIPS:\n\
                ✓ Start with grep to locate error quickly\n\
                ✓ Look for: unwrap(), panic!, expect(), TODO, FIXME comments\n\
                ✗ DO NOT read every file hoping to find the bug\n\
                ✗ DO NOT run bash multiple times - combine with &&\n\
                \n\
                🎯 Target: 3-4 tool calls total for debugging\n\
                ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n".to_string()
            }
            AgentStrategy::General => {
                "\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\
                🎯 ACTIVE STRATEGY: General Assistance\n\
                ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\
                \n\
                💡 EFFICIENCY GUIDELINES:\n\
                \n\
                - Plan your approach before using tools\n\
                - Prefer grep over reading multiple files\n\
                - Combine bash commands with &&\n\
                - Read only what you need to answer the question\n\
                - Remember: 15 tool call limit applies\n\
                \n\
                🎯 Be strategic and efficient with every tool call!\n\
                ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n".to_string()
            }
        }
    }

    /// Reset for new query
    pub fn reset(&mut self) {
        self.files_read.clear();
    }

    /// Get efficiency score (0-100)
    pub fn get_efficiency_score(&self) -> u8 {
        match self.strategy {
            AgentStrategy::ExploreProject => {
                // Good if read 3-5 files, bad if read too many
                if self.files_read.len() <= 5 {
                    100
                } else if self.files_read.len() <= 10 {
                    50
                } else {
                    20
                }
            }
            _ => {
                // General scoring
                if self.files_read.len() <= 3 {
                    100
                } else if self.files_read.len() <= 7 {
                    70
                } else {
                    40
                }
            }
        }
    }
}

impl Default for SmartAgent {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strategy_detection() {
        let mut agent = SmartAgent::new();

        agent.detect_strategy("このプロジェクトの設計教えて");
        assert!(matches!(agent.strategy, AgentStrategy::ExploreProject));

        agent.detect_strategy("Where is the main function?");
        assert!(matches!(agent.strategy, AgentStrategy::FindCode));

        agent.detect_strategy("Fix this bug");
        assert!(matches!(agent.strategy, AgentStrategy::Debug));
    }

    #[test]
    fn test_key_files() {
        let agent = SmartAgent::new();
        let key_files = agent.get_key_files();

        assert!(key_files.contains(&"README.md".to_string()));
        assert!(key_files.contains(&"src/main.rs".to_string()));
    }

    #[test]
    fn test_file_limit() {
        let mut agent = SmartAgent::new();
        agent.detect_strategy("project structure");

        // Should allow first 5 key files
        assert!(agent.should_read_file("README.md"));
        assert!(agent.should_read_file("src/lib.rs"));

        // Simulate reading max files
        for i in 0..10 {
            agent.files_read.insert(format!("file{}.rs", i));
        }

        // Should now reject
        assert!(!agent.should_read_file("another.rs"));
    }

    #[test]
    fn test_efficiency_score() {
        let mut agent = SmartAgent::new();
        agent.detect_strategy("project overview");

        agent.files_read.insert("README.md".to_string());
        agent.files_read.insert("src/lib.rs".to_string());
        agent.files_read.insert("src/main.rs".to_string());

        let score = agent.get_efficiency_score();
        assert_eq!(score, 100);
    }
}
