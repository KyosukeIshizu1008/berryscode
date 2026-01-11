//! Rich terminal display manager for BerryCode
//!
//! Provides Claude Code-style beautiful output with:
//! - Markdown rendering with syntax highlighting
//! - Visual separation between user input and AI output
//! - Animated spinners for thinking states
//! - Color-coded code blocks with backgrounds
//!
//! ## Inspired by world-class CLI tools
//!
//! - **Broot** (github.com/Canop/broot) - Master reference for termimad usage
//!   Created by the author of termimad, showcases the best practices for
//!   rich TUI with Markdown rendering, tables, and styled output.
//!
//! - **Bat** (github.com/sharkdp/bat) - Reference for syntax highlighting
//!   World-class implementation of code syntax highlighting using syntect.
//!   Shows how to beautifully render code with proper colors per language.
//!
//! - **GitHub CLI** (github.com/cli/cli) - Reference for UX design
//!   Excellent CLI user experience with spinners, prompts, and workflows.
//!   Gold standard for modern CLI interaction patterns.

use termimad::{MadSkin, StyledChar, crossterm::style::Color};
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

/// Manager for rich terminal display
pub struct DisplayManager {
    skin: MadSkin,
}

impl DisplayManager {
    /// Create a new DisplayManager with Claude Code-style theming
    /// Inspired by Anthropic's Claude brand colors and Broot (github.com/Canop/broot)
    pub fn new() -> Self {
        let mut skin = MadSkin::default();

        // --- 🎨 Anthropic (Claude) 風テーマ ---
        // Reference: Canop/broot for termimad best practices
        // Reference: sharkdp/bat for syntax highlighting inspiration

        // 1. 基本カラー (少し温かみのある白)
        skin.set_headers_fg(Color::Rgb { r: 230, g: 230, b: 230 });
        skin.paragraph.set_fg(Color::Rgb { r: 210, g: 210, b: 210 });

        // 2. アクセントカラー (Anthropicのブランドカラーっぽいテラコッタ色)
        let accent_color = Color::Rgb { r: 217, g: 119, b: 87 };
        skin.bold.set_fg(accent_color);
        skin.italic.set_fg(Color::Rgb { r: 180, g: 180, b: 180 });

        // 3. コードブロック (ここが一番大事！)
        // 背景: 深いチャコールグレー (Claude Codeと同じ色味)
        skin.code_block.set_bg(Color::Rgb { r: 28, g: 28, b: 30 });
        // 文字: 明るいグレー
        skin.code_block.set_fg(Color::Rgb { r: 220, g: 220, b: 220 });

        // 左の装飾線: アクセントカラーを使うとおしゃれ
        skin.code_block.left_margin = 2;
        skin.code_block.align = termimad::Alignment::Left;

        // 4. インラインコード (`code`)
        // 背景: ダークグレー
        skin.inline_code.set_bg(Color::Rgb { r: 50, g: 50, b: 50 });
        // 文字: 薄いオレンジ（視認性が高い）
        skin.inline_code.set_fg(Color::Rgb { r: 255, g: 200, b: 150 });

        // 5. 引用 (> quote) - アクセントカラーでおしゃれに
        skin.quote_mark = StyledChar::from_fg_char(accent_color, '┃');

        Self { skin }
    }

    /// ユーザーの入力を表示（Claude Code風：矢印付き）
    pub fn print_user_input(&self, input: &str) {
        println!("\n╭── 👤 You ───────────────────────────────");
        println!("│ > {}", input);
        println!("╰─────────────────────────────────────────\n");
    }

    /// AIの応答をMarkdownとしてリッチに表示
    pub fn print_ai_response(&self, markdown_text: &str) {
        println!("╭── 🤖 BerryCode ──────────────────────────");
        // Markdownを解析して綺麗に表示
        self.skin.print_text(markdown_text);
        println!("╰──────────────────────────────────────────\n");
    }

    /// ツール実行メッセージを表示
    pub fn print_tool_execution(&self, tool_name: &str, info: &str) {
        use colored::Colorize;
        println!("{}", format!("  → {}: {}", tool_name, info).bright_black());
    }

    /// エラーメッセージを表示
    pub fn print_error(&self, message: &str) {
        use colored::Colorize;
        println!("\n{} {}", "✗".red().bold(), message.red());
    }

    /// 警告メッセージを表示
    pub fn print_warning(&self, message: &str) {
        use colored::Colorize;
        println!("\n{} {}", "⚠".yellow().bold(), message.yellow());
    }

    /// 成功メッセージを表示
    pub fn print_success(&self, message: &str) {
        use colored::Colorize;
        println!("{} {}", "✓".green().bold(), message.green());
    }

