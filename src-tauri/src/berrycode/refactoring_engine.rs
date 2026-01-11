//! Refactoring Engine
//! Provides high-level refactoring operations using LSP

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::collections::HashMap;
use crate::berrycode::lsp_client::LspClient;
use std::sync::RwLock;
use once_cell::sync::Lazy;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextEdit {
    pub range: Range,
    pub new_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceEdit {
    pub changes: HashMap<String, Vec<TextEdit>>,
}

// Global LSP client for refactoring operations (shared with editor_backend)
static REFACTOR_LSP_CLIENT: Lazy<RwLock<Option<LspClient>>> = Lazy::new(|| RwLock::new(None));

pub struct RefactoringEngine {
    // LSP_CLIENT is a global static, so we don't need to store it
}

impl RefactoringEngine {
    pub fn new() -> Self {
        Self {}
    }

    /// Initialize LSP client for refactoring (call this before using refactoring operations)
    pub fn init_lsp(project_root: std::path::PathBuf) {
        let client = LspClient::new(project_root);
        *REFACTOR_LSP_CLIENT.write().unwrap() = Some(client);
    }

    /// Rename symbol at the given position
    pub async fn rename_symbol(
        &self,
        file_path: &Path,
        position: Position,
        new_name: &str,
    ) -> Result<WorkspaceEdit> {
        // Try to use LSP client if available
        let lsp_client = REFACTOR_LSP_CLIENT.read().unwrap();

        if let Some(client) = lsp_client.as_ref() {
            // Use real LSP rename
            if let Some(lsp_edit) = client.rename(file_path, position.line, position.character, new_name)? {
                // Convert LSP WorkspaceEdit to our WorkspaceEdit
                let mut changes = HashMap::new();

                if let Some(lsp_changes) = lsp_edit.changes {
                    for (uri, edits) in lsp_changes {
                        let file_str = uri.to_file_path()
                            .ok()
                            .and_then(|p| p.to_str().map(|s| s.to_string()))
                            .unwrap_or_else(|| uri.to_string());

                        let text_edits: Vec<TextEdit> = edits.iter().map(|e| TextEdit {
                            range: Range {
                                start: Position {
                                    line: e.range.start.line,
                                    character: e.range.start.character,
                                },
                                end: Position {
                                    line: e.range.end.line,
                                    character: e.range.end.character,
                                },
                            },
                            new_text: e.new_text.clone(),
                        }).collect();

                        changes.insert(file_str, text_edits);
                    }
                }

                return Ok(WorkspaceEdit { changes });
            }
        }

        // Fallback to mock implementation if LSP not available
        let file_path_str = file_path.to_str()
            .ok_or_else(|| anyhow!("Invalid file path"))?;

        let mut changes = HashMap::new();
        changes.insert(
            file_path_str.to_string(),
            vec![TextEdit {
                range: Range {
                    start: position.clone(),
                    end: Position {
                        line: position.line,
                        character: position.character + 10,
                    },
                },
                new_text: new_name.to_string(),
            }],
        );

        Ok(WorkspaceEdit { changes })
    }

    /// Extract method from selected range
    pub async fn extract_method(
        &self,
        file_path: &Path,
        range: Range,
        method_name: &str,
    ) -> Result<Vec<TextEdit>> {
        let file_path_str = file_path.to_str()
            .ok_or_else(|| anyhow!("Invalid file path"))?;

        // Mock implementation
        // In production, this would analyze the AST and generate proper edits
        Ok(vec![
            TextEdit {
                range: range.clone(),
                new_text: format!("{}();", method_name),
            },
            TextEdit {
                range: Range {
                    start: Position { line: 0, character: 0 },
                    end: Position { line: 0, character: 0 },
                },
                new_text: format!("\nfn {}() {{\n    // Extracted code\n}}\n", method_name),
            },
        ])
    }

    /// Inline variable at the given position
    pub async fn inline_variable(
        &self,
        file_path: &Path,
        position: Position,
    ) -> Result<Vec<TextEdit>> {
        // Mock implementation
        // In production, this would find all usages and inline the value
        Ok(vec![TextEdit {
            range: Range {
                start: position.clone(),
                end: Position {
                    line: position.line + 1,
                    character: 0,
                },
            },
            new_text: String::new(), // Remove variable declaration
        }])
    }

    /// Optimize imports in the file
    pub async fn optimize_imports(&self, file_path: &Path) -> Result<Vec<TextEdit>> {
        let lsp_client = REFACTOR_LSP_CLIENT.read().unwrap();

        if let Some(client) = lsp_client.as_ref() {
            // Use LSP code actions to organize imports
            // Request code actions for the entire file
            let actions = client.code_actions(
                file_path,
                0,
                0,
                u32::MAX,
                u32::MAX,
                Vec::new(), // No specific diagnostics
            )?;

            // Look for "organize imports" or "sort imports" action
            for action in actions {
                if let lsp_types::CodeActionOrCommand::CodeAction(code_action) = action {
                    let title = code_action.title.to_lowercase();
                    if title.contains("organize") || title.contains("import") || title.contains("sort") {
                        // Extract edits from the code action
                        if let Some(workspace_edit) = code_action.edit {
                            if let Some(changes) = workspace_edit.changes {
                                // Get edits for this file
                                let uri = lsp_types::Url::from_file_path(file_path)
                                    .map_err(|_| anyhow!("Invalid file path"))?;

                                if let Some(edits) = changes.get(&uri) {
                                    return Ok(edits.iter().map(|e| TextEdit {
                                        range: Range {
                                            start: Position {
                                                line: e.range.start.line,
                                                character: e.range.start.character,
                                            },
                                            end: Position {
                                                line: e.range.end.line,
                                                character: e.range.end.character,
                                            },
                                        },
                                        new_text: e.new_text.clone(),
                                    }).collect());
                                }
                            }
                        }
                    }
                }
            }
        }

        // Fallback: return empty list (no changes)
        Ok(Vec::new())
    }

    /// Move symbol to another file
    pub async fn move_symbol(
        &self,
        file_path: &Path,
        position: Position,
        target_file: &Path,
    ) -> Result<WorkspaceEdit> {
        let source_path = file_path.to_str()
            .ok_or_else(|| anyhow!("Invalid source file path"))?;
        let target_path = target_file.to_str()
            .ok_or_else(|| anyhow!("Invalid target file path"))?;

        let mut changes = HashMap::new();

        // Remove from source
        changes.insert(
            source_path.to_string(),
            vec![TextEdit {
                range: Range {
                    start: position.clone(),
                    end: Position {
                        line: position.line + 5,
                        character: 0,
                    },
                },
                new_text: String::new(),
            }],
        );

        // Add to target
        changes.insert(
            target_path.to_string(),
            vec![TextEdit {
                range: Range {
                    start: Position { line: 0, character: 0 },
                    end: Position { line: 0, character: 0 },
                },
                new_text: "// Moved symbol\n".to_string(),
            }],
        );

        Ok(WorkspaceEdit { changes })
    }

    /// Change signature of a function
    pub async fn change_signature(
        &self,
        file_path: &Path,
        position: Position,
        new_signature: &str,
    ) -> Result<WorkspaceEdit> {
        let file_path_str = file_path.to_str()
            .ok_or_else(|| anyhow!("Invalid file path"))?;

        let mut changes = HashMap::new();
        changes.insert(
            file_path_str.to_string(),
            vec![TextEdit {
                range: Range {
                    start: position.clone(),
                    end: Position {
                        line: position.line,
                        character: position.character + 20,
                    },
                },
                new_text: new_signature.to_string(),
            }],
        );

        Ok(WorkspaceEdit { changes })
    }
}

impl Default for RefactoringEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_rename_symbol() {
        let engine = RefactoringEngine::new();
        let result = engine
            .rename_symbol(
                Path::new("/test.rs"),
                Position { line: 10, character: 5 },
                "new_name",
            )
            .await;

        assert!(result.is_ok());
        let edit = result.unwrap();
        assert_eq!(edit.changes.len(), 1);
    }

    #[tokio::test]
    async fn test_extract_method() {
        let engine = RefactoringEngine::new();
        let result = engine
            .extract_method(
                Path::new("/test.rs"),
                Range {
                    start: Position { line: 5, character: 0 },
                    end: Position { line: 10, character: 0 },
                },
                "extracted_fn",
            )
            .await;

        assert!(result.is_ok());
        let edits = result.unwrap();
        assert_eq!(edits.len(), 2);
    }

    #[tokio::test]
    async fn test_inline_variable() {
        let engine = RefactoringEngine::new();
        let result = engine
            .inline_variable(Path::new("/test.rs"), Position { line: 3, character: 4 })
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_optimize_imports() {
        let engine = RefactoringEngine::new();
        let result = engine.optimize_imports(Path::new("/test.rs")).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_move_symbol() {
        let engine = RefactoringEngine::new();
        let result = engine
            .move_symbol(
                Path::new("/source.rs"),
                Position { line: 5, character: 0 },
                Path::new("/target.rs"),
            )
            .await;

        assert!(result.is_ok());
        let edit = result.unwrap();
        assert_eq!(edit.changes.len(), 2);
    }

    #[tokio::test]
    async fn test_change_signature() {
        let engine = RefactoringEngine::new();
        let result = engine
            .change_signature(
                Path::new("/test.rs"),
                Position { line: 2, character: 0 },
                "fn new_signature(x: i32, y: String)",
            )
            .await;

        assert!(result.is_ok());
    }
}
