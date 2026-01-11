//! Japanese Input and Cursor Position Accuracy Tests
//!
//! Tests for:
//! 1. Keyboard input (ASCII and Japanese)
//! 2. Accurate cursor positioning with Unicode-width awareness
//! 3. Click position calculation for mixed ASCII/CJK text
//! 4. EOL (end-of-line) click handling

use unicode_width::UnicodeWidthChar;

// Layout constants (must match virtual_editor.rs and index.html CSS)
const LINE_HEIGHT: f64 = 20.0;
const CHAR_WIDTH: f64 = 7.8125; // JetBrains Mono 13px の正確な値（CSS変数と同期）
const GUTTER_WIDTH: f64 = 55.0;
const PADDING: f64 = 15.0;

/// Calculate Unicode-width aware cursor position from click coordinates
/// This matches the implementation in virtual_editor.rs exactly
fn calculate_cursor_position_unicode(
    rel_x: f64,  // Already relative to editor pane (not including gutter in some cases)
    rel_y: f64,  // Already includes scroll offset
    line_count: usize,
    lines: &[String],
) -> (usize, usize) {
    // 1. Calculate line index (rel_y already includes scroll offset)
    let clicked_line = (rel_y / LINE_HEIGHT).floor() as usize;
    let clamped_line = clicked_line.min(line_count.saturating_sub(1));

    // 2. Unicode-width aware column calculation
    let line_str = lines.get(clamped_line).map(|s| s.as_str()).unwrap_or("");
    let mut current_x = GUTTER_WIDTH + PADDING;
    let mut target_char_idx = 0;

    for (i, ch) in line_str.chars().enumerate() {
        // Unicode width: 2 for wide chars (CJK), 1 for ASCII, 0 for combining
        let ch_width = ch.width().unwrap_or(1) as f64;
        let next_x = current_x + (ch_width * CHAR_WIDTH);

        // Click position is closer to current character
        if rel_x < (current_x + next_x) / 2.0 {
            target_char_idx = i;
            break;
        }
        current_x = next_x;
        target_char_idx = i + 1;
    }

    let clamped_col = target_char_idx.min(line_str.chars().count());

    (clamped_line, clamped_col)
}

/// Calculate cursor X position using calc() method (matches CSS rendering)
fn calculate_cursor_x_position(line: &str, cursor_col: usize) -> f64 {
    let mut x_offset = 0.0;

    for ch in line.chars().take(cursor_col) {
        // ✅ Use unicode-width to handle combining characters correctly
        // Combining chars have width 0, CJK have width 2, ASCII have width 1
        let ch_width = ch.width().unwrap_or(1) as f64;
        x_offset += ch_width;
    }

    // calc(55px + 15px + x_offset * var(--char-width))
    GUTTER_WIDTH + PADDING + (x_offset * CHAR_WIDTH)
}

// ========================================
// 1. ASCII Input Tests
// ========================================

#[test]
fn test_ascii_input_basic() {
    // Simulate typing "Hello"
    let mut buffer = String::new();
    let input_chars = vec!['H', 'e', 'l', 'l', 'o'];

    for ch in input_chars {
        buffer.push(ch);
    }

    assert_eq!(buffer, "Hello");
    assert_eq!(buffer.chars().count(), 5);
}

#[test]
fn test_ascii_cursor_position() {
    let line = "Hello World";

    // Cursor at position 0 (before 'H')
    let x = calculate_cursor_x_position(line, 0);
    assert_eq!(x, GUTTER_WIDTH + PADDING);

    // Cursor at position 6 (after space, before 'W')
    let x = calculate_cursor_x_position(line, 6);
    assert_eq!(x, GUTTER_WIDTH + PADDING + (6.0 * CHAR_WIDTH));

    // Cursor at end (after 'd')
    let x = calculate_cursor_x_position(line, 11);
    assert_eq!(x, GUTTER_WIDTH + PADDING + (11.0 * CHAR_WIDTH));
}

// ========================================
// 2. Japanese Input Tests
// ========================================

#[test]
fn test_japanese_hiragana_input() {
    // Simulate typing Japanese hiragana
    let mut buffer = String::new();
    buffer.push_str("こんにちは");

    assert_eq!(buffer, "こんにちは");
    assert_eq!(buffer.chars().count(), 5); // 5 characters
}

