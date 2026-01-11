//! Intent Router - Determine which tools to run proactively
//!
//! This module provides a "Flash Brain" that analyzes user input in 0.1s
//! and decides which tools should be executed proactively (before AI sees the query).
//!
//! This prevents "noise injection" - e.g., don't run semantic_search when user says
//! "show me src/main.rs" (they want a specific file, not a search).

use regex::Regex;

/// User intent classification
#[derive(Debug, Clone, PartialEq)]
pub enum Intent {
    /// Conceptual question - needs semantic understanding
    ConceptualQuestion,
    /// File read request - specific file path mentioned
    FileRead,
    /// Code search - looking for specific pattern/function
    CodeSearch,
    /// Bug fix - needs file read + error search + self-healing
    BugFix,
    /// Batch translation - translate multiple files in parallel
    BatchTranslation,
    /// Command execution - run tests, build, etc.
    Command,
    /// Git operation - commit, diff, status
    Git,
    /// Simple chat/acknowledgment - "yes", "ok", "thanks"
    ChitChat,
    /// Consultation mode - asking for suggestions/ideas (read CAPABILITIES.md only)
    Consultation,
    /// Batch edit - modify many places with same pattern (use script, not edit_file!)
    BatchEdit,
}

/// Tools that should be executed proactively
#[derive(Debug, Default)]
pub struct ProactiveTools {
    /// Run semantic_search with user query
    pub semantic_search: bool,
    /// File paths to read proactively
    pub files_to_read: Vec<String>,
    /// Grep patterns to search
    pub grep_patterns: Vec<String>,
    /// Execute file_tree
    pub file_tree: bool,
    /// Enable self-healing loop (for bug fixes)
    pub self_healing: bool,
    /// File paths to translate in batch
    pub files_to_translate: Vec<String>,
    /// Target language for translation
    pub target_language: Option<String>,
    /// Read capabilities file (for consultation mode)
    pub capabilities_file: Option<String>,
    /// Batch edit target file (for generating automation script)
    pub batch_edit_target: Option<String>,
}

/// Intent router that analyzes user input
pub struct IntentRouter {
    file_pattern: Regex,
    conceptual_keywords: Vec<&'static str>,
    search_keywords: Vec<&'static str>,
    bug_fix_keywords: Vec<&'static str>,
    translation_keywords: Vec<&'static str>,
    command_keywords: Vec<&'static str>,
    git_keywords: Vec<&'static str>,
    chitchat_patterns: Vec<&'static str>,
    consultation_keywords: Vec<&'static str>,
    batch_edit_keywords: Vec<&'static str>,
}

impl Default for IntentRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl IntentRouter {
    pub fn new() -> Self {
        Self {
            // Match file paths: src/main.rs, Cargo.toml, etc.
            file_pattern: Regex::new(r"[a-zA-Z0-9_\-./]+\.(rs|toml|json|md|txt|js|ts|py|go|java|c|cpp|h)").unwrap(),

            // Conceptual questions (need semantic search)
            conceptual_keywords: vec![
                "設計", "教えて", "仕組み", "どうなってる", "説明", "概要",
                "architecture", "design", "explain", "how does", "what is",
                "overview", "structure", "どう", "なぜ",
            ],

            // Code search keywords (need grep)
            search_keywords: vec![
                "検索", "探して", "見つけて", "where is", "find", "search for",
                "どこ", "場所",
            ],

            // Bug fix keywords (need file read + error search + self-healing)
            bug_fix_keywords: vec![
                "バグ", "bug", "fix", "修正", "直して", "repair", "エラー", "error",
                "broken", "壊れ", "治して", "fails", "失敗", "panic", "crash",
            ],

            // Translation keywords (batch translation mode)
            translation_keywords: vec![
                "翻訳", "translate", "日本語にして", "日本語に", "英語にして", "英語に",
                "中国語にして", "中国語に", "スペイン語にして", "フランス語にして",
                "to Japanese", "to English", "to Chinese", "to Spanish", "to French",
                "translate to", "翻訳して",
            ],

            // Command execution
            command_keywords: vec![
                "実行", "テスト", "ビルド", "run", "test", "build", "compile",
                "fmt", "check", "cargo", "npm", "install",
            ],

            // Git operations
            git_keywords: vec![
                "commit", "diff", "status", "コミット", "変更", "差分",
                "push", "pull", "branch",
            ],

            // Simple chat (no tools needed)
            chitchat_patterns: vec![
                "yes", "no", "ok", "thanks", "ありがとう", "はい", "いいえ",
                "わかった", "了解", "👍", "good", "great",
            ],

            // Consultation mode (suggestions/ideas/features)
            consultation_keywords: vec![
                "足した方が", "追加した方が", "機能", "提案", "アイデア", "改善",
                "suggest", "idea", "feature", "improvement", "what should", "recommend",
                "おすすめ", "何ができる", "できること", "capabilities", "足りない",
                "missing", "欲しい", "want", "need", "wish",
            ],

            // Batch edit mode (bulk replace, automation needed)
            batch_edit_keywords: vec![
                "全部", "すべて", "一括", "全て", "まとめて", "いっぺんに",
                "all", "every", "each", "bulk", "mass", "batch", "replace all",
                "一気に", "自動化", "スクリプト", "script", "automate",
                "正規表現", "regex", "sed", "awk",
            ],
        }
    }

