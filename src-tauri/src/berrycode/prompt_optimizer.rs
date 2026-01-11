//! Dynamic Prompt Optimizer for conversation turns
//!
//! This module implements three critical optimizations:
//! 1. Retry Booster: Prevents AI from repeating the same mistake
//! 2. Tag Enforcer: Reminds AI to use proper tool format every N turns
//! 3. Context Cleaner: Truncates long outputs to reduce noise

use crate::berrycode::llm::Message;
use std::collections::VecDeque;

const MAX_OUTPUT_LINES: usize = 100;
const MAX_CONTEXT_LENGTH: usize = 2000;
const REMINDER_INTERVAL: usize = 3;

/// Prompt optimizer that enhances messages based on conversation state
pub struct PromptOptimizer {
    turn_count: usize,
    recent_errors: VecDeque<String>,
    max_recent_errors: usize,
}

impl PromptOptimizer {
    pub fn new() -> Self {
        Self {
            turn_count: 0,
            recent_errors: VecDeque::new(),
            max_recent_errors: 3,
        }
    }

    /// Increment turn counter and return current count
    pub fn next_turn(&mut self) -> usize {
        self.turn_count += 1;
        self.turn_count
    }

    /// Get current turn count
    pub fn current_turn(&self) -> usize {
        self.turn_count
    }

    /// Reset optimizer state (for new conversations)
    pub fn reset(&mut self) {
        self.turn_count = 0;
        self.recent_errors.clear();
    }

    /// Optimize user message based on context
    pub fn optimize_user_message(&mut self, content: String, is_error: bool) -> String {
        let mut optimized = content;

        // 1. Retry Booster: If this is an error message
        if is_error {
            optimized = self.apply_retry_booster(optimized);
        }

        // 2. Context Cleaner: Truncate long outputs
        optimized = self.apply_context_cleaner(optimized);

        // 3. Tag Enforcer: Add reminders periodically
        if self.should_add_reminder() {
            optimized = self.apply_tag_enforcer(optimized);
        }

        optimized
    }

    /// Retry Booster: Prevent AI from repeating the same mistake
    fn apply_retry_booster(&mut self, content: String) -> String {
        // Track this error
        self.recent_errors.push_back(content.clone());
        if self.recent_errors.len() > self.max_recent_errors {
            self.recent_errors.pop_front();
        }

        // Detect if error contains common patterns
        let is_command_error = content.contains("Error:")
            || content.contains("error:")
            || content.contains("failed")
            || content.contains("FAILED");

        let is_file_error = content.contains("No such file")
            || content.contains("not found")
            || content.contains("cannot find");

        let mut boosted = content;

        if is_command_error || is_file_error {
            boosted.push_str("\n\n🚨 CRITICAL INSTRUCTION FOR ERROR RECOVERY:\n");
            boosted.push_str("1. Do NOT try the exact same solution again.\n");
            boosted.push_str("2. Analyze WHY it failed (syntax error? wrong file path? missing dependency?).\n");
            boosted.push_str("3. If the error persists after 2 attempts, use different tools:\n");
            boosted.push_str("   - Use 'grep' to find correct function/variable names\n");
            boosted.push_str("   - Use 'read_file' to check current file content before editing\n");
            boosted.push_str("   - Use 'list_files' to verify file paths exist\n");
            boosted.push_str("4. Consider alternative approaches - the current approach may be fundamentally wrong.\n");
        }

        if is_file_error {
            boosted.push_str("\n💡 File Error Hint: Use 'glob' or 'list_files' to find the correct path.\n");
        }

        boosted
    }

    /// Tag Enforcer: Remind AI to use proper tool format
    fn apply_tag_enforcer(&self, content: String) -> String {
        format!(
            "{}\n\n\
            ⚠️  REMINDER (Turn {}):\n\
            - You MUST use the available tools (read_file, edit_file, bash, etc.)\n\
            - Do NOT output code blocks directly - use edit_file or write_file tools\n\
            - When searching, use 'grep' BEFORE reading files\n\
            - Combine bash commands with && to minimize tool calls\n\
            - You have {} tool calls remaining - use them wisely!",
            content,
            self.turn_count,
            30 - self.turn_count.min(30)
        )
    }

    /// Context Cleaner: Truncate long outputs to reduce noise
    fn apply_context_cleaner(&self, content: String) -> String {
        let lines: Vec<&str> = content.lines().collect();

        // If content is short enough, return as-is
        if lines.len() <= MAX_OUTPUT_LINES && content.len() <= MAX_CONTEXT_LENGTH {
            return content;
        }

        // Truncate to last N lines
        let truncated_lines = if lines.len() > MAX_OUTPUT_LINES {
            let start_idx = lines.len().saturating_sub(MAX_OUTPUT_LINES);
            &lines[start_idx..]
        } else {
            &lines[..]
        };

        let mut result = format!(
            "⚠️  Output truncated for clarity (showing last {} of {} lines):\n\n",
            truncated_lines.len(),
            lines.len()
        );

        result.push_str(&truncated_lines.join("\n"));

        // Also check character length
        if result.len() > MAX_CONTEXT_LENGTH {
            // Find safe truncation point at UTF-8 character boundary
            let truncate_at = result.char_indices()
                .take_while(|(idx, _)| *idx < MAX_CONTEXT_LENGTH)
                .last()
                .map(|(idx, ch)| idx + ch.len_utf8())
                .unwrap_or(0);

            result.truncate(truncate_at);
            result.push_str("\n\n... (output truncated to prevent context overflow)");
        }

        result
    }

