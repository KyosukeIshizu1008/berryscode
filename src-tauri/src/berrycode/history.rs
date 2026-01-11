//! Chat history management for aider

use std::path::{Path, PathBuf};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use crate::berrycode::Result;

pub struct ChatHistory {
    history_file: PathBuf,
    messages: Vec<ChatMessage>,
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl ChatHistory {
    pub fn new(history_file: &Path) -> Result<Self> {
        let mut history = Self {
            history_file: history_file.to_path_buf(),
            messages: Vec::new(),
        };

        // Try to load existing history
        let _ = history.load();

        Ok(history)
    }

    pub fn add_message(&mut self, role: String, content: String) {
        self.messages.push(ChatMessage {
            role,
            content,
            timestamp: chrono::Utc::now(),
        });
    }

    pub fn save(&self) -> Result<()> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = self.history_file.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut file = fs::File::create(&self.history_file)?;

        // Save as markdown format
        writeln!(file, "# Aider Chat History\n")?;

        for msg in &self.messages {
            writeln!(file, "## {} - {}", msg.role, msg.timestamp.format("%Y-%m-%d %H:%M:%S"))?;
            writeln!(file, "\n{}\n", msg.content)?;
            writeln!(file, "---\n")?;
        }

        Ok(())
    }

    pub fn load(&mut self) -> Result<()> {
        if !self.history_file.exists() {
            return Ok(());
        }

        let file = fs::File::open(&self.history_file)?;
        let reader = BufReader::new(file);

        let mut current_role = None;
        let mut current_content = String::new();

        for line in reader.lines() {
            let line = line?;

            if line.starts_with("## ") {
                // Save previous message if exists
                if let Some(role) = current_role.take() {
                    if !current_content.trim().is_empty() {
                        self.messages.push(ChatMessage {
                            role,
                            content: current_content.trim().to_string(),
                            timestamp: chrono::Utc::now(),
                        });
                    }
                    current_content.clear();
                }

                // Parse new message header
                let parts: Vec<&str> = line.trim_start_matches("## ").split(" - ").collect();
                if !parts.is_empty() {
                    current_role = Some(parts[0].to_string());
                }
            } else if line == "---" {
                // Message separator
                continue;
            } else if line.starts_with("# ") {
                // Title, skip
                continue;
            } else if current_role.is_some() {
                current_content.push_str(&line);
                current_content.push('\n');
            }
        }

        // Save last message
        if let Some(role) = current_role {
            if !current_content.trim().is_empty() {
                self.messages.push(ChatMessage {
                    role,
                    content: current_content.trim().to_string(),
                    timestamp: chrono::Utc::now(),
                });
            }
        }

        Ok(())
    }

    pub fn clear(&mut self) {
        self.messages.clear();
    }

    pub fn get_messages(&self) -> &[ChatMessage] {
        &self.messages
    }

    pub fn get_recent(&self, count: usize) -> Vec<&ChatMessage> {
        let start = self.messages.len().saturating_sub(count);
        self.messages[start..].iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_chat_history_new() {
        let temp_dir = TempDir::new().unwrap();
        let history_file = temp_dir.path().join("history.md");

        let history = ChatHistory::new(&history_file);
        assert!(history.is_ok());

        let history = history.unwrap();
        assert_eq!(history.messages.len(), 0);
    }

    #[test]
    fn test_add_message() {
        let temp_dir = TempDir::new().unwrap();
        let history_file = temp_dir.path().join("history.md");

        let mut history = ChatHistory::new(&history_file).unwrap();

        history.add_message("user".to_string(), "Hello".to_string());
        history.add_message("assistant".to_string(), "Hi there!".to_string());

        assert_eq!(history.messages.len(), 2);
        assert_eq!(history.messages[0].role, "user");
        assert_eq!(history.messages[0].content, "Hello");
        assert_eq!(history.messages[1].role, "assistant");
        assert_eq!(history.messages[1].content, "Hi there!");
    }

    #[test]
    fn test_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let history_file = temp_dir.path().join("history.md");

        // Create and save history
        {
            let mut history = ChatHistory::new(&history_file).unwrap();
            history.add_message("user".to_string(), "Hello".to_string());
            history.add_message("assistant".to_string(), "Hi there!".to_string());

            let result = history.save();
            assert!(result.is_ok());
        }

        // Load history in a new instance
        {
            let history = ChatHistory::new(&history_file).unwrap();
            assert_eq!(history.messages.len(), 2);
            assert_eq!(history.messages[0].role, "user");
            assert_eq!(history.messages[0].content, "Hello");
            assert_eq!(history.messages[1].role, "assistant");
            assert_eq!(history.messages[1].content, "Hi there!");
        }
    }

    #[test]
    fn test_clear() {
        let temp_dir = TempDir::new().unwrap();
        let history_file = temp_dir.path().join("history.md");

        let mut history = ChatHistory::new(&history_file).unwrap();
        history.add_message("user".to_string(), "Hello".to_string());
        history.add_message("assistant".to_string(), "Hi there!".to_string());

        assert_eq!(history.messages.len(), 2);

        history.clear();
        assert_eq!(history.messages.len(), 0);
    }

    #[test]
    fn test_get_messages() {
        let temp_dir = TempDir::new().unwrap();
        let history_file = temp_dir.path().join("history.md");

        let mut history = ChatHistory::new(&history_file).unwrap();
        history.add_message("user".to_string(), "Hello".to_string());
        history.add_message("assistant".to_string(), "Hi there!".to_string());

        let messages = history.get_messages();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[1].role, "assistant");
    }

    #[test]
    fn test_get_recent() {
        let temp_dir = TempDir::new().unwrap();
        let history_file = temp_dir.path().join("history.md");

        let mut history = ChatHistory::new(&history_file).unwrap();
        history.add_message("user".to_string(), "Message 1".to_string());
        history.add_message("assistant".to_string(), "Message 2".to_string());
        history.add_message("user".to_string(), "Message 3".to_string());
        history.add_message("assistant".to_string(), "Message 4".to_string());

        // Get last 2 messages
        let recent = history.get_recent(2);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].content, "Message 3");
        assert_eq!(recent[1].content, "Message 4");

        // Get more messages than available
        let all = history.get_recent(10);
        assert_eq!(all.len(), 4);
    }

    #[test]
    fn test_save_creates_parent_directory() {
        let temp_dir = TempDir::new().unwrap();
        let history_file = temp_dir.path().join("subdir").join("history.md");

        let mut history = ChatHistory::new(&history_file).unwrap();
        history.add_message("user".to_string(), "Test".to_string());

        let result = history.save();
        assert!(result.is_ok());
        assert!(history_file.exists());
    }

    #[test]
    fn test_load_nonexistent_file() {
        let temp_dir = TempDir::new().unwrap();
        let history_file = temp_dir.path().join("nonexistent.md");

        let mut history = ChatHistory::new(&history_file).unwrap();
        let result = history.load();
        assert!(result.is_ok());
        assert_eq!(history.messages.len(), 0);
    }

    #[test]
    fn test_chat_message_timestamp() {
        let temp_dir = TempDir::new().unwrap();
        let history_file = temp_dir.path().join("history.md");

        let mut history = ChatHistory::new(&history_file).unwrap();

        let before = chrono::Utc::now();
        history.add_message("user".to_string(), "Test".to_string());
        let after = chrono::Utc::now();

        assert_eq!(history.messages.len(), 1);
        let timestamp = history.messages[0].timestamp;
        assert!(timestamp >= before && timestamp <= after);
    }
}
