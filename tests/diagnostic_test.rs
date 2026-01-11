//! Complete Diagnostic Test
//!
//! This test diagnoses every step from file selection to rendering
//! Run with: wasm-pack test --headless --chrome --test diagnostic_test

use berry_editor::core::virtual_editor::VirtualEditorPanel;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;
use web_sys::HtmlElement;

// ✅ Use test helpers instead of web_sys directly
mod test_helpers;
use test_helpers::{get_test_document, get_test_window, wait_for_render};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn diagnostic_full_flow() {
    leptos::logging::log!("========== DIAGNOSTIC TEST START ==========");

    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    let document = get_test_document();
    let window = get_test_window();

    // ========== STEP 1: Check Initial State ==========
    leptos::logging::log!("STEP 1: Checking initial state...");

    let editor_main = document.query_selector(".berry-editor-main").unwrap();
    if editor_main.is_some() {
        leptos::logging::log!("✅ Editor main container exists");
    } else {
        leptos::logging::log!("❌ Editor main container NOT found");
        panic!("Editor main container not found");
    }

    // Note: VirtualEditorPanel doesn't render tab-bar - that's in parent Editor component
    // We're testing the Canvas rendering panel here

    // ========== STEP 2: Simulate File Selection ==========
    leptos::logging::log!("STEP 2: Simulating file selection...");

    let test_content = "fn main() {\n    println!(\"Hello, world!\");\n}\n";
    leptos::logging::log!("Setting file: /test.rs with {} chars", test_content.len());

    selected_file.set(Some(("/test.rs".to_string(), test_content.to_string())));

    leptos::logging::log!("Waiting for effects to propagate...");
    wait_for_render().await;
    wait_for_render().await;
    wait_for_render().await;

    // ========== STEP 3: Check Main Container After File Load ==========
    leptos::logging::log!("STEP 3: Checking main container after file load...");

    let main = document.query_selector(".berry-editor-main").unwrap();
    if let Some(main_el) = main {
        let main_html = main_el.dyn_into::<web_sys::HtmlElement>().unwrap();
        let inner_html = main_html.inner_html();
        leptos::logging::log!("Main container HTML length: {}", inner_html.len());

        if inner_html.len() < 200 {
            leptos::logging::log!("Main HTML: {}", inner_html);
        } else {
            leptos::logging::log!("Main HTML (first 300 chars): {}", &inner_html[..300]);
        }
    }

    // ========== STEP 6: Check Main Container ==========
    leptos::logging::log!("STEP 6: Checking main container...");

    let main_container = document.query_selector(".berry-editor-main").unwrap();
    if let Some(main_el) = main_container {
        let container = main_el.dyn_into::<web_sys::HtmlElement>().unwrap();
        let rect = container.get_bounding_client_rect();

        leptos::logging::log!("✅ Main container: top={}, left={}, width={}, height={}",
            rect.top(), rect.left(), rect.width(), rect.height());

        let child_count = container.children().length();
        leptos::logging::log!("Main container children: {}", child_count);
    } else {
        leptos::logging::log!("❌ Main container NOT found");
    }

    // ========== STEP 7: Check Canvas Rendering Elements ==========
    leptos::logging::log!("STEP 7: Checking Canvas rendering elements...");

    // ✅ Canvas Architecture: Check for canvas and hidden IME input
    let canvas = document.query_selector("canvas").unwrap();
    let hidden_input = document.query_selector("input[type='text']").unwrap();

    let has_canvas = canvas.is_some();
    let has_hidden_input = hidden_input.is_some();

    if has_canvas && has_hidden_input {
        leptos::logging::log!("✅ Canvas rendering elements created (canvas + hidden input)");

        // Check canvas element
        let canvas_el = canvas.unwrap().dyn_into::<web_sys::HtmlCanvasElement>().unwrap();
        let rect = canvas_el.get_bounding_client_rect();

        leptos::logging::log!("Canvas dimensions: width={}, height={}",
            canvas_el.width(), canvas_el.height());
        leptos::logging::log!("Canvas position: top={}, left={}, width={}, height={}",
            rect.top(), rect.left(), rect.width(), rect.height());

        // Check if canvas is visible in viewport
        let viewport_height = window.inner_height().unwrap().as_f64().unwrap();
        if rect.top() >= 0.0 && rect.top() < viewport_height {
            leptos::logging::log!("✅ Canvas IS in viewport (0-{}px)", viewport_height);
        } else {
            leptos::logging::log!("❌ Canvas is OUTSIDE viewport! Position: {}px, Viewport: 0-{}px",
                rect.top(), viewport_height);
        }

        // Verify canvas has rendering context
        let ctx = canvas_el.get_context("2d").unwrap();
        if ctx.is_some() {
            leptos::logging::log!("✅ Canvas has 2D rendering context");
        } else {
            leptos::logging::log!("❌ Canvas missing 2D rendering context");
        }
    } else {
        leptos::logging::log!("❌ NO CANVAS RENDERING ELEMENTS CREATED! canvas: {}, hidden_input: {}",
            has_canvas, has_hidden_input);
    }

    // ========== STEP 8: DOM Tree Structure ==========
    leptos::logging::log!("STEP 8: Checking DOM tree structure...");

    if let Some(main) = document.query_selector(".berry-editor-main").unwrap() {
        let main_el = main.dyn_into::<web_sys::HtmlElement>().unwrap();
        print_dom_tree(&main_el, 0);
    }

    leptos::logging::log!("========== DIAGNOSTIC TEST COMPLETE ==========");

    // Final assertion - ✅ Canvas Architecture: Verify canvas and hidden input exist
    assert!(has_canvas, "❌ CRITICAL: No canvas element was created");
    assert!(has_hidden_input, "❌ CRITICAL: No hidden IME input was created");
}

fn print_dom_tree(element: &HtmlElement, depth: usize) {
    let indent = "  ".repeat(depth);
    let tag_name = element.tag_name().to_lowercase();
    let class_name = element.class_name();

    let info = if class_name.is_empty() {
        format!("{}<{}>", indent, tag_name)
    } else {
        format!("{}<{} class='{}'>", indent, tag_name, class_name)
    };

    leptos::logging::log!("{}", info);

    if depth < 3 {  // Limit depth to avoid too much output
        let children = element.children();
        for i in 0..children.length().min(5) {  // Show first 5 children
            if let Some(child) = children.item(i) {
                if let Ok(child_el) = child.dyn_into::<web_sys::HtmlElement>() {
                    print_dom_tree(&child_el, depth + 1);
                }
            }
        }
        if children.length() > 5 {
            leptos::logging::log!("{}  ... and {} more children", indent, children.length() - 5);
        }
    }
}
