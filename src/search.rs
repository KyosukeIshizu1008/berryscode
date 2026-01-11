//! Search and Replace Functionality
//! 100% Rust - No JavaScript!

use regex::Regex;
use std::ops::Range;

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
        let query = if self.options.case_sensitive {
            self.query.clone()
        } else {
            self.query.to_lowercase()
        };

        for (line_idx, line) in lines.iter().enumerate().skip(start_line).take(end_line - start_line) {
            let search_text = if self.options.case_sensitive {
                line.to_string()
            } else {
                line.to_lowercase()
            };

            let mut start = 0;
            while let Some(pos) = search_text[start..].find(&query) {
                let actual_pos = start + pos;
                let end_pos = actual_pos + query.len();

                // Check whole word boundary
                if self.options.whole_word {
                    let before_ok = actual_pos == 0 || !line.chars().nth(actual_pos - 1).unwrap().is_alphanumeric();
                    let after_ok = end_pos >= line.len() || !line.chars().nth(end_pos).unwrap().is_alphanumeric();

                    if !before_ok || !after_ok {
                        start = actual_pos + 1;
                        continue;
                    }
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

        for (line_idx, line) in lines.iter().enumerate().skip(start_line).take(end_line - start_line) {
            for cap in regex.find_iter(line) {
                let start_pos = cap.start();
                let end_pos = cap.end();

                // Check whole word boundary if needed
                if self.options.whole_word {
                    let before_ok = start_pos == 0 || !line.chars().nth(start_pos - 1).unwrap().is_alphanumeric();
                    let after_ok = end_pos >= line.len() || !line.chars().nth(end_pos).unwrap().is_alphanumeric();

                    if !before_ok || !after_ok {
                        continue;
                    }
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
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_literal_search() {
        let mut engine = SearchEngine::new();
        engine.set_query("test".to_string());

        let text = "This is a test\nAnother test line\nNo match here";
        let matches = engine.search(text);

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].line, 0);
        assert_eq!(matches[1].line, 1);
    }

    #[wasm_bindgen_test]
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

    #[wasm_bindgen_test]
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

    #[wasm_bindgen_test]
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

    #[wasm_bindgen_test]
    fn test_replace_current() {
        let mut engine = SearchEngine::new();
        engine.set_query("old".to_string());

        let mut text = "This is old text with old values".to_string();
        engine.search(&text);

        let replaced = engine.replace_current(&mut text, "new");
        assert!(replaced);
        assert!(text.contains("new"));
    }

    #[wasm_bindgen_test]
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

    #[wasm_bindgen_test]
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
