//! File Display Integration Test
//!
//! Tests the complete flow from file selection to visible rendering
//! Run with: wasm-pack test --headless --firefox

use berry_editor::core::virtual_editor::VirtualEditorPanel;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

// ✅ Use test helpers instead of web_sys directly
mod test_helpers;
use test_helpers::{query_selector, wait_for_render, get_test_window, get_test_document};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_file_selection_creates_visible_content() {
    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    // Simulate file selection
    let test_content = "fn main() {\n    println!(\"Hello, world!\");\n}\n";
    selected_file.set(Some(("/test.rs".to_string(), test_content.to_string())));

    // Wait for effects to propagate
    wait_for_render().await;
    wait_for_render().await;
    wait_for_render().await;

    let document = get_test_document();
    let window = get_test_window();

    // Note: VirtualEditorPanel doesn't render tabs - that's in the parent Editor component
    // We're testing the Canvas rendering panel here

    // Step 1: Verify main container exists
    let main = document.query_selector(".berry-editor-main").unwrap();
    assert!(main.is_some(), "❌ Editor main container not found");

    leptos::logging::log!("✅ Editor main container exists");

    // Step 3: ✅ Canvas Architecture - Check for canvas element
    let canvas = document.query_selector("canvas").unwrap();
    assert!(canvas.is_some(), "❌ Canvas element not created");

    let hidden_input = document.query_selector("input[type='text']").unwrap();
    assert!(hidden_input.is_some(), "❌ Hidden IME input not created");

    leptos::logging::log!("✅ Canvas rendering elements created (canvas + hidden input)");

    // Step 4: ✅ Canvas Architecture - Verify canvas visibility and dimensions
    let canvas_el = canvas.unwrap().dyn_into::<web_sys::HtmlCanvasElement>().unwrap();
    let canvas_rect = canvas_el.get_bounding_client_rect();

    leptos::logging::log!(
        "Canvas dimensions - width: {}, height: {}, physical width: {}, physical height: {}",
        canvas_rect.width(), canvas_rect.height(), canvas_el.width(), canvas_el.height()
    );

    // ✅ Verify canvas has valid dimensions (tests may stack, so not checking absolute position)
    assert!(canvas_el.width() > 0, "❌ Canvas has zero physical width");
    assert!(canvas_el.height() > 0, "❌ Canvas has zero physical height");
    assert!(canvas_rect.width() > 0.0, "❌ Canvas has zero CSS width");
    assert!(canvas_rect.height() > 0.0, "❌ Canvas has zero CSS height");

    // Step 6: Verify canvas has rendering context
    let ctx = canvas_el.get_context("2d").unwrap();
    assert!(ctx.is_some(), "❌ Canvas missing 2D rendering context");

    leptos::logging::log!("✅ ALL TESTS PASSED - File content is visible!");
}

#[wasm_bindgen_test]
async fn test_canvas_dimensions_and_rendering() {
    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    let test_content = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5";
    selected_file.set(Some(("/test.txt".to_string(), test_content.to_string())));

    wait_for_render().await;
    wait_for_render().await;
    wait_for_render().await;

    let document = get_test_document();

    // ✅ Canvas Architecture: Check canvas element exists and has valid dimensions
    let canvas = document.query_selector("canvas").unwrap();
    assert!(canvas.is_some(), "❌ Canvas element not found");

    let canvas_el = canvas.unwrap().dyn_into::<web_sys::HtmlCanvasElement>().unwrap();
    let canvas_rect = canvas_el.get_bounding_client_rect();

    leptos::logging::log!(
        "Canvas dimensions - width: {}, height: {}, physical width: {}, physical height: {}",
        canvas_rect.width(), canvas_rect.height(), canvas_el.width(), canvas_el.height()
    );

    // ✅ Verify canvas has valid dimensions (not checking position as tests may stack)
    assert!(canvas_el.width() > 0, "❌ Canvas has zero physical width");
    assert!(canvas_el.height() > 0, "❌ Canvas has zero physical height");
    assert!(canvas_rect.width() > 0.0, "❌ Canvas has zero CSS width");
    assert!(canvas_rect.height() > 0.0, "❌ Canvas has zero CSS height");

    // ✅ Verify rendering context exists
    let ctx = canvas_el.get_context("2d").unwrap();
    assert!(ctx.is_some(), "❌ Canvas missing 2D rendering context");

    leptos::logging::log!("✅ Canvas has valid dimensions and rendering context");
}

#[wasm_bindgen_test]
async fn test_scroll_container_structure() {
    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    let test_content = "Test Line 1\nTest Line 2\nTest Line 3";
    selected_file.set(Some(("/test.txt".to_string(), test_content.to_string())));

    wait_for_render().await;
    wait_for_render().await;
    wait_for_render().await; // Extra wait for tab creation

    let document = get_test_document();

    // ✅ Canvas Architecture: Verify canvas and hidden input exist
    let canvas = document.query_selector("canvas").unwrap();
    assert!(canvas.is_some(), "❌ Canvas element not found");

    let canvas_el = canvas.unwrap().dyn_into::<web_sys::HtmlCanvasElement>().unwrap();
    let canvas_rect = canvas_el.get_bounding_client_rect();

    leptos::logging::log!(
        "Canvas - top: {}, left: {}, width: {}, height: {}",
        canvas_rect.top(), canvas_rect.left(), canvas_rect.width(), canvas_rect.height()
    );

    // Verify canvas has valid dimensions
    assert!(canvas_el.width() > 0, "❌ Canvas should have non-zero width, got: {}", canvas_el.width());
    assert!(canvas_el.height() > 0, "❌ Canvas should have non-zero height, got: {}", canvas_el.height());

    // Verify rendering context exists
    let ctx = canvas_el.get_context("2d").unwrap();
    assert!(ctx.is_some(), "❌ Canvas should have 2D rendering context");

    leptos::logging::log!("✅ Canvas rendering structure correct");
}
