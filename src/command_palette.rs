//! Command Palette (Search Everywhere)
//!
//! IntelliJ-style Shift+Shift / VS Code Cmd+P equivalent
//!
//! ## Architecture
//! This module uses a plugin-based SearchProvider architecture for extensibility.
//! See `search_provider.rs` for provider implementations.

use dioxus::prelude::*;
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
    results_signal: Signal<Vec<PaletteItem>>,
    search_id_signal: Signal<u32>,
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
        tracing::debug!("🔍 Searching provider: {}", provider.provider_name());

        let items = provider.search(&query).await;

        #[cfg(debug_assertions)]
        tracing::debug!(
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
    *results_signal.write() = immediate_items;

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
        let current_search_id = *search_id_signal.read();

        #[cfg(debug_assertions)]
        tracing::debug!(
            "⏱️  Scheduling debounced search for provider: {} ({}ms)",
            provider_name,
            debounce_ms
        );

        // Spawn async task with delay
        spawn(async move {
            // Wait for debounce period
            gloo_timers::future::sleep(std::time::Duration::from_millis(debounce_ms)).await;

            // Only execute if this is still the latest search
            if *search_id_signal.read() != current_search_id {
                #[cfg(debug_assertions)]
                tracing::debug!("🚫 Search cancelled (query changed): {}", provider_name);
                return;
            }

            #[cfg(debug_assertions)]
            tracing::debug!("🔍 Executing debounced search: {}", provider_name);

            let new_items = provider_clone.search(&query_for_search).await;

            #[cfg(debug_assertions)]
            tracing::debug!(
                "✅ Provider {} returned {} items",
                provider_name,
                new_items.len()
            );

            // Check again if search is still relevant
            if *search_id_signal.read() != current_search_id {
                #[cfg(debug_assertions)]
                tracing::debug!("🚫 Results discarded (query changed)");
                return;
            }

            // Merge with existing results
            let mut combined = results_signal.read().clone();

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

            *results_signal.write() = sorted_items;
        });
    }
}

/// Command Palette Component props
#[derive(Props, Clone, PartialEq)]
pub struct CommandPaletteProps {
    show: Signal<bool>,
    on_select: EventHandler<PaletteItem>,
    #[props(optional)] focus_stack: Option<FocusStack>,
}

/// Command Palette Component
#[component]
pub fn CommandPalette(props: CommandPaletteProps) -> Element {
    let show = props.show;
    let on_select = props.on_select;

    // Use provided focus_stack or create a local one for backwards compatibility
    let focus_stack = props.focus_stack.unwrap_or_else(FocusStack::new);

    let mut query = use_signal(|| String::new());
    let mut filtered_items = use_signal(|| Vec::<PaletteItem>::new());
    let mut selected_index = use_signal(|| 0usize);
    let mut search_id = use_signal(|| 0u32);

    // Initialize providers (stored as constant for this component)
    let providers = vec![
        SearchProvider::FileSearch(FileSearchProvider::new()),
        SearchProvider::GitActions,
        SearchProvider::EditorActions,
        SearchProvider::Settings,
        SearchProvider::SymbolSearch,
    ];

    // Load items when palette opens
    use_effect(move || {
        let is_shown = *show.read();

        if is_shown {
            // 🎯 FOCUS MANAGEMENT: Take focus when palette opens
            focus_stack.push(FocusLayer::CommandPalette);

            *query.write() = String::new();
            *selected_index.write() = 0;
            *search_id.write() = 0;

            // Initial load with empty query
            let provs = providers.clone();
            spawn(async move {
                search_with_providers(&provs, "", filtered_items, search_id).await;
            });
        } else {
            // 🎯 FOCUS MANAGEMENT: Return focus to editor when palette closes
            focus_stack.pop();
        }
    });

    // Search when query changes
    // 🚀 RACE CONDITION MITIGATION:
    // - Each query change spawns a new async task
    // - Old tasks cannot be directly cancelled in Dioxus/WASM
    // - search_id mechanism discards stale results (checked in search_with_providers)
    // - This prevents old results from overwriting newer ones
    use_effect(move || {
        let q = query.read().clone();
        let provs = providers.clone();

        // Increment search_id to invalidate any in-flight searches
        search_id.write().update(|id| *id += 1);

        spawn(async move {
            search_with_providers(&provs, &q, filtered_items, search_id).await;
        });

        *selected_index.write() = 0; // Reset selection on query change
    });

    rsx! {
        {
            let is_shown = *show.read();

            if is_shown {
                let handle_keydown = move |event: Event<KeyboardData>| {
                    let items_count = filtered_items.read().len();

                    match event.key() {
                        Key::ArrowDown => {
                            event.prevent_default();
                            selected_index.write().update(|idx| {
                                *idx = (*idx + 1).min(items_count.saturating_sub(1));
                            });
                        }
                        Key::ArrowUp => {
                            event.prevent_default();
                            selected_index.write().update(|idx| {
                                *idx = idx.saturating_sub(1);
                            });
                        }
                        Key::Enter => {
                            event.prevent_default();
                            let idx = *selected_index.read();
                            if let Some(item) = filtered_items.read().get(idx) {
                                on_select.call(item.clone());
                                *show.write() = false;
                            }
                        }
                        Key::Escape => {
                            event.prevent_default();
                            *show.write() = false;
                        }
                        _ => {}
                    }
                };

                rsx! {
                    div { class: "berry-command-palette-backdrop",
                        onclick: move |_| *show.write() = false,

                        div { class: "berry-command-palette",
                            onclick: move |e: Event<MouseData>| e.stop_propagation(),

                            input {
                                r#type: "text",
                                class: "berry-command-palette-input",
                                placeholder: "Type a command or search...",
                                value: "{query.read()}",
                                oninput: move |evt| *query.write() = evt.value(),
                                onkeydown: handle_keydown,
                                autofocus: true,
                            }

                            div { class: "berry-command-palette-results",
                                {
                                    let current_items = filtered_items.read().clone();
                                    let selected = *selected_index.read();

                                    if current_items.is_empty() {
                                        rsx! {
                                            div { class: "berry-palette-empty",
                                                "No results found"
                                            }
                                        }
                                    } else {
                                        rsx! {
                                            for (idx , item) in current_items.iter().enumerate() {
                                                {
                                                    let is_selected = idx == selected;
                                                    let item_clone = item.clone();
                                                    let class_name = if is_selected {
                                                        "berry-palette-item berry-palette-item-selected"
                                                    } else {
                                                        "berry-palette-item"
                                                    };

                                                    rsx! {
                                                        div {
                                                            class: "{class_name}",
                                                            onclick: move |_| {
                                                                on_select.call(item_clone.clone());
                                                                *show.write() = false;
                                                            },

                                                            i { class: "codicon codicon-{item.icon}" }
                                                            div { class: "berry-palette-item-content",
                                                                div { class: "berry-palette-item-label", "{item.label}" }
                                                                if let Some(desc) = &item.description {
                                                                    div { class: "berry-palette-item-description", "{desc}" }
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
