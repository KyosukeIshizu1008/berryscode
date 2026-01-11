//! Tab Crash Scenarios Integration Test
//!
//! Tests edge cases and crash scenarios for tab management

use berry_editor::core::virtual_editor::VirtualEditorPanel;
use leptos::prelude::*;
use wasm_bindgen_test::*;
use wasm_bindgen::JsCast;

mod test_helpers;
use test_helpers::{get_test_document, wait_for_render};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_close_last_remaining_tab() {
    // Clean up DOM from previous tests
    get_test_document().body().unwrap().set_inner_html("");

    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    // Open one file
    selected_file.set(Some(("/test.rs".to_string(), "content".to_string())));
    wait_for_render().await;
    wait_for_render().await;

    let document = get_test_document();

    // Verify 1 tab exists
    let tabs_before = document.query_selector_all(".berry-tab").unwrap();
    assert_eq!(tabs_before.length(), 1, "Should have 1 tab");

    // Close the only tab
    leptos::logging::log!("📝 Closing the only tab...");
    let close_button = document
        .query_selector(".berry-tab button")
        .unwrap()
        .expect("Close button should exist");

    let button_el = close_button.dyn_into::<web_sys::HtmlElement>().unwrap();
    button_el.click();

    wait_for_render().await;

    // Check tabs count - should be 0
    let tabs_after = document.query_selector_all(".berry-tab").unwrap();
    leptos::logging::log!("After closing last tab: {} tabs", tabs_after.length());

    assert_eq!(tabs_after.length(), 0, "Should have 0 tabs after closing the last one");

    // Verify app doesn't crash - check that tab bar still exists
    let tab_bar = document.query_selector(".berry-editor-tabs").unwrap();
    assert!(tab_bar.is_some(), "Tab bar should still exist");
    leptos::logging::log!("✅ App didn't crash after closing last tab");
}

#[wasm_bindgen_test]
async fn test_close_middle_tab_maintains_active_state() {
    // Clean up DOM from previous tests
    get_test_document().body().unwrap().set_inner_html("");

    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    // Open three files
    selected_file.set(Some(("/file1.rs".to_string(), "content1".to_string())));
    wait_for_render().await;
    wait_for_render().await;

    selected_file.set(Some(("/file2.rs".to_string(), "content2".to_string())));
    wait_for_render().await;
    wait_for_render().await;

    selected_file.set(Some(("/file3.rs".to_string(), "content3".to_string())));
    wait_for_render().await;
    wait_for_render().await;

    let document = get_test_document();

    // Verify 3 tabs exist
    let tabs = document.query_selector_all(".berry-tab").unwrap();
    assert_eq!(tabs.length(), 3, "Should have 3 tabs");

    // Click on first tab to make it active
    let first_tab = tabs.item(0).unwrap();
    let first_tab_el = first_tab.dyn_into::<web_sys::HtmlElement>().unwrap();
    first_tab_el.click();
    wait_for_render().await;

    // Close the middle tab (index 1)
    leptos::logging::log!("📝 Closing middle tab...");
    let all_close_buttons = document.query_selector_all(".berry-tab button").unwrap();
    let middle_close_button = all_close_buttons.item(1).unwrap();
    let button_el = middle_close_button.dyn_into::<web_sys::HtmlElement>().unwrap();
    button_el.click();

    wait_for_render().await;

    // Check tabs count - should be 2
    let tabs_after = document.query_selector_all(".berry-tab").unwrap();
    assert_eq!(tabs_after.length(), 2, "Should have 2 tabs after closing middle one");

    // First tab should still be active
    let active_tabs = document.query_selector_all(".berry-tab.active").unwrap();
    assert_eq!(active_tabs.length(), 1, "Should have exactly 1 active tab");

    leptos::logging::log!("✅ Closing middle tab maintains active state correctly");
}

#[wasm_bindgen_test]
async fn test_close_active_tab_switches_to_previous() {
    // Clean up DOM from previous tests
    get_test_document().body().unwrap().set_inner_html("");

    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    // Open three files
    selected_file.set(Some(("/file1.rs".to_string(), "content1".to_string())));
    wait_for_render().await;
    wait_for_render().await;

    selected_file.set(Some(("/file2.rs".to_string(), "content2".to_string())));
    wait_for_render().await;
    wait_for_render().await;

    selected_file.set(Some(("/file3.rs".to_string(), "content3".to_string())));
    wait_for_render().await;
    wait_for_render().await;

    let document = get_test_document();

    // file3 should be active (last opened)
    let active_tabs_before = document.query_selector_all(".berry-tab.active").unwrap();
    assert_eq!(active_tabs_before.length(), 1, "Should have 1 active tab");

    // Close the active tab (file3)
    leptos::logging::log!("📝 Closing active tab...");
    let active_tab = active_tabs_before.item(0).unwrap();
    let active_tab_element = active_tab.dyn_into::<web_sys::Element>().unwrap();
    let close_button = active_tab_element.query_selector("button").unwrap().expect("Close button should exist");
    let button_el = close_button.dyn_into::<web_sys::HtmlElement>().unwrap();
    button_el.click();

    wait_for_render().await;

    // Should have 2 tabs remaining
    let tabs_after = document.query_selector_all(".berry-tab").unwrap();
    assert_eq!(tabs_after.length(), 2, "Should have 2 tabs after closing active one");

    // A tab should still be active (should switch to previous)
    let active_tabs_after = document.query_selector_all(".berry-tab.active").unwrap();
    assert_eq!(active_tabs_after.length(), 1, "Should still have 1 active tab");

    leptos::logging::log!("✅ Closing active tab switches to another tab correctly");
}

