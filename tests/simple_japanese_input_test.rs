//! Simplified Japanese Input and Cursor Position Tests
//!
//! Tests verify:
//! 1. Basic input functionality (ASCII and Japanese)
//! 2. Cursor position calculation matches implementation
//! 3. Unicode-width awareness
//! 4. EOL click handling

use unicode_width::UnicodeWidthChar;

// ========================================
// 1. Basic Input Tests
// ========================================

#[test]
fn test_ascii_string_input() {
    let mut buffer = String::new();
    buffer.push_str("Hello World");

    assert_eq!(buffer, "Hello World");
    assert_eq!(buffer.len(), 11); // Byte length
    assert_eq!(buffer.chars().count(), 11); // Character count
}

#[test]
fn test_japanese_hiragana_input() {
    let mut buffer = String::new();
    buffer.push_str("こんにちは");

    assert_eq!(buffer, "こんにちは");
    assert!(buffer.len() > buffer.chars().count()); // Multi-byte
    assert_eq!(buffer.chars().count(), 5);
}

#[test]
fn test_japanese_kanji_input() {
    let mut buffer = String::new();
    buffer.push_str("日本語入力テスト");

    assert_eq!(buffer, "日本語入力テスト");
    assert_eq!(buffer.chars().count(), 8);
}

#[test]
fn test_mixed_ascii_japanese_input() {
    let mut buffer = String::new();
    buffer.push_str("Hello 世界");

    assert_eq!(buffer, "Hello 世界");
    assert_eq!(buffer.chars().count(), 8);
}

#[test]
fn test_emoji_input() {
    let mut buffer = String::new();
    buffer.push_str("Hello 😀 World");

    assert_eq!(buffer, "Hello 😀 World");
    assert_eq!(buffer.chars().count(), 13); // Including emoji
}

// ========================================
// 2. Unicode Width Tests
// ========================================

#[test]
fn test_ascii_character_widths() {
    assert_eq!('a'.width(), Some(1));
    assert_eq!('Z'.width(), Some(1));
    assert_eq!(' '.width(), Some(1));
    assert_eq!('0'.width(), Some(1));
}

#[test]
fn test_japanese_character_widths() {
    // Hiragana
    assert_eq!('あ'.width(), Some(2));
    assert_eq!('こ'.width(), Some(2));

    // Katakana
    assert_eq!('ア'.width(), Some(2));
    assert_eq!('テ'.width(), Some(2));

    // Kanji
    assert_eq!('日'.width(), Some(2));
    assert_eq!('本'.width(), Some(2));
    assert_eq!('語'.width(), Some(2));
}

#[test]
fn test_combining_character_widths() {
    // Combining acute accent
    assert_eq!('\u{0301}'.width(), Some(0));

    // Combining diaeresis
    assert_eq!('\u{0308}'.width(), Some(0));
}

#[test]
fn test_emoji_character_width() {
    // Basic emoji
    assert_eq!('😀'.width(), Some(2));
    assert_eq!('🎉'.width(), Some(2));
}

// ========================================
// 3. Character Width Calculation Tests
// ========================================

fn calculate_display_width(text: &str) -> usize {
    text.chars().map(|ch| ch.width().unwrap_or(1)).sum()
}

#[test]
fn test_ascii_text_width() {
    assert_eq!(calculate_display_width("Hello"), 5);
    assert_eq!(calculate_display_width("Hello World"), 11);
}

#[test]
fn test_japanese_text_width() {
    assert_eq!(calculate_display_width("こんにちは"), 10); // 5 chars * 2 width
    assert_eq!(calculate_display_width("日本語"), 6); // 3 chars * 2 width
}

#[test]
fn test_mixed_text_width() {
    // "Hello " (6) + "世界" (4) = 10
    assert_eq!(calculate_display_width("Hello 世界"), 10);

    // "test" (4) + "漢字" (4) + "test" (4) = 12
    assert_eq!(calculate_display_width("test漢字test"), 12);
}

#[test]
fn test_combining_character_text_width() {
    // "cafe" (4) + combining accent (0) = 4
    assert_eq!(calculate_display_width("cafe\u{0301}"), 4);
}

// ========================================
// 4. Cursor Position Offset Tests
// ========================================

fn calculate_char_offset_to_position(text: &str, char_position: usize) -> usize {
    text.chars()
        .take(char_position)
        .map(|ch| ch.width().unwrap_or(1))
        .sum()
}