    /// 思考中のスピナーを表示（indicatifを使用）
    pub fn show_spinner(&self, message: &str) -> ProgressBar {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ ") // くるくる回る文字
                .template("{spinner:.green} {msg}")
                .unwrap()
        );
        pb.set_message(message.to_string());
        pb.enable_steady_tick(Duration::from_millis(100));
        pb
    }

    /// 情報メッセージを表示
    pub fn print_info(&self, message: &str) {
        use colored::Colorize;
        println!("{}", message.bright_black());
    }

    /// セクションヘッダーを表示
    pub fn print_section(&self, title: &str) {
        use colored::Colorize;
        println!("\n{}", title.cyan().bold());
        println!("{}", "─".repeat(title.len()).cyan());
    }

    // ═══════════════════════════════════════════════════════════════════
    // Claude Code-style Action Log (行動ログ)
    // ═══════════════════════════════════════════════════════════════════

    /// ツール実行のアクションログを表示する (Search, Update, Read, Bash等)
    ///
    /// # Example
    /// ```
    /// # use berrycode::display::DisplayManager;
    /// let display = DisplayManager::new();
    /// display.print_action("Search", "pattern: \"...\", glob: \"...\"", true);
    /// ```
    ///
    /// 表示例: ● Search(pattern: "...", glob: "...")
    pub fn print_action(&self, action_name: &str, args: &str, is_success: bool) {
        use colored::Colorize;

        // Claude Code風: 成功=緑●, エラー=赤●
        let bullet = if is_success {
            "●".green()
        } else {
            "●".red()
        };

        println!(
            "{} {}{}{}{}",
            bullet,
            action_name,  // 太字なし（Claude Codeと同じ）
            "(".bright_black(),
            args,
            ")".bright_black()
        );
    }

    /// アクションの結果や補足情報をツリー状に表示する
    ///
    /// # Example
    /// ```
    /// # use berrycode::display::DisplayManager;
    /// let display = DisplayManager::new();
    /// display.print_sub_result("Found 69 lines", true);
    /// ```
    ///
    /// 表示例:   └ Found 69 lines
    pub fn print_sub_result(&self, message: &str, is_success: bool) {
        use colored::Colorize;

        // Claude Code風: 成功時は薄いグレー、エラー時は赤
        let formatted_message = if is_success {
            message.bright_black()
        } else {
            message.red()
        };

        println!(
            "  {} {}",
            "└".bright_black(), // ツリー記号は目立たない色で
            formatted_message
        );
    }

    /// AIの発言や思考を表示する (引数カッコなし版)
    ///
    /// # Example
    /// ```
    /// # use berrycode::display::DisplayManager;
    /// let display = DisplayManager::new();
    /// display.print_agent_message("問題がわかりました。...");
    /// ```
    ///
    /// 表示例: ● 問題がわかりました。...
    pub fn print_agent_message(&self, message: &str) {
        use colored::Colorize;
        println!(
            "{} {}",
            "●".magenta(), // 色を変えて区別
            message
        );
    }

    // --- 便利なラッパー関数 ---

    /// Search専用のアクションログ
    pub fn log_search(&self, pattern: &str, glob: &str) {
        let args = format!("pattern: {:?}, path: {:?}", pattern, glob);
        self.print_action("Search", &args, true);
    }

    /// Bash専用のアクションログ
    pub fn log_bash(&self, command: &str) {
        self.print_action("Bash", command, true);
    }

    /// Update専用のアクションログ
    pub fn log_update(&self, path: &str) {
        self.print_action("Update", path, true);
    }

    /// Update専用のアクションログ（エラーあり）
    pub fn log_update_error(&self, path: &str) {
        self.print_action("Update", path, false);
    }

    /// 汎用アクションログ（任意のメッセージ）
    pub fn log_action(&self, message: &str) {
        use colored::Colorize;
        println!("{}", format!("● {}", message).bright_black());
    }

    /// Read専用のアクションログ
    pub fn log_read(&self, path: &str) {
        self.print_action("Read", path, true);
    }

    // ═══════════════════════════════════════════════════════════════════
    // Claude Code-style Progress Indicators (進捗インジケーター)
    // ═══════════════════════════════════════════════════════════════════

    /// 省略された出力を表示
    ///
    /// # Example
    /// ```
    /// # use berrycode::display::DisplayManager;
    /// let display = DisplayManager::new();
    /// display.print_collapsed_output(2);
    /// ```
    ///
    /// 表示例: … +2 lines (ctrl+o to expand)
    pub fn print_collapsed_output(&self, line_count: usize) {
        use colored::Colorize;
        println!(
            "{}",
            format!("… +{} lines (ctrl+o to expand)", line_count).bright_black()
        );
    }

    /// 思考時間を表示
    ///
    /// # Example
    /// ```
    /// # use berrycode::display::DisplayManager;
    /// let display = DisplayManager::new();
    /// display.print_thinking_time(3);
    /// ```
    ///
    /// 表示例: ∴ Thought for 3s (ctrl+o to show thinking)
    pub fn print_thinking_time(&self, seconds: u64) {
        use colored::Colorize;
        println!(
            "{}",
            format!("∴ Thought for {}s (ctrl+o to show thinking)", seconds).bright_black()
        );
    }

    /// 実行中の進捗を表示（リアルタイム更新）
    ///
    /// # Example
    /// ```
    /// # use berrycode::display::DisplayManager;
    /// let display = DisplayManager::new();
    /// display.print_envisioning("Envisioning...", 33, 836);
    /// ```
    ///
    /// 表示例: * Envisioning... (esc to interrupt · 33s · ↑ 836 tokens)
    pub fn print_envisioning(&self, message: &str, elapsed_seconds: u64, tokens: usize) {
        use colored::Colorize;

        // メッセージ部分はオレンジ色（Claude のアクセントカラー）
        let message_colored = message.truecolor(217, 119, 87); // Anthropic orange

        // 括弧内の詳細は薄いグレー
        let details = format!(
            "(esc to interrupt · {}s · ↑ {} tokens)",
            elapsed_seconds,
            tokens
        ).bright_black();

        print!("\r* {} {}", message_colored, details);
        // フラッシュして即座に表示
        use std::io::{self, Write};
        io::stdout().flush().unwrap();
    }

    /// 進捗表示をクリア（次の行に移動）
    pub fn clear_progress_line(&self) {
        println!(); // 改行して次の行へ
    }

    /// 差分表示（Claude Code風）
    ///
    /// # Example
    /// ```
    /// # use berrycode::display::DisplayManager;
    /// let display = DisplayManager::new();
    /// display.print_diff_line(84, Some("-"), "    petgraph = \"0.6\"");
    /// display.print_diff_line(84, Some("+"), "    petgraph = { version = \"0.6\", features = [\"serde-1\"] }");
    /// ```
    ///
    /// 表示例:
    ///   84 -    petgraph = "0.6"
    ///   84 +    petgraph = { version = "0.6", features = ["serde-1"] }
    pub fn print_diff_line(&self, line_num: usize, change: Option<&str>, content: &str) {
        use colored::Colorize;

        match change {
            Some("-") => {
                // 削除行: 赤色
                println!(
                    "  {:>4} {} {}",
                    line_num.to_string().bright_black(),
                    "-".red(),
                    content.red()
                );
            }
            Some("+") => {
                // 追加行: 緑色
                println!(
                    "  {:>4} {} {}",
                    line_num.to_string().bright_black(),
                    "+".green(),
                    content.green()
                );
            }
            _ => {
                // 変更なし: 通常色
                println!(
                    "  {:>4}   {}",
                    line_num.to_string().bright_black(),
                    content
                );
            }
        }
    }

    /// 差分のサマリーを表示（Claude Code風）
    ///
    /// # Example
    /// ```
    /// # use berrycode::display::DisplayManager;
    /// let display = DisplayManager::new();
    /// display.print_diff_summary("Cargo.toml", 1, 1);
    /// ```
    ///
    /// 表示例: └ Updated Cargo.toml with 1 addition and 1 removal
    pub fn print_diff_summary(&self, file_path: &str, additions: usize, removals: usize) {
        let summary = if additions == 0 && removals == 0 {
            format!("Updated {} (no changes)", file_path)
        } else if removals == 0 {
            format!(
                "Updated {} with {} addition{}",
                file_path,
                additions,
                if additions == 1 { "" } else { "s" }
            )
        } else if additions == 0 {
            format!(
                "Updated {} with {} removal{}",
                file_path,
                removals,
                if removals == 1 { "" } else { "s" }
            )
        } else {
            format!(
                "Updated {} with {} addition{} and {} removal{}",
                file_path,
                additions,
                if additions == 1 { "" } else { "s" },
                removals,
                if removals == 1 { "" } else { "s" }
            )
        };

        self.print_sub_result(&summary, true);
    }
}

impl Default for DisplayManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_manager_creation() {
        let display = DisplayManager::new();
        // Just verify it doesn't panic
        display.print_info("Test message");
    }

    #[test]
    fn test_user_input_display() {
        let display = DisplayManager::new();
        display.print_user_input("Hello, BerryCode!");
    }

    #[test]
    fn test_ai_response_display() {
        let display = DisplayManager::new();
        let markdown = r#"
# Response

Here is the **answer**:

```rust
fn main() {
    println!("Hello!");
}
```

This is `inline code`.
"#;
        display.print_ai_response(markdown);
    }

    #[test]
    fn test_spinner() {
        let display = DisplayManager::new();
        let spinner = display.show_spinner("Thinking...");
        std::thread::sleep(std::time::Duration::from_millis(100));
        spinner.finish_and_clear();
    }
}
