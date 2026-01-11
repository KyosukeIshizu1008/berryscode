use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos::html::Input;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use crate::tauri_bindings_terminal::*;

/// Terminal output line
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalLine {
    pub text: String,
    pub is_command: bool,
}

#[component]
pub fn TerminalPanel(
    /// Project root path for terminal session
    #[prop(into)]
    project_path: Signal<String>,
) -> impl IntoView {
    leptos::logging::log!("🚀 TerminalPanel component created!");

    let command_input = RwSignal::new(String::new());
    let terminal_output = RwSignal::new(Vec::<TerminalLine>::new());
    let current_dir = RwSignal::new(String::from("~"));
    let command_history = RwSignal::new(Vec::<String>::new());
    let history_index = RwSignal::new(0usize);
    let background_processes = RwSignal::new(Vec::<BackgroundProcessInfo>::new());

    // Input element reference for manual focus
    let input_ref = NodeRef::<Input>::new();

    // Focus the input element after mount
    create_effect(move |_| {
        // Wait for the input to be mounted
        if let Some(input_el) = input_ref.get() {
            leptos::logging::log!("📍 Input element found, attempting to focus...");

            // Use setTimeout to ensure DOM is fully ready
            let window = web_sys::window().expect("should have window");
            let input_clone = input_el.clone();

            let closure = wasm_bindgen::closure::Closure::once(Box::new(move || {
                leptos::logging::log!("⏰ setTimeout callback executing...");
                match input_clone.focus() {
                    Ok(_) => {
                        leptos::logging::log!("✅ Focus successful!");
                        // Double-check if focus worked
                        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                            if let Some(active) = doc.active_element() {
                                let is_focused = active.is_same_node(Some(input_clone.as_ref()));
                                leptos::logging::log!("🔍 Is input focused? {}", is_focused);
                            }
                        }
                    }
                    Err(e) => {
                        leptos::logging::error!("❌ Focus failed: {:?}", e);
                    }
                }
            }) as Box<dyn FnOnce()>);

            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                closure.as_ref().unchecked_ref(),
                100 // 100ms delay
            );
            closure.forget();
        } else {
            leptos::logging::log!("⏳ Waiting for input element to mount...");
        }
    });

    // Load current directory on mount
    create_effect(move |_| {
        let path = project_path.get();
        leptos::logging::log!("📂 TerminalPanel mounted with project_path: {}", path);
        spawn_local(async move {
            leptos::logging::log!("🔍 Fetching current directory for: {}", path);
            match terminal_get_current_directory(path.clone()).await {
                Ok(cwd) => {
                    leptos::logging::log!("✅ Current directory: {}", cwd);
                    current_dir.set(cwd);
                }
                Err(e) => {
                    leptos::logging::error!("❌ Failed to get current directory: {}", e);
                }
            }
        });
    });

    // Execute command
    let execute_command = move |background: bool| {
        let cmd = command_input.get();
        leptos::logging::log!("🔍 execute_command called: cmd='{}', background={}", cmd, background);

        if cmd.is_empty() {
            leptos::logging::warn!("⚠️ Command is empty, skipping execution");
            return;
        }

        leptos::logging::log!("📝 Adding command to terminal output: $ {}", cmd);

        // Add to terminal output
        terminal_output.update(|lines| {
            lines.push(TerminalLine {
                text: format!("$ {}", cmd),
                is_command: true,
            });
        });

        // Add to history
        command_history.update(|history| {
            history.push(cmd.clone());
        });
        history_index.set(0);

        // Clear input
        command_input.set(String::new());

        // Execute
        let path = project_path.get();
        leptos::logging::log!("🚀 Spawning terminal command execution for path: {}", path);

        spawn_local(async move {
            leptos::logging::log!("🔄 Calling terminal_execute_command(path='{}', cmd='{}', background={})", path, cmd, background);

            match terminal_execute_command(path.clone(), cmd.clone(), Some(background)).await {
                    Ok(response) => {
                        leptos::logging::log!("✅ Terminal command succeeded: success={}, process_id={:?}", response.success, response.process_id);

                        if let Some(process_id) = response.process_id {
                            terminal_output.update(|lines| {
                                lines.push(TerminalLine {
                                    text: format!("[Background process started: {}]", process_id),
                                    is_command: false,
                                });
                            });
                            // Refresh background processes list
                            if let Ok(processes) = terminal_list_background_processes(path.clone()).await {
                                background_processes.set(processes);
                            }
                        } else {
                            // Normal command output
                            for line in response.output.lines() {
                                terminal_output.update(|lines| {
                                    lines.push(TerminalLine {
                                        text: line.to_string(),
                                        is_command: false,
                                    });
                                });
                            }
                        }

                        // Update current directory
                        if let Ok(cwd) = terminal_get_current_directory(path).await {
                            current_dir.set(cwd);
                        }
                    }
                    Err(e) => {
                        leptos::logging::error!("❌ Terminal command failed: {}", e);
                        terminal_output.update(|lines| {
                            lines.push(TerminalLine {
                                text: format!("Error: {}", e),
                                is_command: false,
                            });
                        });
                    }
                }
        });
    };

    // Handle Enter key
    let on_keydown = move |ev: leptos::ev::KeyboardEvent| {
        let key = ev.key();
        leptos::logging::log!("⌨️ Terminal keydown: key='{}', shift={}", key, ev.shift_key());

        if key == "Enter" {
            leptos::logging::log!("✅ Enter key detected, executing command");
            ev.prevent_default();
            let shift_key = ev.shift_key();
            execute_command(shift_key); // Shift+Enter = background
        } else if key == "ArrowUp" {
            ev.prevent_default();
            let history = command_history.get();
            let idx = history_index.get();
            if idx < history.len() {
                history_index.update(|i| *i += 1);
                if let Some(cmd) = history.get(history.len() - idx - 1) {
                    command_input.set(cmd.clone());
                }
            }
        } else if ev.key() == "ArrowDown" {
            ev.prevent_default();
            let history = command_history.get();
            let idx = history_index.get();
            if idx > 1 {
                history_index.update(|i| *i -= 1);
                if let Some(cmd) = history.get(history.len() - idx + 1) {
                    command_input.set(cmd.clone());
                }
            } else {
                history_index.set(0);
                command_input.set(String::new());
            }
        }
    };

    // Kill background process
    let kill_process = move |process_id: String| {
        let path = project_path.get();
        spawn_local(async move {
            if terminal_kill_process(path.clone(), process_id.clone()).await.is_ok() {
                    terminal_output.update(|lines| {
                        lines.push(TerminalLine {
                            text: format!("[Killed background process: {}]", process_id),
                            is_command: false,
                        });
                    });
                    // Refresh background processes list
                    if let Ok(processes) = terminal_list_background_processes(path).await {
                        background_processes.set(processes);
                    }
                }
        });
    };

    view! {
        <div class="terminal-panel" style="display: flex; flex-direction: column; width: 100%; height: 100%; background: var(--bg-main); color: var(--tree-text); font-family: 'Consolas', 'Courier New', monospace;">
            // Header
            <div class="terminal-header" style="padding: 8px 12px; background: #2d2d30; border-bottom: 1px solid var(--border-color); display: flex; justify-content: space-between; align-items: center;">
                <div style="font-weight: bold; color: var(--tree-text-active);">Terminal</div>
                <div style="font-size: 12px; color: var(--tree-text);">{move || current_dir.get()}</div>
            </div>

            // Output area
            <div class="terminal-output" style="flex: 1; overflow-y: auto; padding: 8px 12px; font-size: 13px; line-height: 1.5;">
                <For
                    each=move || terminal_output.get()
                    key=|line| line.text.clone()
                    children=move |line| {
                        view! {
                            <div style=move || if line.is_command {
                                "color: var(--color-success); font-weight: bold;"
                            } else {
                                "color: var(--tree-text);"
                            }>
                                {line.text.clone()}
                            </div>
                        }
                    }
                />
            </div>

            // Background processes
            <Show when=move || !background_processes.get().is_empty()>
                <div class="background-processes" style="padding: 8px 12px; background: var(--bg-sidebar); border-top: 1px solid var(--border-color); max-height: 150px; overflow-y: auto;">
                    <div style="font-size: 11px; color: var(--icon-muted); margin-bottom: 4px;">Background Processes:</div>
                    <For
                        each=move || background_processes.get()
                        key=|p| p.id.clone()
                        children=move |process| {
                            let process_id = process.id.clone();
                            view! {
                                <div style="display: flex; justify-content: space-between; align-items: center; padding: 4px; font-size: 12px; background: var(--bg-main); margin-bottom: 2px; border-radius: 3px;">
                                    <div>
                                        <span style="color: var(--color-success);">{process.command.clone()}</span>
                                        <span style="color: var(--icon-muted); margin-left: 8px;">{"(PID: "}{process.pid}{")"}</span>
                                        <span style="color: var(--icon-muted); margin-left: 8px;">{process.status.clone()}</span>
                                    </div>
                                    <button
                                        on:click=move |_| kill_process(process_id.clone())
                                        style="background: var(--color-error); color: white; border: none; padding: 2px 8px; border-radius: 3px; cursor: pointer; font-size: 11px;"
                                    >
                                        Kill
                                    </button>
                                </div>
                            }
                        }
                    />
                </div>
            </Show>

            // Input area
            <div
                class="terminal-input"
                style="padding: 8px 12px; background: #2d2d30; border-top: 1px solid var(--border-color); display: flex; align-items: center; cursor: text;"
            >
                <span
                    style="color: var(--color-success); margin-right: 8px; user-select: none;"
                    on:click=move |_| {
                        leptos::logging::log!("🖱️ $ prompt clicked, focusing input");
                        if let Some(input_el) = input_ref.get() {
                            let _ = input_el.focus();
                        }
                    }
                >$</span>
                <input
                    node_ref=input_ref
                    type="text"
                    prop:value=move || command_input.get()
                    on:input=move |ev| {
                        let value = event_target_value(&ev);
                        leptos::logging::log!("📝 Terminal input changed: '{}'", value);
                        command_input.set(value);
                    }
                    on:keydown=on_keydown
                    on:focus=move |_| {
                        leptos::logging::log!("🎯 Terminal input focused");
                    }
                    on:blur=move |_| {
                        leptos::logging::log!("👋 Terminal input blurred");
                    }
                    placeholder="Enter command (Shift+Enter for background)"
                    style="flex: 1; background: transparent; color: var(--tree-text); border: none; padding: 0; outline: none; font-family: 'Consolas', 'Courier New', monospace; font-size: 13px; caret-color: var(--tree-text);"
                />
            </div>
        </div>
    }
}
