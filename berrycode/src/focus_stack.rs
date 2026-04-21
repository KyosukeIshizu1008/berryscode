//! Global Focus Stack Management
//!
//! Prevents keyboard event conflicts between multiple UI layers
//! (Editor, Command Palette, Dialogs, etc.).
//!
//! In the egui-based UI, focus is managed by egui's built-in focus system.
//! This module provides a lightweight compatibility layer so that code
//! referencing `FocusLayer` or `FocusStack` continues to compile. The
//! `should_handle_keys` method always returns `true` because egui routes
//! keyboard events to the focused widget automatically.

/// UI layers that can receive keyboard focus.
///
/// Ordered by priority: higher-valued layers take precedence when
/// multiple layers are active (e.g., a dialog over the editor).
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

/// Compatibility shim for legacy focus-management code.
///
/// The egui backend handles focus natively, so this struct is intentionally
/// minimal. All keyboard queries return `true`, deferring actual routing
/// to egui's widget focus system.
#[derive(Debug, Clone, Copy)]
pub struct FocusStack;

impl FocusStack {
    pub fn new() -> Self {
        FocusStack
    }

    /// Returns whether the given layer should process keyboard events.
    ///
    /// Always returns `true` in the egui backend; focus arbitration is
    /// handled by egui's own focus tracking.
    pub fn should_handle_keys(&self, _layer: FocusLayer) -> bool {
        true
    }
}

impl Default for FocusStack {
    fn default() -> Self {
        Self::new()
    }
}
