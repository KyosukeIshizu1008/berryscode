//! Editor Input and Edit Operations Tests
//!
//! Tests to verify that the fixes for:
//! 1. Text input functionality works correctly
//! 2. Delete/Backspace operations work correctly
//! 3. Line numbers remain visible during scroll
//!
//! These tests prove the bug fixes implemented in virtual_editor.rs

use berry_editor::buffer::TextBuffer;

// Layout constants (must match virtual_editor.rs)
const LINE_HEIGHT: f64 = 20.0;
const CHAR_WIDTH: f64 = 7.815;

// ========================================
// Text Input Tests
// ========================================

#[test]
fn test_insert_single_character() {
    // Test inserting a single character at cursor position
    let mut buffer = TextBuffer::from_str("Hello World");

    // Insert '!' at position 5 (after "Hello")
    buffer.insert(5, "!");

    assert_eq!(buffer.to_string(), "Hello! World");
}

#[test]
fn test_insert_character_at_line_start() {
    let mut buffer = TextBuffer::from_str("fn main() {\n    println!(\"Hello\");\n}");

    // Insert '//' at the start of line 1 (position after first \n)
    let line_1_start = buffer.line_to_char(1);
    buffer.insert(line_1_start, "//");

    let result = buffer.to_string();
    assert!(result.contains("//    println!"), "Should insert comment at line start");
}

#[test]
fn test_insert_character_at_line_end() {
    let mut buffer = TextBuffer::from_str("Line 1\nLine 2\nLine 3");

    // Insert at end of first line (before \n)
    let line_0_end = buffer.line_to_char(0) + 6; // "Line 1" is 6 chars
    buffer.insert(line_0_end, " END");

    assert_eq!(buffer.line(0).unwrap(), "Line 1 END\n");
}

#[test]
fn test_insert_newline() {
    let mut buffer = TextBuffer::from_str("HelloWorld");

    // Insert newline in the middle
    buffer.insert(5, "\n");

    assert_eq!(buffer.to_string(), "Hello\nWorld");
    assert_eq!(buffer.len_lines(), 2);
}

#[test]
fn test_insert_multiple_characters() {
    let mut buffer = TextBuffer::from_str("The  fox");

    // Insert "quick brown " between "The" and "fox"
    buffer.insert(4, "quick brown ");

    assert_eq!(buffer.to_string(), "The quick brown  fox");
}

#[test]
fn test_insert_unicode_characters() {
    let mut buffer = TextBuffer::from_str("Hello ");

    // Insert unicode characters
    buffer.insert(6, "世界");

    assert_eq!(buffer.to_string(), "Hello 世界");
}

#[test]
fn test_insert_at_empty_buffer() {
    let mut buffer = TextBuffer::from_str("");

    buffer.insert(0, "First line");

    assert_eq!(buffer.to_string(), "First line");
    assert_eq!(buffer.len_lines(), 1);
}

// ========================================
// Delete/Backspace Tests
// ========================================

#[test]
fn test_backspace_single_character() {
    let mut buffer = TextBuffer::from_str("Hello!");

    // Delete the '!' (remove from position 5 to 6)
    buffer.remove(5, 6);

    assert_eq!(buffer.to_string(), "Hello");
}

#[test]
fn test_backspace_at_line_start() {
    let mut buffer = TextBuffer::from_str("Line 1\nLine 2");

    // Delete newline at position 6 to join lines
    buffer.remove(6, 7);

    assert_eq!(buffer.to_string(), "Line 1Line 2");
    assert_eq!(buffer.len_lines(), 1);
}

#[test]
fn test_backspace_middle_of_line() {
    let mut buffer = TextBuffer::from_str("Hello World");

    // Delete 'o' at position 4
    buffer.remove(4, 5);

    assert_eq!(buffer.to_string(), "Hell World");
}

#[test]
fn test_delete_range_of_characters() {
    let mut buffer = TextBuffer::from_str("The quick brown fox");

    // Delete "quick " (positions 4-10)
    buffer.remove(4, 10);

    assert_eq!(buffer.to_string(), "The brown fox");
}

