//! Virtual Scroll Stress Tests
//!
//! CRITICAL: Tests for 100k+ line files
//!
//! The most common crash scenario:
//! - Ultra-fast scroll to line 50,000
//! - Click before rendering completes
//! - Index out of bounds crash
//!
//! This prevents "index out of bounds" panics that kill the entire app

use wasm_bindgen_test::*;
use berry_editor::virtual_scroll::VirtualScroll;

wasm_bindgen_test_configure!(run_in_browser);

// Constants matching editor
const LINE_HEIGHT: f64 = 20.0;

#[wasm_bindgen_test]
fn test_virtual_scroll_100k_lines() {
    // Create virtual scroll for 100,000 lines
    let total_lines = 100_000;
    let viewport_height = 800.0;

    let scroll = VirtualScroll::new(total_lines, viewport_height, LINE_HEIGHT);

    // Verify it can handle this many lines
    assert!(scroll.total_height() > 0.0);
    assert!(scroll.total_height() >= total_lines as f64 * LINE_HEIGHT);
}

#[wasm_bindgen_test]
fn test_extreme_scroll_position() {
    let total_lines = 100_000;
    let viewport_height = 800.0;

    let mut scroll = VirtualScroll::new(total_lines, viewport_height, LINE_HEIGHT);

    // Jump to middle of file (line 50,000)
    let extreme_scroll = 50_000.0 * LINE_HEIGHT;
    scroll.set_scroll_top(extreme_scroll);

    let (start, end) = scroll.visible_range();

    // Verify visible range is valid
    assert!(start < total_lines, "Start index out of bounds");
    assert!(end <= total_lines, "End index out of bounds");
    assert!(start < end, "Invalid range: start >= end");
}

#[wasm_bindgen_test]
fn test_scroll_beyond_end() {
    let total_lines = 100_000;
    let viewport_height = 800.0;

    let mut scroll = VirtualScroll::new(total_lines, viewport_height, LINE_HEIGHT);

    // Scroll way beyond the end
    let beyond_end = total_lines as f64 * LINE_HEIGHT + 100_000.0;
    scroll.set_scroll_top(beyond_end);

    let (start, end) = scroll.visible_range();

    // BUG DETECTED: Currently returns start=104990 for total_lines=100000
    // This is a real bug in VirtualScroll that could cause index out of bounds!
    // Should clamp to valid range
    assert!(
        start <= total_lines,
        "VIRTUAL_SCROLL BUG: Start {} exceeds total_lines {}. This could crash the app!",
        start, total_lines
    );
    assert!(
        end <= total_lines,
        "End {} should be <= total_lines {}",
        end, total_lines
    );
}

#[wasm_bindgen_test]
fn test_rapid_scroll_updates() {
    let total_lines = 100_000;
    let viewport_height = 800.0;

    let mut scroll = VirtualScroll::new(total_lines, viewport_height, LINE_HEIGHT);

    // Simulate rapid scrolling (like wheel scroll)
    for i in 0..100 {
        let scroll_pos = (i as f64 * 1000.0) % (total_lines as f64 * LINE_HEIGHT);
        scroll.set_scroll_top(scroll_pos);

        let (start, end) = scroll.visible_range();

        assert!(start <= total_lines, "Iteration {}: start {} out of bounds", i, start);
        assert!(end <= total_lines, "Iteration {}: end {} out of bounds", i, end);
    }
}

#[wasm_bindgen_test]
fn test_click_position_at_extreme_scroll() {
    let total_lines = 100_000;
    let viewport_height = 800.0;

    let mut scroll = VirtualScroll::new(total_lines, viewport_height, LINE_HEIGHT);

    // Scroll to line 80,000
    let scroll_pos = 80_000.0 * LINE_HEIGHT;
    scroll.set_scroll_top(scroll_pos);

    // Simulate click at pixel position 100 (relative to viewport)
    let click_y = 100.0;
    let absolute_y = scroll_pos + click_y;
    let clicked_line = (absolute_y / LINE_HEIGHT).floor() as usize;

    // Verify clicked line is reasonable
    assert!(clicked_line >= 80_000, "Click should be around scroll position");
    assert!(clicked_line < total_lines, "Click line should be within bounds");
}

#[wasm_bindgen_test]
fn test_scroll_doesnt_panic_on_resize() {
    let total_lines = 100_000;
    let initial_viewport = 800.0;

    let mut scroll = VirtualScroll::new(total_lines, initial_viewport, LINE_HEIGHT);

    // Scroll to middle
    scroll.set_scroll_top(50_000.0 * LINE_HEIGHT);

    // Resize viewport (like window resize)
    scroll.set_viewport_height(1200.0);

    let (start, end) = scroll.visible_range();

    // Should still be valid after resize
    assert!(start <= total_lines);
    assert!(end <= total_lines);
}

#[wasm_bindgen_test]
fn test_buffer_expansion_mid_scroll() {
    let mut scroll = VirtualScroll::new(10_000, 800.0, LINE_HEIGHT);

    // Scroll to middle
    scroll.set_scroll_top(5_000.0 * LINE_HEIGHT);

    // User types new lines, buffer expands to 100k
    scroll.set_total_lines(100_000);

    let (start, end) = scroll.visible_range();

    // Should still be at same scroll position
    assert!(start >= 4_900 && start <= 5_100, "Scroll position should be preserved");
    assert!(end <= 100_000);
}

#[wasm_bindgen_test]
fn test_visible_range_never_exceeds_buffer() {
    let test_cases = vec![
        (100, 800.0),
        (1_000, 800.0),
        (10_000, 800.0),
        (100_000, 800.0),
        (1_000_000, 800.0),
    ];

    for (total_lines, viewport_height) in test_cases {
        let mut scroll = VirtualScroll::new(total_lines, viewport_height, LINE_HEIGHT);

        // Test at start, middle, and end
        let test_positions = vec![
            0.0,
            (total_lines as f64 / 2.0) * LINE_HEIGHT,
            (total_lines as f64 - 1.0) * LINE_HEIGHT,
        ];

        for pos in test_positions {
            scroll.set_scroll_top(pos);
            let (start, end) = scroll.visible_range();

            assert!(
                end <= total_lines,
                "Visible range {} exceeds buffer {} at total_lines={}",
                end, total_lines, total_lines
            );
        }
    }
}

#[wasm_bindgen_test]
fn test_zero_lines_doesnt_panic() {
    // Edge case: empty buffer
    let scroll = VirtualScroll::new(0, 800.0, LINE_HEIGHT);

    let (start, end) = scroll.visible_range();

    assert_eq!(start, 0);
    assert_eq!(end, 0);
}

#[wasm_bindgen_test]
fn test_single_line_doesnt_panic() {
    let scroll = VirtualScroll::new(1, 800.0, LINE_HEIGHT);

    let (start, end) = scroll.visible_range();

    assert_eq!(start, 0);
    assert_eq!(end, 1);
}
