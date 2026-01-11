//! Multi-Cursor Support
//! 100% Rust - No JavaScript!

use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CursorPosition {
    pub line: usize,
    pub column: usize,
}

impl CursorPosition {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    pub anchor: CursorPosition,
    pub cursor: CursorPosition,
}

impl Selection {
    pub fn new(anchor: CursorPosition, cursor: CursorPosition) -> Self {
        Self { anchor, cursor }
    }

    pub fn collapsed(position: CursorPosition) -> Self {
        Self {
            anchor: position,
            cursor: position,
        }
    }

    pub fn is_collapsed(&self) -> bool {
        self.anchor == self.cursor
    }

    pub fn start(&self) -> CursorPosition {
        if self.anchor.line < self.cursor.line
            || (self.anchor.line == self.cursor.line && self.anchor.column < self.cursor.column)
        {
            self.anchor
        } else {
            self.cursor
        }
    }

    pub fn end(&self) -> CursorPosition {
        if self.anchor.line > self.cursor.line
            || (self.anchor.line == self.cursor.line && self.anchor.column > self.cursor.column)
        {
            self.anchor
        } else {
            self.cursor
        }
    }

    pub fn contains(&self, position: CursorPosition) -> bool {
        let start = self.start();
        let end = self.end();

        if position.line < start.line || position.line > end.line {
            return false;
        }

        if position.line == start.line && position.column < start.column {
            return false;
        }

        if position.line == end.line && position.column > end.column {
            return false;
        }

        true
    }
}

pub struct MultiCursor {
    selections: Vec<Selection>,
    primary_index: usize,
}

impl MultiCursor {
    pub fn new() -> Self {
        Self {
            selections: vec![Selection::collapsed(CursorPosition::new(0, 0))],
            primary_index: 0,
        }
    }

    pub fn from_position(position: CursorPosition) -> Self {
        Self {
            selections: vec![Selection::collapsed(position)],
            primary_index: 0,
        }
    }

    /// Add a new cursor at the given position
    pub fn add_cursor(&mut self, position: CursorPosition) {
        // Check if position already has a cursor
        if !self.has_cursor_at(position) {
            self.selections.push(Selection::collapsed(position));
        }
    }

    /// Remove cursor at position
    pub fn remove_cursor_at(&mut self, position: CursorPosition) {
        self.selections.retain(|s| s.cursor != position);
        // Ensure at least one cursor remains
        if self.selections.is_empty() {
            self.selections.push(Selection::collapsed(position));
        }
        self.primary_index = self.primary_index.min(self.selections.len() - 1);
    }

    /// Check if there's a cursor at the given position
    pub fn has_cursor_at(&self, position: CursorPosition) -> bool {
        self.selections.iter().any(|s| s.cursor == position)
    }

    /// Get all cursor positions
    pub fn get_cursors(&self) -> Vec<CursorPosition> {
        self.selections.iter().map(|s| s.cursor).collect()
    }

    /// Get all selections
    pub fn get_selections(&self) -> &[Selection] {
        &self.selections
    }

    /// Get primary selection
    pub fn primary_selection(&self) -> &Selection {
        &self.selections[self.primary_index]
    }

    /// Clear all cursors except primary
    pub fn clear_secondary_cursors(&mut self) {
        let primary = self.selections[self.primary_index].clone();
        self.selections.clear();
        self.selections.push(primary);
        self.primary_index = 0;
    }

    /// Move all cursors by offset
    pub fn move_all(&mut self, line_offset: i32, column_offset: i32) {
        for selection in &mut self.selections {
            selection.cursor.line = (selection.cursor.line as i32 + line_offset).max(0) as usize;
            selection.cursor.column =
                (selection.cursor.column as i32 + column_offset).max(0) as usize;

            // Move anchor if selection is collapsed
            if selection.is_collapsed() {
                selection.anchor = selection.cursor;
            }
        }
    }

    /// Select word at all cursor positions
    pub fn select_word_at_cursors(&mut self, get_word_bounds: impl Fn(CursorPosition) -> (CursorPosition, CursorPosition)) {
        for selection in &mut self.selections {
            let (start, end) = get_word_bounds(selection.cursor);
            selection.anchor = start;
            selection.cursor = end;
        }
    }