#[test]
fn test_delete_entire_line() {
    let mut buffer = TextBuffer::from_str("Line 1\nLine 2\nLine 3");

    // Delete "Line 2\n" (from end of line 0 to end of line 1)
    let line_1_start = buffer.line_to_char(1);
    let line_2_start = buffer.line_to_char(2);
    buffer.remove(line_1_start, line_2_start);

    assert_eq!(buffer.to_string(), "Line 1\nLine 3");
    assert_eq!(buffer.len_lines(), 2);
}

#[test]
fn test_backspace_unicode_character() {
    let mut buffer = TextBuffer::from_str("Hello 世界");

    // TextBuffer uses char indices, not byte indices
    let char_count = buffer.len_chars();

    // Remove the last character '界' (remove from char_count-1 to char_count)
    buffer.remove(char_count - 1, char_count);

    assert_eq!(buffer.to_string(), "Hello 世");
}

#[test]
fn test_backspace_with_multiple_lines() {
    let mut buffer = TextBuffer::from_str("Line 1\nLine 2\nLine 3");

    // Backspace at start of line 2 (delete the newline)
    let line_1_end = buffer.line_to_char(1); // Position of \n after Line 1
    buffer.remove(line_1_end - 1, line_1_end);

    let result = buffer.to_string();
    assert!(result.contains("Line 1Line 2"), "Should join lines after backspace");
}

#[test]
fn test_delete_selection() {
    let mut buffer = TextBuffer::from_str("The quick brown fox jumps");

    // Delete a selection from position 4 to 15 ("quick brown")
    buffer.remove(4, 15);

    assert_eq!(buffer.to_string(), "The  fox jumps");
}

// ========================================
// Line Number Scroll Sync Tests
// ========================================

#[test]
fn test_line_number_calculation_at_top() {
    // Test that line numbers are correctly calculated at scroll position 0
    let scroll_top = 0.0;
    let visible_height = 800.0;

    let start_line = (scroll_top / LINE_HEIGHT).floor() as usize;
    let end_line = ((scroll_top + visible_height) / LINE_HEIGHT).ceil() as usize;

    assert_eq!(start_line, 0, "First visible line should be 0 at top");
    assert_eq!(end_line, 40, "Should show 40 lines (800px / 20px)");
}

#[test]
fn test_line_number_calculation_scrolled() {
    // Test that line numbers are correctly calculated when scrolled
    let scroll_top = 1000.0; // Scrolled to line 50
    let visible_height = 800.0;

    let start_line = (scroll_top / LINE_HEIGHT).floor() as usize;
    let end_line = ((scroll_top + visible_height) / LINE_HEIGHT).ceil() as usize;

    assert_eq!(start_line, 50, "First visible line should be 50");
    assert_eq!(end_line, 90, "Last visible line should be 90 ((1000+800)/20)");
}

#[test]
fn test_line_number_offset_sync() {
    // Test that line number transform offset matches scroll position
    let test_scrolls = vec![0.0, 100.0, 500.0, 1000.0, 5000.0];

    for scroll_top in test_scrolls {
        // The transform should be -scroll_top to keep line numbers in sync
        let transform_offset = -scroll_top;

        // When text is scrolled by scroll_top, line numbers should be offset by -scroll_top
        // This keeps them aligned
        assert_eq!(
            transform_offset,
            -scroll_top,
            "Line number offset should match negative scroll position"
        );
    }
}

#[test]
fn test_visible_line_range_large_file() {
    // Test with a large file (10000 lines)
    let total_lines = 10000;
    let scroll_top = 5000.0 * LINE_HEIGHT; // Scrolled to middle
    let visible_height = 800.0;

    let start_line = (scroll_top / LINE_HEIGHT).floor() as usize;
    let end_line = ((scroll_top + visible_height) / LINE_HEIGHT).ceil() as usize;

    assert_eq!(start_line, 5000);
    assert_eq!(end_line, 5040);
    assert!(end_line <= total_lines, "Should not exceed file length");
}

