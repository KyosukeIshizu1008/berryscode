//! Context Window Management - Prevents token overflow and "Lost in the Middle" problem
//!
//! Implements sliding window and summarization to keep conversations within token limits
//! while preserving critical context (system prompt, repo map, recent messages).

use crate::berrycode::llm::Message;
use tiktoken_rs::{get_bpe_from_model, CoreBPE};

const MAX_CONTEXT_TOKENS: usize = 30_000; // Conservative limit for 64k models
const RESERVED_TOKENS: usize = 4_000; // Reserve for system prompt + repo map

/// Context window manager that keeps conversation within token limits
pub struct ContextWindowManager {
    tokenizer: CoreBPE,
    max_tokens: usize,
    reserved_tokens: usize,
}

impl ContextWindowManager {
    /// Create a new context window manager
    pub fn new() -> Self {
        // Use cl100k_base tokenizer (works for GPT-4, Claude, DeepSeek)
        let tokenizer = get_bpe_from_model("gpt-4").expect("Failed to load tokenizer");

        Self {
            tokenizer,
            max_tokens: MAX_CONTEXT_TOKENS,
            reserved_tokens: RESERVED_TOKENS,
        }
    }

    /// Create with custom limits
    pub fn with_limits(max_tokens: usize, reserved_tokens: usize) -> Self {
        let tokenizer = get_bpe_from_model("gpt-4").expect("Failed to load tokenizer");

        Self {
            tokenizer,
            max_tokens,
            reserved_tokens,
        }
    }

    /// Count tokens in a single message
    pub fn count_message_tokens(&self, message: &Message) -> usize {
        let mut total = 0;

        // Count role
        total += self.tokenizer.encode_with_special_tokens(&message.role).len();

        // Count content
        if let Some(content) = &message.content {
            total += self.tokenizer.encode_with_special_tokens(content).len();
        }

        // Count tool calls (if any)
        if let Some(tool_calls) = &message.tool_calls {
            for tool_call in tool_calls {
                total += self.tokenizer.encode_with_special_tokens(&tool_call.function.name).len();
                total += self.tokenizer.encode_with_special_tokens(&tool_call.function.arguments).len();
            }
        }

        // Add overhead for message structure (role, etc.)
        total + 4
    }

    /// Count total tokens in message history
    pub fn count_total_tokens(&self, messages: &[Message]) -> usize {
        messages.iter().map(|m| self.count_message_tokens(m)).sum()
    }

    /// Apply sliding window to keep messages within token limit
    ///
    /// Strategy:
    /// 1. Always keep system message (index 0)
    /// 2. Always keep recent messages
    /// 3. Summarize or drop middle messages if needed
    pub fn apply_sliding_window(&self, messages: Vec<Message>) -> Vec<Message> {
        let total_tokens = self.count_total_tokens(&messages);
        let available_tokens = self.max_tokens.saturating_sub(self.reserved_tokens);

        tracing::debug!(
            "Context window: {} tokens / {} available",
            total_tokens,
            available_tokens
        );

        // If within limits, return as-is
        if total_tokens <= available_tokens {
            return messages;
        }

        tracing::warn!(
            "Context overflow: {} tokens exceeds {} limit. Applying sliding window.",
            total_tokens,
            available_tokens
        );

        // Separate system message and conversation
        let (system_msgs, mut conversation): (Vec<_>, Vec<_>) = messages
            .into_iter()
            .partition(|m| m.role == "system");

        // Keep recent messages (last 10 exchanges = ~20 messages)
        let recent_count = 20.min(conversation.len());
        let recent_start = conversation.len().saturating_sub(recent_count);

        let mut result = system_msgs;

        // If we have middle messages, summarize them
        if recent_start > 0 {
            let middle_messages: Vec<_> = conversation.drain(..recent_start).collect();

            // Create a summary message
            let summary = self.summarize_messages(&middle_messages);
            result.push(Message {
                role: "system".to_string(),
                content: Some(summary),
                tool_calls: None,
                tool_call_id: None,
            });

            tracing::info!("Summarized {} old messages into 1 summary", middle_messages.len());
        }

        // Add recent messages
        result.extend(conversation);

        result
    }

