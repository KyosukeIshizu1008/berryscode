//! Refactoring Preview Dialog
//! Shows diff of changes before applying

use dioxus::prelude::*;
use super::{WorkspaceEdit, TextEdit};
use std::collections::HashMap;

/// Refactoring Preview props
#[derive(Props, Clone, PartialEq)]
pub struct RefactoringPreviewProps {
    changes: WorkspaceEdit,
    on_apply: EventHandler<()>,
    on_cancel: EventHandler<()>,
}

#[component]
pub fn RefactoringPreview(props: RefactoringPreviewProps) -> Element {
    let changes = props.changes;
    let on_apply = props.on_apply;
    let on_cancel = props.on_cancel;

    let mut changes_signal = use_signal(|| changes.changes.clone());
    let mut selected_file = use_signal(|| {
        changes_signal.read().keys().next().cloned().unwrap_or_default()
    });

    rsx! {
        div {
            class: "refactoring-preview-overlay",
            style: "position: fixed; top: 0; left: 0; right: 0; bottom: 0; background: rgba(0,0,0,0.5); display: flex; align-items: center; justify-content: center; z-index: 2000;",

            div {
                class: "refactoring-preview",
                style: "background: var(--bg-sidebar); border: 1px solid var(--border-color); border-radius: 6px; width: 80%; max-width: 1000px; height: 80%; max-height: 800px; display: flex; flex-direction: column; box-shadow: 0 4px 16px rgba(0,0,0,0.5);",

                div {
                    style: "padding: 16px; border-bottom: 1px solid var(--border-color);",
                    div {
                        style: "font-size: 16px; font-weight: bold; color: var(--tree-text); margin-bottom: 8px;",
                        "Refactoring Preview"
                    }
                    div {
                        class: "text-muted text-base",
                        "{changes_signal.read().len()} file(s) will be changed"
                    }
                }

                div {
                    class: "flex-1 flex-row overflow-hidden",

                    div {
                        style: "width: 250px; border-right: 1px solid var(--border-color); overflow-y: auto;",
                        div {
                            style: "padding: 8px; color: var(--icon-muted); font-size: 11px; font-weight: bold;",
                            "MODIFIED FILES"
                        }
                        {
                            let files: Vec<(String, usize)> = changes_signal.read()
                                .iter()
                                .map(|(file, edits)| (file.clone(), edits.len()))
                                .collect();

                            rsx! {
                                for (file , edit_count) in files {
                                    {
                                        let file_clone = file.clone();
                                        let file_for_click = file.clone();
                                        let file_display = file.split('/').last().unwrap_or(&file).to_string();
                                        let is_selected = *selected_file.read() == file_clone;
                                        let bg = if is_selected { "#094771" } else { "transparent" };

                                        rsx! {
                                            div {
                                                class: "file-item",
                                                style: "padding: 8px 12px; cursor: pointer; color: var(--tree-text); font-size: 13px; background: {bg};",
                                                onclick: move |_| *selected_file.write() = file_for_click.clone(),

                                                div {
                                                    class: "font-medium",
                                                    "{file_display}"
                                                }
                                                div {
                                                    style: "color: var(--icon-muted); font-size: 11px; margin-top: 2px;",
                                                    "{edit_count} change(s)"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    div {
                        style: "flex: 1; overflow-y: auto; padding: 16px;",
                        {
                            let current_file = selected_file.read().clone();
                            if let Some(edits) = changes_signal.read().get(&current_file) {
                                rsx! {
                                    div {
                                        div {
                                            style: "color: var(--tree-text); font-size: 14px; font-weight: bold; margin-bottom: 16px;",
                                            "{current_file}"
                                        }
                                        for (idx , edit) in edits.iter().enumerate() {
                                            div {
                                                style: "margin-bottom: 16px; background: var(--bg-main); border: 1px solid var(--border-color); border-radius: 4px; overflow: hidden;",
                                                div {
                                                    style: "padding: 8px; background: var(--bg-sidebar); color: var(--icon-muted); font-size: 12px;",
                                                    "Change {idx + 1} - Line {edit.range.start.line + 1}"
                                                }
                                                div {
                                                    style: "padding: 12px;",
                                                    div {
                                                        style: "margin-bottom: 8px;",
                                                        div {
                                                            style: "color: var(--color-error); font-size: 11px; margin-bottom: 4px;",
                                                            "- OLD"
                                                        }
                                                        pre {
                                                            style: "margin: 0; padding: 8px; background: var(--color-bg-error); border-left: 3px solid var(--color-error); color: var(--tree-text); font-size: 12px; overflow-x: auto;",
                                                            "(removed text)"
                                                        }
                                                    }
                                                    div {
                                                        div {
                                                            style: "color: var(--color-success); font-size: 11px; margin-bottom: 4px;",
                                                            "+ NEW"
                                                        }
                                                        pre {
                                                            style: "margin: 0; padding: 8px; background: var(--color-bg-success); border-left: 3px solid var(--color-success); color: var(--tree-text); font-size: 12px; overflow-x: auto;",
                                                            "{edit.new_text}"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            } else {
                                rsx! {
                                    div {
                                        style: "color: var(--icon-muted); font-size: 13px; text-align: center; padding: 32px;",
                                        "No changes selected"
                                    }
                                }
                            }
                        }
                    }
                }

                div {
                    style: "padding: 16px; border-top: 1px solid var(--border-color); display: flex; justify-content: space-between; align-items: center;",
                    div {
                        class: "text-muted text-base",
                        "Review the changes carefully before applying"
                    }
                    div {
                        class: "flex-row flex-gap-8",
                        button {
                            onclick: move |_| on_cancel.call(()),
                            style: "padding: 8px 20px; background: var(--bg-tab-hover); border: 1px solid var(--border-color); border-radius: 4px; color: var(--tree-text); cursor: pointer; font-size: 13px;",
                            "Cancel"
                        }
                        button {
                            onclick: move |_| on_apply.call(()),
                            style: "padding: 8px 20px; background: var(--color-accent-secondary); border: 1px solid var(--color-accent-secondary); border-radius: 4px; color: var(--tree-text-active); cursor: pointer; font-size: 13px; font-weight: bold;",
                            "Apply Refactoring"
                        }
                    }
                }
            }
        }
    }
}

/// Diff View props
#[derive(Props, Clone, PartialEq)]
pub struct DiffViewProps {
    old_text: String,
    new_text: String,
}

#[component]
pub fn DiffView(props: DiffViewProps) -> Element {
    let old_text = props.old_text;
    let new_text = props.new_text;

    rsx! {
        div {
            class: "diff-view",
            style: "font-family: 'Courier New', monospace; font-size: 12px;",

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 16px;",
                div {
                    div {
                        style: "padding: 4px 8px; background: var(--color-bg-error); color: var(--color-error); font-weight: bold; border-bottom: 2px solid var(--color-error);",
                        "BEFORE"
                    }
                    pre {
                        style: "margin: 0; padding: 12px; background: var(--bg-main); color: var(--tree-text); overflow-x: auto; white-space: pre-wrap;",
                        "{old_text}"
                    }
                }
                div {
                    div {
                        style: "padding: 4px 8px; background: var(--color-bg-success); color: var(--color-success); font-weight: bold; border-bottom: 2px solid var(--color-success);",
                        "AFTER"
                    }
                    pre {
                        style: "margin: 0; padding: 12px; background: var(--bg-main); color: var(--tree-text); overflow-x: auto; white-space: pre-wrap;",
                        "{new_text}"
                    }
                }
            }
        }
    }
}
