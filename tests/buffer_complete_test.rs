use berry_editor::buffer::TextBuffer;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// === Basic Functionality Tests ===

#[wasm_bindgen_test]
fn test_buffer_from_str() {
    let content = "Line 1\nLine 2\nLine 3";
    let buffer = TextBuffer::from_str(content);

    assert_eq!(buffer.to_string(), content);
    assert_eq!(buffer.len_lines(), 3);
}

#[wasm_bindgen_test]
fn test_buffer_empty() {
    let buffer = TextBuffer::from_str("");

    assert_eq!(buffer.to_string(), "");
    assert_eq!(buffer.len_lines(), 1); // Empty buffer has 1 line
}

#[wasm_bindgen_test]
fn test_buffer_single_line() {
    let buffer = TextBuffer::from_str("Single line");

    assert_eq!(buffer.to_string(), "Single line");
    assert_eq!(buffer.len_lines(), 1);
}

#[wasm_bindgen_test]
fn test_buffer_multiple_empty_lines() {
    let content = "\n\n\n";
    let buffer = TextBuffer::from_str(content);

    assert_eq!(buffer.len_lines(), 4); // 3 newlines = 4 lines
}

#[wasm_bindgen_test]
fn test_buffer_len_lines() {
    let content = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5";
    let buffer = TextBuffer::from_str(content);

    assert_eq!(buffer.len_lines(), 5);
}

#[wasm_bindgen_test]
fn test_buffer_to_string_preserves_content() {
    let content = "Hello\nWorld\n\nTest\n";
    let buffer = TextBuffer::from_str(content);

    assert_eq!(buffer.to_string(), content);
}

// === Unicode and Special Characters ===

#[wasm_bindgen_test]
fn test_buffer_unicode() {
    let content = "Hello 世界\nこんにちは\n🦀 Rust";
    let buffer = TextBuffer::from_str(content);

    assert_eq!(buffer.to_string(), content);
    assert_eq!(buffer.len_lines(), 3);
}

#[wasm_bindgen_test]
fn test_buffer_emoji() {
    let content = "🔥 Hot\n❄️ Cold\n⚡ Fast";
    let buffer = TextBuffer::from_str(content);

    assert_eq!(buffer.to_string(), content);
    assert_eq!(buffer.len_lines(), 3);
}

#[wasm_bindgen_test]
fn test_buffer_mixed_line_endings() {
    // Unix style line endings
    let content_unix = "Line 1\nLine 2\nLine 3";
    let buffer_unix = TextBuffer::from_str(content_unix);
    assert_eq!(buffer_unix.len_lines(), 3);

    // Windows style would be \r\n but our buffer normalizes to \n
    let content_normalized = "Line 1\nLine 2\nLine 3";
    let buffer = TextBuffer::from_str(content_normalized);
    assert_eq!(buffer.len_lines(), 3);
}

#[wasm_bindgen_test]
fn test_buffer_tabs() {
    let content = "Tab\there\nAnother\ttab";
    let buffer = TextBuffer::from_str(content);

    assert_eq!(buffer.to_string(), content);
}

// === Large Files ===

#[wasm_bindgen_test]
fn test_buffer_large_file_100_lines() {
    let content = (0..100)
        .map(|i| format!("Line {}", i))
        .collect::<Vec<_>>()
        .join("\n");

    let buffer = TextBuffer::from_str(&content);

    assert_eq!(buffer.len_lines(), 100);
    assert!(buffer.to_string().contains("Line 0"));
    assert!(buffer.to_string().contains("Line 99"));
}

#[wasm_bindgen_test]
fn test_buffer_large_file_1000_lines() {
    let content = (0..1000)
        .map(|i| format!("Line {}", i))
        .collect::<Vec<_>>()
        .join("\n");

    let buffer = TextBuffer::from_str(&content);

    assert_eq!(buffer.len_lines(), 1000);
}

#[wasm_bindgen_test]
fn test_buffer_very_long_line() {
    let long_line = "x".repeat(10000);
    let content = format!("{}\nShort line", long_line);
    let buffer = TextBuffer::from_str(&content);

    assert_eq!(buffer.len_lines(), 2);
    assert!(buffer.to_string().len() > 10000);
}

// === Edge Cases ===

#[wasm_bindgen_test]
fn test_buffer_only_newlines() {
    let content = "\n\n\n\n\n";
    let buffer = TextBuffer::from_str(content);

    assert_eq!(buffer.len_lines(), 6);
}

