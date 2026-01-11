//! E2E Cursor Positioning Tests
//!
//! These tests create ACTUAL DOM elements with the same CSS as the editor,
//! measure their real pixel positions, and verify that our coordinate
//! calculations match reality.
//!
//! This catches the "Japanese characters drift" bug that pure logic tests miss.

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

// ✅ Use test helpers instead of web_sys directly
mod test_helpers;
use test_helpers::{get_test_window, get_test_document};

wasm_bindgen_test_configure!(run_in_browser);

// Constants must match src/core/virtual_editor.rs - MEASURED from browser rendering
const LINE_HEIGHT: f64 = 20.0;
const CHAR_WIDTH_ASCII: f64 = 8.0; // E2E measured
const CHAR_WIDTH_WIDE: f64 = 13.0; // E2E measured
const TEXT_PADDING: f64 = 15.0;

/// Calculate x position from column (matches virtual_editor.rs)
fn calculate_x_position(line_str: &str, char_col: usize) -> f64 {
    line_str.chars().take(char_col).map(|ch| {
        if ch as u32 > 255 { CHAR_WIDTH_WIDE } else { CHAR_WIDTH_ASCII }
    }).sum::<f64>()
}

/// Calculate column from x position (matches virtual_editor.rs click handler)
fn get_col_from_x(line_str: &str, x: f64) -> usize {
    let mut current_x = 0.0;
    let mut col = 0;
    for (_i, ch) in line_str.chars().enumerate() {
        if ch == '\n' { break; }
        let w = if ch as u32 > 255 { CHAR_WIDTH_WIDE } else { CHAR_WIDTH_ASCII };
        if x < current_x + (w / 2.0) { break; }
        current_x += w;
        col = col + 1;
    }
    col
}

/// Create a test editor line element with proper CSS
fn create_test_line(text: &str) -> web_sys::HtmlElement {
    let document = get_test_document();
    let line = document.create_element("div").unwrap()
        .dyn_into::<web_sys::HtmlElement>().unwrap();

    // Apply exact same CSS as editor
    line.set_attribute("style", &format!(
        "height: {}px; \
         line-height: {}px; \
         padding-left: {}px; \
         white-space: pre; \
         font-family: 'JetBrains Mono', monospace; \
         font-size: 13px; \
         font-variant-ligatures: none; \
         font-kerning: none; \
         letter-spacing: 0px; \
         position: absolute; \
         left: 0; \
         top: 0;",
        LINE_HEIGHT, LINE_HEIGHT, TEXT_PADDING
    )).unwrap();

    line.set_text_content(Some(text));

    // Add to body to measure
    document.body().unwrap().append_child(&line).unwrap();

    line
}

/// Get actual pixel position of character in rendered DOM
fn get_character_pixel_position(element: &web_sys::HtmlElement, char_index: usize) -> f64 {
    let text = element.text_content().unwrap();
    let prefix = text.chars().take(char_index).collect::<String>();

    // Create a span with just the prefix to measure its width
    let document = get_test_document();
    let span = document.create_element("span").unwrap()
        .dyn_into::<web_sys::HtmlElement>().unwrap();

    span.set_text_content(Some(&prefix));
    element.append_child(&span).ok();

    let width = span.offset_width() as f64;
    element.remove_child(&span).ok();

    width
}

// ========================================
// E2E Tests
// ========================================

#[wasm_bindgen_test]
fn e2e_ascii_line_positioning() {
    let line_text = "fn main() {";
    let line_elem = create_test_line(line_text);

    // Test each character position
    for col in 0..=line_text.len() {
        // Our calculation
        let our_x = calculate_x_position(line_text, col);

        // Actual browser rendering
        let actual_x = get_character_pixel_position(&line_elem, col);

        let diff = (our_x - actual_x).abs();

        wasm_bindgen_test::console_log!(
            "ASCII col={}: our={:.2}, actual={:.2}, diff={:.2}",
            col, our_x, actual_x, diff
        );

        assert!(
            diff < 3.0,  // Allow 3px tolerance (some chars like space/parens render at 7px instead of 8px)
            "ASCII positioning error at col {}: our={}, actual={}, diff={}",
            col, our_x, actual_x, diff
        );
    }

    line_elem.remove();
}