    /// Analyze user input and determine primary intent
    pub fn determine_intent(&self, input: &str) -> Intent {
        let input_lower = input.to_lowercase();
        let input_trimmed = input.trim();

        // 1. Consultation (suggestions/ideas) - Check FIRST to prevent context explosion
        // "What features should I add?" should NOT trigger semantic_search
        for keyword in &self.consultation_keywords {
            if input_lower.contains(keyword) {
                return Intent::Consultation;
            }
        }

        // 2. Translation (check EARLY to avoid false positives from "all" in filenames like "INSTALL.md")
        // "Translate README.md" should be BatchTranslation, not BatchEdit (because "INSTALL" contains "all")
        for keyword in &self.translation_keywords {
            if input_lower.contains(keyword) {
                return Intent::BatchTranslation;
            }
        }

        // 2.5. Batch Edit (bulk replace) - Check BEFORE ChitChat but AFTER Translation
        // "Replace all X with Y" should generate script, not call edit_file 100 times
        for keyword in &self.batch_edit_keywords {
            if input_lower.contains(keyword) {
                return Intent::BatchEdit;
            }
        }

        // 3. ChitChat (acknowledgments, short responses)
        // Check both exact match and contains for flexibility
        for pattern in &self.chitchat_patterns {
            if input_trimmed.eq_ignore_ascii_case(pattern) ||
               (input.len() < 15 && input_lower.contains(pattern)) {
                return Intent::ChitChat;
            }
        }

        // 3. Bug fix (file path + bug/fix keywords)
        let has_file = self.file_pattern.is_match(input);
        let has_bug_keyword = self.bug_fix_keywords.iter().any(|k| input_lower.contains(k));

        if has_file && has_bug_keyword {
            return Intent::BugFix;
        }

        // 4. File read (specific file path mentioned, but not bug fix or translation)
        if has_file {
            return Intent::FileRead;
        }

        // 5. Bug fix without specific file (general bug fix request)
        if has_bug_keyword {
            return Intent::BugFix;
        }

        // 6. Git operations
        for keyword in &self.git_keywords {
            if input_lower.contains(keyword) {
                return Intent::Git;
            }
        }

        // 4. Command execution
        for keyword in &self.command_keywords {
            if input_lower.contains(keyword) {
                return Intent::Command;
            }
        }

        // 5. Code search (specific function/pattern lookup)
        for keyword in &self.search_keywords {
            if input_lower.contains(keyword) {
                return Intent::CodeSearch;
            }
        }

        // 6. Conceptual question (default for "explain", "how", etc.)
        for keyword in &self.conceptual_keywords {
            if input_lower.contains(keyword) {
                return Intent::ConceptualQuestion;
            }
        }

        // Default: Treat as conceptual question if nothing else matches
        Intent::ConceptualQuestion
    }

