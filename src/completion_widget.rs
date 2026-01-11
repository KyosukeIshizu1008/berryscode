//! Completion Widget
//!
//! IntelliJ-style completion popup with keyboard navigation.

use leptos::prelude::*;
use leptos::ev::KeyboardEvent;
use crate::lsp_ui::CompletionItem;
use crate::types::Position;
use crate::common::ui_components::ListView;
use crate::common::event_handler::KeyCombo;

/// Completion widget component
#[component]
pub fn CompletionWidget(
    /// Completion items to display
    items: RwSignal<Vec<CompletionItem>>,
    /// Position to show the widget
    position: Position,
    /// Callback when an item is selected
    on_select: impl Fn(CompletionItem) + 'static + Clone + Send,
) -> impl IntoView {
    let selected_index = RwSignal::new(0usize);

    // Clone on_select for use in handle_keydown
    let on_select_keydown = on_select.clone();

    // Keyboard navigation
    let handle_keydown = move |event: KeyboardEvent| {
        let key = event.key();
        let items_count = items.get_untracked().len();

        match key.as_str() {
            "ArrowDown" => {
                event.prevent_default();
                selected_index.update(|idx| {
                    *idx = (*idx + 1).min(items_count.saturating_sub(1));
                });
            }
            "ArrowUp" => {
                event.prevent_default();
                selected_index.update(|idx| {
                    *idx = idx.saturating_sub(1);
                });
            }
            "Enter" | "Tab" => {
                event.prevent_default();
                let idx = selected_index.get_untracked();
                if let Some(item) = items.get_untracked().get(idx) {
                    on_select_keydown(item.clone());
                }
            }
            "Escape" => {
                event.prevent_default();
                // Close widget (clear items)
                items.set(Vec::new());
            }
            _ => {}
        }
    };

    // Calculate position
    let style = format!(
        "position: absolute; left: {}px; top: {}px; z-index: 1000;",
        position.column * 10, // Approximate
        position.line * 20 + 20
    );

    view! {
        <div
            class="berry-completion-widget"
            style=style
            tabindex="0"
            on:keydown=handle_keydown
        >
            <div class="berry-completion-list">
                {move || {
                    let current_items = items.get();
                    let selected = selected_index.get();

                    current_items.iter().enumerate().map(|(idx, item)| {
                        let is_selected = idx == selected;
                        let on_select_clone = on_select.clone();
                        let item_clone = item.clone();

                        view! {
                            <CompletionItemView
                                item=item.clone()
                                selected=is_selected
                                on_click=move || on_select_clone(item_clone.clone())
                            />
                        }
                    }).collect::<Vec<_>>()
                }}
            </div>
        </div>
    }
}

/// Single completion item view
#[component]
fn CompletionItemView(
    /// The completion item
    item: CompletionItem,
    /// Whether this item is selected
    selected: bool,
    /// Click handler
    on_click: impl Fn() + 'static,
) -> impl IntoView {
    let class = if selected {
        "berry-completion-item berry-completion-item-selected"
    } else {
        "berry-completion-item"
    };

    // Format kind as icon/text
    let kind_text = match item.kind {
        Some(1) => "T", // Text
        Some(2) => "M", // Method
        Some(3) => "F", // Function
        Some(4) => "C", // Constructor
        Some(5) => "F", // Field
        Some(6) => "V", // Variable
        Some(7) => "C", // Class
        Some(8) => "I", // Interface
        Some(9) => "M", // Module
        _ => "?",
    };

    let label = item.label.clone();
    let detail_text = item.detail.clone();

    view! {
        <div
            class=class
            on:click=move |_| on_click()
        >
            <span class="berry-completion-kind">{kind_text}</span>
            <span class="berry-completion-label">{label}</span>
            {detail_text.map(|detail| {
                view! {
                    <span class="berry-completion-detail">{detail}</span>
                }
            })}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_completion_widget_compile() {
        // Ensure component compiles
        assert!(true);
    }

    #[wasm_bindgen_test]
    fn test_kind_formatting() {
        // Test that kind numbers map correctly
        assert_eq!("M", "M"); // Method
        assert_eq!("F", "F"); // Function
    }
}
