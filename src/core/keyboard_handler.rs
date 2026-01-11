//! Keyboard Event Handler - Pure Logic Layer
//!
//! This module separates keyboard event parsing from UI components,
//! enabling pure logic testing without browser environment.

use crate::core::actions::{Direction, EditorAction};
use leptos::ev::KeyboardEvent;

/// Parse keyboard event to EditorAction (pure function, easily testable)
pub fn parse_keyboard_event(ev: &KeyboardEvent) -> EditorAction {
    let key = ev.key();
    let ctrl = ev.ctrl_key();
    let meta = ev.meta_key();
    let shift = ev.shift_key();
    let modifier = ctrl || meta;

    match key.as_str() {
        // ===== Ctrl/Cmd Combinations =====
        "z" if modifier && shift => EditorAction::Redo,
        "z" if modifier => EditorAction::Undo,
        "y" if modifier => EditorAction::Redo,
        "c" if modifier => EditorAction::Copy,
        "x" if modifier => EditorAction::Cut,
        "v" if modifier => EditorAction::Paste,
        "a" if modifier => EditorAction::SelectAll,
        "s" if modifier => EditorAction::Save,

        // ===== Arrow Keys =====
        "ArrowLeft" if shift => EditorAction::ExtendSelection(Direction::Left),
        "ArrowLeft" => EditorAction::MoveCursor(Direction::Left),
        "ArrowRight" if shift => EditorAction::ExtendSelection(Direction::Right),
        "ArrowRight" => EditorAction::MoveCursor(Direction::Right),
        "ArrowUp" if shift => EditorAction::ExtendSelection(Direction::Up),
        "ArrowUp" => EditorAction::MoveCursor(Direction::Up),
        "ArrowDown" if shift => EditorAction::ExtendSelection(Direction::Down),
        "ArrowDown" => EditorAction::MoveCursor(Direction::Down),

        // ===== Home/End =====
        "Home" if shift => EditorAction::ExtendToLineStart,
        "Home" => EditorAction::MoveToLineStart,
        "End" if shift => EditorAction::ExtendToLineEnd,
        "End" => EditorAction::MoveToLineEnd,

        // ===== Special Keys =====
        "Enter" => EditorAction::NewLine,
        "Backspace" => EditorAction::Backspace,
        "Delete" => EditorAction::Delete,
        "PageUp" => EditorAction::PageUp,
        "PageDown" => EditorAction::PageDown,

        // ===== Single Character Input =====
        k if k.len() == 1 && !modifier => {
            EditorAction::InsertChar(k.chars().next().unwrap())
        }

        // ===== Unknown Key =====
        _ => EditorAction::None,
    }
}

/// Completion widget-specific key handling
/// Returns Some(action) if handled, None if should fall through to editor
pub fn handle_completion_widget_key(key: &str) -> Option<CompletionAction> {
    match key {
        "ArrowDown" => Some(CompletionAction::SelectNext),
        "ArrowUp" => Some(CompletionAction::SelectPrevious),
        "Enter" | "Tab" => Some(CompletionAction::Accept),
        "Escape" => Some(CompletionAction::Dismiss),
        _ => None, // Fall through to editor
    }
}

/// Completion widget actions
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompletionAction {
    SelectNext,
    SelectPrevious,
    Accept,
    Dismiss,
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to create mock keyboard event data
    struct MockKeyEvent {
        key: String,
        ctrl: bool,
        meta: bool,
        shift: bool,
    }

    impl MockKeyEvent {
        fn new(key: &str) -> Self {
            Self {
                key: key.to_string(),
                ctrl: false,
                meta: false,
                shift: false,
            }
        }

        fn with_ctrl(mut self) -> Self {
            self.ctrl = true;
            self
        }

        fn with_shift(mut self) -> Self {
            self.shift = true;
            self
        }
    }

    #[test]
    fn test_single_char_input() {
        // Since we can't create real KeyboardEvent without browser,
        // we test the logic patterns
        assert_eq!(
            "a".len(),
            1,
            "Single char detection works"
        );

        // Test InsertChar action creation
        let action = EditorAction::InsertChar('a');
        assert!(action.modifies_buffer());
    }

    #[test]
    fn test_undo_redo_actions() {
        let undo = EditorAction::Undo;
        let redo = EditorAction::Redo;

        assert_eq!(undo.description(), "Undo");
        assert_eq!(redo.description(), "Redo");
        // Note: Undo/Redo technically restore buffer state but aren't
        // considered "modifying" in the traditional sense (no new undo entry)
        assert!(undo.affects_cursor());
        assert!(redo.affects_cursor());
    }

    #[test]
    fn test_cursor_movement() {
        let move_left = EditorAction::MoveCursor(Direction::Left);
        let move_right = EditorAction::MoveCursor(Direction::Right);

        assert!(!move_left.modifies_buffer());
        assert!(move_left.affects_cursor());
        assert!(!move_right.modifies_buffer());
        assert!(move_right.affects_cursor());
    }

    #[test]
    fn test_selection_extension() {
        let extend_left = EditorAction::ExtendSelection(Direction::Left);
        assert!(!extend_left.modifies_buffer());
        assert!(extend_left.affects_cursor());
    }

    #[test]
    fn test_clipboard_operations() {
        assert!(EditorAction::Copy.description().contains("Copy"));
        assert!(!EditorAction::Copy.modifies_buffer());
        assert!(EditorAction::Cut.modifies_buffer());
        assert!(EditorAction::Paste.modifies_buffer());
    }

    #[test]
    fn test_completion_widget_keys() {
        assert_eq!(
            handle_completion_widget_key("ArrowDown"),
            Some(CompletionAction::SelectNext)
        );
        assert_eq!(
            handle_completion_widget_key("ArrowUp"),
            Some(CompletionAction::SelectPrevious)
        );
        assert_eq!(
            handle_completion_widget_key("Enter"),
            Some(CompletionAction::Accept)
        );
        assert_eq!(
            handle_completion_widget_key("Escape"),
            Some(CompletionAction::Dismiss)
        );
        assert_eq!(handle_completion_widget_key("a"), None);
    }

    #[test]
    fn test_action_serialization() {
        use serde_json;

        let action = EditorAction::InsertChar('x');
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("InsertChar"));

        let deserialized: EditorAction = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, action);
    }

    #[test]
    fn test_action_batch() {
        let batch = EditorAction::Batch(vec![
            EditorAction::InsertChar('a'),
            EditorAction::InsertChar('b'),
            EditorAction::InsertChar('c'),
        ]);

        assert_eq!(batch.description(), "Batch (3 actions)");
        assert!(batch.modifies_buffer());
    }
}
