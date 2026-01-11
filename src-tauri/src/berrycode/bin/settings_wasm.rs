//! BerryCode Settings Window (WASM Binary)
//!
//! This binary is compiled to WASM and loaded in the settings window.
//! It mounts the Leptos SettingsApp component.

#![cfg(feature = "leptos-ui")]

use leptos::*;

fn main() {
    // Mount the Leptos app to the body
    console_error_panic_hook::set_once();

    mount_to_body(|| {
        view! {
            <berrycode::ui::SettingsApp />
        }
    });
}
