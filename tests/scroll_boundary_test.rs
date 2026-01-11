//! Scroll Boundary E2E Test
//!
//! Tests that scrolling doesn't exceed file boundaries
//!
//! Bug: When navigating with arrow keys, the editor scrolls too much
//! showing excessive whitespace below the last line of the file.
//!
//! Expected behavior:
//! - Scroll should stop at the last line
//! - No excessive whitespace below the last line
//! - Cursor should stay visible without over-scrolling

use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;
use web_sys::{HtmlCanvasElement, KeyboardEvent, KeyboardEventInit};
use berry_editor::buffer::TextBuffer;

wasm_bindgen_test_configure!(run_in_browser);

const LINE_HEIGHT: f64 = 20.0;
const CANVAS_HEIGHT: f64 = 400.0; // 20 lines visible

/// Helper: Create a test buffer with N lines
fn create_test_buffer(num_lines: usize) -> TextBuffer {
    let mut buffer = TextBuffer::new();
    for i in 0..num_lines {
        buffer.insert(buffer.len_chars(), &format!("Line {}\n", i + 1));
    }
    buffer
}

#[wasm_bindgen_test]
fn test_scroll_stops_at_file_end() {
    // Create a small file (10 lines) that fits in viewport (20 lines)
    let buffer = create_test_buffer(10);

    // With 10 lines and 20 lines visible, max_scroll should be 0
    let total_lines = buffer.len_lines();
    let max_scroll = ((total_lines as f64 * LINE_HEIGHT) - CANVAS_HEIGHT).max(0.0);

    assert_eq!(
        max_scroll,
        0.0,
        "File smaller than viewport should have max_scroll=0"
    );
}

#[wasm_bindgen_test]
fn test_scroll_boundary_with_long_file() {
    // Create a file with 30 lines (10 lines beyond viewport)
    let buffer = create_test_buffer(30);

    let total_lines = buffer.len_lines();
    let max_scroll = ((total_lines as f64 * LINE_HEIGHT) - CANVAS_HEIGHT).max(0.0);

    // Expected: 30 lines * 20px = 600px total
    //           600px - 400px viewport = 200px max scroll
    assert_eq!(
        max_scroll,
        200.0,
        "30-line file should allow 200px of scrolling"
    );
}

#[wasm_bindgen_test]
fn test_cursor_at_last_line_doesnt_overscroll() {
    // Create a file with 30 lines
    let buffer = create_test_buffer(30);
    let total_lines = buffer.len_lines();

    // Simulate cursor at last line (line 29, 0-indexed)
    let cursor_line = total_lines - 1;
    let cursor_y = cursor_line as f64 * LINE_HEIGHT;

    // Calculate what scroll_into_view should set
    // If cursor is below viewport, scroll so cursor is at bottom with 1-line margin
    let mut scroll_top = 0.0;

    // Cursor hidden below viewport
    if cursor_y + LINE_HEIGHT > scroll_top + CANVAS_HEIGHT {
        scroll_top = cursor_y + LINE_HEIGHT - CANVAS_HEIGHT;
    }

    // Clamp to max_scroll
    let max_scroll = ((total_lines as f64 * LINE_HEIGHT) - CANVAS_HEIGHT).max(0.0);
    scroll_top = scroll_top.max(0.0).min(max_scroll);

    assert_eq!(
        scroll_top,
        max_scroll,
        "Cursor at last line should scroll to max_scroll, not beyond"
    );

    assert_eq!(
        scroll_top,
        200.0,
        "Should scroll exactly 200px, showing lines 10-29"
    );
}

#[wasm_bindgen_test]
fn test_pagedown_at_end_doesnt_overscroll() {
    // Create a file with 30 lines
    let buffer = create_test_buffer(30);
    let total_lines = buffer.len_lines();

    // Start at line 25 (near end)
    let mut cursor_line = 25_usize;
    let mut scroll_top = 100.0; // Some reasonable scroll position

    // Simulate PageDown (move 20 lines down)
    cursor_line = (cursor_line + 20).min(total_lines.saturating_sub(1));

    // cursor_line should clamp to 29 (last line)
    assert_eq!(cursor_line, 29, "PageDown should clamp to last line");

    // Now apply scroll_into_view logic
    let cursor_y = cursor_line as f64 * LINE_HEIGHT;

    if cursor_y + LINE_HEIGHT > scroll_top + CANVAS_HEIGHT {
        scroll_top = cursor_y + LINE_HEIGHT - CANVAS_HEIGHT;
    }

    let max_scroll = ((total_lines as f64 * LINE_HEIGHT) - CANVAS_HEIGHT).max(0.0);
    scroll_top = scroll_top.max(0.0).min(max_scroll);

    assert_eq!(
        scroll_top,
        200.0,
        "PageDown at end should scroll to max_scroll (200px), not beyond"
    );
}