#[test]
fn test_japanese_kanji_input() {
    // Simulate typing Japanese kanji
    let mut buffer = String::new();
    buffer.push_str("日本語入力");

    assert_eq!(buffer, "日本語入力");
    assert_eq!(buffer.chars().count(), 5); // 5 characters
}

#[test]
fn test_mixed_ascii_japanese_input() {
    // Simulate typing mixed text
    let mut buffer = String::new();
    buffer.push_str("Hello 世界");

    assert_eq!(buffer, "Hello 世界");
    assert_eq!(buffer.chars().count(), 8); // 'H','e','l','l','o',' ','世','界'
}

// ========================================
// 3. Unicode-Width Cursor Position Tests
// ========================================

#[test]
fn test_japanese_cursor_position_start() {
    let line = "こんにちは";

    // Cursor at position 0 (before first hiragana)
    let x = calculate_cursor_x_position(line, 0);
    assert_eq!(x, GUTTER_WIDTH + PADDING);
}

#[test]
fn test_japanese_cursor_position_middle() {
    let line = "こんにちは";

    // Cursor at position 2 (after 'こん', before 'に')
    // Each hiragana = 2 char widths
    let x = calculate_cursor_x_position(line, 2);
    let expected = GUTTER_WIDTH + PADDING + (4.0 * CHAR_WIDTH); // 2 chars * 2 width each
    assert_eq!(x, expected);
}

#[test]
fn test_japanese_cursor_position_end() {
    let line = "こんにちは";

    // Cursor at end (after 'は')
    // 5 hiragana characters * 2 width each = 10
    let x = calculate_cursor_x_position(line, 5);
    let expected = GUTTER_WIDTH + PADDING + (10.0 * CHAR_WIDTH);
    assert_eq!(x, expected);
}

#[test]
fn test_mixed_text_cursor_position() {
    let line = "Hello 世界";

    // 'H','e','l','l','o',' ' = 6 ASCII chars (width 1 each) = 6
    // '世','界' = 2 kanji chars (width 2 each) = 4
    // Total offset = 6 + 4 = 10

    // Cursor after "Hello 世界"
    let x = calculate_cursor_x_position(line, 8);
    let expected = GUTTER_WIDTH + PADDING + (10.0 * CHAR_WIDTH);
    assert_eq!(x, expected);
}

#[test]
fn test_mixed_text_cursor_between_ascii_and_kanji() {
    let line = "test漢字test";

    // Cursor after "test" (4 ASCII chars)
    let x = calculate_cursor_x_position(line, 4);
    let expected = GUTTER_WIDTH + PADDING + (4.0 * CHAR_WIDTH);
    assert_eq!(x, expected);

    // Cursor after "test漢字" (4 ASCII + 2 kanji = 4 + 4 = 8)
    let x = calculate_cursor_x_position(line, 6);
    let expected = GUTTER_WIDTH + PADDING + (8.0 * CHAR_WIDTH);
    assert_eq!(x, expected);
}

// ========================================
// 4. Click Position Calculation Tests
// ========================================

#[test]
fn test_click_on_ascii_character() {
    let lines = vec!["Hello World".to_string()];

    // Click on 'W' (position 6)
    // "Hello " = 6 chars, click in left half of 'W' to get position 6
    let click_x = GUTTER_WIDTH + PADDING + (6.3 * CHAR_WIDTH);
    let rel_y = 0.0; // First line
    let (line, col) = calculate_cursor_position_unicode(click_x, rel_y, 1, &lines);

    assert_eq!(line, 0);
    assert_eq!(col, 6);
}

#[test]
fn test_click_on_japanese_character() {
    let lines = vec!["こんにちは".to_string()];

    // Click on 'に' (position 2)
    // 'こん' = 2 chars * 2 width = 4 char widths
    // Click in left half of 'に' to get position 2
    let click_x = GUTTER_WIDTH + PADDING + (4.5 * CHAR_WIDTH);
    let rel_y = 0.0;
    let (line, col) = calculate_cursor_position_unicode(click_x, rel_y, 1, &lines);

    assert_eq!(line, 0);
    assert_eq!(col, 2); // Character index 2 ('に')
}

