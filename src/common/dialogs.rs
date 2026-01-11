//! Dialog Components
//! Reusable dialog components for confirmations, inputs, etc.

use leptos::prelude::*;
use wasm_bindgen::JsCast;

/// Confirmation Dialog
#[component]
pub fn ConfirmDialog(
    is_open: RwSignal<bool>,
    title: String,
    message: String,
    on_confirm: impl Fn() + 'static + Clone + Send + Sync,
    on_cancel: impl Fn() + 'static + Clone + Send + Sync,
) -> impl IntoView {
    let on_confirm = StoredValue::new(on_confirm);
    let on_cancel = StoredValue::new(on_cancel);

    view! {
        {move || {
            if is_open.get() {
                view! {
                    <div class="berry-dialog-overlay">
                        <div class="berry-dialog">
                            <div class="berry-dialog-header">
                                <h3>{title.clone()}</h3>
                            </div>
                            <div class="berry-dialog-body">
                                <p>{message.clone()}</p>
                            </div>
                            <div class="berry-dialog-footer">
                                <button
                                    class="berry-dialog-button berry-dialog-button-cancel"
                                    on:click=move |_| {
                                        on_cancel.with_value(|f| f());
                                        is_open.set(false);
                                    }
                                >
                                    "Cancel"
                                </button>
                                <button
                                    class="berry-dialog-button berry-dialog-button-confirm"
                                    on:click=move |_| {
                                        on_confirm.with_value(|f| f());
                                        is_open.set(false);
                                    }
                                >
                                    "Confirm"
                                </button>
                            </div>
                        </div>
                    </div>
                }.into_any()
            } else {
                view! { <></> }.into_any()
            }
        }}
    }
}

/// Input Dialog
#[component]
pub fn InputDialog(
    is_open: RwSignal<bool>,
    title: String,
    placeholder: String,
    initial_value: String,
    on_submit: impl Fn(String) + 'static + Clone + Send + Sync,
    on_cancel: impl Fn() + 'static + Clone + Send + Sync,
) -> impl IntoView {
    let on_submit = StoredValue::new(on_submit);
    let on_cancel = StoredValue::new(on_cancel);
    let input_value = RwSignal::new(initial_value.clone());

    // Reset input value when dialog opens
    Effect::new(move |_| {
        if is_open.get() {
            input_value.set(initial_value.clone());
        }
    });

    view! {
        {move || {
            if is_open.get() {
                view! {
                    <div class="berry-dialog-overlay">
                        <div class="berry-dialog">
                            <div class="berry-dialog-header">
                                <h3>{title.clone()}</h3>
                            </div>
                            <div class="berry-dialog-body">
                                <input
                                    type="text"
                                    class="berry-dialog-input"
                                    placeholder=placeholder.clone()
                                    prop:value=move || input_value.get()
                                    on:input=move |ev| {
                                        input_value.set(event_target_value(&ev));
                                    }
                                    on:keydown=move |ev| {
                                        if ev.key() == "Enter" {
                                            let value = input_value.get();
                                            if !value.trim().is_empty() {
                                                on_submit.with_value(|f| f(value));
                                                is_open.set(false);
                                            }
                                        } else if ev.key() == "Escape" {
                                            on_cancel.with_value(|f| f());
                                            is_open.set(false);
                                        }
                                    }
                                />
                            </div>
                            <div class="berry-dialog-footer">
                                <button
                                    class="berry-dialog-button berry-dialog-button-cancel"
                                    on:click=move |_| {
                                        on_cancel.with_value(|f| f());
                                        is_open.set(false);
                                    }
                                >
                                    "Cancel"
                                </button>
                                <button
                                    class="berry-dialog-button berry-dialog-button-confirm"
                                    on:click=move |_| {
                                        let value = input_value.get();
                                        if !value.trim().is_empty() {
                                            on_submit.with_value(|f| f(value));
                                            is_open.set(false);
                                        }
                                    }
                                >
                                    "OK"
                                </button>
                            </div>
                        </div>
                    </div>
                }.into_any()
            } else {
                view! { <></> }.into_any()
            }
        }}
    }
}

