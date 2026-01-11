//! Automatic conversation summarization

/// Conversation summarizer
#[derive(Clone, Copy)]
pub struct Summarizer {
    enabled: bool,
}

impl Summarizer {
    pub fn new() -> Self {
        Self { enabled: true }
    }

    /// Summarize a tool result to reduce tokens
    pub fn summarize_tool_result(&self, tool_name: &str, result: &str) -> String {
        if !self.enabled {
            return result.to_string();
        }

        // If result is short, don't summarize
        if result.len() < 500 {
            return result.to_string();
        }

        match tool_name {
            "read_file" => self.summarize_file_content(result),
            "list_files" => self.summarize_file_list(result),
            "grep" | "search_files" => self.summarize_search_results(result),
            "bash" => self.summarize_command_output(result),
            _ => self.truncate_if_needed(result, 2000),
        }
    }

    /// Summarize file content
    fn summarize_file_content(&self, content: &str) -> String {
        let lines: Vec<&str> = content.lines().collect();

        if lines.len() <= 50 {
            return content.to_string();
        }

        // Extract key information
        let mut summary = String::new();

        // First 10 lines
        summary.push_str("=== File Start ===\n");
        for line in lines.iter().take(10) {
            summary.push_str(line);
            summary.push('\n');
        }

        summary.push_str(&format!("\n... ({} lines omitted) ...\n\n", lines.len() - 20));

        // Last 10 lines
        summary.push_str("=== File End ===\n");
        for line in lines.iter().skip(lines.len().saturating_sub(10)) {
            summary.push_str(line);
            summary.push('\n');
        }

        summary
    }

    /// Summarize file list
    fn summarize_file_list(&self, list: &str) -> String {
        let lines: Vec<&str> = list.lines().collect();

        if lines.len() <= 20 {
            return list.to_string();
        }

        let mut summary = String::new();
        summary.push_str(&lines[0]); // Header
        summary.push('\n');

        for line in lines.iter().take(15).skip(1) {
            summary.push_str(line);
            summary.push('\n');
        }

        summary.push_str(&format!("\n... ({} more files) ...\n", lines.len() - 15));

        summary
    }

    /// Summarize search results
    fn summarize_search_results(&self, results: &str) -> String {
        let lines: Vec<&str> = results.lines().collect();

        if lines.len() <= 30 {
            return results.to_string();
        }

        let mut summary = String::new();
        summary.push_str(&lines[0]); // Header
        summary.push('\n');

        // Show first 20 results
        for line in lines.iter().take(21).skip(1) {
            summary.push_str(line);
            summary.push('\n');
        }

        summary.push_str(&format!("\n... ({} more results) ...\n", lines.len() - 21));

        summary
    }

    /// Summarize command output
    fn summarize_command_output(&self, output: &str) -> String {
        self.truncate_if_needed(output, 1500)
    }

    /// Truncate text if it exceeds limit
    fn truncate_if_needed(&self, text: &str, max_chars: usize) -> String {
        let char_count = text.chars().count();
        if char_count <= max_chars {
            return text.to_string();
        }

        let mut result = String::new();
        let chars_per_section = max_chars / 2;

        // First part - use char indices, not byte indices
        let first_part: String = text.chars().take(chars_per_section).collect();
        result.push_str(&first_part);
        result.push_str(&format!("\n\n... ({} chars omitted) ...\n\n", char_count - max_chars));

        // Last part - use char indices, not byte indices
        let last_part: String = text.chars().skip(char_count - chars_per_section).collect();
        result.push_str(&last_part);

        result
    }

    /// Summarize multiple messages into one
    pub fn compress_conversation(&self, messages: &[(String, String)]) -> String {
        if messages.is_empty() {
            return String::new();
        }

        let mut summary = String::from("=== Conversation Summary ===\n\n");

        for (i, (role, content)) in messages.iter().enumerate() {
            summary.push_str(&format!("[{}] ", role));

            // Truncate long messages
            if content.len() > 200 {
                summary.push_str(&content[..200]);
                summary.push_str("...");
            } else {
                summary.push_str(content);
            }
            summary.push('\n');

            if i >= 10 {
                summary.push_str(&format!("\n... ({} more messages) ...\n", messages.len() - 10));
                break;
            }
        }

        summary
    }
}

impl Default for Summarizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_summarization() {
        let summarizer = Summarizer::new();
        let content = (0..100).map(|i| format!("Line {}", i)).collect::<Vec<_>>().join("\n");

        let summary = summarizer.summarize_file_content(&content);
        assert!(summary.contains("File Start"));
        assert!(summary.contains("File End"));
        assert!(summary.contains("lines omitted"));
    }

    #[test]
    fn test_short_content_not_summarized() {
        let summarizer = Summarizer::new();
        let short = "Short content";

        let result = summarizer.summarize_tool_result("read_file", short);
        assert_eq!(result, short);
    }

    #[test]
    fn test_truncate() {
        let summarizer = Summarizer::new();
        let long_text = "a".repeat(5000);

        let truncated = summarizer.truncate_if_needed(&long_text, 1000);
        assert!(truncated.len() < 1500);
        assert!(truncated.contains("chars omitted"));
    }
}
