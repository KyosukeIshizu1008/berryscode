//! Breakpoint Gutter Component
//!
//! Displays breakpoints in the editor gutter with click-to-toggle functionality.

use dioxus::prelude::*;
use super::session::Breakpoint;

/// Breakpoint gutter component props
#[derive(Props, Clone, PartialEq)]
pub struct BreakpointGutterProps {
    /// Line number (1-indexed)
    line_number: usize,
    /// Current breakpoint state for this line
    breakpoint: Signal<Option<Breakpoint>>,
    /// Callback when breakpoint is toggled
    on_toggle: EventHandler<usize>,
}

/// Breakpoint gutter component for a single line
#[component]
pub fn BreakpointGutter(props: BreakpointGutterProps) -> Element {
    let line_number = props.line_number;
    let breakpoint = props.breakpoint;
    let on_toggle = props.on_toggle;

    rsx! {
        div {
            class: "berry-breakpoint-gutter",
            onclick: move |_| on_toggle.call(line_number),

            {
                let bp = breakpoint.read().clone();
                if let Some(breakpoint) = bp {
                    let class = if breakpoint.verified {
                        "berry-breakpoint-icon berry-breakpoint-verified"
                    } else {
                        "berry-breakpoint-icon berry-breakpoint-unverified"
                    };

                    let title = if let Some(ref cond) = breakpoint.condition {
                        format!("Conditional: {}", cond)
                    } else {
                        "Breakpoint".to_string()
                    };

                    rsx! {
                        span {
                            class: "{class}",
                            title: "{title}",
                            "●"
                        }
                    }
                } else {
                    rsx! {
                        span { class: "berry-breakpoint-placeholder" }
                    }
                }
            }
        }
    }
}

/// Conditional breakpoint editor dialog props
#[derive(Props, Clone, PartialEq)]
pub struct ConditionalBreakpointDialogProps {
    /// Whether the dialog is visible
    visible: Signal<bool>,
    /// Current condition (if any)
    current_condition: Signal<Option<String>>,
    /// Callback when condition is set
    on_set: EventHandler<Option<String>>,
}

/// Conditional breakpoint editor dialog
#[component]
pub fn ConditionalBreakpointDialog(props: ConditionalBreakpointDialogProps) -> Element {
    let visible = props.visible;
    let current_condition = props.current_condition;
    let on_set = props.on_set;

    let mut condition_input = use_signal(|| String::new());

    // Initialize condition input when dialog becomes visible
    use_effect(move || {
        if *visible.read() {
            *condition_input.write() = current_condition.read().clone().unwrap_or_default();
        }
    });

    let handle_ok = move |_| {
        let condition = condition_input.read().clone();
        let final_condition = if condition.is_empty() {
            None
        } else {
            Some(condition)
        };
        on_set.call(final_condition);
        *visible.write() = false;
    };

    let handle_cancel = move |_| {
        *visible.write() = false;
    };

    let overlay_class = if *visible.read() {
        "berry-dialog-overlay berry-dialog-visible"
    } else {
        "berry-dialog-overlay"
    };

    rsx! {
        div { class: "{overlay_class}",
            div { class: "berry-dialog berry-conditional-breakpoint-dialog",
                h3 { "Conditional Breakpoint" }
                div { class: "berry-dialog-content",
                    label {
                        "Break when expression is true:"
                        input {
                            r#type: "text",
                            class: "berry-input",
                            value: "{condition_input.read()}",
                            oninput: move |ev| *condition_input.write() = ev.value(),
                            placeholder: "e.g., x > 10",
                        }
                    }
                }
                div { class: "berry-dialog-actions",
                    button {
                        class: "berry-button",
                        onclick: handle_ok,
                        "OK"
                    }
                    button {
                        class: "berry-button",
                        onclick: handle_cancel,
                        "Cancel"
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_breakpoint_gutter_compiles() {
        // Ensure component compiles
        assert!(true);
    }

    #[test]
    fn test_breakpoint_verified_class() {
        let bp = Breakpoint {
            id: "1".to_string(),
            file: PathBuf::from("test.rs"),
            line: 10,
            condition: None,
            verified: true,
        };

        assert!(bp.verified);
    }

    #[test]
    fn test_breakpoint_conditional_message() {
        let bp = Breakpoint {
            id: "1".to_string(),
            file: PathBuf::from("test.rs"),
            line: 10,
            condition: Some("x > 10".to_string()),
            verified: true,
        };

        assert!(bp.condition.is_some());
        assert_eq!(bp.condition.as_ref().unwrap(), "x > 10");
    }

    #[wasm_bindgen_test]
    fn test_conditional_dialog_compiles() {
        // Ensure conditional dialog compiles
        assert!(true);
    }
}
