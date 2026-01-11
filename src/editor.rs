//! Editor Panel Component
//! 100% Canvas + 100% Rust Architecture

use leptos::prelude::*;
use crate::core::virtual_editor::VirtualEditorPanel;

#[component]
pub fn EditorPanel(
    selected_file: RwSignal<Option<(String, String)>>,
) -> impl IntoView {
    // ✅ Canvas Architecture: Use VirtualEditorPanel directly
    // VirtualEditorPanel handles:
    // - Canvas rendering
    // - Text buffer management
    // - Cursor positioning
    // - Mouse/keyboard events
    // - IME support
    // - Undo/Redo
    view! {
        <VirtualEditorPanel selected_file=selected_file />
    }
}
