//! IntelliJ-style Search Dialog
//!
//! Provides an overlay search dialog with tabs for different search types.
//! This appears when clicking the search icon and overlays the current view.

use dioxus::prelude::*;
use crate::tauri_bindings_search::{SearchOptions, SearchResult};

#[derive(Clone, Copy, PartialEq, Eq)]
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

/// Search Dialog component props
#[derive(Props, Clone, PartialEq)]
pub struct SearchDialogProps {
    is_open: Signal<bool>,
    root_path: String,
    on_result_click: EventHandler<(String, usize)>,
}

#[component]
pub fn SearchDialog(props: SearchDialogProps) -> Element {
    let is_open = props.is_open;
    let root_path = props.root_path;
    let on_result_click = props.on_result_click;

    let mut search_query = use_signal(|| String::new());
    let mut search_results = use_signal(|| Vec::<SearchResult>::new());
    let mut is_searching = use_signal(|| false);
    let mut active_tab = use_signal(|| SearchTab::All);
    let mut error_message = use_signal(|| None::<String>);

    // Close on Escape key
    let on_keydown = move |ev: Event<KeyboardData>| {
        if ev.key() == Key::Escape {
            *is_open.write() = false;
        }
    };

    // Perform search function
    let perform_search = move || {
        let query = search_query.read().clone();
        if query.is_empty() {
            *search_results.write() = vec![];
            return;
        }

        *is_searching.write() = true;
        *error_message.write() = None;

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

        *search_results.write() = demo_results;
        *is_searching.write() = false;
    };

    rsx! {
        {
            let dialog_open = *is_open.read();
            #[cfg(debug_assertions)]
            tracing::debug!("🔍 SearchDialog render: is_open = {}", dialog_open);

            if dialog_open {
                rsx! {
                    div {
                        class: "berry-search-dialog",
                        onkeydown: on_keydown,
                        style: "
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
                        ",

                        // Header with tabs
                        div {
                            class: "berry-search-dialog-header",
                            style: "
                                display: flex;
                                align-items: center;
                                background: var(--bg-sidebar);
                                border-bottom: 1px solid var(--border-color);
                                padding: 8px 12px;
                            ",

                            // Tabs
                            div {
                                class: "berry-search-tabs",
                                style: "
                                    display: flex;
                                    gap: 4px;
                                    flex: 1;
                                ",

                                for tab in [SearchTab::All, SearchTab::Types, SearchTab::Files,
                                           SearchTab::Symbols, SearchTab::Actions, SearchTab::Text] {
                                    {
                                        let tab_name = tab.as_str();
                                        let data_attr = tab.data_attr();
                                        let is_active = *active_tab.read() == tab;
                                        let bg_color = if is_active { "var(--bg-tab-hover)" } else { "transparent" };
                                        let text_color = if is_active { "var(--tree-text-active)" } else { "var(--tree-text)" };
                                        let style = format!(
                                            "padding: 6px 12px; \
                                            cursor: pointer; \
                                            font-size: 12px; \
                                            border-radius: 4px; \
                                            background: {}; \
                                            color: {}; \
                                            transition: all 0.2s;",
                                            bg_color, text_color
                                        );

                                        rsx! {
                                            div {
                                                class: "berry-search-tab",
                                                "data-tab": data_attr,
                                                onclick: move |_| *active_tab.write() = tab,
                                                style: "{style}",
                                                "{tab_name}"
                                            }
                                        }
                                    }
                                }
                            }

                            // Close button
                            button {
                                class: "berry-search-close",
                                onclick: move |_| *is_open.write() = false,
                                style: "
                                    background: transparent;
                                    border: none;
                                    color: var(--tree-text);
                                    font-size: 20px;
                                    cursor: pointer;
                                    padding: 0 4px;
                                    line-height: 1;
                                ",
                                "×"
                            }
                        }

                        // Search input
                        div {
                            class: "berry-search-input-section",
                            style: "
                                padding: 12px;
                                background: var(--bg-sidebar);
                                border-bottom: 1px solid var(--border-color);
                            ",

                            input {
                                r#type: "text",
                                class: "berry-search-input",
                                placeholder: "Type / to see commands",
                                value: "{search_query.read()}",
                                oninput: move |ev| *search_query.write() = ev.value(),
                                onkeydown: move |ev: Event<KeyboardData>| {
                                    if ev.key() == Key::Enter {
                                        perform_search();
                                    }
                                },
                                style: "
                                    width: 100%;
                                    background: var(--bg-tab-hover);
                                    border: 1px solid #555;
                                    color: var(--tree-text);
                                    padding: 8px 12px;
                                    border-radius: 4px;
                                    font-size: 13px;
                                    outline: none;
                                ",
                            }
                        }

                        // Options bar (like IntelliJ)
                        div {
                            class: "berry-search-options",
                            style: "
                                padding: 8px 12px;
                                background: var(--bg-sidebar);
                                border-bottom: 1px solid var(--border-color);
                                display: flex;
                                align-items: center;
                                gap: 12px;
                                font-size: 11px;
                                color: var(--tree-text);
                            ",

                            label {
                                style: "display: flex; align-items: center; gap: 4px; cursor: pointer;",
                                input { r#type: "checkbox", class: "cursor-pointer" }
                                "Include non-project items"
                            }
                            div { class: "flex-1" }
                            div {
                                style: "color: var(--icon-muted);",
                                {
                                    let count = search_results.read().len();
                                    if count > 0 {
                                        format!("{} results", count)
                                    } else {
                                        String::new()
                                    }
                                }
                            }
                        }

                        // Error message
                        {
                            if let Some(ref err) = *error_message.read() {
                                rsx! {
                                    div {
                                        class: "berry-search-error",
                                        style: "
                                            padding: 8px 12px;
                                            background: var(--color-bg-error);
                                            color: var(--color-error);
                                            font-size: 12px;
                                            border-bottom: 1px solid var(--border-color);
                                        ",
                                        "{err}"
                                    }
                                }
                            } else {
                                rsx! {}
                            }
                        }

                        // Results list
                        div {
                            class: "berry-search-results",
                            style: "
                                flex: 1;
                                overflow-y: auto;
                                background: var(--bg-sidebar);
                            ",

                            {
                                let results = search_results.read().clone();
                                let query = search_query.read().clone();

                                if results.is_empty() && !query.is_empty() {
                                    rsx! {
                                        div {
                                            class: "berry-search-no-results",
                                            style: "
                                                padding: 24px;
                                                text-align: center;
                                                color: var(--icon-muted);
                                                font-size: 12px;
                                            ",
                                            "No results found"
                                        }
                                    }
                                } else {
                                    rsx! {
                                        for result in results {
                                            {
                                                let path_clone = result.path.clone();
                                                let line_num = result.line_number;
                                                let filename = result.path.split('/').last().unwrap_or(&result.path).to_string();
                                                let directory = result.path.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("").to_string();

                                                rsx! {
                                                    div {
                                                        class: "berry-search-result-item",
                                                        onclick: move |_| {
                                                            on_result_click.call((path_clone.clone(), line_num));
                                                            *is_open.write() = false;
                                                        },
                                                        style: "
                                                            padding: 8px 12px;
                                                            cursor: pointer;
                                                            border-bottom: 1px solid var(--bg-sidebar);
                                                            transition: background 0.2s;
                                                        ",

                                                        div {
                                                            style: "
                                                                display: flex;
                                                                align-items: center;
                                                                gap: 8px;
                                                                margin-bottom: 4px;
                                                            ",
                                                            i {
                                                                class: "codicon codicon-file",
                                                                style: "color: var(--tree-text); font-size: 14px;"
                                                            }
                                                            span {
                                                                style: "color: var(--tree-text-active); font-size: 12px; font-weight: 500;",
                                                                "{filename}"
                                                            }
                                                            span {
                                                                class: "text-muted text-sm",
                                                                "{directory}"
                                                            }
                                                        }
                                                        div {
                                                            style: "
                                                                padding-left: 22px;
                                                                color: var(--tree-text);
                                                                font-size: 11px;
                                                                font-family: 'Monaco', 'Courier New', monospace;
                                                            ",
                                                            span {
                                                                style: "color: var(--icon-muted); margin-right: 8px;",
                                                                "{result.line_number}:"
                                                            }
                                                            "{result.line_text}"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
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
