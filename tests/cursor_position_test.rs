//! Cursor Position Calculation Tests
//!
//! Tests for converting physical pixel coordinates to logical (line, column) positions
//! in a virtual scrolling editor.

// Layout constants (must match CSS in virtual_editor.rs)
const LINE_HEIGHT: f64 = 20.0;
const CHAR_WIDTH: f64 = 8.4;
const GUTTER_WIDTH: f64 = 50.0;

/// Helper function to calculate cursor position from click coordinates
fn calculate_cursor_position(
    click_x: f64,
    click_y: f64,
    scroll_top: f64,
    line_count: usize,
    line_lengths: &[usize],
) -> (usize, usize) {
    // 1. Add scroll offset to get absolute Y position in document
    let y_absolute = click_y + scroll_top;

    // 2. Calculate line index
    let clicked_line = (y_absolute / LINE_HEIGHT).floor() as usize;

    // 3. Calculate column index (x is already relative to content area, subtract padding)
    let clicked_col = ((click_x - 10.0).max(0.0) / CHAR_WIDTH).round() as usize;

    // 4. Clamp to valid range
    let clamped_line = clicked_line.min(line_count.saturating_sub(1));
    let line_len = line_lengths.get(clamped_line).copied().unwrap_or(0);
    let clamped_col = clicked_col.min(line_len);

    (clamped_line, clamped_col)
}

// ========================================
// 1. Basic Mapping Tests
// ========================================

#[test]
fn test_top_left_click() {
    // Click at the top-left corner (excluding gutter)
    let (line, col) = calculate_cursor_position(
        10.0,  // x: just the padding
        0.0,   // y: top
        0.0,   // no scroll
        100,   // 100 lines
        &vec![80; 100], // all lines have 80 chars
    );

    assert_eq!(line, 0, "Should be on the first line");
    assert_eq!(col, 0, "Should be at column 0");
}

#[test]
fn test_line_detection() {
    // Click on line 1 (second line, y=30px with LINE_HEIGHT=20px)
    let (line, col) = calculate_cursor_position(
        10.0,  // x: padding only
        30.0,  // y: 30px = line 1
        0.0,   // no scroll
        100,
        &vec![80; 100],
    );

    assert_eq!(line, 1, "Should be on the second line (index 1)");
}

#[test]
fn test_column_detection() {
    // Click at column 5 (x = 10 + 5 * 8.4 = 52px)
    let expected_x = 10.0 + 5.0 * CHAR_WIDTH;
    let (line, col) = calculate_cursor_position(
        expected_x,
        0.0,   // first line
        0.0,   // no scroll
        100,
        &vec![80; 100],
    );

    assert_eq!(col, 5, "Should be at column 5");
}

// ========================================
// 2. Virtual Scroll Tests
// ========================================

#[test]
fn test_scrolled_click() {
    // Scroll down 1000px (50 lines with LINE_HEIGHT=20px)
    // Click at y=10px on screen, which is actually line 50 + 0.5 = line 50
    let scroll_top = 1000.0;
    let click_y = 10.0;

    let (line, col) = calculate_cursor_position(
        10.0,
        click_y,
        scroll_top,
        200,  // large file
        &vec![80; 200],
    );

    let expected_line = ((click_y + scroll_top) / LINE_HEIGHT).floor() as usize;
    assert_eq!(line, expected_line, "Should calculate line based on scroll offset");
    assert_eq!(line, 50, "Should be on line 50 (1010 / 20 = 50.5, floor = 50)");
}

#[test]
fn test_large_file_boundary() {
    // Simulate a 10,000 line file scrolled near the end
    let line_count = 10000;
    let scroll_top = (line_count - 50) as f64 * LINE_HEIGHT; // Near bottom

    let (line, _) = calculate_cursor_position(
        10.0,
        100.0, // Click 100px down from visible top
        scroll_top,
        line_count,
        &vec![80; line_count],
    );

    assert!(line < line_count, "Should not exceed total line count");
    assert!(line >= line_count - 50, "Should be near the end of the file");
}

// ========================================
// 3. Edge Cases Tests
// ========================================

#[test]
fn test_gutter_click() {
    // Click in the gutter area (x < 10.0, the padding)
    let (_, col) = calculate_cursor_position(
        5.0,   // Before padding starts
        0.0,
        0.0,
        100,
        &vec![80; 100],
    );

    assert_eq!(col, 0, "Gutter click should snap to column 0");
}

#[test]
fn test_beyond_eol() {
    // Line has only 10 characters, but click at column 20
    let line_lengths = vec![10, 20, 30];
    let click_x = 10.0 + 20.0 * CHAR_WIDTH; // Position for column 20

    let (line, col) = calculate_cursor_position(
        click_x,
        0.0,   // First line
        0.0,
        3,
        &line_lengths,
    );

    assert_eq!(line, 0);
    assert_eq!(col, 10, "Should clamp to end of line (10 chars)");
}

