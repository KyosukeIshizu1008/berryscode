use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos::html::Textarea;
use crate::tauri_bindings_berrycode::*;
use crate::focus_stack::{FocusStack, FocusLayer};

// Re-export for use in this module
pub use crate::tauri_bindings_berrycode::{ChatSessionData, ChatMessage};

#[component]
pub fn BerryCodePanel(
    /// Project root path
    #[prop(into)]
    project_path: Signal<String>,
    /// Focus stack for managing keyboard events
    focus_stack: RwSignal<FocusStack>,
) -> impl IntoView {
    leptos::logging::log!("🚀 BerryCodePanel component created!");

    let user_input = RwSignal::new(String::new());
    let chat_messages = RwSignal::new(Vec::<ChatMessage>::new());
    let is_loading = RwSignal::new(false);

    // Chat session management
    let current_chat_id = RwSignal::new(Option::<String>::None);
    let chat_sessions = RwSignal::new(Vec::<ChatSessionData>::new());
    let show_history_sidebar = RwSignal::new(false);

    // Connection status
    let is_connected = RwSignal::new(false);
    let connection_error = RwSignal::new(Option::<String>::None);

    // Input element reference for focus
    let input_ref = NodeRef::<Textarea>::new();

    // Initialize BerryCode session on mount
    create_effect(move |_| {
        let path = project_path.get();
        leptos::logging::log!("🔧 Initializing BerryCode session for: {}", path);
        spawn_local(async move {
            // 1. Initialize BerryCode gRPC session
            match berrycode_init(
                Some("gpt-4o".to_string()),
                Some("code".to_string()),
                Some(path.clone())
            ).await {
                Ok(msg) => {
                    leptos::logging::log!("✅ BerryCode initialized: {}", msg);
                    is_connected.set(true);
                    connection_error.set(None);

                    // 2. Create a new chat session
                    match berrycode_create_chat_session(None).await {
                        Ok(chat_id) => {
                            leptos::logging::log!("✅ Created chat session: {}", chat_id);
                            current_chat_id.set(Some(chat_id.clone()));

                            // 3. Load existing messages (if any)
                            match berrycode_load_chat_messages(chat_id).await {
                                Ok(messages) => {
                                    if !messages.is_empty() {
                                        leptos::logging::log!("✅ Loaded {} messages", messages.len());
                                        chat_messages.set(messages);
                                    }
                                }
                                Err(e) => {
                                    leptos::logging::error!("❌ Failed to load messages: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            leptos::logging::error!("❌ Failed to create chat session: {}", e);
                            connection_error.set(Some(format!("Failed to create chat session: {}", e)));
                        }
                    }
                }
                Err(e) => {
                    leptos::logging::error!("❌ BerryCode init failed: {}", e);
                    is_connected.set(false);
                    connection_error.set(Some(format!("Cannot connect to Berry API Server. Make sure it's running on port 50051. Error: {}", e)));
                }
            }
        });
    });

    // Send message logic
    let do_send_message = move || {
        let message = user_input.get();
        if message.trim().is_empty() {
            return;
        }

        // Get current chat session ID
        let Some(chat_id) = current_chat_id.get() else {
            leptos::logging::error!("❌ No active chat session");
            return;
        };

        // Add user message to chat
        chat_messages.update(|msgs| {
            msgs.push(ChatMessage {
                role: "user".to_string(),
                content: message.clone(),
            });
        });

        // Save user message to database
        let chat_id_clone = chat_id.clone();
        let message_clone = message.clone();
        spawn_local(async move {
            match berrycode_save_message(chat_id_clone, "user".to_string(), message_clone).await {
                Ok(msg_id) => {
                    leptos::logging::log!("✅ Saved user message (ID: {})", msg_id);
                }
                Err(e) => {
                    leptos::logging::error!("❌ Failed to save user message: {}", e);
                }
            }
        });

        user_input.set(String::new());
        is_loading.set(true);

        spawn_local(async move {
            match berrycode_chat(message).await {
                Ok(response) => {
                    chat_messages.update(|msgs| {
                        msgs.push(ChatMessage {
                            role: "assistant".to_string(),
                            content: response.clone(),
                        });
                    });

                    // Save AI response to database
                    let save_chat_id = chat_id.clone();
                    spawn_local(async move {
                        match berrycode_save_message(save_chat_id, "assistant".to_string(), response).await {
                            Ok(msg_id) => {
                                leptos::logging::log!("✅ Saved assistant message (ID: {})", msg_id);
                            }
                            Err(e) => {
                                leptos::logging::error!("❌ Failed to save assistant message: {}", e);
                            }
                        }
                    });
                }
                Err(e) => {
                    leptos::logging::error!("❌ Chat error: {}", e);
                    chat_messages.update(|msgs| {
                        msgs.push(ChatMessage {
                            role: "assistant".to_string(),
                            content: format!("Error: {}", e),
                        });
                    });
                }
            }
            is_loading.set(false);
        });
    };

    // Button click handler
    let send_message_click = move |_ev: leptos::ev::MouseEvent| {
        do_send_message();
    };

    // Enter key handler (not used for textarea, using Ctrl+Enter instead)

    view! {
        // ✅ Cursor AI風のチャットパネル
        <div class="flex flex-col h-full overflow-hidden" style="background: #1e1e1e;">
            // ✅ Cursor風のヘッダー
            <div class="flex flex-row items-center justify-between px-4 py-3 border-b" style="border-color: #2d2d2d; flex-shrink: 0;">
                // 左側: タイトル
                <div class="flex flex-row items-center gap-2">
                    <span style="font-size: 15px; font-weight: 600; color: #cccccc;">"AI Chat"</span>
                </div>

                // 右側: アクションボタン
                <div class="flex flex-row items-center gap-2">
                    <button
                        class="flex items-center gap-1 px-2 py-1 rounded hover:bg-berry-bg-tab-hover transition-colors"
                        style="border: 1px solid #3e3e3e; background: transparent; color: #cccccc; font-size: 12px;"
                        on:click=move |_| {
                            spawn_local(async move {
                                match berrycode_create_chat_session(None).await {
                                    Ok(new_chat_id) => {
                                        leptos::logging::log!("✅ Created new chat session: {}", new_chat_id);
                                        current_chat_id.set(Some(new_chat_id));
                                        chat_messages.set(Vec::new());  // Clear UI
                                    }
                                    Err(e) => {
                                        leptos::logging::error!("❌ Failed to create new chat: {}", e);
                                    }
                                }
                            });
                        }
                    >
                        <i class="codicon codicon-add" style="font-size: 14px;"></i>
                        <span>"New Chat"</span>
                    </button>
                    <button
                        class="p-1 rounded hover:bg-berry-bg-tab-hover transition-colors"
                        style="background: transparent; border: none; color: #858585;"
                        title="History"
                        on:click=move |_| {
                            show_history_sidebar.update(|v| *v = !*v);

                            // Load chat sessions when opening history
                            if show_history_sidebar.get() {
                                spawn_local(async move {
                                    match berrycode_list_chat_sessions().await {
                                        Ok(sessions) => {
                                            leptos::logging::log!("✅ Loaded {} chat sessions", sessions.len());
                                            chat_sessions.set(sessions);
                                        }
                                        Err(e) => {
                                            leptos::logging::error!("❌ Failed to load chat sessions: {}", e);
                                        }
                                    }
                                });
                            }
                        }
                    >
                        <i class="codicon codicon-history" style="font-size: 16px;"></i>
                    </button>
                    <button
                        class="p-1 rounded hover:bg-berry-bg-tab-hover transition-colors"
                        style="background: transparent; border: none; color: #858585;"
                        title="More"
                    >
                        <i class="codicon codicon-kebab-vertical" style="font-size: 16px;"></i>
                    </button>
                </div>
            </div>

            // Chat messages area (Cursor風)
            <div class="flex-1 overflow-y-auto px-4 py-4 flex flex-col gap-4" style="background: #1e1e1e;">
                // Connection error banner
                {move || {
                    if let Some(error) = connection_error.get() {
                        view! {
                            <div class="p-4 rounded-lg border" style="background: #3c1f1e; border-color: #f48771; color: #f48771;">
                                <div class="flex flex-row items-start gap-3">
                                    <i class="codicon codicon-warning" style="font-size: 20px; flex-shrink: 0;"></i>
                                    <div class="flex-1">
                                        <div class="font-semibold mb-1">"Connection Error"</div>
                                        <div class="text-sm opacity-90">{error}</div>
                                        <div class="text-xs opacity-75 mt-2">
                                            "💡 Tip: Start berry-api-server with: "
                                            <code style="background: rgba(255,255,255,0.1); padding: 2px 6px; border-radius: 4px;">
                                                "cargo run -- --rest-port 8081 --grpc-port 50051"
                                            </code>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        }.into_any()
                    } else {
                        view! { <></> }.into_any()
                    }
                }}

                {move || {
                    chat_messages.get().into_iter().map(|msg| {
                        if msg.role == "user" {
                            // ユーザーメッセージ（暗いカード）
                            view! {
                                <div class="p-4 rounded-lg" style="background: #2d2d2d;">
                                    <div class="text-sm text-white whitespace-pre-wrap">
                                        {msg.content.clone()}
                                    </div>
                                </div>
                            }.into_any()
                        } else {
                            // AIメッセージ（背景なし、アイコン付き）
                            view! {
                                <div class="flex flex-row gap-3">
                                    <div class="flex-shrink-0">
                                        <i class="codicon codicon-hubot" style="font-size: 20px; color: #858585;"></i>
                                    </div>
                                    <div class="flex-1 flex flex-col gap-2">
                                        <div class="text-xs font-semibold" style="color: #858585;">
                                            "AI Assistant"
                                        </div>
                                        <div class="text-sm whitespace-pre-wrap" style="color: #cccccc; line-height: 1.6;">
                                            {msg.content.clone()}
                                        </div>
                                        <div class="flex flex-row gap-2 mt-1">
                                            <button class="p-1 rounded hover:bg-berry-bg-tab-hover" style="background: transparent; border: none; color: #858585;">
                                                <i class="codicon codicon-copy" style="font-size: 14px;"></i>
                                            </button>
                                            <button class="p-1 rounded hover:bg-berry-bg-tab-hover" style="background: transparent; border: none; color: #858585;">
                                                <i class="codicon codicon-thumbsup" style="font-size: 14px;"></i>
                                            </button>
                                            <button class="p-1 rounded hover:bg-berry-bg-tab-hover" style="background: transparent; border: none; color: #858585;">
                                                <i class="codicon codicon-thumbsdown" style="font-size: 14px;"></i>
                                            </button>
                                        </div>
                                    </div>
                                </div>
                            }.into_any()
                        }
                    }).collect_view()
                }}
                {move || if is_loading.get() {
                    view! {
                        <div class="flex flex-row gap-3">
                            <div class="flex-shrink-0">
                                <i class="codicon codicon-hubot" style="font-size: 20px; color: #858585;"></i>
                            </div>
                            <div class="flex-1">
                                <div class="text-xs font-semibold mb-2" style="color: #858585;">
                                    "AI Assistant"
                                </div>
                                <div class="text-sm" style="color: #858585;">
                                    "Thinking..."
                                </div>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! { <></> }.into_any()
                }}
            </div>

            // Input area (Cursor風)
            <div class="p-4 border-t flex flex-col gap-2" style="border-color: #2d2d2d; background: #1e1e1e;">
                <textarea
                    node_ref=input_ref
                    placeholder="Ask AI Assistant, use @mentions or /commands"
                    class="w-full px-3 py-3 rounded-lg outline-none resize-none"
                    style="background: #2d2d2d; border: 1px solid #3e3e3e; color: #cccccc; font-size: 13px; min-height: 80px; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;"
                    prop:value=move || user_input.get()
                    on:input=move |ev| {
                        user_input.set(event_target_value(&ev));
                    }
                    on:keydown=move |ev: leptos::ev::KeyboardEvent| {
                        // Ctrl/Cmd+Enter to send
                        if (ev.ctrl_key() || ev.meta_key()) && ev.key() == "Enter" {
                            ev.prevent_default();
                            do_send_message();
                        }
                    }
                    on:focus=move |_| {
                        focus_stack.update(|stack| stack.push(FocusLayer::Dialog));
                        leptos::logging::log!("🎯 Chat input focused, pushed Dialog layer");
                    }
                    on:blur=move |_| {
                        focus_stack.update(|stack| stack.pop());
                        leptos::logging::log!("🎯 Chat input blurred, popped Dialog layer");
                    }
                ></textarea>

                <div class="flex flex-row items-center justify-between">
                    <div class="flex flex-row items-center gap-2">
                        <button
                            class="flex items-center gap-1 px-2 py-1 rounded"
                            style="background: transparent; border: 1px solid #3e3e3e; color: #858585; font-size: 12px;"
                        >
                            <i class="codicon codicon-add" style="font-size: 14px;"></i>
                            <span>"Chat"</span>
                            <i class="codicon codicon-chevron-down" style="font-size: 12px;"></i>
                        </button>
                        <button
                            class="flex items-center gap-1 px-2 py-1 rounded"
                            style="background: transparent; border: none; color: #858585; font-size: 12px;"
                        >
                            <span>"Auto"</span>
                            <i class="codicon codicon-chevron-down" style="font-size: 12px;"></i>
                        </button>
                    </div>

                    <button
                        class="p-2 rounded-lg transition-opacity"
                        style=move || {
                            if is_connected.get() {
                                "background: #007ACC; border: none; color: white;".to_string()
                            } else {
                                "background: #3e3e3e; border: none; color: #666666;".to_string()
                            }
                        }
                        on:click=send_message_click
                        prop:disabled=move || user_input.get().trim().is_empty() || is_loading.get() || !is_connected.get()
                        title=move || {
                            if is_connected.get() {
                                "Send message (Ctrl+Enter)".to_string()
                            } else {
                                "Cannot send: Berry API Server is not connected".to_string()
                            }
                        }
                    >
                        <i class="codicon codicon-send" style="font-size: 16px;"></i>
                    </button>
                </div>
            </div>
        </div>
    }
}