/// File/Folder Creation Dialog
#[component]
pub fn CreateFileDialog(
    is_open: RwSignal<bool>,
    is_folder: bool,
    parent_path: String,
    on_create: impl Fn(String, bool) + 'static + Clone + Send + Sync,
) -> impl IntoView {
    let on_create = StoredValue::new(on_create);
    let filename = RwSignal::new(String::new());
    let error_message = RwSignal::new(None::<String>);

    let title = if is_folder { "New Folder" } else { "New File" };
    let placeholder = if is_folder { "Folder name" } else { "File name" };

    view! {
        {move || {
            if is_open.get() {
                view! {
                    <div class="berry-dialog-overlay">
                        <div class="berry-dialog">
                            <div class="berry-dialog-header">
                                <h3>{title}</h3>
                            </div>
                            <div class="berry-dialog-body">
                                <p class="berry-dialog-parent-path">
                                    "Parent: " {parent_path.clone()}
                                </p>
                                <input
                                    type="text"
                                    class="berry-dialog-input"
                                    placeholder=placeholder
                                    prop:value=move || filename.get()
                                    on:input=move |ev| {
                                        filename.set(event_target_value(&ev));
                                        error_message.set(None);
                                    }
                                    on:keydown=move |ev| {
                                        if ev.key() == "Enter" {
                                            let name = filename.get();
                                            if validate_filename(&name) {
                                                on_create.with_value(|f| f(name, is_folder));
                                                is_open.set(false);
                                                filename.set(String::new());
                                            } else {
                                                error_message.set(Some("Invalid filename".to_string()));
                                            }
                                        } else if ev.key() == "Escape" {
                                            is_open.set(false);
                                            filename.set(String::new());
                                        }
                                    }
                                />
                                {move || {
                                    if let Some(ref err) = error_message.get() {
                                        view! {
                                            <p class="berry-dialog-error">{err.clone()}</p>
                                        }.into_any()
                                    } else {
                                        view! { <></> }.into_any()
                                    }
                                }}
                            </div>
                            <div class="berry-dialog-footer">
                                <button
                                    class="berry-dialog-button berry-dialog-button-cancel"
                                    on:click=move |_| {
                                        is_open.set(false);
                                        filename.set(String::new());
                                        error_message.set(None);
                                    }
                                >
                                    "Cancel"
                                </button>
                                <button
                                    class="berry-dialog-button berry-dialog-button-confirm"
                                    on:click=move |_| {
                                        let name = filename.get();
                                        if validate_filename(&name) {
                                            on_create.with_value(|f| f(name, is_folder));
                                            is_open.set(false);
                                            filename.set(String::new());
                                        } else {
                                            error_message.set(Some("Invalid filename".to_string()));
                                        }
                                    }
                                >
                                    "Create"
                                </button>
                            </div>
                        </div>
                    </div>
                }.into_any()
            } else {
                view! { <></> }.into_any()
            }
        }}
    }
}

/// Validate filename
fn validate_filename(name: &str) -> bool {
    if name.trim().is_empty() {
        return false;
    }

    // Check for invalid characters
    let invalid_chars = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];
    for ch in invalid_chars.iter() {
        if name.contains(*ch) {
            return false;
        }
    }

    // Check for reserved names (Windows)
    let reserved = ["CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4",
                    "COM5", "COM6", "COM7", "COM8", "COM9", "LPT1", "LPT2",
                    "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9"];
    let name_upper = name.to_uppercase();
    for res in reserved.iter() {
        if name_upper == *res {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_filename_valid() {
        assert!(validate_filename("test.txt"));
        assert!(validate_filename("myfile.rs"));
        assert!(validate_filename("folder_name"));
        assert!(validate_filename("file-with-dash.md"));
    }

    #[test]
    fn test_validate_filename_invalid() {
        assert!(!validate_filename(""));
        assert!(!validate_filename("   "));
        assert!(!validate_filename("file/path.txt"));
        assert!(!validate_filename("file\\path.txt"));
        assert!(!validate_filename("file:name.txt"));
        assert!(!validate_filename("file*name.txt"));
        assert!(!validate_filename("file?name.txt"));
        assert!(!validate_filename("CON"));
        assert!(!validate_filename("PRN"));
    }
}