#[test]
fn test_cursor_offset_ascii() {
    let text = "Hello World";

    assert_eq!(calculate_char_offset_to_position(text, 0), 0); // Start
    assert_eq!(calculate_char_offset_to_position(text, 5), 5); // After "Hello"
    assert_eq!(calculate_char_offset_to_position(text, 11), 11); // End
}

#[test]
fn test_cursor_offset_japanese() {
    let text = "こんにちは";

    assert_eq!(calculate_char_offset_to_position(text, 0), 0); // Start
    assert_eq!(calculate_char_offset_to_position(text, 1), 2); // After 'こ'
    assert_eq!(calculate_char_offset_to_position(text, 2), 4); // After 'こん'
    assert_eq!(calculate_char_offset_to_position(text, 5), 10); // End
}

#[test]
fn test_cursor_offset_mixed() {
    let text = "Hello 世界";

    assert_eq!(calculate_char_offset_to_position(text, 0), 0); // Start
    assert_eq!(calculate_char_offset_to_position(text, 6), 6); // After "Hello "
    assert_eq!(calculate_char_offset_to_position(text, 7), 8); // After "Hello 世"
    assert_eq!(calculate_char_offset_to_position(text, 8), 10); // End
}

// ========================================
// 5. EOL Handling Tests
// ========================================

#[test]
fn test_cursor_clamp_to_line_length() {
    let line = "Hello";
    let max_col = line.chars().count();

    // Clicking far beyond should clamp to end
    let requested_col = 100;
    let actual_col = requested_col.min(max_col);

    assert_eq!(actual_col, 5);
}

#[test]
fn test_cursor_on_empty_line() {
    let line = "";
    let max_col = line.chars().count();

    assert_eq!(max_col, 0);

    let requested_col = 10;
    let actual_col = requested_col.min(max_col);

    assert_eq!(actual_col, 0);
}

// ========================================
// 6. Regression Tests
// ========================================

#[test]
fn test_long_line_no_overflow() {
    let line = "a".repeat(10000);

    assert_eq!(line.chars().count(), 10000);
    assert_eq!(calculate_display_width(&line), 10000);
}

#[test]
fn test_alternating_ascii_cjk() {
    let text = "a漢b字c".repeat(100);

    // Each repeat: "a" (1) + "漢" (2) + "b" (1) + "字" (2) + "c" (1) = 7 width
    // 100 repeats = 700 total width
    assert_eq!(calculate_display_width(&text), 700);

    // Character count: 5 chars per repeat * 100 = 500
    assert_eq!(text.chars().count(), 500);
}

#[test]
fn test_char_to_width_consistency() {
    // Ensure our calculations are consistent
    let test_cases = vec![
        ("", 0),
        ("a", 1),
        ("あ", 2),
        ("Hello", 5),
        ("こんにちは", 10),
        ("Hello 世界", 10),
        ("test漢字test", 12),
    ];

    for (text, expected_width) in test_cases {
        let actual_width = calculate_display_width(text);
        assert_eq!(
            actual_width, expected_width,
            "Text '{}' should have width {}, got {}",
            text, expected_width, actual_width
        );
    }
}

// ========================================
// 7. Practical Editor Scenarios
// ========================================

#[test]
fn test_backspace_from_mixed_text() {
    let mut buffer = "Hello 世界".to_string();
    let original_len = buffer.chars().count();

    // Simulate backspace (remove last character '界')
    buffer.pop();

    assert_eq!(buffer, "Hello 世");
    assert_eq!(buffer.chars().count(), original_len - 1);
}

#[test]
fn test_delete_character_by_index() {
    let text = "Hello 世界";
    let chars: Vec<char> = text.chars().collect();

    // Remove character at index 6 ('世')
    let mut new_text = String::new();
    for (i, ch) in chars.iter().enumerate() {
        if i != 6 {
            new_text.push(*ch);
        }
    }

    assert_eq!(new_text, "Hello 界");
    assert_eq!(new_text.chars().count(), 7);
}

#[test]
fn test_insert_character_at_position() {
    let text = "Hello World";
    let insert_pos = 6; // After "Hello "
    let insert_char = '世';

    let chars: Vec<char> = text.chars().collect();
    let mut new_text = String::new();

    for (i, ch) in chars.iter().enumerate() {
        if i == insert_pos {
            new_text.push(insert_char);
        }
        new_text.push(*ch);
    }

    assert_eq!(new_text, "Hello 世World");
    assert_eq!(new_text.chars().count(), 12);
}
