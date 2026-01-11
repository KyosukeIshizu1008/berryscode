//! Watch Panel Component
//!
//! Displays watch expressions and their evaluated values.

use leptos::prelude::*;
use leptos::task::spawn_local;
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

/// Watch panel component
#[component]
pub fn WatchPanel(
    /// Watch expressions
    watches: RwSignal<Vec<WatchExpression>>,
    /// Debug session for evaluation
    session: DebugSession,
) -> impl IntoView {
    let new_expression = RwSignal::new(String::new());

    // Add new watch expression
    let add_watch = move || {
        let expr = new_expression.get_untracked();
        if !expr.is_empty() {
            let watch = WatchExpression {
                id: uuid::Uuid::new_v4().to_string(),
                expression: expr.clone(),
                value: None,
                error: None,
            };

            watches.update(|w| w.push(watch.clone()));
            new_expression.set(String::new());

            // Evaluate immediately if debugging
            let watch_clone = watch.clone();
            spawn_local(async move {
                if session.session_id.get_untracked().is_some() {
                    match session.evaluate(watch_clone.expression.clone(), None).await {
                        Ok(result) => {
                            watches.update(|w| {
                                if let Some(w) = w.iter_mut().find(|w| w.id == watch_clone.id) {
                                    w.value = Some(result);
                                    w.error = None;
                                }
                            });
                        }
                        Err(e) => {
                            watches.update(|w| {
                                if let Some(w) = w.iter_mut().find(|w| w.id == watch_clone.id) {
                                    w.value = None;
                                    w.error = Some(e);
                                }
                            });
                        }
                    }
                }
            });
        }
    };

    view! {
        <Panel title="Watch">
            <div class="berry-watch-panel">
                <div class="berry-watch-add">
                    <input
                        type="text"
                        class="berry-input"
                        prop:value=move || new_expression.get()
                        on:input=move |ev| {
                            new_expression.set(event_target_value(&ev));
                        }
                        on:keydown=move |ev| {
                            if ev.key() == "Enter" {
                                add_watch();
                            }
                        }
                        placeholder="Add watch expression..."
                    />
                    <button class="berry-button" on:click=move |_| add_watch()>"+"</button>
                </div>
                <div class="berry-watch-list">
                    {move || {
                        let current_watches = watches.get();

                        if current_watches.is_empty() {
                            view! {
                                <div class="berry-watch-empty">
                                    "No watch expressions"
                                </div>
                            }.into_any()
                        } else {
                            current_watches.iter().map(|watch| {
                                let watch_clone = watch.clone();
                                view! {
                                    <WatchExpressionView
                                        watch=watch.clone()
                                        on_remove=move || {
                                            let id = watch_clone.id.clone();
                                            watches.update(|w| w.retain(|w| w.id != id));
                                        }
                                    />
                                }
                            }).collect::<Vec<_>>().into_any()
                        }
                    }}
                </div>
            </div>
        </Panel>
    }
}

/// Single watch expression view
#[component]
fn WatchExpressionView(
    /// The watch expression
    watch: WatchExpression,
    /// Remove callback
    on_remove: impl Fn() + 'static,
) -> impl IntoView {
    let expression = watch.expression.clone();
    let value_text = watch.value.clone();
    let error_text = watch.error.clone();

    view! {
        <div class="berry-watch-expression">
            <div class="berry-watch-expr-name">{expression}</div>
            <div class="berry-watch-expr-value">
                {if let Some(value) = value_text {
                    view! {
                        <span class="berry-watch-value">{value}</span>
                    }.into_any()
                } else if let Some(error) = error_text {
                    view! {
                        <span class="berry-watch-error">{error}</span>
                    }.into_any()
                } else {
                    view! {
                        <span class="berry-watch-not-evaluated">"not evaluated"</span>
                    }.into_any()
                }}
            </div>
            <button
                class="berry-watch-remove"
                on:click=move |_| on_remove()
                title="Remove watch"
            >
                "×"
            </button>
        </div>
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
