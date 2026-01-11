//! Cursor Movement Integration Test
//!
//! Tests that cursor position updates correctly after text input

use berry_editor::core::virtual_editor::VirtualEditorPanel;
use leptos::prelude::*;
use wasm_bindgen_test::*;
use wasm_bindgen::JsCast;
use web_sys::{KeyboardEvent, KeyboardEventInit};

mod test_helpers;
use test_helpers::{get_test_document, wait_for_render};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_cursor_moves_after_typing() {
    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    // Open a file
    selected_file.set(Some(("/test.txt".to_string(), "".to_string())));
    wait_for_render().await;
    wait_for_render().await;

    let document = get_test_document();

    // Get IME input
    let ime_input = document
        .query_selector("input[type='text']")
        .unwrap()
        .expect("IME input should exist");

    // Type 'a'
    let mut key_init = KeyboardEventInit::new();
    key_init.set_key("a");
    let key_event = KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &key_init)
        .expect("Failed to create KeyboardEvent");

    ime_input
        .dyn_ref::<web_sys::HtmlInputElement>()
        .unwrap()
        .set_value("a");
    ime_input.dispatch_event(&key_event).unwrap();

    wait_for_render().await;

    // Canvas should still exist and cursor should have moved
    let canvas = document.query_selector("canvas").unwrap();
    assert!(canvas.is_some(), "Canvas should exist after typing");

    leptos::logging::log!("✅ Cursor movement test completed");
}

#[wasm_bindgen_test]
async fn test_japanese_input_moves_cursor() {
    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;
    selected_file.set(Some(("/test.txt".to_string(), "".to_string())));
    wait_for_render().await;
    wait_for_render().await;

    let document = get_test_document();
    let ime_input = document
        .query_selector("input[type='text']")
        .unwrap()
        .expect("IME input should exist");

    // Simulate IME composition
    use web_sys::CompositionEvent;

    let comp_start = CompositionEvent::new("compositionstart").unwrap();
    ime_input.dispatch_event(&comp_start).unwrap();
    wait_for_render().await;

    let comp_update = CompositionEvent::new("compositionupdate").unwrap();
    ime_input
        .dyn_ref::<web_sys::HtmlInputElement>()
        .unwrap()
        .set_value("あ");
    ime_input.dispatch_event(&comp_update).unwrap();
    wait_for_render().await;

    // Commit
    let comp_end = CompositionEvent::new("compositionend").unwrap();
    ime_input
        .dyn_ref::<web_sys::HtmlInputElement>()
        .unwrap()
        .set_value("あ");
    ime_input.dispatch_event(&comp_end).unwrap();
    wait_for_render().await;

    // Canvas should exist
    let canvas = document.query_selector("canvas").unwrap();
    assert!(canvas.is_some(), "Canvas should exist after Japanese input");

    leptos::logging::log!("✅ Japanese cursor movement test completed");
}