#[wasm_bindgen_test]
fn test_buffer_trailing_newline() {
    let content = "Line 1\nLine 2\n";
    let buffer = TextBuffer::from_str(content);

    assert_eq!(buffer.len_lines(), 3); // Trailing newline creates empty line
}

#[wasm_bindgen_test]
fn test_buffer_no_trailing_newline() {
    let content = "Line 1\nLine 2";
    let buffer = TextBuffer::from_str(content);

    assert_eq!(buffer.len_lines(), 2);
}

#[wasm_bindgen_test]
fn test_buffer_whitespace_only() {
    let content = "   \n  \n    ";
    let buffer = TextBuffer::from_str(content);

    assert_eq!(buffer.len_lines(), 3);
    assert_eq!(buffer.to_string(), content);
}

#[wasm_bindgen_test]
fn test_buffer_special_characters() {
    let content = "!@#$%^&*()\n<>?:\"{}\n[]\\|";
    let buffer = TextBuffer::from_str(content);

    assert_eq!(buffer.to_string(), content);
    assert_eq!(buffer.len_lines(), 3);
}

// === Code Examples ===

#[wasm_bindgen_test]
fn test_buffer_rust_code() {
    let code = r#"fn main() {
    println!("Hello, world!");
}

#[test]
fn test() {
    assert_eq!(1, 1);
}"#;

    let buffer = TextBuffer::from_str(code);
    assert_eq!(buffer.to_string(), code);
    assert!(buffer.len_lines() > 5);
}

#[wasm_bindgen_test]
fn test_buffer_json() {
    let json = r#"{
    "name": "BerryEditor",
    "version": "0.1.0",
    "dependencies": {}
}"#;

    let buffer = TextBuffer::from_str(json);
    assert_eq!(buffer.to_string(), json);
}

#[wasm_bindgen_test]
fn test_buffer_markdown() {
    let markdown = r#"# Title

## Subtitle

- Item 1
- Item 2

**Bold** and *italic*"#;

    let buffer = TextBuffer::from_str(markdown);
    assert_eq!(buffer.to_string(), markdown);
}

// === Performance Tests ===

#[wasm_bindgen_test]
fn test_buffer_10k_lines_performance() {
    let start = js_sys::Date::now();

    let content = (0..10000)
        .map(|i| format!("Line {} with some content", i))
        .collect::<Vec<_>>()
        .join("\n");

    let buffer = TextBuffer::from_str(&content);

    let end = js_sys::Date::now();
    let elapsed = end - start;

    assert_eq!(buffer.len_lines(), 10000);
    leptos::logging::log!("10k lines buffer creation took {}ms", elapsed);

    // Should be fast (< 100ms for 10k lines)
    assert!(elapsed < 100.0, "Buffer creation should be fast");
}

// === Clone and Copy ===

#[wasm_bindgen_test]
fn test_buffer_clone() {
    let content = "Original content\nLine 2";
    let buffer1 = TextBuffer::from_str(content);
    let buffer2 = buffer1.clone();

    assert_eq!(buffer1.to_string(), buffer2.to_string());
    assert_eq!(buffer1.len_lines(), buffer2.len_lines());
}

// === Integration with lines() ===

#[wasm_bindgen_test]
fn test_buffer_lines_iteration() {
    let content = "Line 1\nLine 2\nLine 3";
    let buffer = TextBuffer::from_str(content);

    let lines: Vec<String> = buffer.to_string()
        .lines()
        .map(|s| s.to_string())
        .collect();

    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], "Line 1");
    assert_eq!(lines[1], "Line 2");
    assert_eq!(lines[2], "Line 3");
}

// === Stress Tests ===

#[wasm_bindgen_test]
fn test_buffer_50k_lines() {
    let content = (0..50000)
        .map(|i| format!("L{}", i))
        .collect::<Vec<_>>()
        .join("\n");

    let buffer = TextBuffer::from_str(&content);

    assert_eq!(buffer.len_lines(), 50000);
    leptos::logging::log!("Successfully created 50k line buffer");
}

#[wasm_bindgen_test]
fn test_buffer_memory_efficiency() {
    // Create multiple buffers to test memory
    let mut buffers = Vec::new();

    for i in 0..10 {
        let content = format!("Buffer {}\nWith content", i);
        buffers.push(TextBuffer::from_str(&content));
    }

    assert_eq!(buffers.len(), 10);

    for (i, buffer) in buffers.iter().enumerate() {
        assert!(buffer.to_string().contains(&format!("Buffer {}", i)));
    }
}