    /// Find and select all occurrences of current selection
    pub fn select_all_occurrences(&mut self, text: &str, current_selection: &str) {
        if current_selection.is_empty() {
            return;
        }

        let mut new_selections = Vec::new();
        let mut current_line = 0;
        let mut current_col = 0;

        for (idx, line) in text.lines().enumerate() {
            let mut search_start = 0;
            while let Some(pos) = line[search_start..].find(current_selection) {
                let actual_pos = search_start + pos;
                new_selections.push(Selection {
                    anchor: CursorPosition::new(idx, actual_pos),
                    cursor: CursorPosition::new(idx, actual_pos + current_selection.len()),
                });
                search_start = actual_pos + 1;
            }
        }

        if !new_selections.is_empty() {
            self.selections = new_selections;
            self.primary_index = 0;
        }
    }

    /// Merge overlapping selections
    pub fn merge_overlapping(&mut self) {
        if self.selections.len() <= 1 {
            return;
        }

        // Sort selections by start position
        self.selections.sort_by(|a, b| {
            let a_start = a.start();
            let b_start = b.start();
            a_start
                .line
                .cmp(&b_start.line)
                .then(a_start.column.cmp(&b_start.column))
        });

        let mut merged = Vec::new();
        let mut current = self.selections[0].clone();

        for selection in self.selections.iter().skip(1) {
            if current.end().line >= selection.start().line
                && (current.end().line > selection.start().line
                    || current.end().column >= selection.start().column)
            {
                // Overlapping - merge
                let new_end = if selection.end().line > current.end().line
                    || (selection.end().line == current.end().line
                        && selection.end().column > current.end().column)
                {
                    selection.end()
                } else {
                    current.end()
                };
                current.cursor = new_end;
            } else {
                // No overlap - save current and start new
                merged.push(current);
                current = selection.clone();
            }
        }
        merged.push(current);

        self.selections = merged;
        self.primary_index = self.primary_index.min(self.selections.len() - 1);
    }

    /// Count total cursors
    pub fn count(&self) -> usize {
        self.selections.len()
    }
}

impl Default for MultiCursor {
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
    fn test_cursor_position_new() {
        let pos = CursorPosition::new(5, 10);
        assert_eq!(pos.line, 5);
        assert_eq!(pos.column, 10);
    }

    #[wasm_bindgen_test]
    fn test_selection_new() {
        let sel = Selection::new(
            CursorPosition::new(1, 5),
            CursorPosition::new(1, 10),
        );
        assert_eq!(sel.anchor, CursorPosition::new(1, 5));
        assert_eq!(sel.cursor, CursorPosition::new(1, 10));
    }

    #[wasm_bindgen_test]
    fn test_selection_collapsed() {
        let pos = CursorPosition::new(1, 5);
        let sel = Selection::collapsed(pos);
        assert!(sel.is_collapsed());
        assert_eq!(sel.anchor, pos);
        assert_eq!(sel.cursor, pos);
    }

    #[wasm_bindgen_test]
    fn test_selection_is_collapsed() {
        let collapsed = Selection::collapsed(CursorPosition::new(1, 5));
        assert!(collapsed.is_collapsed());

        let not_collapsed = Selection::new(
            CursorPosition::new(1, 5),
            CursorPosition::new(1, 10),
        );
        assert!(!not_collapsed.is_collapsed());
    }

    #[wasm_bindgen_test]
    fn test_selection_start_end() {
        let sel = Selection::new(
            CursorPosition::new(1, 10),
            CursorPosition::new(1, 5),
        );
        assert_eq!(sel.start(), CursorPosition::new(1, 5));
        assert_eq!(sel.end(), CursorPosition::new(1, 10));
    }

    #[wasm_bindgen_test]
    fn test_selection_start_end_multiline() {
        let sel = Selection::new(
            CursorPosition::new(2, 0),
            CursorPosition::new(1, 5),
        );
        assert_eq!(sel.start(), CursorPosition::new(1, 5));
        assert_eq!(sel.end(), CursorPosition::new(2, 0));
    }

    #[wasm_bindgen_test]
    fn test_multi_cursor_add() {
        let mut mc = MultiCursor::new();
        mc.add_cursor(CursorPosition::new(1, 5));
        mc.add_cursor(CursorPosition::new(2, 10));
        assert_eq!(mc.count(), 3);
    }

    #[wasm_bindgen_test]
    fn test_multi_cursor_no_duplicate() {
        let mut mc = MultiCursor::new();
        let pos = CursorPosition::new(1, 5);
        mc.add_cursor(pos);
        mc.add_cursor(pos);
        assert_eq!(mc.count(), 2); // Original + one addition
    }

    #[wasm_bindgen_test]
    fn test_selection_contains() {
        let sel = Selection::new(CursorPosition::new(1, 5), CursorPosition::new(1, 10));
        assert!(sel.contains(CursorPosition::new(1, 7)));
        assert!(!sel.contains(CursorPosition::new(1, 3)));
        assert!(!sel.contains(CursorPosition::new(2, 7)));
    }