    /// Determine which tools to run proactively based on intent
    pub fn get_proactive_tools(&self, input: &str) -> ProactiveTools {
        // ⚡ INSTANT BYPASS: Check for help requests FIRST
        // This returns empty ProactiveTools, letting the caller handle static response
        // (Static response is handled in the CLI/Web layer, not here)
        if crate::berrycode::static_responses::is_help_request(input) {
            tracing::debug!("⚡ Router: Help request detected → bypassing all proactive tools");
            return ProactiveTools::default(); // Return empty tools
        }

        let intent = self.determine_intent(input);
        let mut tools = ProactiveTools::default();

        match intent {
            Intent::ConceptualQuestion => {
                // For conceptual questions, run semantic search
                tools.semantic_search = true;
                tracing::debug!("🧠 Router: ConceptualQuestion → semantic_search");
            }
            Intent::FileRead => {
                // Extract file paths and queue them for reading
                for cap in self.file_pattern.captures_iter(input) {
                    if let Some(path) = cap.get(0) {
                        tools.files_to_read.push(path.as_str().to_string());
                    }
                }
                tracing::debug!("📖 Router: FileRead → read_file({:?})", tools.files_to_read);
            }
            Intent::CodeSearch => {
                // For code search, run both grep AND semantic search
                // (they complement each other)
                tools.semantic_search = true;

                // Extract search terms (words after "find", "where is", etc.)
                let search_term = self.extract_search_term(input);
                if let Some(term) = search_term {
                    tools.grep_patterns.push(term);
                }
                tracing::debug!("🔍 Router: CodeSearch → grep + semantic_search");
            }
            Intent::BugFix => {
                // For bug fixes: read files + enable self-healing
                // 1. Extract file paths
                for cap in self.file_pattern.captures_iter(input) {
                    if let Some(path) = cap.get(0) {
                        tools.files_to_read.push(path.as_str().to_string());
                    }
                }

                // 2. Enable self-healing loop
                tools.self_healing = true;

                // NOTE: We DON'T run proactive grep for bug fixes anymore
                // because "error|ERROR|panic|..." patterns match too many results (18k+ hits)
                // and cause 413 Payload Too Large errors.
                // Instead, let the AI decide if it needs to grep for specific patterns.

                tracing::debug!("🔧 Router: BugFix → read_file + self_healing (no proactive grep)");
            }
            Intent::BatchTranslation => {
                // For batch translation: Don't read files into context!
                // Instead, let AI call translate_file in parallel (SHOTGUN MODE)
                // We can optionally find files to translate proactively

                // Extract target language from input
                let input_lower = input.to_lowercase();
                tools.target_language = if input_lower.contains("japanese") || input_lower.contains("日本語") {
                    Some("Japanese".to_string())
                } else if input_lower.contains("english") || input_lower.contains("英語") {
                    Some("English".to_string())
                } else if input_lower.contains("chinese") || input_lower.contains("中国語") {
                    Some("Chinese".to_string())
                } else if input_lower.contains("spanish") || input_lower.contains("スペイン語") {
                    Some("Spanish".to_string())
                } else if input_lower.contains("french") || input_lower.contains("フランス語") {
                    Some("French".to_string())
                } else {
                    None
                };

                // Extract file paths if mentioned, but DON'T queue them for reading!
                // Just inform AI that these files need translation
                for cap in self.file_pattern.captures_iter(input) {
                    if let Some(path) = cap.get(0) {
                        tools.files_to_translate.push(path.as_str().to_string());
                    }
                }

                tracing::debug!("🌐 Router: BatchTranslation → translate_file (parallel) to {:?}", tools.target_language);
            }
            Intent::Command | Intent::Git => {
                // Don't run any proactive tools - AI will decide what command to run
                tracing::debug!("⚙️ Router: Command/Git → no proactive tools");
            }
            Intent::ChitChat => {
                // Simple response - no tools needed
                tracing::debug!("💬 Router: ChitChat → no tools");
            }
            Intent::Consultation => {
                // Consultation mode: ONLY read CAPABILITIES.md (no code search!)
                tools.capabilities_file = Some("docs/CAPABILITIES.md".to_string());
                tracing::debug!("💡 Router: Consultation → read CAPABILITIES.md ONLY (no semantic_search)");
            }
            Intent::BatchEdit => {
                // Batch Edit mode: Extract file path and signal automation needed
                // AI should generate Python/sed script, NOT call edit_file repeatedly!
                for cap in self.file_pattern.captures_iter(input) {
                    if let Some(path) = cap.get(0) {
                        tools.batch_edit_target = Some(path.as_str().to_string());
                        break; // Only one file for batch edit
                    }
                }
                tracing::debug!("⚡ Router: BatchEdit → AUTOMATION MODE (generate script, don't use edit_file!)");
            }
        }

        tools
    }

