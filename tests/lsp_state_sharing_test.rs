//! LSP State Sharing Test
//!
//! Tests that LspIntegration state (file_path, initialized) is properly shared
//! across different instances obtained via get_untracked()

use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_lsp_integration_state_sharing() {
    use berry_editor::lsp_ui::LspIntegration;
    use leptos::prelude::*;
    use leptos::prelude::{Get, GetUntracked, Set};

    // Create LSP integration wrapped in RwSignal (like in virtual_editor.rs)
    let lsp = RwSignal::new(LspIntegration::new());

    // Initial state should be uninitialized
    let lsp_client_1 = lsp.get_untracked();
    assert_eq!(lsp_client_1.file_path.get_untracked(), "", "Initial file_path should be empty");
    assert_eq!(lsp_client_1.initialized.get_untracked(), false, "Initial initialized should be false");

    // Simulate setting file_path (like in initialize())
    let lsp_client_2 = lsp.get_untracked();
    lsp_client_2.file_path.set("/test/file.rs".to_string());
    lsp_client_2.initialized.set(true);

    // Get a new instance and verify state is shared
    let lsp_client_3 = lsp.get_untracked();
    assert_eq!(
        lsp_client_3.file_path.get_untracked(),
        "/test/file.rs",
        "file_path should be shared across instances"
    );
    assert_eq!(
        lsp_client_3.initialized.get_untracked(),
        true,
        "initialized should be shared across instances"
    );

    // Verify original instance also sees the change
    assert_eq!(
        lsp_client_1.file_path.get_untracked(),
        "/test/file.rs",
        "Original instance should see state changes"
    );
    assert_eq!(
        lsp_client_1.initialized.get_untracked(),
        true,
        "Original instance should see initialized=true"
    );
}

#[wasm_bindgen_test]
fn test_lsp_integration_copy_trait() {
    use berry_editor::lsp_ui::LspIntegration;
    use leptos::prelude::*;
    use leptos::prelude::{GetUntracked, Set};

    let lsp = LspIntegration::new();

    // Test that LspIntegration is Copy
    let lsp_copy = lsp;

    // Set state on original
    lsp.file_path.set("/test/file.rs".to_string());
    lsp.initialized.set(true);

    // Verify copy sees the same state (because RwSignals are shared)
    assert_eq!(
        lsp_copy.file_path.get_untracked(),
        "/test/file.rs",
        "Copy should share RwSignals with original"
    );
    assert_eq!(
        lsp_copy.initialized.get_untracked(),
        true,
        "Copy should share initialized state"
    );
}

#[wasm_bindgen_test]
fn test_lsp_file_path_before_and_after_init() {
    use berry_editor::lsp_ui::LspIntegration;
    use leptos::prelude::{GetUntracked, Set};

    let lsp = LspIntegration::new();

    // BEFORE initialization: file_path should be empty
    assert_eq!(
        lsp.file_path.get_untracked(),
        "",
        "file_path should be empty before initialization"
    );

    // Simulate what initialize() does (without async)
    lsp.file_path.set("/test/file.rs".to_string());
    lsp.initialized.set(true);

    // AFTER initialization: file_path should be set
    assert_eq!(
        lsp.file_path.get_untracked(),
        "/test/file.rs",
        "file_path should be set after initialization"
    );
    assert_eq!(
        lsp.initialized.get_untracked(),
        true,
        "initialized should be true after initialization"
    );
}
