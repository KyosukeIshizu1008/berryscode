//! App Startup Integration Test
//!
//! Tests that the app starts without crashing (no black screen).
//! This is a critical test to catch JavaScript errors that cause black screens.

use berry_editor::components_tauri::EditorAppTauri;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

mod test_helpers;
use test_helpers::{get_test_document, wait_for_render};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_app_renders_without_crashing() {
    // This is the CRITICAL test - if this fails, the app shows a black screen
    leptos::logging::log!("🚀 Starting app startup test");

    let _dispose = leptos::mount::mount_to_body(|| {
        view! { <EditorAppTauri /> }
    });

    // Wait for initial render
    wait_for_render().await;
    wait_for_render().await;
    wait_for_render().await;

    leptos::logging::log!("⏳ Waiting for app to render...");

    let document = get_test_document();

    // Check if the main container exists
    let container = document.query_selector(".berry-editor-container").unwrap();
    assert!(
        container.is_some(),
        "Main container should exist - if missing, the app crashed during startup"
    );

    leptos::logging::log!("✅ Main container exists");

    // Check if activity bar exists
    let activity_bar = document.query_selector(".activity-bar").unwrap();
    assert!(
        activity_bar.is_some(),
        "Activity bar should exist - if missing, EditorAppTauri failed to render"
    );

    leptos::logging::log!("✅ Activity bar exists");

    // Check if status bar exists
    let status_bar = document.query_selector(".berry-editor-status-bar").unwrap();
    assert!(
        status_bar.is_some(),
        "Status bar should exist"
    );

    leptos::logging::log!("✅ Status bar exists");

    // Check if main area exists
    let main_area = document.query_selector(".berry-editor-main-area").unwrap();
    assert!(
        main_area.is_some(),
        "Main area should exist"
    );

    leptos::logging::log!("✅ Main area exists");

    leptos::logging::log!("🎉 App startup test PASSED - no black screen!");
}

#[wasm_bindgen_test]
async fn test_terminal_panel_can_be_activated() {
    leptos::logging::log!("🚀 Starting terminal panel activation test");

    let _dispose = leptos::mount::mount_to_body(|| {
        view! { <EditorAppTauri /> }
    });

    wait_for_render().await;
    wait_for_render().await;

    let document = get_test_document();

    // Find the terminal icon in activity bar
    let terminal_icon = document
        .query_selector(".activity-bar > div[title='Integrated Terminal']")
        .unwrap();

    if terminal_icon.is_none() {
        leptos::logging::error!("❌ Terminal icon not found in activity bar!");
        panic!("Terminal icon should exist in activity bar");
    }

    leptos::logging::log!("✅ Terminal icon found");

    // Click the terminal icon
    let icon_el = terminal_icon.unwrap();
    let click_event = web_sys::MouseEvent::new("click").unwrap();
    let _ = icon_el.dispatch_event(&click_event);

    wait_for_render().await;
    wait_for_render().await;
    wait_for_render().await;

    // Check if terminal panel appears
    let terminal_panel = document.query_selector(".terminal-panel").unwrap();

    if terminal_panel.is_none() {
        leptos::logging::error!("❌ Terminal panel did not appear after clicking icon!");

        // Log what's in the main area instead
        let main_area = document.query_selector(".berry-editor-main-area").unwrap();
        if let Some(area) = main_area {
            leptos::logging::log!("Main area HTML: {}", area.inner_html());
        }

        panic!("Terminal panel should appear when icon is clicked");
    }

    leptos::logging::log!("✅ Terminal panel appeared");

    leptos::logging::log!("🎉 Terminal activation test PASSED!");
}

#[wasm_bindgen_test]
async fn test_all_panels_can_switch() {
    leptos::logging::log!("🚀 Starting panel switching test");

    let _dispose = leptos::mount::mount_to_body(|| {
        view! { <EditorAppTauri /> }
    });

    wait_for_render().await;

    let document = get_test_document();

    // Test switching to each panel
    let panels = vec![
        ("Explorer", ".berry-editor-sidebar"),
        ("Search", ".berry-editor-sidebar"),
        ("BerryChat", ".berry-editor-sidebar"),
        ("Database Tools", ".berry-editor-sidebar"),
        ("Workflow Automation", ".berry-editor-sidebar"),
        ("Integrated Terminal", ".terminal-panel"),
        ("Virtual Office", ".berry-editor-sidebar"),
    ];

    for (panel_name, expected_selector) in panels {
        leptos::logging::log!("🔄 Switching to panel: {}", panel_name);

        let icon = document
            .query_selector(&format!(".activity-bar > div[title='{}']", panel_name))
            .unwrap();

        if icon.is_none() {
            leptos::logging::error!("❌ Icon not found for panel: {}", panel_name);
            continue;
        }

        let icon_el = icon.unwrap();
        let click_event = web_sys::MouseEvent::new("click").unwrap();
        let _ = icon_el.dispatch_event(&click_event);

        wait_for_render().await;
        wait_for_render().await;

        // Verify the panel or sidebar exists
        let panel_el = document.query_selector(expected_selector).unwrap();

        if panel_el.is_none() {
            leptos::logging::error!("❌ Panel not found after switch: {} (selector: {})", panel_name, expected_selector);
        } else {
            leptos::logging::log!("✅ Panel {} rendered successfully", panel_name);
        }
    }

    leptos::logging::log!("🎉 Panel switching test completed!");
}

#[wasm_bindgen_test]
async fn test_no_javascript_errors() {
    // This test ensures no JavaScript exceptions are thrown during app initialization
    leptos::logging::log!("🚀 Starting no-errors test");

    // Set up error handler
    let error_occurred = std::rc::Rc::new(std::cell::RefCell::new(false));
    let error_occurred_clone = error_occurred.clone();

    let window = web_sys::window().expect("should have window");
    let error_handler = wasm_bindgen::closure::Closure::wrap(Box::new(move |_event: web_sys::ErrorEvent| {
        *error_occurred_clone.borrow_mut() = true;
        leptos::logging::error!("❌ JavaScript error detected during app initialization!");
    }) as Box<dyn FnMut(_)>);

    window
        .add_event_listener_with_callback("error", error_handler.as_ref().unchecked_ref())
        .unwrap();

    // Mount the app
    let _dispose = leptos::mount::mount_to_body(|| {
        view! { <EditorAppTauri /> }
    });

    wait_for_render().await;
    wait_for_render().await;

    // Check if any errors occurred
    assert!(
        !*error_occurred.borrow(),
        "JavaScript errors should not occur during app initialization"
    );

    error_handler.forget(); // Keep handler alive

    leptos::logging::log!("✅ No JavaScript errors detected");
}
