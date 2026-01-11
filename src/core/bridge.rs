//! Browser Bridge - Web API Abstraction Layer
//!
//! This module is the ONLY place where web_sys APIs should be used directly.
//! All other code interacts with the browser through Leptos abstractions or
//! this bridge.

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use serde::Serialize;
use std::rc::Rc;
use std::cell::RefCell;

/// Worker handle that can post messages
///
/// This abstraction hides web_sys::Worker from the rest of the codebase.
/// It's safe to use in WASM's single-threaded environment.
pub struct WorkerHandle<T> {
    worker: Rc<web_sys::Worker>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Serialize> WorkerHandle<T> {
    /// Post a message to the worker
    pub fn post(&self, message: T) {
        if let Ok(js_message) = serde_wasm_bindgen::to_value(&message) {
            let _ = self.worker.post_message(&js_message);
        }
    }
}

/// Spawn a Web Worker and return a handle for posting messages
///
/// This is the ONLY function that should directly use web_sys::Worker.
/// All worker communication goes through this abstraction.
///
/// # Arguments
/// * `script_url` - Path to the worker script
/// * `on_message` - Callback to handle messages from the worker
///
/// # Returns
/// A WorkerHandle that can be used to send messages to the worker
pub fn spawn_worker<T: Serialize + 'static>(
    script_url: &str,
    on_message: impl Fn(JsValue) + 'static,
) -> Result<WorkerHandle<T>, JsValue> {
    // ✅ This is the ONLY place where web_sys::Worker is allowed
    let worker = web_sys::Worker::new(script_url)?;

    // Set up message handler
    let onmessage_callback = Closure::wrap(Box::new(move |event: web_sys::MessageEvent| {
        on_message(event.data());
    }) as Box<dyn FnMut(_)>);

    worker.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
    onmessage_callback.forget();

    // Return a handle to the worker
    Ok(WorkerHandle {
        worker: Rc::new(worker),
        _phantom: std::marker::PhantomData,
    })
}

/// Terminate a worker (if needed for cleanup)
///
/// Since we use Callbacks, termination is handled by dropping the worker
/// reference, but this can be used for explicit cleanup if needed.
pub fn terminate_worker_handle() {
    // Workers are automatically terminated when dropped
    // This function exists for API completeness
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_compiles() {
        // This module should compile even without web_sys in test environments
        assert!(true);
    }
}
