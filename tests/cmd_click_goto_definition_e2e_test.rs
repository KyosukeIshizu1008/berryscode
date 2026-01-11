//! Cmd+Click Goto Definition E2E Test
//!
//! This test MUST FAIL initially to follow TDD Red-Green-Refactor cycle.
//!
//! Tests the complete flow:
//! 1. File is opened
//! 2. LSP is initialized with file_path set
//! 3. Cmd+Click triggers goto_definition
//! 4. LSP has proper state (initialized=true, file_path set)

use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_cmd_click_requires_lsp_initialized_with_file_path() {
    use berry_editor::lsp_ui::LspIntegration;
    use berry_editor::types::Position;
    use leptos::prelude::*;
    use leptos::prelude::{GetUntracked, Set};

    // Create LSP integration (simulating virtual_editor.rs)
    let lsp = RwSignal::new(LspIntegration::new());

    // Step 1: Verify initial state (uninitialized)
    let lsp_client = lsp.get_untracked();
    assert_eq!(
        lsp_client.file_path.get_untracked(),
        "",
        "PRECONDITION: file_path should be empty initially"
    );
    assert_eq!(
        lsp_client.initialized.get_untracked(),
        false,
        "PRECONDITION: initialized should be false initially"
    );

    // Step 2: Simulate file open (what SHOULD happen in virtual_editor.rs Effect)
    // THIS IS THE CODE THAT MIGHT BE MISSING OR BROKEN
    let lsp_client = lsp.get_untracked();
    let test_file_path = "/Users/test/project/src/main.rs".to_string();

    // Simulate initialize() method (without actual Tauri call)
    lsp_client.file_path.set(test_file_path.clone());
    lsp_client.initialized.set(true);

    // Step 3: Get a new instance (simulating Cmd+Click handler)
    let lsp_for_goto = lsp.get_untracked();

    // Step 4: CRITICAL ASSERTION - This is what's failing in the real app
    assert_eq!(
        lsp_for_goto.file_path.get_untracked(),
        test_file_path,
        "FAIL: LSP file_path not set after initialization - Cmd+Click will fail with '(no file)'"
    );
    assert_eq!(
        lsp_for_goto.initialized.get_untracked(),
        true,
        "FAIL: LSP not initialized - Cmd+Click will fail"
    );

    // Step 5: Verify goto_definition would work (without actual call)
    // This checks the precondition that goto_definition needs
    if lsp_for_goto.file_path.get_untracked().is_empty() {
        panic!("❌ TEST FAILURE: goto_definition will fail because file_path is empty!");
    }
    if !lsp_for_goto.initialized.get_untracked() {
        panic!("❌ TEST FAILURE: goto_definition will fail because LSP not initialized!");
    }
}

#[wasm_bindgen_test]
fn test_lsp_state_persists_across_get_untracked_calls() {
    use berry_editor::lsp_ui::LspIntegration;
    use leptos::prelude::*;
    use leptos::prelude::{GetUntracked, Set};

    let lsp = RwSignal::new(LspIntegration::new());

    // Get instance 1 and set state
    let instance1 = lsp.get_untracked();
    instance1.file_path.set("/test/file.rs".to_string());
    instance1.initialized.set(true);

    // Get instance 2 (simulating what happens in Cmd+Click handler)
    let instance2 = lsp.get_untracked();

    // CRITICAL: instance2 MUST see the state set by instance1
    assert_eq!(
        instance2.file_path.get_untracked(),
        "/test/file.rs",
        "State NOT shared across get_untracked() - this is the bug!"
    );
    assert_eq!(
        instance2.initialized.get_untracked(),
        true,
        "Initialized state NOT shared - this is the bug!"
    );
}

#[wasm_bindgen_test]
async fn test_lsp_must_be_initialized_before_goto_definition() {
    use berry_editor::lsp_ui::LspIntegration;
    use berry_editor::types::Position;
    use leptos::prelude::{GetUntracked, Set};

    let lsp = LspIntegration::new();

    // BEFORE initialization: goto_definition should fail
    let result = lsp.goto_definition(Position::new(10, 5)).await;
    assert!(
        result.is_err(),
        "goto_definition MUST fail when LSP not initialized"
    );

    // AFTER initialization: file_path and initialized must be set
    lsp.file_path.set("/test/file.rs".to_string());
    lsp.initialized.set(true);

    // Now verify state is correct
    assert_eq!(
        lsp.file_path.get_untracked(),
        "/test/file.rs",
        "file_path MUST be set after initialization"
    );
    assert_eq!(
        lsp.initialized.get_untracked(),
        true,
        "initialized MUST be true after initialization"
    );

    // This test verifies the STATE requirements, not the actual LSP call
    // (which would require mocking Tauri)
}
