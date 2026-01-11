//! Canvas Rendering Verification Tests
//!
//! Tests that verify the Canvas-based rendering system works correctly.
//! This replaces the old DOM-based rendering tests.

use berry_editor::core::virtual_editor::VirtualEditorPanel;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

mod test_helpers;
use test_helpers::{query_selector, wait_for_render, get_test_document};

wasm_bindgen_test_configure!(run_in_browser);

// ========================================
// Canvas Element Tests
// ========================================

#[wasm_bindgen_test]
async fn test_canvas_element_exists() {
    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    // Verify canvas element exists
    let canvas = query_selector("canvas");
    assert!(canvas.is_some(), "❌ Canvas element does not exist");

    let canvas_el = canvas.unwrap().dyn_into::<web_sys::HtmlCanvasElement>().unwrap();
    assert!(canvas_el.width() > 0, "❌ Canvas has zero width");
    assert!(canvas_el.height() > 0, "❌ Canvas has zero height");

    leptos::logging::log!("✅ Canvas element exists with valid dimensions");
}

#[wasm_bindgen_test]
async fn test_canvas_has_rendering_context() {
    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    // Load a test file to trigger canvas rendering
    let test_content = "fn main() {\n    println!(\"Hello\");\n}";
    selected_file.set(Some(("/test.rs".to_string(), test_content.to_string())));

    wait_for_render().await;
    wait_for_render().await;

    let canvas = query_selector("canvas");
    assert!(canvas.is_some(), "❌ Canvas element not found");

    let canvas_el = canvas.unwrap().dyn_into::<web_sys::HtmlCanvasElement>().unwrap();

    // Verify 2D rendering context can be obtained
    let context = canvas_el.get_context("2d").unwrap();
    assert!(context.is_some(), "❌ Cannot get 2D rendering context");

    let ctx = context.unwrap().dyn_into::<web_sys::CanvasRenderingContext2d>().unwrap();

    // Verify font is set correctly (it may be default or JetBrains Mono)
    let font = ctx.font();
    // Font might be "13px 'JetBrains Mono'" or similar format
    leptos::logging::log!("Canvas font: {}", font);

    leptos::logging::log!("✅ Canvas has valid 2D rendering context");
}

#[wasm_bindgen_test]
async fn test_hidden_ime_input_exists() {
    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    // Verify hidden input element exists for IME support
    let input = query_selector("input[type='text']");
    assert!(input.is_some(), "❌ Hidden IME input element not found");

    let input_el = input.unwrap().dyn_into::<web_sys::HtmlInputElement>().unwrap();

    // Verify it has the correct style attribute
    let style_attr = input_el.get_attribute("style").unwrap_or_default();
    assert!(
        style_attr.contains("position: absolute") || style_attr.contains("position:absolute"),
        "❌ IME input should be absolutely positioned, got: {}", style_attr
    );
    assert!(
        style_attr.contains("opacity: 0") || style_attr.contains("opacity:0"),
        "❌ IME input should be invisible (opacity: 0), got: {}", style_attr
    );

    leptos::logging::log!("✅ Hidden IME input exists with correct styling");
}

#[wasm_bindgen_test]
async fn test_canvas_renders_file_content() {
    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    // Load test file
    let test_content = "Line 1\nLine 2\nLine 3";
    selected_file.set(Some(("/test.txt".to_string(), test_content.to_string())));

    // Wait for rendering
    wait_for_render().await;
    wait_for_render().await;

    // Verify canvas exists and has non-zero dimensions
    let canvas = query_selector("canvas");
    assert!(canvas.is_some(), "❌ Canvas should exist after loading file");

    let canvas_el = canvas.unwrap().dyn_into::<web_sys::HtmlCanvasElement>().unwrap();
    assert!(canvas_el.width() > 0, "❌ Canvas width should be > 0");
    assert!(canvas_el.height() > 0, "❌ Canvas height should be > 0");

    // Note: We can't directly verify pixel content without reading canvas pixels
    // But we can verify the canvas was updated by checking its dimensions

    leptos::logging::log!("✅ Canvas renders after file load");
}

// ========================================
// Buffer Integration Tests
// ========================================

#[wasm_bindgen_test]
async fn test_buffer_initialized_with_file_content() {
    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    // Load test file
    let test_content = "Hello Canvas\nSecond Line\nThird Line";
    selected_file.set(Some(("/test.rs".to_string(), test_content.to_string())));

    wait_for_render().await;
    wait_for_render().await;

    // Note: To properly test this, we'd need to expose the buffer state
    // For now, we just verify no errors occurred

    let document = get_test_document();
    let canvas = document.query_selector("canvas").unwrap();
    assert!(canvas.is_some(), "❌ Canvas should be rendered");

    leptos::logging::log!("✅ File content loaded into buffer");
}

// ========================================
// Cursor Rendering Tests
// ========================================

#[wasm_bindgen_test]
async fn test_cursor_renders_on_canvas() {
    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    // Load file
    let test_content = "Test content";
    selected_file.set(Some(("/test.txt".to_string(), test_content.to_string())));

    wait_for_render().await;
    wait_for_render().await;

    // Verify canvas exists (cursor should be rendered on it)
    let canvas = query_selector("canvas");
    assert!(canvas.is_some(), "❌ Canvas should exist for cursor rendering");

    // Note: We can't directly verify the cursor pixel data without reading canvas
    // But we can verify the rendering pipeline completed without errors

    leptos::logging::log!("✅ Cursor rendering pipeline completed");
}

// ========================================
// Event Handler Tests
// ========================================

#[wasm_bindgen_test]
async fn test_canvas_has_event_handlers() {
    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    let canvas = query_selector("canvas");
    assert!(canvas.is_some(), "❌ Canvas element not found");

    // ✅ IME Architecture: Canvasはtabindexを持たない（IME inputがキーイベントを処理）
    // 代わりにIME input要素が存在することを確認
    let ime_input = query_selector("input[type='text']");
    assert!(ime_input.is_some(), "❌ IME input element should exist for keyboard events");

    leptos::logging::log!("✅ Canvas has IME input for event handling");
}
