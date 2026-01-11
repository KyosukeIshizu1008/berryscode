//! Command Palette (Search Everywhere)
//!
//! IntelliJ-style Shift+Shift / VS Code Cmd+P equivalent
//!
//! ## Architecture
//! This module uses a plugin-based SearchProvider architecture for extensibility.
//! See `search_provider.rs` for provider implementations.

use leptos::prelude::*;
use leptos::ev::{KeyboardEvent, MouseEvent};
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use crate::search_provider::*;
use crate::focus_stack::{FocusStack, FocusLayer};

/// Action type for command palette
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ActionType {
    File,
    GitAction,
    EditorAction,
    Settings,
    Symbol,
}

impl ActionType {
    /// Get the default priority for this action type
    /// Lower values = higher priority (shown first in results)
    pub fn priority(&self) -> u32 {
        match self {
            ActionType::File => 10,          // Files first
            ActionType::GitAction => 100,    // Git actions second
            ActionType::EditorAction => 110, // Editor actions third
            ActionType::Symbol => 200,       // Symbols fourth (can be overridden by provider)
            ActionType::Settings => 300,     // Settings last
        }
    }
}

/// Command palette item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaletteItem {
    pub id: String,
    pub label: String,
    pub description: Option<String>,
    pub action_type: ActionType,
    pub icon: String,
    pub action: String,
}

/// Execute search across all enabled providers
///
/// Non-debounced providers execute immediately.
/// Debounced providers execute after their debounce delay.
async fn search_with_providers(
    providers: &[SearchProvider],
    query: &str,
    results_signal: RwSignal<Vec<PaletteItem>>,
    search_id_signal: RwSignal<u32>,
) {
    let query = query.to_string();

    // Separate providers by debounce requirement
    let mut immediate_providers = Vec::new();
    let mut debounced_providers = Vec::new();

    for provider in providers {
        if !provider.is_enabled() {
            continue;
        }

        if query.len() < provider.min_query_length() {
            continue;
        }

        if provider.debounce_ms() > 0 {
            debounced_providers.push(provider);
        } else {
            immediate_providers.push(provider);
        }
    }

    // Execute immediate providers (files, git, editor, settings)
    let mut all_results = Vec::new();

    for provider in immediate_providers {
        #[cfg(debug_assertions)]
        leptos::logging::log!("🔍 Searching provider: {}", provider.provider_name());

        let items = provider.search(&query).await;

        #[cfg(debug_assertions)]
        leptos::logging::log!(
            "✅ Provider {} returned {} items",
            provider.provider_name(),
            items.len()
        );

        for item in items {
            all_results.push((item, provider.priority()));
        }
    }

    // Sort by priority (lower = higher priority), then by label
    all_results.sort_by(|a, b| {
        a.1.cmp(&b.1)
            .then_with(|| a.0.label.cmp(&b.0.label))
    });

    // Extract items (drop priorities)
    let immediate_items: Vec<PaletteItem> = all_results
        .into_iter()
        .map(|(item, _)| item)
        .collect();

    // Update results with immediate items
    results_signal.set(immediate_items);

    // Execute debounced providers (symbols)
    for provider in debounced_providers {
        let query_for_search = query.clone();
        let debounce_ms = provider.debounce_ms();
        let provider_name = provider.provider_name().to_string();
        let priority = provider.priority();
        let provider_clone = provider.clone();  // Clone the provider for async closure

        // 🚀 RACE CONDITION PROTECTION:
        // Capture current search_id before scheduling debounced search
        // If user types again, search_id will increment and this search will be discarded
        let current_search_id = search_id_signal.get_untracked();

        #[cfg(debug_assertions)]
        leptos::logging::log!(
            "⏱️  Scheduling debounced search for provider: {} ({}ms)",
            provider_name,
            debounce_ms
        );

        leptos::prelude::set_timeout(
            move || {
                // Only execute if this is still the latest search
                if search_id_signal.get_untracked() != current_search_id {
                    #[cfg(debug_assertions)]
                    leptos::logging::log!("🚫 Search cancelled (query changed): {}", provider_name);
                    return;
                }

                spawn_local(async move {
                    // Check again if search is still relevant
                    if search_id_signal.get_untracked() != current_search_id {
                        #[cfg(debug_assertions)]
                        leptos::logging::log!("🚫 Results discarded (query changed)");
                        return;
                    }

                    #[cfg(debug_assertions)]
                    leptos::logging::log!("🔍 Executing debounced search: {}", provider_name);

                    let new_items = provider_clone.search(&query_for_search).await;

                    #[cfg(debug_assertions)]
                    leptos::logging::log!(
                        "✅ Provider {} returned {} items",
                        provider_name,
                        new_items.len()
                    );

                    // Merge with existing results
                    let mut combined = results_signal.get_untracked();

                    for item in new_items {
                        combined.push(item);
                    }

                    // Re-sort by priority using ActionType::priority()
                    let mut scored: Vec<(PaletteItem, u32)> = combined
                        .into_iter()
                        .map(|item| {
                            // Use ActionType's priority method for consistency
                            // For Symbol types, use provider-specific priority if available
                            let item_priority = if item.action_type == ActionType::Symbol {
                                priority // Provider-specific priority for symbols
                            } else {
                                item.action_type.priority()
                            };
                            (item, item_priority)
                        })
                        .collect();

                    scored.sort_by(|a, b| {
                        a.1.cmp(&b.1)
                            .then_with(|| a.0.label.cmp(&b.0.label))
                    });

                    let sorted_items: Vec<PaletteItem> = scored
                        .into_iter()
                        .map(|(item, _)| item)
                        .take(100) // Limit total results
                        .collect();

                    results_signal.set(sorted_items);
                });
            },
            std::time::Duration::from_millis(debounce_ms),
        );
    }
}

