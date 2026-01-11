//! LSP-Enhanced Editor Component
//!
//! Editor with integrated LSP support for completions, diagnostics, and hover.

use leptos::prelude::*;
use leptos::ev::{KeyboardEvent, MouseEvent};
use leptos::task::spawn_local;
use crate::buffer::TextBuffer;
use crate::syntax::SyntaxHighlighter;
use crate::lsp_ui::{LspIntegration, CompletionItem, Diagnostic, HoverInfo};
use crate::completion_widget::CompletionWidget;
use crate::hover_tooltip::HoverTooltip;
use crate::types::Position;

#[derive(Clone)]
struct EditorTab {
    path: String,
    buffer: TextBuffer,
    highlighter: SyntaxHighlighter,
}

/// LSP-enhanced editor panel
#[component]
pub fn LspEditorPanel(
    selected_file: RwSignal<Option<(String, String)>>,
    /// Shared diagnostics signal for DiagnosticsPanel
    diagnostics: RwSignal<Vec<Diagnostic>>,
) -> impl IntoView {
    let tabs = RwSignal::new(Vec::<EditorTab>::new());
    let active_tab_index = RwSignal::new(0usize);

    // LSP Integration
    let lsp = LspIntegration::new();

    // Completion state
    let completion_items = RwSignal::new(Vec::<CompletionItem>::new());
    let completion_position = RwSignal::new(Position::new(0, 0));
    let show_completion = RwSignal::new(false);

    // Hover state
    let hover_info = RwSignal::new(Option::<HoverInfo>::None);
    let hover_position = RwSignal::new(Option::<(f64, f64)>::None);

    // Cursor position
    let cursor_position = RwSignal::new(Position::new(0, 0));


    // Effect: Watch for file selection changes
    Effect::new_isomorphic(move |_| {
        let file_data = selected_file.get();

        if let Some((path, content)) = file_data {

            // Update LSP file path
            lsp.set_file_path(path.clone());

            // Request diagnostics for the file
            let lsp_clone = lsp.clone();
            let diagnostics_clone = diagnostics.clone();
            spawn_local(async move {
                if let Ok(diags) = lsp_clone.request_diagnostics().await {
                    diagnostics_clone.set(diags);
                }
            });

            // Check if tab already exists
            let current_tabs = tabs.get_untracked();
            let existing_tab_index = current_tabs.iter().position(|tab| tab.path == path);

            if let Some(idx) = existing_tab_index {
                active_tab_index.set(idx);
            } else {
                // Create new tab
                let buffer = TextBuffer::from_str(&content);
                let highlighter = SyntaxHighlighter::new();
                let tab = EditorTab {
                    path: path.clone(),
                    buffer,
                    highlighter,
                };

                tabs.update(|t| t.push(tab));
                let new_index = tabs.get_untracked().len() - 1;
                active_tab_index.set(new_index);
            }
        }
    });

    // Handler: Request completions
    let request_completions = move |position: Position| {
        let lsp_clone = lsp.clone();
        spawn_local(async move {
            if let Ok(items) = lsp_clone.request_completions(position).await {
                if !items.is_empty() {
                    completion_items.set(items);
                    completion_position.set(position);
                    show_completion.set(true);
                }
            }
        });
    };

    // Handler: Request hover info
    let request_hover = move |position: Position, mouse_x: f64, mouse_y: f64| {
        let lsp_clone = lsp.clone();
        spawn_local(async move {
            if let Ok(Some(info)) = lsp_clone.request_hover(position).await {
                hover_info.set(Some(info));
                hover_position.set(Some((mouse_x, mouse_y)));
            }
        });
    };

    // Handler: Hide hover
    let hide_hover = move || {
        hover_info.set(None);
        hover_position.set(None);
    };

    // Handler: Completion selected
    let on_completion_select = move |item: CompletionItem| {

        // TODO: Insert completion into buffer
        let insert_text = item.insert_text.unwrap_or(item.label);

        // Hide completion widget
        show_completion.set(false);
        completion_items.set(Vec::new());
    };

    // Keyboard event handler
    let handle_keydown = move |event: KeyboardEvent| {
        let key = event.key();

        // Trigger completion on Ctrl+Space
        if key == " " && event.ctrl_key() {
            event.prevent_default();
            let pos = cursor_position.get_untracked();
            request_completions(pos);
            return;
        }

        // Auto-trigger completion on dot
        if key == "." {
            let pos = cursor_position.get_untracked();
            request_completions(pos);
        }
    };

    // Mouse hover handler
    let handle_mousemove = move |event: MouseEvent| {
        // Only show hover if not showing completion
        if !show_completion.get_untracked() {
            // Calculate position from mouse coordinates
            // This is simplified - in real implementation, you'd convert pixel coords to line/col
            let x = event.client_x() as f64;
            let y = event.client_y() as f64;

            // Estimate line/column from pixel position
            let estimated_line = ((y - 50.0) / 20.0).max(0.0) as usize;
            let estimated_col = ((x - 50.0) / 10.0).max(0.0) as usize;

            let position = Position::new(estimated_line, estimated_col);
            request_hover(position, x, y);
        }
    };

    // Get active tab
    let active_tab = move || {
        let idx = active_tab_index.get();
        let current_tabs = tabs.get();
        current_tabs.get(idx).cloned()
    };

    view! {
        <div class="berry-editor-main" on:keydown=handle_keydown>
            // Tab Bar
            <div class="berry-editor-tab-bar">
                {move || {
                    let current_tabs = tabs.get();
                    let current_index = active_tab_index.get();

                    current_tabs.iter().enumerate().map(|(idx, tab)| {
                        let is_active = idx == current_index;
                        let filename = tab.path.split('/').last().unwrap_or(&tab.path).to_string();
                        let tab_class = if is_active {
                            "berry-editor-tab active"
                        } else {
                            "berry-editor-tab"
                        };

                        view! {
                            <div class=tab_class on:click=move |_| {
                                active_tab_index.set(idx);
                            }>
                                <span>{filename}</span>
                            </div>
                        }
                    }).collect::<Vec<_>>()
                }}
            </div>

            // Editor Pane with LSP overlays
            <div class="berry-editor-pane" on:mousemove=handle_mousemove on:mouseleave=move |_| hide_hover()>
                {move || {
                    let tab_opt = active_tab();

                    if let Some(tab) = tab_opt {
                        let content = tab.buffer.to_string();
                        let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
                        let line_count = lines.len();

                        view! {
                            <div class="berry-editor-content">
                                <div class="berry-editor-line-numbers">
                                    {(1..=line_count).map(|line_num| {
                                        view! {
                                            <div class="berry-editor-line-number">{line_num}</div>
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>
                                <div class="berry-editor-lines">
                                    {lines.into_iter().map(|line| {
                                        view! {
                                            <div class="berry-editor-line">
                                                <span>{line}</span>
                                            </div>
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>

                                // LSP UI Overlays
                                {move || {
                                    if show_completion.get() {
                                        view! {
                                            <CompletionWidget
                                                items=completion_items
                                                position=completion_position.get_untracked()
                                                on_select=on_completion_select
                                            />
                                        }.into_any()
                                    } else {
                                        view! { <></> }.into_any()
                                    }
                                }}

                                <HoverTooltip
                                    hover_info=hover_info
                                    position=hover_position
                                />
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <div style="display:flex;align-items:center;justify-content:center;height:100%;color:var(--icon-muted);">
                                "Click a file to start editing..."
                            </div>
                        }.into_any()
                    }
                }}
            </div>

            // Status Bar with diagnostics summary
            <div class="berry-editor-status-bar">
                {move || {
                    let diags = diagnostics.get();
                    let errors = diags.iter().filter(|d| d.severity == 1).count();
                    let warnings = diags.iter().filter(|d| d.severity == 2).count();

                    if let Some(tab) = active_tab() {
                        let lang = tab.highlighter.get_language().unwrap_or("text");
                        if errors > 0 || warnings > 0 {
                            format!("{} | UTF-8 | {} errors, {} warnings", lang, errors, warnings)
                        } else {
                            format!("{} | UTF-8", lang)
                        }
                    } else {
                        "Ready".to_string()
                    }
                }}
            </div>
        </div>
    }
}
