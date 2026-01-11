use leptos::prelude::*;

pub use crate::tauri_bindings_workflow::{WorkflowPreset, WorkflowStatus, StartWorkflowRequest};

#[component]
pub fn WorkflowPanel(is_active: Signal<bool>) -> impl IntoView {
    let presets = RwSignal::new(Vec::<WorkflowPreset>::new());
    let selected_preset = RwSignal::new(None::<String>);
    let show_start_dialog = RwSignal::new(false);
    let initial_prompt = RwSignal::new(String::new());
    let current_execution_id = RwSignal::new(None::<String>);

    // Load presets on mount
    Effect::new(move |_| {
        if is_active.get() {
            leptos::task::spawn_local(async move {
                match crate::tauri_bindings_workflow::workflow_list_presets().await {
                    Ok(p) => presets.set(p),
                    Err(e) => leptos::logging::error!("Failed to load workflow presets: {}", e),
                }
            });
        }
    });

    view! {
        <div class="berry-editor-sidebar">
            <div class="berry-editor-sidebar-header" style="
                display: flex;
                justify-content: space-between;
                align-items: center;
                padding: 8px 12px;
                font-size: 12px;
                font-weight: 600;
                color: var(--tree-text);
            ">
                <span>"WORKFLOW AUTOMATION"</span>
            </div>

            <div class="workflow-presets-list" style="
                flex: 1;
                overflow-y: auto;
                padding: 8px;
            ">
                {move || {
                    let p = presets.get();
                    if p.is_empty() {
                        view! {
                            <div style="padding: 20px; text-align: center; color: var(--icon-muted);">
                                "Loading workflows..."
                            </div>
                        }.into_any()
                    } else {
                        p.iter().map(|preset| {
                            let preset_id = preset.id.clone();
                            let preset_id_for_select = preset.id.clone();
                            let preset_name = preset.name.clone();
                            let preset_desc = preset.description.clone();
                            let preset_icon = preset.icon.clone();
                            let nodes_count = preset.nodes_count;

                            let is_selected = Signal::derive(move || {
                                selected_preset.get().as_ref() == Some(&preset_id_for_select)
                            });

                            view! {
                                <div
                                    class="workflow-preset-item"
                                    style=move || format!(
                                        "padding: 12px; margin-bottom: 8px; border-radius: 6px; cursor: pointer; \
                                         background: {}; border: 1px solid {}; transition: all 0.2s;",
                                        if is_selected.get() { "var(--bg-sidebar)" } else { "var(--bg-main)" },
                                        if is_selected.get() { "var(--color-accent-secondary)" } else { "var(--border-color)" }
                                    )
                                    on:click=move |_| {
                                        selected_preset.set(Some(preset_id.clone()));
                                        show_start_dialog.set(true);
                                    }
                                >
                                    <div style="display: flex; align-items: center; gap: 12px; margin-bottom: 8px;">
                                        <i class=format!("codicon {}", preset_icon) style="font-size: 24px; color: var(--color-accent-secondary);"></i>
                                        <div style="flex: 1;">
                                            <div style="font-size: 13px; font-weight: 600; color: var(--tree-text); margin-bottom: 4px;">
                                                {preset_name.clone()}
                                            </div>
                                            <div style="font-size: 11px; color: var(--icon-muted);">
                                                {format!("{} nodes", nodes_count)}
                                            </div>
                                        </div>
                                    </div>
                                    <div style="font-size: 11px; color: #999999; line-height: 1.5;">
                                        {preset_desc.clone()}
                                    </div>
                                </div>
                            }
                        }).collect_view().into_any()
                    }
                }}
            </div>

            // Start Workflow Dialog
            {move || {
                if show_start_dialog.get() {
                    view! {
                        <div class="modal-overlay">
                            <div class="modal-dialog">
                                <div class="modal-header">
                                    "Start Workflow"
                                </div>

                                <div class="modal-body">
                                    <div>
                                        <label class="mb-4" style="display: block; font-size: 12px;">
                                            "Initial Prompt / Requirements"
                                        </label>
                                        <textarea
                                            class="textarea-field"
                                            prop:value=move || initial_prompt.get()
                                            on:input=move |ev| initial_prompt.set(event_target_value(&ev))
                                            placeholder="Describe what you want to build or fix..."
                                            rows="6"
                                            style="background: var(--bg-main);"
                                        />
                                    </div>

                                    <div class="panel-section">
                                        <i class="codicon codicon-info" style="margin-right: 6px;"></i>
                                        "The workflow will execute automatically based on your requirements."
                                    </div>
                                </div>

                                <div class="modal-footer">
                                    <button
                                        class="btn btn-secondary"
                                        on:click=move |_| {
                                            show_start_dialog.set(false);
                                            initial_prompt.set(String::new());
                                        }
                                    >
                                        "Cancel"
                                    </button>
                                    <button
                                        class="btn btn-primary"
                                        on:click=move |_| {
                                            if let Some(preset_id) = selected_preset.get() {
                                                let prompt = initial_prompt.get();
                                                if !prompt.is_empty() {
                                                    leptos::task::spawn_local(async move {
                                                        let request = StartWorkflowRequest {
                                                            preset_id,
                                                            initial_prompt: prompt,
                                                        };
                                                        match crate::tauri_bindings_workflow::workflow_start(request).await {
                                                            Ok(execution_id) => {
                                                                leptos::logging::log!("✅ Workflow started: {}", execution_id);
                                                                current_execution_id.set(Some(execution_id));
                                                                show_start_dialog.set(false);
                                                                initial_prompt.set(String::new());
                                                            }
                                                            Err(e) => leptos::logging::error!("Failed to start workflow: {}", e),
                                                        }
                                                    });
                                                }
                                            }
                                        }
                                        disabled=move || initial_prompt.get().is_empty()
                                    >
                                        "Start Workflow"
                                    </button>
                                </div>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! { <></> }.into_any()
                }
            }}
        </div>
    }
}
