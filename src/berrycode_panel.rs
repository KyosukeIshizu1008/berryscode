use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos::html::{Textarea, Div};
use crate::tauri_bindings_berrycode::*;
use crate::focus_stack::{FocusStack, FocusLayer};
use wasm_bindgen::JsCast;

// Re-export for use in this module
pub use crate::tauri_bindings_berrycode::{ChatSessionData, ChatMessage};

/// Render diff lines with color coding (GitHub/VSCode style)
fn render_diff_lines(code: String) -> Vec<AnyView> {
    code.lines()
        .filter(|line| !line.trim().is_empty()) // Skip empty lines
        .map(|line| {
            let trimmed = line.trim_start();
            let (bg_color, text_color) = if trimmed.starts_with('+') && !trimmed.starts_with("+++") {
                // Added line (but not +++ header) - bright green on dark green
                ("#1f4d1f", "#a8f5a8")
            } else if trimmed.starts_with('-') && !trimmed.starts_with("---") {
                // Removed line (but not --- header) - bright red on dark red
                ("#5a1f1f", "#ffa8a8")
            } else if trimmed.starts_with("@@") {
                // Hunk header - blue
                ("#1a2a3d", "#89b4fa")
            } else if trimmed.starts_with("---") || trimmed.starts_with("+++") {
                // File headers - gray
                ("#1a1a1a", "#9ca3af")
            } else {
                // Context line - dark background
                ("#1e1e1e", "#d4d4d4")
            };

            let line_text = if line.is_empty() { " ".to_string() } else { line.to_string() };
            let style_str = format!(
                "background-color: {}; color: {}; font-family: 'Monaco', 'Menlo', 'Consolas', monospace; font-size: 13px; line-height: 1.5; padding: 2px 12px; margin: 0;",
                bg_color, text_color
            );
            view! {
                <div style=style_str>
                    {line_text}
                </div>
            }.into_any()
        })
        .collect()
}

/// Render text with bold markdown support
fn render_text_with_markdown(text: String) -> AnyView {
    let parts: Vec<&str> = text.split("**").collect();
    let mut elements = Vec::new();

    for (i, part) in parts.iter().enumerate() {
        if i % 2 == 0 {
            // Normal text
            if !part.is_empty() {
                elements.push(view! {
                    <span>{part.to_string()}</span>
                }.into_any());
            }
        } else {
            // Bold text
            elements.push(view! {
                <strong style="color: #fff; font-weight: 600;">{part.to_string()}</strong>
            }.into_any());
        }
    }

    view! {
        <div class="whitespace-pre-wrap" style="color: #cccccc; line-height: 1.6;">
            {elements}
        </div>
    }.into_any()
}