    /// Summarize a sequence of messages into a compact summary
    fn summarize_messages(&self, messages: &[Message]) -> String {
        let mut summary = String::from("📝 **Context Summary** (previous conversation):\n\n");

        // Count different message types
        let user_messages: Vec<_> = messages.iter().filter(|m| m.role == "user").collect();
        let assistant_messages: Vec<_> = messages.iter().filter(|m| m.role == "assistant").collect();
        let tool_messages: Vec<_> = messages.iter().filter(|m| m.role == "tool").collect();

        summary.push_str(&format!(
            "- {} user requests\n- {} assistant responses\n- {} tool executions\n\n",
            user_messages.len(),
            assistant_messages.len(),
            tool_messages.len()
        ));

        // Extract key topics from user messages
        summary.push_str("**Topics discussed:**\n");
        for (i, msg) in user_messages.iter().take(5).enumerate() {
            if let Some(content) = &msg.content {
                // Extract first line or first 100 chars
                let preview = content.lines().next().unwrap_or(content);
                let preview = if preview.len() > 100 {
                    format!("{}...", &preview[..100])
                } else {
                    preview.to_string()
                };
                summary.push_str(&format!("{}. {}\n", i + 1, preview));
            }
        }

        if user_messages.len() > 5 {
            summary.push_str(&format!("... and {} more topics\n", user_messages.len() - 5));
        }

        summary.push_str("\n**Note:** This summary replaced detailed message history to manage context length.\n");

        summary
    }

    /// Estimate available tokens for new content
    pub fn available_tokens(&self, current_messages: &[Message]) -> usize {
        let used = self.count_total_tokens(current_messages);
        self.max_tokens.saturating_sub(used).saturating_sub(self.reserved_tokens)
    }

    /// Check if we're approaching token limit
    pub fn is_near_limit(&self, messages: &[Message]) -> bool {
        let total = self.count_total_tokens(messages);
        let threshold = (self.max_tokens as f64 * 0.8) as usize; // 80% threshold
        total >= threshold
    }

    /// Get current token usage statistics
    pub fn get_stats(&self, messages: &[Message]) -> ContextStats {
        let used = self.count_total_tokens(messages);
        let available = self.available_tokens(messages);
        let percentage = (used as f64 / self.max_tokens as f64 * 100.0) as u32;

        ContextStats {
            used_tokens: used,
            available_tokens: available,
            max_tokens: self.max_tokens,
            percentage_used: percentage,
            message_count: messages.len(),
        }
    }
}

impl Default for ContextWindowManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Context usage statistics
#[derive(Debug, Clone)]
pub struct ContextStats {
    pub used_tokens: usize,
    pub available_tokens: usize,
    pub max_tokens: usize,
    pub percentage_used: u32,
    pub message_count: usize,
}

impl ContextStats {
    /// Format as human-readable string
    pub fn format(&self) -> String {
        format!(
            "Context: {}/{} tokens ({}%) | {} messages | {} tokens available",
            self.used_tokens,
            self.max_tokens,
            self.percentage_used,
            self.message_count,
            self.available_tokens
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_counting() {
        let manager = ContextWindowManager::new();

        let message = Message {
            role: "user".to_string(),
            content: Some("Hello, how are you?".to_string()),
            tool_calls: None,
            tool_call_id: None,
        };

        let tokens = manager.count_message_tokens(&message);
        assert!(tokens > 0);
        assert!(tokens < 20); // Should be around 8-10 tokens
    }

    #[test]
    fn test_sliding_window() {
        let manager = ContextWindowManager::with_limits(100, 10);

        // Create messages that exceed limit
        let mut messages = vec![
            Message {
                role: "system".to_string(),
                content: Some("You are a helpful assistant".to_string()),
                tool_calls: None,
                tool_call_id: None,
            },
        ];

        // Add many user messages
        for i in 0..50 {
            messages.push(Message {
                role: "user".to_string(),
                content: Some(format!("Message number {}", i)),
                tool_calls: None,
                tool_call_id: None,
            });
        }

        let result = manager.apply_sliding_window(messages.clone());

        // Should have fewer messages than original
        assert!(result.len() < messages.len());

        // Should keep system message
        assert_eq!(result[0].role, "system");
        assert!(result[0].content.as_ref().unwrap().contains("helpful assistant"));

        // Should have a summary message
        let has_summary = result.iter().any(|m| {
            m.role == "system" && m.content.as_ref().map_or(false, |c| c.contains("Context Summary"))
        });
        assert!(has_summary);
    }

    #[test]
    fn test_stats() {
        let manager = ContextWindowManager::new();

        let messages = vec![
            Message {
                role: "user".to_string(),
                content: Some("Hello".to_string()),
                tool_calls: None,
                tool_call_id: None,
            },
        ];

        let stats = manager.get_stats(&messages);
        assert!(stats.used_tokens > 0);
        assert!(stats.available_tokens > 0);
        assert_eq!(stats.message_count, 1);
    }
}