    /// Check if we should add a reminder
    fn should_add_reminder(&self) -> bool {
        self.turn_count > 0 && self.turn_count % REMINDER_INTERVAL == 0
    }

    /// Analyze if recent errors show a pattern (infinite loop detection)
    pub fn detect_loop(&self) -> Option<String> {
        if self.recent_errors.len() < 2 {
            return None;
        }

        // Check if the last 2 errors are very similar
        let errors: Vec<&String> = self.recent_errors.iter().collect();
        let recent_count = errors.len();

        if recent_count >= 2 {
            let last = errors[recent_count - 1];
            let prev = errors[recent_count - 2];

            // Simple similarity check: same error message length and contains same keywords
            if last.len() == prev.len() || Self::similarity_ratio(last, prev) > 0.8 {
                return Some(format!(
                    "⚠️  LOOP DETECTED: The same error occurred {} times in a row.\n\
                    You are likely stuck in an infinite loop. Consider:\n\
                    1. Completely different approach\n\
                    2. Ask user for clarification with 'ask_user'\n\
                    3. Skip this task and move to the next one",
                    recent_count
                ));
            }
        }

        None
    }

    /// Calculate simple similarity ratio between two strings
    fn similarity_ratio(a: &str, b: &str) -> f64 {
        let a_words: Vec<&str> = a.split_whitespace().collect();
        let b_words: Vec<&str> = b.split_whitespace().collect();

        if a_words.is_empty() || b_words.is_empty() {
            return 0.0;
        }

        let common_words = a_words.iter()
            .filter(|word| b_words.contains(word))
            .count();

        common_words as f64 / a_words.len().max(b_words.len()) as f64
    }

    /// Clean message history by removing redundant failed attempts
    pub fn clean_history(&self, messages: Vec<Message>) -> Vec<Message> {
        let mut cleaned = Vec::new();
        let mut skip_next_error_pair = false;

        for (i, msg) in messages.iter().enumerate() {
            // Skip error-response pairs that were later fixed
            if skip_next_error_pair {
                skip_next_error_pair = false;
                continue;
            }

            // If this is an error message and next message is a fix that worked
            if msg.role == "user" {
                if let Some(content) = &msg.content {
                    if content.contains("Error:") || content.contains("failed") {
                        // Check if this error was eventually resolved
                        if i + 2 < messages.len() {
                            // If there's a success after this, we can skip the error
                            if let Some(later_content) = &messages[i + 2].content {
                                if later_content.contains("success") || later_content.contains("✓") {
                                    skip_next_error_pair = true;
                                    continue;
                                }
                            }
                        }
                    }
                }
            }

            cleaned.push(msg.clone());
        }

        cleaned
    }
}

impl Default for PromptOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_booster() {
        let mut optimizer = PromptOptimizer::new();
        let error = "Error: command failed".to_string();
        let boosted = optimizer.apply_retry_booster(error);

        assert!(boosted.contains("CRITICAL INSTRUCTION"));
        assert!(boosted.contains("Do NOT try the exact same solution"));
    }

    #[test]
    fn test_context_cleaner() {
        let optimizer = PromptOptimizer::new();

        // Create long output
        let mut long_output = String::new();
        for i in 0..200 {
            long_output.push_str(&format!("Line {}\n", i));
        }

        let cleaned = optimizer.apply_context_cleaner(long_output.clone());
        assert!(cleaned.contains("Output truncated"));
        assert!(cleaned.lines().count() < long_output.lines().count());
    }

    #[test]
    fn test_tag_enforcer() {
        let mut optimizer = PromptOptimizer::new();
        optimizer.turn_count = 3;

        let content = "Some content".to_string();
        let enforced = optimizer.apply_tag_enforcer(content);

        assert!(enforced.contains("REMINDER"));
        assert!(enforced.contains("Turn 3"));
    }

    #[test]
    fn test_should_add_reminder() {
        let mut optimizer = PromptOptimizer::new();

        assert!(!optimizer.should_add_reminder()); // Turn 0

        optimizer.next_turn(); // Turn 1
        assert!(!optimizer.should_add_reminder());

        optimizer.next_turn(); // Turn 2
        assert!(!optimizer.should_add_reminder());

        optimizer.next_turn(); // Turn 3
        assert!(optimizer.should_add_reminder());
    }

    #[test]
    fn test_loop_detection() {
        let mut optimizer = PromptOptimizer::new();

        optimizer.recent_errors.push_back("Error: file not found".to_string());
        optimizer.recent_errors.push_back("Error: file not found".to_string());

        let loop_msg = optimizer.detect_loop();
        assert!(loop_msg.is_some());
        assert!(loop_msg.unwrap().contains("LOOP DETECTED"));
    }

    #[test]
    fn test_similarity_ratio() {
        let a = "error file not found foo bar";
        let b = "error file not found baz qux";

        let ratio = PromptOptimizer::similarity_ratio(a, b);
        assert!(ratio > 0.5); // Should have decent similarity
    }
}