/// Command Palette Component
#[component]
pub fn CommandPalette(
    show: RwSignal<bool>,
    on_select: impl Fn(PaletteItem) + 'static + Clone + Send,
    #[prop(optional)] focus_stack: Option<FocusStack>,
) -> impl IntoView {
    // Use provided focus_stack or create a local one for backwards compatibility
    let focus_stack = focus_stack.unwrap_or_else(FocusStack::new);
    let query = RwSignal::new(String::new());
    let filtered_items = RwSignal::new(Vec::<PaletteItem>::new());
    let selected_index = RwSignal::new(0usize);
    let search_id = RwSignal::new(0u32);

    // Initialize providers (stored as signal for reactivity)
    let providers = StoredValue::new(vec![
        SearchProvider::FileSearch(FileSearchProvider::new()),
        SearchProvider::GitActions,
        SearchProvider::EditorActions,
        SearchProvider::Settings,
        SearchProvider::SymbolSearch,
    ]);

    // Load items when palette opens
    Effect::new(move || {
        if show.get() {
            // 🎯 FOCUS MANAGEMENT: Take focus when palette opens
            // ✅ FIX: Use untrack to prevent reactive graph explosion
            // This ensures focus_stack operations don't trigger cascading effects
            untrack(|| focus_stack.push(FocusLayer::CommandPalette));

            query.set(String::new());
            selected_index.set(0);
            search_id.set(0);

            // Initial load with empty query
            let provs = providers.get_value();
            spawn_local(async move {
                search_with_providers(&provs, "", filtered_items, search_id).await;
            });
        } else {
            // 🎯 FOCUS MANAGEMENT: Return focus to editor when palette closes
            // ✅ FIX: Use untrack to prevent reactive graph explosion
            untrack(|| focus_stack.pop());
        }
    });

    // Search when query changes
    // 🚀 RACE CONDITION MITIGATION:
    // - Each query change spawns a new async task
    // - Old tasks cannot be directly cancelled in Leptos/WASM
    // - search_id mechanism discards stale results (checked in search_with_providers)
    // - This prevents old results from overwriting newer ones
    Effect::new(move || {
        let q = query.get();
        let provs = providers.get_value();

        // Increment search_id to invalidate any in-flight searches
        search_id.update(|id| *id += 1);

        spawn_local(async move {
            search_with_providers(&provs, &q, filtered_items, search_id).await;
        });

        selected_index.set(0); // Reset selection on query change
    });

    // Clone on_select before the view to avoid FnOnce issues
    let on_select_for_view = on_select.clone();

    view! {
        {move || {
            if show.get() {
                // Clone on_select inside the reactive closure
                let on_select_for_keydown = on_select_for_view.clone();
                let on_select_for_items = on_select_for_view.clone();

                let handle_keydown = move |event: KeyboardEvent| {
                    let key = event.key();
                    let items_count = filtered_items.get_untracked().len();

                    match key.as_str() {
                        "ArrowDown" => {
                            event.prevent_default();
                            selected_index.update(|idx| {
                                *idx = (*idx + 1).min(items_count.saturating_sub(1));
                            });
                        }
                        "ArrowUp" => {
                            event.prevent_default();
                            selected_index.update(|idx| {
                                *idx = idx.saturating_sub(1);
                            });
                        }
                        "Enter" => {
                            event.prevent_default();
                            let idx = selected_index.get_untracked();
                            if let Some(item) = filtered_items.get_untracked().get(idx) {
                                on_select_for_keydown(item.clone());
                                show.set(false);
                            }
                        }
                        "Escape" => {
                            event.prevent_default();
                            show.set(false);
                        }
                        _ => {}
                    }
                };

                view! {
                    <div class="berry-command-palette-backdrop" on:click=move |_| show.set(false)>
                        <div class="berry-command-palette" on:click=move |e: MouseEvent| e.stop_propagation()>
                            <input
                                type="text"
                                class="berry-command-palette-input"
                                placeholder="Type a command or search..."
                                prop:value=move || query.get()
                                on:input=move |ev| {
                                    query.set(event_target_value(&ev));
                                }
                                on:keydown=handle_keydown
                                autofocus
                            />

                            <div class="berry-command-palette-results">
                                {move || {
                                    let current_items = filtered_items.get();
                                    let selected = selected_index.get();

                                    if current_items.is_empty() {
                                        view! {
                                            <div class="berry-palette-empty">
                                                "No results found"
                                            </div>
                                        }.into_any()
                                    } else {
                                        current_items
                                            .into_iter()
                                            .enumerate()
                                            .map(|(idx, item)| {
                                                let is_selected = idx == selected;
                                                let item_clone = item.clone();
                                                let on_select_clone = on_select_for_items.clone();

                                                view! {
                                                    <div
                                                        class:berry-palette-item=true
                                                        class:berry-palette-item-selected=is_selected
                                                        on:click=move |_| {
                                                            on_select_clone(item_clone.clone());
                                                            show.set(false);
                                                        }
                                                    >
                                                        <i class=format!("codicon codicon-{}", item.icon)></i>
                                                        <div class="berry-palette-item-content">
                                                            <div class="berry-palette-item-label">{item.label.clone()}</div>
                                                            {item.description.clone().map(|desc| {
                                                                view! {
                                                                    <div class="berry-palette-item-description">{desc}</div>
                                                                }
                                                            })}
                                                        </div>
                                                    </div>
                                                }
                                            })
                                            .collect_view()
                                            .into_any()
                                    }
                                }}
                            </div>
                        </div>
                    </div>
                }.into_any()
            } else {
                view! {}.into_any()
            }
        }}
    }
}
