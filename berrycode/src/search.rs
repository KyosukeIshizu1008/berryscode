//! Search and Replace Functionality
//! 100% Rust - No JavaScript!

use regex::Regex;
use std::ops::Range;

/// True if `line[start..end]` is a whole-word match — i.e. the chars
/// immediately before `start` and immediately after `end` are not
/// alphanumeric (or don't exist because we're at a string edge).
///
/// Both `start` and `end` MUST be valid UTF-8 byte boundaries in
/// `line`; the function uses slice-based char peeking
/// (`line[..start].chars().next_back()` and
/// `line[end..].chars().next()`) so it never falls into the
/// `chars().nth(byte_offset)` trap that the previous inline checks
/// did. Multi-byte characters at the boundary are handled correctly:
/// a query like `é` against the line `éa` returns the boundary char
/// as the next char (not the panic from over-indexing).
fn is_whole_word(line: &str, start: usize, end: usize) -> bool {
    let before_ok = start == 0
        || line[..start]
            .chars()
            .next_back()
            .map(|c| !c.is_alphanumeric())
            .unwrap_or(true);
    let after_ok = end >= line.len()
        || line[end..]
            .chars()
            .next()
            .map(|c| !c.is_alphanumeric())
            .unwrap_or(true);
    before_ok && after_ok
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchMatch {
    pub line: usize,
    pub start_col: usize,
    pub end_col: usize,
    pub text: String,
}

impl SearchMatch {
    pub fn new(line: usize, start_col: usize, end_col: usize, text: String) -> Self {
        Self {
            line,
            start_col,
            end_col,
            text,
        }
    }

    pub fn range(&self) -> Range<usize> {
        self.start_col..self.end_col
    }
}

#[derive(Debug, Clone)]
pub struct SearchOptions {
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub use_regex: bool,
    pub in_selection: bool,
    pub selection_start_line: Option<usize>,
    pub selection_end_line: Option<usize>,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            case_sensitive: false,
            whole_word: false,
            use_regex: false,
            in_selection: false,
            selection_start_line: None,
            selection_end_line: None,
        }
    }
}

pub struct SearchEngine {
    query: String,
    options: SearchOptions,
    matches: Vec<SearchMatch>,
    current_match_index: Option<usize>,
}

