//! Prompt generation for different edit formats

use crate::berrycode::Result;
use std::path::Path;
use std::sync::OnceLock;
use regex::Regex;

/// 静的な正規表現（一度だけコンパイル）
static FILENAME_REGEX: OnceLock<Regex> = OnceLock::new();

fn get_filename_regex() -> &'static Regex {
    FILENAME_REGEX.get_or_init(|| {
        Regex::new(r"(?:`([^`]+)`|([a-zA-Z0-9_\-./]+\.[a-zA-Z0-9]+)|([a-zA-Z0-9_\-./]+/[a-zA-Z0-9_\-./]+))")
            .expect("Invalid filename regex pattern")
    })
}

/// 共通の検証ルール（全プロンプトで使用）
const VERIFICATION_RULES: &str = r#"
5. **VERIFICATION IS MANDATORY** (検証必須):
   🚨 CRITICAL: After ANY file operation (mv, cp, rm, write_file, edit_file), you MUST verify:
   - After moving/copying files: Run 'ls target_dir' or 'find target_dir -name "*.ext"' to confirm
   - After creating files: Run 'ls -la file_path' to verify file exists
   - After batch operations: Count files BEFORE and AFTER, then compare
   - After deletions: Run 'ls' to confirm files are gone

   Example workflow:
   1. Execute: bash("mv *.md docs/")
   2. Verify: bash("find docs -name '*.md' | wc -l") and bash("find . -maxdepth 1 -name '*.md' | wc -l")
   3. Report: "Moved X files. Verified: docs/ has X files, root has 0 files."

   ⚠️  NEVER say "完了しました" (completed) or "Done" without running verification commands!
   This is NOT nanny behavior - this is PROFESSIONAL ENGINEERING PRACTICE.
"#;

/// Few-shot examples for refactoring tasks
const REFACTORING_EXAMPLES: &str = r#"
═══════════════════════════════════════════════════════════════════
EXAMPLE: GOOD REFACTORING (Extract Function)
═══════════════════════════════════════════════════════════════════

## File: src/processor.rs
```rust
<<<<<<< SEARCH
fn process_data(data: Vec<String>) -> Result<()> {
    for item in data {
        if item.contains("error") {
            eprintln!("Error found: {}", item);
            log_to_file(&format!("ERROR: {}", item))?;
        } else if item.contains("warning") {
            println!("Warning: {}", item);
            log_to_file(&format!("WARN: {}", item))?;
        }
    }
    Ok(())
}
=======
fn process_data(data: Vec<String>) -> Result<()> {
    data.into_iter()
        .try_for_each(|item| process_item(&item))
}

fn process_item(item: &str) -> Result<()> {
    match item {
        s if s.contains("error") => {
            eprintln!("Error found: {}", s);
            log_to_file(&format!("ERROR: {}", s))
        }
        s if s.contains("warning") => {
            println!("Warning: {}", s);
            log_to_file(&format!("WARN: {}", s))
        }
        _ => Ok(())
    }
}
>>>>>>> REPLACE
```

Why this is good:
- Extracted complex logic into smaller, testable function
- Used functional style with try_for_each
- Pattern matching makes intent clearer
- Each function has single responsibility
"#;

/// Few-shot examples for bug fixing
const BUGFIX_EXAMPLES: &str = r#"
═══════════════════════════════════════════════════════════════════
EXAMPLE: GOOD BUG FIX (Off-by-one Error)
═══════════════════════════════════════════════════════════════════

## File: src/indexer.rs
```rust
<<<<<<< SEARCH
fn get_line_at_index(lines: &[String], index: usize) -> Option<&String> {
    if index <= lines.len() {
        Some(&lines[index])
    } else {
        None
    }
}
=======
fn get_line_at_index(lines: &[String], index: usize) -> Option<&String> {
    if index < lines.len() {
        Some(&lines[index])
    } else {
        None
    }
}
>>>>>>> REPLACE
```

Why this is good:
- Fixed off-by-one error (changed <= to <)
- Arrays are 0-indexed, so valid indices are 0..len-1
- Added bounds check prevents panic
- Could also use lines.get(index) for more idiomatic Rust
"#;

/// Few-shot examples for test writing
const TEST_EXAMPLES: &str = r#"
═══════════════════════════════════════════════════════════════════
EXAMPLE: GOOD TEST STRUCTURE
═══════════════════════════════════════════════════════════════════

## File: tests/parser_test.rs
```rust
<<<<<<< SEARCH
=======
#[cfg(test)]
mod parser_tests {
    use super::*;

    #[test]
    fn test_parse_valid_input() {
        // Arrange
        let input = "key=value";

        // Act
        let result = parse_config(input);

        // Assert
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_parse_invalid_input_returns_error() {
        // Arrange
        let input = "invalid";

        // Act
        let result = parse_config(input);

        // Assert
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Invalid format: expected 'key=value'"
        );
    }

    #[test]
    fn test_parse_empty_input() {
        // Arrange
        let input = "";

        // Act
        let result = parse_config(input);

        // Assert
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
>>>>>>> REPLACE
```

Why this is good:
- Uses Arrange-Act-Assert pattern (AAA)
- Tests both success and error cases
- Tests edge cases (empty input)
- Clear, descriptive test names
- Each test is independent and focused
"#;

/// Few-shot examples for adding new features
const FEATURE_EXAMPLES: &str = r#"
═══════════════════════════════════════════════════════════════════
EXAMPLE: GOOD FEATURE ADDITION (Incremental Approach)
═══════════════════════════════════════════════════════════════════

## File: src/user.rs
```rust
<<<<<<< SEARCH
pub struct User {
    pub id: u64,
    pub name: String,
}

impl User {
    pub fn new(id: u64, name: String) -> Self {
        Self { id, name }
    }
}
=======
pub struct User {
    pub id: u64,
    pub name: String,
    pub email: Option<String>,
}

impl User {
    pub fn new(id: u64, name: String) -> Self {
        Self { id, name, email: None }
    }

    pub fn with_email(mut self, email: String) -> Self {
        self.email = Some(email);
        self
    }
}
>>>>>>> REPLACE
```

Why this is good:
- Backward compatible (email is Optional)
- Builder pattern for optional fields
- Minimal changes to existing code
- Easy to test incrementally
"#;

/// Few-shot examples for documentation
const DOCUMENTATION_EXAMPLES: &str = r#"
═══════════════════════════════════════════════════════════════════
EXAMPLE: GOOD DOCUMENTATION
═══════════════════════════════════════════════════════════════════

## File: src/cache.rs
```rust
<<<<<<< SEARCH
pub struct Cache<K, V> {
    data: HashMap<K, V>,
}

impl<K: Eq + Hash, V> Cache<K, V> {
    pub fn new() -> Self {
        Self { data: HashMap::new() }
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.data.get(key)
    }
}
=======
/// A simple in-memory cache for key-value pairs.
///
/// # Examples
///
/// ```
/// use myapp::Cache;
///
/// let mut cache = Cache::new();
/// cache.insert("key", "value");
/// assert_eq!(cache.get(&"key"), Some(&"value"));
/// ```
pub struct Cache<K, V> {
    data: HashMap<K, V>,
}

impl<K: Eq + Hash, V> Cache<K, V> {
    /// Creates a new empty cache.
    pub fn new() -> Self {
        Self { data: HashMap::new() }
    }