    /// Extract search term from code search queries
    fn extract_search_term(&self, input: &str) -> Option<String> {
        let input_lower = input.to_lowercase();

        // Try to extract term after "where is", "find", etc.
        for keyword in &["where is", "find", "どこ", "検索"] {
            if let Some(pos) = input_lower.find(keyword) {
                let after = &input[pos + keyword.len()..];
                let term = after
                    .trim()
                    .split_whitespace()
                    .next()
                    .map(|s| s.trim_matches(|c: char| !c.is_alphanumeric() && c != '_'))
                    .filter(|s| !s.is_empty());

                if term.is_some() {
                    return term.map(|s| s.to_string());
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conceptual_question() {
        let router = IntentRouter::new();

        assert_eq!(
            router.determine_intent("このプロジェクトの設計を教えて"),
            Intent::ConceptualQuestion
        );

        assert_eq!(
            router.determine_intent("How does authentication work?"),
            Intent::ConceptualQuestion
        );

        let tools = router.get_proactive_tools("このプロジェクトの設計を教えて");
        assert!(tools.semantic_search);
        assert!(tools.files_to_read.is_empty());
    }

    #[test]
    fn test_file_read() {
        let router = IntentRouter::new();

        assert_eq!(
            router.determine_intent("src/main.rs を見せて"),
            Intent::FileRead
        );

        assert_eq!(
            router.determine_intent("Show me Cargo.toml"),
            Intent::FileRead
        );

        let tools = router.get_proactive_tools("src/main.rs を見せて");
        assert!(!tools.semantic_search);
        assert_eq!(tools.files_to_read, vec!["src/main.rs"]);
    }

    #[test]
    fn test_code_search() {
        let router = IntentRouter::new();

        assert_eq!(
            router.determine_intent("Where is the authenticate function?"),
            Intent::CodeSearch
        );

        let tools = router.get_proactive_tools("Find the User struct");
        assert!(tools.semantic_search); // Also run semantic for context
        assert!(!tools.grep_patterns.is_empty());
    }

    #[test]
    fn test_command() {
        let router = IntentRouter::new();

        assert_eq!(
            router.determine_intent("テストを実行して"),
            Intent::Command
        );

        assert_eq!(
            router.determine_intent("cargo build"),
            Intent::Command
        );

        let tools = router.get_proactive_tools("cargo test");
        assert!(!tools.semantic_search);
        assert!(tools.files_to_read.is_empty());
    }

    #[test]
    fn test_git() {
        let router = IntentRouter::new();

        assert_eq!(
            router.determine_intent("Show me the diff"),
            Intent::Git
        );

        assert_eq!(
            router.determine_intent("変更をコミットして"),
            Intent::Git
        );
    }

    #[test]
    fn test_chitchat() {
        let router = IntentRouter::new();

        assert_eq!(
            router.determine_intent("ok"),
            Intent::ChitChat
        );

        assert_eq!(
            router.determine_intent("ありがとう"),
            Intent::ChitChat
        );

        let tools = router.get_proactive_tools("thanks");
        assert!(!tools.semantic_search);
    }

    #[test]
    fn test_multiple_files() {
        let router = IntentRouter::new();

        let tools = router.get_proactive_tools("Read src/main.rs and Cargo.toml");
        assert_eq!(tools.files_to_read.len(), 2);
        assert!(tools.files_to_read.contains(&"src/main.rs".to_string()));
        assert!(tools.files_to_read.contains(&"Cargo.toml".to_string()));
    }

    #[test]
    fn test_bug_fix() {
        let router = IntentRouter::new();

        assert_eq!(
            router.determine_intent("src/main.rs のバグを修正して"),
            Intent::BugFix
        );

        assert_eq!(
            router.determine_intent("Fix the error in auth.rs"),
            Intent::BugFix
        );

        let tools = router.get_proactive_tools("src/main.rs のバグを修正して");
        assert!(tools.self_healing);
        assert_eq!(tools.files_to_read, vec!["src/main.rs"]);
        // NOTE: We no longer run proactive grep for bug fixes to prevent payload overflow
        assert!(tools.grep_patterns.is_empty());
    }

    #[test]
    fn test_bug_fix_without_file() {
        let router = IntentRouter::new();

        assert_eq!(
            router.determine_intent("Fix the authentication bug"),
            Intent::BugFix
        );

        let tools = router.get_proactive_tools("テストが失敗するので修正して");
        assert!(tools.self_healing);
        // NOTE: We no longer run proactive grep for bug fixes to prevent payload overflow
        assert!(tools.grep_patterns.is_empty());
    }

    #[test]
    fn test_batch_translation() {
        let router = IntentRouter::new();

        // Test intent detection
        assert_eq!(
            router.determine_intent("ドキュメントを日本語にして"),
            Intent::BatchTranslation
        );

        assert_eq!(
            router.determine_intent("Translate README.md to English"),
            Intent::BatchTranslation
        );

        assert_eq!(
            router.determine_intent("英語のファイルを翻訳して"),
            Intent::BatchTranslation
        );

        // Test tool extraction
        let tools = router.get_proactive_tools("README.md を日本語にして");
        assert_eq!(tools.target_language, Some("Japanese".to_string()));
        assert_eq!(tools.files_to_translate, vec!["README.md"]);
        assert!(tools.files_to_read.is_empty(), "Should NOT read files into context!");

        // Test multiple files
        let tools = router.get_proactive_tools("Translate docs to Japanese: README.md, INSTALL.md, API.md");
        assert_eq!(tools.target_language, Some("Japanese".to_string()));
        assert_eq!(tools.files_to_translate.len(), 3);
        assert!(tools.files_to_translate.contains(&"README.md".to_string()));
        assert!(tools.files_to_translate.contains(&"INSTALL.md".to_string()));
        assert!(tools.files_to_translate.contains(&"API.md".to_string()));

        println!("✅ Batch translation detection works!");
    }
}
