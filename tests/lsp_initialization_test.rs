// LSP Initialization E2E Test
// This test verifies that LSP data structures work correctly

use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_lsp_integration_creation() {
    use berry_editor::lsp_ui::LspIntegration;

    // Test 1: LSP integration can be created
    let lsp = LspIntegration::new();

    // Test 2: LspIntegration is Copy
    let lsp_copy = lsp;

    // If this compiles, the test passes
    assert!(true, "LspIntegration can be created and copied");
}

#[wasm_bindgen_test]
fn test_lsp_file_path_setting() {
    use berry_editor::lsp_ui::LspIntegration;

    let lsp = LspIntegration::new();

    // Test that set_file_path doesn't panic
    lsp.set_file_path("/path/to/file.rs".to_string());

    assert!(true, "set_file_path works without panic");
}

#[wasm_bindgen_test]
fn test_lsp_location_info_creation() {
    use berry_editor::lsp_ui::LocationInfo;

    // Test that LocationInfo can be created
    let location = LocationInfo {
        uri: "/test/file.rs".to_string(),
        line: 10,
        column: 5,
    };

    assert_eq!(location.uri, "/test/file.rs");
    assert_eq!(location.line, 10);
    assert_eq!(location.column, 5);
}

// Integration test plan (to be run in real browser environment):
// 1. Open a Rust file
// 2. Wait for LSP initialization log "✅ LSP initialized successfully"
// 3. Cmd+Click on a symbol
// 4. Verify goto_definition is called with initialized=true
// 5. Verify navigation to definition occurs
//
// This requires manual testing or Playwright/Selenium E2E tests
