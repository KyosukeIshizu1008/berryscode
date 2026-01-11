//! IntelliJ-style Search Dialog
//!
//! Provides an overlay search dialog with tabs for different search types.
//! This appears when clicking the search icon and overlays the current view.

use leptos::prelude::*;
use crate::tauri_bindings_search::{SearchOptions, SearchResult};

#[derive(Clone, Copy, PartialEq)]
enum SearchTab {
    All,
    Types,
    Files,
    Symbols,
    Actions,
    Text,
}

impl SearchTab {
    fn as_str(&self) -> &'static str {
        match self {
            SearchTab::All => "All",
            SearchTab::Types => "Types",
            SearchTab::Files => "Files",
            SearchTab::Symbols => "Symbols",
            SearchTab::Actions => "Actions",
            SearchTab::Text => "Text",
        }
    }

    fn data_attr(&self) -> &'static str {
        match self {
            SearchTab::All => "all",
            SearchTab::Types => "types",
            SearchTab::Files => "files",
            SearchTab::Symbols => "symbols",
            SearchTab::Actions => "actions",
            SearchTab::Text => "text",
        }
    }
}

#[component]
pub fn SearchDialog(
    is_open: RwSignal<bool>,
    root_path: String,
    on_result_click: impl Fn(String, usize) + 'static + Clone + Send + Sync,
) -> impl IntoView {
    let on_result_click = StoredValue::new(on_result_click);
    let search_query = RwSignal::new(String::new());
    let search_results = RwSignal::new(Vec::<SearchResult>::new());
    let is_searching = RwSignal::new(false);
    let active_tab = RwSignal::new(SearchTab::All);
    let error_message = RwSignal::new(None::<String>);

    // Close on Escape key
    let on_keydown = move |ev: leptos::ev::KeyboardEvent| {
        if ev.key() == "Escape" {
            is_open.set(false);
        }
    };

    // Perform search function
    let perform_search = StoredValue::new(move || {
        let query = search_query.get();
        if query.is_empty() {
            search_results.set(vec![]);
            return;
        }

        is_searching.set(true);
        error_message.set(None);

        let root = root_path.clone();
        let _options = SearchOptions {
            case_sensitive: false,
            use_regex: false,
            ..Default::default()
        };

        // Simulated search results for demonstration
        // In real implementation, this would call Tauri search command
        let demo_results = vec![
            SearchResult {
                path: format!("{}/TESTING.md", root),
                line_number: 1,
                column: 0,
                line_text: "# Testing Documentation".to_string(),
                match_start: 0,
                match_end: query.len(),
            },
            SearchResult {
                path: format!("{}/CLAUDE.md", root),
                line_number: 5,
                column: 2,
                line_text: "  Testing canvas rendering".to_string(),
                match_start: 2,
                match_end: 2 + query.len(),
            },
        ];

        search_results.set(demo_results);
        is_searching.set(false);
    });

    view! {
        {move || {
            let dialog_open = is_open.get();
            leptos::logging::log!("🔍 SearchDialog render: is_open = {}", dialog_open);
            if dialog_open {
                view! {
                    <div
                        class="berry-search-dialog"
                        on:keydown=on_keydown
                        style="
                            position: fixed;
                            top: 50px;
                            left: 50%;
                            transform: translateX(-50%);
                            width: 600px;
                            max-height: 600px;
                            background: var(--bg-sidebar);
                            border: 1px solid var(--border-color);
                            border-radius: 8px;
                            box-shadow: 0 4px 16px rgba(0, 0, 0, 0.5);
                            z-index: 1000;
                            display: flex;
                            flex-direction: column;
                            overflow: hidden;
                        "
                    >
                        // Header with tabs
                        <div class="berry-search-dialog-header" style="
                            display: flex;
                            align-items: center;
                            background: var(--bg-sidebar);
                            border-bottom: 1px solid var(--border-color);
                            padding: 8px 12px;
                        ">
                            // Tabs
                            <div class="berry-search-tabs" style="
                                display: flex;
                                gap: 4px;
                                flex: 1;
                            ">
                                {[SearchTab::All, SearchTab::Types, SearchTab::Files,
                                  SearchTab::Symbols, SearchTab::Actions, SearchTab::Text]
                                    .iter()
                                    .map(|&tab| {
                                        let tab_name = tab.as_str();
                                        let data_attr = tab.data_attr();
                                        view! {
                                            <div
                                                class="berry-search-tab"
                                                data-tab=data_attr
                                                on:click=move |_| active_tab.set(tab)
                                                style=move || {
                                                    let is_active = active_tab.get() == tab;
                                                    format!(
                                                        "padding: 6px 12px; \
                                                        cursor: pointer; \
                                                        font-size: 12px; \
                                                        border-radius: 4px; \
                                                        background: {}; \
                                                        color: {}; \
                                                        transition: all 0.2s;",
                                                        if is_active { "var(--bg-tab-hover)" } else { "transparent" },
                                                        if is_active { "var(--tree-text-active)" } else { "var(--tree-text)" }
                                                    )
                                                }
                                            >
                                                {tab_name}
                                            </div>
                                        }
                                    })
                                    .collect::<Vec<_>>()
                                }
                            </div>

                            // Close button
                            <button
                                class="berry-search-close"
                                on:click=move |_| is_open.set(false)
                                style="
                                    background: transparent;
                                    border: none;
                                    color: var(--tree-text);
                                    font-size: 20px;
                                    cursor: pointer;
                                    padding: 0 4px;
                                    line-height: 1;
                                "
                            >
                                "×"
                            </button>
                        </div>

                        // Search input
                        <div class="berry-search-input-section" style="
                            padding: 12px;
                            background: var(--bg-sidebar);
                            border-bottom: 1px solid var(--border-color);
                        ">
                            <input
                                type="text"
                                class="berry-search-input"
                                placeholder="Type / to see commands"
                                prop:value=move || search_query.get()
                                on:input=move |ev| {
                                    search_query.set(event_target_value(&ev));
                                }
                                on:keydown=move |ev| {
                                    if ev.key() == "Enter" {
                                        perform_search.with_value(|f| f());
                                    }
                                }
                                style="
                                    width: 100%;
                                    background: var(--bg-tab-hover);
                                    border: 1px solid #555;
                                    color: var(--tree-text);
                                    padding: 8px 12px;
                                    border-radius: 4px;
                                    font-size: 13px;
                                    outline: none;
                                "
                            />
                        </div>

                        // Options bar (like IntelliJ)
                        <div class="berry-search-options" style="
                            padding: 8px 12px;
                            background: var(--bg-sidebar);
                            border-bottom: 1px solid var(--border-color);
                            display: flex;
                            align-items: center;
                            gap: 12px;
                            font-size: 11px;
                            color: var(--tree-text);
                        ">
                            <label style="display: flex; align-items: center; gap: 4px; cursor: pointer;">
                                <input type="checkbox" class="cursor-pointer" />
                                "Include non-project items"
                            </label>
                            <div class="flex-1"></div>
                            <div style="color: var(--icon-muted);">
                                {move || {
                                    let count = search_results.get().len();
                                    if count > 0 {
                                        format!("{} results", count)
                                    } else {
                                        String::new()
                                    }
                                }}
                            </div>
                        </div>

                        // Error message
                        {move || {
                            if let Some(ref err) = error_message.get() {
                                view! {
                                    <div class="berry-search-error" style="
                                        padding: 8px 12px;
                                        background: var(--color-bg-error);
                                        color: var(--color-error);
                                        font-size: 12px;
                                        border-bottom: 1px solid var(--border-color);
                                    ">
                                        {err.clone()}
                                    </div>
                                }.into_any()
                            } else {
                                view! { <></> }.into_any()
                            }
                        }}

                        // Results list
                        <div class="berry-search-results" style="
                            flex: 1;
                            overflow-y: auto;
                            background: var(--bg-sidebar);
                        ">
                            {move || {
                                let results = search_results.get();
                                if results.is_empty() && !search_query.get().is_empty() {
                                    view! {
                                        <div class="berry-search-no-results" style="
                                            padding: 24px;
                                            text-align: center;
                                            color: var(--icon-muted);
                                            font-size: 12px;
                                        ">
                                            "No results found"
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {
                                        <div>
                                            {results.into_iter().map(|result| {
                                                let path_clone = result.path.clone();
                                                let line_num = result.line_number;
                                                let filename = result.path.split('/').last().unwrap_or(&result.path).to_string();
                                                let directory = result.path.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("").to_string();

                                                view! {
                                                    <div
                                                        class="berry-search-result-item"
                                                        on:click=move |_| {
                                                            on_result_click.with_value(|f| f(path_clone.clone(), line_num));
                                                            is_open.set(false);
                                                        }
                                                        style="
                                                            padding: 8px 12px;
                                                            cursor: pointer;
                                                            border-bottom: 1px solid var(--bg-sidebar);
                                                            transition: background 0.2s;
                                                            &:hover { background: var(--bg-sidebar); }
                                                        "
                                                    >
                                                        <div style="
                                                            display: flex;
                                                            align-items: center;
                                                            gap: 8px;
                                                            margin-bottom: 4px;
                                                        ">
                                                            <i class="codicon codicon-file" style="color: var(--tree-text); font-size: 14px;"></i>
                                                            <span style="color: var(--tree-text-active); font-size: 12px; font-weight: 500;">
                                                                {filename}
                                                            </span>
                                                            <span class="text-muted text-sm">
                                                                {directory}
                                                            </span>
                                                        </div>
                                                        <div style="
                                                            padding-left: 22px;
                                                            color: var(--tree-text);
                                                            font-size: 11px;
                                                            font-family: 'Monaco', 'Courier New', monospace;
                                                        ">
                                                            <span style="color: var(--icon-muted); margin-right: 8px;">
                                                                {result.line_number}":"
                                                            </span>
                                                            {result.line_text.clone()}
                                                        </div>
                                                    </div>
                                                }
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    }.into_any()
                                }
                            }}
                        </div>
                    </div>
                }.into_any()
            } else {
                view! { <></> }.into_any()
            }
        }}
    }
}
