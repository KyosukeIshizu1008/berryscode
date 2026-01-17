//! Global Focus Stack Management
//!
//! Prevents keyboard event conflicts between multiple UI layers
//! (Editor, Command Palette, Dialogs, etc.)
//!
//! egui version: Uses simple enum instead of Dioxus Signal

/// UI layers that can receive keyboard focus
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FocusLayer {
    Editor = 0,
    CommandPalette = 1,
    CompletionWidget = 2,
    Dialog = 3,
}

impl Default for FocusLayer {
    fn default() -> Self {
        FocusLayer::Editor
    }
}

/// Stub FocusStack for compatibility with legacy code
/// (Not used by egui_app.rs)
#[derive(Debug, Clone, Copy)]
pub struct FocusStack;

impl FocusStack {
    pub fn new() -> Self {
        FocusStack
    }

    pub fn should_handle_keys(&self, _layer: FocusLayer) -> bool {
        true // Stub implementation
    }
}

impl Default for FocusStack {
    fn default() -> Self {
        Self::new()
    }
}
