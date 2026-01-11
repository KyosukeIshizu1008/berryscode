//! Multiple Tabs Integration Test
//!
//! Tests for tab creation, switching, and closing to identify crash issues

use berry_editor::core::virtual_editor::VirtualEditorPanel;
use leptos::prelude::*;
use wasm_bindgen_test::*;
use wasm_bindgen::JsCast;

mod test_helpers;
use test_helpers::{get_test_document, wait_for_render};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_open_single_file() {
    // Clean up DOM from previous tests
    get_test_document().body().unwrap().set_inner_html("");

    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    leptos::logging::log!("📝 Opening first file...");
    selected_file.set(Some(("/file1.rs".to_string(), "content1".to_string())));
    wait_for_render().await;
    wait_for_render().await;

    let document = get_test_document();
    let tabs = document.query_selector_all(".berry-tab").unwrap();

    leptos::logging::log!("✅ Single file opened, tabs count: {}", tabs.length());
    assert_eq!(tabs.length(), 1, "Should have 1 tab");
}

#[wasm_bindgen_test]
async fn test_open_two_files() {
    // Clean up DOM from previous tests
    get_test_document().body().unwrap().set_inner_html("");

    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    leptos::logging::log!("📝 Opening first file...");
    selected_file.set(Some(("/file1.rs".to_string(), "content1".to_string())));
    wait_for_render().await;
    wait_for_render().await;

    let document = get_test_document();
    let tabs_after_first = document.query_selector_all(".berry-tab").unwrap();
    leptos::logging::log!("After first file: {} tabs", tabs_after_first.length());

    leptos::logging::log!("📝 Opening second file...");
    selected_file.set(Some(("/file2.rs".to_string(), "content2".to_string())));
    wait_for_render().await;
    wait_for_render().await;

    let tabs_after_second = document.query_selector_all(".berry-tab").unwrap();
    leptos::logging::log!("After second file: {} tabs", tabs_after_second.length());

    assert_eq!(tabs_after_second.length(), 2, "Should have 2 tabs after opening second file");
}

#[wasm_bindgen_test]
async fn test_open_three_files_sequentially() {
    // Clean up DOM from previous tests
    get_test_document().body().unwrap().set_inner_html("");

    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    // Open files one by one with logging
    for i in 1..=3 {
        leptos::logging::log!("📝 Opening file {}...", i);
        selected_file.set(Some((format!("/file{}.rs", i), format!("content{}", i))));
        wait_for_render().await;
        wait_for_render().await;

        let document = get_test_document();
        let tabs = document.query_selector_all(".berry-tab").unwrap();
        leptos::logging::log!("After file {}: {} tabs", i, tabs.length());

        assert_eq!(tabs.length() as usize, i, "Should have {} tab(s) after opening file {}", i, i);
    }

    leptos::logging::log!("✅ All 3 files opened successfully");
}

#[wasm_bindgen_test]
async fn test_click_tab_to_switch() {
    // Clean up DOM from previous tests
    get_test_document().body().unwrap().set_inner_html("");

    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    // Open two files
    selected_file.set(Some(("/file1.rs".to_string(), "content1".to_string())));
    wait_for_render().await;
    wait_for_render().await;

    selected_file.set(Some(("/file2.rs".to_string(), "content2".to_string())));
    wait_for_render().await;
    wait_for_render().await;

    let document = get_test_document();

    // Get all tabs
    let tabs = document.query_selector_all(".berry-tab").unwrap();
    assert_eq!(tabs.length(), 2);

    // Click on first tab
    leptos::logging::log!("📝 Clicking on first tab...");
    let first_tab = tabs.item(0).unwrap();
    let first_tab_el = first_tab.dyn_into::<web_sys::HtmlElement>().unwrap();
    first_tab_el.click();

    wait_for_render().await;

    // Check which tab is active
    let active_tabs = document.query_selector_all(".berry-tab.active").unwrap();
    leptos::logging::log!("Active tabs count: {}", active_tabs.length());

    assert_eq!(active_tabs.length(), 1, "Should have exactly 1 active tab");

    leptos::logging::log!("✅ Tab switching works");
}

#[wasm_bindgen_test]
async fn test_close_tab_button() {
    // Clean up DOM from previous tests
    get_test_document().body().unwrap().set_inner_html("");

    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    // Open two files
    selected_file.set(Some(("/file1.rs".to_string(), "content1".to_string())));
    wait_for_render().await;
    wait_for_render().await;

    selected_file.set(Some(("/file2.rs".to_string(), "content2".to_string())));
    wait_for_render().await;
    wait_for_render().await;

    let document = get_test_document();

    // Verify 2 tabs exist
    let tabs_before = document.query_selector_all(".berry-tab").unwrap();
    assert_eq!(tabs_before.length(), 2);

    // Click close button on first tab
    leptos::logging::log!("📝 Clicking close button on first tab...");
    let close_button = document
        .query_selector(".berry-tab button")
        .unwrap()
        .expect("Close button should exist");

    let button_el = close_button.dyn_into::<web_sys::HtmlElement>().unwrap();
    button_el.click();

    wait_for_render().await;

    // Check tabs count
    let tabs_after = document.query_selector_all(".berry-tab").unwrap();
    leptos::logging::log!("After closing: {} tabs", tabs_after.length());

    assert_eq!(tabs_after.length(), 1, "Should have 1 tab after closing one");

    leptos::logging::log!("✅ Tab closing works");
}

#[wasm_bindgen_test]
async fn test_type_text_in_multiple_tabs() {
    // Clean up DOM from previous tests
    get_test_document().body().unwrap().set_inner_html("");

    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    // Open first file
    selected_file.set(Some(("/file1.rs".to_string(), "".to_string())));
    wait_for_render().await;
    wait_for_render().await;

    let document = get_test_document();
    let ime_input = document
        .query_selector("input[type='text']")
        .unwrap()
        .expect("IME input should exist");

    // Type in first file
    use web_sys::{KeyboardEvent, KeyboardEventInit};
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

    leptos::logging::log!("✅ Typed 'a' in first file");

    // Open second file
    selected_file.set(Some(("/file2.rs".to_string(), "".to_string())));
    wait_for_render().await;
    wait_for_render().await;

    // Type in second file
    ime_input
        .dyn_ref::<web_sys::HtmlInputElement>()
        .unwrap()
        .set_value("b");
    ime_input.dispatch_event(&key_event).unwrap();
    wait_for_render().await;

    leptos::logging::log!("✅ Typed 'b' in second file");

    // Both tabs should still exist
    let tabs = document.query_selector_all(".berry-tab").unwrap();
    assert_eq!(tabs.length(), 2, "Should still have 2 tabs after typing");

    leptos::logging::log!("✅ Typing in multiple tabs works");
}
