//! Tool usage monitoring and control

use std::collections::HashMap;
use std::time::{SystemTime, Duration};

/// Tool usage statistics
#[derive(Debug, Clone)]
pub struct ToolStats {
    pub name: String,
    pub call_count: usize,
    pub total_tokens: usize,
    pub last_called: SystemTime,
    pub average_tokens: usize,
}

/// Tool usage monitor
pub struct ToolMonitor {
    stats: HashMap<String, ToolStats>,
    total_calls: usize,
    duplicate_threshold: usize,
}

impl ToolMonitor {
    pub fn new() -> Self {
        Self {
            stats: HashMap::new(),
            total_calls: 0,
            duplicate_threshold: 3,
        }
    }

    /// Record a tool call
    pub fn record_call(&mut self, tool_name: &str, result_tokens: usize) {
        self.total_calls += 1;

        let stats = self.stats.entry(tool_name.to_string()).or_insert(ToolStats {
            name: tool_name.to_string(),
            call_count: 0,
            total_tokens: 0,
            last_called: SystemTime::now(),
            average_tokens: 0,
        });

        stats.call_count += 1;
        stats.total_tokens += result_tokens;
        stats.last_called = SystemTime::now();
        stats.average_tokens = stats.total_tokens / stats.call_count;
    }

    /// Check if a tool is being overused
    pub fn is_overused(&self, tool_name: &str) -> bool {
        if let Some(stats) = self.stats.get(tool_name) {
            // edit_file called more than 5 times? (BATCH EDIT MODE NEEDED!)
            if tool_name == "edit_file" && stats.call_count > 5 {
                return true;
            }

            // read_file called more than 15 times?
            if tool_name == "read_file" && stats.call_count > 15 {
                return true;
            }

            // grep called more than 10 times?
            if tool_name == "grep" && stats.call_count > 10 {
                return true;
            }

            // Any tool called more than 8 times?
            if stats.call_count > 8 {
                return true;
            }
        }
        false
    }

    /// Get suggestion for tool that's being overused
    pub fn get_overuse_suggestion(&self, tool_name: &str) -> Option<String> {
        if !self.is_overused(tool_name) {
            return None;
        }

        let suggestion = match tool_name {
            "edit_file" => {
                format!(
                    "⚠️ TOOL OVERUSE DETECTED: edit_file called {} times!\n\
                     \n\
                     🚨 You're making too many small edits. This will trigger errors.\n\
                     \n\
                     ✅ STOP using edit_file. Instead:\n\
                     1. Write a Python script to do ALL replacements at once\n\
                     2. Use regex/sed for bulk find-replace\n\
                     3. Complete the task in 1 command instead of 100 edits\n\
                     \n\
                     Example: python -c \"import re; content=open('file.rs').read(); ...\"\n\
                     \n\
                     This is 100x faster and won't trigger overuse warnings!",
                    self.stats.get(tool_name).map(|s| s.call_count).unwrap_or(0)
                )
            }
            "read_file" => {
                format!(
                    "⚠️ read_file called {} times - consider using grep or semantic_search instead",
                    self.stats.get(tool_name).map(|s| s.call_count).unwrap_or(0)
                )
            }
            "grep" => {
                format!(
                    "⚠️ grep called {} times - results might be noisy, try narrowing your search",
                    self.stats.get(tool_name).map(|s| s.call_count).unwrap_or(0)
                )
            }
            _ => {
                format!(
                    "⚠️ {} called {} times - this seems excessive",
                    tool_name,
                    self.stats.get(tool_name).map(|s| s.call_count).unwrap_or(0)
                )
            }
        };

        Some(suggestion)
    }

    /// Detect duplicate tool calls (same tool within 10 seconds)
    pub fn is_duplicate(&self, tool_name: &str) -> bool {
        if let Some(stats) = self.stats.get(tool_name) {
            if let Ok(elapsed) = stats.last_called.elapsed() {
                // Same tool called within 10 seconds
                return elapsed < Duration::from_secs(10) && stats.call_count >= self.duplicate_threshold;
            }
        }
        false
    }

    /// Get total tool calls
    pub fn total_calls(&self) -> usize {
        self.total_calls
    }

    /// Get most used tools
    pub fn most_used_tools(&self) -> Vec<(String, usize)> {
        let mut tools: Vec<_> = self.stats
            .iter()
            .map(|(name, stats)| (name.clone(), stats.call_count))
            .collect();

        tools.sort_by(|a, b| b.1.cmp(&a.1));
        tools
    }

    /// Get efficiency report
    pub fn get_report(&self) -> String {
        let mut report = String::from("=== Tool Usage Report ===\n\n");

        report.push_str(&format!("Total calls: {}\n\n", self.total_calls));

        let most_used = self.most_used_tools();
        report.push_str("Most used tools:\n");
        for (tool, count) in most_used.iter().take(5) {
            if let Some(stats) = self.stats.get(tool) {
                report.push_str(&format!(
                    "  {} - {} calls, avg {} tokens\n",
                    tool, count, stats.average_tokens
                ));
            }
        }

        // Detect inefficiencies
        report.push_str("\nPotential inefficiencies:\n");
        let mut found_issue = false;

        for (tool, stats) in &self.stats {
            if stats.call_count > 5 {
                report.push_str(&format!(
                    "  ⚠️  {} called {} times (possibly redundant)\n",
                    tool, stats.call_count
                ));
                found_issue = true;
            }
        }

        if !found_issue {
            report.push_str("  ✓ No major inefficiencies detected\n");
        }

        report
    }

