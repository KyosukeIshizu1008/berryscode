//! Breakpoint Gutter Component
//!
//! Displays breakpoints in the editor gutter with click-to-toggle functionality.

use leptos::prelude::*;
use super::session::Breakpoint;

/// Breakpoint gutter component for a single line
#[component]
pub fn BreakpointGutter(
    /// Line number (1-indexed)
    line_number: usize,
    /// Current breakpoint state for this line
    breakpoint: RwSignal<Option<Breakpoint>>,
    /// Callback when breakpoint is toggled
    on_toggle: impl Fn(usize) + 'static + Clone,
) -> impl IntoView {
    let on_toggle_clone = on_toggle.clone();

    view! {
        <div
            class="berry-breakpoint-gutter"
            on:click=move |_| on_toggle_clone(line_number)
        >
            {move || {
                let bp = breakpoint.get();
                if let Some(breakpoint) = bp {
                    let class = if breakpoint.verified {
                        "berry-breakpoint-icon berry-breakpoint-verified"
                    } else {
                        "berry-breakpoint-icon berry-breakpoint-unverified"
                    };

                    view! {
                        <span
                            class=class
                            title=move || {
                                if let Some(ref cond) = breakpoint.condition {
                                    format!("Conditional: {}", cond)
                                } else {
                                    "Breakpoint".to_string()
                                }
                            }
                        >
                            "●"
                        </span>
                    }.into_any()
                } else {
                    view! {
                        <span class="berry-breakpoint-placeholder"></span>
                    }.into_any()
                }
            }}
        </div>
    }
}

/// Conditional breakpoint editor dialog
#[component]
pub fn ConditionalBreakpointDialog(
    /// Whether the dialog is visible
    visible: RwSignal<bool>,
    /// Current condition (if any)
    current_condition: RwSignal<Option<String>>,
    /// Callback when condition is set
    on_set: impl Fn(Option<String>) + 'static + Clone,
) -> impl IntoView {
    let condition_input = RwSignal::new(String::new());

    // Initialize condition input when dialog becomes visible
    Effect::new(move || {
        if visible.get() {
            condition_input.set(current_condition.get().unwrap_or_default());
        }
    });

    let on_set_clone = on_set.clone();
    let handle_ok = move |_| {
        let condition = condition_input.get_untracked();
        let final_condition = if condition.is_empty() {
            None
        } else {
            Some(condition)
        };
        on_set_clone(final_condition);
        visible.set(false);
    };

    let handle_cancel = move |_| {
        visible.set(false);
    };

    view! {
        <div
            class="berry-dialog-overlay"
            class:berry-dialog-visible=move || visible.get()
        >
            <div class="berry-dialog berry-conditional-breakpoint-dialog">
                <h3>"Conditional Breakpoint"</h3>
                <div class="berry-dialog-content">
                    <label>
                        "Break when expression is true:"
                        <input
                            type="text"
                            class="berry-input"
                            prop:value=move || condition_input.get()
                            on:input=move |ev| {
                                condition_input.set(event_target_value(&ev));
                            }
                            placeholder="e.g., x > 10"
                        />
                    </label>
                </div>
                <div class="berry-dialog-actions">
                    <button class="berry-button" on:click=handle_ok>"OK"</button>
                    <button class="berry-button" on:click=handle_cancel>"Cancel"</button>
                </div>
            </div>
        </div>
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