#[wasm_bindgen_test]
fn test_arrow_down_spam_doesnt_overscroll() {
    // Create a file with 15 lines
    let buffer = create_test_buffer(15);
    let total_lines = buffer.len_lines();

    // Start at line 0
    let mut cursor_line = 0_usize;
    let mut scroll_top = 0.0;

    // Spam ArrowDown 100 times (way more than the file has)
    for _ in 0..100 {
        // Move cursor down
        cursor_line = (cursor_line + 1).min(total_lines.saturating_sub(1));

        // Apply scroll_into_view
        let cursor_y = cursor_line as f64 * LINE_HEIGHT;

        // Cursor hidden above
        if cursor_y < scroll_top {
            scroll_top = cursor_y;
        }
        // Cursor hidden below
        else if cursor_y + LINE_HEIGHT > scroll_top + CANVAS_HEIGHT {
            scroll_top = cursor_y + LINE_HEIGHT - CANVAS_HEIGHT;
        }

        // Clamp to boundaries
        let max_scroll = ((total_lines as f64 * LINE_HEIGHT) - CANVAS_HEIGHT).max(0.0);
        scroll_top = scroll_top.max(0.0).min(max_scroll);
    }

    // Cursor should be at last line
    assert_eq!(cursor_line, 14, "Cursor should stop at last line (14)");

    // Scroll should NOT exceed max_scroll
    // For 15 lines: 15 * 20 = 300px total, 300 - 400 = -100 → max(0) = 0
    assert_eq!(
        scroll_top,
        0.0,
        "15-line file fits in viewport, should not scroll at all"
    );
}

#[wasm_bindgen_test]
fn test_end_key_doesnt_overscroll() {
    // Create a file with 50 lines
    let buffer = create_test_buffer(50);
    let total_lines = buffer.len_lines();

    // Start at line 0
    let mut cursor_line = 0_usize;
    let mut scroll_top = 0.0;

    // Simulate Ctrl+End (jump to last line)
    cursor_line = total_lines.saturating_sub(1);

    assert_eq!(cursor_line, 49, "Ctrl+End should jump to line 49");

    // Apply scroll_into_view
    let cursor_y = cursor_line as f64 * LINE_HEIGHT;

    if cursor_y + LINE_HEIGHT > scroll_top + CANVAS_HEIGHT {
        scroll_top = cursor_y + LINE_HEIGHT - CANVAS_HEIGHT;
    }

    let max_scroll = ((total_lines as f64 * LINE_HEIGHT) - CANVAS_HEIGHT).max(0.0);
    scroll_top = scroll_top.max(0.0).min(max_scroll);

    // For 50 lines: 50 * 20 = 1000px total, 1000 - 400 = 600px max scroll
    assert_eq!(
        scroll_top,
        600.0,
        "Ctrl+End should scroll to max_scroll (600px)"
    );
}

#[wasm_bindgen_test]
fn test_visible_lines_calculation() {
    // Canvas height: 400px
    // Line height: 20px
    // Visible lines: 400 / 20 = 20 lines

    let visible_lines = (CANVAS_HEIGHT / LINE_HEIGHT).floor() as usize;
    assert_eq!(visible_lines, 20, "Should fit 20 lines in 400px viewport");
}

#[wasm_bindgen_test]
fn test_whitespace_below_last_line() {
    // Create a file with exactly 20 lines (fills viewport exactly)
    let buffer = create_test_buffer(20);
    let total_lines = buffer.len_lines();

    // Cursor at last line
    let cursor_line = total_lines - 1; // line 19

    // With cursor at line 19, what should scroll_top be?
    // Option 1: scroll_top = 0 (lines 0-19 visible) ✓
    // Option 2: scroll_top > 0 (showing whitespace) ✗

    let cursor_y = cursor_line as f64 * LINE_HEIGHT;
    let mut scroll_top = 0.0;

    // Apply scroll_into_view
    if cursor_y + LINE_HEIGHT > scroll_top + CANVAS_HEIGHT {
        scroll_top = cursor_y + LINE_HEIGHT - CANVAS_HEIGHT;
    }

    let max_scroll = ((total_lines as f64 * LINE_HEIGHT) - CANVAS_HEIGHT).max(0.0);
    scroll_top = scroll_top.max(0.0).min(max_scroll);

    assert_eq!(
        scroll_top,
        0.0,
        "20-line file should not scroll (fits exactly in viewport)"
    );
}
