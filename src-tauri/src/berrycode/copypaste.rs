//! Clipboard monitoring and code detection

use crate::berrycode::Result;
use anyhow::Context;
use arboard::Clipboard;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

/// Content type detected in clipboard
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "language")]
pub enum ContentType {
    /// Code content with detected language
    Code(String),
    /// Plain text
    Text,
    /// URL
    Url,
    /// JSON data
    Json,
    /// Markdown
    Markdown,
}

/// Clipboard content with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardContent {
    /// The actual text content
    pub text: String,
    /// Detected content type
    pub content_type: ContentType,
    /// When this was captured
    pub timestamp: u64,
}

/// Clipboard watcher
pub struct ClipboardWatcher {
    clipboard: Arc<Mutex<Clipboard>>,
    last_content: Arc<Mutex<String>>,
}

impl ClipboardWatcher {
    /// Create a new clipboard watcher
    pub fn new() -> Result<Self> {
        let clipboard = Clipboard::new()
            .context("Failed to initialize clipboard")?;

        Ok(Self {
            clipboard: Arc::new(Mutex::new(clipboard)),
            last_content: Arc::new(Mutex::new(String::new())),
        })
    }

    /// Check clipboard for new content
    pub fn check(&self) -> Result<Option<ClipboardContent>> {
        let mut clipboard = self.clipboard.lock().unwrap();
        let current = clipboard.get_text().unwrap_or_default();

        let mut last = self.last_content.lock().unwrap();

        // Only process if content changed
        if current.is_empty() || current == *last {
            return Ok(None);
        }

        // Update last content
        *last = current.clone();

        // Detect content type
        let content_type = detect_content_type(&current);

        // Only return if it's potentially interesting (code, JSON, markdown, or URLs)
        if matches!(content_type, ContentType::Text) && current.len() < 20 {
            return Ok(None);
        }

        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Ok(Some(ClipboardContent {
            text: current,
            content_type,
            timestamp,
        }))
    }

    /// Get current clipboard content without updating last content
    pub fn get_current(&self) -> Result<Option<ClipboardContent>> {
        let mut clipboard = self.clipboard.lock().unwrap();
        let current = clipboard.get_text().unwrap_or_default();

        if current.is_empty() {
            return Ok(None);
        }

        let content_type = detect_content_type(&current);
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Ok(Some(ClipboardContent {
            text: current,
            content_type,
            timestamp,
        }))
    }

    /// Set clipboard content
    pub fn set(&self, text: &str) -> Result<()> {
        let mut clipboard = self.clipboard.lock().unwrap();
        clipboard.set_text(text)
            .context("Failed to set clipboard content")?;

        // Update last content to avoid detecting our own set
        *self.last_content.lock().unwrap() = text.to_string();

        Ok(())
    }

    /// Legacy watch method (deprecated)
    pub fn watch(&mut self) -> Result<()> {
        // This method is now deprecated in favor of check()
        Ok(())
    }
}

/// Detect content type from text
fn detect_content_type(text: &str) -> ContentType {
    let trimmed = text.trim();

    // Check for URLs
    if is_url(trimmed) {
        return ContentType::Url;
    }

    // Check for JSON
    if is_json(trimmed) {
        return ContentType::Json;
    }

    // Check for Markdown
    if is_markdown(trimmed) {
        return ContentType::Markdown;
    }

    // Check for code
    if let Some(language) = detect_code_language(trimmed) {
        return ContentType::Code(language);
    }

    ContentType::Text
}

/// Check if text is a URL
fn is_url(text: &str) -> bool {
    text.starts_with("http://")
        || text.starts_with("https://")
        || text.starts_with("ftp://")
        || text.starts_with("ws://")
        || text.starts_with("wss://")
}

/// Check if text is JSON
fn is_json(text: &str) -> bool {
    (text.starts_with('{') && text.ends_with('}'))
        || (text.starts_with('[') && text.ends_with(']'))
}

/// Check if text is Markdown
fn is_markdown(text: &str) -> bool {
    let lines: Vec<&str> = text.lines().collect();

    // Check for Markdown indicators
    lines.iter().any(|line| {
        line.starts_with('#')
            || line.starts_with("```")
            || line.starts_with('-')
            || line.starts_with('*')
            || line.starts_with('>')
            || line.contains("](")
    })
}