    #[wasm_bindgen_test]
    fn test_multi_cursor_remove() {
        let mut mc = MultiCursor::new();
        let pos = CursorPosition::new(1, 5);
        mc.add_cursor(pos);
        assert_eq!(mc.count(), 2);

        mc.remove_cursor_at(pos);
        assert_eq!(mc.count(), 1);
    }

    #[wasm_bindgen_test]
    fn test_multi_cursor_always_keeps_one() {
        let mut mc = MultiCursor::new();
        mc.remove_cursor_at(CursorPosition::new(0, 0));
        assert_eq!(mc.count(), 1); // Should always have at least one cursor
    }

    #[wasm_bindgen_test]
    fn test_has_cursor_at() {
        let mut mc = MultiCursor::new();
        let pos = CursorPosition::new(1, 5);
        assert!(!mc.has_cursor_at(pos));

        mc.add_cursor(pos);
        assert!(mc.has_cursor_at(pos));
    }

    #[wasm_bindgen_test]
    fn test_get_cursors() {
        let mut mc = MultiCursor::new();
        mc.add_cursor(CursorPosition::new(1, 5));
        mc.add_cursor(CursorPosition::new(2, 10));

        let cursors = mc.get_cursors();
        assert_eq!(cursors.len(), 3);
    }

    #[wasm_bindgen_test]
    fn test_clear_secondary_cursors() {
        let mut mc = MultiCursor::new();
        mc.add_cursor(CursorPosition::new(1, 5));
        mc.add_cursor(CursorPosition::new(2, 10));
        assert_eq!(mc.count(), 3);

        mc.clear_secondary_cursors();
        assert_eq!(mc.count(), 1);
    }

    #[wasm_bindgen_test]
    fn test_move_all() {
        let mut mc = MultiCursor::new();
        mc.add_cursor(CursorPosition::new(1, 5));

        mc.move_all(1, 2);

        let cursors = mc.get_cursors();
        assert!(cursors.iter().any(|c| c.line == 1 && c.column == 2));
    }

    #[wasm_bindgen_test]
    fn test_move_all_no_negative() {
        let mut mc = MultiCursor::new();
        mc.move_all(-10, -10);

        let cursors = mc.get_cursors();
        assert!(cursors.iter().all(|c| c.line == 0 && c.column == 0));
    }

    #[wasm_bindgen_test]
    fn test_select_all_occurrences() {
        let text = "test test test";
        let mut mc = MultiCursor::new();

        mc.select_all_occurrences(text, "test");

        assert_eq!(mc.count(), 3);
    }

    #[wasm_bindgen_test]
    fn test_select_all_occurrences_empty() {
        let text = "test test test";
        let mut mc = MultiCursor::new();

        mc.select_all_occurrences(text, "");

        assert_eq!(mc.count(), 1); // Should remain unchanged
    }

    #[wasm_bindgen_test]
    fn test_merge_overlapping() {
        let mut mc = MultiCursor::new();
        mc.selections = vec![
            Selection::new(CursorPosition::new(1, 0), CursorPosition::new(1, 5)),
            Selection::new(CursorPosition::new(1, 3), CursorPosition::new(1, 8)),
        ];
        mc.merge_overlapping();
        assert_eq!(mc.count(), 1);
        assert_eq!(mc.selections[0].start(), CursorPosition::new(1, 0));
        assert_eq!(mc.selections[0].end(), CursorPosition::new(1, 8));
    }

    #[wasm_bindgen_test]
    fn test_merge_non_overlapping() {
        let mut mc = MultiCursor::new();
        mc.selections = vec![
            Selection::new(CursorPosition::new(1, 0), CursorPosition::new(1, 5)),
            Selection::new(CursorPosition::new(2, 0), CursorPosition::new(2, 5)),
        ];
        mc.merge_overlapping();
        assert_eq!(mc.count(), 2); // Should remain separate
    }

    #[wasm_bindgen_test]
    fn test_from_position() {
        let pos = CursorPosition::new(5, 10);
        let mc = MultiCursor::from_position(pos);

        assert_eq!(mc.count(), 1);
        assert_eq!(mc.primary_selection().cursor, pos);
    }

    #[wasm_bindgen_test]
    fn test_primary_selection() {
        let mc = MultiCursor::new();
        let primary = mc.primary_selection();

        assert!(primary.is_collapsed());
        assert_eq!(primary.cursor, CursorPosition::new(0, 0));
    }
}