    /// Retrieves a value from the cache by key.
    ///
    /// Returns `None` if the key is not present.
    pub fn get(&self, key: &K) -> Option<&V> {
        self.data.get(key)
    }
}
>>>>>>> REPLACE
```

Why this is good:
- Clear module-level documentation
- Usage examples in doc comments
- Describes parameters and return values
- Examples are testable (doc tests)
"#;

/// Few-shot examples for performance optimization
const PERFORMANCE_EXAMPLES: &str = r#"
═══════════════════════════════════════════════════════════════════
EXAMPLE: GOOD PERFORMANCE OPTIMIZATION
═══════════════════════════════════════════════════════════════════

## File: src/processor.rs
```rust
<<<<<<< SEARCH
pub fn find_duplicates(items: &[String]) -> Vec<String> {
    let mut duplicates = Vec::new();
    for i in 0..items.len() {
        for j in i+1..items.len() {
            if items[i] == items[j] && !duplicates.contains(&items[i]) {
                duplicates.push(items[i].clone());
            }
        }
    }
    duplicates
}
=======
pub fn find_duplicates(items: &[String]) -> Vec<String> {
    use std::collections::HashSet;

    let mut seen = HashSet::new();
    let mut duplicates = HashSet::new();

    for item in items {
        if !seen.insert(item) {
            duplicates.insert(item.clone());
        }
    }

    duplicates.into_iter().collect()
}
>>>>>>> REPLACE
```

Why this is good:
- Reduced from O(n²) to O(n) complexity
- Uses HashSet for O(1) lookups
- Avoids nested loops
- More efficient memory usage
"#;

/// Few-shot examples for security improvements
const SECURITY_EXAMPLES: &str = r#"
═══════════════════════════════════════════════════════════════════
EXAMPLE: GOOD SECURITY IMPROVEMENT
═══════════════════════════════════════════════════════════════════

## File: src/auth.rs
```rust
<<<<<<< SEARCH
pub fn verify_password(input: &str, stored: &str) -> bool {
    input == stored
}

pub fn hash_password(password: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    password.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}
=======
pub fn verify_password(input: &str, stored_hash: &str) -> Result<bool, Error> {
    use argon2::{Argon2, PasswordHash, PasswordVerifier};

    let parsed_hash = PasswordHash::new(stored_hash)?;
    Ok(Argon2::default()
        .verify_password(input.as_bytes(), &parsed_hash)
        .is_ok())
}

