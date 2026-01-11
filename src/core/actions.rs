//! Editor Actions - Command Pattern Implementation
//!
//! This module defines all possible editor actions as a strongly-typed enum,
//! enabling better testing, undo/redo management, and macro recording.

use serde::{Deserialize, Serialize};

/// Direction for cursor movement
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

/// All possible editor actions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EditorAction {
    // ===== Text Input =====
    /// Insert a single character at cursor position
    InsertChar(char),
    /// Insert a string (for paste, IME, etc.)
    InsertText(String),
    /// Insert newline and handle auto-indent
    NewLine,
    /// Delete character before cursor (or selection)
    Backspace,
    /// Delete character after cursor (or selection)
    Delete,

    // ===== Cursor Movement =====
    /// Move cursor in specified direction
    MoveCursor(Direction),
    /// Move cursor to start of current line
    MoveToLineStart,
    /// Move cursor to end of current line
    MoveToLineEnd,
    /// Move cursor to specific line and column
    MoveToPosition { line: usize, col: usize },
    /// Scroll viewport up by page
    PageUp,
    /// Scroll viewport down by page
    PageDown,

    // ===== Selection =====
    /// Extend selection in specified direction
    ExtendSelection(Direction),
    /// Extend selection to line start
    ExtendToLineStart,
    /// Extend selection to line end
    ExtendToLineEnd,
    /// Select all text
    SelectAll,
    /// Clear selection
    ClearSelection,
    /// Set selection range
    SetSelection {
        start_line: usize,
        start_col: usize,
        end_line: usize,
        end_col: usize,
    },

    // ===== Clipboard Operations =====
    /// Copy selection to clipboard (or current line if no selection)
    Copy,
    /// Cut selection to clipboard (or current line if no selection)
    Cut,
    /// Paste from clipboard
    Paste,

    // ===== Undo/Redo =====
    /// Undo last action
    Undo,
    /// Redo previously undone action
    Redo,

    // ===== File Operations =====
    /// Save current file
    Save,
    /// Save as (with new path)
    SaveAs(String),
    /// Close current tab
    Close,

    // ===== Formatting & Editing =====
    /// Format entire document
    Format,
    /// Comment/uncomment selected lines
    ToggleComment,
    /// Indent selected lines or current line
    Indent,
    /// Dedent selected lines or current line
    Dedent,

    // ===== LSP Integration =====
    /// Go to definition of symbol at cursor
    GotoDefinition,
    /// Trigger code completion
    TriggerCompletion,
    /// Show hover information
    ShowHover,
    /// Find references
    FindReferences,
    /// Rename symbol
    Rename(String),

    // ===== Search & Navigation =====
    /// Open command palette
    OpenCommandPalette,
    /// Go to specific line
    GotoLine(usize),
    /// Find text in current file
    Find(String),
    /// Replace text
    Replace { find: String, replace: String },

    // ===== View Control =====
    /// Toggle sidebar visibility
    ToggleSidebar,
    /// Split editor horizontally
    SplitHorizontal,
    /// Split editor vertically
    SplitVertical,

    // ===== Special =====
    /// Batch multiple actions (for macros, complex operations)
    Batch(Vec<EditorAction>),
    /// No-op action
    None,
}

impl EditorAction {
    /// Check if this action modifies the buffer content
    pub fn modifies_buffer(&self) -> bool {
        matches!(
            self,
            EditorAction::InsertChar(_)
                | EditorAction::InsertText(_)
                | EditorAction::NewLine
                | EditorAction::Backspace
                | EditorAction::Delete
                | EditorAction::Cut
                | EditorAction::Paste
                | EditorAction::Format
                | EditorAction::ToggleComment
                | EditorAction::Indent
                | EditorAction::Dedent
                | EditorAction::Replace { .. }
                | EditorAction::Rename(_)
                | EditorAction::Batch(_) // May contain modifying actions
        )
    }

    /// Check if this action should save undo state before execution
    pub fn requires_undo_save(&self) -> bool {
        self.modifies_buffer()
    }

    /// Check if this action affects cursor position
    pub fn affects_cursor(&self) -> bool {
        !matches!(
            self,
            EditorAction::Copy
                | EditorAction::Save
                | EditorAction::SaveAs(_)
                | EditorAction::ToggleSidebar
                | EditorAction::None
        )
    }

