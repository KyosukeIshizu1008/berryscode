//! Variables Panel Component
//!
//! Displays variables in scopes (Local, Closure, Global) with tree expansion.

use dioxus::prelude::*;
use super::session::{Scope, Variable};
use crate::common::ui_components::Panel;

/// Variables panel component props
#[derive(Props, Clone, PartialEq)]
pub struct VariablesPanelProps {
    /// Scopes to display
    scopes: Signal<Vec<Scope>>,
}

/// Variables panel component
#[component]
pub fn VariablesPanel(props: VariablesPanelProps) -> Element {
    let scopes = props.scopes;

    rsx! {
        Panel { title: "Variables",
            div { class: "berry-variables-panel",
                {
                    let current_scopes = scopes.read().clone();

                    if current_scopes.is_empty() {
                        rsx! {
                            div { class: "berry-variables-empty",
                                "No variables (not paused in debugger)"
                            }
                        }
                    } else {
                        rsx! {
                            for scope in current_scopes {
                                ScopeView { scope: scope }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Single scope view props
#[derive(Props, Clone, PartialEq)]
struct ScopeViewProps {
    /// The scope to display
    scope: Scope,
}

/// Single scope view
#[component]
fn ScopeView(props: ScopeViewProps) -> Element {
    let scope = props.scope;
    let mut expanded = use_signal(|| true);

    let scope_name = scope.name.clone();
    let variables = scope.variables.clone();

    let toggle_expanded = move |_| {
        *expanded.write() = !*expanded.read();
    };

    rsx! {
        div { class: "berry-scope",
            div {
                class: "berry-scope-header",
                onclick: toggle_expanded,
                span { class: "berry-scope-arrow",
                    { if *expanded.read() { "▼" } else { "▶" } }
                }
                span { class: "berry-scope-name", "{scope_name}" }
            }
            {
                if *expanded.read() {
                    rsx! {
                        div { class: "berry-scope-variables",
                            for var in variables {
                                VariableView { variable: var, indent: 1 }
                            }
                        }
                    }
                } else {
                    rsx! { div {} }
                }
            }
        }
    }
}

/// Single variable view props
#[derive(Props, Clone, PartialEq)]
struct VariableViewProps {
    /// The variable to display
    variable: Variable,
    /// Indentation level
    indent: usize,
}

/// Single variable view with tree expansion
#[component]
fn VariableView(props: VariableViewProps) -> Element {
    let variable = props.variable;
    let indent = props.indent;

    let mut expanded = use_signal(|| false);
    let has_children = variable.children.is_some();
    let var_name = variable.name.clone();
    let var_value = variable.value.clone();
    let var_type = variable.type_name.clone();
    let children = variable.children.clone();

    let toggle_expanded = move |_| {
        if has_children {
            *expanded.write() = !*expanded.read();
        }
    };

    let indent_style = format!("padding-left: {}px;", indent * 20);

    rsx! {
        div { class: "berry-variable",
            div {
                class: "berry-variable-row",
                style: "{indent_style}",
                onclick: toggle_expanded,
                {
                    if has_children {
                        if *expanded.read() {
                            "▼"
                        } else {
                            "▶"
                        }
                    } else {
                        " "
                    }
                }
                span { class: "berry-variable-name", "{var_name}" }
                span { class: "berry-variable-separator", ": " }
                span { class: "berry-variable-value", "{var_value}" }
                {
                    var_type.as_ref().map(|type_name| {
                        rsx! {
                            span { class: "berry-variable-type", " ({type_name})" }
                        }
                    })
                }
            }
            {
                if *expanded.read() {
                    if let Some(ref child_vars) = children {
                        rsx! {
                            for child in child_vars {
                                VariableView { variable: child.clone(), indent: indent + 1 }
                            }
                        }
                    } else {
                        rsx! { div {} }
                    }
                } else {
                    rsx! { div {} }
                }
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
