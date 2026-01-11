//! Tab Close Crash Test - Simplified version to test the critical close tab bug fix

use berry_editor::core::virtual_editor::VirtualEditorPanel;
use leptos::prelude::*;
use wasm_bindgen_test::*;
use wasm_bindgen::JsCast;

mod test_helpers;
use test_helpers::{get_test_document, wait_for_render};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_close_single_tab_doesnt_crash() {
    get_test_document().body().unwrap().set_inner_html("");

    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    // Open one file
    selected_file.set(Some(("/test.rs".to_string(), "hello".to_string())));
    wait_for_render().await;
    wait_for_render().await;

    let document = get_test_document();
    let tabs = document.query_selector_all(".berry-tab").unwrap();
    assert_eq!(tabs.length(), 1);

    // Close the tab
    let close_button = document.query_selector(".berry-tab button").unwrap().expect("Button exists");
    close_button.dyn_into::<web_sys::HtmlElement>().unwrap().click();
    wait_for_render().await;

    // Verify no crash and no tabs remain
    let tabs_after = document.query_selector_all(".berry-tab").unwrap();
    assert_eq!(tabs_after.length(), 0, "No tabs should remain");
}

#[wasm_bindgen_test]
async fn test_close_first_of_two_tabs() {
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
    let tabs = document.query_selector_all(".berry-tab").unwrap();
    assert_eq!(tabs.length(), 2);

    // Close first tab
    let close_buttons = document.query_selector_all(".berry-tab button").unwrap();
    let first_button = close_buttons.item(0).unwrap();
    first_button.dyn_into::<web_sys::HtmlElement>().unwrap().click();
    wait_for_render().await;

    // Should have 1 tab remaining
    let tabs_after = document.query_selector_all(".berry-tab").unwrap();
    assert_eq!(tabs_after.length(), 1, "Should have 1 tab remaining");

    // Should have an active tab
    let active_tabs = document.query_selector_all(".berry-tab.active").unwrap();
    assert_eq!(active_tabs.length(), 1, "Should have 1 active tab");
}

#[wasm_bindgen_test]
async fn test_close_second_of_two_tabs() {
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

    // Close second tab (which is active)
    let close_buttons = document.query_selector_all(".berry-tab button").unwrap();
    let second_button = close_buttons.item(1).unwrap();
    second_button.dyn_into::<web_sys::HtmlElement>().unwrap().click();
    wait_for_render().await;

    // Should have 1 tab remaining and it should be active
    let tabs_after = document.query_selector_all(".berry-tab").unwrap();
    assert_eq!(tabs_after.length(), 1);

    let active_tabs = document.query_selector_all(".berry-tab.active").unwrap();
    assert_eq!(active_tabs.length(), 1);
}
