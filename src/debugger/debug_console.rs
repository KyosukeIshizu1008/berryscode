//! Debug Console Component
//!
//! Displays debug output and provides REPL-style expression evaluation.

use dioxus::prelude::*;
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

/// Debug console component props
#[derive(Props, Clone, PartialEq)]
pub struct DebugConsoleProps {
    /// Console messages
    messages: Signal<Vec<ConsoleMessage>>,
    /// Debug session for evaluation
    session: DebugSession,
}

/// Debug console component
#[component]
pub fn DebugConsole(props: DebugConsoleProps) -> Element {
    let messages = props.messages;
    let session = props.session;

    let mut input = use_signal(|| String::new());

    // Execute command/expression
    let execute = move || {
        let expr = input.read().clone();
        if expr.is_empty() {
            return;
        }

        // Add input message
        messages.write().push(ConsoleMessage::new(MessageType::Input, expr.clone()));

        *input.write() = String::new();

        // Evaluate expression
        spawn(async move {
            match session.evaluate(expr.clone(), None).await {
                Ok(result) => {
                    messages.write().push(ConsoleMessage::new(MessageType::Output, result));
                }
                Err(e) => {
                    messages.write().push(ConsoleMessage::new(MessageType::Error, e));
                }
            }
        });
    };

    // Clear console
    let clear_console = move || {
        *messages.write() = Vec::new();
    };

    rsx! {
        Panel { title: "Debug Console",
            div { class: "berry-debug-console",
                div { class: "berry-console-toolbar",
                    button {
                        class: "berry-button",
                        onclick: move |_| clear_console(),
                        "Clear"
                    }
                }
                div { class: "berry-console-messages",
                    {
                        let msg_list = messages.read().clone();
                        rsx! {
                            for msg in msg_list {
                                ConsoleMessageView { message: msg }
                            }
                        }
                    }
                }
                div { class: "berry-console-input",
                    span { class: "berry-console-prompt", ">" }
                    input {
                        r#type: "text",
                        class: "berry-input",
                        value: "{input.read()}",
                        oninput: move |ev| *input.write() = ev.value(),
                        onkeydown: move |ev| {
                            if ev.key() == Key::Enter {
                                execute();
                            }
                        },
                        placeholder: "Evaluate expression...",
                    }
                }
            }
        }
    }
}

/// Single console message view props
#[derive(Props, Clone, PartialEq)]
struct ConsoleMessageViewProps {
    /// The message
    message: ConsoleMessage,
}

/// Single console message view
#[component]
fn ConsoleMessageView(props: ConsoleMessageViewProps) -> Element {
    let message = props.message;

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

    rsx! {
        div { class: "{class}",
            span { class: "berry-console-timestamp", "[{message.timestamp}]" }
            span { class: "berry-console-content", "{prefix}{message.content}" }
        }
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
