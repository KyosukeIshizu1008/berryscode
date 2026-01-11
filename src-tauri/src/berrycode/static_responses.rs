//! Static responses for instant replies without LLM invocation
//!
//! This module provides pre-generated responses for common queries that don't
//! require AI processing. This dramatically improves response time (from ~9s to ~0.002s)
//! for help and documentation requests.

/// BerryCode options help text (generated from `berrycode --help`)
///
/// This is returned instantly when users ask for "オプション全部", "help", etc.
/// instead of invoking the LLM and running semantic search.
pub const OPTIONS_HELP_TEXT: &str = r#"
📚 BerryCode Complete Options Reference
=======================================

BerryCode - AI-powered pair programming assistant

Quick Start:
  berrycode                          # Start interactive session
  berrycode src/main.rs              # Edit specific file
  berrycode --model deepseek-chat    # Use specific model

Environment Variables:
  BERRYCODE_MODEL              Default model to use (e.g., qwen3-coder:30b, llama4:scout)
  BERRYCODE_CLASSIFIER_MODEL   Lightweight model for auto task type detection (default: qwen3-coder:30b)
  BERRYCODE_MODE               Default mode (architect/code/ask)
  OPENAI_API_KEY               OpenAI API key (use any string for local Ollama)
  ANTHROPIC_API_KEY            Anthropic API key
  OPENAI_API_BASE              Custom API base URL (default: http://KyosukenoMac-Studio.local:11434/v1)

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

## 📁 Files & Basic Usage

  [FILE]...
      Specify files to edit. Without files, starts interactive mode.
      Examples:
        berrycode src/main.rs
        berrycode Cargo.toml README.md

  -h, --help              Print help
  -V, --version           Print version
  -v, --verbose           Show detailed operation logs

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

## 🤖 Model Settings

  --model <MODEL>
      Main AI model for chat.
      Examples: gpt-4, claude-3-opus, deepseek-chat
      [env: BERRYCODE_MODEL]

  --mode <MODE>
      Operation mode: architect | code | ask
      [env: BERRYCODE_MODE]

  --weak-model <MODEL>
      Faster model for simple tasks (file listing, searches)

  --editor-model <MODEL>
      Specialized model for code editing

  --edit-format <FORMAT>
      Code edit format: whole | diff | diff-fenced | udiff
      (Auto-selected by default)

  --editor-edit-format <FORMAT>
      Edit format override for editor model

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

## 🔑 API Keys & Authentication

  --openai-api-key <KEY>
      OpenAI API key (also for DeepSeek, etc.)
      [env: OPENAI_API_KEY]

  --anthropic-api-key <KEY>
      Anthropic API key for Claude models
      [env: ANTHROPIC_API_KEY]

  --openai-api-base <URL>
      Custom API endpoint
      Examples:
        https://api.deepseek.com
        http://localhost:11434/v1
      [env: OPENAI_API_BASE]

  --set-env <ENV_VAR=value>
      Set environment variable (can be used multiple times)

  --api-key <PROVIDER=KEY>
      Set API key for provider

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

## 📝 Model Configuration

  --models <MODEL>
      List known models matching name

  --model-settings-file <FILE>
      Custom model settings file
      [default: .berrycode.model.settings.yml]

  --model-metadata-file <FILE>
      Model context window & costs file
      [default: .berrycode.model.metadata.json]

  --alias <ALIAS:MODEL>
      Add model alias (can be used multiple times)

  --reasoning-effort <EFFORT>
      Set reasoning_effort API parameter

  --thinking-tokens <TOKENS>
      Set thinking token budget

  --check-model-accepts-settings
      Check model compatibility before applying settings

  --show-model-warnings
      Display model capability warnings (default: true)

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

## 🔧 Git Integration

  --git                  Enable git integration (default: true)
  --no-git               Disable git

  --auto-commits         Auto-create commits after changes (default: true)
  --no-auto-commits      Disable auto-commits

  --dirty-commits        Prompt before committing dirty working dir (default: true)
  --no-dirty-commits     Allow commits even with uncommitted changes

  --dry-run              Preview mode - don't actually modify files

  --git-root <PATH>      Use different git repository path

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

## 💬 Chat History & Context

  --input-history-file <FILE>
      Command history file
      [default: .berrycode.input.history]

  --chat-history-file <FILE>
      Full conversation history (Markdown)
      [default: .berrycode.chat.history.md]

  --restore-chat-history
      Continue from previous session

  --context-lines <N>
      Lines of context around code snippets
      [default: 2]

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

## 🎨 Output & Display

  --vim                  Use VI keybindings (default: Emacs mode)
  --dark-mode            Dark terminal color scheme
  --light-mode           Light terminal color scheme

  --user-input-color <COLOR>
      Customize user input color
      [env: BERRYCODE_USER_INPUT_COLOR]

  --tool-error-color <COLOR>
      Customize tool error color
      [env: BERRYCODE_TOOL_ERROR_COLOR]

  --tool-warning-color <COLOR>
      Customize tool warning color
      [env: BERRYCODE_TOOL_WARNING_COLOR]

  --assistant-output-color <COLOR>
      Customize assistant response color
      [env: BERRYCODE_ASSISTANT_OUTPUT_COLOR]

  --code-theme <THEME>
      Syntax highlighting theme
      Options: default, monokai, solarized, github
      [default: default]

  --yes-always <true|false>
      Auto-approve all prompts (use with caution!)

  --copy-paste
      Optimize for web chat copy-paste

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

## 🔍 Linting & Testing

  --lint-cmd <LANG:CMD>
      Language-specific linter
      Examples:
        --lint-cmd rust:"cargo clippy"
        --lint-cmd python:"flake8"

  --auto-lint            Auto-run linters after changes

  --test-cmd <CMD>
      Test command
      Examples:
        --test-cmd "cargo test"
        --test-cmd "npm test"

  --auto-test            Auto-run tests after changes

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

## 👁️ File Watching & Input

  --watch                Watch files for external changes
  --voice                Enable voice-to-text input

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

## ⚙️ System & Configuration

  --timeout <SECONDS>
      API request timeout

  --verify-ssl           Verify SSL certificates (default: true)
  --no-verify-ssl        Disable SSL verification (⚠️ security risk!)

  --encoding <ENCODING>
      File encoding
      [default: utf-8]

  -c, --config <FILE>
      Load config from YAML/JSON file

  --env-file <FILE>
      Load environment variables from file
      [default: .env]

  --shell-completions <SHELL>
      Generate shell completions
      Shells: bash, zsh, fish, powershell, elvish

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

## 📂 File Ignoring

  --gitignore            Respect .gitignore (default: true)
  --no-gitignore         Show all files

  --berrycodeignore <FILE>
      BerryCode ignore file
      [default: .berrycodeignore]

  --berrycodeignore-extra <FILE>
      Additional ignore files

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

## 🚀 Upgrade & Maintenance

  --upgrade              Upgrade to latest release
  --install-main-branch  Install from main branch (⚠️ unstable!)
  --check-update         Check for new version
  --analytics-disable    Disable telemetry permanently

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

## 🖥️ GUI & Special Modes

  --gui                  Launch GUI mode
  --apply <FILE>         Apply changes from diff file and exit

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

💡 Tips:
  • For detailed help on any option: berrycode --help
  • Most options have environment variable equivalents
  • Use --dry-run to preview changes safely
  • Combine --auto-lint and --auto-test for TDD workflow

📖 Full documentation: https://github.com/your-repo/berrycode
"#;

/// Check if user input is asking for help/options
///
/// Returns true for queries like:
/// - "オプション全部教えて"
/// - "help"
/// - "ヘルプ"
/// - "全オプション"
/// - etc.
pub fn is_help_request(input: &str) -> bool {
    let lower = input.to_lowercase();
    let trimmed = input.trim().to_lowercase();

    // Exact matches (case-insensitive)
    if trimmed == "help" || trimmed == "ヘルプ" {
        return true;
    }

    // Pattern matches for Japanese
    if (lower.contains("オプション") && (lower.contains("全部") || lower.contains("一覧") || lower.contains("教え")))
        || (lower.contains("option") && (lower.contains("all") || lower.contains("list")))
        || lower.contains("全オプション")
    {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_help_request() {
        assert!(is_help_request("オプション全部教えて"));
        assert!(is_help_request("help"));
        assert!(is_help_request("HELP"));
        assert!(is_help_request("ヘルプ"));
        assert!(is_help_request("全オプション"));
        assert!(is_help_request("オプション一覧"));
        assert!(is_help_request("show all options"));

        assert!(!is_help_request("create a new file"));
        assert!(!is_help_request("fix the bug"));
    }

    #[test]
    fn test_options_help_text_not_empty() {
        assert!(!OPTIONS_HELP_TEXT.is_empty());
        assert!(OPTIONS_HELP_TEXT.contains("BerryCode"));
        assert!(OPTIONS_HELP_TEXT.contains("--model"));
    }
}
