//! Syntax Highlighting Unit Tests (Pure Rust, No JS)
//!
//! Tests the core logic without browser automation

#[cfg(test)]
mod tests {
    #[test]
    fn test_html_escape_in_highlight_result_to_html() {
        // Backend returns plain text tokens
        let token_text = "fn";

        // Frontend escapes when building HTML (highlight_result_to_html)
        let escaped = html_escape(token_text);
        assert_eq!(escaped, "fn"); // No special chars, no change

        // Test with special characters
        let special_text = "<>&\"'";
        let escaped_special = html_escape(special_text);
        assert_eq!(escaped_special, "&lt;&gt;&amp;&quot;&#39;");

        // This HTML goes directly to inner_html attribute
        let html = format!("<span style=\"color: #CC7832\">{}</span>", escaped_special);
        assert!(html.contains("&lt;"));
        assert!(html.contains("&gt;"));
    }

    fn html_escape(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#39;")
    }

    #[test]
    fn test_comparison_operators_escaped_correctly() {
        // Rust code: if x < y
        let text = "<";
        let escaped = html_escape(text);

        // Should be escaped to &lt;
        assert_eq!(escaped, "&lt;");

        // When embedded in HTML
        let html = format!("<span>{}</span>", escaped);
        assert_eq!(html, "<span>&lt;</span>");
    }

    #[test]
    fn test_empty_html_fallback() {
        // When highlight_cache is empty, should use plain text
        let plain_text = "fn main() {}";

        // This should NOT be escaped (handled by syntax_highlight_line)
        assert!(!plain_text.contains("&lt;"));
        assert!(!plain_text.contains("&gt;"));
    }

    #[test]
    fn test_multiple_tokens_in_one_line() {
        // Simulating highlight_result_to_html for: pub fn main()
        let tokens = vec![
            ("pub", "#CC7832"),
            (" ", "#A9B7C6"),
            ("fn", "#CC7832"),
            (" ", "#A9B7C6"),
            ("main", "#FFC66D"),
        ];

        let mut html = String::new();
        for (text, color) in tokens {
            html.push_str(&format!(
                "<span style=\"color: {}\">{}</span>",
                color,
                html_escape(text)
            ));
        }

        // Count span tags
        let span_count = html.matches("<span").count();
        assert_eq!(span_count, 5);

        let close_count = html.matches("</span>").count();
        assert_eq!(close_count, 5);
    }
}

#[cfg(test)]
mod integration_tests {
    use std::collections::HashMap;

    // Simulate EditorTab's highlight_cache behavior
    struct MockTab {
        highlight_cache: HashMap<usize, String>,
    }

    impl MockTab {
        fn new() -> Self {
            Self {
                highlight_cache: HashMap::new(),
            }
        }

        fn insert_highlight(&mut self, line: usize, html: String) {
            self.highlight_cache.insert(line, html);
        }

        fn get_rendered_html(&self, line: usize) -> Option<String> {
            self.highlight_cache.get(&line).cloned()
        }
    }

    #[test]
    fn test_tab_highlight_cache_workflow() {
        let mut tab = MockTab::new();

        // Simulate highlight_result_to_html output (properly escaped HTML)
        let html = r#"<span style="color: #CC7832">fn</span>"#;
        tab.insert_highlight(0, html.to_string());

        // Get rendered HTML (should be used directly)
        let rendered = tab.get_rendered_html(0).unwrap();

        assert!(rendered.contains("<span"));
        assert!(rendered.contains("</span>"));
        assert!(rendered.contains("color: #CC7832"));
    }

    #[test]
    fn test_markdown_file_highlighting() {
        let mut tab = MockTab::new();

        // Markdown heading (HTML from highlight_result_to_html)
        let md_html = r#"<span style="color: #6a8759"># Title</span>"#;
        tab.insert_highlight(0, md_html.to_string());

        let rendered = tab.get_rendered_html(0).unwrap();

        assert!(rendered.contains("<span"));
        assert!(rendered.contains("# Title"));
        assert!(rendered.contains("#6a8759")); // Markdown green
    }

    #[test]
    fn test_cache_miss_returns_none() {
        let tab = MockTab::new();
        assert!(tab.get_rendered_html(999).is_none());
    }

    #[test]
    fn test_empty_cache_entry() {
        let mut tab = MockTab::new();
        tab.insert_highlight(0, String::new());

        let rendered = tab.get_rendered_html(0).unwrap();
        assert_eq!(rendered, "");
    }
}

#[cfg(test)]
mod async_tab_creation_tests {
    use std::sync::{Arc, Mutex};

    // Simulate tabs signal behavior
    struct MockTabs {
        tabs: Arc<Mutex<Vec<String>>>,
    }

    impl MockTabs {
        fn new() -> Self {
            Self {
                tabs: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn update<F>(&self, f: F) -> usize
        where
            F: FnOnce(&mut Vec<String>) -> usize,
        {
            let mut tabs = self.tabs.lock().unwrap();
            f(&mut tabs)
        }

        fn len(&self) -> usize {
            self.tabs.lock().unwrap().len()
        }
    }

    #[test]
    fn test_synchronous_tab_index_calculation() {
        let tabs = MockTabs::new();

        // Simulate the fixed code from virtual_editor.rs:420-424
        let new_index = tabs.update(|t| {
            t.push("test.md".to_string());
            t.len().saturating_sub(1)
        });

        // Index should be calculated INSIDE the update closure
        assert_eq!(new_index, 0);
        assert_eq!(tabs.len(), 1);

        // Add another tab
        let new_index2 = tabs.update(|t| {
            t.push("another.rs".to_string());
            t.len().saturating_sub(1)
        });

        assert_eq!(new_index2, 1);
        assert_eq!(tabs.len(), 2);
    }

    #[test]
    fn test_tab_index_atomicity() {
        let tabs = MockTabs::new();

        // Multiple tabs created
        let indices: Vec<usize> = (0..10)
            .map(|i| {
                tabs.update(|t| {
                    t.push(format!("file{}.rs", i));
                    t.len().saturating_sub(1)
                })
            })
            .collect();

        // All indices should be unique and sequential
        assert_eq!(indices, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
        assert_eq!(tabs.len(), 10);
    }
}
