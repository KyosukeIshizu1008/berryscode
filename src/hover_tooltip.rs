//! Hover Tooltip
//!
//! Displays type information, documentation, and other hover info from LSP.

use dioxus::prelude::*;
use crate::lsp_ui::HoverInfo;
use crate::types::Position;

/// Hover tooltip component properties
#[derive(Props, Clone, PartialEq)]
pub struct HoverTooltipProps {
    /// Hover information to display
    hover_info: Signal<Option<HoverInfo>>,
    /// Position to show the tooltip (in pixels)
    position: Signal<Option<(f64, f64)>>,
}

/// Hover tooltip component
#[component]
pub fn HoverTooltip(props: HoverTooltipProps) -> Element {
    let hover_info = props.hover_info;
    let position = props.position;

    rsx! {
        {
            let info = hover_info.read().clone();
            let pos = position.read().clone();

            if let (Some(hover), Some((x, y))) = (info, pos) {
                let style = format!(
                    "position: absolute; left: {}px; top: {}px; z-index: 2000;",
                    x, y + 20.0 // Offset below cursor
                );

                rsx! {
                    div { class: "berry-hover-tooltip", style: "{style}",
                        div { class: "berry-hover-content",
                            {format_hover_contents(&hover.contents)}
                        }
                    }
                }
            } else {
                rsx! {}
            }
        }
    }
}

/// Format hover contents for display
fn format_hover_contents(contents: &str) -> Element {
    // Split by code blocks and regular text
    let parts: Vec<String> = contents.split("```").map(|s| s.to_string()).collect();

    rsx! {
        {parts.into_iter().enumerate().map(|(idx, part)| {
            if part.is_empty() {
                return rsx! {};
            }

            if idx % 2 == 0 {
                // Regular text
                let lines: Vec<String> = part.lines().map(|s| s.to_string()).collect();
                rsx! {
                    {lines.into_iter().map(|line| {
                        if !line.trim().is_empty() {
                            rsx! {
                                div { class: "berry-hover-text", "{line}" }
                            }
                        } else {
                            rsx! {}
                        }
                    })}
                }
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

                rsx! {
                    div { class: "berry-hover-code",
                        pre { "{code_text}" }
                    }
                }
            }
        })}
    }
}

/// Simple hover tooltip properties
#[derive(Props, Clone, PartialEq)]
pub struct SimpleHoverTooltipProps {
    /// Text to display
    text: String,
    /// Position in pixels
    position: (f64, f64),
}

/// Simple hover tooltip without markdown parsing
#[component]
pub fn SimpleHoverTooltip(props: SimpleHoverTooltipProps) -> Element {
    let (x, y) = props.position;
    let style = format!(
        "position: absolute; left: {}px; top: {}px; z-index: 2000;",
        x, y + 20.0
    );

    rsx! {
        div { class: "berry-hover-tooltip", style: "{style}",
            div { class: "berry-hover-content",
                div { class: "berry-hover-text", "{props.text}" }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_hover_tooltip_compile() {
        // Ensure component compiles
        assert!(true);
    }
}
