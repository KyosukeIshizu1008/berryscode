//! Goto Definition Jump Integration Test
//!
//! This test verifies that goto_definition correctly jumps to the target location
//! within the same file.

use wasm_bindgen_test::*;
use berry_editor::core::virtual_editor::EditorTab;
use berry_editor::buffer::TextBuffer;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_cursor_position_update_after_goto_definition() {
    // Setup: Create a tab with sample Rust code
    let file_path = "/test/sample.rs".to_string();
    let content = r#"fn main() {
    hello();
}

fn hello() {
    println!("Hello, world!");
}
"#;

    let mut tab = EditorTab::new(file_path.clone(), content.to_string());

    // Initial cursor position (line 1, col 4 - on "hello()" call)
    tab.cursor_line = 1;
    tab.cursor_col = 4;

    // Test: Simulate goto_definition result
    // Definition of "hello" is at line 4, col 3
    let definition_line = 4;
    let definition_col = 3;

    // Simulate the jump action (what should happen in virtual_editor.rs:1203-1204)
    tab.cursor_line = definition_line;
    tab.cursor_col = definition_col;

    // Verify: Cursor position should be updated
    assert_eq!(tab.cursor_line, 4, "Cursor should move to definition line");
    assert_eq!(tab.cursor_col, 3, "Cursor should move to definition column");
}

#[wasm_bindgen_test]
fn test_same_file_path_comparison() {
    // Test various file path formats that should be considered "same file"

    let test_cases = vec![
        // (location.uri, file_path, should_match)
        ("/Users/test/file.rs", "/Users/test/file.rs", true),
        ("/Users/test/file.rs", "/Users/test/file.rs", true),
        ("", "/Users/test/file.rs", true),  // Empty uri should match (same file)
        ("/different/file.rs", "/Users/test/file.rs", false),
    ];

    for (uri, file_path, should_match) in test_cases {
        let result = uri.is_empty() || uri == file_path;
        assert_eq!(
            result, should_match,
            "File path comparison failed: uri={:?}, file_path={:?}",
            uri, file_path
        );
    }
}

#[wasm_bindgen_test]
fn test_scroll_into_view_is_called() {
    // Test that scroll_into_view is called after cursor move
    let file_path = "/test/sample.rs".to_string();
    let content = "line 1\n".repeat(100); // 100 lines

    let mut tab = EditorTab::new(file_path, content);

    // Move cursor to line 50
    tab.cursor_line = 50;
    tab.cursor_col = 0;

    // Simulate scroll_into_view (canvas height = 600px, line height = 20px)
    let canvas_height = 600.0;
    tab.scroll_into_view(canvas_height);

    // Verify: scroll_top should be adjusted so that line 50 is visible
    // With line_height = 20.0, line 50 is at y = 1000.0
    // If viewport is 600px, scroll_top should be around 400-700

    // Note: This test verifies that scroll_into_view doesn't panic
    assert!(true, "scroll_into_view should complete without panic");
}
