//! Backend Syntax Highlighter Tests (Tauri側のUnit Test)
//!
//! highlight_file_parallelが正しいHTMLを生成することを検証

#[cfg(test)]
mod tests {
    use crate::syntax_highlighter::{highlight_file_parallel, HighlightResult, HighlightedToken};

    #[tokio::test]
    async fn test_highlight_returns_unescaped_html() {
        let lines = vec![(0, "fn main() {}".to_string())];

        let results = highlight_file_parallel("test.rs".to_string(), lines)
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        let result = &results[0];

        // Tokens should contain plain text, not HTML entities
        for token in &result.tokens {
            // Should NOT contain escaped characters
            assert!(!token.text.contains("&lt;"));
            assert!(!token.text.contains("&gt;"));
            assert!(!token.text.contains("&quot;"));

            // Color should be hex format
            assert!(token.color.starts_with('#'));
        }
    }

    #[tokio::test]
    async fn test_highlight_rust_keywords() {
        let lines = vec![
            (0, "fn test() {}".to_string()),
            (1, "let x = 42;".to_string()),
        ];

        let results = highlight_file_parallel("test.rs".to_string(), lines)
            .await
            .unwrap();

        assert_eq!(results.len(), 2);

        // First line should have "fn" highlighted
        let line0_tokens = &results[0].tokens;
        let fn_token = line0_tokens
            .iter()
            .find(|t| t.text == "fn")
            .expect("Should find 'fn' keyword");

        // Should be IntelliJ Darcula keyword color
        assert_eq!(fn_token.color, "#CC7832");
    }

    #[tokio::test]
    async fn test_highlight_preserves_spaces() {
        let lines = vec![(0, "    fn indented() {}".to_string())];

        let results = highlight_file_parallel("test.rs".to_string(), lines)
            .await
            .unwrap();

        let tokens = &results[0].tokens;

        // First token should be spaces
        assert_eq!(tokens[0].text, "    ");
    }

    #[tokio::test]
    async fn test_highlight_strings() {
        let lines = vec![(0, r#"let s = "hello";"#.to_string())];

        let results = highlight_file_parallel("test.rs".to_string(), lines)
            .await
            .unwrap();

        let tokens = &results[0].tokens;

        let string_token = tokens
            .iter()
            .find(|t| t.text.contains("hello"))
            .expect("Should find string token");

        // Should be IntelliJ Darcula string color
        assert_eq!(string_token.color, "#6A8759");
    }

    #[tokio::test]
    async fn test_highlight_markdown() {
        let lines = vec![
            (0, "# Heading".to_string()),
            (1, "Some text".to_string()),
        ];

        let results = highlight_file_parallel("README.md".to_string(), lines)
            .await
            .unwrap();

        assert_eq!(results.len(), 2);
        // Markdown highlighting should work
        assert!(!results[0].tokens.is_empty());
    }

    #[tokio::test]
    async fn test_highlight_empty_line() {
        let lines = vec![(0, "".to_string())];

        let results = highlight_file_parallel("test.rs".to_string(), lines)
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        // Empty line should have empty or whitespace token
        assert!(results[0].tokens.is_empty() || results[0].tokens[0].text.is_empty());
    }

    #[tokio::test]
    async fn test_highlight_special_characters() {
        let lines = vec![(0, "if x < y && y > z {}".to_string())];

        let results = highlight_file_parallel("test.rs".to_string(), lines)
            .await
            .unwrap();

        let tokens = &results[0].tokens;

        // Should contain < and > as plain text, not escaped
        let has_lt = tokens.iter().any(|t| t.text.contains('<'));
        let has_gt = tokens.iter().any(|t| t.text.contains('>'));

        assert!(has_lt || has_gt, "Should contain comparison operators");

        // Should NOT be HTML-escaped
        for token in tokens {
            assert!(!token.text.contains("&lt;"));
            assert!(!token.text.contains("&gt;"));
        }
    }

    #[tokio::test]
    async fn test_token_structure() {
        let lines = vec![(0, "fn".to_string())];

        let results = highlight_file_parallel("test.rs".to_string(), lines)
            .await
            .unwrap();

        let token = &results[0].tokens[0];

        // Verify token structure
        assert_eq!(token.text, "fn");
        assert!(token.start == 0);
        assert!(token.end == 2);
        assert!(!token.color.is_empty());
    }

    #[tokio::test]
    async fn test_large_file_performance() {
        // 1000 lines
        let lines: Vec<(usize, String)> = (0..1000)
            .map(|i| (i, format!("fn func_{}() {{}}", i)))
            .collect();

        let start = std::time::Instant::now();
        let results = highlight_file_parallel("large.rs".to_string(), lines)
            .await
            .unwrap();
        let duration = start.elapsed();

        assert_eq!(results.len(), 1000);
        // Should complete in reasonable time (< 1 second)
        assert!(duration.as_millis() < 1000, "Highlighting took too long");
    }
}