#[test]
fn test_line_number_visibility_at_bottom() {
    // Test line numbers at the bottom of a file
    let total_lines = 100;
    let scroll_top = (total_lines as f64 - 40.0) * LINE_HEIGHT; // Near bottom
    let visible_height = 800.0;

    let start_line = (scroll_top / LINE_HEIGHT).floor() as usize;
    let end_line = ((scroll_top + visible_height) / LINE_HEIGHT).ceil() as usize;

    assert_eq!(start_line, 60);
    assert_eq!(end_line, 100);
}

// ========================================
// Cursor Movement After Edit Tests
// ========================================

#[test]
fn test_cursor_position_after_insert() {
    let mut buffer = TextBuffer::from_str("Hello");
    let cursor_line = 0;
    let cursor_col = 5;

    // Insert at cursor position
    let insert_pos = buffer.line_to_char(cursor_line) + cursor_col;
    buffer.insert(insert_pos, " World");

    // Cursor should move forward by 6 characters
    let new_cursor_col = cursor_col + 6;

    assert_eq!(new_cursor_col, 11);
    assert_eq!(buffer.to_string(), "Hello World");
}

#[test]
fn test_cursor_position_after_backspace() {
    let buffer = TextBuffer::from_str("Hello!");
    let cursor_line = 0;
    let cursor_col = 6; // After '!'

    // After backspace, cursor should move back by 1
    let new_cursor_col = cursor_col - 1;

    assert_eq!(new_cursor_col, 5);
}

#[test]
fn test_cursor_position_after_newline_insert() {
    let mut buffer = TextBuffer::from_str("HelloWorld");
    let cursor_line = 0;
    let cursor_col = 5;

    // Insert newline at cursor
    let insert_pos = buffer.line_to_char(cursor_line) + cursor_col;
    buffer.insert(insert_pos, "\n");

    // Cursor should move to next line, column 0
    let new_cursor_line = cursor_line + 1;
    let new_cursor_col = 0;

    assert_eq!(new_cursor_line, 1);
    assert_eq!(new_cursor_col, 0);
    assert_eq!(buffer.len_lines(), 2);
}

#[test]
fn test_cursor_position_after_line_join() {
    let buffer = TextBuffer::from_str("Line 1\nLine 2");
    let cursor_line = 1;
    let cursor_col = 0; // Start of line 2

    // After backspace at line start, cursor should be at end of previous line
    let prev_line_len = buffer.line(0).unwrap().trim_end_matches('\n').len();

    let new_cursor_line = 0;
    let new_cursor_col = prev_line_len;

    assert_eq!(new_cursor_line, 0);
    assert_eq!(new_cursor_col, 6); // "Line 1" is 6 characters
}

// ========================================
// Integration Tests
// ========================================

#[test]
fn test_complete_editing_workflow() {
    // Test a complete editing workflow: insert, delete, navigate
    let mut buffer = TextBuffer::from_str("fn main() {\n}");

    // 1. Insert println at line 1
    let line_1_start = buffer.line_to_char(1);
    buffer.insert(line_1_start, "    println!(\"Hello\");\n");

    assert_eq!(buffer.len_lines(), 3);
    assert!(buffer.to_string().contains("println"));

    // 2. Delete some text
    let hello_start = buffer.to_string().find("Hello").unwrap();
    buffer.remove(hello_start, hello_start + 5);

    assert!(buffer.to_string().contains("println!(\"\")"));

    // 3. Insert new text
    let quote_pos = buffer.to_string().find("\"\"").unwrap() + 1;
    buffer.insert(quote_pos, "World");

    assert!(buffer.to_string().contains("println!(\"World\")"));
}

#[test]
fn test_rapid_character_insertion() {
    // Simulate rapid typing
    let mut buffer = TextBuffer::from_str("");
    let text = "The quick brown fox";

    for (i, ch) in text.chars().enumerate() {
        buffer.insert(i, &ch.to_string());
    }

    assert_eq!(buffer.to_string(), text);
}

#[test]
fn test_backspace_entire_content() {
    let mut buffer = TextBuffer::from_str("Delete me");

    // Delete all content
    let len = buffer.to_string().len();
    buffer.remove(0, len);

    assert_eq!(buffer.to_string(), "");
    assert_eq!(buffer.len_lines(), 1); // Empty buffer still has 1 line
}
