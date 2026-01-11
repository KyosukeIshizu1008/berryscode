//! Reusable UI components
//!
//! Common UI components to ensure consistent styling and eliminate duplication.

use leptos::prelude::*;

/// Generic panel component with title and children
#[component]
pub fn Panel(
    /// Panel title
    title: &'static str,
    /// Child content
    children: Children,
) -> impl IntoView
{
    view! {
        <div class="berry-panel">
            <div class="berry-panel-header">{title}</div>
            <div class="berry-panel-content">{children()}</div>
        </div>
    }
}

/// Standard button component
#[component]
pub fn Button(
    /// Button label
    label: &'static str,
    /// Click handler
    on_click: impl Fn() + 'static,
) -> impl IntoView {
    view! {
        <button
            class="berry-button"
            on:click=move |_| on_click()
        >
            {label}
        </button>
    }
}

/// Icon button with tooltip
#[component]
pub fn IconButton(
    /// Icon character or emoji
    icon: &'static str,
    /// Tooltip text
    tooltip: &'static str,
    /// Click handler
    on_click: impl Fn() + 'static,
    /// Disabled state
    #[prop(optional, default = false)]
    disabled: bool,
) -> impl IntoView {
    view! {
        <button
            class="berry-icon-button"
            title=tooltip
            disabled=disabled
            on:click=move |_| on_click()
        >
            {icon}
        </button>
    }
}

/// SVG Icon button with tooltip (IntelliJ-style flat icons)
#[component]
pub fn SvgIconButton<IV>(
    /// SVG icon view
    icon: IV,
    /// Tooltip text
    tooltip: &'static str,
    /// Click handler
    on_click: impl Fn() + 'static,
    /// Disabled state
    #[prop(optional, default = false)]
    disabled: bool,
) -> impl IntoView
where
    IV: IntoView + 'static,
{
    view! {
        <button
            class="berry-icon-button"
            title=tooltip
            disabled=disabled
            on:click=move |_| on_click()
        >
            {icon}
        </button>
    }
}

/// Generic list view component
#[component]
pub fn ListView<T, F, IV>(
    /// List items
    #[prop(into)]
    items: Vec<T>,
    /// Item renderer
    render_item: F,
) -> impl IntoView
where
    T: Clone + 'static,
    F: Fn(T) -> IV + 'static,
    IV: IntoView,
{
    view! {
        <div class="berry-list-view">
            {items.into_iter().map(|item| {
                render_item(item)
            }).collect::<Vec<_>>()}
        </div>
    }
}

/// Text input component
#[component]
pub fn TextInput(
    /// Input value signal
    value: RwSignal<String>,
    /// Placeholder text
    placeholder: &'static str,
) -> impl IntoView {
    view! {
        <input
            type="text"
            class="berry-text-input"
            placeholder=placeholder
            prop:value=move || value.get()
            on:input=move |ev| {
                value.set(event_target_value(&ev));
            }
        />
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