    /// Check if we should warn early before tool usage becomes problematic
    pub fn should_warn_early(&self, tool_name: &str) -> Option<String> {
        let call_count = self.stats.get(tool_name).map(|s| s.call_count).unwrap_or(0);

        // Early warnings for specific tools
        match tool_name {
            "read_file" if call_count >= 8 => {
                Some("🚨 STOP: 8+ files read. Use grep to search, not multiple read_file calls.".to_string())
            }
            "read_file" if call_count >= 5 => {
                Some("⚠️  You've read 5+ files. Consider using grep to find specific content across files.".to_string())
            }
            "grep" if call_count >= 8 => {
                Some("🚨 STOP: Too many grep calls. Read the files you've found instead of searching more.".to_string())
            }
            "grep" if call_count >= 5 => {
                Some("💡 Multiple grep calls. Consider reading the files you've found with grep.".to_string())
            }
            "bash" if call_count >= 4 => {
                Some("🚨 STOP: Too many bash calls. Combine commands with && operator.".to_string())
            }
            "bash" if call_count >= 2 => {
                Some("💡 Multiple bash calls detected. Can you combine commands with && or ; ?".to_string())
            }
            "list_files" if call_count >= 3 => {
                Some("⚠️  You've listed files 3+ times. Use glob or grep instead.".to_string())
            }
            _ => {
                // General warnings based on total calls (max_iterations = 30)
                if self.total_calls >= 25 {
                    Some(format!("🔴 EMERGENCY: {} / 30 calls used. FINISH NOW!", self.total_calls))
                } else if self.total_calls >= 20 {
                    Some(format!("🚨 CRITICAL: {} / 30 calls used. Complete task immediately!", self.total_calls))
                } else if self.total_calls >= 15 {
                    Some(format!("⚠️  Tool limit warning: {} / 30 calls used. Be strategic!", self.total_calls))
                } else {
                    None
                }
            }
        }
    }

    /// Get current tool call count for a specific tool
    pub fn get_call_count(&self, tool_name: &str) -> usize {
        self.stats.get(tool_name).map(|s| s.call_count).unwrap_or(0)
    }

    /// Check if approaching tool limit
    pub fn is_approaching_limit(&self) -> bool {
        self.total_calls >= 15
    }

    /// Check if at critical tool limit
    pub fn is_at_critical_limit(&self) -> bool {
        self.total_calls >= 20
    }

    /// Get efficiency percentage (higher is better, max 100)
    pub fn get_efficiency_percentage(&self) -> f64 {
        if self.total_calls == 0 {
            return 100.0;
        }

        // Efficient: 1-8 calls = 100-80%
        // Moderate: 9-15 calls = 79-50%
        // Poor: 16+ calls = <50%
        let percentage = if self.total_calls <= 8 {
            100.0 - (self.total_calls as f64 * 2.5)
        } else if self.total_calls <= 15 {
            80.0 - ((self.total_calls - 8) as f64 * 4.0)
        } else {
            50.0 - ((self.total_calls - 15) as f64 * 5.0).min(40.0)
        };

        percentage.max(10.0) // Floor at 10%
    }

    /// Suggest optimization
    pub fn suggest_optimization(&self) -> Option<String> {
        // If read_file is overused
        if self.is_overused("read_file") {
            return Some(
                "Consider using 'grep' or 'glob' to narrow down files before reading.".to_string()
            );
        }

        // If grep is overused
        if self.is_overused("grep") {
            return Some(
                "You've searched many times. Read the files you found instead of searching more.".to_string()
            );
        }

        // If bash is overused
        if self.get_call_count("bash") >= 4 {
            return Some(
                "Combine bash commands with && instead of separate calls.".to_string()
            );
        }

        // If too many total calls
        if self.total_calls >= 20 {
            return Some(
                "Critical: Approaching 30 tool limit. Finish your current task immediately.".to_string()
            );
        } else if self.total_calls >= 12 {
            return Some(
                "You're using many tools. Focus on answering with what you have.".to_string()
            );
        }

        None
    }
}

impl Default for ToolMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_call() {
        let mut monitor = ToolMonitor::new();
        monitor.record_call("read_file", 100);
        monitor.record_call("read_file", 200);

        assert_eq!(monitor.total_calls(), 2);
        let stats = monitor.stats.get("read_file").unwrap();
        assert_eq!(stats.call_count, 2);
        assert_eq!(stats.average_tokens, 150);
    }

    #[test]
    fn test_overuse_detection() {
        let mut monitor = ToolMonitor::new();

        for _ in 0..11 {
            monitor.record_call("read_file", 100);
        }

        assert!(monitor.is_overused("read_file"));
    }

    #[test]
    fn test_most_used_tools() {
        let mut monitor = ToolMonitor::new();

        monitor.record_call("read_file", 100);
        monitor.record_call("read_file", 100);
        monitor.record_call("grep", 50);

        let most_used = monitor.most_used_tools();
        assert_eq!(most_used[0].0, "read_file");
        assert_eq!(most_used[0].1, 2);
    }
}
