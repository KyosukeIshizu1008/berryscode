//! Settings Window - Leptos Component
//!
//! Demonstrates Leptos reactive state management with type-safe form inputs.

use leptos::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EditorSettings {
    pub theme: String,
    pub font_size: u32,
    pub tab_size: u32,
    pub word_wrap: bool,
    pub minimap_enabled: bool,
    pub auto_save: bool,
    pub format_on_save: bool,
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            theme: "vs-dark".to_string(),
            font_size: 14,
            tab_size: 4,
            word_wrap: true,
            minimap_enabled: true,
            auto_save: true,
            format_on_save: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AISettings {
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub enable_inline_completions: bool,
}

impl Default for AISettings {
    fn default() -> Self {
        Self {
            model: "gpt-4".to_string(),
            temperature: 0.7,
            max_tokens: 2000,
            enable_inline_completions: true,
        }
    }
}

#[component]
pub fn SettingsApp() -> impl IntoView {
    // Reactive state with Leptos signals
    let (editor_settings, set_editor_settings) = create_signal(EditorSettings::default());
    let (ai_settings, set_ai_settings) = create_signal(AISettings::default());
    let (active_tab, set_active_tab) = create_signal("editor".to_string());
    let (save_message, set_save_message) = create_signal(None::<String>);

    // Load settings from backend on mount
    create_effect(move |_| {
        // Settings are loaded from default values
        // In Phase 2, this will load from Tauri config file
        logging::log!("Settings initialized with defaults");
    });

    // Save settings handler
    let save_settings = move |_| {
        logging::log!("Saving settings...");

        // In Phase 2, this will save to Tauri config file via IPC
        // For now, settings persist in component state during session
        let editor = editor_settings.get();
        let ai = ai_settings.get();

        logging::log!(&format!("Editor settings: theme={}, font_size={}", editor.theme, editor.font_size));
        logging::log!(&format!("AI settings: model={}, temperature={}", ai.model, ai.temperature));

        set_save_message(Some("Settings saved successfully".to_string()));

        // Clear message after 3 seconds
        set_timeout(
            move || set_save_message(None),
            std::time::Duration::from_secs(3),
        );
    };

    view! {
        <div class="settings-window">
            <header class="settings-header">
                <h1>"BerryCode Settings"</h1>
                <button on:click=save_settings class="save-btn">
                    "Save"
                </button>
            </header>

            {move || save_message.get().map(|msg| view! {
                <div class="save-notification">{msg}</div>
            })}

            <div class="settings-layout">
                // Tab navigation
                <nav class="settings-tabs">
                    <button
                        class=move || if active_tab.get() == "editor" { "tab-btn active" } else { "tab-btn" }
                        on:click=move |_| set_active_tab("editor".to_string())
                    >
                        "Editor"
                    </button>
                    <button
                        class=move || if active_tab.get() == "ai" { "tab-btn active" } else { "tab-btn" }
                        on:click=move |_| set_active_tab("ai".to_string())
                    >
                        "AI"
                    </button>
                    <button
                        class=move || if active_tab.get() == "keybindings" { "tab-btn active" } else { "tab-btn" }
                        on:click=move |_| set_active_tab("keybindings".to_string())
                    >
                        "Keybindings"
                    </button>
                </nav>

                // Tab content
                <div class="settings-content">
                    {move || match active_tab.get().as_str() {
                        "editor" => view! { <EditorSettingsPanel settings=editor_settings set_settings=set_editor_settings /> }.into_view(),
                        "ai" => view! { <AISettingsPanel settings=ai_settings set_settings=set_ai_settings /> }.into_view(),
                        "keybindings" => view! { <div>"Keybindings settings (Not implemented yet)"</div> }.into_view(),
                        _ => view! { <div>"Unknown tab"</div> }.into_view(),
                    }}
                </div>
            </div>

            <footer class="settings-footer">
                <p>"BerryCode v0.86.2 - Powered by Leptos + Tauri v2"</p>
            </footer>
        </div>
    }
}

#[component]
fn EditorSettingsPanel(
    settings: ReadSignal<EditorSettings>,
    set_settings: WriteSignal<EditorSettings>,
) -> impl IntoView {
    view! {
        <div class="settings-panel">
            <h2>"Editor Settings"</h2>

            <div class="setting-group">
                <label for="theme">"Theme"</label>
                <select
                    id="theme"
                    on:change=move |ev| {
                        let value = event_target_value(&ev);
                        set_settings.update(|s| s.theme = value);
                    }
                    prop:value=move || settings.get().theme
                >
                    <option value="vs-dark">"Dark (VS Code style)"</option>
                    <option value="vs-light">"Light"</option>
                    <option value="hc-black">"High Contrast"</option>
                </select>
            </div>

            <div class="setting-group">
                <label for="font-size">"Font Size: " {move || settings.get().font_size} "px"</label>
                <input
                    type="range"
                    id="font-size"
                    min="10"
                    max="30"
                    on:input=move |ev| {
                        let value = event_target_value(&ev).parse().unwrap_or(14);
                        set_settings.update(|s| s.font_size = value);
                    }
                    prop:value=move || settings.get().font_size
                />
            </div>

            <div class="setting-group">
                <label for="tab-size">"Tab Size"</label>
                <input
                    type="number"
                    id="tab-size"
                    min="2"
                    max="8"
                    on:input=move |ev| {
                        let value = event_target_value(&ev).parse().unwrap_or(4);
                        set_settings.update(|s| s.tab_size = value);
                    }
                    prop:value=move || settings.get().tab_size
                />
            </div>

            <div class="setting-group checkbox-group">
                <label>
                    <input
                        type="checkbox"
                        on:change=move |ev| {
                            let checked = event_target_checked(&ev);
                            set_settings.update(|s| s.word_wrap = checked);
                        }
                        prop:checked=move || settings.get().word_wrap
                    />
                    "Word Wrap"
                </label>
            </div>

            <div class="setting-group checkbox-group">
                <label>
                    <input
                        type="checkbox"
                        on:change=move |ev| {
                            let checked = event_target_checked(&ev);
                            set_settings.update(|s| s.minimap_enabled = checked);
                        }
                        prop:checked=move || settings.get().minimap_enabled
                    />
                    "Show Minimap"
                </label>
            </div>

            <div class="setting-group checkbox-group">
                <label>
                    <input
                        type="checkbox"
                        on:change=move |ev| {
                            let checked = event_target_checked(&ev);
                            set_settings.update(|s| s.auto_save = checked);
                        }
                        prop:checked=move || settings.get().auto_save
                    />
                    "Auto Save"
                </label>
            </div>

            <div class="setting-group checkbox-group">
                <label>
                    <input
                        type="checkbox"
                        on:change=move |ev| {
                            let checked = event_target_checked(&ev);
                            set_settings.update(|s| s.format_on_save = checked);
                        }
                        prop:checked=move || settings.get().format_on_save
                    />
                    "Format on Save"
                </label>
            </div>
        </div>
    }
}

#[component]
fn AISettingsPanel(
    settings: ReadSignal<AISettings>,
    set_settings: WriteSignal<AISettings>,
) -> impl IntoView {
    view! {
        <div class="settings-panel">
            <h2>"AI Settings"</h2>

            <div class="setting-group">
                <label for="model">"Model"</label>
                <select
                    id="model"
                    on:change=move |ev| {
                        let value = event_target_value(&ev);
                        set_settings.update(|s| s.model = value);
                    }
                    prop:value=move || settings.get().model
                >
                    <option value="gpt-4">"GPT-4"</option>
                    <option value="gpt-4-turbo">"GPT-4 Turbo"</option>
                    <option value="gpt-3.5-turbo">"GPT-3.5 Turbo"</option>
                    <option value="claude-3-opus">"Claude 3 Opus"</option>
                    <option value="claude-3-sonnet">"Claude 3 Sonnet"</option>
                </select>
            </div>

            <div class="setting-group">
                <label for="temperature">"Temperature: " {move || format!("{:.1}", settings.get().temperature)}</label>
                <input
                    type="range"
                    id="temperature"
                    min="0"
                    max="2"
                    step="0.1"
                    on:input=move |ev| {
                        let value = event_target_value(&ev).parse().unwrap_or(0.7);
                        set_settings.update(|s| s.temperature = value);
                    }
                    prop:value=move || settings.get().temperature
                />
            </div>

            <div class="setting-group">
                <label for="max-tokens">"Max Tokens"</label>
                <input
                    type="number"
                    id="max-tokens"
                    min="100"
                    max="8000"
                    step="100"
                    on:input=move |ev| {
                        let value = event_target_value(&ev).parse().unwrap_or(2000);
                        set_settings.update(|s| s.max_tokens = value);
                    }
                    prop:value=move || settings.get().max_tokens
                />
            </div>

            <div class="setting-group checkbox-group">
                <label>
                    <input
                        type="checkbox"
                        on:change=move |ev| {
                            let checked = event_target_checked(&ev);
                            set_settings.update(|s| s.enable_inline_completions = checked);
                        }
                        prop:checked=move || settings.get().enable_inline_completions
                    />
                    "Enable inline completions (Copilot-style)"
                </label>
            </div>
        </div>
    }
}
