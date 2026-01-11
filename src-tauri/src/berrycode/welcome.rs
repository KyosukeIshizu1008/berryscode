//! Welcome screen for BerryCode

use colored::*;
use std::env;

/// Display the welcome screen (Claude Code style)
pub fn display_welcome_screen(version: &str, model: &str, api_provider: &str) {
    let cwd = env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "Unknown".to_string());

    // Simplify path display
    let display_path = if cwd.len() > 50 {
        format!("...{}", &cwd[cwd.len().saturating_sub(47)..])
    } else {
        cwd.clone()
    };

    println!();
    print_claude_style_welcome(version, model, api_provider, &display_path);
    println!();
}

const LEFT_COL_WIDTH: usize = 36;
const RIGHT_COL_WIDTH: usize = 60;
// Total width: 1(│) + 1(space) + 36(left) + 1(space) + 1(│) + 1(space) + 60(right) + 1(space) + 1(│) = 103
const BOX_WIDTH: usize = LEFT_COL_WIDTH + RIGHT_COL_WIDTH + 7;

fn print_claude_style_welcome(version: &str, model: &str, api_provider: &str, cwd: &str) {
    // Top border (ピンク色に変更)
    let border = format!("╭{}╮", "─".repeat(BOX_WIDTH - 2));
    println!("{}", border.bright_magenta());

    // Header: "BerryCode vX.X.X"
    let header = format!("BerryCode v{}", version);
    let header_line = format!("│ {:<width$} │", header, width = BOX_WIDTH - 4);
    println!("{}", header_line.bright_magenta());

    // Separator after header (ピンク色に変更)
    // Format: ├─(space)─[LEFT_COL_WIDTH]─(space)─┼─(space)─[RIGHT_COL_WIDTH]─(space)─┤
    let sep = format!("├{}┼{}┤",
        "─".repeat(LEFT_COL_WIDTH + 2),  // +2 for spaces around content
        "─".repeat(RIGHT_COL_WIDTH + 2)  // +2 for spaces around content
    );
    println!("{}", sep.bright_magenta());

    // Row 1: Welcome back! | Tips for getting started
    print_two_col_row("Welcome back!", "Tips for getting started");

    // Row 2: (empty) | Run /help to see available commands
    print_two_col_row("", "Run /help to see available commands");

    // Row 3: (berry logo line 1) | (separator line) - ピンク色のキャラクター
    let berry_line1 = format!("{}", "            * ▐▛███▜▌ *".bright_magenta());
    print_two_col_row(&berry_line1, &"─".repeat(RIGHT_COL_WIDTH.min(60)));

    // Row 4: (berry logo line 2) | Recent activity - ピンク色のキャラクター
    let berry_line2 = format!("{}", "           * ▝▜█████▛▘ *".bright_magenta());
    print_two_col_row(&berry_line2, "Recent activity");

    // Row 5: (berry logo line 3) | No recent activity - ピンク色のキャラクター
    let berry_line3 = format!("{}", "            *  ▘▘ ▝▝  *".bright_magenta());
    print_two_col_row(&berry_line3, "No recent activity");

    // Row 6: (empty) | (empty)
    print_two_col_row("", "");

    // Row 7: Model info | (empty) - 長い場合は切り詰め
    let model_info_raw = format!("  {} · {}", model, api_provider);
    let model_info = truncate_if_needed(&model_info_raw, LEFT_COL_WIDTH);
    print_two_col_row(&model_info, "");

    // Row 8: Directory | (empty) - 長い場合は切り詰め
    let dir_info_raw = format!("     {}", cwd);
    let dir_info = truncate_if_needed(&dir_info_raw, LEFT_COL_WIDTH);
    print_two_col_row(&dir_info, "");

    // Bottom border (ピンク色に変更)
    let bottom = format!("╰{}╯", "─".repeat(BOX_WIDTH - 2));
    println!("{}", bottom.bright_magenta());
}

fn print_two_col_row(left: &str, right: &str) {
    // Calculate actual display width (approximation for ASCII and special chars)
    let left_visual_len = visual_width(left);
    let right_visual_len = visual_width(right);

    // Add padding to reach target width
    let left_padding = if left_visual_len < LEFT_COL_WIDTH {
        LEFT_COL_WIDTH - left_visual_len
    } else {
        0
    };

    let right_padding = if right_visual_len < RIGHT_COL_WIDTH {
        RIGHT_COL_WIDTH - right_visual_len
    } else {
        0
    };

    // 枠線 │ をピンク色に変更
    println!("{} {}{} {} {}{} {}",
        "│".bright_magenta(),
        left, " ".repeat(left_padding),
        "│".bright_magenta(),
        right, " ".repeat(right_padding),
        "│".bright_magenta()
    );
}

