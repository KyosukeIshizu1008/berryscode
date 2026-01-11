//! Variables Panel Component
//!
//! Displays variables in scopes (Local, Closure, Global) with tree expansion.

use leptos::prelude::*;
use super::session::{Scope, Variable};
use crate::common::ui_components::Panel;

/// Variables panel component
#[component]
pub fn VariablesPanel(
    /// Scopes to display
    scopes: RwSignal<Vec<Scope>>,
) -> impl IntoView {
    view! {
        <Panel title="Variables">
            <div class="berry-variables-panel">
                {move || {
                    let current_scopes = scopes.get();

                    if current_scopes.is_empty() {
                        view! {
                            <div class="berry-variables-empty">
                                "No variables (not paused in debugger)"
                            </div>
                        }.into_any()
                    } else {
                        current_scopes.iter().map(|scope| {
                            view! {
                                <ScopeView scope=scope.clone() />
                            }
                        }).collect::<Vec<_>>().into_any()
                    }
                }}
            </div>
        </Panel>
    }
}

/// Single scope view
#[component]
fn ScopeView(
    /// The scope to display
    scope: Scope,
) -> impl IntoView {
    let expanded = RwSignal::new(true);
    let scope_name = scope.name.clone();

    let toggle_expanded = move |_| {
        expanded.update(|e| *e = !*e);
    };

    view! {
        <div class="berry-scope">
            <div
                class="berry-scope-header"
                on:click=toggle_expanded
            >
                <span class="berry-scope-arrow">
                    {move || if expanded.get() { "▼" } else { "▶" }}
                </span>
                <span class="berry-scope-name">{scope_name}</span>
            </div>
            {move || {
                if expanded.get() {
                    view! {
                        <div class="berry-scope-variables">
                            {scope.variables.iter().map(|var| {
                                view! {
                                    <VariableView variable=var.clone() indent=1 />
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    }.into_any()
                } else {
                    view! { <div></div> }.into_any()
                }
            }}
        </div>
    }
}

/// Single variable view with tree expansion
#[component]
fn VariableView(
    /// The variable to display
    variable: Variable,
    /// Indentation level
    indent: usize,
) -> impl IntoView {
    let expanded = RwSignal::new(false);
    let has_children = variable.children.is_some();
    let var_name = variable.name.clone();
    let var_value = variable.value.clone();

    let toggle_expanded = move |_| {
        if has_children {
            expanded.update(|e| *e = !*e);
        }
    };

    let indent_style = format!("padding-left: {}px;", indent * 20);

    view! {
        <div class="berry-variable">
            <div
                class="berry-variable-row"
                style=indent_style
                on:click=toggle_expanded
            >
                {move || {
                    if has_children {
                        if expanded.get() {
                            "▼"
                        } else {
                            "▶"
                        }
                    } else {
                        " "
                    }
                }}
                <span class="berry-variable-name">{var_name}</span>
                <span class="berry-variable-separator">": "</span>
                <span class="berry-variable-value">{var_value}</span>
                {variable.type_name.as_ref().map(|type_name| {
                    view! {
                        <span class="berry-variable-type">{format!(" ({})", type_name)}</span>
                    }
                })}
            </div>
            {move || {
                if expanded.get() {
                    if let Some(ref children) = variable.children {
                        children.iter().map(|child| {
                            view! {
                                <VariableView variable=child.clone() indent=indent + 1 />
                            }
                        }).collect::<Vec<_>>().into_any()
                    } else {
                        view! { <div></div> }.into_any()
                    }
                } else {
                    view! { <div></div> }.into_any()
                }
            }}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_variables_panel_compiles() {
        // Ensure component compiles
        assert!(true);
    }

    #[test]
    fn test_scope_creation() {
        let scope = Scope {
            name: "Local".to_string(),
            variables: vec![],
        };

        assert_eq!(scope.name, "Local");
        assert_eq!(scope.variables.len(), 0);
    }

    #[test]
    fn test_variable_with_type() {
        let var = Variable {
            name: "x".to_string(),
            value: "42".to_string(),
            type_name: Some("i32".to_string()),
            children: None,
        };

        assert_eq!(var.name, "x");
        assert_eq!(var.value, "42");
        assert!(var.type_name.is_some());
        assert_eq!(var.type_name.unwrap(), "i32");
    }

    #[test]
    fn test_variable_tree_structure() {
        let child = Variable {
            name: "field".to_string(),
            value: "10".to_string(),
            type_name: None,
            children: None,
        };

        let parent = Variable {
            name: "struct_var".to_string(),
            value: "MyStruct".to_string(),
            type_name: Some("MyStruct".to_string()),
            children: Some(vec![child]),
        };

        assert!(parent.children.is_some());
        assert_eq!(parent.children.as_ref().unwrap().len(), 1);
        assert_eq!(parent.children.as_ref().unwrap()[0].name, "field");
    }

    #[test]
    fn test_indent_calculation() {
        let indent_level_1 = 1 * 20;
        let indent_level_2 = 2 * 20;
        let indent_level_3 = 3 * 20;

        assert_eq!(indent_level_1, 20);
        assert_eq!(indent_level_2, 40);
        assert_eq!(indent_level_3, 60);
    }
}