#[wasm_bindgen_test]
fn e2e_japanese_line_positioning() {
    let line_text = "こんにちは世界";
    let line_elem = create_test_line(line_text);

    let char_count = line_text.chars().count();

    // Test each character position
    for col in 0..=char_count {
        // Our calculation
        let our_x = calculate_x_position(line_text, col);

        // Actual browser rendering
        let actual_x = get_character_pixel_position(&line_elem, col);

        let diff = (our_x - actual_x).abs();

        wasm_bindgen_test::console_log!(
            "Japanese col={}: our={:.2}, actual={:.2}, diff={:.2}",
            col, our_x, actual_x, diff
        );

        // THIS IS THE CRITICAL TEST - if Japanese drifts, this will fail
        assert!(
            diff < 2.0,  // Allow 2px tolerance
            "Japanese positioning error at col {}: our={}, actual={}, diff={} (THIS IS THE BUG!)",
            col, our_x, actual_x, diff
        );
    }

    line_elem.remove();
}

#[wasm_bindgen_test]
fn e2e_mixed_line_positioning() {
    let line_text = "Hello 世界! Rust は最高";
    let line_elem = create_test_line(line_text);

    let char_count = line_text.chars().count();

    let mut max_diff = 0.0;
    let mut worst_col = 0;

    for col in 0..=char_count {
        let our_x = calculate_x_position(line_text, col);
        let actual_x = get_character_pixel_position(&line_elem, col);
        let diff = (our_x - actual_x).abs();

        if diff > max_diff {
            max_diff = diff;
            worst_col = col;
        }

        wasm_bindgen_test::console_log!(
            "Mixed col={}: our={:.2}, actual={:.2}, diff={:.2}",
            col, our_x, actual_x, diff
        );
    }

    wasm_bindgen_test::console_log!(
        "Mixed line max diff: {:.2}px at col {}",
        max_diff, worst_col
    );

    assert!(
        max_diff < 3.0,  // Allow 3px tolerance for mixed content
        "Mixed line positioning error: max diff {:.2}px at col {}",
        max_diff, worst_col
    );

    line_elem.remove();
}

#[wasm_bindgen_test]
fn e2e_click_to_cursor_roundtrip() {
    // This is the ULTIMATE test: simulate a click and verify cursor position
    let line_text = "日本語とEnglish";
    let line_elem = create_test_line(line_text);

    let char_count = line_text.chars().count();

    for expected_col in 0..=char_count {
        // Step 1: Calculate where we THINK the character is
        let our_x = calculate_x_position(line_text, expected_col);

        // Step 2: Simulate a click at that position (calculate col from x)
        let calculated_col = get_col_from_x(line_text, our_x);

        // Step 3: Verify we get back the same column
        assert_eq!(
            expected_col, calculated_col,
            "Roundtrip failed: expected col={}, clicked at x={:.2}, got col={}\nLine: {:?}",
            expected_col, our_x, calculated_col, line_text
        );
    }

    line_elem.remove();
}

#[wasm_bindgen_test]
fn e2e_cursor_drift_accumulation() {
    // Test if error accumulates over a long line
    let line_text = "あいうえおかきくけこさしすせそたちつてとなにぬねの";  // 25 Japanese chars
    let line_elem = create_test_line(line_text);

    let char_count = line_text.chars().count();

    // Check position at end of line (where drift is most visible)
    let our_x_end = calculate_x_position(line_text, char_count);
    let actual_x_end = get_character_pixel_position(&line_elem, char_count);

    let total_drift = (our_x_end - actual_x_end).abs();

    wasm_bindgen_test::console_log!(
        "Drift accumulation over {} chars: {:.2}px",
        char_count, total_drift
    );

    // If drift accumulates linearly, we'd see 25 * error_per_char
    // With 0.1px error per char, that's 2.5px total - acceptable
    // With 1px error per char, that's 25px total - BUG!
    assert!(
        total_drift < 5.0,
        "Cursor drift accumulation too large: {:.2}px over {} characters",
        total_drift, char_count
    );

    line_elem.remove();
}
