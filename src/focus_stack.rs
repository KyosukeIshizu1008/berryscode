//! Global Focus Stack Management
//!
//! Prevents keyboard event conflicts between multiple UI layers
//! (Editor, Command Palette, Dialogs, etc.)

use dioxus::prelude::*;

/// UI layers that can receive keyboard focus
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FocusLayer {
    Editor = 0,
    CommandPalette = 1,
    CompletionWidget = 2,
    Dialog = 3,
}

/// Global focus stack for managing keyboard event routing
#[derive(Clone, Copy)]
pub struct FocusStack {
    active_layer: Signal<FocusLayer>,
}

impl FocusStack {
    /// Create a new focus stack with Editor as the default active layer
    pub fn new() -> Self {
        Self {
            active_layer: Signal::new(FocusLayer::Editor),
        }
    }

    /// Check if the given layer should handle keyboard events
    pub fn should_handle_keys(&self, layer: FocusLayer) -> bool {
        *self.active_layer.read() == layer
    }

    /// Set the active layer (e.g., when opening Command Palette)
    pub fn set_active(&self, layer: FocusLayer) {
        #[cfg(debug_assertions)]
        tracing::debug!("🎯 FocusStack: Switching to {:?}", layer);

        *self.active_layer.write() = layer;
    }

    /// Get the current active layer
    pub fn get_active(&self) -> FocusLayer {
        *self.active_layer.read()
    }

    /// Push a layer onto the stack (set as active)
    pub fn push(&self, layer: FocusLayer) {
        self.set_active(layer);
    }

    /// Pop back to the previous layer (defaults to Editor)
    pub fn pop(&self) {
        // For now, always return to Editor
        // In future, implement a real stack if needed
        self.set_active(FocusLayer::Editor);
    }
}

impl Default for FocusStack {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_focus_stack_creation() {
        let stack = FocusStack::new();
        assert_eq!(stack.get_active(), FocusLayer::Editor);
    }

    #[test]
    fn test_should_handle_keys() {
        let stack = FocusStack::new();

        // Initially, only Editor should handle keys
        assert!(stack.should_handle_keys(FocusLayer::Editor));
        assert!(!stack.should_handle_keys(FocusLayer::CommandPalette));

        // Switch to CommandPalette
        stack.set_active(FocusLayer::CommandPalette);
        assert!(!stack.should_handle_keys(FocusLayer::Editor));
        assert!(stack.should_handle_keys(FocusLayer::CommandPalette));
    }

    #[test]
    fn test_push_pop() {
        let stack = FocusStack::new();

        stack.push(FocusLayer::Dialog);
        assert_eq!(stack.get_active(), FocusLayer::Dialog);

        stack.pop();
        assert_eq!(stack.get_active(), FocusLayer::Editor);
    }

    #[test]
    fn test_layer_ordering() {
        // Verify layer priorities are correct
        assert!(FocusLayer::Editor < FocusLayer::CommandPalette);
        assert!(FocusLayer::CommandPalette < FocusLayer::CompletionWidget);
        assert!(FocusLayer::CompletionWidget < FocusLayer::Dialog);
    }
}
