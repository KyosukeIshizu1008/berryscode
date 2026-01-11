//! Input/Output handling for aider

use console::Style;
use dialoguer::Confirm;
use reedline::{Reedline, Signal, FileBackedHistory};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use crate::berrycode::Result;
use crate::berrycode::display::DisplayManager;

#[derive(Clone)]
pub struct InputOutput {
    pub pretty: bool,
    pub yes_always: Option<bool>,
    pub user_input_color: Option<String>,
    pub tool_output_color: Option<String>,
    pub tool_error_color: Option<String>,
    pub tool_warning_color: Option<String>,
    pub assistant_output_color: Option<String>,
    pub code_theme: String,
    pub encoding: String,
    pub dry_run: bool,
    pub num_error_outputs: usize,
    pub num_user_asks: usize,
    #[allow(dead_code)]
    input_history_file: Option<PathBuf>,
    #[allow(dead_code)]
    chat_history_file: Option<PathBuf>,
}

impl InputOutput {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        pretty: bool,
        yes_always: Option<bool>,
        input_history_file: Option<PathBuf>,
        chat_history_file: Option<PathBuf>,
        user_input_color: Option<String>,
        tool_output_color: Option<String>,
        tool_error_color: Option<String>,
        tool_warning_color: Option<String>,
        assistant_output_color: Option<String>,
        code_theme: String,
        encoding: String,
        dry_run: bool,
    ) -> Self {
        // Check NO_COLOR environment variable
        let no_color = std::env::var("NO_COLOR").is_ok();
        let pretty = if no_color { false } else { pretty };

        Self {
            pretty,
            yes_always,
            user_input_color: if pretty { user_input_color } else { None },
            tool_output_color: if pretty { tool_output_color } else { None },
            tool_error_color: if pretty { tool_error_color } else { None },
            tool_warning_color: if pretty { tool_warning_color } else { None },
            assistant_output_color,
            code_theme,
            encoding,
            dry_run,
            num_error_outputs: 0,
            num_user_asks: 0,
            input_history_file,
            chat_history_file,
        }
    }

    /// Output a message to the user
    pub fn tool_output(&self, message: &str) {
        if self.pretty {
            // Format specific types of messages
            self.print_tool_message(message);
        } else if let Some(color) = &self.tool_output_color {
            self.print_colored(message, color);
        } else {
            println!("{}", message);
        }
    }

    /// Print tool message with appropriate formatting
    fn print_tool_message(&self, message: &str) {
        use colored::*;

        // Check if this is a multi-line diff output
        if message.contains("diff --git") || message.contains("@@") ||
           (message.contains("\n") && (message.contains("\n+") || message.contains("\n-"))) {
            // This is likely a diff - use formatted output
            self.print_formatted_message(message);
            return;
        }

        // Check for specific message patterns
        if message.starts_with("  → ") {
            // Tool execution message - already formatted
            println!("{}", message);
        } else if message.contains("Update(") {
            // Update message with file path - format with bullet
            println!("{}", format!("● {}", message).green().bold());
        } else if message.contains("Updated ") && (message.contains("additions") || message.contains("removals")) {
            // Diff summary line
            println!("{}", format!("  └ {}", message).bright_black());
        } else {
            println!("{}", message);
        }
    }

    /// Output an error message
    pub fn tool_error(&mut self, message: &str) {
        self.num_error_outputs += 1;
        if self.pretty {
            let display = DisplayManager::new();
            display.print_error(message);
        } else if let Some(color) = &self.tool_error_color {
            self.print_colored(&format!("Error: {}", message), color);
        } else {
            eprintln!("Error: {}", message);
        }
    }

    /// Output a warning message
    pub fn tool_warning(&self, message: &str) {
        if self.pretty {
            let display = DisplayManager::new();
            display.print_warning(message);
        } else if let Some(color) = &self.tool_warning_color {
            self.print_colored(&format!("Warning: {}", message), color);
        } else {
            eprintln!("Warning: {}", message);
        }
    }

    /// Output assistant message with syntax highlighting
    pub fn ai_output(&self, message: &str) {
        if self.pretty {
            // Use DisplayManager for rich Markdown rendering
            let display = DisplayManager::new();
            display.print_ai_response(message);
        } else if let Some(color) = &self.assistant_output_color {
            self.print_colored(message, color);
        } else {
            println!("{}", message);
        }
    }

    /// Display user input in a beautiful box (Claude Code style)
    pub fn user_input_display(&self, message: &str) {
        if self.pretty {
            let display = DisplayManager::new();
            display.print_user_input(message);
        }
    }

    /// Print message with formatting (diff highlighting, bold, etc.)
    fn print_formatted_message(&self, message: &str) {
        use colored::*;

        for line in message.lines() {
            // Check for line numbers at the start (e.g., "   35  ", "   36 +", "   36 -")
            let (line_num_part, rest) = self.split_line_number(line);

            if !line_num_part.is_empty() {
                // Has line number - color it dim
                print!("{}", line_num_part.bright_black());

                // Check if the line number part contains a marker
                let has_plus_marker = line_num_part.contains('+');
                let has_minus_marker = line_num_part.contains('-');

                // Color the rest based on markers
                if has_plus_marker {
                    println!("{}", rest.green());
                } else if has_minus_marker {
                    println!("{}", rest.red());
                } else {
                    println!("{}", rest);
                }
            } else {
                // No line number - apply normal diff coloring
                if line.starts_with("+ ") || line.starts_with("+    ") {
                    // Added line (green)
                    println!("{}", line.green());
                } else if line.starts_with("- ") || line.starts_with("-    ") {
                    // Removed line (red)
                    println!("{}", line.red());
                } else if line.starts_with("@@ ") || line.starts_with("@@") {
                    // Diff hunk header (cyan)
                    println!("{}", line.cyan().bold());
                } else if line.starts_with("--- ") || line.starts_with("+++ ") {
                    // File markers (bold white)
                    println!("{}", line.white().bold());
                } else if line.starts_with("diff --git") {
                    // Git diff header (yellow)
                    println!("{}", line.yellow().bold());
                } else if line.starts_with("index ") {
                    // Index line (dim)
                    println!("{}", line.bright_black());
                } else if line.contains("Updated ") || line.contains("update") {
                    // Update messages (bold green)
                    println!("{}", line.green().bold());
                } else if line.trim().starts_with("**") && line.trim().ends_with("**") {
                    // Markdown bold (extract and bold)
                    let content = line.trim().trim_start_matches("**").trim_end_matches("**");
                    println!("{}", content.bold());
                } else if line.contains("**") {
                    // Inline bold - process mixed content
                    self.print_inline_formatted(line);
                } else {
                    // Regular line
                    println!("{}", line);
                }
            }
        }
    }

    /// Split line into line number part and rest
    /// Returns (line_number_part, rest)
    fn split_line_number(&self, line: &str) -> (String, String) {
        // Pattern examples:
        // "    35      let header = ..."
        // "    38 -    // Old comment"
        // "    38 +    // New comment"

        // Check if line starts with spaces and digits
        let bytes = line.as_bytes();
        if bytes.is_empty() {
            return (String::new(), line.to_string());
        }

        // Skip leading spaces
        let mut i = 0;
        while i < bytes.len() && bytes[i] == b' ' {
            i += 1;
        }

        if i >= bytes.len() {
            return (String::new(), line.to_string());
        }

        // Check if next chars are digits
        let start_digit = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }

        if i == start_digit {
            // No digits found
            return (String::new(), line.to_string());
        }

        // Skip one more space or marker after digits
        if i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'+' || bytes[i] == b'-') {
            i += 1;
        }

        // Include one more space if present
        if i < bytes.len() && bytes[i] == b' ' {
            i += 1;
        }

        // Split at this position
        let line_num_part = &line[..i];
        let rest = &line[i..];

        (line_num_part.to_string(), rest.to_string())
    }

    /// Print line with inline formatting (e.g., **bold**)
    fn print_inline_formatted(&self, line: &str) {
        use colored::*;

        let mut result = String::new();
        let mut chars = line.chars().peekable();
        let mut in_bold = false;

        while let Some(ch) = chars.next() {
            if ch == '*' && chars.peek() == Some(&'*') {
                chars.next(); // consume second *
                in_bold = !in_bold;
            } else if in_bold {
                result.push_str(&format!("{}", ch.to_string().bold()));
            } else {
                result.push(ch);
            }
        }

        println!("{}", result);
    }

    /// Print colored text
    fn print_colored(&self, message: &str, color_str: &str) {
        if self.pretty {
            let style = Self::parse_color(color_str);
            println!("{}", style.apply_to(message));
        } else {
            println!("{}", message);
        }
    }

    /// Parse color string to Style
    fn parse_color(color_str: &str) -> Style {
        let style = Style::new();

        // Handle hex colors
        if color_str.starts_with('#') {
            // console crate doesn't directly support hex colors,
            // so we'll use named colors as fallback
            return style;
        }

        // Handle named colors
        match color_str.to_lowercase().as_str() {
            "red" => style.red(),
            "green" => style.green(),
            "blue" => style.blue(),
            "yellow" => style.yellow(),
            "cyan" => style.cyan(),
            "magenta" => style.magenta(),
            "white" => style.white(),
            "black" => style.black(),
            _ => style,
        }
    }

    /// Ask user for confirmation
    pub fn confirm_ask(&mut self, message: &str) -> bool {
        self.num_user_asks += 1;

        if let Some(yes) = self.yes_always {
            return yes;
        }

        Confirm::new()
            .with_prompt(message)
            .default(false)
            .interact()
            .unwrap_or(false)
    }

    /// Get user input
    pub fn get_input(&mut self, prompt: &str) -> Result<String> {
        self.num_user_asks += 1;

        // Initialize reedline with history if available
        let mut line_editor = if let Some(ref history_file) = self.input_history_file {
            // Create history directory if it doesn't exist
            if let Some(parent) = history_file.parent() {
                fs::create_dir_all(parent)?;
            }

            // Use file-backed history
            let history = Box::new(
                FileBackedHistory::with_file(100, history_file.clone())
                    .map_err(|e| anyhow::anyhow!(
                        "コマンド履歴ファイルの作成に失敗しました: {}\n\
                        解決方法:\n\
                        1. ディレクトリの権限を確認してください: {:?}\n\
                        2. 十分なディスク容量があるか確認してください\n\
                        3. 別の場所に履歴ファイルを設定するには --history-path オプションを使用してください",
                        e, history_file.parent()
                    ))?
            );
            Reedline::create()
                .with_history(history)
                .use_bracketed_paste(true)  // Enable bracketed paste for multi-line input
        } else {
            Reedline::create()
                .use_bracketed_paste(true)  // Enable bracketed paste for multi-line input
        };

        // Print prompt
        print!("{}", prompt);
        io::stdout().flush()?;

        // Read line with history support
        let sig = line_editor.read_line(&reedline::DefaultPrompt::default())
            .map_err(|e| anyhow::anyhow!(
                "ユーザー入力の読み込みに失敗しました: {}\n\
                解決方法:\n\
                1. 端末の設定を確認してください (TERM={})\n\
                2. 標準入力がリダイレクトされていないか確認してください\n\
                3. 別の端末で試してみてください",
                e, std::env::var("TERM").unwrap_or_else(|_| "未設定".to_string())
            ))?;

        match sig {
            Signal::Success(buffer) => Ok(buffer),
            Signal::CtrlC => {
                println!("^C");
                Err(anyhow::anyhow!(
                    "操作がユーザーによって中断されました\n\
                    ヒント:\n\
                    - 続行するにはもう一度コマンドを実行してください\n\
                    - バッチモードで実行するには --yes オプションを使用してください\n\
                    - 対話モードをスキップするには --no-input オプションを使用してください"
                ))
            }
            Signal::CtrlD => {
                println!("^D");
                Err(anyhow::anyhow!(
                    "入力の終了 (EOF) が検出されました\n\
                    ヒント:\n\
                    - 標準入力が空または閉じられています\n\
                    - ファイルから入力をリダイレクトしている場合はファイルの内容を確認してください\n\
                    - 対話モードで続行するには端末から直接実行してください"
                ))
            }
        }
    }

    /// Read text from a file
    pub fn read_text(&self, path: &Path) -> Result<Option<String>> {
        match fs::read_to_string(path) {
            Ok(content) => Ok(Some(content)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Write text to a file
    pub fn write_text(&self, path: &Path, content: &str) -> Result<()> {
        if self.dry_run {
            self.tool_output(&format!("Dry run: would write to {}", path.display()));
            return Ok(());
        }

        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(path, content)?;
        Ok(())
    }

    /// Offer to open a URL
    pub fn offer_url(&mut self, url: &str, message: &str) -> bool {
        if self.confirm_ask(message) {
            if let Err(e) = webbrowser::open(url) {
                self.tool_error(&format!("Failed to open URL: {}", e));
                return false;
            }
            true
        } else {
            false
        }
    }

    /// Send bell notification
    pub fn bell(&self) {
        if self.pretty {
            print!("\x07");
            io::stdout().flush().ok();
        }
    }

    /// Print information about the files being worked on
    pub fn print_file_list(&self, files: &[PathBuf], read_only_files: &[PathBuf]) {
        if !files.is_empty() {
            self.tool_output("Files to edit:");
            for file in files {
                self.tool_output(&format!("  - {}", file.display()));
            }
        }

        if !read_only_files.is_empty() {
            self.tool_output("Read-only files:");
            for file in read_only_files {
                self.tool_output(&format!("  - {}", file.display()));
            }
        }
    }

    /// Print git repository information
    pub fn print_git_info(&self, git_root: &Path) {
        self.tool_output(&format!("Git repository: {}", git_root.display()));
    }

    /// Print model information
    pub fn print_model_info(&self, model_name: &str) {
        self.tool_output(&format!("Using model: {}", model_name));
    }
}

impl Default for InputOutput {
    fn default() -> Self {
        Self::new(
            true,
            None,
            None,
            None,
            Some("blue".to_string()),
            None,
            Some("red".to_string()),
            Some("#FFA500".to_string()),
            Some("blue".to_string()),
            "default".to_string(),
            "utf-8".to_string(),
            false,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    // Mutex to serialize tests that modify environment variables
    static ENV_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

    fn get_env_lock() -> &'static Mutex<()> {
        ENV_MUTEX.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn test_inputoutput_creation() {
        let _lock = get_env_lock().lock().unwrap();
        // Save current NO_COLOR state
        let prev_no_color = std::env::var("NO_COLOR").ok();
        std::env::remove_var("NO_COLOR");

        let io = InputOutput::new(
            true,
            None,
            None,
            None,
            Some("blue".to_string()),
            None,
            Some("red".to_string()),
            None,
            None,
            "default".to_string(),
            "utf-8".to_string(),
            false,
        );

        assert!(io.pretty);
        assert_eq!(io.encoding, "utf-8");
        assert!(!io.dry_run);

        // Restore previous state
        match prev_no_color {
            Some(val) => std::env::set_var("NO_COLOR", val),
            None => std::env::remove_var("NO_COLOR"),
        }
    }

    #[test]
    fn test_inputoutput_default() {
        let _lock = get_env_lock().lock().unwrap();
        // Save current NO_COLOR state
        let prev_no_color = std::env::var("NO_COLOR").ok();

        // Ensure NO_COLOR is not set
        std::env::remove_var("NO_COLOR");
        let io = InputOutput::default();
        assert!(io.pretty);
        assert_eq!(io.code_theme, "default");

        // Restore previous state
        match prev_no_color {
            Some(val) => std::env::set_var("NO_COLOR", val),
            None => std::env::remove_var("NO_COLOR"),
        }
    }

    #[test]
    fn test_parse_color() {
        let red_style = InputOutput::parse_color("red");
        // Style tests are hard to assert, but we can verify it doesn't panic
        let _ = red_style.apply_to("test");

        let blue_style = InputOutput::parse_color("blue");
        let _ = blue_style.apply_to("test");

        let hex_style = InputOutput::parse_color("#FF0000");
        let _ = hex_style.apply_to("test");
    }

    #[test]
    fn test_no_color_env() {
        // Acquire lock to prevent parallel tests from interfering
        let _lock = get_env_lock().lock().unwrap();

        // Save current state
        let prev_no_color = std::env::var("NO_COLOR").ok();

        // Test with NO_COLOR set
        std::env::set_var("NO_COLOR", "1");
        let io = InputOutput::new(
            true, // pretty = true initially
            None,
            None,
            None,
            Some("blue".to_string()),
            None,
            None,
            None,
            None,
            "default".to_string(),
            "utf-8".to_string(),
            false,
        );
        // With NO_COLOR set, pretty should be false
        assert!(!io.pretty);

        // Restore previous state
        match prev_no_color {
            Some(val) => std::env::set_var("NO_COLOR", val),
            None => std::env::remove_var("NO_COLOR"),
        }
    }

    #[test]
    fn test_split_line_number() {
        let io = InputOutput::default();

        // Line with number
        let (num_part, rest) = io.split_line_number("   35      let x = 1;");
        assert!(num_part.contains("35"));
        assert!(rest.contains("let"));

        // Line with number and plus marker
        // Note: The implementation separates line number part and rest
        // The + or - marker is part of the line number section
        let (num_part, rest) = io.split_line_number("   38 +    // New comment");
        assert!(num_part.contains("38"));
        // The marker may or may not be in num_part depending on implementation
        assert!(rest.contains("//") || num_part.contains("+"));

        // Line with number and minus marker
        let (num_part, rest) = io.split_line_number("   38 -    // Old comment");
        assert!(num_part.contains("38"));
        assert!(rest.contains("//") || num_part.contains("-"));

        // Line without number
        let (num_part, rest) = io.split_line_number("no number here");
        assert!(num_part.is_empty());
        assert!(rest.contains("no number"));
    }

    #[test]
    fn test_dry_run() {
        let io = InputOutput::new(
            true,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            "default".to_string(),
            "utf-8".to_string(),
            true, // dry_run = true
        );
        assert!(io.dry_run);
    }

    #[test]
    fn test_print_formatted_message_with_diff() {
        // Save current NO_COLOR state
        let _lock = get_env_lock().lock().unwrap();
        let prev_no_color = std::env::var("NO_COLOR").ok();
        std::env::remove_var("NO_COLOR");

        let io = InputOutput::default();

        // Test diff-like content formatting
        let diff_content = r#"--- a/file.txt
+++ b/file.txt
@@ -1,3 +1,3 @@
 unchanged line
-removed line
+added line
 unchanged line"#;

        // This should not panic
        io.print_formatted_message(diff_content);

        // Restore previous state
        match prev_no_color {
            Some(val) => std::env::set_var("NO_COLOR", val),
            None => std::env::remove_var("NO_COLOR"),
        }
    }

    #[test]
    fn test_print_formatted_message_with_line_numbers() {
        // Save current NO_COLOR state
        let _lock = get_env_lock().lock().unwrap();
        let prev_no_color = std::env::var("NO_COLOR").ok();
        std::env::remove_var("NO_COLOR");

        let io = InputOutput::default();

        // Test content with line numbers
        let content_with_numbers = r#"   35      let header = format!("BerryCode v{}", version);
   36 -    // Old comment
   36 +    // New comment
   37      println!("{}", header);"#;

        // This should not panic
        io.print_formatted_message(content_with_numbers);

        // Restore previous state
        match prev_no_color {
            Some(val) => std::env::set_var("NO_COLOR", val),
            None => std::env::remove_var("NO_COLOR"),
        }
    }

    #[test]
    fn test_print_tool_message() {
        // Save current NO_COLOR state
        let _lock = get_env_lock().lock().unwrap();
        let prev_no_color = std::env::var("NO_COLOR").ok();
        std::env::remove_var("NO_COLOR");

        let io = InputOutput::default();

        // Test various message types
        io.print_tool_message("  → Tool execution");
        io.print_tool_message("Update(file.rs)");
        io.print_tool_message("Updated 5 additions, 2 removals");

        // Restore previous state
        match prev_no_color {
            Some(val) => std::env::set_var("NO_COLOR", val),
            None => std::env::remove_var("NO_COLOR"),
        }
    }

    #[test]
    fn test_ai_output_with_formatting() {
        // Save current NO_COLOR state
        let _lock = get_env_lock().lock().unwrap();
        let prev_no_color = std::env::var("NO_COLOR").ok();
        std::env::remove_var("NO_COLOR");

        let io = InputOutput::default();

        // Test AI output with formatting
        let formatted_output = r#"Here's the code:
```rust
fn main() {
    println!("Hello");
}
```
**Important**: This is bold text."#;

        io.ai_output(formatted_output);

        // Restore previous state
        match prev_no_color {
            Some(val) => std::env::set_var("NO_COLOR", val),
            None => std::env::remove_var("NO_COLOR"),
        }
    }

    #[test]
    fn test_tool_output_with_diff() {
        // Save current NO_COLOR state
        let _lock = get_env_lock().lock().unwrap();
        let prev_no_color = std::env::var("NO_COLOR").ok();
        std::env::remove_var("NO_COLOR");

        let io = InputOutput::default();

        let diff_message = r#"diff --git a/src/main.rs b/src/main.rs
index abc123..def456 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,5 +1,5 @@
 fn main() {
-    println!("Old");
+    println!("New");
 }"#;

        io.tool_output(diff_message);

        // Restore previous state
        match prev_no_color {
            Some(val) => std::env::set_var("NO_COLOR", val),
            None => std::env::remove_var("NO_COLOR"),
        }
    }

    #[test]
    fn test_tool_error() {
        // Save current NO_COLOR state
        let _lock = get_env_lock().lock().unwrap();
        let prev_no_color = std::env::var("NO_COLOR").ok();
        std::env::remove_var("NO_COLOR");

        let mut io = InputOutput::default();
        let initial_errors = io.num_error_outputs;

        io.tool_error("Test error message");

        assert_eq!(io.num_error_outputs, initial_errors + 1);

        // Restore previous state
        match prev_no_color {
            Some(val) => std::env::set_var("NO_COLOR", val),
            None => std::env::remove_var("NO_COLOR"),
        }
    }

    #[test]
    fn test_tool_warning() {
        // Save current NO_COLOR state
        let _lock = get_env_lock().lock().unwrap();
        let prev_no_color = std::env::var("NO_COLOR").ok();
        std::env::remove_var("NO_COLOR");

        let io = InputOutput::default();

        io.tool_warning("Test warning message");

        // Restore previous state
        match prev_no_color {
            Some(val) => std::env::set_var("NO_COLOR", val),
            None => std::env::remove_var("NO_COLOR"),
        }
    }

    #[test]
    fn test_print_inline_formatted() {
        // Save current NO_COLOR state
        let _lock = get_env_lock().lock().unwrap();
        let prev_no_color = std::env::var("NO_COLOR").ok();
        std::env::remove_var("NO_COLOR");

        let io = InputOutput::default();

        // Test inline formatting with bold
        io.print_inline_formatted("This is **bold** and **more bold** text");
        io.print_inline_formatted("No formatting here");

        // Restore previous state
        match prev_no_color {
            Some(val) => std::env::set_var("NO_COLOR", val),
            None => std::env::remove_var("NO_COLOR"),
        }
    }
}