#[test]
fn test_beyond_eof() {
    // File has 50 lines, but click calculates to line 100
    let line_count = 50;
    let click_y = 2000.0; // Would be line 100 (2000 / 20 = 100)

    let (line, _) = calculate_cursor_position(
        10.0,
        click_y,
        0.0,
        line_count,
        &vec![80; line_count],
    );

    assert_eq!(line, 49, "Should clamp to last line (index 49)");
}

#[test]
fn test_negative_coordinates() {
    // Simulate accidental negative coordinates (should be clamped to 0, 0)
    let (line, col) = calculate_cursor_position(
        -50.0, // Negative X
        -20.0, // Negative Y
        0.0,
        100,
        &vec![80; 100],
    );

    assert_eq!(line, 0, "Negative Y should result in line 0");
    assert_eq!(col, 0, "Negative X should result in column 0");
}

// ========================================
// 4. Font and Layout Consistency Tests
// ========================================

#[test]
fn test_char_width_consistency() {
    // Test multiple column positions
    let test_cases = vec![
        (0, 10.0 + 0.0 * CHAR_WIDTH),
        (1, 10.0 + 1.0 * CHAR_WIDTH),
        (5, 10.0 + 5.0 * CHAR_WIDTH),
        (10, 10.0 + 10.0 * CHAR_WIDTH),
    ];

    for (expected_col, click_x) in test_cases {
        let (_, col) = calculate_cursor_position(
            click_x,
            0.0,
            0.0,
            100,
            &vec![80; 100],
        );

        assert_eq!(
            col, expected_col,
            "Column {} should be calculated from x={}",
            expected_col, click_x
        );
    }
}

#[test]
fn test_fractional_coordinates() {
    // Test that fractional pixel coordinates are properly rounded
    // Click between column 2 and 3
    let x_col_2_5 = 10.0 + 2.5 * CHAR_WIDTH;

    let (_, col) = calculate_cursor_position(
        x_col_2_5,
        0.0,
        0.0,
        100,
        &vec![80; 100],
    );

    // Should round to nearest column (2.5 rounds to 3 with .round())
    assert!(col == 2 || col == 3, "Should round to nearest column");
}

#[test]
fn test_multiline_scroll_precision() {
    // Test precision across multiple scroll positions
    let line_count = 1000;
    let line_lengths = vec![80; line_count];

    // Test at different scroll positions
    for scroll_lines in [0, 10, 50, 100, 500, 900] {
        let scroll_top = scroll_lines as f64 * LINE_HEIGHT;
        let click_y = 5.0 * LINE_HEIGHT; // 5 lines down from visible top

        let (line, _) = calculate_cursor_position(
            10.0,
            click_y,
            scroll_top,
            line_count,
            &line_lengths,
        );

        let expected_line = scroll_lines + 5;
        assert_eq!(
            line, expected_line,
            "At scroll position {}, clicking 5 lines down should give line {}",
            scroll_lines, expected_line
        );
    }
}

// ========================================
// 5. Integration Tests
// ========================================

#[test]
fn test_realistic_editing_scenario() {
    // Simulate a realistic file with varying line lengths
    let line_lengths = vec![
        0,   // empty line
        15,  // short line
        80,  // full line
        5,   // very short
        120, // long line
    ];
    let line_count = line_lengths.len();

    // Scroll to line 2
    let scroll_top = 2.0 * LINE_HEIGHT;

    // Click on line 3 (visible as line 1), column 3
    let click_y = 1.0 * LINE_HEIGHT;
    let click_x = 10.0 + 3.0 * CHAR_WIDTH;

    let (line, col) = calculate_cursor_position(
        click_x,
        click_y,
        scroll_top,
        line_count,
        &line_lengths,
    );

    assert_eq!(line, 3, "Should be on line 3");
    assert_eq!(col, 3, "Should be at column 3");
}

#[test]
fn test_empty_file() {
    // Edge case: empty file or single empty line
    let (line, col) = calculate_cursor_position(
        50.0,
        50.0,
        0.0,
        1,
        &vec![0], // Single empty line
    );

    assert_eq!(line, 0, "Empty file should have cursor on line 0");
    assert_eq!(col, 0, "Empty file should have cursor at column 0");
}

#[test]
fn test_single_character_precision() {
    // Test clicking precisely on individual characters
    for target_col in 0..20 {
        let click_x = 10.0 + (target_col as f64) * CHAR_WIDTH;
        let (_, col) = calculate_cursor_position(
            click_x,
            0.0,
            0.0,
            100,
            &vec![80; 100],
        );

        assert_eq!(
            col, target_col,
            "Clicking at x={} should give column {}",
            click_x, target_col
        );
    }
}
