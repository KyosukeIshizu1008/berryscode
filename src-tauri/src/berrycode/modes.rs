//! Custom Modes for different workflows
//!
//! Implements Architect/Code/Ask mode switching similar to Roo Code.
//! Each mode has different system prompts, models, and tool availability.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Available execution modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Architect mode: Design-focused, uses thinking models (DeepSeek-R1)
    /// Outputs plans and designs in Markdown, no code implementation
    Architect,

    /// Code mode: Implementation-focused, uses fast models (DeepSeek-V3)
    /// Can read, write, edit files and execute tools
    Code,

    /// Ask mode: Read-only, answers questions about codebase
    /// Cannot modify files, only read and analyze
    Ask,
}

impl Mode {
    /// Parse mode from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "architect" | "arch" => Some(Mode::Architect),
            "code" => Some(Mode::Code),
            "ask" => Some(Mode::Ask),
            _ => None,
        }
    }

    /// Get mode as string
    pub fn as_str(&self) -> &'static str {
        match self {
            Mode::Architect => "architect",
            Mode::Code => "code",
            Mode::Ask => "ask",
        }
    }

    /// Get mode display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Mode::Architect => "🏗️  Architect Mode",
            Mode::Code => "⚡ Code Mode",
            Mode::Ask => "💬 Ask Mode",
        }
    }

    /// Get mode configuration
    pub fn config(&self) -> ModeConfig {
        match self {
            Mode::Architect => ModeConfig::architect(),
            Mode::Code => ModeConfig::code(),
            Mode::Ask => ModeConfig::ask(),
        }
    }
}

impl Default for Mode {
    fn default() -> Self {
        Mode::Code
    }
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Configuration for a specific mode
#[derive(Debug, Clone)]
pub struct ModeConfig {
    /// Mode type
    pub mode: Mode,

    /// System prompt for this mode
    pub system_prompt: String,

    /// Preferred model for this mode
    pub preferred_model: Option<String>,

    /// Tools allowed in this mode (if None, all tools allowed)
    pub allowed_tools: Option<HashSet<String>>,

    /// Whether this mode is read-only (cannot modify files)
    pub read_only: bool,