/// Detect programming language from code
fn detect_code_language(text: &str) -> Option<String> {
    let indicators = vec![
        // Rust
        ("Rust", vec!["fn ", "let ", "pub ", "impl ", "trait ", "use ", "->", "::", "unwrap()"]),
        // Python
        ("Python", vec!["def ", "import ", "from ", "class ", "__init__", "self.", "elif "]),
        // JavaScript/TypeScript
        ("JavaScript", vec!["function ", "const ", "let ", "var ", "=>", "console.log", "async "]),
        ("TypeScript", vec!["interface ", "type ", ": string", ": number", "as ", "export "]),
        // Go
        ("Go", vec!["func ", "package ", "import ", "type ", "interface ", "go ", ":= "]),
        // Java
        ("Java", vec!["public class", "private ", "protected ", "void ", "new ", "System.out"]),
        // C/C++
        ("C++", vec!["#include", "int main", "std::", "cout <<", "template<", "namespace "]),
        ("C", vec!["#include", "int main", "printf", "scanf", "malloc", "free("]),
        // SQL
        ("SQL", vec!["SELECT ", "FROM ", "WHERE ", "INSERT INTO", "UPDATE ", "CREATE TABLE"]),
        // HTML/CSS
        ("HTML", vec!["<html", "<div", "<span", "<body", "</", "class="]),
        ("CSS", vec!["{", "}", ":", ";", "px;", "color:", "display:"]),
        // Shell
        ("Bash", vec!["#!/bin/bash", "#!/bin/sh", "echo ", "if [", "for ", "while "]),
    ];

    // Count indicators for each language
    let mut scores: Vec<(String, usize)> = indicators
        .iter()
        .map(|(lang, keywords)| {
            let score = keywords
                .iter()
                .filter(|&&keyword| text.contains(keyword))
                .count();
            (lang.to_string(), score)
        })
        .collect();

    // Sort by score descending
    scores.sort_by(|a, b| b.1.cmp(&a.1));

    // Return language if we found at least 2 indicators
    if scores[0].1 >= 2 {
        Some(scores[0].0.clone())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_url() {
        assert!(is_url("https://example.com"));
        assert!(is_url("http://localhost:3000"));
        assert!(!is_url("not a url"));
    }

    #[test]
    fn test_detect_json() {
        assert!(is_json(r#"{"key": "value"}"#));
        assert!(is_json(r#"[1, 2, 3]"#));
        assert!(!is_json("not json"));
    }

    #[test]
    fn test_detect_markdown() {
        assert!(is_markdown("# Heading\nSome text"));
        assert!(is_markdown("- List item\n- Another item"));
        assert!(is_markdown("[Link](https://example.com)"));
        assert!(!is_markdown("Plain text"));
    }

    #[test]
    fn test_detect_rust_code() {
        let rust_code = r#"
fn main() {
    let x = 42;
    println!("Hello");
}
"#;
        assert_eq!(
            detect_code_language(rust_code),
            Some("Rust".to_string())
        );
    }

    #[test]
    fn test_detect_python_code() {
        let python_code = r#"
def hello():
    print("Hello")
    import sys
"#;
        assert_eq!(
            detect_code_language(python_code),
            Some("Python".to_string())
        );
    }

    #[test]
    fn test_detect_javascript_code() {
        let js_code = r#"
function hello() {
    const msg = "Hello";
    console.log(msg);
}
"#;
        assert_eq!(
            detect_code_language(js_code),
            Some("JavaScript".to_string())
        );
    }

    #[test]
    fn test_content_type_detection() {
        assert!(matches!(
            detect_content_type("https://example.com"),
            ContentType::Url
        ));

        assert!(matches!(
            detect_content_type(r#"{"key": "value"}"#),
            ContentType::Json
        ));

        assert!(matches!(
            detect_content_type("# Title\n- Item"),
            ContentType::Markdown
        ));

        let rust_code = "fn main() { let x = 42; }";
        if let ContentType::Code(lang) = detect_content_type(rust_code) {
            assert_eq!(lang, "Rust");
        } else {
            panic!("Expected Code type");
        }
    }
}
