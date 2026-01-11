//! Context management and token tracking

/// Context window manager
pub struct ContextManager {
    pub max_tokens: usize,
    pub current_tokens: usize,
    #[allow(dead_code)]
    model_name: String,
}

impl ContextManager {
    pub fn new(model_name: String) -> Self {
        let max_tokens = Self::get_context_limit(&model_name);

        // Debug log to help diagnose context issues
        tracing::info!(
            "🧠 Context Manager initialized: model='{}', max_tokens={}",
            model_name, max_tokens
        );

        Self {
            max_tokens,
            current_tokens: 0,
            model_name,
        }
    }

    /// Get context window size for a model
    fn get_context_limit(model_name: &str) -> usize {
        if model_name.contains("gpt-4") {
            128000 // GPT-4 Turbo
        } else if model_name.contains("gpt-3.5") {
            16385
        } else if model_name.contains("claude-3") || model_name.contains("sonnet") {
            200000 // Claude 3.5 Sonnet
        } else if model_name.contains("deepseek") {
            64000 // DeepSeek
        } else {
            8192 // Default conservative estimate
        }
    }

    /// Estimate tokens in text (rough approximation: 1 token ~= 4 chars)
    pub fn estimate_tokens(&self, text: &str) -> usize {
        (text.len() / 4).max(1)
    }

    /// Add tokens to current count
    pub fn add_tokens(&mut self, count: usize) {
        self.current_tokens += count;

        // Warn if a single addition is suspiciously large (>10k tokens = ~40k chars)
        if count > 10000 {
            tracing::warn!(
                "⚠️  Large token addition: +{} tokens (current: {}/{}, {:.1}%)",
                count, self.current_tokens, self.max_tokens, self.usage_percentage()
            );
        }

        // Debug trace for normal additions
        tracing::debug!(
            "Tokens: +{} → {}/{} ({:.1}%)",
            count, self.current_tokens, self.max_tokens, self.usage_percentage()
        );
    }

    /// Check if we're approaching context limit
    pub fn is_near_limit(&self) -> bool {
        self.current_tokens > (self.max_tokens * 70 / 100) // 70% threshold
    }

    /// Check if we should summarize (more aggressive than is_near_limit)
    pub fn should_summarize(&self) -> bool {
        // Trigger summarization at 80% (was 90% in old implementation)
        self.usage_percentage() > 80.0
    }

    /// Check if context is critically high
    pub fn is_critical(&self) -> bool {
        self.usage_percentage() > 120.0
    }

    /// Get warning message based on current usage
    pub fn get_warning_message(&self) -> Option<String> {
        let usage = self.usage_percentage();

        if usage > 150.0 {
            Some(format!(
                "🚨 CRITICAL: Context severely exceeded at {:.1}%! Conversation must be summarized immediately.",
                usage
            ))
        } else if usage > 120.0 {
            Some(format!(
                "🚨 CRITICAL: Context at {:.1}% - summarization REQUIRED now!",
                usage
            ))
        } else if usage > 80.0 {
            Some(format!(
                "⚠️  Context at {:.1}% - consider summarizing conversation soon",
                usage
            ))
        } else if usage > 60.0 {
            Some(format!(
                "📊 Context at {:.1}% - stay concise in responses",
                usage
            ))
        } else {
            None
        }
    }

    /// Get detailed context status for display
    pub fn get_status_display(&self) -> String {
        let usage = self.usage_percentage();
        let icon = if usage > 120.0 {
            "🚨"
        } else if usage > 80.0 {
            "⚠️"
        } else if usage > 60.0 {
            "📊"
        } else {
            "✓"
        };

        format!(
            "{} Context: {:.1}% ({} / {} tokens)",
            icon,
            usage,
            self.current_tokens,
            self.max_tokens
        )
    }

    /// Get remaining tokens
    pub fn remaining_tokens(&self) -> usize {
        self.max_tokens.saturating_sub(self.current_tokens)
    }

    /// Get usage percentage
    pub fn usage_percentage(&self) -> f32 {
        let percentage = (self.current_tokens as f32 / self.max_tokens as f32) * 100.0;

        // Warn if percentage is absurdly high (indicates a bug)
        if percentage > 1000.0 {
            tracing::error!(
                "🚨 BUG DETECTED: Context usage at {:.1}%! current_tokens={}, max_tokens={}. \
                This indicates a token counting bug. Please report this issue.",
                percentage, self.current_tokens, self.max_tokens
            );
        }

        percentage
    }