    /// Whether to use streaming for responses
    pub streaming: bool,
}

impl ModeConfig {
    /// Architect mode configuration
    pub fn architect() -> Self {
        let system_prompt = r#"You are an expert software architect and system designer.

Your role is to:
- Analyze requirements and propose implementation plans
- Design system architecture and component structure
- Identify potential risks and trade-offs
- Create detailed technical specifications
- Output designs in clear, structured Markdown format

IMPORTANT:
- Focus on DESIGN, not implementation
- Use propose_plan tool to present your architecture
- Output should be documentation and diagrams, not code
- Think deeply about scalability, maintainability, and best practices

Available approaches:
- Break down complex features into manageable components
- Consider multiple implementation strategies
- Evaluate trade-offs (performance vs. complexity, etc.)
- Recommend best practices and patterns

Output format:
- Use Markdown with clear headings
- Include diagrams where helpful (ASCII art is fine)
- Provide rationale for architectural decisions
- List files that would need to be created/modified"#;

        let allowed_tools = Some(
            vec![
                "read_file",
                "list_files",
                "glob",
                "grep",
                "bash", // For analysis commands only
                "propose_plan",
                "ask_user",
                "web_fetch",
                "web_search",
                "git_diff",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
        );

        Self {
            mode: Mode::Architect,
            system_prompt: system_prompt.to_string(),
            preferred_model: Some("Llama 4 Scout".to_string()), // Llama 4 Scout for architecture design
            allowed_tools,
            read_only: true,
            streaming: false,
        }
    }

    /// Code mode configuration
    pub fn code() -> Self {
        let system_prompt = r#"You are an expert software engineer and coding assistant.

Your role is to:
- Implement features based on requirements or designs
- Write clean, efficient, and maintainable code
- Fix bugs and errors
- Refactor code to improve quality
- Ensure code compiles and passes tests

IMPORTANT:
- Focus on IMPLEMENTATION
- Always use lint_code tool AFTER editing files to catch errors
- Write code that follows project conventions and style
- Test your changes when possible

Best practices:
- Read existing code before making changes
- Use edit_file for targeted changes (preferred over write_file)
- Run linter/tests after changes
- Commit changes with clear messages

Available tools: All file operations, bash, linting, git operations"#;

        Self {
            mode: Mode::Code,
            system_prompt: system_prompt.to_string(),
            preferred_model: Some("Llama 4 Scout".to_string()), // Llama 4 Scout for coding
            allowed_tools: None, // All tools available
            read_only: false,
            streaming: true,
        }
    }

    /// Ask mode configuration
    pub fn ask() -> Self {
        let system_prompt = r#"You are a helpful coding assistant focused on answering questions.

Your role is to:
- Answer questions about the codebase
- Explain how code works
- Provide suggestions and recommendations
- Help understand project structure and patterns

IMPORTANT:
- You are in READ-ONLY mode - you CANNOT modify files
- Use read_file, grep, glob to analyze code
- Provide clear, concise explanations
- Reference specific files and line numbers when explaining

Best practices:
- Use grep to search for patterns across files
- Use glob to find files by name
- Read relevant files to understand context
- Provide code examples in your explanations"#;

        let allowed_tools = Some(
            vec![
                "read_file",
                "list_files",
                "glob",
                "grep",
                "bash", // For read-only commands (git log, etc.)
                "ask_user",
                "web_fetch",
                "web_search",
                "git_diff",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
        );

        Self {
            mode: Mode::Ask,
            system_prompt: system_prompt.to_string(),
            preferred_model: Some("deepseek-chat".to_string()), // Fast model for Q&A
            allowed_tools,
            read_only: true,
            streaming: true,
        }
    }

    /// Check if a tool is allowed in this mode
    pub fn is_tool_allowed(&self, tool_name: &str) -> bool {
        match &self.allowed_tools {
            None => true, // All tools allowed
            Some(allowed) => allowed.contains(tool_name),
        }
    }

    /// Filter tools based on mode configuration
    pub fn filter_tools(&self, tools: Vec<crate::berrycode::tools::Tool>) -> Vec<crate::berrycode::tools::Tool> {
        if self.allowed_tools.is_none() {
            return tools; // All tools allowed
        }

        tools
            .into_iter()
            .filter(|tool| self.is_tool_allowed(&tool.function.name))
            .collect()
    }

    /// Get system message for this mode
    pub fn get_system_message(&self) -> crate::berrycode::llm::Message {
        crate::berrycode::llm::Message {
            role: "system".to_string(),
            content: Some(self.system_prompt.clone()),
            tool_calls: None,
            tool_call_id: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mode_from_str() {
        assert_eq!(Mode::from_str("architect"), Some(Mode::Architect));
        assert_eq!(Mode::from_str("arch"), Some(Mode::Architect));
        assert_eq!(Mode::from_str("code"), Some(Mode::Code));
        assert_eq!(Mode::from_str("ask"), Some(Mode::Ask));
        assert_eq!(Mode::from_str("invalid"), None);
    }

    #[test]
    fn test_mode_display() {
        assert_eq!(Mode::Architect.display_name(), "🏗️  Architect Mode");
        assert_eq!(Mode::Code.display_name(), "⚡ Code Mode");
        assert_eq!(Mode::Ask.display_name(), "💬 Ask Mode");
    }

    #[test]
    fn test_architect_config() {
        let config = ModeConfig::architect();
        assert_eq!(config.mode, Mode::Architect);
        assert!(config.read_only);
        assert!(!config.streaming);
        assert!(config.is_tool_allowed("read_file"));
        assert!(config.is_tool_allowed("propose_plan"));
        assert!(!config.is_tool_allowed("write_file"));
        assert!(!config.is_tool_allowed("edit_file"));
    }

    #[test]
    fn test_code_config() {
        let config = ModeConfig::code();
        assert_eq!(config.mode, Mode::Code);
        assert!(!config.read_only);
        assert!(config.streaming);
        assert!(config.is_tool_allowed("write_file"));
        assert!(config.is_tool_allowed("edit_file"));
        assert!(config.is_tool_allowed("lint_code"));
    }

    #[test]
    fn test_ask_config() {
        let config = ModeConfig::ask();
        assert_eq!(config.mode, Mode::Ask);
        assert!(config.read_only);
        assert!(config.streaming);
        assert!(config.is_tool_allowed("read_file"));
        assert!(config.is_tool_allowed("grep"));
        assert!(!config.is_tool_allowed("write_file"));
        assert!(!config.is_tool_allowed("edit_file"));
    }
}
