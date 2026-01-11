//! Accessibility Layer for Screen Readers
//!
//! Since the editor uses 100% Canvas rendering, screen readers cannot read text.
//! This module provides a hidden DOM mirror that synchronizes with the Canvas content.

use leptos::prelude::*;
use crate::buffer::TextBuffer;

/// Render accessibility layer for screen readers
/// This creates a hidden DOM representation of visible text
#[component]
pub fn AccessibilityLayer(
    buffer: Signal<TextBuffer>,
    cursor_line: Signal<usize>,
    cursor_col: Signal<usize>,
    visible_start: Signal<usize>,
    visible_end: Signal<usize>,
) -> impl IntoView {
    view! {
        <div
            class="berry-editor-a11y"
            role="textbox"
            aria-multiline="true"
            aria-label="Code editor"
            tabindex="-1"
            style="position: absolute; left: -9999px; width: 1px; height: 1px; overflow: hidden; clip: rect(0,0,0,0); white-space: nowrap;"
        >
            {move || {
                let buf = buffer.get();
                let start = visible_start.get();
                let end = visible_end.get();
                let cur_line = cursor_line.get();
                let cur_col = cursor_col.get();

                // Generate lines for visible range
                let lines: Vec<_> = (start..end)
                    .filter_map(|line_idx| {
                        buf.line(line_idx).map(|line_text| (line_idx, line_text))
                    })
                    .collect();

                view! {
                    <div aria-live="polite" aria-atomic="false">
                        {lines.into_iter().map(|(line_idx, line_text)| {
                            let is_cursor_line = line_idx == cur_line;
                            let aria_label = if is_cursor_line {
                                format!("Line {}, cursor at column {}", line_idx + 1, cur_col + 1)
                            } else {
                                format!("Line {}", line_idx + 1)
                            };

                            // Convert to owned string to avoid lifetime issues
                            let text_content = line_text.trim_end_matches('\n').to_string();

                            view! {
                                <div
                                    role="textbox"
                                    aria-label=aria_label
                                    aria-current=if is_cursor_line { "location" } else { "false" }
                                >
                                    {text_content}
                                </div>
                            }
                        }).collect_view()}
                    </div>
                }
            }}
        </div>
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