impl SearchEngine {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            options: SearchOptions::default(),
            matches: Vec::new(),
            current_match_index: None,
        }
    }

    pub fn set_query(&mut self, query: String) {
        self.query = query;
    }

    pub fn set_options(&mut self, options: SearchOptions) {
        self.options = options;
    }

    pub fn search(&mut self, text: &str) -> Vec<SearchMatch> {
        self.matches.clear();
        self.current_match_index = None;

        if self.query.is_empty() {
            return self.matches.clone();
        }

        let lines: Vec<&str> = text.lines().collect();
        let start_line = if self.options.in_selection {
            self.options.selection_start_line.unwrap_or(0)
        } else {
            0
        };
        let end_line = if self.options.in_selection {
            self.options.selection_end_line.unwrap_or(lines.len())
        } else {
            lines.len()
        };

        if self.options.use_regex {
            self.search_regex(&lines, start_line, end_line);
        } else {
            self.search_literal(&lines, start_line, end_line);
        }

        if !self.matches.is_empty() {
            self.current_match_index = Some(0);
        }

        self.matches.clone()
    }

    fn search_literal(&mut self, lines: &[&str], start_line: usize, end_line: usize) {
        // Case-insensitive matching can't reuse byte offsets from a
        // lowered string against the original line: Unicode case
        // folding is allowed to change byte length (e.g. Turkish
        // capital `İ` lowercases to two-codepoint `i̇`), so a query
        // like `i` against `İ` would produce an `end_pos` that's not
        // on a UTF-8 boundary in the original. Slicing
        // `line[actual_pos..end_pos]` then panics.
        //
        // The regex crate's case-insensitive mode does the right
        // thing — it returns byte offsets in the *original* string
        // even when the case-folded representation is a different
        // length. We escape the literal first so meta characters
        // don't accidentally turn the query into a real pattern.
        if self.options.case_sensitive {
            self.search_literal_case_sensitive(lines, start_line, end_line);
        } else {
            self.search_literal_case_insensitive(lines, start_line, end_line);
        }
    }

    fn search_literal_case_sensitive(
        &mut self,
        lines: &[&str],
        start_line: usize,
        end_line: usize,
    ) {
        let query = &self.query;
        for (line_idx, line) in lines
            .iter()
            .enumerate()
            .skip(start_line)
            .take(end_line - start_line)
        {
            let mut start = 0;
            while let Some(pos) = line[start..].find(query) {
                let actual_pos = start + pos;
                let end_pos = actual_pos + query.len();

                if self.options.whole_word && !is_whole_word(line, actual_pos, end_pos) {
                    start = actual_pos + 1;
                    continue;
                }

                self.matches.push(SearchMatch::new(
                    line_idx,
                    actual_pos,
                    end_pos,
                    line[actual_pos..end_pos].to_string(),
                ));

                start = actual_pos + 1;
            }
        }
    }

    fn search_literal_case_insensitive(
        &mut self,
        lines: &[&str],
        start_line: usize,
        end_line: usize,
    ) {
        // Build a case-insensitive regex from the literal query. The
        // `regex::escape` call neutralises any meta characters in the
        // user's input so a query like `a.b` matches the literal
        // string `a.b`, not "a, any char, b". The (?i) flag tells the
        // regex engine to do Unicode-aware case folding internally
        // while still reporting byte offsets in the original line —
        // which is the property that the previous lower-and-slice
        // approach didn't have.
        let pattern = format!("(?i){}", regex::escape(&self.query));
        let regex = match Regex::new(&pattern) {
            Ok(r) => r,
            Err(_) => return,
        };
        for (line_idx, line) in lines
            .iter()
            .enumerate()
            .skip(start_line)
            .take(end_line - start_line)
        {
            for m in regex.find_iter(line) {
                let actual_pos = m.start();
                let end_pos = m.end();

                if self.options.whole_word && !is_whole_word(line, actual_pos, end_pos) {
                    continue;
                }

                self.matches.push(SearchMatch::new(
                    line_idx,
                    actual_pos,
                    end_pos,
                    line[actual_pos..end_pos].to_string(),
                ));
            }
        }
    }

    fn search_regex(&mut self, lines: &[&str], start_line: usize, end_line: usize) {
        let pattern = if self.options.case_sensitive {
            self.query.clone()
        } else {
            format!("(?i){}", self.query)
        };

        let regex = match Regex::new(&pattern) {
            Ok(r) => r,
            Err(_) => return, // Invalid regex
        };

        for (line_idx, line) in lines
            .iter()
            .enumerate()
            .skip(start_line)
            .take(end_line - start_line)
        {
            for cap in regex.find_iter(line) {
                let start_pos = cap.start();
                let end_pos = cap.end();

                // Same Unicode-safety story as the literal-case-sensitive
                // path — `chars().nth(byte_offset)` was the previous shape
                // and panicked on multi-byte chars. `is_whole_word` peels a
                // single char off either side via slice-based iteration,
                // which is sound on every byte boundary the regex returns.
                if self.options.whole_word && !is_whole_word(line, start_pos, end_pos) {
                    continue;
                }

                self.matches.push(SearchMatch::new(
                    line_idx,
                    start_pos,
                    end_pos,
                    line[start_pos..end_pos].to_string(),
                ));
            }
        }
    }

    pub fn find_next(&mut self) -> Option<SearchMatch> {
        if self.matches.is_empty() {
            return None;
        }

        let current = self.current_match_index?;
        let next = (current + 1) % self.matches.len();
        self.current_match_index = Some(next);
        Some(self.matches[next].clone())
    }

    pub fn find_previous(&mut self) -> Option<SearchMatch> {
        if self.matches.is_empty() {
            return None;
        }

        let current = self.current_match_index?;
        let prev = if current == 0 {
            self.matches.len() - 1
        } else {
            current - 1
        };
        self.current_match_index = Some(prev);
        Some(self.matches[prev].clone())
    }

    pub fn get_current_match(&self) -> Option<SearchMatch> {
        let idx = self.current_match_index?;
        self.matches.get(idx).cloned()
    }

    pub fn get_matches(&self) -> &[SearchMatch] {
        &self.matches
    }

    pub fn match_count(&self) -> usize {
        self.matches.len()
    }

    pub fn current_match_index(&self) -> Option<usize> {
        self.current_match_index
    }

    /// Replace current match
    pub fn replace_current(&mut self, text: &mut String, replacement: &str) -> bool {
        let current_match = match self.get_current_match() {
            Some(m) => m,
            None => return false,
        };

        let lines: Vec<&str> = text.lines().collect();
        if current_match.line >= lines.len() {
            return false;
        }

        let mut result_lines = Vec::new();
        for (idx, line) in lines.iter().enumerate() {
            if idx == current_match.line {
                let mut new_line = line.to_string();
                new_line.replace_range(current_match.range(), replacement);
                result_lines.push(new_line);
            } else {
                result_lines.push(line.to_string());
            }
        }

        *text = result_lines.join("\n");

        // Re-search after replacement
        self.search(text);
        true
    }

    /// Replace all matches
    pub fn replace_all(&mut self, text: &mut String, replacement: &str) -> usize {
        let matches = self.matches.clone();
        if matches.is_empty() {
            return 0;
        }

        let lines: Vec<String> = text.lines().map(|s| s.to_string()).collect();
        let mut result_lines = lines.clone();

        // Process in reverse order to maintain correct indices
        for search_match in matches.iter().rev() {
            if search_match.line < result_lines.len() {
                let line = &mut result_lines[search_match.line];
                line.replace_range(search_match.range(), replacement);
            }
        }

        let count = matches.len();
        *text = result_lines.join("\n");

        // Re-search after replacement
        self.search(text);
        count
    }
}

