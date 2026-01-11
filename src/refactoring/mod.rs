//! Refactoring UI Module
//! Provides IntelliJ-style refactoring UI components

pub mod refactor_menu;
pub mod preview_dialog;

pub use refactor_menu::RefactorMenu;
pub use preview_dialog::RefactoringPreview;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Position in a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

/// Range in a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

/// Text edit operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextEdit {
    pub range: Range,
    pub new_text: String,
}

/// Workspace edit containing changes to multiple files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceEdit {
    pub changes: HashMap<String, Vec<TextEdit>>,
}

/// Refactoring operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefactorOperation {
    Rename,
    ExtractMethod,
    InlineVariable,
    OptimizeImports,
    MoveSymbol,
    ChangeSignature,
}

impl RefactorOperation {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Rename => "Rename",
            Self::ExtractMethod => "Extract Method",
            Self::InlineVariable => "Inline Variable",
            Self::OptimizeImports => "Optimize Imports",
            Self::MoveSymbol => "Move Symbol",
            Self::ChangeSignature => "Change Signature",
        }
    }

    pub fn shortcut(&self) -> &'static str {
        match self {
            Self::Rename => "F2",
            Self::ExtractMethod => "Ctrl+Alt+M",
            Self::InlineVariable => "Ctrl+Alt+N",
            Self::OptimizeImports => "Ctrl+Alt+O",
            Self::MoveSymbol => "F6",
            Self::ChangeSignature => "Ctrl+F6",
        }
    }
}
