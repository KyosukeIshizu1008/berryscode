//! Reusable UI components
//!
//! Common UI components to ensure consistent styling and eliminate duplication.

use dioxus::prelude::*;

/// Generic panel component with title and children
#[derive(Props, Clone, PartialEq)]
pub struct PanelProps {
    /// Panel title
    title: &'static str,
    /// Child content
    children: Element,
}

#[component]
pub fn Panel(props: PanelProps) -> Element {
    let title = props.title;
    let children = props.children;

    rsx! {
        div { class: "berry-panel",
            div { class: "berry-panel-header", "{title}" }
            div { class: "berry-panel-content", {children} }
        }
    }
}

/// Standard button component
#[derive(Props, Clone, PartialEq)]
pub struct ButtonProps {
    /// Button label
    label: &'static str,
    /// Click handler
    on_click: EventHandler<()>,
}

#[component]
pub fn Button(props: ButtonProps) -> Element {
    let label = props.label;
    let on_click = props.on_click;

    rsx! {
        button {
            class: "berry-button",
            onclick: move |_| on_click.call(()),
            "{label}"
        }
    }
}

/// Icon button with tooltip
#[derive(Props, Clone, PartialEq)]
pub struct IconButtonProps {
    /// Icon character or emoji
    icon: &'static str,
    /// Tooltip text
    tooltip: &'static str,
    /// Click handler
    on_click: EventHandler<()>,
    /// Disabled state
    #[props(default = false)]
    disabled: bool,
}

#[component]
pub fn IconButton(props: IconButtonProps) -> Element {
    let icon = props.icon;
    let tooltip = props.tooltip;
    let on_click = props.on_click;
    let disabled = props.disabled;

    rsx! {
        button {
            class: "berry-icon-button",
            title: "{tooltip}",
            disabled: disabled,
            onclick: move |_| on_click.call(()),
            "{icon}"
        }
    }
}

/// SVG Icon button with tooltip (IntelliJ-style flat icons)
#[derive(Props, Clone, PartialEq)]
pub struct SvgIconButtonProps {
    /// SVG icon element
    icon: Element,
    /// Tooltip text
    tooltip: &'static str,
    /// Click handler
    on_click: EventHandler<()>,
    /// Disabled state
    #[props(default = false)]
    disabled: bool,
}

#[component]
pub fn SvgIconButton(props: SvgIconButtonProps) -> Element {
    let icon = props.icon;
    let tooltip = props.tooltip;
    let on_click = props.on_click;
    let disabled = props.disabled;

    rsx! {
        button {
            class: "berry-icon-button",
            title: "{tooltip}",
            disabled: disabled,
            onclick: move |_| on_click.call(()),
            {icon}
        }
    }
}

/// Generic list view component
#[derive(Props)]
pub struct ListViewProps<T: Clone + PartialEq + 'static> {
    /// List items
    items: Vec<T>,
    /// Item renderer
    render_item: Callback<T, Element>,
}

#[component]
pub fn ListView<T: Clone + PartialEq + 'static>(props: ListViewProps<T>) -> Element {
    let items = props.items;
    let render_item = props.render_item;

    rsx! {
        div { class: "berry-list-view",
            for item in items {
                {render_item.call(item)}
            }
        }
    }
}

/// Text input component
#[derive(Props, Clone, PartialEq)]
pub struct TextInputProps {
    /// Input value signal
    value: Signal<String>,
    /// Placeholder text
    placeholder: &'static str,
}

#[component]
pub fn TextInput(props: TextInputProps) -> Element {
    let value = props.value;
    let placeholder = props.placeholder;

    rsx! {
        input {
            r#type: "text",
            class: "berry-text-input",
            placeholder: "{placeholder}",
            value: "{value.read()}",
            oninput: move |ev| *value.write() = ev.value(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_components_compile() {
        // Ensure components compile correctly
        assert!(true);
    }
}