pub fn hash_password(password: &str) -> Result<String, Error> {
    use argon2::{Argon2, PasswordHasher};
    use argon2::password_hash::SaltString;
    use rand_core::OsRng;

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2.hash_password(password.as_bytes(), &salt)?;
    Ok(hash.to_string())
}
>>>>>>> REPLACE
```

Why this is good:
- Uses cryptographically secure Argon2 instead of DefaultHasher
- Adds proper salt generation
- Returns Result for error handling
- Follows security best practices (OWASP)
"#;

/// Task intent detected from user message
#[derive(Debug, Clone, Copy, PartialEq)]
enum TaskIntent {
    Refactoring,
    BugFix,
    Testing,
    AddFeature,
    Documentation,
    Performance,
    Security,
}

pub struct PromptGenerator {
    edit_format: String,
    max_iterations: usize,
}

impl PromptGenerator {
    pub fn new(edit_format: String) -> Self {
        Self {
            edit_format,
            max_iterations: 30, // Default value
        }
    }

    /// Create with custom max iterations
    pub fn with_max_iterations(mut self, max_iterations: usize) -> Self {
        self.max_iterations = max_iterations;
        self
    }

    /// Detect intent from user message to select appropriate examples
    fn detect_intent(message: &str) -> TaskIntent {
        let message_lower = message.to_lowercase();

        // Check for security-related keywords (highest priority)
        if message_lower.contains("security")
            || message_lower.contains("secure")
            || message_lower.contains("vulnerab")
            || message_lower.contains("セキュリティ")
            || message_lower.contains("脆弱性")
            || message_lower.contains("exploit")
            || message_lower.contains("injection") {
            return TaskIntent::Security;
        }

        // Check for performance-related keywords
        if message_lower.contains("performance")
            || message_lower.contains("optimize")
            || message_lower.contains("speed")
            || message_lower.contains("slow")
            || message_lower.contains("パフォーマンス")
            || message_lower.contains("最適化")
            || message_lower.contains("高速化")
            || message_lower.contains("遅い") {
            return TaskIntent::Performance;
        }

        // Check for documentation keywords
        if message_lower.contains("document")
            || message_lower.contains("docs")
            || message_lower.contains("comment")
            || message_lower.contains("ドキュメント")
            || message_lower.contains("コメント")
            || message_lower.contains("説明")
            || message_lower.contains("javadoc")
            || message_lower.contains("rustdoc") {
            return TaskIntent::Documentation;
        }

        // Check for test-related keywords
        if message_lower.contains("test")
            || message_lower.contains("テスト")
            || message_lower.contains("unit test")
            || message_lower.contains("add tests") {
            return TaskIntent::Testing;
        }

        // Check for bug fix keywords
        if message_lower.contains("fix")
            || message_lower.contains("bug")
            || message_lower.contains("修正")
            || message_lower.contains("バグ")
            || message_lower.contains("error")
            || message_lower.contains("issue") {
            return TaskIntent::BugFix;
        }

        // Check for feature addition keywords
        if message_lower.contains("add")
            || message_lower.contains("implement")
            || message_lower.contains("create")
            || message_lower.contains("new feature")
            || message_lower.contains("追加")
            || message_lower.contains("実装")
            || message_lower.contains("新機能")
            || message_lower.contains("feature") {
            return TaskIntent::AddFeature;
        }

        // Check for refactoring keywords
        if message_lower.contains("refactor")
            || message_lower.contains("improve")
            || message_lower.contains("clean")
            || message_lower.contains("リファクタ")
            || message_lower.contains("改善")
            || message_lower.contains("extract")
            || message_lower.contains("simplify") {
            return TaskIntent::Refactoring;
        }

        // Default: show refactoring example as it's most generally useful
        TaskIntent::Refactoring
    }

    /// Select appropriate examples based on detected intent
    fn select_examples(intent: TaskIntent) -> &'static str {
        match intent {
            TaskIntent::Refactoring => REFACTORING_EXAMPLES,
            TaskIntent::BugFix => BUGFIX_EXAMPLES,
            TaskIntent::Testing => TEST_EXAMPLES,
            TaskIntent::AddFeature => FEATURE_EXAMPLES,
            TaskIntent::Documentation => DOCUMENTATION_EXAMPLES,
            TaskIntent::Performance => PERFORMANCE_EXAMPLES,
            TaskIntent::Security => SECURITY_EXAMPLES,
        }
    }

    /// Generate system prompt based on edit format
    pub fn generate_system_prompt(&self, file_list: &[&Path]) -> String {
        self.generate_system_prompt_with_intent(file_list, None)
    }

    /// Generate system prompt with intent detection for better examples
    pub fn generate_system_prompt_with_intent(&self, file_list: &[&Path], user_message: Option<&str>) -> String {
        let files_section = if !file_list.is_empty() {
            let files: Vec<String> = file_list
                .iter()
                .map(|p| format!("- {}", p.display()))
                .collect();
            format!("\n\nFiles in context:\n{}", files.join("\n"))
        } else {
            String::new()
        };

        // Detect intent and select appropriate examples
        let intent = user_message
            .map(Self::detect_intent)
            .unwrap_or(TaskIntent::Refactoring);
        let examples = Self::select_examples(intent);

        match self.edit_format.as_str() {
            "diff" => self.diff_system_prompt(&files_section, examples),
            "whole" | "wholefile-func" => self.whole_system_prompt(&files_section),
            _ => self.default_system_prompt(&files_section),
        }
    }

    fn diff_system_prompt(&self, files_section: &str, examples: &str) -> String {
        format!(
            "You are BerryCode, a Senior Software Engineer who values SPEED and EFFICIENCY.\n\
            Your goal is to execute the user's requests with MINIMAL conversation and MAXIMUM speed.\n\
            \n\
            🚨 CRITICAL: You have a STRICT LIMIT of {0} tool calls per conversation. EVERY call counts!\n\
            \n\
            AVAILABLE TOOLS (with signatures):\n\
            READ TOOLS:\n\
            - read_file(path: str) → str: Read the contents of any file (with caching)\n\
            - file_tree(directory: str) → str: Get full project structure\n\
            - list_files(directory: str) → List[str]: List files in a directory\n\
            - glob(pattern: str) → List[str]: Search for files by glob pattern (e.g., \"**/*.rs\")\n\
            - grep(pattern: str, path: str) → str: Search for code patterns across files\n\
            \n\
            WRITE TOOLS:\n\
            - write_file(path: str, content: str) → str: Create a new file or completely replace existing file\n\
            - edit_file(path: str, search: str, replace: str) → str: Edit a file by replacing specific text (SEARCH/REPLACE)\n\
            \n\
            EXECUTION TOOLS:\n\
            - bash(command: str) → str: Execute bash commands (build, test, git, etc.)\n\
            - ask_user(question: str) → str: Ask the user for clarification (ONLY if absolutely necessary)\n\
            \n\
            GIT TOOLS:\n\
            - git_diff(ref: str) → str: Show git diff of changes\n\
            - git_commit(message: str) → str: Commit changes\n\
            \n\
            ═══════════════════════════════════════════════════════════════════\n\
            CORE DIRECTIVES (AGENTIC MODE):\n\
            ═══════════════════════════════════════════════════════════════════\n\
            \n\
            1. **BIAS FOR ACTION** (即断即決):\n\
               - If the user's request is specific (e.g., \"Change 5MB to 15MB\"), EXECUTE IT IMMEDIATELY.\n\
               - DO NOT search, grep, or read source code unless the path is ambiguous.\n\
               - Assume the user knows the codebase better than you.\n\
            \n\
            2. **NO NANNY BEHAVIOR** (過保護禁止):\n\
               - STOP asking \"Are you sure?\" or \"Should I proceed?\".\n\
               - Do not waste turns asking for confirmation before executing.\n\
               - However, ALWAYS verify file operations completed successfully (see verification rules below).\n\
            \n\
            3. **TOOL EFFICIENCY** (一撃必殺):\n\
               - For large replacements (10+ lines or files), use `bash` to run a Python script or `sed`.\n\
               - AVOID using `edit_file` line-by-line for massive changes. It's too slow.\n\
               - If `grep` fails once, don't loop. Ask the user or guess the path.\n\
            \n\
            4. **CONCISE RESPONSES** (簡潔さ):\n\
               - Responses should be: \"Done.\", \"Fixed.\", \"Updated X to Y.\".\n\
               - No explanations needed for simple tasks.\n\
            \n\
            {1}\
            \n\
            6. **CHAIN OF THOUGHT** (思考の連鎖):\n\
               - Before making complex changes, create a plan inside <thinking> tags:\n\
                 <thinking>\n\
                 1. Analyze the file structure and identify affected areas\n\
                 2. Verify the exact lines to change (check indentation!)\n\
                 3. Consider impact on other files/functions\n\
                 4. Plan the verification steps\n\
                 </thinking>\n\
               - Then execute your plan with tool calls.\n\
            \n\
            ═══════════════════════════════════════════════════════════════════\n\
            SEARCH/REPLACE FORMAT:\n\
            ═══════════════════════════════════════════════════════════════════\n\
            \n\
            When making changes using `edit_file` or responding with code, use this format:\n\
            \n\
            ## File: path/to/file.ext\n\
            ```language\n\
            <<<<<<< SEARCH\n\
            exact lines to find\n\
            =======\n\
            replacement lines\n\
            >>>>>>> REPLACE\n\
            ```\n\
            \n\
            Critical Rules:\n\
            1. **FILE PATH**: ALWAYS specify the file path with \"## File: path/to/file.ext\" before the code block\n\
            2. **EXACT MATCH**: The SEARCH block must match EXACTLY, including:\n\
               - Indentation (spaces/tabs) - DO NOT omit leading whitespace\n\
               - Line endings and blank lines\n\
               - All characters, even if they seem redundant\n\
            3. **INDENTATION**: Verify indentation levels BEFORE outputting (2 spaces? 4 spaces? tabs?)\n\
            4. **CONTEXT**: Include enough context lines (2-3 before/after) to uniquely identify the location\n\
            5. **MULTIPLE BLOCKS**: You can have multiple SEARCH/REPLACE blocks in one response\n\
            \n\
            {2}\
            \n\
            {3}\n\
            \n\
            Think like a Senior Engineer: Plan efficiently, act immediately, and provide accurate working code.",
            self.max_iterations,
            VERIFICATION_RULES,
            examples,
            files_section
        )
    }

    fn whole_system_prompt(&self, files_section: &str) -> String {
        format!(
            "You are BerryCode, a Code Generation Specialist.\n\
            Your task is to rewrite files COMPLETELY and ACCURATELY.\n\
            \n\
            🚨 CRITICAL RULES FOR WHOLE FILE REWRITE:\n\
            1. **NO TRUNCATION**: NEVER use comments like `// ... rest of code` or `// ... existing code`.\n\
            2. **FULL CONTENT**: You MUST output the ENTIRE file content from top to bottom, even if it is 1000+ lines.\n\
            3. **CORRECTNESS**: The code must be syntactically perfect. No placeholders.\n\
            4. **FILE SIZE LIMIT**: If a file exceeds 500 lines, STOP and ask the user to switch to `diff` mode.\n\
               - For files > 500 lines, use: \"This file is too large for whole mode. Please use diff mode.\"\n\
               - Only proceed if the user explicitly confirms.\n\
            \n\
            When making changes, provide the COMPLETE modified file content:\n\
            \n\
            ```language\n\
            filename.ext\n\
            <<<<<<< FULL FILE\n\
            (The FULL content of the file goes here)\n\
            >>>>>>> END\n\
            ```\n\
            \n\
            If the file is too large to output in one go, STOP and ask the user to use `edit_file` (diff mode) instead.\n\
            However, if you proceed, you commit to outputting every single byte.\n\
            \n\
            {}\
            {}\n\
            \n\
            Provide the FULL file content now.",
            VERIFICATION_RULES,
            files_section
        )
    }

    fn default_system_prompt(&self, files_section: &str) -> String {
        format!(
            "You are BerryCode, a Principal Software Architect and Technical Advisor.\n\
            User asks you for advice, debugging help, or explanations.\n\
            \n\
            CORE PERSONALITY:\n\
            - **Direct & Technical**: Do not apologize. Do not say \"As an AI\". Go straight to the answer.\n\
            - **Solution Focused**: If there is an error, propose a fix immediately.\n\
            - **Concise**: Use bullet points and code snippets. Avoid long paragraphs.\n\
            \n\
            Your answers should be equivalent to a Senior Engineer answering a colleague on Slack.\n\
            Short, accurate, and helpful.\n\
            {}\n\
            \n\
            Provide your expert advice.",
            files_section
        )
    }

    /// Generate a user prompt for a code change request
    pub fn generate_user_prompt(&self, request: &str, context: Option<&str>) -> String {
        let mut prompt = String::new();

        if let Some(ctx) = context {
            prompt.push_str("# Codebase Context\n\n");
            prompt.push_str(ctx);
            prompt.push_str("\n\n");
        }

        prompt.push_str("# Request\n\n");
        prompt.push_str(request);

        prompt
    }

    /// Generate file content section for prompts
    pub fn generate_file_content(&self, file_path: &Path, content: &str) -> String {
        format!(
            "# {}\n```\n{}\n```\n",
            file_path.display(),
            content
        )
    }

    /// Parse edit format response
    pub fn parse_edit_response(&self, response: &str) -> Result<Vec<EditBlock>> {
        match self.edit_format.as_str() {
            "diff" => self.parse_search_replace(response),
            "whole" => self.parse_whole_file(response),
            _ => Ok(Vec::new()),
        }
    }

    fn parse_search_replace(&self, response: &str) -> Result<Vec<EditBlock>> {
        let mut blocks = Vec::new();
        let lines: Vec<&str> = response.lines().collect();
        let mut i = 0;
        let mut current_file_path: Option<String> = None;

        // Use static compiled regex for performance
        let filename_regex = get_filename_regex();

        while i < lines.len() {
            let line = lines[i].trim();

            // Check for explicit file marker: "## File: path/to/file.ext"
            if line.starts_with("## File:") || line.starts_with("##File:") {
                let path = line.trim_start_matches("## File:")
                    .trim_start_matches("##File:")
                    .trim();
                if !path.is_empty() && Self::is_valid_filename_relaxed(path) {
                    current_file_path = Some(path.to_string());
                    i += 1;
                    continue;
                }
            }

            // Try to extract filename from current line using regex
            if let Some(caps) = filename_regex.captures(line) {
                // Try backtick-quoted path first, then unquoted paths
                let potential_path = caps.get(1)
                    .or_else(|| caps.get(2))
                    .or_else(|| caps.get(3))
                    .map(|m| m.as_str());

                if let Some(p) = potential_path {
                    // Validate that it looks like a file path
                    if Self::is_valid_filename_relaxed(p) {
                        current_file_path = Some(p.to_string());
                    }
                }
            }

            // Also check for standalone filename before code fence
            if line.starts_with("```") {
                // Look back 1-3 lines for a filename
                for back_i in 1..=3 {
                    if i >= back_i {
                        let prev_line = lines[i - back_i].trim();
                        if prev_line.is_empty() {
                            continue;
                        }

                        if let Some(caps) = filename_regex.captures(prev_line) {
                            let potential_path = caps.get(1)
                                .or_else(|| caps.get(2))
                                .or_else(|| caps.get(3))
                                .map(|m| m.as_str());

                            if let Some(p) = potential_path {
                                if Self::is_valid_filename_relaxed(p) {
                                    current_file_path = Some(p.to_string());
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            if lines[i].contains("<<<<<<< SEARCH") {
                i += 1;
                let mut search = Vec::new();
                while i < lines.len() && !lines[i].contains("=======") {
                    search.push(lines[i].to_string());
                    i += 1;
                }

                i += 1; // Skip =======
                let mut replace = Vec::new();
                while i < lines.len() && !lines[i].contains(">>>>>>> REPLACE") {
                    replace.push(lines[i].to_string());
                    i += 1;
                }

                blocks.push(EditBlock {
                    block_type: EditBlockType::SearchReplace,
                    file_path: current_file_path.clone(),
                    search: Some(search.join("\n")),
                    replace: Some(replace.join("\n")),
                    content: None,
                });
            }
            i += 1;
        }

        Ok(blocks)
    }

    fn parse_whole_file(&self, response: &str) -> Result<Vec<EditBlock>> {
        let mut blocks = Vec::new();
        let lines: Vec<&str> = response.lines().collect();
        let mut i = 0;

        // Use static compiled regex for performance
        let filename_regex = get_filename_regex();

        while i < lines.len() {
            // Look for filename followed by code fence
            if i + 1 < lines.len() && lines[i + 1].starts_with("```") {
                let potential_filename = lines[i].trim();

                // Try regex extraction first
                let file_path = if let Some(caps) = filename_regex.captures(potential_filename) {
                    caps.get(1)
                        .or_else(|| caps.get(2))
                        .or_else(|| caps.get(3))
                        .and_then(|m| {
                            let p = m.as_str();
                            if Self::is_valid_filename_relaxed(p) {
                                Some(p.to_string())
                            } else {
                                None
                            }
                        })
                } else if Self::is_valid_filename(potential_filename) {
                    Some(potential_filename.to_string())
                } else {
                    None
                };

                if let Some(path) = file_path {
                    i += 2; // Skip filename and opening fence

                    let mut content = Vec::new();
                    while i < lines.len() && !lines[i].starts_with("```") {
                        content.push(lines[i].to_string());
                        i += 1;
                    }

                    blocks.push(EditBlock {
                        block_type: EditBlockType::WholeFile,
                        file_path: Some(path),
                        search: None,
                        replace: None,
                        content: Some(content.join("\n")),
                    });
                }
            }
            i += 1;
        }

        Ok(blocks)
    }

    /// Validate if a string looks like a valid filename (strict version for backward compatibility)
    fn is_valid_filename(s: &str) -> bool {
        if s.is_empty() || s.len() > 255 {
            return false;
        }

        // Should not contain spaces or look like a sentence
        if s.contains(' ') && (s.contains("is") || s.contains("the") || s.contains(':')) {
            return false;
        }

        // Should have a file extension or be a valid path
        s.contains('.') || s.contains('/')
    }

    /// More relaxed filename validation - allows paths extracted from sentences
    fn is_valid_filename_relaxed(s: &str) -> bool {
        if s.is_empty() || s.len() > 255 {
            return false;
        }

        // Reject obvious non-filenames
        if s.starts_with("http://") || s.starts_with("https://") {
            return false;
        }

        // Must have extension OR contain a path separator
        let has_extension = s.contains('.') && s.rsplit('.').next().map_or(false, |ext| ext.len() <= 10 && ext.chars().all(|c| c.is_alphanumeric()));
        let has_path = s.contains('/') || s.contains('\\');

        // Accept if it looks like a file path
        has_extension || has_path
    }

    /// Generate error recovery prompt to help AI fix issues autonomously
    pub fn error_recovery_prompt(error: &str) -> String {
        let error_type = Self::detect_error_type(error);
        let specific_guidance = Self::get_error_specific_guidance(error_type, error);

        format!(
            "🚨 SYSTEM ERROR DETECTED\n\
            \n\
            Error: {}\n\
            \n\
            Detected Error Type: {:?}\n\
            \n\
            {}\
            \n\
            ⚠️ DO NOT APOLOGIZE. FIX IT IMMEDIATELY.\n\
            \n\
            Recovery Strategy:\n\
            1. Analyze the error message carefully\n\
            2. Apply the specific guidance above\n\
            3. Execute the fix with a tool call\n\
            4. Verify the fix worked\n\
            \n\
            Remember: You are a Senior Engineer. Errors are learning opportunities, not failures.",
            error,
            error_type,
            specific_guidance
        )
    }

    /// Detect the type of error from error message
    fn detect_error_type(error: &str) -> ErrorType {
        let error_lower = error.to_lowercase();

        if error_lower.contains("search block not found")
            || error_lower.contains("search pattern not found")
            || error_lower.contains("exact match")
            || error_lower.contains("could not find") {
            return ErrorType::SearchNotFound;
        }

        if error_lower.contains("no such file")
            || error_lower.contains("file not found")
            || error_lower.contains("cannot find file") {
            return ErrorType::FileNotFound;
        }

        if error_lower.contains("syntax error")
            || error_lower.contains("parse error")
            || error_lower.contains("expected")
            || error_lower.contains("unexpected token") {
            return ErrorType::SyntaxError;
        }

        if error_lower.contains("permission denied")
            || error_lower.contains("access denied") {
            return ErrorType::PermissionDenied;
        }

        if error_lower.contains("ambiguous")
            || error_lower.contains("multiple matches") {
            return ErrorType::AmbiguousMatch;
        }

        if error_lower.contains("timeout")
            || error_lower.contains("timed out") {
            return ErrorType::Timeout;
        }

        ErrorType::Unknown
    }

    /// Get specific guidance based on error type
    fn get_error_specific_guidance(error_type: ErrorType, error: &str) -> String {
        match error_type {
            ErrorType::SearchNotFound => {
                "📍 SEARCH Block Not Found - Recovery Steps:\n\
                \n\
                Common Causes:\n\
                1. Indentation mismatch (spaces vs tabs, or wrong number of spaces)\n\
                2. Missing or extra blank lines\n\
                3. Typo in the search text\n\
                4. File has changed since you last read it\n\
                \n\
                Recovery Actions (try in order):\n\
                1. READ the file again: read_file(path) - verify current content\n\
                2. CHECK indentation: Count spaces/tabs carefully in your SEARCH block\n\
                3. TRY smaller search: Use fewer lines (3-5 unique lines is usually enough)\n\
                4. GREP for key line: grep(\"unique_function_name\", path) to find exact location\n\
                5. If still failing: Use write_file() to replace entire file (only for small files)\n\
                \n\
                Example Fix:\n\
                Instead of:\n\
                ```\n\
                <<<<<<< SEARCH\n\
                  fn example() {  // Wrong indentation!\n\
                =======\n\
                ```\n\
                \n\
                Do:\n\
                ```\n\
                <<<<<<< SEARCH\n\
                fn example() {  // Correct indentation\n\
                =======\n\
                ```".to_string()
            },
            ErrorType::FileNotFound => {
                format!(
                    "📁 File Not Found - Recovery Steps:\n\
                    \n\
                    Possible file path: {}\n\
                    \n\
                    Recovery Actions:\n\
                    1. LIST directory: list_files(\".\") or file_tree(\".\") to see structure\n\
                    2. SEARCH by pattern: glob(\"**/*.rs\") to find similar files\n\
                    3. GREP for content: grep(\"keyword\", \".\") to locate the file\n\
                    4. CHECK parent directory: Maybe the file is in src/, tests/, or examples/\n\
                    5. ASK user if truly unclear: ask_user(\"Where is the file X?\")\n\
                    \n\
                    Common Mistakes:\n\
                    - Relative vs absolute paths\n\
                    - Missing file extension (.rs, .py, .js)\n\
                    - Wrong directory (src/ vs tests/)\n\
                    \n\
                    Try: glob(\"**/*{}\") to find similar files",
                    Self::extract_file_path(error).unwrap_or("unknown"),
                    Self::extract_file_path(error).unwrap_or("unknown")
                )
            },
            ErrorType::SyntaxError => {
                "⚠️ Syntax Error - Recovery Steps:\n\
                \n\
                Recovery Actions:\n\
                1. READ the file: read_file(path) to see current state\n\
                2. CHECK your REPLACE block: Ensure valid syntax\n\
                3. RUN language check: bash(\"cargo check\") or bash(\"python -m py_compile file.py\")\n\
                4. FIX incrementally: Make smaller changes, verify each step\n\
                \n\
                Common Syntax Issues:\n\
                - Missing semicolons, braces, or parentheses\n\
                - Incorrect indentation in Python\n\
                - Unclosed strings or comments\n\
                - Type mismatches\n\
                \n\
                Always verify syntax BEFORE applying changes.".to_string()
            },
            ErrorType::PermissionDenied => {
                "🔒 Permission Denied - Recovery Steps:\n\
                \n\
                This file/directory requires elevated permissions.\n\
                \n\
                Recovery Actions:\n\
                1. CHECK file permissions: bash(\"ls -la path\")\n\
                2. ASK user: ask_user(\"Do I have permission to modify this file?\")\n\
                3. SUGGEST alternative: Propose creating file in different location\n\
                \n\
                Do NOT attempt to use sudo without user confirmation.".to_string()
            },
            ErrorType::AmbiguousMatch => {
                "🔀 Ambiguous Match - Recovery Steps:\n\
                \n\
                Your SEARCH block matched multiple locations in the file.\n\
                \n\
                Recovery Actions:\n\
                1. ADD more context: Include 2-3 lines before and after\n\
                2. INCLUDE unique identifiers: Function names, unique comments, etc.\n\
                3. READ the file: Find the exact location you want to change\n\
                4. USE line numbers: If available, target specific line range\n\
                \n\
                Example:\n\
                Instead of:\n\
                ```\n\
                <<<<<<< SEARCH\n\
                println!(\"Hello\");\n\
                =======\n\
                ```\n\
                \n\
                Do:\n\
                ```\n\
                <<<<<<< SEARCH\n\
                fn main() {\n\
                    println!(\"Hello\");\n\
                    // Initialize app\n\
                =======\n\
                ```".to_string()
            },
            ErrorType::Timeout => {
                "⏱️ Timeout - Recovery Steps:\n\
                \n\
                The operation took too long to complete.\n\
                \n\
                Recovery Actions:\n\
                1. SIMPLIFY operation: Break into smaller steps\n\
                2. USE more specific paths: Avoid searching entire project\n\
                3. LIMIT scope: Use glob patterns to narrow search\n\
                4. CHECK system load: Maybe the system is busy\n\
                \n\
                For large operations, process files in batches.".to_string()
            },
            ErrorType::Unknown => {
                format!(
                    "❓ Unknown Error - General Recovery Steps:\n\
                    \n\
                    Error details: {}\n\
                    \n\
                    Recovery Actions:\n\
                    1. READ the error message carefully - look for clues\n\
                    2. CHECK last successful operation - what changed?\n\
                    3. VERIFY file state: read_file() or list_files()\n\
                    4. TRY simpler approach: Break down the operation\n\
                    5. ASK user if stuck: ask_user(\"I encountered: {}. What should I do?\")\n\
                    \n\
                    Debug systematically:\n\
                    - What was I trying to do?\n\
                    - What did the system return?\n\
                    - What's the simplest fix?",
                    error,
                    error
                )
            }
        }
    }

    /// Extract file path from error message
    fn extract_file_path(error: &str) -> Option<&str> {
        // Try to find patterns like "file 'path'" or "path: error"
        if let Some(start) = error.find('\'') {
            if let Some(end) = error[start + 1..].find('\'') {
                return Some(&error[start + 1..start + 1 + end]);
            }
        }

        if let Some(start) = error.find('"') {
            if let Some(end) = error[start + 1..].find('"') {
                return Some(&error[start + 1..start + 1 + end]);
            }
        }

        None
    }
}

/// Error types for better recovery strategies
#[derive(Debug, Clone, Copy, PartialEq)]
enum ErrorType {
    SearchNotFound,
    FileNotFound,
    SyntaxError,
    PermissionDenied,
    AmbiguousMatch,
    Timeout,
    Unknown,
}

/// Context window manager for prioritizing files
pub struct ContextManager {
    max_tokens: usize,
}

impl ContextManager {
    /// Create a new context manager with token limit
    pub fn new(max_tokens: usize) -> Self {
        Self { max_tokens }
    }

    /// Score files by importance (0.0 - 1.0)
    pub fn score_file_importance(&self, file_path: &Path, user_query: &str) -> f64 {
        let path_str = file_path.to_string_lossy().to_lowercase();
        let query_lower = user_query.to_lowercase();
        let mut score: f64 = 0.0;

        // 1. Direct mention in query (+0.5)
        if query_lower.contains(&path_str) {
            score += 0.5;
        }

        // 1b. Filename stem mentioned in query (+0.4)
        let filename = file_path.file_name().unwrap_or_default().to_string_lossy();
        if let Some(stem) = file_path.file_stem() {
            let stem_str = stem.to_string_lossy().to_lowercase();
            if query_lower.contains(&stem_str) {
                score += 0.4;
            }
        }

        // 2. File type relevance (+0.3)
        if let Some(ext) = file_path.extension() {
            let ext_str = ext.to_string_lossy();
            if query_lower.contains(&format!(".{}", ext_str)) {
                score += 0.3;
            }

            // Core implementation files
            if matches!(ext_str.as_ref(), "rs" | "py" | "js" | "ts" | "go" | "java") {
                score += 0.2;
            }

            // Configuration files (lower priority)
            if matches!(ext_str.as_ref(), "json" | "toml" | "yaml" | "yml" | "txt" | "md") {
                score += 0.1;
            }
        }

        // 3. Directory importance (+0.2)
        if path_str.contains("/src/") || path_str.contains("\\src\\") {
            score += 0.2;
        } else if path_str.contains("/tests/") || path_str.contains("\\tests\\") {
            score += 0.15;
        } else if path_str.contains("/examples/") || path_str.contains("\\examples\\") {
            score += 0.1;
        }

        // 4. Main files (+0.3)
        if matches!(filename.to_lowercase().as_ref(), "main.rs" | "main.py" | "index.js" | "app.rs" | "lib.rs") {
            score += 0.3;
        }

        // 5. Recently modified (if available) - placeholder for future enhancement
        // This would require file metadata

        // Normalize score to 0.0 - 1.0
        f64::min(score, 1.0)
    }

    /// Prioritize files for context window
    pub fn prioritize_files<'a>(
        &self,
        files: &'a [(&'a Path, String)],
        user_query: &str,
    ) -> Vec<(&'a Path, &'a String, f64)> {
        let mut scored: Vec<_> = files
            .iter()
            .map(|(path, content)| {
                let score = self.score_file_importance(path, user_query);
                (*path, content, score)
            })
            .collect();

        // Sort by score (highest first)
        scored.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

        scored
    }

    /// Select files that fit within token limit
    pub fn select_files_within_limit<'a>(
        &self,
        files: &'a [(&'a Path, String)],
        user_query: &str,
    ) -> Vec<&'a Path> {
        let prioritized = self.prioritize_files(files, user_query);
        let mut selected: Vec<&'a Path> = Vec::new();
        let mut total_tokens = 0;

        for (path, content, _score) in prioritized {
            // Rough estimation: 1 token ≈ 4 characters
            let estimated_tokens = content.len() / 4;

            if total_tokens + estimated_tokens <= self.max_tokens {
                selected.push(path);
                total_tokens += estimated_tokens;
            } else {
                // Check if we can include a truncated version
                let remaining_tokens = self.max_tokens.saturating_sub(total_tokens);
                if remaining_tokens > 100 {
                    // Include at least 100 tokens worth
                    selected.push(path);
                }
                break;
            }
        }

        selected
    }

    /// Generate context summary when at token limit
    pub fn generate_context_summary(&self, excluded_files: &[&Path]) -> String {
        if excluded_files.is_empty() {
            return String::new();
        }

        let mut summary = String::from("\n\n⚠️ CONTEXT WINDOW LIMIT REACHED\n\n");
        summary.push_str(&format!(
            "The following {} file(s) were excluded due to token limits:\n",
            excluded_files.len()
        ));

        for path in excluded_files.iter().take(10) {
            summary.push_str(&format!("- {}\n", path.display()));
        }

        if excluded_files.len() > 10 {
            summary.push_str(&format!("... and {} more files\n", excluded_files.len() - 10));
        }

        summary.push_str("\nIf you need these files:\n");
        summary.push_str("1. Use read_file(path) to access specific files\n");
        summary.push_str("2. Ask user which files are most important\n");
        summary.push_str("3. Process files in smaller batches\n");

        summary
    }

    /// Estimate tokens for a string (rough approximation)
    pub fn estimate_tokens(text: &str) -> usize {
        // GPT-3/4 tokenization: roughly 1 token per 4 characters
        // This is a simple heuristic; real tokenization is more complex
        text.len() / 4
    }
}

