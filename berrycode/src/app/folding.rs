//! Code folding support
//! Tracks folded regions and transforms text for display

use super::BerryCodeApp;

impl BerryCodeApp {
    /// Toggle fold at the given line (fold/unfold the block starting at this line)
    pub(crate) fn toggle_fold_at_line(&mut self, line: usize) {
        let tab = match self.editor_tabs.get_mut(self.active_tab_idx) {
            Some(t) => t,
            None => return,
        };

        // Check if this line is already folded
        if let Some(idx) = tab.folded_regions.iter().position(|(start, _)| *start == line) {
            tab.folded_regions.remove(idx);
            tab.text_cache_version = 0; // invalidate cache
            return;
        }

        // Find the foldable block starting at this line
        let text = tab.buffer.to_string();
        if let Some(end_line) = find_fold_end(&text, line) {
            tab.folded_regions.push((line, end_line));
            tab.text_cache_version = 0;
        }
    }
}

/// Find the end line of a foldable block starting at `start_line`.
/// Uses brace matching: finds the matching `}` for a `{` on the start line.
pub fn find_fold_end(text: &str, start_line: usize) -> Option<usize> {
    let lines: Vec<&str> = text.lines().collect();
    if start_line >= lines.len() {
        return None;
    }

    let start = lines[start_line];
    // Must contain an opening brace
    if !start.contains('{') {
        return None;
    }

    let mut depth = 0;
    for (idx, line) in lines.iter().enumerate().skip(start_line) {
        for ch in line.chars() {
            if ch == '{' {
                depth += 1;
            }
            if ch == '}' {
                depth -= 1;
            }
        }
        if depth == 0 && idx > start_line {
            return Some(idx + 1); // exclusive end
        }
    }
    None
}

/// Apply folding: transform source text by replacing folded regions with placeholders.
#[allow(dead_code)]
/// Returns (folded_text, line_mapping) where line_mapping maps folded line indices to original line indices.
pub fn apply_folding(text: &str, folded_regions: &[(usize, usize)]) -> (String, Vec<usize>) {
    if folded_regions.is_empty() {
        let mapping: Vec<usize> = (0..text.lines().count()).collect();
        return (text.to_string(), mapping);
    }

    let lines: Vec<&str> = text.lines().collect();
    let mut result = String::new();
    let mut mapping = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        if let Some((_, end)) = folded_regions.iter().find(|(start, _)| *start == i) {
            // Write the first line of the fold, then a placeholder
            result.push_str(lines[i]);
            result.push_str("  // ... ");
            let folded_count = end - i - 1;
            result.push_str(&format!("({} lines)", folded_count));
            result.push('\n');
            mapping.push(i);
            i = *end; // skip to end of fold
        } else {
            result.push_str(lines[i]);
            result.push('\n');
            mapping.push(i);
            i += 1;
        }
    }

    // Remove trailing newline if original didn't have one
    if !text.ends_with('\n') && result.ends_with('\n') {
        result.pop();
    }

    (result, mapping)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_fold_end_simple() {
        let text = "fn main() {\n    println!(\"hello\");\n}\n";
        assert_eq!(find_fold_end(text, 0), Some(3));
    }

    #[test]
    fn test_find_fold_end_nested() {
        let text = "fn main() {\n    if true {\n        x();\n    }\n}\n";
        assert_eq!(find_fold_end(text, 0), Some(5));
        assert_eq!(find_fold_end(text, 1), Some(4));
    }

    #[test]
    fn test_find_fold_end_no_brace() {
        let text = "let x = 5;\nlet y = 10;\n";
        assert_eq!(find_fold_end(text, 0), None);
    }

    #[test]
    fn test_apply_folding_empty() {
        let text = "line1\nline2\nline3";
        let (result, mapping) = apply_folding(text, &[]);
        assert_eq!(result, text);
        assert_eq!(mapping, vec![0, 1, 2]);
    }

    #[test]
    fn test_apply_folding_with_fold() {
        let text = "fn main() {\n    line1;\n    line2;\n}\nafter";
        let (result, mapping) = apply_folding(text, &[(0, 4)]);
        assert!(result.contains("(3 lines)"));
        assert_eq!(mapping.len(), 2); // folded line + "after"
        assert_eq!(mapping[0], 0);
        assert_eq!(mapping[1], 4);
    }
}
