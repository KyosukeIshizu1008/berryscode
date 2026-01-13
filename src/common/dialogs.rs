//! Dialog Components
//! Reusable dialog components for confirmations, inputs, etc.

use dioxus::prelude::*;

/// Confirmation Dialog props
#[derive(Props, Clone, PartialEq)]
pub struct ConfirmDialogProps {
    is_open: Signal<bool>,
    title: String,
    message: String,
    on_confirm: EventHandler<()>,
    on_cancel: EventHandler<()>,
}

/// Confirmation Dialog
#[component]
pub fn ConfirmDialog(props: ConfirmDialogProps) -> Element {
    let is_open = props.is_open;
    let title = props.title;
    let message = props.message;
    let on_confirm = props.on_confirm;
    let on_cancel = props.on_cancel;

    rsx! {
        {
            if *is_open.read() {
                rsx! {
                    div { class: "berry-dialog-overlay",
                        div { class: "berry-dialog",
                            div { class: "berry-dialog-header",
                                h3 { "{title}" }
                            }
                            div { class: "berry-dialog-body",
                                p { "{message}" }
                            }
                            div { class: "berry-dialog-footer",
                                button {
                                    class: "berry-dialog-button berry-dialog-button-cancel",
                                    onclick: move |_| {
                                        on_cancel.call(());
                                        *is_open.write() = false;
                                    },
                                    "Cancel"
                                }
                                button {
                                    class: "berry-dialog-button berry-dialog-button-confirm",
                                    onclick: move |_| {
                                        on_confirm.call(());
                                        *is_open.write() = false;
                                    },
                                    "Confirm"
                                }
                            }
                        }
                    }
                }
            } else {
                rsx! {}
            }
        }
    }
}

/// Input Dialog props
#[derive(Props, Clone, PartialEq)]
pub struct InputDialogProps {
    is_open: Signal<bool>,
    title: String,
    placeholder: String,
    initial_value: String,
    on_submit: EventHandler<String>,
    on_cancel: EventHandler<()>,
}

/// Input Dialog
#[component]
pub fn InputDialog(props: InputDialogProps) -> Element {
    let is_open = props.is_open;
    let title = props.title;
    let placeholder = props.placeholder;
    let initial_value = props.initial_value.clone();
    let on_submit = props.on_submit;
    let on_cancel = props.on_cancel;

    let mut input_value = use_signal(|| initial_value.clone());

    // Reset input value when dialog opens
    use_effect(move || {
        if *is_open.read() {
            *input_value.write() = initial_value.clone();
        }
    });

    rsx! {
        {
            if *is_open.read() {
                rsx! {
                    div { class: "berry-dialog-overlay",
                        div { class: "berry-dialog",
                            div { class: "berry-dialog-header",
                                h3 { "{title}" }
                            }
                            div { class: "berry-dialog-body",
                                input {
                                    r#type: "text",
                                    class: "berry-dialog-input",
                                    placeholder: "{placeholder}",
                                    value: "{input_value.read()}",
                                    oninput: move |ev| *input_value.write() = ev.value(),
                                    onkeydown: move |ev| {
                                        if ev.key() == Key::Enter {
                                            let value = input_value.read().clone();
                                            if !value.trim().is_empty() {
                                                on_submit.call(value);
                                                *is_open.write() = false;
                                            }
                                        } else if ev.key() == Key::Escape {
                                            on_cancel.call(());
                                            *is_open.write() = false;
                                        }
                                    },
                                }
                            }
                            div { class: "berry-dialog-footer",
                                button {
                                    class: "berry-dialog-button berry-dialog-button-cancel",
                                    onclick: move |_| {
                                        on_cancel.call(());
                                        *is_open.write() = false;
                                    },
                                    "Cancel"
                                }
                                button {
                                    class: "berry-dialog-button berry-dialog-button-confirm",
                                    onclick: move |_| {
                                        let value = input_value.read().clone();
                                        if !value.trim().is_empty() {
                                            on_submit.call(value);
                                            *is_open.write() = false;
                                        }
                                    },
                                    "OK"
                                }
                            }
                        }
                    }
                }
            } else {
                rsx! {}
            }
        }
    }
}

/// File/Folder Creation Dialog props
#[derive(Props, Clone, PartialEq)]
pub struct CreateFileDialogProps {
    is_open: Signal<bool>,
    is_folder: bool,
    parent_path: String,
    on_create: EventHandler<(String, bool)>,
}

/// File/Folder Creation Dialog
#[component]
pub fn CreateFileDialog(props: CreateFileDialogProps) -> Element {
    let is_open = props.is_open;
    let is_folder = props.is_folder;
    let parent_path = props.parent_path;
    let on_create = props.on_create;

    let mut filename = use_signal(|| String::new());
    let mut error_message = use_signal(|| None::<String>);

    let title = if is_folder { "New Folder" } else { "New File" };
    let placeholder = if is_folder { "Folder name" } else { "File name" };

    rsx! {
        {
            if *is_open.read() {
                rsx! {
                    div { class: "berry-dialog-overlay",
                        div { class: "berry-dialog",
                            div { class: "berry-dialog-header",
                                h3 { "{title}" }
                            }
                            div { class: "berry-dialog-body",
                                p { class: "berry-dialog-parent-path",
                                    "Parent: {parent_path}"
                                }
                                input {
                                    r#type: "text",
                                    class: "berry-dialog-input",
                                    placeholder: "{placeholder}",
                                    value: "{filename.read()}",
                                    oninput: move |ev| {
                                        *filename.write() = ev.value();
                                        *error_message.write() = None;
                                    },
                                    onkeydown: move |ev| {
                                        if ev.key() == Key::Enter {
                                            let name = filename.read().clone();
                                            if validate_filename(&name) {
                                                on_create.call((name, is_folder));
                                                *is_open.write() = false;
                                                *filename.write() = String::new();
                                            } else {
                                                *error_message.write() = Some("Invalid filename".to_string());
                                            }
                                        } else if ev.key() == Key::Escape {
                                            *is_open.write() = false;
                                            *filename.write() = String::new();
                                        }
                                    },
                                }
                                {
                                    error_message.read().as_ref().map(|err| {
                                        rsx! {
                                            p { class: "berry-dialog-error", "{err}" }
                                        }
                                    })
                                }
                            }
                            div { class: "berry-dialog-footer",
                                button {
                                    class: "berry-dialog-button berry-dialog-button-cancel",
                                    onclick: move |_| {
                                        *is_open.write() = false;
                                        *filename.write() = String::new();
                                        *error_message.write() = None;
                                    },
                                    "Cancel"
                                }
                                button {
                                    class: "berry-dialog-button berry-dialog-button-confirm",
                                    onclick: move |_| {
                                        let name = filename.read().clone();
                                        if validate_filename(&name) {
                                            on_create.call((name, is_folder));
                                            *is_open.write() = false;
                                            *filename.write() = String::new();
                                        } else {
                                            *error_message.write() = Some("Invalid filename".to_string());
                                        }
                                    },
                                    "Create"
                                }
                            }
                        }
                    }
                }
            } else {
                rsx! {}
            }
        }
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
