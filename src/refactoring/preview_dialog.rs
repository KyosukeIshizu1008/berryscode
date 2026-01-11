//! Refactoring Preview Dialog
//! Shows diff of changes before applying

use leptos::prelude::*;
use super::{WorkspaceEdit, TextEdit};
use std::collections::HashMap;

#[component]
pub fn RefactoringPreview(
    changes: WorkspaceEdit,
    on_apply: impl Fn() + 'static,
    on_cancel: impl Fn() + 'static,
) -> impl IntoView {
    let changes_signal = RwSignal::new(changes.changes.clone());
    let selected_file = RwSignal::new(
        changes_signal.get().keys().next().cloned().unwrap_or_default()
    );

    view! {
        <div class="refactoring-preview-overlay" style="position: fixed; top: 0; left: 0; right: 0; bottom: 0; background: rgba(0,0,0,0.5); display: flex; align-items: center; justify-content: center; z-index: 2000;">
            <div class="refactoring-preview" style="background: var(--bg-sidebar); border: 1px solid var(--border-color); border-radius: 6px; width: 80%; max-width: 1000px; height: 80%; max-height: 800px; display: flex; flex-direction: column; box-shadow: 0 4px 16px rgba(0,0,0,0.5);">

                <div style="padding: 16px; border-bottom: 1px solid var(--border-color);">
                    <div style="font-size: 16px; font-weight: bold; color: var(--tree-text); margin-bottom: 8px;">
                        "Refactoring Preview"
                    </div>
                    <div class="text-muted text-base">
                        {move || changes_signal.get().len()} " file(s) will be changed"
                    </div>
                </div>

                <div class="flex-1 flex-row overflow-hidden">

                    <div style="width: 250px; border-right: 1px solid var(--border-color); overflow-y: auto;">
                        <div style="padding: 8px; color: var(--icon-muted); font-size: 11px; font-weight: bold;">
                            "MODIFIED FILES"
                        </div>
                        {move || {
                            let files: Vec<(String, usize)> = changes_signal.get()
                                .iter()
                                .map(|(file, edits)| (file.clone(), edits.len()))
                                .collect();

                            files.into_iter().map(|(file, edit_count)| {
                                let file_clone = file.clone();
                                let file_for_click = file.clone();
                                let file_display = file.split('/').last().unwrap_or(&file).to_string();

                                view! {
                                    <div
                                        class="file-item"
                                        style:background=move || {
                                            if selected_file.get() == file_clone {
                                                "#094771"
                                            } else {
                                                "transparent"
                                            }
                                        }
                                        style="padding: 8px 12px; cursor: pointer; color: var(--tree-text); font-size: 13px;"
                                        on:click=move |_| {
                                            selected_file.set(file_for_click.clone());
                                        }
                                    >
                                        <div class="font-medium">
                                            {file_display}
                                        </div>
                                        <div style="color: var(--icon-muted); font-size: 11px; margin-top: 2px;">
                                            {edit_count} " change(s)"
                                        </div>
                                    </div>
                                }
                            }).collect::<Vec<_>>()
                        }}
                    </div>

                    <div style="flex: 1; overflow-y: auto; padding: 16px;">
                        {move || {
                            let current_file = selected_file.get();
                            if let Some(edits) = changes_signal.get().get(&current_file) {
                                view! {
                                    <div>
                                        <div style="color: var(--tree-text); font-size: 14px; font-weight: bold; margin-bottom: 16px;">
                                            {current_file.clone()}
                                        </div>
                                        {edits.iter().enumerate().map(|(idx, edit)| {
                                            view! {
                                                <div style="margin-bottom: 16px; background: var(--bg-main); border: 1px solid var(--border-color); border-radius: 4px; overflow: hidden;">
                                                    <div style="padding: 8px; background: var(--bg-sidebar); color: var(--icon-muted); font-size: 12px;">
                                                        "Change " {idx + 1} " - Line " {edit.range.start.line + 1}
                                                    </div>
                                                    <div style="padding: 12px;">
                                                        <div style="margin-bottom: 8px;">
                                                            <div style="color: var(--color-error); font-size: 11px; margin-bottom: 4px;">
                                                                "- OLD"
                                                            </div>
                                                            <pre style="margin: 0; padding: 8px; background: var(--color-bg-error); border-left: 3px solid var(--color-error); color: var(--tree-text); font-size: 12px; overflow-x: auto;">
                                                                "(removed text)"
                                                            </pre>
                                                        </div>
                                                        <div>
                                                            <div style="color: var(--color-success); font-size: 11px; margin-bottom: 4px;">
                                                                "+ NEW"
                                                            </div>
                                                            <pre style="margin: 0; padding: 8px; background: var(--color-bg-success); border-left: 3px solid var(--color-success); color: var(--tree-text); font-size: 12px; overflow-x: auto;">
                                                                {edit.new_text.clone()}
                                                            </pre>
                                                        </div>
                                                    </div>
                                                </div>
                                            }
                                        }).collect_view()}
                                    </div>
                                }.into_any()
                            } else {
                                view! {
                                    <div style="color: var(--icon-muted); font-size: 13px; text-align: center; padding: 32px;">
                                        "No changes selected"
                                    </div>
                                }.into_any()
                            }
                        }}
                    </div>
                </div>

                <div style="padding: 16px; border-top: 1px solid var(--border-color); display: flex; justify-content: space-between; align-items: center;">
                    <div class="text-muted text-base">
                        "Review the changes carefully before applying"
                    </div>
                    <div class="flex-row flex-gap-8">
                        <button
                            on:click=move |_| on_cancel()
                            style="padding: 8px 20px; background: var(--bg-tab-hover); border: 1px solid var(--border-color); border-radius: 4px; color: var(--tree-text); cursor: pointer; font-size: 13px;"
                        >
                            "Cancel"
                        </button>
                        <button
                            on:click=move |_| on_apply()
                            style="padding: 8px 20px; background: var(--color-accent-secondary); border: 1px solid var(--color-accent-secondary); border-radius: 4px; color: var(--tree-text-active); cursor: pointer; font-size: 13px; font-weight: bold;"
                        >
                            "Apply Refactoring"
                        </button>
                    </div>
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn DiffView(
    old_text: String,
    new_text: String,
) -> impl IntoView {
    view! {
        <div class="diff-view" style="font-family: 'Courier New', monospace; font-size: 12px;">
            <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 16px;">
                <div>
                    <div style="padding: 4px 8px; background: var(--color-bg-error); color: var(--color-error); font-weight: bold; border-bottom: 2px solid var(--color-error);">
                        "BEFORE"
                    </div>
                    <pre style="margin: 0; padding: 12px; background: var(--bg-main); color: var(--tree-text); overflow-x: auto; white-space: pre-wrap;">
                        {old_text}
                    </pre>
                </div>
                <div>
                    <div style="padding: 4px 8px; background: var(--color-bg-success); color: var(--color-success); font-weight: bold; border-bottom: 2px solid var(--color-success);">
                        "AFTER"
                    </div>
                    <pre style="margin: 0; padding: 12px; background: var(--bg-main); color: var(--tree-text); overflow-x: auto; white-space: pre-wrap;">
                        {new_text}
                    </pre>
                </div>
            </div>
        </div>
    }
}