#[derive(Debug, Clone)]
pub struct EditBlock {
    pub block_type: EditBlockType,
    pub file_path: Option<String>,
    pub search: Option<String>,
    pub replace: Option<String>,
    pub content: Option<String>,
}

#[derive(Debug, Clone)]
pub enum EditBlockType {
    SearchReplace,
    WholeFile,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_system_prompt() {
        let gen = PromptGenerator::new("diff".to_string());
        let prompt = gen.generate_system_prompt(&[]);
        assert!(prompt.contains("SEARCH/REPLACE"));
    }

    #[test]
    fn test_parse_search_replace() {
        let gen = PromptGenerator::new("diff".to_string());
        let response = "```python\n<<<<<<< SEARCH\nold code\n=======\nnew code\n>>>>>>> REPLACE\n```";
        let blocks = gen.parse_search_replace(response).unwrap();
        assert_eq!(blocks.len(), 1);
    }

    /// Test that all prompts contain verification rules
    #[test]
    fn test_all_prompts_contain_verification_rules() {
        let gen = PromptGenerator::new("diff".to_string());

        // Test diff prompt
        let diff_prompt = gen.diff_system_prompt("", REFACTORING_EXAMPLES);
        assert!(diff_prompt.contains("VERIFICATION IS MANDATORY"),
            "diff prompt should contain verification rules");
        assert!(diff_prompt.contains("完了しました"),
            "diff prompt should mention '完了しました'");

        // Test whole prompt
        let whole_prompt = gen.whole_system_prompt("");
        assert!(whole_prompt.contains("VERIFICATION IS MANDATORY"),
            "whole prompt should contain verification rules");
        assert!(whole_prompt.contains("完了しました"),
            "whole prompt should mention '完了しました'");

        // Default prompt doesn't need verification rules (it's for advice only)
        let default_prompt = gen.default_system_prompt("");
        assert!(default_prompt.contains("Principal Software Architect"),
            "default prompt should be for advice");
    }

