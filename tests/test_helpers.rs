//! Test Helpers - Web-sys abstraction for tests
//!
//! This module hides web_sys details from test code surface, making tests
//! more maintainable and easier to debug when they fail.

use leptos::ev::KeyboardEvent;
use wasm_bindgen::JsCast;

/// Create a keyboard event for testing
///
/// This is the ONLY place in test code that should use web_sys directly.
/// All other test code should use this helper.
///
/// # Why this abstraction?
/// - Easier debugging: When tests fail, you see "simulate_key failed" not "web_sys panic"
/// - Maintainability: If Leptos changes how events work, we only update this one place
/// - Type safety: Returns leptos::ev::KeyboardEvent, not web_sys::KeyboardEvent
pub fn simulate_key(key: &str) -> KeyboardEvent {
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document");

    // Create event using document.createEvent
    let event = document
        .create_event("KeyboardEvent")
        .expect("should create event")
        .dyn_into::<web_sys::KeyboardEvent>()
        .expect("should cast to KeyboardEvent");

    // Initialize the event
    let _ = event.init_keyboard_event_with_bubbles_arg_and_cancelable_arg_and_view_arg_and_key_arg(
        "keydown",
        true,  // bubbles
        true,  // cancelable
        Some(&window),
        key,
    );

    // ✅ Convert to Leptos type before returning
    event.unchecked_into()
}

/// Get an element by ID for testing
///
/// This is safer than web_sys::window().document().get_element_by_id()
/// because it provides better error messages when the element is not found.
pub fn get_test_element(id: &str) -> web_sys::Element {
    let window = web_sys::window().expect("no global window in test");
    let document = window.document().expect("no document in test");

    document
        .get_element_by_id(id)
        .unwrap_or_else(|| panic!("Test element '{}' not found. Did you forget to mount the component?", id))
}

/// Get test element as HtmlElement
pub fn get_test_html_element(id: &str) -> web_sys::HtmlElement {
    get_test_element(id)
        .dyn_into()
        .unwrap_or_else(|_| panic!("Element '{}' is not an HtmlElement", id))
}

/// Query selector - safer wrapper around querySelector
pub fn query_selector(selector: &str) -> Option<web_sys::Element> {
    let window = web_sys::window().expect("no global window in test");
    let document = window.document().expect("no document in test");

    document
        .query_selector(selector)
        .unwrap_or_else(|_| panic!("Invalid selector: '{}'", selector))
}

/// Get elements by class name
pub fn get_elements_by_class_name(class_name: &str) -> web_sys::HtmlCollection {
    let window = web_sys::window().expect("no global window in test");
    let document = window.document().expect("no document in test");

    document.get_elements_by_class_name(class_name)
}

/// Wait for Leptos to render (async helper)
pub async fn wait_for_render() {
    // Use setTimeout to wait for next tick
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        let window = web_sys::window().expect("no window");
        let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 0);
    });

    wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .expect("setTimeout failed");
}

/// Setup root element for tests
///
/// Creates the berry-editor-wasm-root element if it doesn't exist.
/// Removes existing one if present to ensure clean test state.
pub fn setup_root_element() {
    let window = web_sys::window().expect("no window in test");
    let document = window.document().expect("no document in test");

    // Remove existing root if present
    if let Some(existing) = document.get_element_by_id("berry-editor-wasm-root") {
        existing.remove();
    }

    let root = document.create_element("div").expect("failed to create div");
    root.set_id("berry-editor-wasm-root");
    document
        .body()
        .expect("no body element")
        .append_child(&root)
        .expect("failed to append root");
}

/// Get document for test
pub fn get_test_document() -> web_sys::Document {
    web_sys::window()
        .expect("no window in test")
        .document()
        .expect("no document in test")
}

/// Get window for test
pub fn get_test_window() -> web_sys::Window {
    web_sys::window().expect("no window in test")
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_simulate_key_creates_valid_event() {
        let event = simulate_key("a");
        assert_eq!(event.key(), "a");
    }

    #[wasm_bindgen_test]
    fn test_simulate_key_is_preventable() {
        let event = simulate_key("Enter");
        event.prevent_default();
        assert!(event.default_prevented());
    }
}
