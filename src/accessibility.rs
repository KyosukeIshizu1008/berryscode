//! Accessibility Layer for Screen Readers
//!
//! Since the editor uses 100% Canvas rendering, screen readers cannot read text.
//! This module provides a hidden DOM mirror that synchronizes with the Canvas content.

use dioxus::prelude::*;
use crate::buffer::TextBuffer;

/// Accessibility layer props
#[derive(Props, Clone, PartialEq)]
pub struct AccessibilityLayerProps {
    buffer: Signal<TextBuffer>,
    cursor_line: Signal<usize>,
    cursor_col: Signal<usize>,
    visible_start: Signal<usize>,
    visible_end: Signal<usize>,
}

/// Render accessibility layer for screen readers
/// This creates a hidden DOM representation of visible text
#[component]
pub fn AccessibilityLayer(props: AccessibilityLayerProps) -> Element {
    let buffer = props.buffer;
    let cursor_line = props.cursor_line;
    let cursor_col = props.cursor_col;
    let visible_start = props.visible_start;
    let visible_end = props.visible_end;

    rsx! {
        div {
            class: "berry-editor-a11y",
            role: "textbox",
            "aria-multiline": "true",
            "aria-label": "Code editor",
            tabindex: "-1",
            style: "position: absolute; left: -9999px; width: 1px; height: 1px; overflow: hidden; clip: rect(0,0,0,0); white-space: nowrap;",

            {
                let buf = buffer.read().clone();
                let start = visible_start.read();
                let end = visible_end.read();
                let cur_line = cursor_line.read();
                let cur_col = cursor_col.read();

                // Generate lines for visible range
                let lines: Vec<_> = (*start..*end)
                    .filter_map(|line_idx| {
                        buf.line(line_idx).map(|line_text| (line_idx, line_text.to_string()))
                    })
                    .collect();

                rsx! {
                    div {
                        "aria-live": "polite",
                        "aria-atomic": "false",

                        for (line_idx , line_text) in lines {
                            {
                                let is_cursor_line = line_idx == *cur_line;
                                let aria_label = if is_cursor_line {
                                    format!("Line {}, cursor at column {}", line_idx + 1, cur_col + 1)
                                } else {
                                    format!("Line {}", line_idx + 1)
                                };
                                let text_content = line_text.trim_end_matches('\n').to_string();
                                let aria_current = if is_cursor_line { "location" } else { "false" };

                                rsx! {
                                    div {
                                        role: "textbox",
                                        "aria-label": "{aria_label}",
                                        "aria-current": "{aria_current}",
                                        "{text_content}"
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_accessibility_layer_exists() {
        // This is a placeholder test to ensure the module compiles
        // Real testing would require WASM environment with DOM
        assert!(true);
    }
}
