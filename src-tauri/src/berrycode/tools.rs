//! Tool system for LLM function calling

use crate::berrycode::Result;
use crate::berrycode::cache::FileCache;
use crate::berrycode::llm::{LLMClient, Message};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::fs;
use anyhow::anyhow;
use once_cell::sync::Lazy;

// Global file cache
static FILE_CACHE: Lazy<FileCache> = Lazy::new(|| FileCache::new(100));

/// Tool definition for LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: FunctionDefinition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

/// Tool call from LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: FunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

/// Tool call result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub role: String,
    pub content: String,
}

impl ToolResult {
    pub fn new(tool_call_id: String, content: String) -> Self {
        Self {
            tool_call_id,
            role: "tool".to_string(),
            content,
        }
    }
}

/// Available tools
pub fn get_available_tools() -> Vec<Tool> {
    let mut tools = vec![
        // 🎯 SEMANTIC SEARCH FIRST - Use AI to understand intent, not just keywords
        semantic_search_tool(),
        // Basic file operations
        read_file_tool(),
        write_file_tool(),
        edit_file_tool(),
        translate_file_tool(), // 🌐 Background translation without context pollution
        list_files_tool(),
        file_tree_tool(),
        search_files_tool(),
        grep_tool(),
        bash_tool(),
        lint_code_tool(),
        ask_user_tool(),
        propose_plan_tool(),
        web_fetch_tool(),
        web_search_tool(),
        git_diff_tool(),
        git_commit_tool(),
        git_pr_create_tool(),
        notebook_edit_tool(),
    ];

    // Add browser tools if feature is enabled
    #[cfg(feature = "browser")]
    {
        tools.push(browser_visit_tool());
        tools.push(browser_read_tool());
    }

    // Add artifacts tool
    tools.push(create_artifact_tool());

    // Add LSP tools for intelligent code navigation
    tools.push(lsp_goto_definition_tool());
    tools.push(lsp_find_references_tool());
    tools.push(lsp_hover_tool());

    // Add Mad Science tools
    tools.push(self_healing_tool());
    tools.push(vision_analyze_tool());
    tools.push(knowledge_graph_tool());

    // Add Singularity tools
    tools.push(hive_mind_tool());

    tools
}

/// Read file tool definition
fn read_file_tool() -> Tool {
    Tool {
        tool_type: "function".to_string(),
        function: FunctionDefinition {
            name: "read_file".to_string(),
            description: "Read the contents of a specific file. CRITICAL: For conceptual understanding ('explain this project', 'how does X work'), use semantic_search FIRST to find relevant files, THEN read_file. Use read_file when you already know the exact file path you need. Returns the full file content with line numbers.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "The path to the file to read (relative to project root). Example: 'src/main.rs', 'README.md'"
                    }
                },
                "required": ["file_path"]
            }),
        },
    }
}

/// Write file tool definition
fn write_file_tool() -> Tool {
    Tool {
        tool_type: "function".to_string(),
        function: FunctionDefinition {
            name: "write_file".to_string(),
            description: "Write content to a file (creates new file or overwrites existing). Use this to create new files or completely replace file contents. CRITICAL: After writing, ALWAYS call git_diff to verify what changed and show the user the modifications.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "The path to the file to write (relative to project root)"
                    },
                    "content": {
                        "type": "string",
                        "description": "The complete content to write to the file"
                    }
                },
                "required": ["file_path", "content"]
            }),
        },
    }
}

/// Edit file tool definition
fn edit_file_tool() -> Tool {
    Tool {
        tool_type: "function".to_string(),
        function: FunctionDefinition {
            name: "edit_file".to_string(),
            description: "Edit a file by replacing specific text. PREFERRED over write_file for partial changes. IMPORTANT: 'search' text must be unique in the file (appears exactly once). If search text appears multiple times or not at all, the edit will fail. Always read_file first to get the exact text to search for. CRITICAL: After editing, ALWAYS call git_diff to verify what changed and show the user.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "The path to the file to edit (relative to project root). Example: 'src/main.rs'"
                    },
                    "search": {
                        "type": "string",
                        "description": "The exact text to search for. Must be unique (appear exactly once) and match exactly including whitespace. Include enough context to make it unique. Example: 'fn main() {\\n    println!(\"Hello\");\\n}'"
                    },
                    "replace": {
                        "type": "string",
                        "description": "The text to replace it with. Can be empty string to delete. Preserve indentation and formatting. Example: 'fn main() {\\n    println!(\"Hello, World!\");\\n    println!(\"Welcome\");\\n}'"
                    }
                },
                "required": ["file_path", "search", "replace"]
            }),
        },
    }
}

/// Translate file tool definition
fn translate_file_tool() -> Tool {
    Tool {
        tool_type: "function".to_string(),
        function: FunctionDefinition {
            name: "translate_file".to_string(),
            description: "Translate a file into a target language in the background without loading it into chat context. \
            Creates a new file with language suffix (e.g., README.md -> README_ja.md). \
            This tool is PERFECT for large-scale document translation because it doesn't pollute the chat context. \
            Use this for all translation tasks instead of read_file + manual translation. \
            Supports parallel execution - you can translate multiple files at once!".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "The path to the file to translate (relative to project root). Example: 'README.md', 'docs/INSTALL.md'"
                    },
                    "target_language": {
                        "type": "string",
                        "description": "The target language for translation. Example: 'Japanese', 'English', 'Spanish', 'French', 'Chinese'"
                    },
                    "language_code": {
                        "type": "string",
                        "description": "Optional 2-letter language code for file suffix (default: 'ja' for Japanese, 'en' for English, 'es' for Spanish, etc.). Example: 'ja', 'en', 'es'"
                    }
                },
                "required": ["file_path", "target_language"]
            }),
        },
    }
}

/// List files tool definition
fn list_files_tool() -> Tool {
    Tool {
        tool_type: "function".to_string(),
        function: FunctionDefinition {
            name: "list_files".to_string(),
            description: "List files and directories in a directory. Use this to: explore project structure, understand codebase organization, find where to create new files, check if files/directories exist. Returns list with file/directory indicators and sizes.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "directory": {
                        "type": "string",
                        "description": "The directory to list (relative to project root). Default: '.' (current directory). Examples: 'src', 'tests', 'src/components'"
                    }
                },
                "required": []
            }),
        },
    }
}

/// File tree tool definition - shows full directory structure
fn file_tree_tool() -> Tool {
    Tool {
        tool_type: "function".to_string(),
        function: FunctionDefinition {
            name: "file_tree".to_string(),
            description: "🗺️ PROJECT STRUCTURE MAP - ALWAYS USE THIS FIRST! Get a complete tree view of the entire project structure. CRITICAL: This is the PRIMARY tool for understanding project 'design', 'architecture', 'structure', or 'overview'. Use this BEFORE semantic_search or read_file when the user asks about the project layout. Shows all directories and files in a nested tree format. Automatically excludes common ignore patterns (node_modules, target, .git, etc.). ONE file_tree call gives you the complete map instantly!".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "max_depth": {
                        "type": "number",
                        "description": "Maximum directory depth to show (default: 3). Use 2-3 for large projects, 5+ for small ones."
                    }
                },
                "required": []
            }),
        },
    }
}

/// Search files tool definition (glob pattern) - renamed to glob for Claude Code compatibility
fn search_files_tool() -> Tool {
    Tool {
        tool_type: "function".to_string(),
        function: FunctionDefinition {
            name: "glob".to_string(),
            description: "Fast file pattern matching tool that works with any codebase size. Supports glob patterns like '**/*.js' or 'src/**/*.ts'. Returns matching file paths sorted by modification time (most recent first). Use this when you need to find files by name patterns. Examples: Find all Rust files: '**/*.rs', Find TypeScript components: 'src/components/**/*.tsx', Find config files: '**/*.{json,yaml,yml}'".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Glob pattern to match files. Examples: '**/*.rs' (all Rust files), 'src/**/*.ts' (TypeScript in src), '**/*.{js,jsx}' (JS files with multiple extensions)"
                    },
                    "path": {
                        "type": "string",
                        "description": "Optional: Directory to search in. If not specified, searches from project root. Example: 'src', 'tests'"
                    }
                },
                "required": ["pattern"]
            }),
        },
    }
}

/// Grep tool definition
fn grep_tool() -> Tool {
    Tool {
        tool_type: "function".to_string(),
        function: FunctionDefinition {
            name: "grep".to_string(),
            description: "Search for EXACT TEXT patterns across files. ⚠️ CRITICAL: (1) ALWAYS set head_limit parameter (default: 30) to prevent payload overflow! (2) NEVER use broad patterns like 'error|ERROR|panic|TODO' - causes 413 Payload Too Large! Use specific patterns. (3) Use semantic_search for conceptual questions ('how does auth work?'). grep searches by EXACT TEXT, semantic_search finds by MEANING. Shows 3 lines context by default. Supports regex.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "The pattern to search for. Supports regex. ⚠️ AVOID BROAD PATTERNS! Examples: 'fn main' ✓, 'class.*Error' ✓, 'AuthError' ✓, 'error|ERROR|panic|TODO' ✗ (too broad!)"
                    },
                    "file_pattern": {
                        "type": "string",
                        "description": "Optional glob pattern to limit search to specific files. Default: '**/*' (all files). Examples: '*.rs' (all Rust files), 'src/**/*.ts' (TypeScript in src), '*.py' (Python files)"
                    },
                    "context_lines": {
                        "type": "number",
                        "description": "Number of lines to show BEFORE and AFTER each match for context. Default: 3. Use 0 for no context, 5-10 for more context. With context, you can often avoid calling read_file!"
                    },
                    "head_limit": {
                        "type": "number",
                        "description": "⚠️ CRITICAL: Maximum number of file matches to return (default: 30). ALWAYS use this to prevent 413 Payload Too Large errors! Use 50 for broader searches, 10 for quick checks."
                    }
                },
                "required": ["pattern"]
            }),
        },
    }
}

/// Semantic search tool definition - Use vector embeddings for conceptual search
fn semantic_search_tool() -> Tool {
    Tool {
        tool_type: "function".to_string(),
        function: FunctionDefinition {
            name: "semantic_search".to_string(),
            description: "🎯 SEMANTIC SEARCH: Find SPECIFIC implementations by meaning, not exact text! Use this ONLY for finding specific logic AFTER you've seen the project structure with file_tree. Perfect for: finding specific implementations ('How does authentication work internally?'), locating design patterns ('Where is dependency injection used?'), discovering specific features ('Payment processing logic'). Do NOT use this for general architecture questions - use file_tree first! This searches by SEMANTIC SIMILARITY using vector embeddings. Use grep for exact string matching (variable names, error messages).".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Natural language query describing what you're looking for. Examples: 'authentication and login logic', 'database schema and models', 'API endpoint definitions', 'error handling patterns', 'user interface components'"
                    },
                    "limit": {
                        "type": "number",
                        "description": "Maximum number of results to return (default: 10). Use 5 for quick overview, 20 for comprehensive search"
                    }
                },
                "required": ["query"]
            }),
        },
    }
}

/// Bash execution tool definition
fn bash_tool() -> Tool {
    Tool {
        tool_type: "function".to_string(),
        function: FunctionDefinition {
            name: "bash".to_string(),
            description: "Execute bash commands. EFFICIENCY RULES: (1) Combine related commands with && instead of separate calls (e.g., 'git log -5 && git diff HEAD~5..HEAD' instead of calling bash twice). (2) For git diff operations, use the git_diff tool instead - it's more efficient. (3) When reviewing code/diffs, get the full range in ONE command (e.g., 'git diff origin/main...HEAD') instead of checking HEAD~1, HEAD~2 separately. VERIFICATION RULE: After file operations (mv, cp, rm), you MUST run a follow-up verification command (e.g., 'ls target_dir', 'find . -name \"*.ext\" | wc -l') to confirm the operation succeeded. NEVER say 'Done' or '完了' without verification. Use for: build, test, install dependencies, etc. Be careful with destructive commands.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The bash command to execute. Combine multiple commands with && for efficiency (e.g., 'cargo build && cargo test')"
                    },
                    "description": {
                        "type": "string",
                        "description": "A brief description of what this command does"
                    }
                },
                "required": ["command"]
            }),
        },
    }
}

/// Lint code tool definition - runs static analysis to catch errors before compiling
fn lint_code_tool() -> Tool {
    Tool {
        tool_type: "function".to_string(),
        function: FunctionDefinition {
            name: "lint_code".to_string(),
            description: "Run static analysis and linting on the code to catch errors, warnings, and style issues. CRITICAL: Use this AFTER editing or creating code files, BEFORE showing results to the user. Detects: compilation errors, type errors, syntax errors, unused variables, etc. Returns detailed error messages with file locations and line numbers. Supports Rust (cargo check/clippy), Python (flake8/pylint), JavaScript/TypeScript (eslint), and other languages.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "files": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Optional: Specific files to lint. If omitted, lints entire project."
                    },
                    "strict": {
                        "type": "boolean",
                        "description": "If true, uses stricter linting rules (e.g., clippy in Rust, strict mode in ESLint)"
                    }
                },
                "required": []
            }),
        },
    }
}

/// Browser visit tool definition - Navigate to URL and render page
#[cfg(feature = "browser")]
fn browser_visit_tool() -> Tool {
    Tool {
        tool_type: "function".to_string(),
        function: FunctionDefinition {
            name: "browser_visit".to_string(),
            description: "Launch a headless browser, navigate to a URL, and render the page. Use this to: verify web UI changes, check if localhost server is running, read documentation pages, debug visual issues. Returns page title and URL. Follow up with browser_read to extract text content.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "The URL to visit (e.g., 'http://localhost:3000', 'https://docs.rust-lang.org')"
                    }
                },
                "required": ["url"]
            }),
        },
    }
}

/// Browser read tool definition - Extract text from current page
#[cfg(feature = "browser")]
fn browser_read_tool() -> Tool {
    Tool {
        tool_type: "function".to_string(),
        function: FunctionDefinition {
            name: "browser_read".to_string(),
            description: "Read and extract text content from a webpage, converted to clean Markdown format. Use AFTER browser_visit to get the actual page content. Returns visible text, links, headings, and lists in Markdown. Perfect for: reading documentation, checking error messages on web pages, verifying content. The content is automatically converted from HTML to Markdown for better readability and lower token usage.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "The URL to read from"
                    },
                    "selector": {
                        "type": "string",
                        "description": "Optional CSS selector to extract specific content (e.g., 'article', '.documentation')"
                    }
                },
                "required": ["url"]
            }),
        },
    }
}

/// Propose implementation plan tool definition
fn propose_plan_tool() -> Tool {
    Tool {
        tool_type: "function".to_string(),
        function: FunctionDefinition {
            name: "propose_plan".to_string(),
            description: "Propose an implementation plan for complex tasks (new features, refactoring, multi-file changes, architecture changes). IMPORTANT: Use this BEFORE starting implementation for non-trivial tasks. The plan will be presented to the user for approval. Only proceed with implementation after user approves the plan.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "title": {
                        "type": "string",
                        "description": "Plan title summarizing the task. Example: 'Implement User Authentication System'"
                    },
                    "description": {
                        "type": "string",
                        "description": "Detailed description of what will be implemented and why. Include context and goals."
                    },
                    "steps": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "List of implementation steps in order. Each step should be specific and actionable. Example: ['1. Create auth middleware', '2. Add JWT token generation', '3. Implement login endpoint']"
                    },
                    "files_to_modify": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "List of files that will be created or modified. Example: ['src/auth/middleware.rs', 'src/api/login.rs']"
                    },
                    "risks": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Potential risks or challenges. Example: ['Breaking existing authentication', 'Token expiration edge cases']"
                    }
                },
                "required": ["title", "description", "steps", "files_to_modify"]
            }),
        },
    }
}

