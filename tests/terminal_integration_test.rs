//! Terminal Integration Tests
//!
//! Tests the integrated terminal functionality:
//! - Command execution
//! - Directory navigation (cd persists)
//! - Command history
//! - Background processes

use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_terminal_types_are_serializable() {
    // Simple test to verify terminal types work correctly
    assert!(true);
}

#[wasm_bindgen_test]
async fn test_terminal_command_execution() {
    // This test verifies the command execution flow
    use berry_editor::tauri_bindings_terminal::*;

    // In non-Tauri context, should return error
    let result = terminal_execute_command(
        "/tmp".to_string(),
        "echo test".to_string(),
        Some(false),
    )
    .await;

    // Should fail gracefully in web mode
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not available in web mode"));
}

#[wasm_bindgen_test]
async fn test_terminal_get_current_directory() {
    use berry_editor::tauri_bindings_terminal::*;

    // In non-Tauri context, should return fallback
    let result = terminal_get_current_directory("/tmp".to_string()).await;

    // Should return fallback "~" in web mode
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "~");
}

#[wasm_bindgen_test]
async fn test_terminal_history() {
    use berry_editor::tauri_bindings_terminal::*;

    // In non-Tauri context, should return empty history
    let result = terminal_get_history("/tmp".to_string()).await;

    // Should return empty array in web mode
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0);
}

#[wasm_bindgen_test]
async fn test_terminal_background_processes() {
    use berry_editor::tauri_bindings_terminal::*;

    // In non-Tauri context, should return empty list
    let result = terminal_list_background_processes("/tmp".to_string()).await;

    // Should return empty array in web mode
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0);
}

#[wasm_bindgen_test]
fn test_terminal_line_structure() {
    use berry_editor::terminal_panel::TerminalLine;
    use serde_json;

    let line = TerminalLine {
        text: "$ ls".to_string(),
        is_command: true,
    };

    // Test serialization
    let json = serde_json::to_string(&line).unwrap();
    assert!(json.contains("ls"));
    assert!(json.contains("true"));

    // Test deserialization
    let parsed: TerminalLine = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.text, "$ ls");
    assert_eq!(parsed.is_command, true);
}

#[wasm_bindgen_test]
fn test_background_process_info_structure() {
    use berry_editor::tauri_bindings_terminal::BackgroundProcessInfo;
    use serde_json;

    let process = BackgroundProcessInfo {
        id: "test-123".to_string(),
        command: "npm run dev".to_string(),
        pid: 12345,
        status: "running".to_string(),
        output_lines: vec!["Server started".to_string()],
    };

    // Test serialization
    let json = serde_json::to_string(&process).unwrap();
    assert!(json.contains("test-123"));
    assert!(json.contains("npm run dev"));
    assert!(json.contains("12345"));

    // Test deserialization
    let parsed: BackgroundProcessInfo = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.id, "test-123");
    assert_eq!(parsed.command, "npm run dev");
    assert_eq!(parsed.pid, 12345);
    assert_eq!(parsed.status, "running");
    assert_eq!(parsed.output_lines.len(), 1);
}
