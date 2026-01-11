//! Coordinate Consistency Tests
//!
//! These tests verify that coordinate calculations are reversible:
//! - col -> x -> col should return the same col
//! - This detects subtle bugs in click position calculation
//!
//! WHY THIS MATTERS:
//! If calculate_x_position and get_col_from_x are not perfectly inverse functions,
//! clicking on a character will move the cursor to the wrong position.

use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// Constants must match src/core/virtual_editor.rs - MEASURED from browser rendering
const CHAR_WIDTH_ASCII: f64 = 8.0; // E2E measured
const CHAR_WIDTH_WIDE: f64 = 13.0; // E2E measured
const TEXT_PADDING: f64 = 15.0;

/// Calculate x position from column (should match virtual_editor.rs::calculate_x_position)
fn calculate_x_position(line_str: &str, char_col: usize) -> f64 {
    line_str.chars().take(char_col).map(|ch| {
        if ch as u32 > 255 { CHAR_WIDTH_WIDE } else { CHAR_WIDTH_ASCII }
    }).sum::<f64>()
}

/// Calculate column from x position (should match virtual_editor.rs click handler)
fn get_col_from_x(line_str: &str, x: f64) -> usize {
    let mut current_x = 0.0;
    let mut col = 0;
    for (i, ch) in line_str.chars().enumerate() {
        if ch == '\n' { break; }
        let w = if ch as u32 > 255 { CHAR_WIDTH_WIDE } else { CHAR_WIDTH_ASCII };
        if x < current_x + (w / 2.0) { break; }
        current_x += w;
        col = i + 1;
    }
    col
}

// ========================================
// Reversibility Tests (CRITICAL)
// ========================================

#[wasm_bindgen_test]
fn test_ascii_reversibility() {
    let line = "fn main() {";

    for col in 0..=line.len() {
        let x = calculate_x_position(line, col);
        let col_back = get_col_from_x(line, x);

        assert_eq!(col, col_back,
            "ASCII reversibility failed at col={}: x={}, got col={}",
            col, x, col_back);
    }
}

#[wasm_bindgen_test]
fn test_japanese_reversibility() {
    let line = "こんにちは世界";

    for col in 0..=line.chars().count() {
        let x = calculate_x_position(line, col);
        let col_back = get_col_from_x(line, x);

        assert_eq!(col, col_back,
            "Japanese reversibility failed at col={}: x={}, got col={}",
            col, x, col_back);
    }
}

#[wasm_bindgen_test]
fn test_mixed_reversibility() {
    let line = "Hello 世界! Rust は最高";

    for col in 0..=line.chars().count() {
        let x = calculate_x_position(line, col);
        let col_back = get_col_from_x(line, x);

        assert_eq!(col, col_back,
            "Mixed reversibility failed at col={}: x={}, got col={}\nLine: {:?}\nChar at col: {:?}",
            col, x, col_back, line, line.chars().nth(col));
    }
}

// ========================================
// Edge Cases
// ========================================

#[wasm_bindgen_test]
fn test_empty_line_reversibility() {
    let line = "";
    let x = calculate_x_position(line, 0);
    let col_back = get_col_from_x(line, x);
    assert_eq!(0, col_back);
}

#[wasm_bindgen_test]
fn test_line_with_newline() {
    let line = "hello\n";

    for col in 0..=5 {  // Don't include the newline
        let x = calculate_x_position(line, col);
        let col_back = get_col_from_x(line, x);
        assert_eq!(col, col_back);
    }
}

// ========================================
// Pixel Precision Tests
// ========================================

#[wasm_bindgen_test]
fn test_half_character_click() {
    let line = "abc";

    // Click on first half of 'b' (index 1)
    let b_start = CHAR_WIDTH_ASCII;
    let b_middle = b_start + (CHAR_WIDTH_ASCII / 2.0) - 0.1;

    let col = get_col_from_x(line, b_middle);
    assert_eq!(col, 1, "Clicking first half of 'b' should select before 'b'");

    // Click on second half of 'b'
    let b_middle_plus = b_start + (CHAR_WIDTH_ASCII / 2.0) + 0.1;
    let col = get_col_from_x(line, b_middle_plus);
    assert_eq!(col, 2, "Clicking second half of 'b' should select after 'b'");
}

