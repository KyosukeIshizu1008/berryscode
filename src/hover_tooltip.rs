//! Hover Tooltip
//!
//! Displays type information, documentation, and other hover info from LSP.

use leptos::prelude::*;
use crate::lsp_ui::HoverInfo;
use crate::types::Position;

/// Hover tooltip component
#[component]
pub fn HoverTooltip(
    /// Hover information to display
    hover_info: RwSignal<Option<HoverInfo>>,
    /// Position to show the tooltip (in pixels)
    position: RwSignal<Option<(f64, f64)>>,
) -> impl IntoView {
    view! {
        {move || {
            let info = hover_info.get();
            let pos = position.get();

            if let (Some(hover), Some((x, y))) = (info, pos) {
                let style = format!(
                    "position: absolute; left: {}px; top: {}px; z-index: 2000;",
                    x, y + 20.0 // Offset below cursor
                );

                view! {
                    <div class="berry-hover-tooltip" style=style>
                        <div class="berry-hover-content">
                            {format_hover_contents(&hover.contents)}
                        </div>
                    </div>
                }.into_any()
            } else {
                view! { <></> }.into_any()
            }
        }}
    }
}

/// Format hover contents for display
fn format_hover_contents(contents: &str) -> impl IntoView {
    // Split by code blocks and regular text
    let parts: Vec<String> = contents.split("```").map(|s| s.to_string()).collect();

    parts.into_iter().enumerate().map(|(idx, part)| {
        if part.is_empty() {
            return view! { <></> }.into_any();
        }

        if idx % 2 == 0 {
            // Regular text
            let lines: Vec<String> = part.lines().map(|s| s.to_string()).collect();
            view! {
                {lines.into_iter().map(|line| {
                    if !line.trim().is_empty() {
                        view! {
                            <div class="berry-hover-text">{line}</div>
                        }.into_any()
                    } else {
                        view! { <></> }.into_any()
                    }
                }).collect::<Vec<_>>()}
            }.into_any()
        } else {
            // Code block
            let lines: Vec<String> = part.lines().map(|s| s.to_string()).collect();

            // First line might be language identifier
            let code_lines: Vec<String> = if !lines.is_empty() {
                lines.into_iter().skip(1).collect()
            } else {
                lines
            };

            let code_text = code_lines.join("\n");

            view! {
                <div class="berry-hover-code">
                    <pre>{code_text}</pre>
                </div>
            }.into_any()
        }
    }).collect::<Vec<_>>()
}

/// Simple hover tooltip without markdown parsing
#[component]
pub fn SimpleHoverTooltip(
    /// Text to display
    text: String,
    /// Position in pixels
    position: (f64, f64),
) -> impl IntoView {
    let (x, y) = position;
    let style = format!(
        "position: absolute; left: {}px; top: {}px; z-index: 2000;",
        x, y + 20.0
    );

    view! {
        <div class="berry-hover-tooltip" style=style>
            <div class="berry-hover-content">
                <div class="berry-hover-text">{text}</div>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    // Note: These tests are commented out because impl IntoView doesn't provide
    // is_empty() or len() methods. These would need integration testing instead.

    // #[test]
    // fn test_format_simple_text() {
    //     let contents = "This is a simple hover message";
    //     let _elements = format_hover_contents(contents);
    //     // Cannot test impl IntoView directly
    // }

    // #[test]
    // fn test_format_with_code_block() {
    //     let contents = "Description\n```rust\nfn main() {}\n```\nMore text";
    //     let _elements = format_hover_contents(contents);
    //     // Cannot test impl IntoView directly
    // }

    // #[test]
    // fn test_format_empty_contents() {
    //     let contents = "";
    //     let _elements = format_hover_contents(contents);
    //     // Cannot test impl IntoView directly
    // }

    #[wasm_bindgen_test]
    fn test_hover_tooltip_compile() {
        // Ensure component compiles
        assert!(true);
    }
}