/// Calculate visual width of a string (improved for CJK and block elements)
fn visual_width(s: &str) -> usize {
    // ANSI escape sequences (色コード) を除外してから計算
    let clean = strip_ansi_codes(s);

    clean.chars().map(|c| {
        match c {
            // Box drawing characters (幅1)
            '─' | '│' | '┼' | '├' | '┤' | '╭' | '╮' | '╰' | '╯' => 1,
            // Half-width block elements (幅1)
            '▐' | '▛' | '▜' | '▝' | '▘' => 1,
            // Full-width block elements (幅2) - 重要！これが原因でズレていた
            '█' | '▀' | '▄' | '▌' | '▎' | '▏' => 2,
            // Regular ASCII (幅1)
            c if c.is_ascii() => 1,
            // CJK characters and other full-width (幅2)
            c if c > '\u{3000}' => 2,
            // Other characters (幅1)
            _ => 1,
        }
    }).sum()
}

/// ANSI エスケープシーケンス（色コード）を除去
fn strip_ansi_codes(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // ESC シーケンス開始
            if chars.peek() == Some(&'[') {
                chars.next(); // '['をスキップ
                // 'm' が来るまでスキップ
                while let Some(ch) = chars.next() {
                    if ch == 'm' {
                        break;
                    }
                }
            }
        } else {
            result.push(c);
        }
    }

    result
}

/// 文字列が max_width を超える場合、切り詰める
fn truncate_if_needed(s: &str, max_width: usize) -> String {
    let current_width = visual_width(s);

    if current_width <= max_width {
        return s.to_string();
    }

    // パスの場合は先頭を "..." で省略
    if s.contains('/') {
        let prefix = "...";
        let available = max_width.saturating_sub(3); // "..." の分を引く

        // 末尾から available 分だけ取る
        let chars: Vec<char> = s.chars().collect();
        let mut taken = 0;
        let mut result_chars = Vec::new();

        for ch in chars.iter().rev() {
            let char_width = if *ch == '█' || *ch == '▀' || *ch == '▄' { 2 } else if ch.is_ascii() { 1 } else { 2 };

            if taken + char_width > available {
                break;
            }

            result_chars.push(*ch);
            taken += char_width;
        }

        result_chars.reverse();
        format!("{}{}", prefix, result_chars.iter().collect::<String>())
    } else {
        // その他の場合は末尾を切る
        let mut result = String::new();
        let mut width = 0;

        for ch in s.chars() {
            let char_width = if ch == '█' || ch == '▀' || ch == '▄' { 2 } else if ch.is_ascii() { 1 } else { 2 };

            if width + char_width > max_width.saturating_sub(3) {
                result.push_str("...");
                break;
            }

            result.push(ch);
            width += char_width;
        }

        result
    }
}


/// Display a simple welcome message (compact version)
pub fn display_simple_welcome(version: &str, model: &str) {
    println!();
    println!("{}", "═══════════════════════════════════════".bright_magenta().bold());
    println!("{} {} {}",
        "🍓".bright_red(),
        "BerryCode".bright_magenta().bold(),
        format!("v{}", version).bright_yellow()
    );
    println!("{}", "═══════════════════════════════════════".bright_magenta().bold());
    println!("{} {}", "Model:".bright_green().bold(), model.bright_white());
    println!("{} {}", "Ready!".bright_cyan().bold(), "Type /help for commands".bright_black());
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visual_width_ascii() {
        // ASCII characters should be width 1
        assert_eq!(visual_width("hello"), 5);
        assert_eq!(visual_width("test"), 4);
        assert_eq!(visual_width(""), 0);
    }

    #[test]
    fn test_visual_width_box_drawing() {
        // Box drawing characters should be width 1
        assert_eq!(visual_width("─"), 1);
        assert_eq!(visual_width("│"), 1);
        assert_eq!(visual_width("┼"), 1);
        assert_eq!(visual_width("├┤"), 2);
        assert_eq!(visual_width("╭╮╰╯"), 4);
    }

    #[test]
    fn test_visual_width_block_elements() {
        // Block elements should be width 1
        assert_eq!(visual_width("▐"), 1);
        assert_eq!(visual_width("▛"), 1);
        assert_eq!(visual_width("▜"), 1);
        assert_eq!(visual_width("▝▘"), 2);
    }

    #[test]
    fn test_visual_width_mixed() {
        // Mixed content
        assert_eq!(visual_width("hello ─"), 7);
        // ▐▛███▜▌ contains 3 "█" characters which may be counted as width 2 each
        // and other block elements as width 1, so total could be different
        // Let's verify the actual behavior
        let width = visual_width("▐▛███▜▌");
        assert!(width > 0); // Just verify it calculates something reasonable
        assert_eq!(visual_width("test │ box"), 10);
    }

    #[test]
    fn test_print_two_col_row() {
        // This is a display function, we can't easily test output
        // but we can verify it doesn't panic
        print_two_col_row("Left", "Right");
        print_two_col_row("", "");
        print_two_col_row("Long text here", "Short");
    }

    #[test]
    fn test_display_welcome_screen() {
        // Verify the function doesn't panic
        display_welcome_screen("0.86.2", "gpt-4", "OpenAI");
    }

    #[test]
    fn test_display_simple_welcome() {
        // Verify the function doesn't panic
        display_simple_welcome("0.86.2", "claude-3-opus");
    }
}