    /// Reset token count
    pub fn reset(&mut self) {
        self.current_tokens = 0;
    }

    /// Check if text fits in remaining context
    pub fn can_fit(&self, text: &str) -> bool {
        let tokens = self.estimate_tokens(text);
        self.current_tokens + tokens < self.max_tokens
    }
}

/// Message priority for context management
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MessagePriority {
    Critical = 3,   // System prompts, user queries
    High = 2,       // Recent tool results
    Medium = 1,     // Older tool results
    Low = 0,        // Very old context
}

/// Message with metadata for context management
#[derive(Debug, Clone)]
pub struct ManagedMessage {
    pub role: String,
    pub content: String,
    pub priority: MessagePriority,
    pub token_count: usize,
    pub timestamp: std::time::SystemTime,
}

impl ManagedMessage {
    pub fn new(role: String, content: String, priority: MessagePriority) -> Self {
        let token_count = content.len() / 4; // Rough estimate
        Self {
            role,
            content,
            priority,
            token_count,
            timestamp: std::time::SystemTime::now(),
        }
    }

    /// Check if message is recent (within last 5 minutes)
    pub fn is_recent(&self) -> bool {
        if let Ok(elapsed) = self.timestamp.elapsed() {
            elapsed.as_secs() < 300 // 5 minutes
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_limits() {
        let gpt4 = ContextManager::new("gpt-4-turbo".to_string());
        assert_eq!(gpt4.max_tokens, 128000);

        let claude = ContextManager::new("claude-3-sonnet".to_string());
        assert_eq!(claude.max_tokens, 200000);

        let deepseek = ContextManager::new("deepseek-chat".to_string());
        assert_eq!(deepseek.max_tokens, 64000);
    }

    #[test]
    fn test_token_estimation() {
        let ctx = ContextManager::new("gpt-4".to_string());
        let text = "a".repeat(1000);
        let tokens = ctx.estimate_tokens(&text);
        assert_eq!(tokens, 250); // 1000 / 4
    }

    #[test]
    fn test_near_limit() {
        let mut ctx = ContextManager::new("gpt-4".to_string());
        assert!(!ctx.is_near_limit());

        ctx.current_tokens = 100000; // 78% of 128000
        assert!(ctx.is_near_limit());
    }

    #[test]
    fn test_usage_percentage_normal() {
        let mut ctx = ContextManager::new("gpt-4".to_string());
        ctx.current_tokens = 64000; // 50% of 128000

        let percentage = ctx.usage_percentage();
        assert_eq!(percentage, 50.0);
    }

    #[test]
    fn test_usage_percentage_abnormal() {
        // Test abnormal percentage detection (>1000%)
        let mut ctx = ContextManager::new("gpt-4".to_string());
        ctx.current_tokens = 1_300_000; // >1000% of 128000

        let percentage = ctx.usage_percentage();
        assert!(percentage > 1000.0);
        // Should trigger error log (check tracing output)
    }

    #[test]
    fn test_add_tokens_with_warning() {
        let mut ctx = ContextManager::new("gpt-4".to_string());

        // Normal addition
        ctx.add_tokens(5000);
        assert_eq!(ctx.current_tokens, 5000);

        // Large addition (>10k tokens) - should trigger warning
        ctx.add_tokens(15000);
        assert_eq!(ctx.current_tokens, 20000);
    }

    #[test]
    fn test_summarizer_integration() {
        use crate::berrycode::summarizer::Summarizer;

        let summarizer = Summarizer::new();

        // Create large multi-line content (100 lines)
        let lines: Vec<String> = (0..100).map(|i| format!("Line {} with some content", i)).collect();
        let large_result = lines.join("\n");

        let summarized = summarizer.summarize_tool_result("read_file", &large_result);

        // Summarized result should be much smaller (only first 10 + last 10 lines)
        assert!(summarized.len() < large_result.len());
        assert!(summarized.contains("File Start"));
        assert!(summarized.contains("File End"));
    }
}