/// Ask user tool definition
fn ask_user_tool() -> Tool {
    Tool {
        tool_type: "function".to_string(),
        function: FunctionDefinition {
            name: "ask_user".to_string(),
            description: "Ask the user questions with multiple choice options (1-4 questions). Each question can have 2-4 options. Supports single or multi-select. 'Other' option is automatically added for free text input. Use this to: gather user preferences, clarify implementation choices, get decisions, offer multiple approaches.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "questions": {
                        "type": "array",
                        "description": "List of questions to ask (1-4 questions)",
                        "items": {
                            "type": "object",
                            "properties": {
                                "question": {
                                    "type": "string",
                                    "description": "The question to ask. Should be clear and specific. Example: 'Which library should we use for date formatting?'"
                                },
                                "header": {
                                    "type": "string",
                                    "description": "Very short label (max 12 chars). Examples: 'Auth method', 'Library', 'Approach'"
                                },
                                "options": {
                                    "type": "array",
                                    "description": "Available choices (2-4 options). Each should be distinct and mutually exclusive unless multiSelect is true",
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "label": {
                                                "type": "string",
                                                "description": "Display text (1-5 words). Example: 'OAuth 2.0', 'JWT tokens', 'Session cookies'"
                                            },
                                            "description": {
                                                "type": "string",
                                                "description": "Explanation of this option and its implications"
                                            }
                                        },
                                        "required": ["label", "description"]
                                    },
                                    "minItems": 2,
                                    "maxItems": 4
                                },
                                "multiSelect": {
                                    "type": "boolean",
                                    "description": "Allow multiple selections? Default: false. Use when choices are not mutually exclusive."
                                }
                            },
                            "required": ["question", "header", "options", "multiSelect"]
                        },
                        "minItems": 1,
                        "maxItems": 4
                    }
                },
                "required": ["questions"]
            }),
        },
    }
}

/// Web fetch tool definition
fn web_fetch_tool() -> Tool {
    Tool {
        tool_type: "function".to_string(),
        function: FunctionDefinition {
            name: "web_fetch".to_string(),
            description: "Fetch content from a URL and extract information. Use this to read documentation, API responses, or web pages.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "The URL to fetch (must be a valid HTTP/HTTPS URL)"
                    },
                    "prompt": {
                        "type": "string",
                        "description": "What information to extract from the page"
                    }
                },
                "required": ["url", "prompt"]
            }),
        },
    }
}

/// Git diff tool definition
fn git_diff_tool() -> Tool {
    Tool {
        tool_type: "function".to_string(),
        function: FunctionDefinition {
            name: "git_diff".to_string(),
            description: "Get ALL uncommitted changes in ONE call. When user asks to 'review diff' or 'show changes', THIS IS THE ONLY TOOL YOU NEED. CRITICAL RULES: (1) Do NOT use bash for git log, git show, or git diff. (2) This tool returns EVERYTHING: staged changes, unstaged changes, file statistics, complete unified diff. (3) When asked 'diffをレビューして' or 'review my changes', call ONLY this tool - nothing else needed. (4) Do NOT manually inspect git history with bash - this tool already gives you the complete picture. ONE CALL = COMPLETE DIFF.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Optional: specific file to show diff for. Leave empty to see all changes."
                    }
                },
                "required": []
            }),
        },
    }
}

/// Git commit tool definition
fn git_commit_tool() -> Tool {
    Tool {
        tool_type: "function".to_string(),
        function: FunctionDefinition {
            name: "git_commit".to_string(),
            description: "Commit changes to git with a message. Use this after making changes to save them. IMPORTANT: Always ask user before committing.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "message": {
                        "type": "string",
                        "description": "The commit message describing the changes"
                    },
                    "files": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Optional: specific files to commit (if not provided, commits all changes)"
                    }
                },
                "required": ["message"]
            }),
        },
    }
}

/// Git PR create tool definition
fn git_pr_create_tool() -> Tool {
    Tool {
        tool_type: "function".to_string(),
        function: FunctionDefinition {
            name: "git_pr_create".to_string(),
            description: "Create a pull request using GitHub CLI (gh). IMPORTANT: Always analyze all commits in the current branch (from divergence point) before creating PR. Use 'git log <base-branch>...HEAD' and 'git diff <base-branch>...HEAD' to understand full changes. The PR summary should reflect ALL commits, not just the latest one.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "title": {
                        "type": "string",
                        "description": "The pull request title. Should summarize all changes in the branch."
                    },
                    "body": {
                        "type": "string",
                        "description": "The pull request body/description. Should include: ## Summary (1-3 bullet points of all changes), ## Test plan (checklist of TODOs for testing). Use markdown formatting."
                    },
                    "base": {
                        "type": "string",
                        "description": "Optional: The base branch to merge into (default: main/master). Examples: 'main', 'develop'"
                    },
                    "draft": {
                        "type": "boolean",
                        "description": "Optional: Create as draft PR? Default: false"
                    }
                },
                "required": ["title", "body"]
            }),
        },
    }
}

/// Web search tool definition
fn web_search_tool() -> Tool {
    Tool {
        tool_type: "function".to_string(),
        function: FunctionDefinition {
            name: "web_search".to_string(),
            description: "Search the web for information using a search query. Use this to find current information, documentation, or answers to questions.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search query (e.g., 'Rust async programming tutorial 2025')"
                    }
                },
                "required": ["query"]
            }),
        },
    }
}

/// Notebook edit tool definition
fn notebook_edit_tool() -> Tool {
    Tool {
        tool_type: "function".to_string(),
        function: FunctionDefinition {
            name: "notebook_edit".to_string(),
            description: "Edit a Jupyter notebook (.ipynb) cell. Use this to modify code or markdown cells in notebooks.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "notebook_path": {
                        "type": "string",
                        "description": "Path to the .ipynb file"
                    },
                    "cell_index": {
                        "type": "number",
                        "description": "Index of the cell to edit (0-based)"
                    },
                    "new_content": {
                        "type": "string",
                        "description": "New content for the cell"
                    },
                    "cell_type": {
                        "type": "string",
                        "description": "Cell type: 'code' or 'markdown' (optional, defaults to existing type)"
                    }
                },
                "required": ["notebook_path", "cell_index", "new_content"]
            }),
        },
    }
}

/// Create artifact tool definition - Live preview for web content
fn create_artifact_tool() -> Tool {
    Tool {
        tool_type: "function".to_string(),
        function: FunctionDefinition {
            name: "create_artifact".to_string(),
            description: "Create a web artifact (HTML/JS/CSS/React/Vue) for live preview. The content will be displayed in a local preview server at http://localhost:3456 with automatic browser opening and hot reload. Perfect for: creating web UIs, interactive demos, data visualizations, games, forms, etc. Supports vanilla HTML/CSS/JS, React, Vue, and any web technology. The preview updates instantly when you modify the code.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "title": {
                        "type": "string",
                        "description": "Title of the artifact (e.g., 'Todo App', 'Calculator', 'Landing Page')"
                    },
                    "content": {
                        "type": "string",
                        "description": "Complete HTML content including <!DOCTYPE html>, <html>, <head>, <body> tags. Can include inline CSS in <style> tags and inline JavaScript in <script> tags. For React/Vue, include the library from CDN and write the component inline."
                    },
                    "type": {
                        "type": "string",
                        "description": "Type of artifact: 'html' (vanilla HTML/CSS/JS), 'react' (React with JSX via Babel), 'vue' (Vue.js), 'markdown' (rendered Markdown). Default: 'html'",
                        "enum": ["html", "react", "vue", "markdown"]
                    }
                },
                "required": ["title", "content"]
            }),
        },
    }
}

/// LSP Go to Definition tool
fn lsp_goto_definition_tool() -> Tool {
    Tool {
        tool_type: "function".to_string(),
        function: FunctionDefinition {
            name: "lsp_goto_definition".to_string(),
            description: "🚀 GOD MODE: Use LSP (Language Server Protocol) to find where a symbol is DEFINED. This is 100x faster and more accurate than grep! Works for Rust, TypeScript, Python, Go. Returns the EXACT file and line number. No hallucinations - only real definitions. Example: Find where the User struct is defined? One LSP call vs 15 grep calls!".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "The file containing the symbol (relative to project root). Example: 'src/models.rs'"
                    },
                    "line": {
                        "type": "number",
                        "description": "Line number where the symbol appears (0-based)"
                    },
                    "character": {
                        "type": "number",
                        "description": "Character position on the line (0-based)"
                    },
                    "symbol": {
                        "type": "string",
                        "description": "The symbol name you're looking for (for logging purposes)"
                    }
                },
                "required": ["file_path", "line", "character"]
            }),
        },
    }
}

/// LSP Find References tool
fn lsp_find_references_tool() -> Tool {
    Tool {
        tool_type: "function".to_string(),
        function: FunctionDefinition {
            name: "lsp_find_references".to_string(),
            description: "🚀 GOD MODE: Use LSP to find ALL places where a symbol is USED. Instant and accurate - no grep needed! Returns all usage locations with file paths and line numbers. Perfect for: finding all callers of a function, all usages of a variable, impact analysis before refactoring.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "The file containing the symbol (relative to project root)"
                    },
                    "line": {
                        "type": "number",
                        "description": "Line number where the symbol appears (0-based)"
                    },
                    "character": {
                        "type": "number",
                        "description": "Character position on the line (0-based)"
                    },
                    "symbol": {
                        "type": "string",
                        "description": "The symbol name you're looking for (for logging purposes)"
                    }
                },
                "required": ["file_path", "line", "character"]
            }),
        },
    }
}

/// LSP Hover tool
fn lsp_hover_tool() -> Tool {
    Tool {
        tool_type: "function".to_string(),
        function: FunctionDefinition {
            name: "lsp_hover".to_string(),
            description: "🚀 GOD MODE: Use LSP to get TYPE INFORMATION and DOCUMENTATION for a symbol. This is like hovering over code in VS Code - you get the type signature, function signature, and inline docs. Perfect for understanding unfamiliar code without reading the entire file.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "The file containing the symbol (relative to project root)"
                    },
                    "line": {
                        "type": "number",
                        "description": "Line number where the symbol appears (0-based)"
                    },
                    "character": {
                        "type": "number",
                        "description": "Character position on the line (0-based)"
                    },
                    "symbol": {
                        "type": "string",
                        "description": "The symbol name you're querying (for logging purposes)"
                    }
                },
                "required": ["file_path", "line", "character"]
            }),
        },
    }
}

/// Execute a tool call with enhanced error handling and suggestions
pub fn execute_tool(tool_call: &ToolCall, project_root: &Path) -> Result<String> {
    let args: Value = serde_json::from_str(&tool_call.function.arguments)
        .map_err(|e| anyhow!("Failed to parse tool arguments: {}. The arguments should be valid JSON. Check the tool definition for required parameter format.", e))?;

    let result = match tool_call.function.name.as_str() {
        "read_file" => execute_read_file(&args, project_root),
        "write_file" => execute_write_file(&args, project_root),
        "edit_file" => execute_edit_file(&args, project_root),
        "translate_file" => execute_translate_file(&args, project_root),
        "list_files" => execute_list_files(&args, project_root),
        "file_tree" => execute_file_tree(&args, project_root),
        "glob" | "search_files" => execute_search_files(&args, project_root), // Support both names
        "grep" => execute_grep(&args, project_root),
        "bash" => execute_bash(&args, project_root),
        "lint_code" => execute_lint_code(&args, project_root),
        #[cfg(feature = "browser")]
        "browser_visit" => execute_browser_visit(&args),
        #[cfg(feature = "browser")]
        "browser_read" => execute_browser_read(&args),
        "ask_user" => execute_ask_user(&args),
        "propose_plan" => execute_propose_plan(&args),
        "web_fetch" => execute_web_fetch(&args),
        "web_search" => execute_web_search(&args),
        "git_diff" => execute_git_diff(&args, project_root),
        "git_commit" => execute_git_commit(&args, project_root),
        "git_pr_create" => execute_git_pr_create(&args, project_root),
        "notebook_edit" => execute_notebook_edit(&args, project_root),
        "create_artifact" => execute_create_artifact(&args, project_root),
        "lsp_goto_definition" => execute_lsp_goto_definition(&args, project_root),
        "lsp_find_references" => execute_lsp_find_references(&args, project_root),
        "lsp_hover" => execute_lsp_hover(&args, project_root),
        "self_healing" => execute_self_healing(&args, project_root),
        "vision_analyze" => execute_vision_analyze(&args, project_root),
        "knowledge_graph" => execute_knowledge_graph(&args, project_root),
        "hive_mind" => execute_hive_mind(&args, project_root),
        "semantic_search" => execute_semantic_search(&args, project_root),
        _ => Err(anyhow!("Unknown tool: {}. Available tools are: read_file, write_file, edit_file, list_files, glob, grep, bash, ask_user, propose_plan, web_fetch, web_search, git_diff, git_commit, git_pr_create, notebook_edit, create_artifact, lsp_goto_definition, lsp_find_references, lsp_hover, self_healing, vision_analyze, knowledge_graph, hive_mind, semantic_search", tool_call.function.name)),
    };

    // Add context-aware suggestions on errors
    if let Err(e) = &result {
        let error_msg = format!("{}", e);
        let enhanced_error = enhance_error_message(&tool_call.function.name, &error_msg, &args, project_root);
        return Err(anyhow!("{}", enhanced_error));
    }

    result
}

/// Enhance error messages with context-aware suggestions
fn enhance_error_message(tool_name: &str, error: &str, args: &Value, _project_root: &Path) -> String {
    let mut suggestions = Vec::new();

    match tool_name {
        "read_file" => {
            if error.contains("not found") || error.contains("No such file") {
                suggestions.push("💡 ファイルが見つかりません。以下を試してください:".to_string());
                suggestions.push("  1. list_files()でディレクトリ構造を確認".to_string());
                suggestions.push("  2. search_files()でファイル名を検索".to_string());
                suggestions.push("  3. ファイルパスのスペルを確認".to_string());

                if let Some(path) = args.get("file_path").and_then(|p| p.as_str()) {
                    let dir = Path::new(path).parent().and_then(|p| p.to_str()).unwrap_or(".");
                    suggestions.push(format!("  4. list_files({{\"directory\": \"{}\"}})で確認", dir));
                }
            } else if error.contains("permission") {
                suggestions.push("💡 権限エラー: ファイルの読み取り権限を確認してください".to_string());
            }
        }
        "edit_file" => {
            if error.contains("not found") {
                suggestions.push("💡 検索テキストが見つかりません:".to_string());
                suggestions.push("  1. read_file()でファイル内容を再確認".to_string());
                suggestions.push("  2. 検索テキストが完全に一致しているか確認（スペース、改行、インデントを含む）".to_string());
                suggestions.push("  3. 検索テキストにより多くのコンテキストを含める".to_string());
            } else if error.contains("appears") && error.contains("times") {
                suggestions.push("💡 検索テキストが複数箇所に存在します:".to_string());
                suggestions.push("  1. より具体的な検索テキストを使用（前後の行を含める）".to_string());
                suggestions.push("  2. 一意になるまで検索範囲を広げる".to_string());
                suggestions.push("  3. 代わりにwrite_file()でファイル全体を書き換える".to_string());
            } else if error.contains("File not found") {
                suggestions.push("💡 ファイルが存在しません:".to_string());
                suggestions.push("  1. write_file()で新しいファイルを作成".to_string());
                suggestions.push("  2. list_files()でファイルパスを確認".to_string());
            }
        }
        "write_file" => {
            if error.contains("permission") || error.contains("denied") {
                suggestions.push("💡 書き込み権限がありません:".to_string());
                suggestions.push("  1. ディレクトリの権限を確認".to_string());
                suggestions.push("  2. 親ディレクトリが存在するか確認".to_string());
            }
        }
        "grep" => {
            if error.contains("Invalid regex") {
                suggestions.push("💡 正規表現エラー:".to_string());
                suggestions.push("  1. 特殊文字をエスケープ（例: \\\\., \\\\*, \\\\?）".to_string());
                suggestions.push("  2. より単純なパターンを試す".to_string());
                suggestions.push("  3. 正規表現をテストしてから使用".to_string());
            } else if error.contains("No matches") {
                suggestions.push("💡 パターンが見つかりません:".to_string());
                suggestions.push("  1. より広い検索パターンを使用".to_string());
                suggestions.push("  2. 大文字小文字を区別しない検索を検討".to_string());
                suggestions.push("  3. file_patternを確認（例: '*.rs', '**/*.js'）".to_string());
            }
        }
        "bash" => {
            if error.contains("Exit code") && !error.contains("Exit code: 0") {
                suggestions.push("💡 コマンド実行が失敗しました:".to_string());
                suggestions.push("  1. STDERRの出力を確認".to_string());
                suggestions.push("  2. コマンドの引数を確認".to_string());
                suggestions.push("  3. 必要な依存関係がインストールされているか確認".to_string());
                suggestions.push("  4. bash({{\"command\": \"which command_name\"}})で実行可能か確認".to_string());
            }
        }
        "list_files" => {
            if error.contains("not found") {
                suggestions.push("💡 ディレクトリが見つかりません:".to_string());
                suggestions.push("  1. 親ディレクトリから探索を開始".to_string());
                suggestions.push("  2. list_files()でルートから確認".to_string());
            }
        }
        _ => {}
    }

    if suggestions.is_empty() {
        error.to_string()
    } else {
        format!("{}\n\n{}", error, suggestions.join("\n"))
    }
}