    /// Test that udiff and editblock formats are no longer supported
    #[test]
    fn test_deprecated_formats_not_supported() {
        // udiff should fall back to default prompt
        let gen_udiff = PromptGenerator::new("udiff".to_string());
        let prompt = gen_udiff.generate_system_prompt(&[]);
        assert!(prompt.contains("Principal Software Architect"),
            "udiff should fall back to default prompt");

        // editblock should fall back to default prompt
        let gen_editblock = PromptGenerator::new("editblock".to_string());
        let prompt = gen_editblock.generate_system_prompt(&[]);
        assert!(prompt.contains("Principal Software Architect"),
            "editblock should fall back to default prompt");
    }

    /// Test that parse_edit_response doesn't handle deprecated formats
    #[test]
    fn test_parse_deprecated_formats_returns_empty() {
        // udiff format should return empty
        let gen_udiff = PromptGenerator::new("udiff".to_string());
        let result = gen_udiff.parse_edit_response("some response").unwrap();
        assert_eq!(result.len(), 0, "udiff parsing should return empty");

        // editblock format should return empty
        let gen_editblock = PromptGenerator::new("editblock".to_string());
        let result = gen_editblock.parse_edit_response("some response").unwrap();
        assert_eq!(result.len(), 0, "editblock parsing should return empty");
    }

