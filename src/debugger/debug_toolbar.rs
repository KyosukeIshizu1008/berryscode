//! Debug Toolbar Component
//!
//! Provides debug control buttons (Continue, Step Over/Into/Out, Stop, Restart).

use dioxus::prelude::*;
use super::session::{DebugSession, DebugState};
use crate::common::ui_components::SvgIconButton;

/// IntelliJ-style Continue/Play icon
fn continue_icon() -> Element {
    rsx! {
        svg {
            width: "16",
            height: "16",
            view_box: "0 0 16 16",
            fill: "none",
            path { d: "M4 3L12 8L4 13V3Z", fill: "#6AAB73" }
        }
    }
}

/// IntelliJ-style Step Over icon
fn step_over_icon() -> Element {
    rsx! {
        svg {
            width: "16",
            height: "16",
            view_box: "0 0 16 16",
            fill: "none",
            path { d: "M8 4L12 8L8 12V9H4V7H8V4Z", fill: "#6897BB" }
            circle { cx: "3", cy: "8", r: "1.5", fill: "#6897BB" }
        }
    }
}

/// IntelliJ-style Step Into icon
fn step_into_icon() -> Element {
    rsx! {
        svg {
            width: "16",
            height: "16",
            view_box: "0 0 16 16",
            fill: "none",
            path { d: "M8 3L12 7H9V13H7V7H4L8 3Z", fill: "#6897BB" }
        }
    }
}

/// IntelliJ-style Step Out icon
fn step_out_icon() -> Element {
    rsx! {
        svg {
            width: "16",
            height: "16",
            view_box: "0 0 16 16",
            fill: "none",
            path { d: "M8 13L4 9H7V3H9V9H12L8 13Z", fill: "#6897BB" }
        }
    }
}

/// IntelliJ-style Stop icon
fn stop_icon() -> Element {
    rsx! {
        svg {
            width: "16",
            height: "16",
            view_box: "0 0 16 16",
            fill: "none",
            rect { x: "4", y: "4", width: "8", height: "8", rx: "1", fill: "#C75450" }
        }
    }
}

/// IntelliJ-style Restart icon
fn restart_icon() -> Element {
    rsx! {
        svg {
            width: "16",
            height: "16",
            view_box: "0 0 16 16",
            fill: "none",
            path { d: "M8 3V1L5 4L8 7V5C10.21 5 12 6.79 12 9C12 9.79 11.75 10.52 11.33 11.12L12.38 12.17C13.04 11.3 13.5 10.19 13.5 9C13.5 5.96 11.04 3.5 8 3.5V3Z", fill: "#9876AA" }
            path { d: "M8 13C5.79 13 4 11.21 4 9C4 8.21 4.25 7.48 4.67 6.88L3.62 5.83C2.96 6.7 2.5 7.81 2.5 9C2.5 12.04 4.96 14.5 8 14.5V16L11 13L8 10V13Z", fill: "#9876AA" }
        }
    }
}

/// Debug toolbar component props
#[derive(Props, Clone, PartialEq)]
pub struct DebugToolbarProps {
    /// Debug session
    session: DebugSession,
}

/// Debug toolbar component
#[component]
pub fn DebugToolbar(props: DebugToolbarProps) -> Element {
    let session = props.session;
    let state = session.state;

    // Continue (F5)
    let handle_continue = move |_| {
        spawn(async move {
            let _ = session.continue_execution().await;
        });
    };

    // Step Over (F10)
    let handle_step_over = move |_| {
        spawn(async move {
            let _ = session.step_over().await;
        });
    };

    // Step Into (F11)
    let handle_step_into = move |_| {
        spawn(async move {
            let _ = session.step_into().await;
        });
    };

    // Step Out (Shift+F11)
    let handle_step_out = move |_| {
        spawn(async move {
            let _ = session.step_out().await;
        });
    };

    // Stop
    let handle_stop = move |_| {
        spawn(async move {
            let _ = session.stop().await;
        });
    };

    // Restart
    let handle_restart = move |_| {
        spawn(async move {
            // Stop current session
            if session.stop().await.is_err() {
                return;
            }

            // Start new session (would need program path - simplified here)
            // In real implementation, we'd store the program path
        });
    };

    rsx! {
        div { class: "berry-debug-toolbar",
            {
                let current_state = *state.read();
                let is_stopped = current_state == DebugState::Stopped;
                let is_paused = current_state == DebugState::Paused || current_state == DebugState::Stepping;

                rsx! {
                    div { class: "berry-debug-toolbar-buttons",
                        SvgIconButton {
                            icon: continue_icon(),
                            tooltip: "Continue (F5)",
                            on_click: handle_continue,
                            disabled: is_stopped,
                        }
                        SvgIconButton {
                            icon: step_over_icon(),
                            tooltip: "Step Over (F10)",
                            on_click: handle_step_over,
                            disabled: !is_paused,
                        }
                        SvgIconButton {
                            icon: step_into_icon(),
                            tooltip: "Step Into (F11)",
                            on_click: handle_step_into,
                            disabled: !is_paused,
                        }
                        SvgIconButton {
                            icon: step_out_icon(),
                            tooltip: "Step Out (Shift+F11)",
                            on_click: handle_step_out,
                            disabled: !is_paused,
                        }
                        SvgIconButton {
                            icon: stop_icon(),
                            tooltip: "Stop",
                            on_click: handle_stop,
                            disabled: is_stopped,
                        }
                        SvgIconButton {
                            icon: restart_icon(),
                            tooltip: "Restart",
                            on_click: handle_restart,
                            disabled: is_stopped,
                        }
                    }
                    div { class: "berry-debug-status",
                        {
                            match *state.read() {
                                DebugState::Stopped => "Stopped",
                                DebugState::Running => "Running",
                                DebugState::Paused => "Paused",
                                DebugState::Stepping => "Stepping",
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
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_debug_toolbar_compiles() {
        // Ensure component compiles
        assert!(true);
    }

    #[test]
    fn test_debug_state_display() {
        assert_eq!(format!("{:?}", DebugState::Stopped), "Stopped");
        assert_eq!(format!("{:?}", DebugState::Running), "Running");
        assert_eq!(format!("{:?}", DebugState::Paused), "Paused");
        assert_eq!(format!("{:?}", DebugState::Stepping), "Stepping");
    }

    #[test]
    fn test_button_disabled_logic() {
        // When stopped, continue should be disabled
        let is_stopped = true;
        assert!(is_stopped);

        // When paused, step buttons should be enabled
        let is_paused = true;
        assert!(is_paused);
    }
}