    /// Get human-readable description of action
    pub fn description(&self) -> String {
        match self {
            EditorAction::InsertChar(c) => format!("Insert '{}'", c),
            EditorAction::InsertText(s) => format!("Insert text ({})", s.chars().count()),
            EditorAction::NewLine => "New line".to_string(),
            EditorAction::Backspace => "Backspace".to_string(),
            EditorAction::Delete => "Delete".to_string(),
            EditorAction::MoveCursor(dir) => format!("Move {:?}", dir),
            EditorAction::MoveToLineStart => "Move to line start".to_string(),
            EditorAction::MoveToLineEnd => "Move to line end".to_string(),
            EditorAction::MoveToPosition { line, col } => {
                format!("Move to {}:{}", line, col)
            }
            EditorAction::PageUp => "Page up".to_string(),
            EditorAction::PageDown => "Page down".to_string(),
            EditorAction::ExtendSelection(dir) => format!("Extend selection {:?}", dir),
            EditorAction::ExtendToLineStart => "Extend to line start".to_string(),
            EditorAction::ExtendToLineEnd => "Extend to line end".to_string(),
            EditorAction::SelectAll => "Select all".to_string(),
            EditorAction::ClearSelection => "Clear selection".to_string(),
            EditorAction::SetSelection { .. } => "Set selection".to_string(),
            EditorAction::Copy => "Copy".to_string(),
            EditorAction::Cut => "Cut".to_string(),
            EditorAction::Paste => "Paste".to_string(),
            EditorAction::Undo => "Undo".to_string(),
            EditorAction::Redo => "Redo".to_string(),
            EditorAction::Save => "Save".to_string(),
            EditorAction::SaveAs(path) => format!("Save as '{}'", path),
            EditorAction::Close => "Close".to_string(),
            EditorAction::Format => "Format document".to_string(),
            EditorAction::ToggleComment => "Toggle comment".to_string(),
            EditorAction::Indent => "Indent".to_string(),
            EditorAction::Dedent => "Dedent".to_string(),
            EditorAction::GotoDefinition => "Go to definition".to_string(),
            EditorAction::TriggerCompletion => "Trigger completion".to_string(),
            EditorAction::ShowHover => "Show hover".to_string(),
            EditorAction::FindReferences => "Find references".to_string(),
            EditorAction::Rename(new_name) => format!("Rename to '{}'", new_name),
            EditorAction::OpenCommandPalette => "Open command palette".to_string(),
            EditorAction::GotoLine(n) => format!("Go to line {}", n),
            EditorAction::Find(text) => format!("Find '{}'", text),
            EditorAction::Replace { find, replace } => {
                format!("Replace '{}' with '{}'", find, replace)
            }
            EditorAction::ToggleSidebar => "Toggle sidebar".to_string(),
            EditorAction::SplitHorizontal => "Split horizontal".to_string(),
            EditorAction::SplitVertical => "Split vertical".to_string(),
            EditorAction::Batch(actions) => format!("Batch ({} actions)", actions.len()),
            EditorAction::None => "No action".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modifies_buffer() {
        assert!(EditorAction::InsertChar('a').modifies_buffer());
        assert!(EditorAction::NewLine.modifies_buffer());
        assert!(EditorAction::Backspace.modifies_buffer());
        assert!(!EditorAction::MoveCursor(Direction::Left).modifies_buffer());
        assert!(!EditorAction::Copy.modifies_buffer());
    }

    #[test]
    fn test_affects_cursor() {
        assert!(EditorAction::MoveCursor(Direction::Right).affects_cursor());
        assert!(EditorAction::InsertChar('x').affects_cursor());
        assert!(!EditorAction::Copy.affects_cursor());
        assert!(!EditorAction::Save.affects_cursor());
    }

    #[test]
    fn test_description() {
        assert_eq!(EditorAction::Undo.description(), "Undo");
        assert_eq!(EditorAction::InsertChar('A').description(), "Insert 'A'");
        assert_eq!(
            EditorAction::MoveToPosition { line: 10, col: 5 }.description(),
            "Move to 10:5"
        );
    }
}
