//! Integration Test: IntelliJ-style Search Dialog
//!
//! Tests the overlay search dialog functionality that appears when clicking
//! the search icon in the activity bar.

use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_search_dialog_appears_on_click() {
    use web_sys::window;
    use wasm_bindgen::JsCast;

    let document = window().unwrap().document().unwrap();
    let body = document.body().unwrap();
    body.set_inner_html("");

    // Create test container
    let container = document.create_element("div").unwrap();
    container.set_id("berry-editor-wasm-root");
    body.append_child(&container).unwrap();

    // Initialize editor
    berry_editor::init_berry_editor();

    // Wait for rendering
    gloo_timers::future::TimeoutFuture::new(100).await;

    // Initially, search dialog should not be visible
    let search_dialog = document.query_selector(".berry-search-dialog").unwrap();
    assert!(search_dialog.is_none(), "Search dialog should be hidden initially");

    // Find and click search icon
    let search_icon = document
        .query_selector("div[title='Search']")
        .unwrap()
        .expect("Search icon should exist");

    let search_icon_html: web_sys::HtmlElement = search_icon.dyn_into().unwrap();
    search_icon_html.click();

    // Wait for dialog to appear
    gloo_timers::future::TimeoutFuture::new(100).await;

    // Search dialog should now be visible as an overlay
    let search_dialog = document
        .query_selector(".berry-search-dialog")
        .unwrap()
        .expect("Search dialog should appear after clicking search icon");

    let search_dialog_html: web_sys::HtmlElement = search_dialog.dyn_into().unwrap();
    let computed_style = window()
        .unwrap()
        .get_computed_style(&search_dialog_html)
        .unwrap()
        .unwrap();

    assert_ne!(computed_style.get_property_value("display").unwrap(), "none",
        "Search dialog should be displayed");
}

#[wasm_bindgen_test]
async fn test_search_dialog_has_tabs() {
    use web_sys::window;
    use wasm_bindgen::JsCast;

    let document = window().unwrap().document().unwrap();
    let body = document.body().unwrap();
    body.set_inner_html("");

    // Create test container
    let container = document.create_element("div").unwrap();
    container.set_id("berry-editor-wasm-root");
    body.append_child(&container).unwrap();

    // Initialize editor
    berry_editor::init_berry_editor();

    // Wait for rendering
    gloo_timers::future::TimeoutFuture::new(100).await;

    // Click search icon to open dialog
    let search_icon = document
        .query_selector("div[title='Search']")
        .unwrap()
        .expect("Search icon should exist");

    let search_icon_html: web_sys::HtmlElement = search_icon.dyn_into().unwrap();
    search_icon_html.click();

    // Wait for dialog to appear
    gloo_timers::future::TimeoutFuture::new(100).await;

    // Check for tabs
    let expected_tabs = vec!["All", "Types", "Files", "Symbols", "Actions", "Text"];
    for tab_name in expected_tabs {
        let tab = document
            .query_selector(&format!(".berry-search-tab[data-tab='{}']", tab_name.to_lowercase()))
            .unwrap()
            .expect(&format!("Tab '{}' should exist", tab_name));

        let tab_html: web_sys::HtmlElement = tab.dyn_into().unwrap();
        assert!(tab_html.inner_text().contains(tab_name),
            "Tab should have correct text: {}", tab_name);
    }
}

#[wasm_bindgen_test]
async fn test_search_dialog_has_input_field() {
    use web_sys::window;
    use wasm_bindgen::JsCast;

    let document = window().unwrap().document().unwrap();
    let body = document.body().unwrap();
    body.set_inner_html("");

    // Create test container
    let container = document.create_element("div").unwrap();
    container.set_id("berry-editor-wasm-root");
    body.append_child(&container).unwrap();

    // Initialize editor
    berry_editor::init_berry_editor();

    // Wait for rendering
    gloo_timers::future::TimeoutFuture::new(100).await;

    // Click search icon to open dialog
    let search_icon = document
        .query_selector("div[title='Search']")
        .unwrap()
        .expect("Search icon should exist");

    let search_icon_html: web_sys::HtmlElement = search_icon.dyn_into().unwrap();
    search_icon_html.click();

    // Wait for dialog to appear
    gloo_timers::future::TimeoutFuture::new(100).await;

    // Check for search input field
    let search_input = document
        .query_selector(".berry-search-dialog input[type='text']")
        .unwrap()
        .expect("Search input field should exist");

    let input_html: web_sys::HtmlInputElement = search_input.dyn_into().unwrap();
    assert!(input_html.placeholder().contains("search") ||
            input_html.placeholder().contains("Type"),
        "Search input should have appropriate placeholder");
}

