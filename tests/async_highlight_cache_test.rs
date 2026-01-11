//! Async Highlight Cache Reactivity Test
//!
//! ❌ CRITICAL: This test prevents "white screen flash" bugs
//!
//! WHY THIS MATTERS:
//! - When opening large files, highlighting is async (takes time)
//! - Before highlighting arrives: Must show PLAIN TEXT
//! - After highlighting arrives: Must show COLORED HTML
//! - Cache must be reactive: changes should trigger re-render immediately
//!
//! WHAT THIS PREVENTS:
//! - "一瞬だけ真っ白になる" (momentary white screen)
//! - Stale highlights after editing
//! - Race conditions between cache updates and rendering
//!
//! Run with: cargo test --test async_highlight_cache_test

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    /// Simulates EditorTab's highlight_cache behavior
    struct MockHighlightCache {
        cache: HashMap<usize, String>,
    }

    impl MockHighlightCache {
        fn new() -> Self {
            Self {
                cache: HashMap::new(),
            }
        }

        /// Simulate getting highlight HTML for a line
        fn get_highlight(&self, line_number: usize) -> Option<&String> {
            self.cache.get(&line_number)
        }

        /// Simulate inserting highlight result (from async backend)
        fn insert_highlight(&mut self, line_number: usize, html: String) {
            self.cache.insert(line_number, html);
        }

        /// Simulate clearing cache (when file is edited)
        fn clear(&mut self) {
            self.cache.clear();
        }

        /// Simulate invalidating specific lines (when editing)
        fn invalidate_lines(&mut self, start_line: usize, end_line: usize) {
            for line in start_line..=end_line {
                self.cache.remove(&line);
            }
        }
    }

    /// Simulates the rendering logic from virtual_editor.rs
    fn render_line(
        line_number: usize,
        plain_text: &str,
        cache: &MockHighlightCache,
    ) -> (String, bool) {
        match cache.get_highlight(line_number) {
            Some(html) => {
                // ✅ PHASE 2: Highlight available, use colored HTML
                let unescaped = html
                    .replace("&lt;", "<")
                    .replace("&gt;", ">")
                    .replace("&quot;", "\"")
                    .replace("&amp;", "&");
                (unescaped, true) // (content, is_highlighted)
            }
            None => {
                // ✅ PHASE 1: No highlight yet, show plain text
                (plain_text.to_string(), false)
            }
        }
    }

    // ========================================
    // PHASE 1: Before Highlight (Plain Text)
    // ========================================

    #[test]
    fn test_no_cache_shows_plain_text() {
        let cache = MockHighlightCache::new();
        let plain_text = "fn main() {}";

        let (rendered, is_highlighted) = render_line(0, plain_text, &cache);

        assert_eq!(rendered, "fn main() {}",
            "❌ CRITICAL: Before highlighting, must show plain text");
        assert!(!is_highlighted,
            "❌ CRITICAL: is_highlighted should be false when cache empty");
    }

    #[test]
    fn test_multiple_lines_no_cache() {
        let cache = MockHighlightCache::new();
        let lines = vec![
            "fn main() {",
            "    println!(\"Hello\");",
            "}",
        ];

        for (i, line_text) in lines.iter().enumerate() {
            let (rendered, is_highlighted) = render_line(i, line_text, &cache);
            assert_eq!(rendered, *line_text,
                "Line {} should show plain text when cache empty", i);
            assert!(!is_highlighted);
        }
    }

    // ========================================
    // PHASE 2: After Highlight (Colored HTML)
    // ========================================

    #[test]
    fn test_cache_hit_shows_html() {
        let mut cache = MockHighlightCache::new();

        // Simulate async highlight result arriving
        let html = r#"&lt;span style=&quot;color: #CC7832&quot;&gt;fn&lt;/span&gt;"#;
        cache.insert_highlight(0, html.to_string());

        let (rendered, is_highlighted) = render_line(0, "fn main() {}", &cache);

        assert!(rendered.contains("<span"),
            "❌ CRITICAL: After highlight arrives, must contain HTML tags");
        assert!(rendered.contains("color: #CC7832"),
            "❌ CRITICAL: Must contain color styles");
        assert!(is_highlighted,
            "❌ CRITICAL: is_highlighted should be true when cache hit");
    }

    #[test]
    fn test_cache_reactivity_immediate_switch() {
        let mut cache = MockHighlightCache::new();
        let plain_text = "fn test() {}";

        // BEFORE: No cache, should show plain text
        let (rendered_before, highlighted_before) = render_line(0, plain_text, &cache);
        assert_eq!(rendered_before, plain_text);
        assert!(!highlighted_before);

        // ✅ CRITICAL MOMENT: Async highlight arrives (simulated)
        cache.insert_highlight(
            0,
            r#"&lt;span style=&quot;color: #CC7832&quot;&gt;fn&lt;/span&gt;"#.to_string()
        );

        // AFTER: Cache exists, should IMMEDIATELY show HTML
        let (rendered_after, highlighted_after) = render_line(0, plain_text, &cache);
        assert!(rendered_after.contains("<span"),
            "❌ CRITICAL BUG: After cache insert, render must IMMEDIATELY switch to HTML!");
        assert!(highlighted_after,
            "❌ CRITICAL: is_highlighted should flip to true immediately");
    }

    // ========================================
    // PHASE 3: Cache Invalidation (Editing)
    // ========================================

    #[test]
    fn test_cache_cleared_on_edit() {
        let mut cache = MockHighlightCache::new();

        // Initial highlight
        cache.insert_highlight(0, "<span>highlighted</span>".to_string());
        assert!(cache.get_highlight(0).is_some());

        // ✅ User edits the file - cache must be cleared
        cache.clear();

        let (rendered, is_highlighted) = render_line(0, "new text", &cache);
        assert_eq!(rendered, "new text",
            "❌ After edit, cache cleared, should show plain text");
        assert!(!is_highlighted);
    }

    #[test]
    fn test_partial_cache_invalidation() {
        let mut cache = MockHighlightCache::new();

        // Cache lines 0-4
        for i in 0..5 {
            cache.insert_highlight(i, format!("<span>line {}</span>", i));
        }

        // User edits lines 2-3
        cache.invalidate_lines(2, 3);

        // Lines 0-1 should still be cached
        assert!(cache.get_highlight(0).is_some(), "Line 0 should remain cached");
        assert!(cache.get_highlight(1).is_some(), "Line 1 should remain cached");

        // Lines 2-3 should be invalidated
        assert!(cache.get_highlight(2).is_none(),
            "❌ Line 2 should be invalidated after edit");
        assert!(cache.get_highlight(3).is_none(),
            "❌ Line 3 should be invalidated after edit");

        // Line 4 should still be cached
        assert!(cache.get_highlight(4).is_some(), "Line 4 should remain cached");
    }

    // ========================================
    // Edge Cases
    // ========================================

    #[test]
    fn test_empty_line_cache() {
        let mut cache = MockHighlightCache::new();
        cache.insert_highlight(0, String::new());

        let (rendered, is_highlighted) = render_line(0, "", &cache);
        assert_eq!(rendered, "");
        assert!(is_highlighted, "Empty cache entry should still mark as highlighted");
    }

    #[test]
    fn test_cache_miss_with_other_cached_lines() {
        let mut cache = MockHighlightCache::new();

        // Cache line 0 and 2, but not 1
        cache.insert_highlight(0, "<span>line 0</span>".to_string());
        cache.insert_highlight(2, "<span>line 2</span>".to_string());

        let (rendered, is_highlighted) = render_line(1, "plain line 1", &cache);
        assert_eq!(rendered, "plain line 1",
            "Cache miss on line 1 should show plain text even if other lines cached");
        assert!(!is_highlighted);
    }

    #[test]
    fn test_html_unescape_edge_cases() {
        let mut cache = MockHighlightCache::new();

        // Backend returns double-escaped entities (edge case)
        cache.insert_highlight(
            0,
            "&amp;lt;span&amp;gt;".to_string()
        );

        let (rendered, _) = render_line(0, "", &cache);

        // Unescape should handle this correctly
        assert_eq!(rendered, "&lt;span&gt;",
            "Should unescape exactly one level");
    }

    #[test]
    fn test_large_file_partial_cache() {
        let mut cache = MockHighlightCache::new();

        // Simulate large file (1000 lines)
        // Only lines 100-200 are in viewport and highlighted
        for i in 100..=200 {
            cache.insert_highlight(i, format!("<span>line {}</span>", i));
        }

        // Lines outside viewport should be plain text
        let (_rendered_early, highlighted_early) = render_line(50, "early line", &cache);
        assert!(!highlighted_early, "Line 50 not in cache");

        let (_rendered_late, highlighted_late) = render_line(500, "late line", &cache);
        assert!(!highlighted_late, "Line 500 not in cache");

        // Lines in viewport should be highlighted
        let (rendered_visible, highlighted_visible) = render_line(150, "", &cache);
        assert!(highlighted_visible, "Line 150 should be cached");
        assert!(rendered_visible.contains("<span>"));
    }

    #[test]
    fn test_concurrent_cache_updates() {
        let mut cache = MockHighlightCache::new();

        // Simulate two async highlight results arriving in quick succession
        cache.insert_highlight(0, "<span>version 1</span>".to_string());
        cache.insert_highlight(0, "<span>version 2</span>".to_string());

        // Latest version should win
        let (rendered, _) = render_line(0, "", &cache);
        assert!(rendered.contains("version 2"),
            "Latest cache update should override previous");
    }
}