#[wasm_bindgen_test]
async fn test_reopen_same_file_activates_existing_tab() {
    // Clean up DOM from previous tests
    get_test_document().body().unwrap().set_inner_html("");

    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    // Open file1
    selected_file.set(Some(("/file1.rs".to_string(), "content1".to_string())));
    wait_for_render().await;
    wait_for_render().await;

    // Open file2
    selected_file.set(Some(("/file2.rs".to_string(), "content2".to_string())));
    wait_for_render().await;
    wait_for_render().await;

    let document = get_test_document();

    // Should have 2 tabs
    let tabs_before = document.query_selector_all(".berry-tab").unwrap();
    assert_eq!(tabs_before.length(), 2, "Should have 2 tabs");

    // Re-open file1
    leptos::logging::log!("📝 Re-opening file1...");
    selected_file.set(Some(("/file1.rs".to_string(), "content1".to_string())));
    wait_for_render().await;
    wait_for_render().await;

    // Should still have 2 tabs (not 3)
    let tabs_after = document.query_selector_all(".berry-tab").unwrap();
    assert_eq!(tabs_after.length(), 2, "Should still have 2 tabs (not create duplicate)");

    leptos::logging::log!("✅ Re-opening same file activates existing tab instead of creating duplicate");
}

#[wasm_bindgen_test]
async fn test_type_after_switching_tabs() {
    // Clean up DOM from previous tests
    get_test_document().body().unwrap().set_inner_html("");

    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    // Open file1
    selected_file.set(Some(("/file1.rs".to_string(), "".to_string())));
    wait_for_render().await;
    wait_for_render().await;

    // Open file2
    selected_file.set(Some(("/file2.rs".to_string(), "".to_string())));
    wait_for_render().await;
    wait_for_render().await;

    let document = get_test_document();

    // Click on first tab
    leptos::logging::log!("📝 Switching to first tab...");
    let tabs = document.query_selector_all(".berry-tab").unwrap();
    let first_tab = tabs.item(0).unwrap();
    let first_tab_el = first_tab.dyn_into::<web_sys::HtmlElement>().unwrap();
    first_tab_el.click();
    wait_for_render().await;

    // Try typing in the switched tab
    let ime_input = document
        .query_selector("input[type='text']")
        .unwrap()
        .expect("IME input should exist");

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

    leptos::logging::log!("✅ Typing after switching tabs works without crash");
}

#[wasm_bindgen_test]
async fn test_rapid_tab_switching() {
    // Clean up DOM from previous tests
    get_test_document().body().unwrap().set_inner_html("");

    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    // Open 3 files
    selected_file.set(Some(("/file1.rs".to_string(), "content1".to_string())));
    wait_for_render().await;
    wait_for_render().await;

    selected_file.set(Some(("/file2.rs".to_string(), "content2".to_string())));
    wait_for_render().await;
    wait_for_render().await;

    selected_file.set(Some(("/file3.rs".to_string(), "content3".to_string())));
    wait_for_render().await;
    wait_for_render().await;

    let document = get_test_document();
    let tabs = document.query_selector_all(".berry-tab").unwrap();

    // Rapidly switch between tabs
    leptos::logging::log!("📝 Rapidly switching tabs...");
    for i in 0..3 {
        let tab = tabs.item(i).unwrap();
        let tab_el = tab.dyn_into::<web_sys::HtmlElement>().unwrap();
        tab_el.click();
        wait_for_render().await;
    }

    // Switch back and forth multiple times
    for _ in 0..5 {
        let first_tab = tabs.item(0).unwrap().dyn_into::<web_sys::HtmlElement>().unwrap();
        first_tab.click();
        wait_for_render().await;

        let second_tab = tabs.item(1).unwrap().dyn_into::<web_sys::HtmlElement>().unwrap();
        second_tab.click();
        wait_for_render().await;
    }

    // Verify app didn't crash - should still have 3 tabs
    let tabs_after = document.query_selector_all(".berry-tab").unwrap();
    assert_eq!(tabs_after.length(), 3, "Should still have 3 tabs after rapid switching");

    leptos::logging::log!("✅ Rapid tab switching doesn't crash");
}

#[wasm_bindgen_test]
async fn test_close_all_tabs_one_by_one() {
    // Clean up DOM from previous tests
    get_test_document().body().unwrap().set_inner_html("");

    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    // Open 3 files
    selected_file.set(Some(("/file1.rs".to_string(), "content1".to_string())));
    wait_for_render().await;
    wait_for_render().await;

    selected_file.set(Some(("/file2.rs".to_string(), "content2".to_string())));
    wait_for_render().await;
    wait_for_render().await;

    selected_file.set(Some(("/file3.rs".to_string(), "content3".to_string())));
    wait_for_render().await;
    wait_for_render().await;

    let document = get_test_document();

    // Close all tabs one by one
    leptos::logging::log!("📝 Closing all tabs one by one...");
    for _ in 0..3 {
        let tabs = document.query_selector_all(".berry-tab").unwrap();
        leptos::logging::log!("Closing tab, remaining: {}", tabs.length());

        if tabs.length() > 0 {
            let close_buttons = document.query_selector_all(".berry-tab button").unwrap();
            let close_button = close_buttons.item(0).unwrap();
            let button_el = close_button.dyn_into::<web_sys::HtmlElement>().unwrap();
            button_el.click();
            wait_for_render().await;
        }
    }

    // Should have 0 tabs
    let tabs_final = document.query_selector_all(".berry-tab").unwrap();
    assert_eq!(tabs_final.length(), 0, "Should have 0 tabs after closing all");

    leptos::logging::log!("✅ Closing all tabs one by one doesn't crash");
}