#[test]
fn test_click_on_mixed_text() {
    let lines = vec!["Hello 世界".to_string()];

    // Click on '世' (position 6)
    // "Hello " = 6 ASCII chars = 6 char widths
    // Click in left half of '世' to get position 6
    let click_x = GUTTER_WIDTH + PADDING + (6.5 * CHAR_WIDTH);
    let rel_y = 0.0;
    let (line, col) = calculate_cursor_position_unicode(click_x, rel_y, 1, &lines);

    assert_eq!(line, 0);
    assert_eq!(col, 6); // Character index 6 ('世')
}

#[test]
fn test_click_between_ascii_characters() {
    let lines = vec!["Hello".to_string()];

    // Click between 'e' and 'l' (should round to position 2)
    let click_x = GUTTER_WIDTH + PADDING + (1.5 * CHAR_WIDTH);
    let rel_y = 0.0;
    let (line, col) = calculate_cursor_position_unicode(click_x, rel_y, 1, &lines);

    assert_eq!(line, 0);
    assert_eq!(col, 2); // Rounds to position 2
}

#[test]
fn test_click_between_japanese_characters() {
    let lines = vec!["こんにちは".to_string()];

    // Click between 'ん' and 'に'
    // 'こん' = 2 chars * 2 width = 4 char widths
    // Click at 4.0 widths (right at the boundary)
    // With < comparison, clicking at boundary advances to next char
    let click_x = GUTTER_WIDTH + PADDING + (4.0 * CHAR_WIDTH);
    let rel_y = 0.0;
    let (line, col) = calculate_cursor_position_unicode(click_x, rel_y, 1, &lines);

    assert_eq!(line, 0);
    assert_eq!(col, 2); // Position 2 (before 'に')
}

// ========================================
// 5. EOL (End-of-Line) Click Handling Tests
// ========================================

#[test]
fn test_click_after_last_ascii_character() {
    let lines = vec!["Hello".to_string()];

    // Click far right (beyond "Hello")
    let click_x = GUTTER_WIDTH + PADDING + (100.0 * CHAR_WIDTH);
    let rel_y = 0.0;
    let (line, col) = calculate_cursor_position_unicode(click_x, rel_y, 1, &lines);

    assert_eq!(line, 0);
    assert_eq!(col, 5); // End of "Hello"
}

#[test]
fn test_click_after_last_japanese_character() {
    let lines = vec!["こんにちは".to_string()];

    // Click far right (beyond all hiragana)
    let click_x = GUTTER_WIDTH + PADDING + (100.0 * CHAR_WIDTH);
    let rel_y = 0.0;
    let (line, col) = calculate_cursor_position_unicode(click_x, rel_y, 1, &lines);

    assert_eq!(line, 0);
    assert_eq!(col, 5); // End of line (5 characters)
}

#[test]
fn test_click_on_empty_line() {
    let lines = vec!["".to_string()];

    // Click anywhere on empty line
    let click_x = GUTTER_WIDTH + PADDING + (10.0 * CHAR_WIDTH);
    let rel_y = 0.0;
    let (line, col) = calculate_cursor_position_unicode(click_x, rel_y, 1, &lines);

    assert_eq!(line, 0);
    assert_eq!(col, 0); // Cursor at start of empty line
}

#[test]
fn test_click_after_mixed_text_eol() {
    let lines = vec!["test漢字test".to_string()];

    // Click beyond the last character
    // "test漢字test" = 4 + 4 + 4 = 12 char widths
    let click_x = GUTTER_WIDTH + PADDING + (20.0 * CHAR_WIDTH);
    let rel_y = 0.0;
    let (line, col) = calculate_cursor_position_unicode(click_x, rel_y, 1, &lines);

    assert_eq!(line, 0);
    assert_eq!(col, 10); // 10 characters total
}

// ========================================
// 6. Multi-line Tests with Scrolling
// ========================================

