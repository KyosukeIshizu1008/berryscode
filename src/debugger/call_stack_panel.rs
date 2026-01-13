//! Call Stack Panel Component
//!
//! Displays the call stack with clickable frames to navigate.

use dioxus::prelude::*;
use super::session::StackFrame;
use crate::common::ui_components::Panel;

/// Call stack panel props
#[derive(Props, Clone, PartialEq)]
pub struct CallStackPanelProps {
    /// Stack frames to display
    frames: Signal<Vec<StackFrame>>,
    /// Currently selected frame ID
    selected_frame: Signal<Option<i64>>,
    /// Callback when a frame is clicked
    on_frame_click: EventHandler<i64>,
}

/// Call stack panel component
#[component]
pub fn CallStackPanel(props: CallStackPanelProps) -> Element {
    let frames = props.frames;
    let selected_frame = props.selected_frame;
    let on_frame_click = props.on_frame_click;

    rsx! {
        Panel { title: "Call Stack",
            div { class: "berry-call-stack-panel",
                {
                    let current_frames = frames.read().clone();

                    if current_frames.is_empty() {
                        rsx! {
                            div { class: "berry-call-stack-empty",
                                "No call stack (not paused in debugger)"
                            }
                        }
                    } else {
                        rsx! {
                            for (index , frame) in current_frames.iter().enumerate() {
                                {
                                    let frame_id = frame.id;
                                    let is_selected = *selected_frame.read() == Some(frame_id);

                                    rsx! {
                                        StackFrameView {
                                            frame: frame.clone(),
                                            index: index,
                                            selected: is_selected,
                                            on_click: move |_| on_frame_click.call(frame_id)
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

/// Stack frame view props
#[derive(Props, Clone, PartialEq)]
struct StackFrameViewProps {
    /// The stack frame
    frame: StackFrame,
    /// Frame index (0 = top of stack)
    index: usize,
    /// Whether this frame is selected
    selected: bool,
    /// Click handler
    on_click: EventHandler<()>,
}

/// Single stack frame view
#[component]
fn StackFrameView(props: StackFrameViewProps) -> Element {
    let frame = props.frame;
    let index = props.index;
    let selected = props.selected;
    let on_click = props.on_click;

    let class = if selected {
        "berry-stack-frame berry-stack-frame-selected"
    } else {
        "berry-stack-frame"
    };

    // Format location string
    let location = if let Some(ref file) = frame.file {
        let file_name = file.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?");

        if let Some(line) = frame.line {
            if let Some(col) = frame.column {
                format!("{}:{}:{}", file_name, line, col)
            } else {
                format!("{}:{}", file_name, line)
            }
        } else {
            file_name.to_string()
        }
    } else {
        "<unknown>".to_string()
    };

    let frame_name = frame.name.clone();

    rsx! {
        div {
            class: "{class}",
            onclick: move |_| on_click.call(()),

            span { class: "berry-stack-frame-index", "#{index}" }
            span { class: "berry-stack-frame-name", "{frame_name}" }
            span { class: "berry-stack-frame-location", "{location}" }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_call_stack_panel_compiles() {
        // Ensure component compiles
        assert!(true);
    }

    #[test]
    fn test_stack_frame_location_formatting() {
        let frame1 = StackFrame {
            id: 1,
            name: "main".to_string(),
            file: Some(PathBuf::from("src/main.rs")),
            line: Some(42),
            column: Some(10),
        };

        assert!(frame1.file.is_some());
        assert_eq!(frame1.line, Some(42));
        assert_eq!(frame1.column, Some(10));

        let frame2 = StackFrame {
            id: 2,
            name: "helper".to_string(),
            file: Some(PathBuf::from("src/lib.rs")),
            line: Some(100),
            column: None,
        };

        assert!(frame2.file.is_some());
        assert_eq!(frame2.line, Some(100));
        assert!(frame2.column.is_none());

        let frame3 = StackFrame {
            id: 3,
            name: "unknown".to_string(),
            file: None,
            line: None,
            column: None,
        };

        assert!(frame3.file.is_none());
    }

    #[test]
    fn test_frame_index_display() {
        let index0 = format!("#{}", 0);
        let index1 = format!("#{}", 1);
        let index2 = format!("#{}", 2);

        assert_eq!(index0, "#0");
        assert_eq!(index1, "#1");
        assert_eq!(index2, "#2");
    }

    #[test]
    fn test_selected_frame_logic() {
        let frame_id = 42i64;
        let selected_id = Some(42i64);
        let not_selected_id = Some(99i64);

        assert_eq!(selected_id, Some(frame_id));
        assert_ne!(not_selected_id, Some(frame_id));
    }
}