/// Simple Markdown-like renderer for chat messages
fn render_message_content(content: String) -> Vec<AnyView> {
    let mut views = Vec::new();
    let mut remaining = content.as_str();

    leptos::logging::log!("📝 Rendering content: {}", &content[..content.len().min(100)]);

    while !remaining.is_empty() {
        // Check for code blocks (```...```)
        if let Some(start_idx) = remaining.find("```") {
            // Add text before code block with markdown support
            if start_idx > 0 {
                let text = remaining[..start_idx].to_string();
                views.push(render_text_with_markdown(text));
            }

            // Find end of code block
            let after_start = &remaining[start_idx + 3..];
            if let Some(end_idx) = after_start.find("```") {
                // Extract language hint (first line)
                let code_content = &after_start[..end_idx];
                let (lang, code) = if let Some(newline_idx) = code_content.find('\n') {
                    (code_content[..newline_idx].trim().to_string(), code_content[newline_idx + 1..].to_string())
                } else {
                    (String::new(), code_content.to_string())
                };

                leptos::logging::log!("📦 Code block found - lang: '{}', has_diff_markers: {}",
                    &lang,
                    code.contains("---") && code.contains("+++"));

                // Check if this is a diff block (check both lang tag and content)
                let is_diff = lang.to_lowercase() == "diff"
                    || (code.contains("---") && code.contains("+++") && code.contains("@@"));

                views.push(view! {
                    <div class="my-2">
                        {if !lang.is_empty() && !is_diff {
                            view! {
                                <div class="text-xs px-3 py-1 rounded-t" style="background: #2d2d2d; color: #858585; font-family: monospace;">
                                    {lang.clone()}
                                </div>
                            }.into_any()
                        } else {
                            view! { <></> }.into_any()
                        }}
                        {if is_diff {
                            // Render diff with colored lines
                            let diff_lines = render_diff_lines(code.clone());
                            view! {
                                <div class="overflow-x-auto" style="background: #0d0d0d; border: 1px solid #3e3e3e; border-radius: 4px; padding: 0;">
                                    {diff_lines}
                                </div>
                            }.into_any()
                        } else {
                            // Regular code block
                            view! {
                                <pre class="p-3 rounded overflow-x-auto" style="background: #1e1e1e; border: 1px solid #3e3e3e;">
                                    <code class="text-sm" style="color: #d4d4d4; font-family: 'Monaco', 'Menlo', 'Consolas', monospace;">
                                        {code}
                                    </code>
                                </pre>
                            }.into_any()
                        }}
                    </div>
                }.into_any());

                remaining = &after_start[end_idx + 3..];
            } else {
                // No closing ```, treat as regular text with markdown
                let text = remaining.to_string();
                views.push(render_text_with_markdown(text));
                break;
            }
        } else {
            // No code blocks, add remaining text with markdown
            let text = remaining.to_string();
            views.push(render_text_with_markdown(text));
            break;
        }
    }

    views
}

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

    // Mode management: autonomous vs interactive
    let autonomous_mode = RwSignal::new(true); // Default to autonomous (auto-continue)

    // Input element reference for focus
    let input_ref = NodeRef::<Textarea>::new();

    // Chat messages area reference for auto-scroll
    let chat_area_ref = NodeRef::<Div>::new();

    // Helper function to scroll to bottom
    let scroll_to_bottom = move || {
        if let Some(element) = chat_area_ref.get() {
            let element = element.unchecked_into::<web_sys::HtmlElement>();
            element.set_scroll_top(element.scroll_height());
        }
    };

    // Initialize BerryCode session on mount
    create_effect(move |_| {
        let path = project_path.get();
        let autonomous = autonomous_mode.get();
        leptos::logging::log!("🔧 Initializing BerryCode session for: {} (autonomous: {})", path, autonomous);
        spawn_local(async move {
            // 1. Initialize BerryCode gRPC session
            match berrycode_init(
                Some("qwen3-coder:30b".to_string()),  // プログラミング特化モデル
                Some("code".to_string()),
                Some(path.clone()),
                Some(autonomous)
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

    // Signal to accumulate streaming response
    let streaming_content = RwSignal::new(String::new());
    let is_streaming = RwSignal::new(false);

    // Send message logic with real-time streaming
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

        // Auto-scroll to bottom after adding message
        request_animation_frame(move || scroll_to_bottom());

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
        is_streaming.set(true);
        streaming_content.set(String::new());

        // Add placeholder for assistant message
        chat_messages.update(|msgs| {
            msgs.push(ChatMessage {
                role: "assistant".to_string(),
                content: String::new(),
            });
        });

        // Setup event listeners for streaming
        use wasm_bindgen::prelude::*;
        use crate::tauri_bindings_berrycode::{listen_stream_chunk, listen_stream_end, listen_stream_error};

        // Chunk handler - updates last message in real-time
        let chunk_callback = Closure::wrap(Box::new(move |chunk: String| {
            leptos::logging::log!("📡 Stream chunk: {}", chunk);
            streaming_content.update(|content| content.push_str(&chunk));

            // Update the last message (assistant) in place
            chat_messages.update(|msgs| {
                if let Some(last_msg) = msgs.last_mut() {
                    if last_msg.role == "assistant" {
                        last_msg.content = streaming_content.get();
                    }
                }
            });

            // Auto-scroll to bottom as content arrives
            request_animation_frame(move || scroll_to_bottom());
        }) as Box<dyn Fn(String)>);

        // End handler - finalize and save
        let end_callback = Closure::wrap(Box::new(move || {
            leptos::logging::log!("✅ Stream completed");
            is_loading.set(false);
            is_streaming.set(false);

            let final_content = streaming_content.get();
            let save_chat_id = chat_id.clone();

            // Save completed assistant message to database
            spawn_local(async move {
                match berrycode_save_message(save_chat_id, "assistant".to_string(), final_content).await {
                    Ok(msg_id) => {
                        leptos::logging::log!("✅ Saved assistant message (ID: {})", msg_id);
                    }
                    Err(e) => {
                        leptos::logging::error!("❌ Failed to save assistant message: {}", e);
                    }
                }
            });
        }) as Box<dyn Fn()>);

        // Error handler
        let error_callback = Closure::wrap(Box::new(move |error: String| {
            leptos::logging::error!("❌ Stream error: {}", error);
            is_loading.set(false);
            is_streaming.set(false);

            chat_messages.update(|msgs| {
                if let Some(last_msg) = msgs.last_mut() {
                    if last_msg.role == "assistant" && last_msg.content.is_empty() {
                        last_msg.content = format!("Error: {}", error);
                    }
                }
            });

            request_animation_frame(move || scroll_to_bottom());
        }) as Box<dyn Fn(String)>);

        // Register event listeners
        let _ = listen_stream_chunk(&chunk_callback);
        let _ = listen_stream_end(&end_callback);
        let _ = listen_stream_error(&error_callback);

        // Keep closures alive
        chunk_callback.forget();
        end_callback.forget();
        error_callback.forget();

        // Invoke berrycode_chat command (streams via events)
        spawn_local(async move {
            match berrycode_chat(message).await {
                Ok(_) => {
                    leptos::logging::log!("✅ Chat command invoked successfully");
                }
                Err(e) => {
                    leptos::logging::error!("❌ Chat command failed: {}", e);
                    is_loading.set(false);
                    is_streaming.set(false);

                    chat_messages.update(|msgs| {
                        if let Some(last_msg) = msgs.last_mut() {
                            if last_msg.role == "assistant" && last_msg.content.is_empty() {
                                last_msg.content = format!("Error: {}", e);
                            }
                        }
                    });
                }
            }
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
                    // Mode toggle button: Autonomous vs Interactive
                    <button
                        class="flex items-center gap-1 px-2 py-1 rounded hover:bg-berry-bg-tab-hover transition-colors"
                        style=move || format!(
                            "border: 1px solid #3e3e3e; background: {}; color: #cccccc; font-size: 11px;",
                            if autonomous_mode.get() { "#2a5a2a" } else { "#5a2a2a" }
                        )
                        title=move || if autonomous_mode.get() {
                            "自律型モード: AIが自動でタスクを完了します"
                        } else {
                            "対話型モード: ツール実行ごとに確認します"
                        }
                        on:click=move |_| {
                            autonomous_mode.update(|v| *v = !*v);
                            let new_mode = autonomous_mode.get();
                            leptos::logging::log!("🔄 Mode switched to: {}", if new_mode { "Autonomous" } else { "Interactive" });

                            // Re-initialize session with new mode
                            let path = project_path.get();
                            spawn_local(async move {
                                match berrycode_init(
                                    Some("qwen3-coder:30b".to_string()),
                                    Some("code".to_string()),
                                    Some(path.clone()),
                                    Some(new_mode)
                                ).await {
                                    Ok(msg) => leptos::logging::log!("✅ Session reinitialized: {}", msg),
                                    Err(e) => leptos::logging::error!("❌ Failed to reinitialize: {}", e),
                                }
                            });
                        }
                    >
                        <i class=move || if autonomous_mode.get() {
                            "codicon codicon-debug-start"
                        } else {
                            "codicon codicon-debug-step-over"
                        } style="font-size: 13px;"></i>
                        <span>{move || if autonomous_mode.get() { "自律型" } else { "対話型" }}</span>
                    </button>

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
            <div
                node_ref=chat_area_ref
                class="flex-1 overflow-y-auto px-4 py-4 flex flex-col gap-4"
                style="background: #1e1e1e;"
            >
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
                            // AIメッセージ（背景なし、アイコン付き、Markdown風）
                            let content_views = render_message_content(msg.content.clone());
                            view! {
                                <div class="flex flex-row gap-3">
                                    <div class="flex-shrink-0">
                                        <i class="codicon codicon-hubot" style="font-size: 20px; color: #858585;"></i>
                                    </div>
                                    <div class="flex-1 flex flex-col gap-2">
                                        <div class="text-xs font-semibold" style="color: #858585;">
                                            "AI Assistant"
                                        </div>
                                        <div class="text-sm">
                                            {content_views}
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
