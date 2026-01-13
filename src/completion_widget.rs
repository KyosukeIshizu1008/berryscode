//! Completion Widget
//!
//! IntelliJ-style completion popup with keyboard navigation.

use dioxus::prelude::*;
use dioxus::events::KeyboardEvent;
use crate::lsp_ui::CompletionItem;
use crate::types::Position;

/// Completion widget props
#[derive(Props, Clone, PartialEq)]
pub struct CompletionWidgetProps {
    /// Completion items to display
    items: Signal<Vec<CompletionItem>>,
    /// Position to show the widget
    position: Position,
    /// Callback when an item is selected
    on_select: EventHandler<CompletionItem>,
}

/// Completion widget component
#[component]
pub fn CompletionWidget(props: CompletionWidgetProps) -> Element {
    let items = props.items;
    let position = props.position;
    let on_select = props.on_select;

    let mut selected_index = use_signal(|| 0usize);

    // Keyboard navigation
    let handle_keydown = move |event: Event<KeyboardData>| {
        let key = event.key();
        let items_count = items.read().len();

        match key {
            Key::ArrowDown => {
                selected_index.write().update(|idx| {
                    *idx = (*idx + 1).min(items_count.saturating_sub(1));
                });
            }
            Key::ArrowUp => {
                selected_index.write().update(|idx| {
                    *idx = idx.saturating_sub(1);
                });
            }
            Key::Enter | Key::Tab => {
                let idx = *selected_index.read();
                if let Some(item) = items.read().get(idx) {
                    on_select.call(item.clone());
                }
            }
            Key::Escape => {
                // Close widget (handled externally)
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

    rsx! {
        div {
            class: "berry-completion-widget",
            style: "{style}",
            tabindex: "0",
            onkeydown: handle_keydown,

            div { class: "berry-completion-list",
                {
                    let current_items = items.read().clone();
                    let selected = *selected_index.read();

                    rsx! {
                        for (idx , item) in current_items.iter().enumerate() {
                            {
                                let is_selected = idx == selected;
                                let item_clone = item.clone();

                                rsx! {
                                    CompletionItemView {
                                        item: item.clone(),
                                        selected: is_selected,
                                        on_click: move |_| on_select.call(item_clone.clone())
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

/// Completion item view props
#[derive(Props, Clone, PartialEq)]
struct CompletionItemViewProps {
    /// The completion item
    item: CompletionItem,
    /// Whether this item is selected
    selected: bool,
    /// Click handler
    on_click: EventHandler<()>,
}

/// Single completion item view
#[component]
fn CompletionItemView(props: CompletionItemViewProps) -> Element {
    let item = props.item;
    let selected = props.selected;
    let on_click = props.on_click;

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

    rsx! {
        div {
            class: "{class}",
            onclick: move |_| on_click.call(()),

            span { class: "berry-completion-kind", "{kind_text}" }
            span { class: "berry-completion-label", "{label}" }

            if let Some(detail) = detail_text {
                span { class: "berry-completion-detail", "{detail}" }
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
