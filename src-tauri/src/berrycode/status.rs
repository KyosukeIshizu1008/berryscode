//! Claude Code-style status display with progress tracking
//!
//! Provides beautiful multi-line status display showing:
//! - Current action with spinner
//! - Elapsed time (auto-updating)
//! - Token count (auto-formatted: 1.7k, 2.1M, etc.)
//! - Next action preview

use indicatif::{ProgressBar, ProgressStyle};
use console::style;
use std::time::Duration;

/// Status manager for Claude Code-style progress display
///
/// # Example
///
/// ```
/// use berrycode::status::StatusManager;
///
/// let status = StatusManager::new();
///
/// // Update status during processing
/// status.update(
///     "Compacting conversation...",
///     Some("テストと確認"),
///     1750  // 1.75k tokens
/// );
///
/// // Complete
/// status.finish("Done!");
/// ```
pub struct StatusManager {
    pb: ProgressBar,
}

impl StatusManager {
    /// Create a new status manager with Claude Code-style display
    ///
    /// The display format:
    /// ```text
    /// • Compacting conversation... (esc to interrupt · ctrl+t to show todos · 1m 13s · ↓ 1.7k tokens)
    ///   └ Next: テストと確認
    /// ```
    pub fn new() -> Self {
        let pb = ProgressBar::new_spinner();

        // --- 🎨 魔法のテンプレート ---
        // {spinner:.blue}  : 青いスピナー
        // {msg}            : 現在のアクション (例: Compacting...)
        // {elapsed}        : 経過時間 (自動更新)
        // {human_len}      : 全長(トークン数)を "1.7k" のように自動フォーマット
        // {prefix}         : 次のアクション (2行目)
        let template = format!(
            "{{spinner:.blue}} {{msg}} {}\n{{prefix}}",
            style("(esc to interrupt · ctrl+t to show todos · {elapsed} · ↓ {human_len} tokens)").dim()
        );

        let style_obj = ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏") // くるくる回る文字
            .template(&template)
            .unwrap();

        pb.set_style(style_obj);
        pb.enable_steady_tick(Duration::from_millis(100)); // 100msごとに描画更新

        Self { pb }
    }

    /// 状態を更新するメソッド
    ///
    /// # Arguments
    ///
    /// - `action`: 今やっていること ("Compacting conversation...")
    /// - `next`: 次やること ("テストと確認")
    /// - `tokens`: 現在のトークン数 (1700 -> "1.7k" と表示される)
    ///
    /// # Example
    ///
    /// ```
    /// # use berrycode::status::StatusManager;
    /// let status = StatusManager::new();
    /// status.update(
    ///     "Generating code...",
    ///     Some("ファイル書き込み"),
    ///     2100  // 2.1k tokens
    /// );
    /// ```
    pub fn update(&self, action: &str, next: Option<&str>, tokens: u64) {
        // 1. 今やっていること (青紫っぽく表示)
        // Claude Codeの色味: style(action).blue().bold() などお好みで
        self.pb.set_message(format!("{}", style(action).bold().blue()));

        // 2. トークン数 (indicatifのlength機能を借用！)
        self.pb.set_length(tokens);

        // 3. 次のアクション (2行目として表示)
        // インデント "  └ " をつける
        if let Some(next_action) = next {
            self.pb.set_prefix(format!(
                "  {} Next: {}",
                style("└").dim(),
                style(next_action).dim()
            ));
        } else {
            self.pb.set_prefix("".to_string());
        }
    }

    /// 完了時の処理
    ///
    /// # Example
    ///
    /// ```
    /// # use berrycode::status::StatusManager;
    /// let status = StatusManager::new();
    /// status.finish("Done!");
    /// ```
    pub fn finish(&self, message: &str) {
        self.pb.finish_with_message(message.to_string());
    }

    /// Clear the progress bar (remove from terminal)
    pub fn clear(&self) {
        self.pb.finish_and_clear();
    }
}

impl Default for StatusManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_manager_creation() {
        let status = StatusManager::new();
        // Just verify it doesn't panic
        status.update("Testing...", Some("Next step"), 1000);
        status.finish("Done!");
    }

    #[test]
    fn test_status_manager_no_next() {
        let status = StatusManager::new();
        status.update("Processing...", None, 500);
        status.clear();
    }

    #[test]
    fn test_status_manager_large_tokens() {
        let status = StatusManager::new();
        // Test with large token count (should display as "1.5M")
        status.update("Compacting...", Some("Summarizing"), 1_500_000);
        status.finish("Complete!");
    }
}
