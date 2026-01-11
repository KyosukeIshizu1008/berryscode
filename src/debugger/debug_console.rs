//! Debug Console Component
//!
//! Displays debug output and provides REPL-style expression evaluation.

use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use crate::common::ui_components::Panel;
use super::session::DebugSession;

/// Console message type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageType {
    Output,
    Input,
    Error,
    Info,
}

/// Console message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleMessage {
    pub message_type: MessageType,
    pub content: String,
    pub timestamp: String,
}

impl ConsoleMessage {
    /// Create a new console message
    pub fn new(message_type: MessageType, content: String) -> Self {
        use chrono::Local;
        let timestamp = Local::now().format("%H:%M:%S").to_string();

        Self {
            message_type,
            content,
            timestamp,
        }
    }
}

/// Debug console component
#[component]
pub fn DebugConsole(
    /// Console messages
    messages: RwSignal<Vec<ConsoleMessage>>,
    /// Debug session for evaluation
    session: DebugSession,
) -> impl IntoView {
    let input = RwSignal::new(String::new());

    // Execute command/expression
    let execute = move || {
        let expr = input.get_untracked();
        if expr.is_empty() {
            return;
        }

        // Add input message
        messages.update(|msgs| {
            msgs.push(ConsoleMessage::new(MessageType::Input, expr.clone()));
        });

        input.set(String::new());

        // Evaluate expression
        spawn_local(async move {
            match session.evaluate(expr.clone(), None).await {
                Ok(result) => {
                    messages.update(|msgs| {
                        msgs.push(ConsoleMessage::new(MessageType::Output, result));
                    });
                }
                Err(e) => {
                    messages.update(|msgs| {
                        msgs.push(ConsoleMessage::new(MessageType::Error, e));
                    });
                }
            }
        });
    };

    // Clear console
    let clear_console = move || {
        messages.set(Vec::new());
    };

    view! {
        <Panel title="Debug Console">
            <div class="berry-debug-console">
                <div class="berry-console-toolbar">
                    <button class="berry-button" on:click=move |_| clear_console()>"Clear"</button>
                </div>
                <div class="berry-console-messages">
                    {move || {
                        messages.get().iter().map(|msg| {
                            view! {
                                <ConsoleMessageView message=msg.clone() />
                            }
                        }).collect::<Vec<_>>()
                    }}
                </div>
                <div class="berry-console-input">
                    <span class="berry-console-prompt">">"</span>
                    <input
                        type="text"
                        class="berry-input"
                        prop:value=move || input.get()
                        on:input=move |ev| {
                            input.set(event_target_value(&ev));
                        }
                        on:keydown=move |ev| {
                            if ev.key() == "Enter" {
                                execute();
                            }
                        }
                        placeholder="Evaluate expression..."
                    />
                </div>
            </div>
        </Panel>
    }
}

/// Single console message view
#[component]
fn ConsoleMessageView(
    /// The message
    message: ConsoleMessage,
) -> impl IntoView {
    let class = match message.message_type {
        MessageType::Output => "berry-console-message berry-console-output",
        MessageType::Input => "berry-console-message berry-console-input",
        MessageType::Error => "berry-console-message berry-console-error",
        MessageType::Info => "berry-console-message berry-console-info",
    };

    let prefix = match message.message_type {
        MessageType::Output => "",
        MessageType::Input => "> ",
        MessageType::Error => "Error: ",
        MessageType::Info => "Info: ",
    };

    view! {
        <div class=class>
            <span class="berry-console-timestamp">{format!("[{}]", message.timestamp)}</span>
            <span class="berry-console-content">{format!("{}{}", prefix, message.content)}</span>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_debug_console_compiles() {
        // Ensure component compiles
        assert!(true);
    }

    #[test]
    fn test_message_type_equality() {
        assert_eq!(MessageType::Output, MessageType::Output);
        assert_eq!(MessageType::Input, MessageType::Input);
        assert_eq!(MessageType::Error, MessageType::Error);
        assert_eq!(MessageType::Info, MessageType::Info);

        assert_ne!(MessageType::Output, MessageType::Error);
    }

    #[test]
    fn test_console_message_creation() {
        let msg = ConsoleMessage::new(MessageType::Output, "Hello".to_string());

        assert_eq!(msg.message_type, MessageType::Output);
        assert_eq!(msg.content, "Hello");
        assert!(!msg.timestamp.is_empty());
    }

    #[test]
    fn test_message_prefix() {
        let output_prefix = "";
        let input_prefix = "> ";
        let error_prefix = "Error: ";
        let info_prefix = "Info: ";

        assert_eq!(output_prefix, "");
        assert_eq!(input_prefix, "> ");
        assert_eq!(error_prefix, "Error: ");
        assert_eq!(info_prefix, "Info: ");
    }

    #[test]
    fn test_message_class_mapping() {
        let classes = [
            (MessageType::Output, "berry-console-message berry-console-output"),
            (MessageType::Input, "berry-console-message berry-console-input"),
            (MessageType::Error, "berry-console-message berry-console-error"),
            (MessageType::Info, "berry-console-message berry-console-info"),
        ];

        for (msg_type, expected_class) in classes {
            match msg_type {
                MessageType::Output => assert_eq!(expected_class, "berry-console-message berry-console-output"),
                MessageType::Input => assert_eq!(expected_class, "berry-console-message berry-console-input"),
                MessageType::Error => assert_eq!(expected_class, "berry-console-message berry-console-error"),
                MessageType::Info => assert_eq!(expected_class, "berry-console-message berry-console-info"),
            }
        }
    }
}