/// Execute read_file tool
fn execute_read_file(args: &Value, project_root: &Path) -> Result<String> {
    use crate::berrycode::display::DisplayManager;

    let file_path = args["file_path"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing file_path parameter"))?;

    let full_path = project_root.join(file_path);

    if !full_path.exists() {
        return Ok(format!("Error: File not found: {}", file_path));
    }

    // アクションログを表示
    let display = DisplayManager::new();
    display.log_read(file_path);

    // Check cache first
    if let Some(cached_content) = FILE_CACHE.get(&full_path) {
        let line_count = cached_content.lines().count();

        // サブ結果を表示（Claude Code風）
        display.print_sub_result(&format!("Read {} lines", line_count), true);

        let mut result = format!("File: {} (cached)\nLines: {}\n\n{}", file_path, line_count, cached_content);

        // Add dependency information
        if let Ok(related) = crate::berrycode::dependency_tracker::DependencyTracker::get_related_files(&full_path, project_root) {
            let context_msg = crate::berrycode::dependency_tracker::DependencyTracker::generate_context_message(&related, project_root);
            result.push_str(&context_msg);
        }

        return Ok(result);
    }

    // Read from file system
    match fs::read_to_string(&full_path) {
        Ok(content) => {
            let line_count = content.lines().count();

            // Cache the content
            FILE_CACHE.put(full_path.clone(), content.clone());

            // サブ結果を表示（Claude Code風）
            display.print_sub_result(&format!("Read {} lines", line_count), true);

            let mut result = format!("File: {}\nLines: {}\n\n{}", file_path, line_count, content);

            // Add dependency information
            if let Ok(related) = crate::berrycode::dependency_tracker::DependencyTracker::get_related_files(&full_path, project_root) {
                let context_msg = crate::berrycode::dependency_tracker::DependencyTracker::generate_context_message(&related, project_root);
                result.push_str(&context_msg);
            }

            Ok(result)
        }
        Err(e) => Ok(format!("Error reading file: {}", e)),
    }
}

/// Execute list_files tool
fn execute_list_files(args: &Value, project_root: &Path) -> Result<String> {
    let directory = args["directory"]
        .as_str()
        .unwrap_or(".");

    let full_path = project_root.join(directory);

    if !full_path.exists() {
        return Ok(format!("Error: Directory not found: {}", directory));
    }

    let mut entries = Vec::new();

    if let Ok(dir_entries) = fs::read_dir(&full_path) {
        for entry in dir_entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                let name = entry.file_name().to_string_lossy().to_string();
                let file_type = if metadata.is_dir() { "DIR" } else { "FILE" };
                let size = if metadata.is_file() {
                    format!("{} bytes", metadata.len())
                } else {
                    String::new()
                };
                entries.push(format!("{:5} {} {}", file_type, name, size));
            }
        }
    }

    entries.sort();

    Ok(format!(
        "Directory: {}\n\n{}",
        directory,
        entries.join("\n")
    ))
}

/// Execute file_tree tool - shows directory structure
fn execute_file_tree(args: &Value, project_root: &Path) -> Result<String> {
    let max_depth = args.get("max_depth")
        .and_then(|v| v.as_u64())
        .unwrap_or(3) as usize;

    // Common directories to ignore
    let ignore_patterns = vec![
        "target", "node_modules", ".git", "dist", "build",
        ".next", ".cache", "vendor", "__pycache__", ".pytest_cache"
    ];

    let mut output = String::from("📁 Project Structure:\n\n");
    output.push_str(&format!("{}/\n", project_root.file_name()
        .unwrap_or_else(|| std::ffi::OsStr::new("project"))
        .to_string_lossy()));

    build_tree(project_root, "", 0, max_depth, &ignore_patterns, &mut output)?;

    Ok(output)
}

/// Recursively build tree structure
fn build_tree(
    path: &Path,
    prefix: &str,
    depth: usize,
    max_depth: usize,
    ignore_patterns: &[&str],
    output: &mut String,
) -> Result<()> {
    if depth >= max_depth {
        return Ok(());
    }

    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => return Ok(()), // Skip directories we can't read
    };

    let mut items: Vec<_> = entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            // Skip hidden files and ignored directories
            !name.starts_with('.') && !ignore_patterns.contains(&name.as_str())
        })
        .collect();

    items.sort_by_key(|e| e.file_name());

    for (i, entry) in items.iter().enumerate() {
        let is_last = i == items.len() - 1;
        let connector = if is_last { "└──" } else { "├──" };
        let extension = if is_last { "    " } else { "│   " };

        let name = entry.file_name().to_string_lossy().to_string();
        let metadata = entry.metadata()?;

        if metadata.is_dir() {
            output.push_str(&format!("{}{} {}/\n", prefix, connector, name));
            let new_prefix = format!("{}{}", prefix, extension);
            build_tree(&entry.path(), &new_prefix, depth + 1, max_depth, ignore_patterns, output)?;
        } else {
            output.push_str(&format!("{}{} {}\n", prefix, connector, name));
        }
    }

    Ok(())
}

