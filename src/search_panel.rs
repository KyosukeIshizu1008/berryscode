//! Search Panel Component
//! Project-wide search functionality

use dioxus::prelude::*;
use std::collections::HashMap;

// Re-export search types from tauri_bindings_search
pub use crate::tauri_bindings_search::{SearchOptions, SearchResult};

/// Search Panel component props
#[derive(Props, Clone, PartialEq)]
pub struct SearchPanelProps {
    is_open: Signal<bool>,
    root_path: String,
    on_result_click: EventHandler<(String, usize)>,
}

#[component]
pub fn SearchPanel(props: SearchPanelProps) -> Element {
    let is_open = props.is_open;
    let root_path = props.root_path;
    let on_result_click = props.on_result_click;

    let mut search_query = use_signal(|| String::new());
    let mut search_results = use_signal(|| Vec::<SearchResult>::new());
    let mut is_searching = use_signal(|| false);
    let mut case_sensitive = use_signal(|| false);
    let mut use_regex = use_signal(|| false);
    let mut error_message = use_signal(|| None::<String>);

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
            case_sensitive: *case_sensitive.read(),
            use_regex: *use_regex.read(),
            ..Default::default()
        };

        // In a real implementation, this would call the Tauri search command
        // For now, we just log it

        // Simulated search results for demonstration
        let demo_results = vec![
            SearchResult {
                path: format!("{}/src/main.rs", root),
                line_number: 10,
                column: 5,
                line_text: "fn main() {".to_string(),
                match_start: 3,
                match_end: 7,
            },
            SearchResult {
                path: format!("{}/src/lib.rs", root),
                line_number: 25,
                column: 12,
                line_text: "    // Main module".to_string(),
                match_start: 7,
                match_end: 11,
            },
        ];

        *search_results.write() = demo_results;
        *is_searching.write() = false;
    };

    rsx! {
        {
            if *is_open.read() {
                rsx! {
                    div { class: "berry-search-panel",
                        div { class: "berry-search-header",
                            h3 { "SEARCH" }
                            button {
                                class: "berry-search-close",
                                onclick: move |_| *is_open.write() = false,
                                "×"
                            }
                        }

                        div { class: "berry-search-input-section",
                            input {
                                r#type: "text",
                                class: "berry-search-input",
                                placeholder: "Search...",
                                value: "{search_query.read()}",
                                oninput: move |ev| *search_query.write() = ev.value(),
                                onkeydown: move |ev: Event<KeyboardData>| {
                                    if ev.key() == Key::Enter {
                                        perform_search();
                                    }
                                },
                            }
                            button {
                                class: "berry-search-button",
                                onclick: move |_| perform_search(),
                                disabled: *is_searching.read(),
                                { if *is_searching.read() { "Searching..." } else { "Search" } }
                            }
                        }

                        div { class: "berry-search-options",
                            label {
                                input {
                                    r#type: "checkbox",
                                    checked: *case_sensitive.read(),
                                    onchange: move |ev| *case_sensitive.write() = ev.checked(),
                                }
                                " Match Case"
                            }
                            label {
                                input {
                                    r#type: "checkbox",
                                    checked: *use_regex.read(),
                                    onchange: move |ev| *use_regex.write() = ev.checked(),
                                }
                                " Use Regex"
                            }
                        }

                        {
                            if let Some(ref err) = *error_message.read() {
                                rsx! {
                                    div { class: "berry-search-error", "{err}" }
                                }
                            } else {
                                rsx! {}
                            }
                        }

                        div { class: "berry-search-results",
                            {
                                let results = search_results.read().clone();
                                let query = search_query.read().clone();

                                if results.is_empty() && !query.is_empty() {
                                    rsx! {
                                        div { class: "berry-search-no-results", "No results found" }
                                    }
                                } else {
                                    // Group results by file
                                    let mut grouped: HashMap<String, Vec<SearchResult>> = HashMap::new();
                                    for result in results {
                                        grouped.entry(result.path.clone()).or_insert_with(Vec::new).push(result);
                                    }

                                    rsx! {
                                        for (path , file_results) in grouped {
                                            {
                                                let filename = path.split('/').last().unwrap_or(&path).to_string();
                                                let result_count = file_results.len();

                                                rsx! {
                                                    div { class: "berry-search-file-group",
                                                        div { class: "berry-search-file-header",
                                                            i { class: "codicon codicon-file" }
                                                            " {filename} ({result_count} results)"
                                                        }
                                                        div { class: "berry-search-file-results",
                                                            for result in file_results {
                                                                {
                                                                    let path_clone = result.path.clone();
                                                                    let line_num = result.line_number;

                                                                    rsx! {
                                                                        div {
                                                                            class: "berry-search-result-item",
                                                                            onclick: move |_| on_result_click.call((path_clone.clone(), line_num)),

                                                                            span {
                                                                                class: "berry-search-result-line-num",
                                                                                "{result.line_number}:"
                                                                            }
                                                                            span {
                                                                                class: "berry-search-result-text",
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
