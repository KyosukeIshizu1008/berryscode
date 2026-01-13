//! Editor Panel Component
//! 100% Canvas + 100% Rust Architecture

use dioxus::prelude::*;
use crate::core::virtual_editor::VirtualEditorPanel;

/// Editor Panel props
#[derive(Props, Clone, PartialEq)]
pub struct EditorPanelProps {
    selected_file: Signal<Option<(String, String)>>,
}

#[component]
pub fn EditorPanel(props: EditorPanelProps) -> Element {
    let selected_file = props.selected_file;

    // ✅ Canvas Architecture: Use VirtualEditorPanel directly
    // VirtualEditorPanel handles:
    // - Canvas rendering
    // - Text buffer management
    // - Cursor positioning
    // - Mouse/keyboard events
    // - IME support
    // - Undo/Redo
    rsx! {
        VirtualEditorPanel { selected_file: selected_file }
    }
}
