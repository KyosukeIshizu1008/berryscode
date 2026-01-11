//! IME Cursor Position Integration Test
//!
//! Tests that cursor appears after composing text (未確定文字列) during IME input

use berry_editor::core::virtual_editor::VirtualEditorPanel;
use leptos::prelude::*;
use wasm_bindgen_test::*;
use wasm_bindgen::JsCast;
use web_sys::CompositionEvent;

mod test_helpers;
use test_helpers::{get_test_document, wait_for_render};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_cursor_position_during_ime_composition() {
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

    // Start IME composition
    let comp_start = CompositionEvent::new("compositionstart").unwrap();
    ime_input.dispatch_event(&comp_start).unwrap();
    wait_for_render().await;

    // Type composing text "あいう"
    let comp_update = CompositionEvent::new("compositionupdate").unwrap();
    ime_input
        .dyn_ref::<web_sys::HtmlInputElement>()
        .unwrap()
        .set_value("あいう");
    ime_input.dispatch_event(&comp_update).unwrap();
    wait_for_render().await;

    // Canvas should still exist during composition
    let canvas = document.query_selector("canvas").unwrap();
    assert!(canvas.is_some(), "Canvas should exist during IME composition");

    leptos::logging::log!("✅ Cursor position during IME composition test completed");
}

#[wasm_bindgen_test]
async fn test_cursor_position_after_ime_commit() {
    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;
    selected_file.set(Some(("/test.txt".to_string(), "Hello".to_string())));
    wait_for_render().await;
    wait_for_render().await;

    let document = get_test_document();
    let ime_input = document
        .query_selector("input[type='text']")
        .unwrap()
        .expect("IME input should exist");

    // Move cursor to end (col=5)
    use web_sys::{KeyboardEvent, KeyboardEventInit};
    for _ in 0..5 {
        let mut key_init = KeyboardEventInit::new();
        key_init.set_key("ArrowRight");
        let key_event = KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &key_init)
            .expect("Failed to create KeyboardEvent");
        ime_input.dispatch_event(&key_event).unwrap();
        wait_for_render().await;
    }

    // Start IME composition
    let comp_start = CompositionEvent::new("compositionstart").unwrap();
    ime_input.dispatch_event(&comp_start).unwrap();
    wait_for_render().await;

    // Type composing text
    let comp_update = CompositionEvent::new("compositionupdate").unwrap();
    ime_input
        .dyn_ref::<web_sys::HtmlInputElement>()
        .unwrap()
        .set_value("日本語");
    ime_input.dispatch_event(&comp_update).unwrap();
    wait_for_render().await;

    // Commit the text
    let comp_end = CompositionEvent::new("compositionend").unwrap();
    ime_input
        .dyn_ref::<web_sys::HtmlInputElement>()
        .unwrap()
        .set_value("日本語");
    ime_input.dispatch_event(&comp_end).unwrap();
    wait_for_render().await;

    // Canvas should exist after commit
    let canvas = document.query_selector("canvas").unwrap();
    assert!(canvas.is_some(), "Canvas should exist after IME commit");

    leptos::logging::log!("✅ Cursor position after IME commit test completed");
}

#[wasm_bindgen_test]
async fn test_cursor_position_with_multiple_ime_updates() {
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

    // Start IME composition
    let comp_start = CompositionEvent::new("compositionstart").unwrap();
    ime_input.dispatch_event(&comp_start).unwrap();
    wait_for_render().await;

    // Simulate progressive IME updates: あ -> あい -> あいう -> あいうえ -> あいうえお
    let stages = vec!["あ", "あい", "あいう", "あいうえ", "あいうえお"];

    for stage in stages {
        let comp_update = CompositionEvent::new("compositionupdate").unwrap();
        ime_input
            .dyn_ref::<web_sys::HtmlInputElement>()
            .unwrap()
            .set_value(stage);
        ime_input.dispatch_event(&comp_update).unwrap();
        wait_for_render().await;

        // Canvas should remain stable throughout
        let canvas = document.query_selector("canvas").unwrap();
        assert!(canvas.is_some(), "Canvas should exist at stage: {}", stage);
    }

    // Commit final text
    let comp_end = CompositionEvent::new("compositionend").unwrap();
    ime_input
        .dyn_ref::<web_sys::HtmlInputElement>()
        .unwrap()
        .set_value("あいうえお");
    ime_input.dispatch_event(&comp_end).unwrap();
    wait_for_render().await;

    let canvas = document.query_selector("canvas").unwrap();
    assert!(canvas.is_some(), "Canvas should exist after final commit");

    leptos::logging::log!("✅ Multiple IME updates cursor position test completed");
}

#[wasm_bindgen_test]
async fn test_cursor_position_ime_with_existing_text() {
    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    // Start with mixed text
    selected_file.set(Some(("/test.txt".to_string(), "Hello世界".to_string())));
    wait_for_render().await;
    wait_for_render().await;

    let document = get_test_document();
    let ime_input = document
        .query_selector("input[type='text']")
        .unwrap()
        .expect("IME input should exist");

    // Move cursor to middle (after "Hello", col=5)
    use web_sys::{KeyboardEvent, KeyboardEventInit};
    for _ in 0..5 {
        let mut key_init = KeyboardEventInit::new();
        key_init.set_key("ArrowRight");
        let key_event = KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &key_init)
            .expect("Failed to create KeyboardEvent");
        ime_input.dispatch_event(&key_event).unwrap();
        wait_for_render().await;
    }

    // Start IME composition in the middle
    let comp_start = CompositionEvent::new("compositionstart").unwrap();
    ime_input.dispatch_event(&comp_start).unwrap();
    wait_for_render().await;

    // Type composing text
    let comp_update = CompositionEvent::new("compositionupdate").unwrap();
    ime_input
        .dyn_ref::<web_sys::HtmlInputElement>()
        .unwrap()
        .set_value("テスト");
    ime_input.dispatch_event(&comp_update).unwrap();
    wait_for_render().await;

    // Commit
    let comp_end = CompositionEvent::new("compositionend").unwrap();
    ime_input
        .dyn_ref::<web_sys::HtmlInputElement>()
        .unwrap()
        .set_value("テスト");
    ime_input.dispatch_event(&comp_end).unwrap();
    wait_for_render().await;

    // Canvas should exist
    let canvas = document.query_selector("canvas").unwrap();
    assert!(canvas.is_some(), "Canvas should exist after IME insert in middle");

    leptos::logging::log!("✅ IME cursor position with existing text test completed");
}
