//! Search Panel Component
//! Project-wide search functionality

use leptos::prelude::*;

// Re-export search types from tauri_bindings_search
pub use crate::tauri_bindings_search::{SearchOptions, SearchResult};

#[component]
pub fn SearchPanel(
    is_open: RwSignal<bool>,
    root_path: String,
    on_result_click: impl Fn(String, usize) + 'static + Clone + Send + Sync,
) -> impl IntoView {
    let on_result_click = StoredValue::new(on_result_click);
    let search_query = RwSignal::new(String::new());
    let search_results = RwSignal::new(Vec::<SearchResult>::new());
    let is_searching = RwSignal::new(false);
    let case_sensitive = RwSignal::new(false);
    let use_regex = RwSignal::new(false);
    let error_message = RwSignal::new(None::<String>);

    // Perform search function - stored as a signal to allow multiple uses
    let perform_search = StoredValue::new(move || {
        let query = search_query.get();
        if query.is_empty() {
            search_results.set(vec![]);
            return;
        }

        is_searching.set(true);
        error_message.set(None);

        let root = root_path.clone();
        let options = SearchOptions {
            case_sensitive: case_sensitive.get(),
            use_regex: use_regex.get(),
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

        search_results.set(demo_results);
        is_searching.set(false);
    });

    view! {
        {move || {
            if is_open.get() {
                view! {
                    <div class="berry-search-panel">
                        <div class="berry-search-header">
                            <h3>"SEARCH"</h3>
                            <button
                                class="berry-search-close"
                                on:click=move |_| is_open.set(false)
                            >
                                "×"
                            </button>
                        </div>

                        <div class="berry-search-input-section">
                            <input
                                type="text"
                                class="berry-search-input"
                                placeholder="Search..."
                                prop:value=move || search_query.get()
                                on:input=move |ev| {
                                    search_query.set(event_target_value(&ev));
                                }
                                on:keydown=move |ev| {
                                    if ev.key() == "Enter" {
                                        perform_search.with_value(|f| f());
                                    }
                                }
                            />
                            <button
                                class="berry-search-button"
                                on:click=move |_| perform_search.with_value(|f| f())
                                disabled=move || is_searching.get()
                            >
                                {move || if is_searching.get() { "Searching..." } else { "Search" }}
                            </button>
                        </div>

                        <div class="berry-search-options">
                            <label>
                                <input
                                    type="checkbox"
                                    prop:checked=move || case_sensitive.get()
                                    on:change=move |ev| case_sensitive.set(event_target_checked(&ev))
                                />
                                " Match Case"
                            </label>
                            <label>
                                <input
                                    type="checkbox"
                                    prop:checked=move || use_regex.get()
                                    on:change=move |ev| use_regex.set(event_target_checked(&ev))
                                />
                                " Use Regex"
                            </label>
                        </div>

                        {move || {
                            if let Some(ref err) = error_message.get() {
                                view! {
                                    <div class="berry-search-error">{err.clone()}</div>
                                }.into_any()
                            } else {
                                view! { <></> }.into_any()
                            }
                        }}

                        <div class="berry-search-results">
                            {move || {
                                let results = search_results.get();
                                if results.is_empty() && !search_query.get().is_empty() {
                                    view! {
                                        <div class="berry-search-no-results">"No results found"</div>
                                    }.into_any()
                                } else {
                                    // Group results by file
                                    let mut grouped: std::collections::HashMap<String, Vec<SearchResult>> = std::collections::HashMap::new();
                                    for result in results {
                                        grouped.entry(result.path.clone()).or_insert_with(Vec::new).push(result);
                                    }

                                    view! {
                                        <div>
                                            {grouped.into_iter().map(|(path, results)| {
                                                let filename = path.split('/').last().unwrap_or(&path).to_string();
                                                let result_count = results.len();

                                                view! {
                                                    <div class="berry-search-file-group">
                                                        <div class="berry-search-file-header">
                                                            <i class="codicon codicon-file"></i>
                                                            " " {filename.clone()} " (" {result_count} " results)"
                                                        </div>
                                                        <div class="berry-search-file-results">
                                                            {results.into_iter().map(|result| {
                                                                let path_clone = result.path.clone();
                                                                let line_num = result.line_number;

                                                                view! {
                                                                    <div
                                                                        class="berry-search-result-item"
                                                                        on:click=move |_| on_result_click.with_value(|f| f(path_clone.clone(), line_num))
                                                                    >
                                                                        <span class="berry-search-result-line-num">
                                                                            {result.line_number}":"
                                                                        </span>
                                                                        <span class="berry-search-result-text">
                                                                            {result.line_text.clone()}
                                                                        </span>
                                                                    </div>
                                                                }
                                                            }).collect::<Vec<_>>()}
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