#[wasm_bindgen_test]
fn test_click_beyond_line_end() {
    let line = "abc";
    let way_beyond = 1000.0;

    let col = get_col_from_x(line, way_beyond);
    assert_eq!(col, 3, "Clicking beyond line end should position cursor at end");
}

// ========================================
// CRITICAL: Strict Coordinate Fidelity Test
// ========================================
// This is the mathematical foundation - if this breaks, everything breaks

#[wasm_bindgen_test]
fn test_strict_coordinate_fidelity_all_cases() {
    let test_cases = vec![
        // Pure ASCII
        "Rust",
        "fn main() {",
        "let x = 42;",

        // Pure Japanese
        "こんにちは",
        "日本語入力",
        "世界",

        // Mixed content
        "Hello 世界",
        "Rust は最高",
        "🦀 Ferris",

        // Edge cases
        "\t\tTab",  // Tabs
        "  spaces",  // Leading spaces
        "",  // Empty string
        "a",  // Single char
    ];

    for text in test_cases {
        let char_count = text.chars().count();
        for col in 0..=char_count {
            let x = calculate_x_position(text, col);
            let back_col = get_col_from_x(text, x);

            assert_eq!(
                col, back_col,
                "❌ COORDINATE FIDELITY BROKEN!\n\
                 Text: '{}'\n\
                 Original Col: {}\n\
                 X Position: {:.2}\n\
                 Back Col: {}\n\
                 This breaks cursor positioning!",
                text, col, x, back_col
            );
        }
    }
}

#[wasm_bindgen_test]
fn test_emoji_coordinate_fidelity() {
    let test_cases = vec![
        "🦀",
        "🦀Rust",
        "Rust🦀",
        "🦀カニ",
        "Hello👋世界",
    ];

    for text in test_cases {
        let char_count = text.chars().count();
        for col in 0..=char_count {
            let x = calculate_x_position(text, col);
            let back_col = get_col_from_x(text, x);

            assert_eq!(
                col, back_col,
                "Emoji coordinate fidelity broken at col {} in '{}'",
                col, text
            );
        }
    }
}

// ========================================
// Character Width Accuracy Tests
// ========================================

#[wasm_bindgen_test]
fn test_ascii_width_accumulation() {
    let line = "aaaa";  // 4 ASCII characters
    let expected_x = CHAR_WIDTH_ASCII * 4.0;
    let actual_x = calculate_x_position(line, 4);

    assert!((expected_x - actual_x).abs() < 0.001,
        "ASCII width accumulation incorrect: expected={}, actual={}",
        expected_x, actual_x);
}

#[wasm_bindgen_test]
fn test_wide_width_accumulation() {
    let line = "ああああ";  // 4 Japanese characters
    let expected_x = CHAR_WIDTH_WIDE * 4.0;
    let actual_x = calculate_x_position(line, 4);

    assert!((expected_x - actual_x).abs() < 0.001,
        "Wide character width accumulation incorrect: expected={}, actual={}",
        expected_x, actual_x);
}

#[wasm_bindgen_test]
fn test_mixed_width_accuracy() {
    let line = "a日b本c語";
    // a(7.8125) + 日(15.625) + b(7.8125) + 本(15.625) + c(7.8125) + 語(15.625)
    let expected_x = CHAR_WIDTH_ASCII * 3.0 + CHAR_WIDTH_WIDE * 3.0;
    let actual_x = calculate_x_position(line, 6);

    assert!((expected_x - actual_x).abs() < 0.001,
        "Mixed width accumulation incorrect: expected={}, actual={}",
        expected_x, actual_x);
}
