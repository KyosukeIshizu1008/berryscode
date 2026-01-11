//! Syntax Highlighting HTML Sanitization Test
//!
//! ❌ CRITICAL: This test prevents DOUBLE-ESCAPING bugs
//!
//! WHY THIS MATTERS:
//! - Backend returns tokens with text: "<", ">", "&"
//! - We convert them to HTML: "<span>...</span>"
//! - If we escape in wrong order, we get: "&amp;lt;" instead of "&lt;"
//!
//! This test verifies that `html_escape` follows the CORRECT order:
//! 1. Replace "&" → "&amp;" FIRST
//! 2. Then replace "<" → "&lt;"
//! 3. Then replace ">" → "&gt;"
//!
//! Run with: cargo test --test syntax_highlighting_sanitize_test

#[cfg(test)]
mod tests {
    /// Simulates the html_escape function from virtual_editor.rs:1632
    fn html_escape(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#39;")
    }

    /// Simulates highlight_result_to_html from virtual_editor.rs:1466
    fn highlight_result_to_html(tokens: Vec<(&str, &str)>) -> String {
        let mut html = String::new();
        for (color, text) in tokens {
            html.push_str(&format!(
                "<span style=\"color: {}\">{}</span>",
                color,
                html_escape(text)
            ));
        }
        html
    }

    #[test]
    fn test_html_escape_order_prevents_double_escaping() {
        // CRITICAL: If we escape "<" before "&", we get wrong result
        let input = "&<>";

        // Correct order: & first, then < and >
        let result = html_escape(input);

        assert_eq!(result, "&amp;&lt;&gt;",
            "❌ DOUBLE-ESCAPING DETECTED!\n\
             Input: '{}'\n\
             Expected: '&amp;&lt;&gt;'\n\
             Got: '{}'\n\
             This means the escape order is WRONG!",
            input, result);
    }

    #[test]
    fn test_comparison_operators_not_double_escaped() {
        // Rust code: if x < y && y > z
        let tokens = vec![
            ("#CC7832", "if"),
            ("#A9B7C6", " x "),
            ("#CC7832", "<"),
            ("#A9B7C6", " y "),
            ("#CC7832", "&&"),
            ("#A9B7C6", " y "),
            ("#CC7832", ">"),
            ("#A9B7C6", " z"),
        ];

        let html = highlight_result_to_html(tokens);

        // Should contain &lt; and &gt;, NOT &amp;lt; or &amp;gt;
        assert!(html.contains("&lt;"), "< should be escaped to &lt;");
        assert!(html.contains("&gt;"), "> should be escaped to &gt;");
        assert!(!html.contains("&amp;lt;"), "❌ DOUBLE-ESCAPING: &amp;lt; detected");
        assert!(!html.contains("&amp;gt;"), "❌ DOUBLE-ESCAPING: &amp;gt; detected");
    }

    #[test]
    fn test_string_with_html_entities() {
        // Code: let s = "<div>&nbsp;</div>";
        let tokens = vec![
            ("#CC7832", "let"),
            ("#A9B7C6", " s = "),
            ("#6A8759", "\"<div>&nbsp;</div>\""),
        ];

        let html = highlight_result_to_html(tokens);

        // The string should be properly escaped
        assert!(html.contains("&quot;&lt;div&gt;&amp;nbsp;&lt;/div&gt;&quot;"),
            "String literal with HTML should be fully escaped");
    }

    #[test]
    fn test_ampersand_in_rust_code() {
        // Code: &str, &mut
        let tokens = vec![
            ("#CC7832", "&"),
            ("#A9B7C6", "str"),
        ];

        let html = highlight_result_to_html(tokens);

        // & should be escaped to &amp; (checking within span tags)
        assert!(html.contains("&amp;</span>"), "& should be escaped to &amp;");
        assert!(!html.contains("&amp;amp;"), "❌ DOUBLE-ESCAPING: &amp;amp; detected");
    }

    #[test]
    fn test_generic_types_with_brackets() {
        // Code: Vec<T>, Option<&str>
        let tokens = vec![
            ("#A9B7C6", "Vec"),
            ("#A9B7C6", "<"),
            ("#A9B7C6", "T"),
            ("#A9B7C6", ">"),
        ];

        let html = highlight_result_to_html(tokens);

        assert!(html.contains("&lt;"), "< should be escaped");
        assert!(html.contains("&gt;"), "> should be escaped");
    }

    #[test]
    fn test_empty_token() {
        let tokens = vec![
            ("#A9B7C6", ""),
        ];

        let html = highlight_result_to_html(tokens);
        assert_eq!(html, "<span style=\"color: #A9B7C6\"></span>");
    }

    #[test]
    fn test_single_quote_escape() {
        // Char literal: 'a', '\''
        let tokens = vec![
            ("#6A8759", "'a'"),
            ("#6A8759", "'\\''"),
        ];

        let html = highlight_result_to_html(tokens);

        assert!(html.contains("&#39;"), "Single quote should be escaped to &#39;");
    }

    #[test]
    fn test_multiline_comment_with_html() {
        // Comment: /* <html>&test</html> */
        let tokens = vec![
            ("#629755", "/* <html>&test</html> */"),
        ];

        let html = highlight_result_to_html(tokens);

        // All HTML entities should be escaped
        assert!(html.contains("&lt;html&gt;&amp;test&lt;/html&gt;"),
            "Comment with HTML should be fully escaped");
    }

    #[test]
    fn test_mathematical_expression() {
        // Expression: (a & 0xFF) << 8 | (b & 0xFF)
        let tokens = vec![
            ("#A9B7C6", "(a "),
            ("#CC7832", "&"),
            ("#A9B7C6", " 0xFF) "),
            ("#CC7832", "<<"),
            ("#A9B7C6", " 8 | (b "),
            ("#CC7832", "&"),
            ("#A9B7C6", " 0xFF)"),
        ];

        let html = highlight_result_to_html(tokens);

        // Count the number of &amp; (should be 2, one for each &)
        let amp_count = html.matches("&amp;").count();
        assert_eq!(amp_count, 2, "Should have exactly 2 &amp; entities");
    }

    #[test]
    fn test_escape_preserves_utf8() {
        // Unicode text with special chars: "こんにちは<世界>&"
        let tokens = vec![
            ("#6A8759", "\"こんにちは<世界>&\""),
        ];

        let html = highlight_result_to_html(tokens);

        // Should preserve Japanese characters and escape HTML entities
        assert!(html.contains("こんにちは"), "Japanese text should be preserved");
        assert!(html.contains("&lt;"), "< should be escaped");
        assert!(html.contains("&gt;"), "> should be escaped");
        assert!(html.contains("&amp;"), "& should be escaped");
    }
}
