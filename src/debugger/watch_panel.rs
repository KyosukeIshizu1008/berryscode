//! Watch Panel Component
//!
//! Displays watch expressions and their evaluated values.

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use crate::common::ui_components::Panel;
use super::session::DebugSession;

/// Watch expression
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WatchExpression {
    pub id: String,
    pub expression: String,
    pub value: Option<String>,
    pub error: Option<String>,
}

/// Watch panel component props
#[derive(Props, Clone, PartialEq)]
pub struct WatchPanelProps {
    /// Watch expressions
    watches: Signal<Vec<WatchExpression>>,
    /// Debug session for evaluation
    session: DebugSession,
}

/// Watch panel component
#[component]
pub fn WatchPanel(props: WatchPanelProps) -> Element {
    let watches = props.watches;
    let session = props.session;

    let mut new_expression = use_signal(|| String::new());

    // Add new watch expression
    let add_watch = move || {
        let expr = new_expression.read().clone();
        if !expr.is_empty() {
            let watch = WatchExpression {
                id: uuid::Uuid::new_v4().to_string(),
                expression: expr.clone(),
                value: None,
                error: None,
            };

            watches.write().push(watch.clone());
            *new_expression.write() = String::new();

            // Evaluate immediately if debugging
            let watch_clone = watch.clone();
            spawn(async move {
                if session.session_id.read().is_some() {
                    match session.evaluate(watch_clone.expression.clone(), None).await {
                        Ok(result) => {
                            let mut w = watches.write();
                            if let Some(watch) = w.iter_mut().find(|w| w.id == watch_clone.id) {
                                watch.value = Some(result);
                                watch.error = None;
                            }
                        }
                        Err(e) => {
                            let mut w = watches.write();
                            if let Some(watch) = w.iter_mut().find(|w| w.id == watch_clone.id) {
                                watch.value = None;
                                watch.error = Some(e);
                            }
                        }
                    }
                }
            });
        }
    };

    rsx! {
        Panel { title: "Watch",
            div { class: "berry-watch-panel",
                div { class: "berry-watch-add",
                    input {
                        r#type: "text",
                        class: "berry-input",
                        value: "{new_expression.read()}",
                        oninput: move |ev| *new_expression.write() = ev.value(),
                        onkeydown: move |ev| {
                            if ev.key() == Key::Enter {
                                add_watch();
                            }
                        },
                        placeholder: "Add watch expression...",
                    }
                    button {
                        class: "berry-button",
                        onclick: move |_| add_watch(),
                        "+"
                    }
                }
                div { class: "berry-watch-list",
                    {
                        let current_watches = watches.read().clone();

                        if current_watches.is_empty() {
                            rsx! {
                                div { class: "berry-watch-empty",
                                    "No watch expressions"
                                }
                            }
                        } else {
                            rsx! {
                                for watch in current_watches {
                                    {
                                        let watch_clone = watch.clone();
                                        rsx! {
                                            WatchExpressionView {
                                                watch: watch.clone(),
                                                on_remove: move |_| {
                                                    let id = watch_clone.id.clone();
                                                    watches.write().retain(|w| w.id != id);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Single watch expression view props
#[derive(Props, Clone, PartialEq)]
struct WatchExpressionViewProps {
    /// The watch expression
    watch: WatchExpression,
    /// Remove callback
    on_remove: EventHandler<()>,
}

/// Single watch expression view
#[component]
fn WatchExpressionView(props: WatchExpressionViewProps) -> Element {
    let watch = props.watch;
    let on_remove = props.on_remove;

    let expression = watch.expression.clone();
    let value_text = watch.value.clone();
    let error_text = watch.error.clone();

    rsx! {
        div { class: "berry-watch-expression",
            div { class: "berry-watch-expr-name", "{expression}" }
            div { class: "berry-watch-expr-value",
                {
                    if let Some(value) = value_text {
                        rsx! {
                            span { class: "berry-watch-value", "{value}" }
                        }
                    } else if let Some(error) = error_text {
                        rsx! {
                            span { class: "berry-watch-error", "{error}" }
                        }
                    } else {
                        rsx! {
                            span { class: "berry-watch-not-evaluated", "not evaluated" }
                        }
                    }
                }
            }
            button {
                class: "berry-watch-remove",
                onclick: move |_| on_remove.call(()),
                title: "Remove watch",
                "×"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_watch_panel_compiles() {
        // Ensure component compiles
        assert!(true);
    }

    #[test]
    fn test_watch_expression_creation() {
        let watch = WatchExpression {
            id: "1".to_string(),
            expression: "x + y".to_string(),
            value: None,
            error: None,
        };

        assert_eq!(watch.expression, "x + y");
        assert!(watch.value.is_none());
        assert!(watch.error.is_none());
    }

    #[test]
    fn test_watch_expression_with_value() {
        let watch = WatchExpression {
            id: "1".to_string(),
            expression: "x".to_string(),
            value: Some("42".to_string()),
            error: None,
        };

        assert!(watch.value.is_some());
        assert_eq!(watch.value.as_ref().unwrap(), "42");
    }

    #[test]
    fn test_watch_expression_with_error() {
        let watch = WatchExpression {
            id: "1".to_string(),
            expression: "invalid".to_string(),
            value: None,
            error: Some("undefined variable".to_string()),
        };

        assert!(watch.error.is_some());
        assert_eq!(watch.error.as_ref().unwrap(), "undefined variable");
    }

    #[test]
    fn test_watch_expression_equality() {
        let watch1 = WatchExpression {
            id: "1".to_string(),
            expression: "x".to_string(),
            value: None,
            error: None,
        };

        let watch2 = WatchExpression {
            id: "1".to_string(),
            expression: "x".to_string(),
            value: None,
            error: None,
        };

        assert_eq!(watch1, watch2);
    }
}