    /// Test that only SearchReplace and WholeFile block types exist
    #[test]
    fn test_editblock_types_cleaned_up() {
        use std::mem::discriminant;

        let search_replace = EditBlockType::SearchReplace;
        let whole_file = EditBlockType::WholeFile;

        // These should compile (variants exist)
        let _ = discriminant(&search_replace);
        let _ = discriminant(&whole_file);

        // This test ensures the enum only has 2 variants
        // If we add more variants, this will need updating
        match search_replace {
            EditBlockType::SearchReplace => {},
            EditBlockType::WholeFile => {},
            // Uncommenting the line below should cause a compile error
            // because UnifiedDiff and EditBlock no longer exist
            // EditBlockType::UnifiedDiff => {},
            // EditBlockType::EditBlock => {},
        }
    }

    /// Test verification rules constant content
    #[test]
    fn test_verification_rules_content() {
        assert!(VERIFICATION_RULES.contains("VERIFICATION IS MANDATORY"));
        assert!(VERIFICATION_RULES.contains("After ANY file operation"));
        assert!(VERIFICATION_RULES.contains("mv, cp, rm, write_file, edit_file"));
        assert!(VERIFICATION_RULES.contains("完了しました"));
        assert!(VERIFICATION_RULES.contains("Done"));
        assert!(VERIFICATION_RULES.contains("PROFESSIONAL ENGINEERING PRACTICE"));
    }

