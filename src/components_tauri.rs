//! UI Components for BerryEditor - Tauri Version
//! Uses native file system access

use crate::core::virtual_editor::VirtualEditorPanel;
use crate::file_tree_tauri::FileTreePanelTauri;
use crate::search_dialog::SearchDialog;
use crate::database_panel::DatabasePanel;
use crate::workflow_panel::WorkflowPanel;
use crate::terminal_panel::TerminalPanel;
use crate::berrycode_panel::BerryCodePanel;
use crate::settings::EditorSettings;
use crate::tauri_bindings;
use crate::common::icons::*;
use leptos::prelude::*;

/// Active panel in the sidebar
#[derive(Debug, Clone, Copy, PartialEq)]
enum ActivePanel {
    Explorer,
    Chat,
    Database,
    Workflow,
    Terminal,
    VirtualOffice,
    Settings,
}

/// Sidebar panel definition for data-driven ActivityBar
#[derive(Clone)]
struct SidebarPanel {
    id: ActivePanel,
    icon: &'static str,
    title: &'static str,
}

/// Main activity bar panels (top section)
/// ✅ Using type-safe icon constants from common::icons
const MAIN_PANELS: &[SidebarPanel] = &[
    SidebarPanel { id: ActivePanel::Explorer, icon: ICON_FILES, title: "Explorer" },
    SidebarPanel { id: ActivePanel::Chat, icon: ICON_COMMENT_DISCUSSION, title: "BerryCode AI" },
    SidebarPanel { id: ActivePanel::Database, icon: ICON_DATABASE, title: "Database Tools" },
    SidebarPanel { id: ActivePanel::Workflow, icon: ICON_REFERENCES, title: "Workflow Automation" },
    SidebarPanel { id: ActivePanel::Terminal, icon: ICON_TERMINAL, title: "Integrated Terminal" },
    SidebarPanel { id: ActivePanel::VirtualOffice, icon: ICON_HUBOT, title: "Virtual Office" },
];

/// Bottom activity bar panels
const BOTTOM_PANELS: &[SidebarPanel] = &[
    SidebarPanel { id: ActivePanel::Settings, icon: ICON_SETTINGS_GEAR, title: "Settings" },
];

/// Status Bar component with branding
#[component]
pub fn StatusBar() -> impl IntoView {
    view! {
        <div class="berry-editor-status-bar">
            <div class="berry-editor-status-left">
                <span class="title">"BerryEditor"</span>
                <span class="subtitle">"100% Rust"</span>
            </div>
            <div class="berry-editor-status-right">
                <span>"WASM"</span>
            </div>
        </div>
    }
}

