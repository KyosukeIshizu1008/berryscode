//! Refactor Menu Component
//! Context menu for refactoring operations

use dioxus::prelude::*;
use super::{RefactorOperation, Position, Range};

#[derive(Debug, Clone, PartialEq)]
pub struct RefactorContext {
    pub file_path: String,
    pub position: Position,
    pub selection: Option<Range>,
}

/// Refactor Menu props
#[derive(Props, Clone, PartialEq)]
pub struct RefactorMenuProps {
    context: RefactorContext,
    on_select: EventHandler<RefactorOperation>,
    on_close: EventHandler<()>,
}

#[component]
pub fn RefactorMenu(props: RefactorMenuProps) -> Element {
    let context = props.context;
    let on_select = props.on_select;
    let on_close = props.on_close;

    let available_operations = vec![
        RefactorOperation::Rename,
        RefactorOperation::ExtractMethod,
        RefactorOperation::InlineVariable,
        RefactorOperation::OptimizeImports,
        RefactorOperation::MoveSymbol,
        RefactorOperation::ChangeSignature,
    ];

    rsx! {
        div {
            class: "refactor-menu",
            style: "position: absolute; background: var(--bg-sidebar); border: 1px solid var(--border-color); border-radius: 4px; padding: 4px 0; min-width: 200px; box-shadow: 0 2px 8px rgba(0,0,0,0.3); z-index: 1000;",

            div {
                style: "padding: 8px 12px; color: var(--icon-muted); font-size: 11px; font-weight: bold; border-bottom: 1px solid var(--border-color);",
                "REFACTOR"
            }

            for op in available_operations {
                {
                    let op_clone = op;
                    rsx! {
                        div {
                            class: "refactor-menu-item",
                            style: "padding: 6px 12px; cursor: pointer; display: flex; justify-content: space-between; align-items: center; color: var(--tree-text); font-size: 13px;",
                            onclick: move |_| on_select.call(op_clone),

                            span { "{op.label()}" }
                            span {
                                style: "color: var(--icon-muted); font-size: 11px; margin-left: 16px;",
                                "{op.shortcut()}"
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Refactor Dialog props
#[derive(Props, Clone, PartialEq)]
pub struct RefactorDialogProps {
    operation: RefactorOperation,
    context: RefactorContext,
    on_apply: EventHandler<RefactorParams>,
    on_cancel: EventHandler<()>,
}

#[component]
pub fn RefactorDialog(props: RefactorDialogProps) -> Element {
    let operation = props.operation;
    let context = props.context;
    let on_apply = props.on_apply;
    let on_cancel = props.on_cancel;

    let mut input_value = use_signal(|| String::new());

    rsx! {
        div {
            class: "refactor-dialog-overlay",
            style: "position: fixed; top: 0; left: 0; right: 0; bottom: 0; background: rgba(0,0,0,0.5); display: flex; align-items: center; justify-content: center; z-index: 2000;",

            div {
                class: "refactor-dialog",
                style: "background: var(--bg-sidebar); border: 1px solid var(--border-color); border-radius: 6px; min-width: 400px; max-width: 600px; padding: 16px; box-shadow: 0 4px 16px rgba(0,0,0,0.5);",

                div {
                    style: "font-size: 16px; font-weight: bold; color: var(--tree-text); margin-bottom: 16px;",
                    "{operation.label()}"
                }

                {
                    match operation {
                        RefactorOperation::Rename => rsx! {
                            div {
                                label {
                                    style: "display: block; color: var(--tree-text); font-size: 13px; margin-bottom: 8px;",
                                    "New name:"
                                }
                                input {
                                    r#type: "text",
                                    value: "{input_value.read()}",
                                    oninput: move |e| *input_value.write() = e.value(),
                                    class: "input-field",
                                    placeholder: "Enter new name",
                                }
                            }
                        },
                        RefactorOperation::ExtractMethod => rsx! {
                            div {
                                label {
                                    style: "display: block; color: var(--tree-text); font-size: 13px; margin-bottom: 8px;",
                                    "Method name:"
                                }
                                input {
                                    r#type: "text",
                                    value: "{input_value.read()}",
                                    oninput: move |e| *input_value.write() = e.value(),
                                    class: "input-field",
                                    placeholder: "Enter method name",
                                }
                            }
                        },
                        RefactorOperation::ChangeSignature => rsx! {
                            div {
                                label {
                                    style: "display: block; color: var(--tree-text); font-size: 13px; margin-bottom: 8px;",
                                    "New signature:"
                                }
                                input {
                                    r#type: "text",
                                    value: "{input_value.read()}",
                                    oninput: move |e| *input_value.write() = e.value(),
                                    class: "input-field",
                                    placeholder: "fn name(params)",
                                }
                            }
                        },
                        _ => rsx! {
                            div {
                                style: "color: var(--tree-text); font-size: 13px;",
                                "Apply {operation.label()}?"
                            }
                        },
                    }
                }

                div {
                    style: "display: flex; justify-content: flex-end; gap: 8px; margin-top: 16px;",

                    button {
                        onclick: move |_| on_cancel.call(()),
                        style: "padding: 6px 16px; background: var(--bg-tab-hover); border: 1px solid var(--border-color); border-radius: 4px; color: var(--tree-text); cursor: pointer; font-size: 13px;",
                        "Cancel"
                    }

                    button {
                        onclick: move |_| {
                            let params = match operation {
                                RefactorOperation::Rename => RefactorParams::Rename {
                                    new_name: input_value.read().clone(),
                                },
                                RefactorOperation::ExtractMethod => RefactorParams::ExtractMethod {
                                    method_name: input_value.read().clone(),
                                    range: context.selection.clone().unwrap_or(Range {
                                        start: context.position.clone(),
                                        end: context.position.clone(),
                                    }),
                                },
                                RefactorOperation::ChangeSignature => RefactorParams::ChangeSignature {
                                    new_signature: input_value.read().clone(),
                                },
                                _ => RefactorParams::Simple,
                            };
                            on_apply.call(params);
                        },
                        style: "padding: 6px 16px; background: var(--color-accent-secondary); border: 1px solid var(--color-accent-secondary); border-radius: 4px; color: var(--tree-text-active); cursor: pointer; font-size: 13px; font-weight: bold;",
                        "Apply"
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RefactorParams {
    Rename { new_name: String },
    ExtractMethod { method_name: String, range: Range },
    InlineVariable,
    OptimizeImports,
    MoveSymbol { target_file: String },
    ChangeSignature { new_signature: String },
    Simple,
}
