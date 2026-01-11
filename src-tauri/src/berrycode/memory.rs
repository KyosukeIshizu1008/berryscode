//! Memory system for user preferences and project-specific rules
//!
//! Inspired by Claude's Project feature, this module manages persistent user preferences
//! that are automatically injected into the system prompt.

use std::path::{Path, PathBuf};
use std::fs;
use anyhow::{Result, Context};

/// Memory manager for user preferences
pub struct Memory {
    memory_file: PathBuf,
    content: Option<String>,
}

impl Memory {
    /// Create a new Memory manager
    pub fn new(project_root: &Path) -> Self {
        let memory_file = project_root.join(".berrycode").join("memory.md");

        Self {
            memory_file,
            content: None,
        }
    }

    /// Create with custom memory file path
    pub fn with_file(memory_file: PathBuf) -> Self {
        Self {
            memory_file,
            content: None,
        }
    }

    /// Load memory from file
    pub fn load(&mut self) -> Result<()> {
        if !self.memory_file.exists() {
            tracing::debug!("Memory file not found: {:?}", self.memory_file);
            self.content = None;
            return Ok(());
        }

        let content = fs::read_to_string(&self.memory_file)
            .with_context(|| format!("Failed to read memory file: {:?}", self.memory_file))?;

        if content.trim().is_empty() {
            self.content = None;
        } else {
            let len = content.len();
            self.content = Some(content);
            tracing::info!("Loaded memory from {:?} ({} bytes)", self.memory_file, len);
        }

        Ok(())
    }

    /// Get memory content
    pub fn get(&self) -> Option<&str> {
        self.content.as_deref()
    }

    /// Check if memory is loaded
    pub fn is_loaded(&self) -> bool {
        self.content.is_some()
    }

    /// Save new memory content
    pub fn save(&mut self, content: &str) -> Result<()> {
        // Create .berrycode directory if it doesn't exist
        if let Some(parent) = self.memory_file.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {:?}", parent))?;
        }

        fs::write(&self.memory_file, content)
            .with_context(|| format!("Failed to write memory file: {:?}", self.memory_file))?;

        self.content = Some(content.to_string());

        tracing::info!("Saved memory to {:?}", self.memory_file);

        Ok(())
    }

    /// Append to existing memory
    pub fn append(&mut self, additional: &str) -> Result<()> {
        let new_content = match &self.content {
            Some(existing) => format!("{}\n\n{}", existing, additional),
            None => additional.to_string(),
        };

        self.save(&new_content)
    }

    /// Clear memory
    pub fn clear(&mut self) -> Result<()> {
        if self.memory_file.exists() {
            fs::remove_file(&self.memory_file)
                .with_context(|| format!("Failed to remove memory file: {:?}", self.memory_file))?;
        }

        self.content = None;

        tracing::info!("Cleared memory");

        Ok(())
    }

    /// Format memory for system prompt injection
    pub fn format_for_prompt(&self) -> Option<String> {
        self.content.as_ref().map(|content| {
            format!(
                r#"## User Preferences & Project Memory

The following preferences and project-specific rules should be followed:

{}

---
"#,
                content
            )
        })
    }

    /// Create default memory file template
    pub fn create_template(&self) -> Result<()> {
        let template = r#"# Project Memory

This file contains user preferences and project-specific rules that BerryCode will remember.

## Language Preferences
- Always respond in Japanese (日本語で答えてください)

## Code Style
- Use descriptive variable names
- Add comments for complex logic
- Follow Rust best practices

## Testing
- Write tests for new features
- Use `cargo test` to verify changes

## Communication Style
- Be concise and clear
- Show code examples when explaining
- Point to specific files and line numbers

## Project-Specific Rules
- This is a Rust CLI tool project
- Main executable: `berrycode`
- Web version: `berrycode-web` (requires `web` feature)

---

You can edit this file to customize how BerryCode behaves in this project.
"#;

        // Create .berrycode directory if it doesn't exist
        if let Some(parent) = self.memory_file.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {:?}", parent))?;
        }

        fs::write(&self.memory_file, template)
            .with_context(|| format!("Failed to write template: {:?}", self.memory_file))?;

        tracing::info!("Created memory template at {:?}", self.memory_file);

        Ok(())
    }

    /// Get memory file path
    pub fn path(&self) -> &Path {
        &self.memory_file
    }
}

impl Default for Memory {
    fn default() -> Self {
        Self::new(Path::new("."))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_memory_new() {
        let temp_dir = TempDir::new().unwrap();
        let memory = Memory::new(temp_dir.path());

        assert!(memory.path().ends_with(".berrycode/memory.md"));
        assert!(!memory.is_loaded());
    }

    #[test]
    fn test_memory_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let mut memory = Memory::new(temp_dir.path());

        let content = "Always use Japanese\nFollow Rust style guide";
        memory.save(content).unwrap();

        assert!(memory.is_loaded());
        assert_eq!(memory.get(), Some(content));

        // Load again
        let mut memory2 = Memory::new(temp_dir.path());
        memory2.load().unwrap();
        assert_eq!(memory2.get(), Some(content));
    }

    #[test]
    fn test_memory_append() {
        let temp_dir = TempDir::new().unwrap();
        let mut memory = Memory::new(temp_dir.path());

        memory.save("First rule").unwrap();
        memory.append("Second rule").unwrap();

        assert!(memory.get().unwrap().contains("First rule"));
        assert!(memory.get().unwrap().contains("Second rule"));
    }

    #[test]
    fn test_memory_clear() {
        let temp_dir = TempDir::new().unwrap();
        let mut memory = Memory::new(temp_dir.path());

        memory.save("Some content").unwrap();
        assert!(memory.is_loaded());

        memory.clear().unwrap();
        assert!(!memory.is_loaded());
        assert!(!memory.path().exists());
    }

    #[test]
    fn test_format_for_prompt() {
        let temp_dir = TempDir::new().unwrap();
        let mut memory = Memory::new(temp_dir.path());

        memory.save("Use Japanese\nFollow best practices").unwrap();

        let formatted = memory.format_for_prompt().unwrap();
        assert!(formatted.contains("User Preferences"));
        assert!(formatted.contains("Use Japanese"));
        assert!(formatted.contains("Follow best practices"));
    }

    #[test]
    fn test_create_template() {
        let temp_dir = TempDir::new().unwrap();
        let memory = Memory::new(temp_dir.path());

        memory.create_template().unwrap();
        assert!(memory.path().exists());

        let content = fs::read_to_string(memory.path()).unwrap();
        assert!(content.contains("Project Memory"));
        assert!(content.contains("Language Preferences"));
    }

    #[test]
    fn test_nonexistent_file() {
        let temp_dir = TempDir::new().unwrap();
        let mut memory = Memory::new(temp_dir.path());

        // Should not error when file doesn't exist
        memory.load().unwrap();
        assert!(!memory.is_loaded());
        assert_eq!(memory.get(), None);
    }
}