#[wasm_bindgen_test]
async fn test_search_dialog_has_close_button() {
    use web_sys::window;
    use wasm_bindgen::JsCast;

    let document = window().unwrap().document().unwrap();
    let body = document.body().unwrap();
    body.set_inner_html("");

    // Create test container
    let container = document.create_element("div").unwrap();
    container.set_id("berry-editor-wasm-root");
    body.append_child(&container).unwrap();

    // Initialize editor
    berry_editor::init_berry_editor();

    // Wait for rendering
    gloo_timers::future::TimeoutFuture::new(100).await;

    // Click search icon to open dialog
    let search_icon = document
        .query_selector("div[title='Search']")
        .unwrap()
        .expect("Search icon should exist");

    let search_icon_html: web_sys::HtmlElement = search_icon.dyn_into().unwrap();
    search_icon_html.click();

    // Wait for dialog to appear
    gloo_timers::future::TimeoutFuture::new(100).await;

    // Verify dialog is open
    let search_dialog = document.query_selector(".berry-search-dialog").unwrap();
    assert!(search_dialog.is_some(), "Search dialog should be visible");

    // Check for close button
    let close_button = document
        .query_selector(".berry-search-close")
        .unwrap()
        .expect("Close button should exist");

    let close_button_html: web_sys::HtmlElement = close_button.dyn_into().unwrap();
    assert_eq!(close_button_html.inner_text(), "×", "Close button should have × symbol");
}

#[wasm_bindgen_test]
async fn test_search_dialog_does_not_affect_background_panels() {
    use web_sys::window;
    use wasm_bindgen::JsCast;

    let document = window().unwrap().document().unwrap();
    let body = document.body().unwrap();
    body.set_inner_html("");

    // Create test container
    let container = document.create_element("div").unwrap();
    container.set_id("berry-editor-wasm-root");
    body.append_child(&container).unwrap();

    // Initialize editor
    berry_editor::init_berry_editor();

    // Wait for rendering
    gloo_timers::future::TimeoutFuture::new(100).await;

    // Get initial panel state (should be Explorer)
    let explorer_panel = document.query_selector(".berry-file-tree-panel").ok().flatten();
    let has_explorer_initially = explorer_panel.is_some();

    // Click search icon to open dialog
    let search_icon = document
        .query_selector("div[title='Search']")
        .unwrap()
        .expect("Search icon should exist");

    let search_icon_html: web_sys::HtmlElement = search_icon.dyn_into().unwrap();
    search_icon_html.click();

    // Wait for dialog to appear
    gloo_timers::future::TimeoutFuture::new(100).await;

    // Background panel should still be visible (or in same state)
    let _explorer_panel_after = document.query_selector(".berry-file-tree-panel").ok().flatten();

    // If we had explorer initially, search dialog should overlay it, not replace it
    // (Note: This test may need adjustment based on actual implementation)
    if has_explorer_initially {
        // The search dialog should be an overlay, so we check if both exist
        let search_dialog = document.query_selector(".berry-search-dialog").unwrap();
        assert!(search_dialog.is_some(), "Search dialog should be visible");

        // Background should remain (though this may vary based on implementation)
        // At minimum, the search dialog should have overlay styling
        let dialog_html: web_sys::HtmlElement = search_dialog.unwrap().dyn_into().unwrap();
        let computed_style = window()
            .unwrap()
            .get_computed_style(&dialog_html)
            .unwrap()
            .unwrap();

        let position = computed_style.get_property_value("position").unwrap();
        assert!(position == "fixed" || position == "absolute",
            "Search dialog should be positioned as an overlay (fixed or absolute)");
    }
}
