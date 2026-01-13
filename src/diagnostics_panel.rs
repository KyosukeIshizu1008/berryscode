//! Diagnostics Panel
//!
//! Displays errors, warnings, and information from LSP.

use dioxus::prelude::*;
use crate::lsp_ui::Diagnostic;
use crate::common::ui_components::Panel;

/// Diagnostics panel props
#[derive(Props, Clone, PartialEq)]
pub struct DiagnosticsPanelProps {
    /// Diagnostics to display
    diagnostics: Signal<Vec<Diagnostic>>,
    /// Callback when a diagnostic is clicked (to jump to location)
    on_click: EventHandler<(u32, u32)>,
}

/// Diagnostics panel component
#[component]
pub fn DiagnosticsPanel(props: DiagnosticsPanelProps) -> Element {
    let diagnostics = props.diagnostics;
    let on_click = props.on_click;

    rsx! {
        Panel { title: "Problems",
            div { class: "berry-diagnostics-list",
                {
                    let diags = diagnostics.read().clone();

                    if diags.is_empty() {
                        rsx! {}
                    } else {
                        rsx! {
                            for diagnostic in diags {
                                {
                                    let line = diagnostic.range.start.line;
                                    let character = diagnostic.range.start.character;

                                    rsx! {
                                        DiagnosticItem {
                                            diagnostic: diagnostic.clone(),
                                            on_click: move |_| on_click.call((line, character))
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

/// Diagnostic item props
#[derive(Props, Clone, PartialEq)]
struct DiagnosticItemProps {
    /// The diagnostic
    diagnostic: Diagnostic,
    /// Click handler
    on_click: EventHandler<()>,
}

/// Single diagnostic item
#[component]
fn DiagnosticItem(props: DiagnosticItemProps) -> Element {
    let diagnostic = props.diagnostic;
    let on_click = props.on_click;

    // Severity: 1=Error, 2=Warning, 3=Info, 4=Hint
    let (severity_class, severity_icon) = match diagnostic.severity {
        1 => ("error", "E"),
        2 => ("warning", "W"),
        3 => ("info", "I"),
        _ => ("hint", "H"),
    };

    let class = format!("berry-diagnostic berry-diagnostic-{}", severity_class);
    let message = diagnostic.message.clone();
    let location = format!("[{}:{}]", diagnostic.range.start.line + 1, diagnostic.range.start.character + 1);
    let source_text = diagnostic.source.clone();

    rsx! {
        div {
            class: "{class}",
            onclick: move |_| on_click.call(()),

            span { class: "berry-diagnostic-icon", "{severity_icon}" }
            span { class: "berry-diagnostic-message", "{message}" }
            span { class: "berry-diagnostic-location", "{location}" }

            if let Some(source) = source_text {
                span { class: "berry-diagnostic-source", "{source}" }
            }
        }
    }
}

/// Group diagnostics by severity for summary
pub fn diagnostics_summary(diagnostics: &[Diagnostic]) -> (usize, usize, usize) {
    let errors = diagnostics.iter().filter(|d| d.severity == 1).count();
    let warnings = diagnostics.iter().filter(|d| d.severity == 2).count();
    let info = diagnostics.iter().filter(|d| d.severity >= 3).count();

    (errors, warnings, info)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lsp_ui::{DiagnosticPosition, DiagnosticRange};

    #[test]
    fn test_diagnostics_summary() {
        let diagnostics = vec![
            Diagnostic {
                range: DiagnosticRange {
                    start: DiagnosticPosition { line: 0, character: 0 },
                    end: DiagnosticPosition { line: 0, character: 5 },
                },
                severity: 1, // Error
                message: "Error 1".to_string(),
                source: None,
            },
            Diagnostic {
                range: DiagnosticRange {
                    start: DiagnosticPosition { line: 1, character: 0 },
                    end: DiagnosticPosition { line: 1, character: 5 },
                },
                severity: 2, // Warning
                message: "Warning 1".to_string(),
                source: None,
            },
            Diagnostic {
                range: DiagnosticRange {
                    start: DiagnosticPosition { line: 2, character: 0 },
                    end: DiagnosticPosition { line: 2, character: 5 },
                },
                severity: 3, // Info
                message: "Info 1".to_string(),
                source: None,
            },
        ];

        let (errors, warnings, info) = diagnostics_summary(&diagnostics);
        assert_eq!(errors, 1);
        assert_eq!(warnings, 1);
        assert_eq!(info, 1);
    }

    #[test]
    fn test_empty_diagnostics() {
        let (errors, warnings, info) = diagnostics_summary(&[]);
        assert_eq!(errors, 0);
        assert_eq!(warnings, 0);
        assert_eq!(info, 0);
    }
}
