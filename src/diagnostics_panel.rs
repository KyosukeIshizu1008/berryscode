//! Diagnostics Panel
//!
//! Displays errors, warnings, and information from LSP.

use leptos::prelude::*;
use crate::lsp_ui::Diagnostic;
use crate::common::ui_components::Panel;

/// Diagnostics panel component
#[component]
pub fn DiagnosticsPanel(
    /// Diagnostics to display
    diagnostics: RwSignal<Vec<Diagnostic>>,
    /// Callback when a diagnostic is clicked (to jump to location)
    on_click: impl Fn(u32, u32) + 'static + Clone + Send,
) -> impl IntoView {
    view! {
        <Panel title="Problems">
            <div class="berry-diagnostics-list">
                {move || {
                    let diags = diagnostics.get();

                    if diags.is_empty() {
                        view! { <></> }.into_any()
                    } else {
                        diags.iter().map(|diagnostic| {
                            let on_click_clone = on_click.clone();
                            let line = diagnostic.range.start.line;
                            let character = diagnostic.range.start.character;

                            view! {
                                <DiagnosticItem
                                    diagnostic=diagnostic.clone()
                                    on_click=move || on_click_clone(line, character)
                                />
                            }
                        }).collect::<Vec<_>>().into_any()
                    }
                }}
            </div>
        </Panel>
    }
}

/// Single diagnostic item
#[component]
fn DiagnosticItem(
    /// The diagnostic
    diagnostic: Diagnostic,
    /// Click handler
    on_click: impl Fn() + 'static,
) -> impl IntoView {
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

    view! {
        <div
            class=class
            on:click=move |_| on_click()
        >
            <span class="berry-diagnostic-icon">{severity_icon}</span>
            <span class="berry-diagnostic-message">{message}</span>
            <span class="berry-diagnostic-location">
                {location}
            </span>
            {source_text.map(|source| {
                view! {
                    <span class="berry-diagnostic-source">{source}</span>
                }
            })}
        </div>
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