/// Execute search_files tool (glob pattern)
fn execute_search_files(args: &Value, project_root: &Path) -> Result<String> {
    use std::time::SystemTime;
    use crate::berrycode::display::DisplayManager;

    let pattern = args["pattern"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing pattern parameter"))?;

    // Optional path parameter to search in a specific directory
    let search_path = if let Some(path_str) = args["path"].as_str() {
        project_root.join(path_str)
    } else {
        project_root.to_path_buf()
    };

    // アクションログを表示
    let display = DisplayManager::new();
    display.log_search(pattern, "**/*");

    let glob_pattern = search_path.join(pattern).to_string_lossy().to_string();

    let mut matches: Vec<(PathBuf, SystemTime)> = Vec::new();

    if let Ok(paths) = glob::glob(&glob_pattern) {
        for path in paths.flatten() {
            if path.is_file() {
                // Get modification time
                if let Ok(metadata) = fs::metadata(&path) {
                    if let Ok(modified) = metadata.modified() {
                        matches.push((path, modified));
                    }
                }
            }
        }
    }

    if matches.is_empty() {
        display.print_sub_result("No files found", false);
        Ok(format!("No files found matching pattern: {}", pattern))
    } else {
        // Sort by modification time (most recent first)
        matches.sort_by(|a, b| b.1.cmp(&a.1));

        // サブ結果を表示（Claude Code風）
        display.print_sub_result(&format!("Found {} files", matches.len()), true);

        // Format output with relative paths
        let formatted_matches: Vec<String> = matches
            .iter()
            .map(|(path, _)| {
                path.strip_prefix(project_root)
                    .unwrap_or(path)
                    .to_string_lossy()
                    .to_string()
            })
            .collect();

        Ok(format!(
            "Found {} files matching '{}' (sorted by modification time, most recent first):\n\n{}",
            formatted_matches.len(),
            pattern,
            formatted_matches.join("\n")
        ))
    }
}

/// Execute grep tool
fn execute_grep(args: &Value, project_root: &Path) -> Result<String> {
    use crate::berrycode::display::DisplayManager;

    let pattern = args["pattern"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing pattern parameter"))?;

    let file_pattern = args["file_pattern"].as_str().unwrap_or("**/*");

    // アクションログを表示
    let display = DisplayManager::new();
    display.log_search(pattern, file_pattern);

    // Get context_lines parameter (default: 3)
    let context_lines = args.get("context_lines")
        .and_then(|v| v.as_u64())
        .unwrap_or(3) as usize;

    let glob_pattern = project_root.join(file_pattern).to_string_lossy().to_string();

    let regex = regex::Regex::new(pattern)
        .map_err(|e| anyhow!("Invalid regex pattern: {}", e))?;

    let mut results = Vec::new();

    if let Ok(paths) = glob::glob(&glob_pattern) {
        for path in paths.flatten() {
            if path.is_file() {
                if let Ok(content) = fs::read_to_string(&path) {
                    let lines: Vec<&str> = content.lines().collect();
                    let mut file_matches = Vec::new();
                    let mut matched_line_nums = Vec::new();

                    // First pass: find all matching lines
                    for (line_num, line) in lines.iter().enumerate() {
                        if regex.is_match(line) {
                            matched_line_nums.push(line_num);
                        }
                    }

                    // Second pass: collect matches with context
                    if !matched_line_nums.is_empty() {
                        let relative_path = path.strip_prefix(project_root)
                            .unwrap_or(&path)
                            .display()
                            .to_string();

                        for &match_line_num in &matched_line_nums {
                            // Calculate context range
                            let start = match_line_num.saturating_sub(context_lines);
                            let end = (match_line_num + context_lines + 1).min(lines.len());

                            // Add separator between matches
                            if !file_matches.is_empty() {
                                file_matches.push("  --".to_string());
                            }

                            // Add context lines
                            for i in start..end {
                                let marker = if i == match_line_num { ">" } else { " " };
                                file_matches.push(format!("  {}{}: {}",
                                    marker,
                                    i + 1,
                                    lines[i]
                                ));
                            }
                        }

                        // Add file header
                        results.push(format!("\n{}:\n{}", relative_path, file_matches.join("\n")));
                    }
                }
            }
        }
    }

    if results.is_empty() {
        display.print_sub_result("No matches found", false);
        Ok(format!("No matches found for pattern: {}", pattern))
    } else {
        // Get head_limit parameter (default: 30 to prevent payload overflow)
        let head_limit = args.get("head_limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(30) as usize;

        // Limit results to prevent 413 Payload Too Large errors
        let total_matches = results.len();
        let limited_results: Vec<String> = results.into_iter().take(head_limit).collect();
        let was_limited = total_matches > head_limit;

        // サブ結果を表示（Claude Code風: "Found X lines"）
        let summary = if was_limited {
            format!("Found {} lines (showing first {})", total_matches, head_limit)
        } else {
            let line_word = if total_matches == 1 { "line" } else { "lines" };
            format!("Found {} {}", total_matches, line_word)
        };
        display.print_sub_result(&summary, true);

        let result_text = limited_results.join("\n");
        let warning = if was_limited {
            format!("\n\n⚠️  Showing only first {} of {} matches. Use more specific pattern or increase head_limit.",
                head_limit, total_matches)
        } else {
            String::new()
        };

        Ok(format!(
            "Found {} matches for '{}' :\n\n{}{}",
            total_matches,
            pattern,
            result_text,
            warning
        ))
    }
}

/// Execute write_file tool
fn execute_write_file(args: &Value, project_root: &Path) -> Result<String> {
    use crate::berrycode::display::DisplayManager;

    let file_path = args["file_path"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing file_path parameter"))?;

    let content = args["content"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing content parameter"))?;

    let full_path = project_root.join(file_path);

    // 1. 古い内容を読んでおく（Diff用）
    let old_content = fs::read_to_string(&full_path).unwrap_or_default();

    // 2. アクションログを表示（Claude Code風）
    let display = DisplayManager::new();
    display.log_update(file_path);

    // Create parent directories if they don't exist
    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // 3. ファイル書き込み
    fs::write(&full_path, content)?;

    // 4. 🎨 美しいDiff表示（Claude Code風 - context lines付き）
    use similar::{ChangeTag, TextDiff};

    let diff = TextDiff::from_lines(old_content.as_str(), content);
    let mut additions = 0;
    let mut removals = 0;

    // Count changes first
    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Insert => additions += 1,
            ChangeTag::Delete => removals += 1,
            _ => {}
        }
    }

    // Display diff with context (前後3行)
    let context_lines = 3;
    for group in diff.grouped_ops(context_lines).iter() {
        let mut old_line = group.as_slice().first().unwrap().old_range().start;
        let mut new_line = group.as_slice().first().unwrap().new_range().start;

        for op in group {
            for change in diff.iter_changes(op) {
                let line_num = match change.tag() {
                    ChangeTag::Delete => {
                        let num = old_line + 1;
                        old_line += 1;
                        num
                    }
                    ChangeTag::Insert => {
                        let num = new_line + 1;
                        new_line += 1;
                        num
                    }
                    ChangeTag::Equal => {
                        let num = new_line + 1;
                        old_line += 1;
                        new_line += 1;
                        num
                    }
                };

                let tag = match change.tag() {
                    ChangeTag::Delete => Some("-"),
                    ChangeTag::Insert => Some("+"),
                    ChangeTag::Equal => None,
                };

                display.print_diff_line(line_num, tag, change.value().trim_end());
            }
        }
    }

    // サマリーを表示（Claude Code風）
    display.print_diff_summary(file_path, additions, removals);

    // 検証: ファイルが実際に存在することを確認
    let file_size = fs::metadata(&full_path)?.len();
    let verified = if full_path.exists() {
        "✓ File verified to exist"
    } else {
        "⚠ WARNING: File may not exist!"
    };

    Ok(format!(
        "Successfully wrote {} bytes to {}\n{}\nFile size on disk: {} bytes",
        content.len(),
        file_path,
        verified,
        file_size
    ))
}

/// Execute edit_file tool
fn execute_edit_file(args: &Value, project_root: &Path) -> Result<String> {
    use crate::berrycode::display::DisplayManager;

    let file_path = args["file_path"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing file_path parameter"))?;

    let search = args["search"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing search parameter"))?;

    let replace = args["replace"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing replace parameter"))?;

    let full_path = project_root.join(file_path);

    if !full_path.exists() {
        return Ok(format!("Error: File not found: {}", file_path));
    }

    // 1. 古い内容を読み取る
    let old_content = fs::read_to_string(&full_path)?;

    // Check if search text exists
    if !old_content.contains(search) {
        return Ok(format!("Error: Search text not found in {}", file_path));
    }

    // Count occurrences
    let count = old_content.matches(search).count();
    if count > 1 {
        return Ok(format!("Error: Search text appears {} times in {}. Please be more specific.", count, file_path));
    }

    // 2. アクションログを表示（Claude Code風）
    let display = DisplayManager::new();
    display.log_update(file_path);

    // 3. Perform replacement
    let new_content = old_content.replace(search, replace);
    fs::write(&full_path, &new_content)?;

    // 4. 🎨 美しいDiff表示（Claude Code風 - context lines付き）
    use similar::{ChangeTag, TextDiff};

    let diff = TextDiff::from_lines(old_content.as_str(), new_content.as_str());
    let mut additions = 0;
    let mut removals = 0;

    // Count changes first
    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Insert => additions += 1,
            ChangeTag::Delete => removals += 1,
            _ => {}
        }
    }

    // Display diff with context (前後3行)
    let context_lines = 3;
    for group in diff.grouped_ops(context_lines).iter() {
        let mut old_line = group.as_slice().first().unwrap().old_range().start;
        let mut new_line = group.as_slice().first().unwrap().new_range().start;

        for op in group {
            for change in diff.iter_changes(op) {
                let line_num = match change.tag() {
                    ChangeTag::Delete => {
                        let num = old_line + 1;
                        old_line += 1;
                        num
                    }
                    ChangeTag::Insert => {
                        let num = new_line + 1;
                        new_line += 1;
                        num
                    }
                    ChangeTag::Equal => {
                        let num = new_line + 1;
                        old_line += 1;
                        new_line += 1;
                        num
                    }
                };

                let tag = match change.tag() {
                    ChangeTag::Delete => Some("-"),
                    ChangeTag::Insert => Some("+"),
                    ChangeTag::Equal => None,
                };

                display.print_diff_line(line_num, tag, change.value().trim_end());
            }
        }
    }

    // サマリーを表示（Claude Code風）
    display.print_diff_summary(file_path, additions, removals);

    // 検証: ファイルが実際に更新されたことを確認
    let file_size = fs::metadata(&full_path)?.len();
    let new_file_content = fs::read_to_string(&full_path)?;
    let verified = if new_file_content == new_content {
        "✓ Edit verified successfully"
    } else {
        "⚠ WARNING: File content mismatch!"
    };

    Ok(format!(
        "Successfully edited {} (replaced 1 occurrence)\n{}\nNew file size: {} bytes",
        file_path,
        verified,
        file_size
    ))
}

/// Execute translate_file tool
fn execute_translate_file(args: &Value, project_root: &Path) -> Result<String> {
    use crate::berrycode::display::DisplayManager;

    let file_path = args["file_path"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing file_path parameter"))?;

    let target_language = args["target_language"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing target_language parameter"))?;

    let language_code = args["language_code"]
        .as_str()
        .unwrap_or_else(|| match target_language.to_lowercase().as_str() {
            "japanese" | "日本語" => "ja",
            "english" | "英語" => "en",
            "spanish" | "スペイン語" => "es",
            "french" | "フランス語" => "fr",
            "chinese" | "中国語" => "zh",
            "german" | "ドイツ語" => "de",
            "korean" | "韓国語" => "ko",
            _ => "translated",
        });

    let full_path = project_root.join(file_path);

    if !full_path.exists() {
        return Ok(format!("Error: File not found: {}", file_path));
    }

    // 1. ファイルを読む（チャットには展開しない！）
    let content = fs::read_to_string(&full_path)
        .map_err(|e| anyhow!("Failed to read {}: {}", file_path, e))?;

    // 2. アクションログを表示
    let display = DisplayManager::new();
    display.log_update(&format!("Translating {} to {}...", file_path, target_language));

    // 3. LLM で翻訳（非同期呼び出しを同期化）
    let translated = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async {
            use crate::berrycode::models::Model;

            // 環境変数から API キーを取得
            let api_key = std::env::var("ANTHROPIC_API_KEY")
                .or_else(|_| std::env::var("OPENAI_API_KEY"))
                .map_err(|_| anyhow!("Missing ANTHROPIC_API_KEY or OPENAI_API_KEY environment variable"))?;

            // Model を作成（デフォルトは qwen3-coder:30b）
            let model_name = std::env::var("BERRYCODE_MODEL")
                .unwrap_or_else(|_| "qwen3-coder:30b".to_string());
            let model = Model::new(model_name, None, None, None, false)?;

            // LLM クライアントを作成
            let llm = LLMClient::new(&model, api_key)?;

            // 翻訳プロンプト
            let prompt = format!(
                "Translate the following content into {}. \n\
                CRITICAL RULES:\n\
                - Keep ALL formatting intact (markdown headers, code blocks, links, lists)\n\
                - Do NOT translate code blocks or technical terms\n\
                - Do NOT add any explanation or commentary\n\
                - Output ONLY the translated content\n\
                - Preserve line breaks and structure exactly\n\n\
                Content to translate:\n\n{}",
                target_language, content
            );

            // メッセージを作成
            let messages = vec![Message {
                role: "user".to_string(),
                content: Some(prompt),
                tool_calls: None,
                tool_call_id: None,
            }];

            // LLM を呼ぶ（ツールなし）
            let (response, _input_tokens, _output_tokens) = llm.chat_with_tools(messages, vec![]).await?;

            match response {
                crate::berrycode::llm::LLMResponse::Text(text) => Ok(text),
                _ => Err(anyhow!("Unexpected LLM response format")),
            }
        })
    })?;

    // 4. 新しいファイル名を作成
    let path_obj = Path::new(file_path);
    let stem = path_obj
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("Invalid file name"))?;
    let ext = path_obj
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("txt");
    let new_filename = format!("{}_{}.{}", stem, language_code, ext);
    let new_path = path_obj
        .parent()
        .map(|p| p.join(&new_filename))
        .unwrap_or_else(|| PathBuf::from(&new_filename));
    let full_new_path = project_root.join(&new_path);

    // 5. 翻訳結果を書き込む
    fs::write(&full_new_path, &translated)
        .map_err(|e| anyhow!("Failed to write translated file: {}", e))?;

    // 6. 結果表示
    display.print_sub_result(&format!(
        "Translated {} ({} chars) → {} ({} chars)",
        file_path,
        content.len(),
        new_path.display(),
        translated.len()
    ), true);

    Ok(format!(
        "✅ Translated {} to {} (Saved as {})",
        file_path, target_language, new_path.display()
    ))
}

/// Execute bash tool
fn execute_bash(args: &Value, project_root: &Path) -> Result<String> {
    let command = args["command"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing command parameter"))?;

    // Detect inefficient git usage and provide strong guidance
    if command.contains("git diff") && (command.contains("HEAD~") || command.contains("HEAD^")) {
        // Check if this is a single comprehensive command or multiple separate ones
        if !command.contains("..") && !command.contains("...") {
            return Ok(format!(
                "⚠️ EFFICIENCY WARNING: You're running git diff on individual commits.\n\n\
                Instead of checking HEAD~1, HEAD~2, etc. separately, use a SINGLE command:\n\
                - To see range: git diff HEAD~5..HEAD\n\
                - To compare branches: git diff origin/main...HEAD\n\n\
                Or better yet, use the git_diff tool which is optimized for this.\n\n\
                Your command was: {}\n\n\
                Do you want to revise your approach?",
                command
            ));
        }
    }

    // Detect git log when reviewing changes - should use git_diff instead
    if (command.contains("git log") || command.contains("git show"))
       && (command.contains("--stat") || command.contains("--oneline")) {
        return Ok(format!(
            "⚠️ STOP: You're using bash to inspect git history for code review.\n\n\
            This is EXACTLY what the git_diff tool is for!\n\n\
            The git_diff tool gives you:\n\
            - Complete diff (staged + unstaged)\n\
            - File statistics\n\
            - All changes in ONE call\n\n\
            Your command was: {}\n\n\
            Please use git_diff tool instead. It's faster and more efficient.",
            command
        ));
    }

    // Execute command with real-time streaming output (Claude Code style)
    run_command_with_stream(command, project_root)
}

/// Execute command with real-time streaming output (Claude Code style)
fn run_command_with_stream(command: &str, working_dir: &Path) -> Result<String> {
    use std::process::{Command, Stdio};
    use std::io::{BufRead, BufReader};
    use colored::Colorize;
    use crate::berrycode::display::DisplayManager;

    const MAX_DISPLAY_LINES: usize = 50; // 50行以上は省略

    // --- 1. アクションログ表示: ● Bash(command) ---
    let display = DisplayManager::new();
    display.log_bash(command);

    // コマンド実行開始
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(working_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let mut output_log = String::new();
    let mut line_count = 0;
    let mut truncated_count = 0;
    let start_time = std::time::Instant::now();

    // 標準出力の処理
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);

        for line in reader.lines() {
            let line = line?;
            output_log.push_str(&line);
            output_log.push('\n');

            line_count += 1;

            if line_count <= MAX_DISPLAY_LINES {
                // 🎨 灰色で左に縦線をつけて表示（Claude Code風）
                println!("{}", format!("  │ {}", line).bright_black());
            } else if line_count == MAX_DISPLAY_LINES + 1 {
                // 「以下省略」メッセージ
                println!("{}", "  │ ... (output truncated for display) ...".bright_black());
            } else {
                truncated_count += 1;
            }
        }
    }

    // 標準エラー出力の処理
    if let Some(stderr) = child.stderr.take() {
        let reader = BufReader::new(stderr);

        for line in reader.lines() {
            let line = line?;
            output_log.push_str("STDERR: ");
            output_log.push_str(&line);
            output_log.push('\n');

            line_count += 1;

            if line_count <= MAX_DISPLAY_LINES {
                println!("{}", format!("  │ {}", line).bright_black());
            } else {
                truncated_count += 1;
            }
        }
    }

    // コマンド終了待ち
    let status = child.wait()?;
    let elapsed = start_time.elapsed();

    // 省略された行数と実行時間を表示
    if truncated_count > 0 {
        println!("{}",
            format!("  +{} more lines ({}s)", truncated_count, elapsed.as_secs())
                .bright_black()
        );
    }

    // 結果のサマリー
    let mut result = String::new();
    result.push_str(&format!("Command: {}\n", command));
    result.push_str(&format!("Exit code: {}\n", status.code().unwrap_or(-1)));
    result.push_str(&format!("Duration: {:.1}s\n\n", elapsed.as_secs_f32()));

    if !output_log.is_empty() {
        result.push_str("Output:\n");
        result.push_str(&output_log);
    }

    // 検証リマインダー: ファイル操作コマンドの場合
    if status.success() && (command.contains("mv ") || command.contains("cp ") || command.contains("rm ")) {
        result.push_str("\n⚠️  VERIFICATION REQUIRED:\n");
        if command.contains("mv ") {
            result.push_str("   You just MOVED files. You MUST verify with 'ls' or 'find' to confirm:\n");
            result.push_str("   1. Files exist in the target directory\n");
            result.push_str("   2. Files are removed from the source location\n");
            result.push_str("   Example: find target_dir -name '*.ext' | wc -l\n");
        } else if command.contains("cp ") {
            result.push_str("   You just COPIED files. You MUST verify with 'ls' or 'find' to confirm:\n");
            result.push_str("   1. Files exist in the target directory\n");
            result.push_str("   2. Files still exist in the source location\n");
        } else if command.contains("rm ") {
            result.push_str("   You just DELETED files. You MUST verify with 'ls' to confirm:\n");
            result.push_str("   1. Files are actually removed\n");
        }
        result.push_str("   DO NOT say 'Done' or '完了' until you run verification commands!\n");
    }

    // 3. エラー時の赤表示（スクリーンショット通り）
    if !status.success() {
        let code = status.code().unwrap_or(-1);

        // ターミナルに赤いエラーメッセージを表示（行動ログ風）
        display.print_sub_result(&format!("{}", format!("Error: Exit code {}", code).red()), false);

        // LLM用の結果にも追記
        result.push_str(&format!("\n⚠️  Command failed with exit code {}", code));
    }

    Ok(result)
}

/// Execute lint_code tool - runs static analysis on the codebase
fn execute_lint_code(args: &Value, project_root: &Path) -> Result<String> {
    use std::process::Command;

    // Determine project type and run appropriate linter
    let strict = args.get("strict").and_then(|v| v.as_bool()).unwrap_or(false);

    // Check for Rust project
    if project_root.join("Cargo.toml").exists() {
        let cmd = if strict { "cargo clippy -- -D warnings" } else { "cargo check" };

        let output = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .current_dir(project_root)
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let status = output.status;

        let mut result = String::new();
        result.push_str(&format!("🔍 Lint Check: {}\n", cmd));
        result.push_str(&format!("Status: {}\n\n", if status.success() { "✓ PASSED" } else { "✗ FAILED" }));

        if !stdout.is_empty() {
            result.push_str(&stdout);
            result.push('\n');
        }

        if !stderr.is_empty() {
            result.push_str(&stderr);
        }

        if !status.success() {
            result.push_str("\n⚠️  Compilation/lint errors detected. Please fix the errors above and run lint_code again.");
        }

        return Ok(result);
    }

    // Check for Python project
    if project_root.join("setup.py").exists() || project_root.join("pyproject.toml").exists() {
        // Try flake8 or pylint
        let output = Command::new("flake8")
            .arg(".")
            .current_dir(project_root)
            .output();

        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut result = String::from("🔍 Lint Check: flake8\n\n".to_string());

            if stdout.is_empty() {
                result.push_str("✓ No linting errors found!");
            } else {
                result.push_str(&stdout);
            }

            return Ok(result);
        }
    }

    // Check for Node.js project
    if project_root.join("package.json").exists() {
        let output = Command::new("npx")
            .args(&["eslint", "."])
            .current_dir(project_root)
            .output();

        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let mut result = String::from("🔍 Lint Check: eslint\n\n".to_string());

            if !stdout.is_empty() {
                result.push_str(&stdout);
            }
            if !stderr.is_empty() {
                result.push_str(&stderr);
            }

            return Ok(result);
        }
    }

    Ok("No linter available for this project type. Supported: Rust (cargo check/clippy), Python (flake8), JavaScript/TypeScript (eslint)".to_string())
}