    /// Test that diff format is the only supported edit format for code changes
    #[test]
    fn test_supported_edit_formats() {
        let gen_diff = PromptGenerator::new("diff".to_string());
        let prompt = gen_diff.generate_system_prompt(&[]);
        assert!(prompt.contains("SEARCH/REPLACE"), "diff format should use SEARCH/REPLACE");

        let gen_whole = PromptGenerator::new("whole".to_string());
        let prompt = gen_whole.generate_system_prompt(&[]);
        assert!(prompt.contains("Code Generation Specialist"), "whole format should be for full file");
    }

    /// Test that tool signatures are properly defined
    #[test]
    fn test_tool_signatures_defined() {
        let gen = PromptGenerator::new("diff".to_string());
        let prompt = gen.generate_system_prompt(&[]);

        // Check that tools have signatures
        assert!(prompt.contains("read_file(path: str)"), "read_file should have signature");
        assert!(prompt.contains("edit_file(path: str, search: str, replace: str)"),
            "edit_file should have signature");
        assert!(prompt.contains("bash(command: str)"), "bash should have signature");
        assert!(prompt.contains("→ str"), "tools should have return types");
    }

    /// Test that indentation rules are explicitly stated
    #[test]
    fn test_indentation_rules_present() {
        let gen = PromptGenerator::new("diff".to_string());
        let prompt = gen.generate_system_prompt(&[]);

        assert!(prompt.contains("INDENTATION"), "Should mention indentation");
        assert!(prompt.contains("DO NOT omit leading whitespace"),
            "Should warn about whitespace");
        assert!(prompt.contains("spaces/tabs"), "Should mention spaces and tabs");
    }

    /// Test that Chain of Thought is enforced
    #[test]
    fn test_chain_of_thought_enforced() {
        let gen = PromptGenerator::new("diff".to_string());
        let prompt = gen.generate_system_prompt(&[]);

        assert!(prompt.contains("CHAIN OF THOUGHT"), "Should mention Chain of Thought");
        assert!(prompt.contains("<thinking>"), "Should show thinking tag format");
        assert!(prompt.contains("Analyze the file structure"),
            "Should provide thinking examples");
    }

    /// Test that file path format is specified
    #[test]
    fn test_file_path_format_specified() {
        let gen = PromptGenerator::new("diff".to_string());
        let prompt = gen.generate_system_prompt(&[]);

        assert!(prompt.contains("## File:"), "Should specify file path format");
        assert!(prompt.contains("ALWAYS specify the file path"),
            "Should emphasize file path requirement");
    }