impl Default for SearchEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_literal_search() {
        let mut engine = SearchEngine::new();
        engine.set_query("test".to_string());

        let text = "This is a test\nAnother test line\nNo match here";
        let matches = engine.search(text);

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].line, 0);
        assert_eq!(matches[1].line, 1);
    }

    #[test]
    fn test_case_insensitive() {
        let mut engine = SearchEngine::new();
        engine.set_query("TEST".to_string());
        let mut options = SearchOptions::default();
        options.case_sensitive = false;
        engine.set_options(options);

        let text = "This is a test\nAnother Test line";
        let matches = engine.search(text);

        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn test_whole_word() {
        let mut engine = SearchEngine::new();
        engine.set_query("test".to_string());
        let mut options = SearchOptions::default();
        options.whole_word = true;
        engine.set_options(options);

        let text = "test testing tested test";
        let matches = engine.search(text);

        assert_eq!(matches.len(), 2); // Only standalone "test"
    }

    #[test]
    fn case_insensitive_search_handles_byte_length_changing_fold() {
        // Regression: Turkish capital `İ` (U+0130) lowercases to two
        // codepoints — `i` (U+0069) + COMBINING DOT ABOVE (U+0307) —
        // i.e. the lowered form is *one byte longer* than the
        // uppercase original. The previous implementation lowered the
        // line, found the query in the lowered text, and used those
        // byte offsets to slice the original line. That sliced into
        // the middle of a multi-byte character and panicked. The
        // regex-based path returns offsets in the original string, so
        // it's panic-free.
        let mut engine = SearchEngine::new();
        engine.set_query("i".to_string());
        let mut options = SearchOptions::default();
        options.case_sensitive = false;
        engine.set_options(options);

        // The raw bytes for `İ` are 0xC4 0xB0 (2 bytes); lowercased
        // to `i\u{307}` they're 0x69 0xCC 0x87 (3 bytes). Using byte
        // offsets from the lowered version against the original would
        // run off the end of the original line.
        let text = "İA";
        let matches = engine.search(text);

        // The load-bearing assertion is "search() returned without
        // panicking" — we already got that just by reaching this
        // line. The regex engine's case-fold table may or may not
        // consider `İ` a fold of `i` (`İ` folds to two codepoints,
        // not one), so the match count is implementation-defined and
        // not part of the regression contract.
        //
        // For any match the engine *does* return, the stored text
        // must be a real slice of the original line — if the regex
        // returned a byte offset that didn't sit on a UTF-8 boundary,
        // the slice would have panicked or this contains() check
        // would fail.
        for m in &matches {
            assert!(
                text.contains(&m.text),
                "match text {:?} not in line",
                m.text
            );
        }
    }

    #[test]
    fn case_insensitive_search_finds_unicode_match() {
        // Sanity: a query whose case fold *is* a single codepoint
        // round-trips correctly. `É` (U+00C9) folds to `é` (U+00E9),
        // both single codepoints, so a case-insensitive search for
        // `é` should find the `É` in the line. This is the
        // non-pathological case the regex path handles alongside the
        // pathological `İ` case above.
        let mut engine = SearchEngine::new();
        engine.set_query("é".to_string());
        let mut options = SearchOptions::default();
        options.case_sensitive = false;
        engine.set_options(options);

        let text = "Étude";
        let matches = engine.search(text);

        assert_eq!(matches.len(), 1);
        // The stored text comes from the original line, so it's the
        // uppercase `É` rather than the folded form.
        assert_eq!(matches[0].text, "É");
    }

    #[test]
    fn case_sensitive_search_handles_multibyte_query() {
        // Multi-byte queries like `é` (2 bytes) against multi-byte
        // lines exercise the same UTF-8 boundary code path under the
        // case-sensitive route.
        let mut engine = SearchEngine::new();
        engine.set_query("é".to_string());

        let text = "éa éb éc";
        let matches = engine.search(text);

        assert_eq!(matches.len(), 3);
        for m in &matches {
            assert_eq!(m.text, "é");
        }
    }

    #[test]
    fn whole_word_check_is_panic_free_at_multibyte_boundaries() {
        // The whole-word boundary check used to call
        // `line.chars().nth(byte_offset).unwrap()`, which panics when
        // byte_offset overshoots the char count (any line with
        // non-ASCII chars before the match). The slice-based version
        // peels a single char off either side and never panics.
        let mut engine = SearchEngine::new();
        engine.set_query("test".to_string());
        let mut options = SearchOptions::default();
        options.whole_word = true;
        engine.set_options(options);

        let text = "日本語の test を検索";
        let matches = engine.search(text);

        // We just need this to not panic. The expected match count is
        // 1 (the `test` between the Japanese phrase and the particle),
        // but the panic-free property is the load-bearing assertion.
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].text, "test");
    }

    #[test]
    fn test_regex_search() {
        let mut engine = SearchEngine::new();
        engine.set_query(r"\d+".to_string());
        let mut options = SearchOptions::default();
        options.use_regex = true;
        engine.set_options(options);

        let text = "Line 42 has 100 numbers";
        let matches = engine.search(text);

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].text, "42");
        assert_eq!(matches[1].text, "100");
    }

    #[test]
    fn test_replace_current() {
        let mut engine = SearchEngine::new();
        engine.set_query("old".to_string());

        let mut text = "This is old text with old values".to_string();
        engine.search(&text);

        let replaced = engine.replace_current(&mut text, "new");
        assert!(replaced);
        assert!(text.contains("new"));
    }

    #[test]
    fn test_replace_all() {
        let mut engine = SearchEngine::new();
        engine.set_query("old".to_string());

        let mut text = "This is old text with old values".to_string();
        engine.search(&text);

        let count = engine.replace_all(&mut text, "new");
        assert_eq!(count, 2);
        assert!(!text.contains("old"));
        assert_eq!(text.matches("new").count(), 2);
    }

    #[test]
    fn test_find_next_previous() {
        let mut engine = SearchEngine::new();
        engine.set_query("test".to_string());

        let text = "test1 test2 test3";
        engine.search(text);

        let first = engine.get_current_match();
        assert_eq!(first.unwrap().start_col, 0);

        let second = engine.find_next();
        assert_eq!(second.unwrap().start_col, 6);

        let first_again = engine.find_previous();
        assert_eq!(first_again.unwrap().start_col, 0);
    }
}