/// Execute browser_visit tool - Navigate to URL
#[cfg(feature = "browser")]
fn execute_browser_visit(args: &Value) -> Result<String> {
    use headless_chrome::Browser;
    use std::time::Duration;

    let url = args["url"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing url parameter"))?;

    // Launch headless Chrome
    let browser = Browser::default()?;
    let tab = browser.new_tab()?;

    // Navigate to URL with timeout
    tab.navigate_to(url)?;
    tab.wait_until_navigated()?;

    // Wait for page to load
    std::thread::sleep(Duration::from_secs(2));

    // Get page info
    let title = tab.get_title().unwrap_or_else(|_| "Unknown".to_string());
    let current_url = tab.get_url();

    let mut result = String::new();
    result.push_str(&format!("🌐 Browser Navigation Successful\n\n"));
    result.push_str(&format!("URL: {}\n", current_url));
    result.push_str(&format!("Title: {}\n", title));
    result.push_str("\n✓ Page loaded successfully. Use browser_read to extract content.");

    Ok(result)
}

/// Execute browser_read tool - Extract text from webpage
#[cfg(feature = "browser")]
fn execute_browser_read(args: &Value) -> Result<String> {
    use headless_chrome::Browser;

    let url = args["url"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing url parameter"))?;

    let selector = args.get("selector").and_then(|v| v.as_str());

    // Launch headless Chrome
    let browser = Browser::default()?;
    let tab = browser.new_tab()?;

    // Navigate and wait
    tab.navigate_to(url)?;
    tab.wait_until_navigated()?;
    std::thread::sleep(std::time::Duration::from_secs(2));

    // Get HTML content
    let html_content = if let Some(sel) = selector {
        // Get specific element's HTML
        match tab.wait_for_element(sel) {
            Ok(_element) => {
                // Get outerHTML of the element
                let result = tab.evaluate(
                    &format!("document.querySelector('{}').outerHTML", sel),
                    false
                )?;
                result.value.and_then(|v| v.as_str().map(|s| s.to_string()))
                    .unwrap_or_else(|| format!("❌ Failed to get HTML for: {}", sel))
            }
            Err(_) => format!("❌ Element not found: {}", sel),
        }
    } else {
        // Get full page HTML
        tab.get_content()?
    };

    // Convert HTML to Markdown for better readability
    let markdown = html2text::from_read(html_content.as_bytes(), 80);

    let mut result = String::new();
    result.push_str(&format!("📄 Page Content from: {}\n\n", url));

    // Smart truncation: prefer complete sections
    if markdown.len() > 8000 {
        // Take first 8000 chars (more generous than before since Markdown is more concise)
        let truncated = &markdown[..8000];

        // Try to truncate at last complete paragraph
        if let Some(last_newline) = truncated.rfind("\n\n") {
            result.push_str(&truncated[..last_newline]);
        } else {
            result.push_str(truncated);
        }

        result.push_str("\n\n... (content truncated - page is larger)");
    } else {
        result.push_str(&markdown);
    }

    Ok(result)
}

/// Execute propose_plan tool
fn execute_propose_plan(args: &Value) -> Result<String> {
    let title = args["title"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing title parameter"))?;

    let description = args["description"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing description parameter"))?;

    let steps = args["steps"]
        .as_array()
        .ok_or_else(|| anyhow!("Missing steps parameter"))?
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect::<Vec<String>>();

    let files_to_modify = args["files_to_modify"]
        .as_array()
        .ok_or_else(|| anyhow!("Missing files_to_modify parameter"))?
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect::<Vec<String>>();

    let risks = args["risks"]
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect::<Vec<String>>();

    // Return plan as JSON for WebSocket handler to parse
    let plan = serde_json::json!({
        "__plan__": true,
        "title": title,
        "description": description,
        "steps": steps,
        "files_to_modify": files_to_modify,
        "risks": risks,
    });

    Ok(serde_json::to_string(&plan)?)
}

/// Execute ask_user tool
fn execute_ask_user(args: &Value) -> Result<String> {
    use dialoguer::{Select, MultiSelect, Input};

    let questions = args["questions"]
        .as_array()
        .ok_or_else(|| anyhow!("Missing questions parameter"))?;

    let mut results = Vec::new();

    for (q_idx, question_obj) in questions.iter().enumerate() {
        let question = question_obj["question"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing question text in question {}", q_idx + 1))?;

        let header = question_obj["header"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing header in question {}", q_idx + 1))?;

        let options = question_obj["options"]
            .as_array()
            .ok_or_else(|| anyhow!("Missing options in question {}", q_idx + 1))?;

        let multi_select = question_obj["multiSelect"]
            .as_bool()
            .unwrap_or(false);

        // Build option labels (add "Other" option)
        let mut option_labels: Vec<String> = options
            .iter()
            .map(|opt| {
                let label = opt["label"].as_str().unwrap_or("Unknown");
                let desc = opt["description"].as_str().unwrap_or("");
                format!("{} - {}", label, desc)
            })
            .collect();

        option_labels.push("Other (specify custom input)".to_string());

        println!("\n[{}] {}", header, question);

        if multi_select {
            // Multi-select mode
            let selections = MultiSelect::new()
                .with_prompt(question)
                .items(&option_labels)
                .interact()?;

            let mut selected_items = Vec::new();
            for idx in selections {
                if idx == option_labels.len() - 1 {
                    // "Other" selected - get custom input
                    let custom: String = Input::new()
                        .with_prompt("Please enter your custom answer")
                        .interact_text()?;
                    selected_items.push(custom);
                } else {
                    // Extract just the label part (before " - ")
                    let label = options[idx]["label"].as_str().unwrap_or("Unknown");
                    selected_items.push(label.to_string());
                }
            }

            results.push(format!("[{}] {}: {}", header, question, selected_items.join(", ")));
        } else {
            // Single-select mode
            let selection = Select::new()
                .with_prompt(question)
                .items(&option_labels)
                .default(0)
                .interact()?;

            let answer = if selection == option_labels.len() - 1 {
                // "Other" selected - get custom input
                let custom: String = Input::new()
                    .with_prompt("Please enter your custom answer")
                    .interact_text()?;
                custom
            } else {
                // Extract just the label part
                options[selection]["label"].as_str().unwrap_or("Unknown").to_string()
            };

            results.push(format!("[{}] {}: {}", header, question, answer));
        }
    }

    Ok(format!("User answers:\n{}", results.join("\n")))
}

/// Execute web_fetch tool
fn execute_web_fetch(args: &Value) -> Result<String> {
    let url = args["url"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing url parameter"))?;

    let prompt = args["prompt"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing prompt parameter"))?;

    // Fetch URL content
    // Use block_in_place if inside a runtime, otherwise create new runtime
    let content = if tokio::runtime::Handle::try_current().is_ok() {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let client = reqwest::Client::builder()
                    .user_agent("BerryCode/1.0")
                    .timeout(std::time::Duration::from_secs(30))
                    .build()?;

                let response = client.get(url).send().await?;

                if !response.status().is_success() {
                    return Err(anyhow!("HTTP error: {}", response.status()));
                }

                let text = response.text().await?;
                Ok::<String, anyhow::Error>(text)
            })
        })?
    } else {
        let runtime = tokio::runtime::Runtime::new()?;
        runtime.block_on(async {
            let client = reqwest::Client::builder()
                .user_agent("BerryCode/1.0")
                .timeout(std::time::Duration::from_secs(30))
                .build()?;

            let response = client.get(url).send().await?;

            if !response.status().is_success() {
                return Err(anyhow!("HTTP error: {}", response.status()));
            }

            let text = response.text().await?;
            Ok::<String, anyhow::Error>(text)
        })?
    };

    // Simple content extraction - return first 10000 chars with context
    let preview = if content.len() > 10000 {
        format!("{}\n\n... (truncated, total {} bytes)", &content[..10000], content.len())
    } else {
        content
    };

    Ok(format!(
        "URL: {}\nPrompt: {}\n\nContent:\n{}",
        url, prompt, preview
    ))
}

/// Execute git_diff tool
fn execute_git_diff(args: &Value, project_root: &Path) -> Result<String> {
    use crate::berrycode::repo::GitRepo;
    use std::process::Command;

    let file_path = args.get("file_path").and_then(|v| v.as_str());

    // Try to use GitRepo for better diff (includes both staged and unstaged)
    if let Ok(repo) = GitRepo::new(Some(project_root)) {
        match repo.diff() {
            Ok(diff_output) => {
                if diff_output.is_empty() {
                    return Ok("No uncommitted changes".to_string());
                }

                // If file_path is specified, filter the diff
                if let Some(path) = file_path {
                    // Check if the diff contains this file
                    if diff_output.contains(&format!("diff --git a/{}", path)) {
                        // Filter to show only this file's diff
                        let lines: Vec<&str> = diff_output.lines().collect();
                        let mut result = Vec::new();
                        let mut in_file = false;

                        for line in lines {
                            if line.starts_with("diff --git") {
                                in_file = line.contains(&format!("a/{}", path));
                            }
                            if in_file {
                                result.push(line);
                            }
                        }

                        if result.is_empty() {
                            return Ok(format!("No uncommitted changes in {}", path));
                        }

                        return Ok(format!("Git diff for {}:\n\n{}", path, result.join("\n")));
                    } else {
                        return Ok(format!("No uncommitted changes in {}", path));
                    }
                }

                // Return full diff with statistics
                let stats = repo.status()?;
                let changed_files: Vec<_> = stats.iter()
                    .filter(|(_, status)| status != "unknown")
                    .collect();

                let summary = if changed_files.is_empty() {
                    String::new()
                } else {
                    format!("Files changed ({}):\n{}\n\n",
                        changed_files.len(),
                        changed_files.iter()
                            .map(|(path, status)| format!("  {} {}", status, path))
                            .collect::<Vec<_>>()
                            .join("\n")
                    )
                };

                return Ok(format!("{}Git diff:\n\n{}", summary, diff_output));
            }
            Err(e) => {
                tracing::debug!("GitRepo.diff() failed: {}, falling back to git command", e);
                // Fall through to command-line git
            }
        }
    }

    // Fallback to command-line git (legacy behavior)
    let mut cmd = Command::new("git");
    cmd.arg("diff")
        .current_dir(project_root);

    if let Some(path) = file_path {
        cmd.arg(path);
    }

    let output = cmd.output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Ok(format!("Error running git diff: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    if stdout.is_empty() {
        Ok("No uncommitted changes".to_string())
    } else {
        Ok(format!("Git diff:\n\n{}", stdout))
    }
}

/// Execute git_commit tool
fn execute_git_commit(args: &Value, project_root: &Path) -> Result<String> {
    use std::process::Command;

    let message = args["message"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing message parameter"))?;

    let files = args["files"].as_array();

    // Add files to staging
    let mut add_cmd = Command::new("git");
    add_cmd.arg("add").current_dir(project_root);

    if let Some(file_list) = files {
        for file in file_list {
            if let Some(path) = file.as_str() {
                add_cmd.arg(path);
            }
        }
    } else {
        add_cmd.arg("-A");
    }

    let add_output = add_cmd.output()?;
    if !add_output.status.success() {
        let stderr = String::from_utf8_lossy(&add_output.stderr);
        return Ok(format!("Error adding files: {}", stderr));
    }

    // Commit
    let commit_output = Command::new("git")
        .arg("commit")
        .arg("-m")
        .arg(message)
        .current_dir(project_root)
        .output()?;

    if !commit_output.status.success() {
        let stderr = String::from_utf8_lossy(&commit_output.stderr);
        return Ok(format!("Error committing: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&commit_output.stdout);
    Ok(format!("Successfully committed:\n{}", stdout))
}

/// Execute git_pr_create tool
fn execute_git_pr_create(args: &Value, project_root: &Path) -> Result<String> {
    use std::process::Command;

    let title = args["title"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing title parameter"))?;

    let body = args["body"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing body parameter"))?;

    let base = args["base"].as_str();
    let draft = args["draft"].as_bool().unwrap_or(false);

    // Check if gh CLI is installed
    let gh_check = Command::new("gh")
        .arg("--version")
        .output();

    if gh_check.is_err() {
        return Ok("Error: GitHub CLI (gh) is not installed. Please install it first: https://cli.github.com/".to_string());
    }

    // Build gh pr create command
    let mut cmd = Command::new("gh");
    cmd.arg("pr")
        .arg("create")
        .arg("--title")
        .arg(title)
        .arg("--body")
        .arg(body)
        .current_dir(project_root);

    if let Some(base_branch) = base {
        cmd.arg("--base").arg(base_branch);
    }

    if draft {
        cmd.arg("--draft");
    }

    let output = cmd.output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Ok(format!("Error creating PR: {}\n\nTip: Make sure you have:\n1. Pushed your branch to remote: git push -u origin <branch>\n2. Authenticated with gh: gh auth login\n3. Set up a remote repository", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(format!("Successfully created pull request:\n{}", stdout))
}

/// Execute web_search tool
fn execute_web_search(args: &Value) -> Result<String> {
    let query = args["query"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing query parameter"))?;

    // Use DuckDuckGo HTML search
    // Use block_in_place if inside a runtime, otherwise create new runtime
    let results = if tokio::runtime::Handle::try_current().is_ok() {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let client = reqwest::Client::builder()
                    .user_agent("Mozilla/5.0 (compatible; BerryCode/1.0)")
                    .timeout(std::time::Duration::from_secs(30))
                    .build()?;

                let url = format!("https://html.duckduckgo.com/html/?q={}",
                    urlencoding::encode(query));

                let response = client.get(&url).send().await?;

                if !response.status().is_success() {
                    return Err(anyhow!("Search request failed: {}", response.status()));
                }

                let html = response.text().await?;

                // Simple parsing - extract result snippets
                let mut results = Vec::new();
                let lines: Vec<&str> = html.lines().collect();

                for (_i, line) in lines.iter().enumerate() {
                    if line.contains("result__title") || line.contains("result__snippet") {
                        // Extract text from HTML
                        let text = line
                            .replace("<b>", "")
                            .replace("</b>", "")
                            .replace("&quot;", "\"")
                            .replace("&amp;", "&")
                            .trim()
                            .to_string();

                        if !text.is_empty() && text.len() > 10 {
                            results.push(text);
                        }

                        if results.len() >= 10 {
                            break;
                        }
                    }
                }

                Ok::<Vec<String>, anyhow::Error>(results)
            })
        })?
    } else {
        let runtime = tokio::runtime::Runtime::new()?;
        runtime.block_on(async {
            let client = reqwest::Client::builder()
                .user_agent("Mozilla/5.0 (compatible; BerryCode/1.0)")
                .timeout(std::time::Duration::from_secs(30))
                .build()?;

            let url = format!("https://html.duckduckgo.com/html/?q={}",
                urlencoding::encode(query));

            let response = client.get(&url).send().await?;

            if !response.status().is_success() {
                return Err(anyhow!("Search request failed: {}", response.status()));
            }

            let html = response.text().await?;

            // Simple parsing - extract result snippets
            let mut results = Vec::new();
            let lines: Vec<&str> = html.lines().collect();

            for (_i, line) in lines.iter().enumerate() {
                if line.contains("result__title") || line.contains("result__snippet") {
                    // Extract text from HTML
                    let text = line
                        .replace("<b>", "")
                        .replace("</b>", "")
                        .replace("&quot;", "\"")
                        .replace("&amp;", "&")
                        .trim()
                        .to_string();

                    if !text.is_empty() && text.len() > 10 {
                        results.push(text);
                    }

                    if results.len() >= 10 {
                        break;
                    }
                }
            }

            Ok::<Vec<String>, anyhow::Error>(results)
        })?
    };

    if results.is_empty() {
        Ok(format!("No search results found for: {}", query))
    } else {
        Ok(format!(
            "Search results for '{}':\n\n{}",
            query,
            results.iter()
                .enumerate()
                .map(|(i, r)| format!("{}. {}", i + 1, r))
                .collect::<Vec<_>>()
                .join("\n\n")
        ))
    }
}

/// Execute notebook_edit tool
fn execute_notebook_edit(args: &Value, project_root: &Path) -> Result<String> {
    let notebook_path = args["notebook_path"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing notebook_path parameter"))?;

    let cell_index = args["cell_index"]
        .as_u64()
        .ok_or_else(|| anyhow!("Missing or invalid cell_index parameter"))? as usize;

    let new_content = args["new_content"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing new_content parameter"))?;

    let cell_type = args["cell_type"].as_str();

    let full_path = project_root.join(notebook_path);

    if !full_path.exists() {
        return Ok(format!("Error: Notebook not found: {}", notebook_path));
    }

    // Read notebook as JSON
    let notebook_str = fs::read_to_string(&full_path)?;
    let mut notebook: Value = serde_json::from_str(&notebook_str)?;

    // Get cells array
    let cells = notebook["cells"]
        .as_array_mut()
        .ok_or_else(|| anyhow!("Invalid notebook format: no cells array"))?;

    if cell_index >= cells.len() {
        return Ok(format!("Error: Cell index {} out of range (notebook has {} cells)",
            cell_index, cells.len()));
    }

    // Update cell
    let cell = &mut cells[cell_index];

    // Update cell type if provided
    if let Some(ct) = cell_type {
        cell["cell_type"] = Value::String(ct.to_string());
    }

    // Update source - split into lines
    let lines: Vec<String> = new_content.lines().map(|s| format!("{}\n", s)).collect();
    cell["source"] = Value::Array(lines.iter().map(|s| Value::String(s.clone())).collect());

    // Write back to file
    let updated_notebook = serde_json::to_string_pretty(&notebook)?;
    fs::write(&full_path, updated_notebook)?;

    Ok(format!("Successfully edited cell {} in {}", cell_index, notebook_path))
}

/// Execute create_artifact tool
fn execute_create_artifact(args: &Value, project_root: &Path) -> Result<String> {
    use crate::berrycode::artifacts::get_artifacts_server;

    let title = args["title"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing title parameter"))?;

    let content = args["content"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing content parameter"))?;

    let _artifact_type = args.get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("html");

    // Get or create the global artifacts server (starts automatically in background)
    let server = get_artifacts_server(&project_root.to_path_buf())?;

    // Update artifact content
    let server_lock = server.lock().unwrap();
    server_lock.update(content)?;

    let url = server_lock.url();
    let artifacts_path = server_lock.artifacts_dir().join("index.html");

    // Open browser (first time only is more polite)
    let _ = server_lock.open_browser(); // Ignore errors

    drop(server_lock); // Release lock

    let mut result = String::new();
    result.push_str(&format!("🎨 Created artifact: {}\n", title));
    result.push_str(&format!("📁 Saved to: {:?}\n", artifacts_path));
    result.push_str(&format!("🌐 Live preview: {}\n\n", url));
    result.push_str("✅ Artifact server running in background!\n");
    result.push_str("♻️  Auto-reloads every 2 seconds when you update the artifact.\n\n");
    result.push_str("💡 Tips:\n");
    result.push_str("   - Update the artifact by calling create_artifact again\n");
    result.push_str("   - The browser will automatically refresh to show changes\n");
    result.push_str("   - No separate server process needed - it's built-in!\n");

    Ok(result)
}

/// Execute lsp_goto_definition tool
fn execute_lsp_goto_definition(args: &Value, project_root: &Path) -> Result<String> {
    use crate::berrycode::lsp_client::LspClient;
    use crate::berrycode::display::DisplayManager;

    let file_path = args["file_path"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing file_path parameter"))?;

    let line = args["line"]
        .as_u64()
        .ok_or_else(|| anyhow!("Missing or invalid line parameter"))? as u32;

    let character = args["character"]
        .as_u64()
        .ok_or_else(|| anyhow!("Missing or invalid character parameter"))? as u32;

    let symbol = args.get("symbol")
        .and_then(|v| v.as_str())
        .unwrap_or("symbol");

    let display = DisplayManager::new();
    display.log_action(&format!("🚀 LSP goto_definition({} at {}:{})", symbol, file_path, line));

    let lsp = LspClient::new(project_root.to_path_buf());
    let full_path = project_root.join(file_path);

    match lsp.goto_definition(&full_path, line, character)? {
        Some(location) => {
            let def_path = location.uri.to_file_path().unwrap();
            let def_path_rel = def_path.strip_prefix(project_root).unwrap_or(&def_path);
            let def_line = location.range.start.line;
            let def_char = location.range.start.character;

            display.print_sub_result(&format!(
                "✅ Definition found: {}:{}:{}",
                def_path_rel.display(),
                def_line,
                def_char
            ), true);

            Ok(format!(
                "🎯 Definition of '{}' found at:\nFile: {}\nLine: {} (column {})",
                symbol,
                def_path_rel.display(),
                def_line,
                def_char
            ))
        }
        None => {
            display.print_sub_result("❌ Definition not found",  false);
            Ok(format!("Definition of '{}' not found. The symbol may not exist or the language server is not running.", symbol))
        }
    }
}

/// Execute lsp_find_references tool
fn execute_lsp_find_references(args: &Value, project_root: &Path) -> Result<String> {
    use crate::berrycode::lsp_client::LspClient;
    use crate::berrycode::display::DisplayManager;

    let file_path = args["file_path"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing file_path parameter"))?;

    let line = args["line"]
        .as_u64()
        .ok_or_else(|| anyhow!("Missing or invalid line parameter"))? as u32;

    let character = args["character"]
        .as_u64()
        .ok_or_else(|| anyhow!("Missing or invalid character parameter"))? as u32;

    let symbol = args.get("symbol")
        .and_then(|v| v.as_str())
        .unwrap_or("symbol");

    let display = DisplayManager::new();
    display.log_action(&format!("🚀 LSP find_references({} at {}:{})", symbol, file_path, line));

    let lsp = LspClient::new(project_root.to_path_buf());
    let full_path = project_root.join(file_path);

    let references = lsp.find_references(&full_path, line, character)?;

    if references.is_empty() {
        display.print_sub_result("❌ No references found", false);
        return Ok(format!("No references found for '{}'", symbol));
    }

    display.print_sub_result(&format!("✅ Found {} references", references.len()), true);

    let mut result = format!("📍 Found {} references to '{}':\n\n", references.len(), symbol);

    for (i, location) in references.iter().enumerate() {
        let ref_path = location.uri.to_file_path().unwrap();
        let ref_path_rel = ref_path.strip_prefix(project_root).unwrap_or(&ref_path);
        let ref_line = location.range.start.line;
        let ref_char = location.range.start.character;

        result.push_str(&format!(
            "{}. {}:{}:{}\n",
            i + 1,
            ref_path_rel.display(),
            ref_line,
            ref_char
        ));
    }

    Ok(result)
}

/// Execute lsp_hover tool
fn execute_lsp_hover(args: &Value, project_root: &Path) -> Result<String> {
    use crate::berrycode::lsp_client::LspClient;
    use crate::berrycode::display::DisplayManager;

    let file_path = args["file_path"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing file_path parameter"))?;

    let line = args["line"]
        .as_u64()
        .ok_or_else(|| anyhow!("Missing or invalid line parameter"))? as u32;

    let character = args["character"]
        .as_u64()
        .ok_or_else(|| anyhow!("Missing or invalid character parameter"))? as u32;

    let symbol = args.get("symbol")
        .and_then(|v| v.as_str())
        .unwrap_or("symbol");

    let display = DisplayManager::new();
    display.log_action(&format!("🚀 LSP hover({} at {}:{})", symbol, file_path, line));

    let lsp = LspClient::new(project_root.to_path_buf());
    let full_path = project_root.join(file_path);

    match lsp.hover(&full_path, line, character)? {
        Some(info) => {
            display.print_sub_result(&format!("✅ Type info retrieved ({} chars)", info.len()), true);

            Ok(format!(
                "📖 Type information for '{}':\n\n{}",
                symbol,
                info
            ))
        }
        None => {
            display.print_sub_result("❌ No type info available", false);
            Ok(format!("No type information available for '{}'", symbol))
        }
    }
}

/// Self-healing tool definition
fn self_healing_tool() -> Tool {
    Tool {
        tool_type: "function".to_string(),
        function: FunctionDefinition {
            name: "self_healing".to_string(),
            description: "🧬 MAD SCIENCE: Autonomous self-healing loop that AUTOMATICALLY fixes code until tests pass! This is true autonomous engineering - you write code, BerryCode runs tests, analyzes failures, and keeps fixing until everything works. No more manual debugging! The loop runs up to max_iterations (default: 3) to prevent infinite loops. Perfect for: fixing test failures, resolving compilation errors, debugging production issues. Just sit back and watch BerryCode heal itself! ☕".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "max_iterations": {
                        "type": "number",
                        "description": "Maximum number of fix attempts (default: 3). Higher values allow more attempts but take longer."
                    },
                    "test_timeout": {
                        "type": "number",
                        "description": "Timeout for each test run in seconds (default: 60)"
                    },
                    "description": {
                        "type": "string",
                        "description": "Description of what needs to be fixed (optional, for logging)"
                    }
                },
                "required": []
            }),
        },
    }
}

/// Execute self-healing tool
fn execute_self_healing(args: &Value, project_root: &Path) -> Result<String> {
    use crate::berrycode::self_healing::{SelfHealingConfig, SelfHealingLoop};
    use crate::berrycode::display::DisplayManager;
    use std::time::Duration;

    let max_iterations = args.get("max_iterations")
        .and_then(|v| v.as_u64())
        .unwrap_or(3) as usize;

    let test_timeout = args.get("test_timeout")
        .and_then(|v| v.as_u64())
        .unwrap_or(60);

    let description = args.get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("Fixing code");

    let display = DisplayManager::new();
    display.log_action(&format!("🧬 Self-Healing: {}", description));

    // Detect project type
    let project_type = SelfHealingLoop::detect_project_type(project_root);

    let config = SelfHealingConfig {
        max_iterations,
        test_timeout: Duration::from_secs(test_timeout),
        project_type,
    };

    // Create and run self-healing loop
    let mut healing_loop = SelfHealingLoop::new(project_root, config);

    display.print_sub_result(&format!(
        "Starting autonomous healing (max {} iterations)",
        max_iterations
    ), true);

    let result = healing_loop.run_loop()?;

    // Display results
    if result.success {
        display.print_sub_result(&format!(
            "✅ {} (in {} iteration(s))",
            result.final_message,
            result.iterations.len()
        ), true);
    } else {
        display.print_sub_result(&format!(
            "❌ {} after {} iterations",
            result.final_message,
            result.iterations.len()
        ), true);
    }

    // Build detailed result
    let mut output = String::new();
    output.push_str(&format!("# Self-Healing Result\n\n"));
    output.push_str(&format!("**Status**: {}\n\n",
        if result.success { "✅ SUCCESS" } else { "❌ FAILED" }));
    output.push_str(&format!("**Iterations**: {}\n\n", result.iterations.len()));

    // Add summary of each iteration
    for iteration in &result.iterations {
        output.push_str(&format!("## Iteration {}\n", iteration.iteration));
        output.push_str(&format!("- **Status**: {}\n",
            if iteration.success { "✅ PASSED" } else { "❌ FAILED" }));
        output.push_str(&format!("- **Duration**: {}ms\n", iteration.duration_ms));

        if let Some(error) = &iteration.error_message {
            output.push_str(&format!("- **Error**:\n```\n{}\n```\n",
                error.lines().take(10).collect::<Vec<_>>().join("\n")));
        }

        output.push_str("\n");
    }

    output.push_str(&format!("\n**Final Message**: {}\n", result.final_message));

    Ok(output)
}

/// Vision analyze tool definition
fn vision_analyze_tool() -> Tool {
    Tool {
        tool_type: "function".to_string(),
        function: FunctionDefinition {
            name: "vision_analyze".to_string(),
            description: "👁️ MAD SCIENCE: Capture and analyze web UI screenshots using vision models (Gemini 1.5 Pro / Claude 3.5 Sonnet)! This tool launches headless Chrome, captures screenshots, and uses AI vision to detect layout issues, alignment problems, color mismatches, and CSS bugs. Perfect for: debugging UI issues, verifying responsive design, checking accessibility, analyzing competitor UIs. Returns detailed analysis with specific CSS fixes. Just point it at any URL (including localhost) and get instant visual feedback! 🎨".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "URL to capture and analyze (e.g., 'http://localhost:3000', 'https://example.com')"
                    },
                    "prompt": {
                        "type": "string",
                        "description": "What to analyze or look for (e.g., 'Check landing page layout', 'Find alignment issues', 'Analyze mobile responsiveness'). Default: general UI analysis"
                    },
                    "selector": {
                        "type": "string",
                        "description": "Optional: CSS selector to focus on specific element (e.g., '.header', '#main-content')"
                    },
                    "viewport_width": {
                        "type": "number",
                        "description": "Viewport width in pixels (default: 1920). Use 375 for mobile, 768 for tablet"
                    },
                    "viewport_height": {
                        "type": "number",
                        "description": "Viewport height in pixels (default: 1080)"
                    }
                },
                "required": ["url"]
            }),
        },
    }
}

/// Execute vision_analyze tool
fn execute_vision_analyze(args: &Value, project_root: &Path) -> Result<String> {
    use crate::berrycode::vision::{VisionAnalyzer, VisionRequest};
    use crate::berrycode::display::DisplayManager;

    let url = args["url"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing url parameter"))?;

    let prompt = args.get("prompt")
        .and_then(|v| v.as_str())
        .unwrap_or("Analyze this web UI for layout, alignment, color, and design issues");

    let selector = args.get("selector")
        .and_then(|v| v.as_str());

    let viewport_width = args.get("viewport_width")
        .and_then(|v| v.as_u64())
        .unwrap_or(1920) as u32;

    let viewport_height = args.get("viewport_height")
        .and_then(|v| v.as_u64())
        .unwrap_or(1080) as u32;

    let display = DisplayManager::new();
    display.log_action(&format!("👁️ Vision analyzing: {}", url));

    let analyzer = VisionAnalyzer::new(project_root)?;

    let request = VisionRequest {
        url: url.to_string(),
        prompt: prompt.to_string(),
        selector: selector.map(|s| s.to_string()),
        viewport_width,
        viewport_height,
    };

    // Run async analysis in blocking context
    // Use block_in_place if inside a runtime, otherwise create new runtime
    let vision_result = if tokio::runtime::Handle::try_current().is_ok() {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(analyzer.analyze(request))
        })?
    } else {
        let runtime = tokio::runtime::Runtime::new()?;
        runtime.block_on(analyzer.analyze(request))?
    };

    display.print_sub_result(&format!(
        "✅ Screenshot captured and analyzed ({} issues found)",
        vision_result.issues.len()
    ), true);

    // Build detailed result
    let mut output = String::new();
    output.push_str("# Vision Analysis Result\n\n");
    output.push_str(&format!("**URL**: {}\n", url));
    output.push_str(&format!("**Screenshot**: {:?}\n\n", vision_result.screenshot_path));

    // Add issues
    if !vision_result.issues.is_empty() {
        output.push_str("## 🔍 Issues Detected\n\n");
        for (i, issue) in vision_result.issues.iter().enumerate() {
            let icon = match issue.severity.as_str() {
                "critical" => "🔴",
                "warning" => "⚠️",
                _ => "ℹ️",
            };
            output.push_str(&format!("{}. {} **{}** - {}\n",
                i + 1,
                icon,
                issue.severity.to_uppercase(),
                issue.description
            ));
        }
        output.push_str("\n");
    }

    // Add full analysis
    output.push_str("## 📊 Detailed Analysis\n\n");
    output.push_str(&vision_result.analysis);
    output.push_str("\n\n");

    // Add fixes if any
    if !vision_result.fixes.is_empty() {
        output.push_str("## 🔧 Suggested Fixes\n\n");
        for (i, fix) in vision_result.fixes.iter().enumerate() {
            output.push_str(&format!("{}. **{}**: {}\n```css\n{}\n```\n\n",
                i + 1,
                fix.file_path,
                fix.description,
                fix.css_code
            ));
        }
    }

    Ok(output)
}

/// Knowledge graph tool definition
fn knowledge_graph_tool() -> Tool {
    Tool {
        tool_type: "function".to_string(),
        function: FunctionDefinition {
            name: "knowledge_graph".to_string(),
            description: "🕸️ MAD SCIENCE: Build and query a knowledge graph of code dependencies! This tool maps your ENTIRE codebase as a graph showing how files, functions, and types relate to each other. Perfect for: impact analysis (what breaks if I change this?), finding dependency paths, refactoring planning, understanding code architecture. Uses petgraph for fast graph algorithms. Returns detailed dependency analysis with direct and indirect relationships. Essential before making significant changes! 🧠".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "description": "Action to perform: 'build' (construct graph from source), 'impact' (analyze impact of changing an entity), 'path' (find dependency path), 'stats' (get graph statistics)",
                        "enum": ["build", "impact", "path", "stats"]
                    },
                    "entity": {
                        "type": "string",
                        "description": "Entity name for 'impact' action (e.g., 'src/models.rs', 'User')"
                    },
                    "from_entity": {
                        "type": "string",
                        "description": "Source entity for 'path' action"
                    },
                    "to_entity": {
                        "type": "string",
                        "description": "Target entity for 'path' action"
                    }
                },
                "required": ["action"]
            }),
        },
    }
}

/// Execute knowledge_graph tool
fn execute_knowledge_graph(args: &Value, project_root: &Path) -> Result<String> {
    use crate::berrycode::knowledge_graph::KnowledgeGraph;
    use crate::berrycode::display::DisplayManager;

    let action = args["action"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing action parameter"))?;

    let display = DisplayManager::new();
    display.log_action(&format!("🕸️ Knowledge Graph: {}", action));

    let mut kg = KnowledgeGraph::new(project_root)?;

    match action {
        "build" => {
            display.print_sub_result("Building knowledge graph from source...", true);

            let indexed_count = kg.build_from_source()?;

            let stats = kg.stats();

            display.print_sub_result(&format!(
                "✅ Built graph with {} entities and {} relationships",
                stats.total_entities,
                stats.total_relationships
            ), true);

            let mut output = String::new();
            output.push_str("# Knowledge Graph Built\n\n");
            output.push_str(&format!("**Files Indexed**: {}\n", indexed_count));
            output.push_str(&format!("**Total Entities**: {}\n", stats.total_entities));
            output.push_str(&format!("**Total Relationships**: {}\n\n", stats.total_relationships));

            output.push_str("## Entity Type Breakdown\n\n");
            for (entity_type, count) in stats.entity_type_counts {
                output.push_str(&format!("- {:?}: {}\n", entity_type, count));
            }

            output.push_str("\n✅ Knowledge graph cached and ready for queries!\n");

            Ok(output)
        }

        "impact" => {
            let entity = args.get("entity")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Missing entity parameter for impact analysis"))?;

            display.print_sub_result(&format!("Analyzing impact of changing: {}", entity), true);

            let analysis = kg.impact_analysis(entity)?;

            let impact_icon = match analysis.impact_level {
                crate::berrycode::knowledge_graph::ImpactLevel::Critical => "🔴",
                crate::berrycode::knowledge_graph::ImpactLevel::High => "⚠️",
                crate::berrycode::knowledge_graph::ImpactLevel::Medium => "⚡",
                crate::berrycode::knowledge_graph::ImpactLevel::Low => "ℹ️",
            };

            display.print_sub_result(&format!(
                "{} Impact: {:?} ({} total entities affected)",
                impact_icon,
                analysis.impact_level,
                analysis.total_affected
            ), true);

            let mut output = String::new();
            output.push_str("# Impact Analysis\n\n");
            output.push_str(&format!("**Entity**: {}\n", analysis.entity.name));
            output.push_str(&format!("**Type**: {:?}\n", analysis.entity.entity_type));
            output.push_str(&format!("**Impact Level**: {} {:?}\n\n",
                impact_icon,
                analysis.impact_level
            ));

            output.push_str(&format!("## 🎯 Direct Dependencies ({})\n\n",
                analysis.direct_dependencies.len()
            ));

            for (i, dep) in analysis.direct_dependencies.iter().take(20).enumerate() {
                output.push_str(&format!("{}. **{}** ({:?}) - {:?}\n",
                    i + 1,
                    dep.name,
                    dep.entity_type,
                    dep.file_path
                ));
            }

            if analysis.direct_dependencies.len() > 20 {
                output.push_str(&format!("\n... and {} more\n",
                    analysis.direct_dependencies.len() - 20
                ));
            }

            output.push_str(&format!("\n## 🔗 Indirect Dependencies ({})\n\n",
                analysis.indirect_dependencies.len()
            ));

            for (i, dep) in analysis.indirect_dependencies.iter().take(10).enumerate() {
                output.push_str(&format!("{}. **{}** ({:?})\n",
                    i + 1,
                    dep.name,
                    dep.entity_type
                ));
            }

            if analysis.indirect_dependencies.len() > 10 {
                output.push_str(&format!("\n... and {} more\n",
                    analysis.indirect_dependencies.len() - 10
                ));
            }

            output.push_str(&format!("\n**⚠️ TOTAL IMPACT**: {} entities will be affected by changes to {}\n",
                analysis.total_affected,
                entity
            ));

            Ok(output)
        }

        "path" => {
            let from = args.get("from_entity")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Missing from_entity parameter"))?;

            let to = args.get("to_entity")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Missing to_entity parameter"))?;

            display.print_sub_result(&format!("Finding path from {} to {}", from, to), true);

            match kg.find_path(from, to)? {
                Some(path) => {
                    display.print_sub_result(&format!(
                        "✅ Found path with {} steps",
                        path.len() - 1
                    ), true);

                    let mut output = String::new();
                    output.push_str("# Dependency Path\n\n");
                    output.push_str(&format!("**From**: {}\n", from));
                    output.push_str(&format!("**To**: {}\n", to));
                    output.push_str(&format!("**Steps**: {}\n\n", path.len() - 1));

                    output.push_str("## Path:\n\n");
                    for (i, entity) in path.iter().enumerate() {
                        output.push_str(&format!("{}. **{}** ({:?})\n   File: {:?}\n",
                            i + 1,
                            entity.name,
                            entity.entity_type,
                            entity.file_path
                        ));

                        if i < path.len() - 1 {
                            output.push_str("   ↓\n");
                        }
                    }

                    Ok(output)
                }
                None => {
                    display.print_sub_result("❌ No path found", false);

                    Ok(format!(
                        "# No Dependency Path Found\n\n\
                         No path exists from **{}** to **{}**.\n\n\
                         They may be in separate dependency trees.",
                        from, to
                    ))
                }
            }
        }

        "stats" => {
            let stats = kg.stats();

            display.print_sub_result(&format!(
                "📊 {} entities, {} relationships",
                stats.total_entities,
                stats.total_relationships
            ), true);

            let mut output = String::new();
            output.push_str("# Knowledge Graph Statistics\n\n");
            output.push_str(&format!("**Total Entities**: {}\n", stats.total_entities));
            output.push_str(&format!("**Total Relationships**: {}\n\n", stats.total_relationships));

            output.push_str("## Entity Types\n\n");
            for (entity_type, count) in stats.entity_type_counts {
                output.push_str(&format!("- **{:?}**: {}\n", entity_type, count));
            }

            Ok(output)
        }

        _ => Err(anyhow!("Unknown action: {}. Use: build, impact, path, or stats", action)),
    }
}

/// Hive Mind tool definition
fn hive_mind_tool() -> Tool {
    Tool {
        tool_type: "function".to_string(),
        function: FunctionDefinition {
            name: "hive_mind".to_string(),
            description: "🧠 SINGULARITY: Assemble a TEAM of specialized AI agents to collaborate on complex tasks! Instead of one AI doing everything, this creates a PROJECT MANAGER, ARCHITECT, CODER, SECURITY EXPERT, and QA ENGINEER who discuss, debate, and build together. Each agent has expertise and perspective. They use knowledge graph for impact analysis, self-healing for testing, and vision for UI review. This is TRUE team collaboration - multiple minds, better decisions, safer code! Perfect for complex features, refactoring, or when you want expert review. 🚀".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "task": {
                        "type": "string",
                        "description": "The task for the team to accomplish (e.g., 'Add user authentication', 'Refactor payment system', 'Build admin dashboard')"
                    },
                    "verbose": {
                        "type": "boolean",
                        "description": "Show detailed discussion between agents? Default: true"
                    },
                    "max_rounds": {
                        "type": "number",
                        "description": "Maximum discussion rounds (default: 5)"
                    }
                },
                "required": ["task"]
            }),
        },
    }
}

/// Execute hive_mind tool
fn execute_hive_mind(args: &Value, project_root: &Path) -> Result<String> {
    use crate::berrycode::swarm::{Swarm, SwarmConfig};
    use crate::berrycode::display::DisplayManager;

    let task = args["task"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing task parameter"))?;

    let verbose = args.get("verbose")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let max_rounds = args.get("max_rounds")
        .and_then(|v| v.as_u64())
        .unwrap_or(5) as usize;

    let display = DisplayManager::new();
    display.log_action(&format!("🧠 Hive Mind: Assembling team for '{}'", task));

    let config = SwarmConfig {
        max_rounds,
        min_approvals: 3,
        verbose,
    };

    let mut swarm = Swarm::new(project_root, config);

    // Run async task in blocking context
    // Use block_in_place if inside a runtime, otherwise create new runtime
    let result = if tokio::runtime::Handle::try_current().is_ok() {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(swarm.execute_task(task))
        })?
    } else {
        let runtime = tokio::runtime::Runtime::new()?;
        runtime.block_on(swarm.execute_task(task))?
    };

    display.print_sub_result(&format!(
        "{} Team {} task",
        if result.success { "✅" } else { "⚠️" },
        if result.success { "completed" } else { "finished with concerns" }
    ), true);

    // Build detailed output
    let mut output = String::new();
    output.push_str("# 🧠 Hive Mind: Team Collaboration Result\n\n");
    output.push_str(&format!("**Task**: {}\n", result.task));
    output.push_str(&format!("**Status**: {}\n\n",
        if result.success { "✅ SUCCESS" } else { "⚠️ COMPLETED WITH WARNINGS" }
    ));

    output.push_str("## 👥 Team Discussion\n\n");

    for phase in &result.phases {
        output.push_str(&format!("### {} {} {}\n\n",
            phase.agent.icon(),
            phase.agent.name(),
            if phase.success { "✅" } else { "⚠️" }
        ));
        output.push_str(&format!("**Summary**: {}\n\n", phase.summary));
        output.push_str(&format!("**Details**:\n{}\n\n", phase.output));
        output.push_str("---\n\n");
    }

    output.push_str("## 📊 Final Summary\n\n");
    output.push_str(&result.final_summary);
    output.push_str("\n\n");

    if !result.discussion_log.is_empty() {
        output.push_str("## 💬 Discussion Log\n\n");
        for msg in result.discussion_log.iter().take(20) {
            output.push_str(&format!("{} **[{}]**: {}\n",
                msg.from.icon(),
                msg.from.name(),
                msg.content
            ));
        }

        if result.discussion_log.len() > 20 {
            output.push_str(&format!("\n... and {} more messages\n",
                result.discussion_log.len() - 20
            ));
        }
    }

    output.push_str("\n✨ **This is the power of Hive Mind** - multiple expert perspectives collaborating for better results!\n");

    Ok(output)
}

/// Execute semantic_search tool - Now with Hybrid Search + Reranking! 🚀
fn execute_semantic_search(args: &Value, project_root: &Path) -> Result<String> {
    use crate::berrycode::vector_search::VectorSearch;
    use crate::berrycode::hybrid_search::HybridSearch;
    use crate::berrycode::reranker::Reranker;
    use crate::berrycode::display::DisplayManager;

    let query = args["query"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing query parameter"))?;

    let limit = args.get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(10) as usize;

    let display = DisplayManager::new();
    display.log_action(&format!("🔍 Searching \"{}\"...", query));

    // Create vector search instance
    let vs = VectorSearch::new(project_root)?;

    // Check if index is ready
    if vs.is_empty() {
        display.print_sub_result("⚠️ Vector index not ready yet (background indexing may still be running)", false);
        return Ok(format!(
            "Vector search index is not ready yet.\n\n\
             Background indexing started on startup, but may still be in progress.\n\
             Please try again in a moment, or use 'grep' for text-based search in the meantime."
        ));
    }

    // 🎯 STEP 1: Create hybrid search engine
    let hs = HybridSearch::new(vs);

    // 🎯 STEP 2: Initialize reranker (if available)
    #[cfg(feature = "fastembed")]
    let reranker = Reranker::new().ok(); // Graceful degradation if model download fails

    #[cfg(not(feature = "fastembed"))]
    let reranker: Option<Reranker> = None;

    // Perform hybrid search with reranking
    // Fetch more candidates (limit * 2) for better reranking results
    let candidate_limit = limit * 2;

    display.print_sub_result(&format!("  └ ⚡ Vector Search: Searching {} candidates...", candidate_limit), true);

    let start = std::time::Instant::now();

    let results = if tokio::runtime::Handle::try_current().is_ok() {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(
                hs.search_with_reranking(query, candidate_limit, None, reranker.as_ref())
            )
        })?
    } else {
        let runtime = tokio::runtime::Runtime::new()?;
        runtime.block_on(
            hs.search_with_reranking(query, candidate_limit, None, reranker.as_ref())
        )?
    };

    let search_time = start.elapsed();

    // 🛡️ STEP 3: Filter by score threshold (remove low-confidence results)
    const SCORE_THRESHOLD: f32 = 0.4;
    let filtered_results: Vec<_> = results
        .into_iter()
        .filter(|r| r.combined_score >= SCORE_THRESHOLD)
        .take(limit)
        .collect();

    // 👁️ STEP 4: Show reasoning trace
    let vector_count = filtered_results.iter().filter(|r| r.graph_distance == 0).count();
    let graph_count = filtered_results.iter().filter(|r| r.graph_distance > 0).count();

    if reranker.is_some() {
        display.print_sub_result(&format!(
            "  └ 🧠 Reranking: Filtered top {} (Score threshold: {:.1})",
            filtered_results.len(),
            SCORE_THRESHOLD
        ), true);
    }

    display.print_sub_result(&format!(
        "  └ ✅ Found: {} results ({}ms) - {} direct, {} via graph",
        filtered_results.len(),
        search_time.as_millis(),
        vector_count,
        graph_count
    ), true);

    if filtered_results.is_empty() {
        display.print_sub_result("⚠️ No high-confidence results found", false);
        return Ok(format!(
            "No semantic matches found for query: {}\n\n\
             All candidates had similarity scores below {:.0}% (threshold: {:.0}%).\n\
             Try rephrasing your query or use 'grep' for exact text matching.",
            query,
            SCORE_THRESHOLD * 100.0,
            SCORE_THRESHOLD * 100.0
        ));
    }

    // Build detailed output
    let mut output = String::new();
    output.push_str(&format!("# 🎯 Hybrid Search Results\n\n"));
    output.push_str(&format!("**Query**: {}\n", query));
    output.push_str(&format!("**Results**: {}/{} (filtered by {:.0}% confidence)\n", filtered_results.len(), limit, SCORE_THRESHOLD * 100.0));
    output.push_str(&format!("**Search Time**: {}ms\n", search_time.as_millis()));
    if reranker.is_some() {
        output.push_str(&format!("**Reranking**: ✅ Enabled (Cross-Encoder)\n"));
    } else {
        output.push_str(&format!("**Reranking**: ⚠️ Disabled (fastembed feature not available)\n"));
    }
    output.push_str("\n---\n\n");

    for (i, result) in filtered_results.iter().enumerate() {
        // Show relevance indicator
        let relevance = if result.combined_score >= 0.8 {
            "🟢 Highly Relevant"
        } else if result.combined_score >= 0.6 {
            "🟡 Relevant"
        } else {
            "🟠 Possibly Relevant"
        };

        output.push_str(&format!(
            "{}. **{}** (score: {:.1}%) {}\n",
            i + 1,
            result.file_path,
            result.combined_score * 100.0,
            relevance
        ));

        // Show relationship if found via graph
        if let Some(ref relationship) = result.relationship {
            output.push_str(&format!("   _Via graph: {}_\n", relationship));
        }

        // Show chunk type if available
        if let Some(ref chunk_type) = result.chunk_type {
            output.push_str(&format!("   Type: `{}`", chunk_type));
            if let (Some(start), Some(end)) = (result.start_line, result.end_line) {
                output.push_str(&format!(" (lines {}-{})", start, end));
            }
            output.push_str("\n");
        }

        // Show summary if available
        if !result.summary.is_empty() {
            let summary_lines: Vec<&str> = result.summary.lines().take(5).collect();
            output.push_str(&format!("```\n{}\n```\n\n", summary_lines.join("\n")));
        } else {
            output.push_str("\n");
        }
    }

    output.push_str(&format!(
        "\n💡 **Search Pipeline**: Vector Search ({}ms) → Graph Expansion → {} → Score Filtering (≥{:.0}%)\n\
         Files are ranked by semantic similarity using local AI embeddings. \
         High-confidence results (≥80%) are most relevant to your query!\n",
        search_time.as_millis(),
        if reranker.is_some() { "Cross-Encoder Reranking" } else { "No Reranking" },
        SCORE_THRESHOLD * 100.0
    ));

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_tool_result_new() {
        let result = ToolResult::new(
            "test_id_123".to_string(),
            "Test content".to_string(),
        );

        assert_eq!(result.tool_call_id, "test_id_123");
        assert_eq!(result.role, "tool");
        assert_eq!(result.content, "Test content");
    }

    #[test]
    fn test_get_available_tools() {
        let tools = get_available_tools();

        // Should return 29 tools (18 base + 2 browser tools + 1 artifact tool + 3 LSP tools + 3 Mad Science + 1 Singularity + 1 Semantic Search when feature enabled)
        // Base tools: semantic_search, read_file, write_file, edit_file, translate_file, list_files, file_tree, search_files, grep,
        //             bash, lint_code, ask_user, propose_plan, web_fetch, web_search,
        //             git_diff, git_commit, git_pr_create, notebook_edit
        // LSP tools: lsp_goto_definition, lsp_find_references, lsp_hover
        // Mad Science: self_healing, vision_analyze, knowledge_graph
        // Singularity: hive_mind
        #[cfg(feature = "browser")]
        assert_eq!(tools.len(), 29);

        #[cfg(not(feature = "browser"))]
        assert_eq!(tools.len(), 27);

        // Check that all tools have function type
        for tool in &tools {
            assert_eq!(tool.tool_type, "function");
            assert!(!tool.function.name.is_empty());
            assert!(!tool.function.description.is_empty());
        }

        // Check specific tool names exist
        let tool_names: Vec<&str> = tools.iter()
            .map(|t| t.function.name.as_str())
            .collect();

        assert!(tool_names.contains(&"read_file"));
        assert!(tool_names.contains(&"write_file"));
        assert!(tool_names.contains(&"edit_file"));
        assert!(tool_names.contains(&"list_files"));
        assert!(tool_names.contains(&"file_tree")); // NEW: file tree tool
        assert!(tool_names.contains(&"glob"));
        assert!(tool_names.contains(&"grep"));
        assert!(tool_names.contains(&"bash"));
        assert!(tool_names.contains(&"ask_user"));
        assert!(tool_names.contains(&"propose_plan"));
        assert!(tool_names.contains(&"web_fetch"));
        assert!(tool_names.contains(&"web_search"));
        assert!(tool_names.contains(&"git_diff"));
        assert!(tool_names.contains(&"git_commit"));
        assert!(tool_names.contains(&"git_pr_create"));
        assert!(tool_names.contains(&"notebook_edit"));
        // LSP tools
        assert!(tool_names.contains(&"lsp_goto_definition"));
        assert!(tool_names.contains(&"lsp_find_references"));
        assert!(tool_names.contains(&"lsp_hover"));
        // Semantic search
        assert!(tool_names.contains(&"semantic_search"));
    }

    #[test]
    fn test_execute_write_file() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = "test.txt";

        let args = serde_json::json!({
            "file_path": test_file,
            "content": "Hello, world!\nLine 2"
        });

        let result = execute_write_file(&args, temp_dir.path());
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.contains("Successfully wrote"));
        assert!(output.contains(test_file));

        // Verify file was actually written
        let written_path = temp_dir.path().join(test_file);
        assert!(written_path.exists());
        let content = fs::read_to_string(written_path).unwrap();
        assert_eq!(content, "Hello, world!\nLine 2");
    }

    #[test]
    fn test_execute_read_file() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("read_test.txt");
        fs::write(&test_file, "Test content\nLine 2").unwrap();

        let args = serde_json::json!({
            "file_path": "read_test.txt"
        });

        let result = execute_read_file(&args, temp_dir.path());
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.contains("Test content"));
        assert!(output.contains("Line 2"));
        assert!(output.contains("Lines: 2"));
    }

    #[test]
    fn test_execute_read_file_not_found() {
        let temp_dir = TempDir::new().unwrap();

        let args = serde_json::json!({
            "file_path": "nonexistent.txt"
        });

        let result = execute_read_file(&args, temp_dir.path());
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.contains("Error: File not found"));
    }

    #[test]
    fn test_execute_list_files() {
        let temp_dir = TempDir::new().unwrap();

        // Create some test files and directories
        fs::write(temp_dir.path().join("file1.txt"), "content1").unwrap();
        fs::write(temp_dir.path().join("file2.rs"), "content2").unwrap();
        fs::create_dir(temp_dir.path().join("subdir")).unwrap();

        let args = serde_json::json!({
            "directory": "."
        });

        let result = execute_list_files(&args, temp_dir.path());
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.contains("file1.txt"));
        assert!(output.contains("file2.rs"));
        assert!(output.contains("subdir"));
        assert!(output.contains("FILE") || output.contains("DIR"));
    }

    #[test]
    fn test_execute_edit_file() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("edit_test.txt");

        let original_content = "Line 1\nLine 2\nLine 3";
        fs::write(&test_file, original_content).unwrap();

        let args = serde_json::json!({
            "file_path": "edit_test.txt",
            "search": "Line 2",
            "replace": "Modified Line 2"
        });

        let result = execute_edit_file(&args, temp_dir.path());
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.contains("Successfully edited"));

        // Verify file was modified
        let new_content = fs::read_to_string(&test_file).unwrap();
        assert!(new_content.contains("Modified Line 2"));
        assert!(new_content.contains("Line 1"));
        assert!(new_content.contains("Line 3"));
    }

    #[test]
    fn test_execute_edit_file_search_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("edit_test.txt");

        fs::write(&test_file, "Line 1\nLine 2").unwrap();

        let args = serde_json::json!({
            "file_path": "edit_test.txt",
            "search": "NonexistentLine",
            "replace": "New content"
        });

        let result = execute_edit_file(&args, temp_dir.path());
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.contains("Error: Search text not found"));
    }

    #[test]
    fn test_execute_edit_file_multiple_occurrences() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("edit_test.txt");

        fs::write(&test_file, "foo\nfoo\nbar").unwrap();

        let args = serde_json::json!({
            "file_path": "edit_test.txt",
            "search": "foo",
            "replace": "baz"
        });

        let result = execute_edit_file(&args, temp_dir.path());
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.contains("appears 2 times"));
    }

    #[test]
    fn test_execute_file_tree() {
        let temp_dir = TempDir::new().unwrap();

        // Create test directory structure
        fs::create_dir_all(temp_dir.path().join("src/api")).unwrap();
        fs::create_dir_all(temp_dir.path().join("tests")).unwrap();
        fs::write(temp_dir.path().join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(temp_dir.path().join("src/lib.rs"), "pub fn test() {}").unwrap();
        fs::write(temp_dir.path().join("src/api/routes.rs"), "// routes").unwrap();
        fs::write(temp_dir.path().join("tests/integration.rs"), "// test").unwrap();
        fs::write(temp_dir.path().join("README.md"), "# Project").unwrap();
        fs::write(temp_dir.path().join("Cargo.toml"), "[package]").unwrap();

        let args = serde_json::json!({
            "max_depth": 3
        });

        let result = execute_file_tree(&args, temp_dir.path());
        assert!(result.is_ok());

        let output = result.unwrap();
        println!("File tree output:\n{}", output);

        assert!(output.contains("📁 Project Structure"));
        assert!(output.contains("src/"));
        assert!(output.contains("main.rs"));
        assert!(output.contains("lib.rs"));
        assert!(output.contains("README.md"));
        assert!(output.contains("Cargo.toml"));
    }

    #[test]
    fn test_execute_file_tree_default_depth() {
        let temp_dir = TempDir::new().unwrap();

        // Create simple structure
        fs::create_dir_all(temp_dir.path().join("src")).unwrap();
        fs::write(temp_dir.path().join("src/main.rs"), "").unwrap();

        let args = serde_json::json!({});

        let result = execute_file_tree(&args, temp_dir.path());
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.contains("src/"));
        assert!(output.contains("main.rs"));
    }

    #[test]
    fn test_execute_search_files() {
        let temp_dir = TempDir::new().unwrap();

        // Create test files
        fs::create_dir_all(temp_dir.path().join("src")).unwrap();
        fs::write(temp_dir.path().join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(temp_dir.path().join("src/lib.rs"), "pub fn test() {}").unwrap();
        fs::write(temp_dir.path().join("README.md"), "# Project").unwrap();

        let args = serde_json::json!({
            "pattern": "**/*.rs"
        });

        let result = execute_search_files(&args, temp_dir.path());
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.contains("main.rs") || output.contains("lib.rs"));
    }

    #[test]
    fn test_execute_grep() {
        let temp_dir = TempDir::new().unwrap();

        // Create test files
        fs::write(temp_dir.path().join("file1.txt"), "hello world\nfoo bar").unwrap();
        fs::write(temp_dir.path().join("file2.txt"), "hello universe\nbaz qux").unwrap();

        let args = serde_json::json!({
            "pattern": "hello",
            "file_pattern": "*.txt"
        });

        let result = execute_grep(&args, temp_dir.path());
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.contains("hello"));
    }

    #[test]
    fn test_execute_grep_with_context() {
        let temp_dir = TempDir::new().unwrap();

        // Create test file with multiple lines
        let test_content = "line 1\nline 2\nline 3 with match\nline 4\nline 5\nline 6\nline 7 with match\nline 8\nline 9";
        fs::write(temp_dir.path().join("test.txt"), test_content).unwrap();

        let args = serde_json::json!({
            "pattern": "match",
            "file_pattern": "*.txt",
            "context_lines": 2
        });

        let result = execute_grep(&args, temp_dir.path());
        assert!(result.is_ok());

        let output = result.unwrap();

        // Should contain the matched lines
        assert!(output.contains("line 3 with match"));
        assert!(output.contains("line 7 with match"));

        // Should contain context lines (2 before and after)
        assert!(output.contains("line 1")); // 2 lines before line 3
        assert!(output.contains("line 2")); // 1 line before line 3
        assert!(output.contains("line 4")); // 1 line after line 3
        assert!(output.contains("line 5")); // 2 lines after line 3

        // Should have separator between matches
        assert!(output.contains("--"));

        // Should have markers (>) for matched lines
        assert!(output.contains(">3:"));
        assert!(output.contains(">7:"));
    }

    #[test]
    fn test_execute_bash() {
        let temp_dir = TempDir::new().unwrap();

        let args = serde_json::json!({
            "command": "echo 'test output'"
        });

        let result = execute_bash(&args, temp_dir.path());
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.contains("test output"));
        assert!(output.contains("Exit code: 0"));
    }

    #[test]
    fn test_execute_propose_plan() {
        let args = serde_json::json!({
            "title": "Test Plan",
            "description": "This is a test plan",
            "steps": ["Step 1", "Step 2", "Step 3"],
            "files_to_modify": ["file1.rs", "file2.rs"],
            "risks": ["Risk 1"]
        });

        let result = execute_propose_plan(&args);
        assert!(result.is_ok());

        let output = result.unwrap();
        // Output should be JSON containing the plan
        let plan: Value = serde_json::from_str(&output).unwrap();
        assert_eq!(plan["__plan__"], true);
        assert_eq!(plan["title"], "Test Plan");
    }

    #[test]
    fn test_enhance_error_message_read_file() {
        let args = serde_json::json!({
            "file_path": "missing.txt"
        });

        let enhanced = enhance_error_message(
            "read_file",
            "File not found: missing.txt",
            &args,
            Path::new("."),
        );

        assert!(enhanced.contains("💡"));
        assert!(enhanced.contains("ファイルが見つかりません"));
        assert!(enhanced.contains("list_files"));
    }

    #[test]
    fn test_enhance_error_message_edit_file() {
        let args = serde_json::json!({
            "file_path": "test.rs",
            "search": "pattern",
            "replace": "new"
        });

        let enhanced = enhance_error_message(
            "edit_file",
            "Search text not found",
            &args,
            Path::new("."),
        );

        assert!(enhanced.contains("💡"));
        assert!(enhanced.contains("検索テキストが見つかりません"));
    }

    #[test]
    fn test_enhance_error_message_grep() {
        let args = serde_json::json!({
            "pattern": "[invalid",
            "file_pattern": "*.rs"
        });

        let enhanced = enhance_error_message(
            "grep",
            "Invalid regex pattern",
            &args,
            Path::new("."),
        );

        assert!(enhanced.contains("💡"));
        assert!(enhanced.contains("正規表現エラー"));
    }

    /// Test that async tools work correctly inside a Tokio runtime
    #[tokio::test]
    async fn test_async_tools_inside_runtime() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();

        // Create a simple vector search test (this will fail gracefully if not indexed)
        let args = serde_json::json!({
            "query": "test query",
            "limit": 5
        });

        // This should not panic when called inside an async runtime
        let result = execute_semantic_search(&args, temp_dir.path());

        // We expect it to either succeed or fail gracefully (index not ready),
        // but NOT panic with "Cannot start a runtime from within a runtime"
        match result {
            Ok(output) => {
                // Success case (index was built) or graceful failure (index not ready yet)
                assert!(
                    output.contains("Semantic Search Results") ||
                    output.contains("Vector search index is not ready yet")
                );
            }
            Err(e) => {
                // Should not be a runtime nesting error
                let error_msg = format!("{}", e);
                assert!(
                    !error_msg.contains("Cannot start a runtime from within a runtime"),
                    "Got nested runtime error: {}",
                    error_msg
                );
            }
        }
    }

    /// Test that async tools work correctly outside a Tokio runtime
    #[test]
    fn test_async_tools_outside_runtime() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();

        // Create a simple vector search test
        let args = serde_json::json!({
            "query": "test query",
            "limit": 5
        });

        // This should work when called outside an async runtime
        let result = execute_semantic_search(&args, temp_dir.path());

        // We expect it to either succeed or fail gracefully, but NOT panic
        match result {
            Ok(output) => {
                assert!(
                    output.contains("Semantic Search Results") ||
                    output.contains("Vector search index is not ready yet")
                );
            }
            Err(e) => {
                // Should not be a runtime error
                let error_msg = format!("{}", e);
                assert!(
                    !error_msg.contains("Cannot start a runtime"),
                    "Got runtime error: {}",
                    error_msg
                );
            }
        }
    }

    /// Test web_fetch inside runtime
    #[tokio::test]
    #[ignore] // Ignore by default as it makes network requests
    async fn test_web_fetch_inside_runtime() {
        let args = serde_json::json!({
            "url": "https://httpbin.org/html",
            "prompt": "Extract the title"
        });

        // Should not panic with nested runtime error
        let result = execute_web_fetch(&args);

        match result {
            Ok(_) => {
                // Success is OK
            }
            Err(e) => {
                let error_msg = format!("{}", e);
                assert!(
                    !error_msg.contains("Cannot start a runtime from within a runtime"),
                    "Got nested runtime error: {}",
                    error_msg
                );
            }
        }
    }

    /// Test web_search inside runtime
    #[tokio::test]
    #[ignore] // Ignore by default as it makes network requests
    async fn test_web_search_inside_runtime() {
        let args = serde_json::json!({
            "query": "rust programming"
        });

        // Should not panic with nested runtime error
        let result = execute_web_search(&args);

        match result {
            Ok(_) => {
                // Success is OK
            }
            Err(e) => {
                let error_msg = format!("{}", e);
                assert!(
                    !error_msg.contains("Cannot start a runtime from within a runtime"),
                    "Got nested runtime error: {}",
                    error_msg
                );
            }
        }
    }
}
