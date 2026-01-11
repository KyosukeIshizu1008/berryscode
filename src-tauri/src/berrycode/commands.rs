//! Command system for aider

use crate::berrycode::Result;
use crate::berrycode::io::InputOutput;
use crate::berrycode::coders::Coder;

pub struct Commands {
    io: InputOutput,
}

impl Commands {
    pub fn new(io: InputOutput) -> Self {
        Self { io }
    }

    /// Get list of available commands
    pub fn get_commands(&self) -> Vec<String> {
        vec![
            "/add".to_string(),
            "/drop".to_string(),
            "/undo".to_string(),
            "/diff".to_string(),
            "/commit".to_string(),
            "/help".to_string(),
            "/quit".to_string(),
            "/exit".to_string(),
            "/clear".to_string(),
            "/tokens".to_string(),
            "/model".to_string(),
            "/files".to_string(),
        ]
    }

    /// Execute a command
    pub fn execute(&mut self, command: &str, coder: &mut Coder) -> Result<bool> {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(true);
        }

        match parts[0] {
            "/help" => {
                self.show_help();
                Ok(true)
            }
            "/quit" | "/exit" => {
                self.io.tool_output("Goodbye!");
                Ok(false)
            }
            "/add" => {
                if parts.len() < 2 {
                    self.io.tool_error("Usage: /add <file>");
                } else {
                    let file_path = std::path::PathBuf::from(parts[1]);
                    if file_path.exists() {
                        coder.add_file(file_path)?;
                    } else {
                        self.io.tool_error(&format!("File not found: {}", parts[1]));
                    }
                }
                Ok(true)
            }
            "/drop" => {
                if parts.len() < 2 {
                    self.io.tool_error("Usage: /drop <file>");
                } else {
                    let file_path = std::path::PathBuf::from(parts[1]);
                    coder.drop_file(&file_path)?;
                }
                Ok(true)
            }
            "/undo" => {
                coder.undo_last_commit()?;
                Ok(true)
            }
            "/diff" => {
                coder.show_diff()?;
                Ok(true)
            }
            "/commit" => {
                if let Some(ref repo) = coder.git_repo {
                    let message = if parts.len() > 1 {
                        parts[1..].join(" ")
                    } else {
                        "Manual commit from aider".to_string()
                    };
                    repo.commit(&message)?;
                    self.io.tool_output("Changes committed");
                } else {
                    self.io.tool_warning("Not in a git repository");
                }
                Ok(true)
            }
            "/clear" => {
                coder.chat_history.clear();
                self.io.tool_output("Chat history cleared");
                Ok(true)
            }
            "/tokens" => {
                let total_messages = coder.chat_history.len();
                let total_chars: usize = coder.chat_history.iter().map(|m| m.content.len()).sum();
                self.io.tool_output(&format!("Messages: {}", total_messages));
                self.io.tool_output(&format!("Approximate characters: {}", total_chars));
                self.io.tool_output(&format!("Approximate tokens: ~{}", total_chars / 4));
                Ok(true)
            }
            "/model" => {
                self.io.tool_output(&format!("Current model: {}", coder.model.name));
                self.io.tool_output(&format!("Edit format: {}", coder.model.edit_format.as_ref().unwrap_or(&"default".to_string())));
                Ok(true)
            }
            "/files" => {
                if coder.files.is_empty() {
                    self.io.tool_output("No files in chat");
                } else {
                    self.io.tool_output("Files in chat:");
                    for file in &coder.files {
                        self.io.tool_output(&format!("  - {}", file.display()));
                    }
                }
                Ok(true)
            }
            _ => {
                self.io.tool_error(&format!("Unknown command: {}", parts[0]));
                self.io.tool_output("Type /help for a list of commands");
                Ok(true)
            }
        }
    }

    fn show_help(&mut self) {
        self.io.tool_output("Available commands:");
        self.io.tool_output("  /add <file>    - Add a file to the chat");
        self.io.tool_output("  /drop <file>   - Remove a file from the chat");
        self.io.tool_output("  /undo          - Undo the last git commit");
        self.io.tool_output("  /diff          - Show diff of uncommitted changes");
        self.io.tool_output("  /commit        - Commit changes with a message");
        self.io.tool_output("  /clear         - Clear the chat history");
        self.io.tool_output("  /tokens        - Show token usage");
        self.io.tool_output("  /model         - Show current model");
        self.io.tool_output("  /files         - List files in chat");
        self.io.tool_output("  /help          - Show this help message");
        self.io.tool_output("  /quit, /exit   - Exit aider");
    }
}

pub struct SwitchCoder {
    pub new_edit_format: String,
}
