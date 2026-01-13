use dioxus::prelude::*;

pub use crate::tauri_bindings_workflow::{WorkflowPreset, WorkflowStatus, StartWorkflowRequest};

/// Workflow Panel props
#[derive(Props, Clone, PartialEq)]
pub struct WorkflowPanelProps {
    is_active: Signal<bool>,
}

#[component]
pub fn WorkflowPanel(props: WorkflowPanelProps) -> Element {
    let is_active = props.is_active;

    let mut presets = use_signal(|| Vec::<WorkflowPreset>::new());
    let mut selected_preset = use_signal(|| None::<String>);
    let mut show_start_dialog = use_signal(|| false);
    let mut initial_prompt = use_signal(|| String::new());
    let mut current_execution_id = use_signal(|| None::<String>);

    // Load presets on mount
    use_effect(move || {
        if *is_active.read() {
            spawn(async move {
                match crate::tauri_bindings_workflow::workflow_list_presets().await {
                    Ok(p) => *presets.write() = p,
                    Err(e) => {
                        #[cfg(debug_assertions)]
                        tracing::error!("Failed to load workflow presets: {}", e);
                    }
                }
            });
        }
    });

    rsx! {
        div { class: "berry-editor-sidebar",
            div {
                class: "berry-editor-sidebar-header",
                style: "
                    display: flex;
                    justify-content: space-between;
                    align-items: center;
                    padding: 8px 12px;
                    font-size: 12px;
                    font-weight: 600;
                    color: var(--tree-text);
                ",
                span { "WORKFLOW AUTOMATION" }
            }

            div {
                class: "workflow-presets-list",
                style: "
                    flex: 1;
                    overflow-y: auto;
                    padding: 8px;
                ",

                {
                    let p = presets.read().clone();
                    if p.is_empty() {
                        rsx! {
                            div {
                                style: "padding: 20px; text-align: center; color: var(--icon-muted);",
                                "Loading workflows..."
                            }
                        }
                    } else {
                        rsx! {
                            for preset in p {
                                {
                                    let preset_id = preset.id.clone();
                                    let preset_id_for_select = preset.id.clone();
                                    let preset_name = preset.name.clone();
                                    let preset_desc = preset.description.clone();
                                    let preset_icon = preset.icon.clone();
                                    let nodes_count = preset.nodes_count;

                                    let is_selected = selected_preset.read().as_ref() == Some(&preset_id_for_select);
                                    let bg = if is_selected { "var(--bg-sidebar)" } else { "var(--bg-main)" };
                                    let border = if is_selected { "var(--color-accent-secondary)" } else { "var(--border-color)" };
                                    let style = format!(
                                        "padding: 12px; margin-bottom: 8px; border-radius: 6px; cursor: pointer; \
                                         background: {}; border: 1px solid {}; transition: all 0.2s;",
                                        bg, border
                                    );

                                    rsx! {
                                        div {
                                            class: "workflow-preset-item",
                                            style: "{style}",
                                            onclick: move |_| {
                                                *selected_preset.write() = Some(preset_id.clone());
                                                *show_start_dialog.write() = true;
                                            },

                                            div {
                                                style: "display: flex; align-items: center; gap: 12px; margin-bottom: 8px;",
                                                i {
                                                    class: "codicon {preset_icon}",
                                                    style: "font-size: 24px; color: var(--color-accent-secondary);"
                                                }
                                                div {
                                                    style: "flex: 1;",
                                                    div {
                                                        style: "font-size: 13px; font-weight: 600; color: var(--tree-text); margin-bottom: 4px;",
                                                        "{preset_name}"
                                                    }
                                                    div {
                                                        style: "font-size: 11px; color: var(--icon-muted);",
                                                        "{nodes_count} nodes"
                                                    }
                                                }
                                            }
                                            div {
                                                style: "font-size: 11px; color: #999999; line-height: 1.5;",
                                                "{preset_desc}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Start Workflow Dialog
            {
                if *show_start_dialog.read() {
                    rsx! {
                        div { class: "modal-overlay",
                            div { class: "modal-dialog",
                                div { class: "modal-header",
                                    "Start Workflow"
                                }

                                div { class: "modal-body",
                                    div {
                                        label {
                                            class: "mb-4",
                                            style: "display: block; font-size: 12px;",
                                            "Initial Prompt / Requirements"
                                        }
                                        textarea {
                                            class: "textarea-field",
                                            value: "{initial_prompt.read()}",
                                            oninput: move |ev| *initial_prompt.write() = ev.value(),
                                            placeholder: "Describe what you want to build or fix...",
                                            rows: "6",
                                            style: "background: var(--bg-main);",
                                        }
                                    }

                                    div { class: "panel-section",
                                        i {
                                            class: "codicon codicon-info",
                                            style: "margin-right: 6px;"
                                        }
                                        "The workflow will execute automatically based on your requirements."
                                    }
                                }

                                div { class: "modal-footer",
                                    button {
                                        class: "btn btn-secondary",
                                        onclick: move |_| {
                                            *show_start_dialog.write() = false;
                                            *initial_prompt.write() = String::new();
                                        },
                                        "Cancel"
                                    }
                                    button {
                                        class: "btn btn-primary",
                                        onclick: move |_| {
                                            if let Some(preset_id) = selected_preset.read().clone() {
                                                let prompt = initial_prompt.read().clone();
                                                if !prompt.is_empty() {
                                                    spawn(async move {
                                                        let request = StartWorkflowRequest {
                                                            preset_id,
                                                            initial_prompt: prompt,
                                                        };
                                                        match crate::tauri_bindings_workflow::workflow_start(request).await {
                                                            Ok(execution_id) => {
                                                                #[cfg(debug_assertions)]
                                                                tracing::debug!("✅ Workflow started: {}", execution_id);
                                                                *current_execution_id.write() = Some(execution_id);
                                                                *show_start_dialog.write() = false;
                                                                *initial_prompt.write() = String::new();
                                                            }
                                                            Err(e) => {
                                                                #[cfg(debug_assertions)]
                                                                tracing::error!("Failed to start workflow: {}", e);
                                                            }
                                                        }
                                                    });
                                                }
                                            }
                                        },
                                        disabled: initial_prompt.read().is_empty(),
                                        "Start Workflow"
                                    }
                                }
                            }
                        }
                    }
                } else {
                    rsx! {}
                }
            }
        }
    }
}