    /// Test that parse_search_replace handles "## File:" format
    #[test]
    fn test_parse_file_marker() {
        let gen = PromptGenerator::new("diff".to_string());
        let response = "## File: src/main.rs\n```rust\n<<<<<<< SEARCH\nold\n=======\nnew\n>>>>>>> REPLACE\n```";
        let blocks = gen.parse_search_replace(response).unwrap();

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].file_path, Some("src/main.rs".to_string()));
    }

    /// Test that whole mode has file size limit warning
    #[test]
    fn test_whole_mode_file_size_limit() {
        let gen = PromptGenerator::new("whole".to_string());
        let prompt = gen.generate_system_prompt(&[]);

        assert!(prompt.contains("FILE SIZE LIMIT"), "Should mention file size limit");
        assert!(prompt.contains("500 lines"), "Should specify 500 line limit");
        assert!(prompt.contains("switch to `diff` mode"),
            "Should suggest switching to diff mode");
    }

    /// Test static regex is reused (performance)
    #[test]
    fn test_static_regex_reused() {
        // This test verifies that the regex is compiled only once
        // by checking that multiple calls use the same reference
        let regex1 = get_filename_regex();
        let regex2 = get_filename_regex();

        // Same memory address = same instance = compiled once
        assert!(std::ptr::eq(regex1, regex2),
            "Regex should be compiled once and reused");
    }

    /// Test intent detection for refactoring keywords
    #[test]
    fn test_detect_intent_refactoring() {
        assert_eq!(PromptGenerator::detect_intent("refactor this function"), TaskIntent::Refactoring);
        assert_eq!(PromptGenerator::detect_intent("improve code quality"), TaskIntent::Refactoring);
        assert_eq!(PromptGenerator::detect_intent("clean up this mess"), TaskIntent::Refactoring);
        assert_eq!(PromptGenerator::detect_intent("このコードをリファクタして"), TaskIntent::Refactoring);
        assert_eq!(PromptGenerator::detect_intent("extract this into a function"), TaskIntent::Refactoring);
    }

    /// Test intent detection for bug fix keywords
    #[test]
    fn test_detect_intent_bugfix() {
        assert_eq!(PromptGenerator::detect_intent("fix this bug"), TaskIntent::BugFix);
        assert_eq!(PromptGenerator::detect_intent("there's an error in the code"), TaskIntent::BugFix);
        assert_eq!(PromptGenerator::detect_intent("バグを修正してください"), TaskIntent::BugFix);
        assert_eq!(PromptGenerator::detect_intent("this is causing an issue"), TaskIntent::BugFix);
    }

    /// Test intent detection for testing keywords
    #[test]
    fn test_detect_intent_testing() {
        assert_eq!(PromptGenerator::detect_intent("add tests for this function"), TaskIntent::Testing);
        assert_eq!(PromptGenerator::detect_intent("write unit tests"), TaskIntent::Testing);
        assert_eq!(PromptGenerator::detect_intent("テストを追加して"), TaskIntent::Testing);
        assert_eq!(PromptGenerator::detect_intent("need test coverage"), TaskIntent::Testing);
    }

    /// Test that different intents select different examples
    #[test]
    fn test_select_examples_by_intent() {
        let refactoring = PromptGenerator::select_examples(TaskIntent::Refactoring);
        assert!(refactoring.contains("GOOD REFACTORING"));
        assert!(refactoring.contains("Extract Function"));

        let bugfix = PromptGenerator::select_examples(TaskIntent::BugFix);
        assert!(bugfix.contains("GOOD BUG FIX"));
        assert!(bugfix.contains("Off-by-one Error"));

        let testing = PromptGenerator::select_examples(TaskIntent::Testing);
        assert!(testing.contains("GOOD TEST STRUCTURE"));
        assert!(testing.contains("Arrange-Act-Assert"));
    }

    /// Test that examples are included in diff prompt
    #[test]
    fn test_examples_included_in_diff_prompt() {
        let gen = PromptGenerator::new("diff".to_string());

        // Test refactoring example
        let refactor_prompt = gen.generate_system_prompt_with_intent(&[], Some("refactor this code"));
        assert!(refactor_prompt.contains("GOOD REFACTORING"), "Refactoring example should be in prompt");
        assert!(refactor_prompt.contains("process_data"), "Should contain example code");

        // Test bugfix example
        let bugfix_prompt = gen.generate_system_prompt_with_intent(&[], Some("fix this bug"));
        assert!(bugfix_prompt.contains("GOOD BUG FIX"), "Bugfix example should be in prompt");
        assert!(bugfix_prompt.contains("Off-by-one Error"), "Should contain bugfix example");

        // Test testing example
        let test_prompt = gen.generate_system_prompt_with_intent(&[], Some("add tests"));
        assert!(test_prompt.contains("GOOD TEST STRUCTURE"), "Testing example should be in prompt");
        assert!(test_prompt.contains("Arrange-Act-Assert"), "Should contain testing pattern");
    }

    /// Test backward compatibility - generate_system_prompt should still work
    #[test]
    fn test_backward_compatibility_system_prompt() {
        let gen = PromptGenerator::new("diff".to_string());
        let prompt = gen.generate_system_prompt(&[]);

        // Should include refactoring examples by default
        assert!(prompt.contains("EXAMPLE:"), "Should include examples");
        assert!(prompt.contains("SEARCH/REPLACE"), "Should have SEARCH/REPLACE format");
    }

    /// Test that examples contain proper SEARCH/REPLACE format
    #[test]
    fn test_examples_have_correct_format() {
        assert!(REFACTORING_EXAMPLES.contains("<<<<<<< SEARCH"), "Refactoring example should have SEARCH marker");
        assert!(REFACTORING_EXAMPLES.contains("======="), "Refactoring example should have separator");
        assert!(REFACTORING_EXAMPLES.contains(">>>>>>> REPLACE"), "Refactoring example should have REPLACE marker");
        assert!(REFACTORING_EXAMPLES.contains("## File:"), "Refactoring example should have file marker");

        assert!(BUGFIX_EXAMPLES.contains("<<<<<<< SEARCH"), "Bugfix example should have SEARCH marker");
        assert!(BUGFIX_EXAMPLES.contains("## File:"), "Bugfix example should have file marker");

        assert!(TEST_EXAMPLES.contains("<<<<<<< SEARCH"), "Test example should have SEARCH marker");
        assert!(TEST_EXAMPLES.contains("## File:"), "Test example should have file marker");
    }

    /// Test new intent types detection
    #[test]
    fn test_detect_new_intents() {
        assert_eq!(PromptGenerator::detect_intent("add new feature"), TaskIntent::AddFeature);
        assert_eq!(PromptGenerator::detect_intent("implement user authentication"), TaskIntent::AddFeature);
        assert_eq!(PromptGenerator::detect_intent("新機能を追加"), TaskIntent::AddFeature);

        assert_eq!(PromptGenerator::detect_intent("add documentation"), TaskIntent::Documentation);
        assert_eq!(PromptGenerator::detect_intent("write docs for this"), TaskIntent::Documentation);
        assert_eq!(PromptGenerator::detect_intent("ドキュメントを追加"), TaskIntent::Documentation);

        assert_eq!(PromptGenerator::detect_intent("optimize performance"), TaskIntent::Performance);
        assert_eq!(PromptGenerator::detect_intent("this is too slow"), TaskIntent::Performance);
        assert_eq!(PromptGenerator::detect_intent("パフォーマンス改善"), TaskIntent::Performance);

        assert_eq!(PromptGenerator::detect_intent("fix security vulnerability"), TaskIntent::Security);
        assert_eq!(PromptGenerator::detect_intent("secure this endpoint"), TaskIntent::Security);
        assert_eq!(PromptGenerator::detect_intent("セキュリティ強化"), TaskIntent::Security);
    }

    /// Test new examples are selected correctly
    #[test]
    fn test_new_examples_selection() {
        let feature = PromptGenerator::select_examples(TaskIntent::AddFeature);
        assert!(feature.contains("GOOD FEATURE ADDITION"));
        assert!(feature.contains("Incremental Approach"));

        let docs = PromptGenerator::select_examples(TaskIntent::Documentation);
        assert!(docs.contains("GOOD DOCUMENTATION"));
        assert!(docs.contains("Examples"));

        let perf = PromptGenerator::select_examples(TaskIntent::Performance);
        assert!(perf.contains("PERFORMANCE OPTIMIZATION"));
        assert!(perf.contains("O(n²) to O(n)"));

        let security = PromptGenerator::select_examples(TaskIntent::Security);
        assert!(security.contains("SECURITY IMPROVEMENT"));
        assert!(security.contains("Argon2"));
    }

    /// Test error type detection
    #[test]
    fn test_error_type_detection() {
        assert_eq!(
            PromptGenerator::detect_error_type("SEARCH block not found in file"),
            ErrorType::SearchNotFound
        );

        assert_eq!(
            PromptGenerator::detect_error_type("No such file or directory: 'test.rs'"),
            ErrorType::FileNotFound
        );

        assert_eq!(
            PromptGenerator::detect_error_type("Syntax error: expected ';'"),
            ErrorType::SyntaxError
        );

        assert_eq!(
            PromptGenerator::detect_error_type("Permission denied"),
            ErrorType::PermissionDenied
        );

        assert_eq!(
            PromptGenerator::detect_error_type("Ambiguous match: found 3 occurrences"),
            ErrorType::AmbiguousMatch
        );

        assert_eq!(
            PromptGenerator::detect_error_type("Operation timed out"),
            ErrorType::Timeout
        );
    }

    /// Test error recovery prompt contains specific guidance
    #[test]
    fn test_error_recovery_specific_guidance() {
        let recovery = PromptGenerator::error_recovery_prompt("SEARCH block not found");
        assert!(recovery.contains("SEARCH Block Not Found"));
        assert!(recovery.contains("Indentation mismatch"));
        assert!(recovery.contains("READ the file again"));

        let file_recovery = PromptGenerator::error_recovery_prompt("No such file: 'test.rs'");
        assert!(file_recovery.contains("File Not Found"));
        assert!(file_recovery.contains("LIST directory"));
        assert!(file_recovery.contains("glob"));
    }

    /// Test context manager file scoring
    #[test]
    fn test_context_manager_scoring() {
        use std::path::PathBuf;

        let manager = ContextManager::new(10000);

        // Main file should score high
        let main_rs = PathBuf::from("src/main.rs");
        let score = manager.score_file_importance(&main_rs, "fix bug in main");
        assert!(score > 0.5, "main.rs should have high score");

        // Mentioned file should score higher
        let mentioned = PathBuf::from("src/auth.rs");
        let score_mentioned = manager.score_file_importance(&mentioned, "update src/auth.rs");
        assert!(score_mentioned > 0.5, "Mentioned file should score high");

        // Config file should score lower
        let config = PathBuf::from("config.toml");
        let score_config = manager.score_file_importance(&config, "fix bug");
        assert!(score_config < 0.5, "Config file should score lower");
    }

    /// Test context manager prioritization
    #[test]
    fn test_context_manager_prioritization() {
        use std::path::PathBuf;

        let manager = ContextManager::new(10000);

        let readme = PathBuf::from("README.md");
        let main_rs = PathBuf::from("src/main.rs");
        let lib_rs = PathBuf::from("src/lib.rs");

        let files = vec![
            (readme.as_path(), "readme content".to_string()),
            (main_rs.as_path(), "main content".to_string()),
            (lib_rs.as_path(), "lib content".to_string()),
        ];

        let prioritized = manager.prioritize_files(&files, "update main");

        // main.rs should be first (mentioned + src/ + main file)
        assert_eq!(prioritized[0].0.file_name().unwrap().to_str().unwrap(), "main.rs");
    }

    /// Test token estimation
    #[test]
    fn test_token_estimation() {
        let text = "a".repeat(400); // 400 characters
        let tokens = ContextManager::estimate_tokens(&text);
        assert_eq!(tokens, 100); // 400 / 4 = 100 tokens
    }
}