#[test]
fn test_click_on_line_with_scroll() {
    let lines = vec![
        "Line 0".to_string(),
        "Line 1".to_string(),
        "Line 2".to_string(),
        "Line 3".to_string(),
    ];

    // Scroll down 2 lines (40px = 2 * LINE_HEIGHT)
    // Click at y=0 on screen, which is actually line 2 due to scroll
    let scroll_top = 2.0 * LINE_HEIGHT;
    let rel_y = 0.0 + scroll_top; // Add scroll offset
    let click_x = GUTTER_WIDTH + PADDING;

    let (line, col) = calculate_cursor_position_unicode(click_x, rel_y, 4, &lines);

    assert_eq!(line, 2);
    assert_eq!(col, 0);
}

#[test]
fn test_click_on_japanese_line_with_scroll() {
    let lines = vec![
        "こんにちは".to_string(),
        "日本語".to_string(),
        "入力テスト".to_string(),
    ];

    // Scroll down 1 line
    let scroll_top = LINE_HEIGHT;

    // Click on '本' in "日本語" (character index 1)
    // '日' = 2 char widths, click in left half of '本' to get position 1
    let click_x = GUTTER_WIDTH + PADDING + (2.5 * CHAR_WIDTH);
    let rel_y = 0.0 + scroll_top; // Add scroll offset to get line 1

    let (line, col) = calculate_cursor_position_unicode(click_x, rel_y, 3, &lines);

    assert_eq!(line, 1); // Line 1: "日本語"
    assert_eq!(col, 1);  // Character index 1 ('本')
}

// ========================================
// 7. Edge Cases
// ========================================

#[test]
fn test_click_with_zero_width_combining_characters() {
    // Combining diacritics have width 0
    let lines = vec!["cafe\u{0301}".to_string()]; // café with combining acute

    // The combining character should not add to the width
    // "cafe" = 4 chars (width 1 each) = 4
    // "\u{0301}" = combining acute (width 0) = 0
    // Cursor at position 5 (after all characters including combining)
    let x = calculate_cursor_x_position(&lines[0], 5);
    let expected = GUTTER_WIDTH + PADDING + (4.0 * CHAR_WIDTH); // 4 char widths total

    // Allow small floating point tolerance
    assert!((x - expected).abs() < 0.01, "Expected {}, got {}", expected, x);
}

#[test]
fn test_very_long_line_cursor_position() {
    let line = "a".repeat(1000);

    // Cursor at position 500
    let x = calculate_cursor_x_position(&line, 500);
    let expected = GUTTER_WIDTH + PADDING + (500.0 * CHAR_WIDTH);
    assert_eq!(x, expected);
}

#[test]
fn test_unicode_emoji_width() {
    let line = "Hello 😀 World";

    // Emoji typically have width 2
    // "Hello " = 6, emoji = 2, " World" = 6
    // Cursor after emoji
    let x = calculate_cursor_x_position(line, 7); // After "Hello 😀"

    // Width calculation: 6 ASCII + 1 emoji (width 2) = 8
    let expected = GUTTER_WIDTH + PADDING + (8.0 * CHAR_WIDTH);
    assert_eq!(x, expected);
}

// ========================================
// 8. Regression Tests
// ========================================

#[test]
fn test_no_accumulation_error_long_line() {
    // Test that calc() method prevents accumulation errors
    let line = "abcdefghijklmnopqrstuvwxyz0123456789".repeat(10);

    // Calculate cursor position at end
    let char_count = line.chars().count();
    let x = calculate_cursor_x_position(&line, char_count);

    // All ASCII, so total width = char_count
    let expected = GUTTER_WIDTH + PADDING + (char_count as f64 * CHAR_WIDTH);

    // Should be exact with no floating point drift
    assert_eq!(x, expected);
}

#[test]
fn test_no_accumulation_error_mixed_long_line() {
    // Test with alternating ASCII and kanji
    let line = "a漢b字c".repeat(100);

    // "a漢b字c" = 1 + 2 + 1 + 2 + 1 = 7 char widths per repeat
    // 100 repeats = 700 char widths
    let char_count = line.chars().count(); // 500 characters
    let x = calculate_cursor_x_position(&line, char_count);

    let expected = GUTTER_WIDTH + PADDING + (700.0 * CHAR_WIDTH);
    assert_eq!(x, expected);
}