#[component]
pub fn EditorAppTauri() -> impl IntoView {
    // File selection state (shared between FileTree and Editor)
    // ✅ FIX: Use provide_context to avoid ownership issues in Leptos 0.7
    let selected_file = RwSignal::new(Option::<(String, String)>::None); // (path, content)
    provide_context(selected_file);

    // Active panel state (Explorer or Search)
    let active_panel = RwSignal::new(ActivePanel::Explorer);

    // Search dialog state (overlay)
    let search_dialog_is_open = RwSignal::new(false);

    // Sidebar resize state
    let sidebar_width = RwSignal::new(300.0); // Default width in pixels
    let is_resizing = RwSignal::new(false);

    // ✅ Chat panel state (右側のチャットパネル)
    let chat_panel_visible = RwSignal::new(true); // 初期状態は表示
    let chat_panel_width = RwSignal::new(350.0);   // チャットパネルの幅（ファイルツリーと同じ）
    let is_resizing_chat = RwSignal::new(false);   // チャットパネルのリサイズ中フラグ

    // ✅ Focus stack for keyboard event management
    let focus_stack = RwSignal::new(crate::focus_stack::FocusStack::new());

    // Get current directory dynamically from Tauri
    // ✅ Start with empty path - will be populated by Effect
    // In test environment, get_current_dir() will return "." due to is_tauri_context() check
    let root_path = RwSignal::new(String::new());

    // Load current directory on mount
    Effect::new(move |_| {
        leptos::task::spawn_local(async move {
            match tauri_bindings::get_current_dir().await {
                Ok(path) => {
                    root_path.set(path);
                }
                Err(_e) => {
                    // Fallback to current working directory
                    leptos::logging::warn!("Failed to get current directory: {}", _e);
                    root_path.set(".".to_string());
                }
            }
        });
    });

    // Apply theme on mount
    Effect::new(move |_| {
        let settings = EditorSettings::load();
        settings.apply_theme();
        leptos::logging::log!("Applied theme: {}", settings.color_theme);
    });

    // Resize handlers
    let is_hovering_resize = RwSignal::new(false);

    let on_resize_mousedown = move |_ev: leptos::ev::MouseEvent| {
        is_resizing.set(true);
    };

    let on_mousemove = move |ev: leptos::ev::MouseEvent| {
        if is_resizing.get() {
            // Calculate new width (subtract activity bar width: 54px)
            let new_width = (ev.client_x() as f64 - 54.0)
                .max(200.0)  // 最小幅: 200px（より自由に調整可能）
                .min(1200.0); // 最大幅: 1200px（より自由に調整可能）
            sidebar_width.set(new_width);
        }

        // ✅ チャットパネルのリサイズ処理（より自由なサイズ調整）
        if is_resizing_chat.get() {
            // ウィンドウ幅から現在のマウス位置を引いて、右からの幅を計算
            if let Some(window) = web_sys::window() {
                let window_width = window.inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(1920.0);
                let new_width = (window_width - ev.client_x() as f64)
                    .max(250.0)   // 最小幅: 250px（コンパクトに）
                    .min(1600.0); // 最大幅: 1600px（ワイドに）
                chat_panel_width.set(new_width);
            }
        }
    };

    let on_mouseup = move |_ev: leptos::ev::MouseEvent| {
        is_resizing.set(false);
        is_resizing_chat.set(false); // ✅ チャットパネルのリサイズも解除
    };

    let on_resize_mouseenter = move |_ev: leptos::ev::MouseEvent| {
        is_hovering_resize.set(true);
    };

    let on_resize_mouseleave = move |_ev: leptos::ev::MouseEvent| {
        is_hovering_resize.set(false);
    };

    view! {
        <div
            class="berry-editor-container"
            on:mousemove=on_mousemove
            on:mouseup=on_mouseup
            style=move || {
                let base = "display: flex; flex-direction: column; height: 100vh; width: 100vw; overflow: hidden;";
                if is_resizing.get() {
                    format!("{} cursor: col-resize; user-select: none;", base)
                } else {
                    base.to_string()
                }
            }
        >
            <div class="berry-editor-main-area">
                // 🚀 REFACTORED: Data-driven Activity Bar (leftmost vertical icon bar)
                // Panels are now defined in MAIN_PANELS and BOTTOM_PANELS constants
                // This makes it easy to add/remove/reorder panels without touching the view macro
                <div class="activity-bar">
                    // Main panels (top section)
                    {MAIN_PANELS.iter().map(|panel| {
                        let id = panel.id;
                        view! {
                            <div
                                class="activity-icon"
                                class:active=move || active_panel.get() == id
                                on:click=move |ev| {
                                    ev.stop_propagation();
                                    active_panel.set(id);
                                }
                                title=panel.title
                            >
                                <i class=format!("codicon codicon-{}", panel.icon)></i>
                            </div>
                        }
                    }).collect_view()}

                    // Search icon (special case - opens dialog instead of switching panel)
                    <div
                        class="activity-icon"
                        class:active=search_dialog_is_open
                        on:click=move |ev| {
                            ev.stop_propagation();
                            leptos::logging::log!("🔍 Search icon clicked!");
                            search_dialog_is_open.set(true);
                            leptos::logging::log!("🔍 search_dialog_is_open set to: {}", search_dialog_is_open.get());
                        }
                        title="Search"
                    >
                        <i class="codicon codicon-search"></i>
                    </div>

                    // Spacer to push bottom panels to bottom
                    <div class="flex-1"></div>

                    // Bottom panels (settings, etc.)
                    {BOTTOM_PANELS.iter().map(|panel| {
                        let id = panel.id;
                        view! {
                            <div
                                class="activity-icon"
                                class:active=move || active_panel.get() == id
                                on:click=move |ev| {
                                    ev.stop_propagation();
                                    active_panel.set(id);
                                }
                                title=panel.title
                            >
                                <i class=format!("codicon codicon-{}", panel.icon)></i>
                            </div>
                        }
                    }).collect_view()}
                </div>

                // Sidebar container with resize capability
                <div
                    style=move || {
                        let w = sidebar_width.get();
                        leptos::logging::log!("🔍 Sidebar width: {}px", w);
                        format!("width: {}px; flex-shrink: 0; overflow: hidden; display: flex; flex-direction: column; align-self: stretch;", w)
                    }
                >
                    // Sidebar - switches between all panels
                    {move || {
                        let path = root_path.get();
                        let panel = active_panel.get();
                        leptos::logging::log!("🔍 Active panel: {:?}, root_path: {}", panel, path);
                        match panel {
                        ActivePanel::Explorer => {
                            if !path.is_empty() {
                                view! {
                                    // ✅ FIX: Don't pass on_file_select - it will use context
                                    <FileTreePanelTauri root_path=path.clone() />
                                }.into_any()
                            } else {
                                view! {
                                    <div class="berry-editor-sidebar">
                                        <div class="berry-sidebar-loading">
                                            "Loading..."
                                        </div>
                                    </div>
                                }.into_any()
                            }
                        },
                        ActivePanel::Chat => {
                            view! {
                                <BerryCodePanel
                                    project_path=Signal::derive(move || root_path.get())
                                    focus_stack=focus_stack
                                />
                            }.into_any()
                        },
                        ActivePanel::Database => {
                            view! {
                                <DatabasePanel is_active=Signal::derive(move || active_panel.get() == ActivePanel::Database) />
                            }.into_any()
                        },
                        ActivePanel::Workflow => {
                            view! {
                                <WorkflowPanel is_active=Signal::derive(move || active_panel.get() == ActivePanel::Workflow) />
                            }.into_any()
                        },
                        ActivePanel::Terminal => {
                            // Terminal is shown in main area, hide sidebar
                            view! {
                                <div class="berry-editor-sidebar hidden"></div>
                            }.into_any()
                        },
                        ActivePanel::VirtualOffice => {
                            view! {
                                <div class="berry-editor-sidebar">
                                    <div class="berry-sidebar-panel-header">
                                        "VIRTUAL OFFICE"
                                    </div>
                                    <div class="berry-sidebar-panel-content">
                                        <div class="berry-sidebar-panel-info">
                                            <i class="codicon codicon-info"></i>
                                            "Virtual office collaboration coming soon"
                                        </div>
                                        <div class="berry-sidebar-panel-features">
                                            "Features:"
                                            <ul>
                                                <li>"Team presence awareness"</li>
                                                <li>"Real-time collaboration"</li>
                                                <li>"Screen sharing"</li>
                                                <li>"Code review sessions"</li>
                                                <li>"Pair programming"</li>
                                            </ul>
                                        </div>
                                    </div>
                                </div>
                            }.into_any()
                        },
                        ActivePanel::Settings => {
                            let settings_store = StoredValue::new(EditorSettings::load());

                            let (font_size, set_font_size) = signal(settings_store.get_value().font_size);
                            let (line_height, set_line_height) = signal(settings_store.get_value().line_height);
                            let (tab_size, set_tab_size) = signal(settings_store.get_value().tab_size);
                            let (word_wrap, set_word_wrap) = signal(settings_store.get_value().word_wrap);
                            let (ai_enabled, set_ai_enabled) = signal(settings_store.get_value().ai_enabled);

                            let save_settings = move || {
                                let s = settings_store.get_value();
                                let _ = s.save();
                                leptos::logging::log!("Settings saved");
                            };

                            view! {
                                <div class="berry-settings-sidebar">
                                    <div class="berry-sidebar-panel-header">
                                        "SETTINGS"
                                    </div>
                                    <div class="berry-settings-content">
                                        // Editor Settings
                                        <div class="berry-settings-section">
                                            <div class="berry-settings-section-title">
                                                "Editor"
                                            </div>
                                            <div class="berry-settings-controls">
                                                // Font Size
                                                <div class="berry-settings-control-row">
                                                    <span class="berry-settings-control-label">"Font Size"</span>
                                                    <input
                                                        type="number"
                                                        min="8"
                                                        max="32"
                                                        prop:value=move || font_size.get()
                                                        on:input=move |ev| {
                                                            if let Ok(val) = event_target_value(&ev).parse() {
                                                                set_font_size.set(val);
                                                                settings_store.update_value(|s| s.font_size = val);
                                                                save_settings();
                                                            }
                                                        }
                                                        class="berry-settings-input"
                                                    />
                                                </div>

                                                // Font Family
                                                <div class="berry-settings-control-row">
                                                    <span class="berry-settings-control-label">"Font Family"</span>
                                                    <select
                                                        on:change=move |ev| {
                                                            let val = event_target_value(&ev);
                                                            settings_store.update_value(|s| s.font_family = val);
                                                            save_settings();
                                                        }
                                                        class="berry-settings-select"
                                                    >
                                                        {
                                                            let current = settings_store.get_value().font_family;
                                                            EditorSettings::available_fonts().into_iter().map(|font| {
                                                                let is_selected = font == current.as_str();
                                                                view! {
                                                                    <option value=font selected=is_selected>{font}</option>
                                                                }
                                                            }).collect_view()
                                                        }
                                                    </select>
                                                </div>

                                                // Line Height
                                                <div class="berry-settings-control-row">
                                                    <span class="berry-settings-control-label">"Line Height"</span>
                                                    <input
                                                        type="number"
                                                        min="14"
                                                        max="40"
                                                        prop:value=move || line_height.get()
                                                        on:input=move |ev| {
                                                            if let Ok(val) = event_target_value(&ev).parse() {
                                                                set_line_height.set(val);
                                                                settings_store.update_value(|s| s.line_height = val);
                                                                save_settings();
                                                            }
                                                        }
                                                        class="berry-settings-input"
                                                    />
                                                </div>

                                                // Tab Size
                                                <div class="berry-settings-control-row">
                                                    <span class="berry-settings-control-label">"Tab Size"</span>
                                                    <input
                                                        type="number"
                                                        min="2"
                                                        max="8"
                                                        prop:value=move || tab_size.get()
                                                        on:input=move |ev| {
                                                            if let Ok(val) = event_target_value(&ev).parse() {
                                                                set_tab_size.set(val);
                                                                settings_store.update_value(|s| s.tab_size = val);
                                                                save_settings();
                                                            }
                                                        }
                                                        class="berry-settings-input"
                                                    />
                                                </div>

                                                // Word Wrap
                                                <div class="berry-settings-control-row">
                                                    <span class="berry-settings-control-label">"Word Wrap"</span>
                                                    <input
                                                        type="checkbox"
                                                        prop:checked=move || word_wrap.get()
                                                        on:change=move |ev| {
                                                            let checked = event_target_checked(&ev);
                                                            set_word_wrap.set(checked);
                                                            settings_store.update_value(|s| s.word_wrap = checked);
                                                            save_settings();
                                                        }
                                                        class="berry-settings-checkbox"
                                                    />
                                                </div>
                                            </div>
                                        </div>

                                        // Theme Settings
                                        <div class="berry-settings-section">
                                            <div class="berry-settings-section-title">
                                                "Theme"
                                            </div>
                                            <div class="berry-settings-controls">
                                                // Color Theme
                                                <div class="berry-settings-control-row">
                                                    <span class="berry-settings-control-label">"Color Theme"</span>
                                                    <select
                                                        on:change=move |ev| {
                                                            let val = event_target_value(&ev);
                                                            settings_store.update_value(|s| {
                                                                s.color_theme = val;
                                                                s.apply_theme(); // Apply theme to DOM
                                                            });
                                                            save_settings();
                                                        }
                                                        class="berry-settings-select"
                                                    >
                                                        {
                                                            let current = settings_store.get_value().color_theme;
                                                            EditorSettings::available_themes().into_iter().map(|(theme_id, theme_name)| {
                                                                let is_selected = theme_id == current.as_str();
                                                                view! {
                                                                    <option value=theme_id selected=is_selected>{theme_name}</option>
                                                                }
                                                            }).collect_view()
                                                        }
                                                    </select>
                                                </div>
                                            </div>
                                        </div>

                                        // BerryCode AI Settings
                                        <div class="berry-settings-section">
                                            <div class="berry-settings-section-title">
                                                "BerryCode AI"
                                            </div>
                                            <div class="berry-settings-controls">
                                                // Model
                                                <div class="berry-settings-control-row">
                                                    <span class="berry-settings-control-label">"Model"</span>
                                                    <select
                                                        on:change=move |ev| {
                                                            let val = event_target_value(&ev);
                                                            settings_store.update_value(|s| s.ai_model = val);
                                                            save_settings();
                                                        }
                                                        class="berry-settings-select"
                                                    >
                                                        {
                                                            let current = settings_store.get_value().ai_model;
                                                            EditorSettings::available_models().into_iter().map(|model| {
                                                                let is_selected = model == current.as_str();
                                                                view! {
                                                                    <option value=model selected=is_selected>{model}</option>
                                                                }
                                                            }).collect_view()
                                                        }
                                                    </select>
                                                </div>

                                                // Mode
                                                <div class="berry-settings-control-row">
                                                    <span class="berry-settings-control-label">"Mode"</span>
                                                    <select
                                                        on:change=move |ev| {
                                                            let val = event_target_value(&ev);
                                                            settings_store.update_value(|s| s.ai_mode = val);
                                                            save_settings();
                                                        }
                                                        class="berry-settings-select"
                                                    >
                                                        {
                                                            let current = settings_store.get_value().ai_mode;
                                                            EditorSettings::available_modes().into_iter().map(|mode| {
                                                                let is_selected = mode == current.as_str();
                                                                view! {
                                                                    <option value=mode selected=is_selected>{mode}</option>
                                                                }
                                                            }).collect_view()
                                                        }
                                                    </select>
                                                </div>

                                                // AI Enabled
                                                <div class="berry-settings-control-row">
                                                    <span class="berry-settings-control-label">"Enable AI"</span>
                                                    <input
                                                        type="checkbox"
                                                        prop:checked=move || ai_enabled.get()
                                                        on:change=move |ev| {
                                                            let checked = event_target_checked(&ev);
                                                            set_ai_enabled.set(checked);
                                                            settings_store.update_value(|s| s.ai_enabled = checked);
                                                            save_settings();
                                                        }
                                                        class="berry-settings-checkbox"
                                                    />
                                                </div>
                                            </div>
                                        </div>

                                        // About
                                        <div class="berry-settings-section">
                                            <div class="berry-settings-section-title">
                                                "About"
                                            </div>
                                            <div class="berry-settings-about-controls">
                                                <div class="berry-settings-about-version">
                                                    "BerryEditor v0.1.0"
                                                </div>
                                                <div class="berry-settings-about-desc">
                                                    "100% Rust Code Editor"
                                                </div>
                                                <div class="berry-settings-about-desc">
                                                    "Built with Leptos + Tauri + WASM"
                                                </div>
                                            </div>
                                        </div>
                                    </div>
                                </div>
                            }.into_any()
                        }
                    }
                    }}
                </div>

                // Resize handle (IntelliJ/VS Code style)
                <div
                    on:mousedown=on_resize_mousedown
                    on:mouseenter=on_resize_mouseenter
                    on:mouseleave=on_resize_mouseleave
                    style=move || format!("
                        width: 5px;
                        cursor: col-resize;
                        background: {};
                        user-select: none;
                        flex-shrink: 0;
                        transition: background 0.15s ease;
                        position: relative;
                        z-index: 10;
                    ",
                        if is_resizing.get() {
                            "#007ACC"
                        } else if is_hovering_resize.get() {
                            "#4C4C4C"
                        } else {
                            "#1E1E1E"
                        }
                    )
                ></div>

                // ✅ Main Editor Area: Canvas（左） | Chat Panel（右、リサイズ可能）
                <div class="berry-main-editor-area" style="display: flex; flex: 1; min-width: 0; overflow: hidden; position: relative;">
                    // Canvas Editor (左側、可変幅)
                    <div style="flex: 1; min-width: 0; display: flex; flex-direction: column;">
                        {move || {
                            let path = root_path.get();
                            if active_panel.get() == ActivePanel::Terminal && !path.is_empty() {
                                view! {
                                    <div class="berry-terminal-container">
                                        <TerminalPanel project_path=Signal::derive(move || root_path.get()) />
                                    </div>
                                }.into_any()
                            } else {
                                view! {
                                    // ✅ FIX: Don't pass selected_file - it will use context
                                    <VirtualEditorPanel
                                        is_active=Signal::derive(move || active_panel.get() != ActivePanel::Terminal)
                                        focus_stack=focus_stack
                                    />
                                }.into_any()
                            }
                        }}
                    </div>
                </div>

                // ✅ 右側のアクティビティバー（チャットパネルのトグル）
                <div class="activity-bar border-l border-berry-border">
                    <div
                        class="activity-icon"
                        class:active=move || chat_panel_visible.get()
                        on:click=move |_| {
                            chat_panel_visible.update(|v| *v = !*v);
                            leptos::logging::log!("💬 Chat panel toggled: {}", chat_panel_visible.get());
                        }
                        title="BerryCode AI Chat"
                    >
                        <i class="codicon codicon-comment-discussion"></i>
                    </div>
                </div>

                // ✅ 右側チャットパネル（ファイルツリーと同じスタイル）
                {move || {
                    if chat_panel_visible.get() {
                        let path = root_path.get();
                        view! {
                            // Resize handle
                            <div
                                class="w-1 bg-berry-border cursor-col-resize flex-shrink-0 hover:bg-berry-border-accent transition-colors"
                                on:mousedown=move |_ev: leptos::ev::MouseEvent| {
                                    is_resizing_chat.set(true);
                                }
                            ></div>

                            // Chat panel
                            <div
                                class="flex-shrink-0 flex flex-col border-l border-berry-border h-full overflow-hidden"
                                style=move || format!("width: {}px;", chat_panel_width.get())
                            >
                                <BerryCodePanel
                                    project_path=Signal::derive(move || path.clone())
                                    focus_stack=focus_stack
                                />
                            </div>
                        }.into_any()
                    } else {
                        view! { <></> }.into_any()
                    }
                }}

                // Search Dialog Overlay (IntelliJ-style)
                {move || {
                    let path = root_path.get();
                    if !path.is_empty() {
                        view! {
                            <SearchDialog
                                is_open=search_dialog_is_open
                                root_path=path.clone()
                                on_result_click=move |file_path: String, line: usize| {
                                    leptos::logging::log!("Search result clicked: {} at line {}", file_path, line);
                                    // TODO: Open file and jump to line
                                }
                            />
                        }.into_any()
                    } else {
                        view! { <></> }.into_any()
                    }
                }}
            </div>

            // Status Bar at bottom
            <StatusBar />
        </div>
    }
}
