//! Tauri Git Commands

use crate::git_core::{self as operations, *};
use std::path::PathBuf;
use tauri::State;

/// Git repository manager state
pub struct GitManager {
    repo_path: std::sync::Mutex<Option<PathBuf>>,
}

impl GitManager {
    pub fn new() -> Self {
        Self {
            repo_path: std::sync::Mutex::new(None),
        }
    }

    pub fn set_repo_path(&self, path: PathBuf) {
        *self.repo_path.lock().unwrap() = Some(path);
    }

    pub fn get_repo_path(&self) -> Result<PathBuf, String> {
        self.repo_path
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| "No repository path set".to_string())
    }
}

impl Default for GitManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Set the Git repository path
#[tauri::command]
pub async fn git_set_repo_path(path: String, manager: State<'_, GitManager>) -> Result<(), String> {
    manager.set_repo_path(PathBuf::from(path));
    Ok(())
}

/// Get Git status
#[tauri::command]
pub async fn git_status(manager: State<'_, GitManager>) -> Result<Vec<FileStatus>, String> {
    let repo_path = manager.get_repo_path()?;
    operations::get_status(&repo_path).map_err(|e| e.to_string())
}

/// List branches
#[tauri::command]
pub async fn git_list_branches(manager: State<'_, GitManager>) -> Result<Vec<BranchInfo>, String> {
    let repo_path = manager.get_repo_path()?;
    operations::list_branches(&repo_path).map_err(|e| e.to_string())
}

/// Get current branch
#[tauri::command]
pub async fn git_current_branch(manager: State<'_, GitManager>) -> Result<String, String> {
    let repo_path = manager.get_repo_path()?;
    operations::current_branch(&repo_path).map_err(|e| e.to_string())
}

/// Stage a file
#[tauri::command]
pub async fn git_stage_file(path: String, manager: State<'_, GitManager>) -> Result<(), String> {
    let repo_path = manager.get_repo_path()?;
    operations::stage_file(&repo_path, &path).map_err(|e| e.to_string())
}

/// Unstage a file
#[tauri::command]
pub async fn git_unstage_file(path: String, manager: State<'_, GitManager>) -> Result<(), String> {
    let repo_path = manager.get_repo_path()?;
    operations::unstage_file(&repo_path, &path).map_err(|e| e.to_string())
}

/// Stage all files
#[tauri::command]
pub async fn git_stage_all(manager: State<'_, GitManager>) -> Result<(), String> {
    let repo_path = manager.get_repo_path()?;
    operations::stage_all(&repo_path).map_err(|e| e.to_string())
}

/// Create a commit
#[tauri::command]
pub async fn git_commit(message: String, manager: State<'_, GitManager>) -> Result<String, String> {
    let repo_path = manager.get_repo_path()?;
    operations::commit(&repo_path, &message).map_err(|e| e.to_string())
}

/// Checkout a branch
#[tauri::command]
pub async fn git_checkout_branch(
    branch_name: String,
    manager: State<'_, GitManager>,
) -> Result<(), String> {
    let repo_path = manager.get_repo_path()?;
    operations::checkout_branch(&repo_path, &branch_name).map_err(|e| e.to_string())
}

/// Create a new branch
#[tauri::command]
pub async fn git_create_branch(
    branch_name: String,
    manager: State<'_, GitManager>,
) -> Result<(), String> {
    let repo_path = manager.get_repo_path()?;
    operations::create_branch(&repo_path, &branch_name).map_err(|e| e.to_string())
}

/// Delete a branch
#[tauri::command]
pub async fn git_delete_branch(
    branch_name: String,
    manager: State<'_, GitManager>,
) -> Result<(), String> {
    let repo_path = manager.get_repo_path()?;
    operations::delete_branch(&repo_path, &branch_name).map_err(|e| e.to_string())
}

/// Get commit log
#[tauri::command]
pub async fn git_log(
    limit: Option<usize>,
    manager: State<'_, GitManager>,
) -> Result<Vec<CommitInfo>, String> {
    let repo_path = manager.get_repo_path()?;
    let limit = limit.unwrap_or(100);
    operations::get_log(&repo_path, limit).map_err(|e| e.to_string())
}

/// Get file diff
#[tauri::command]
pub async fn git_diff_file(
    file_path: String,
    manager: State<'_, GitManager>,
) -> Result<FileDiff, String> {
    let repo_path = manager.get_repo_path()?;
    operations::get_file_diff(&repo_path, &file_path).map_err(|e| e.to_string())
}

/// Get blame for a file
#[tauri::command]
pub async fn git_blame(
    file_path: String,
    manager: State<'_, GitManager>,
) -> Result<Vec<BlameLine>, String> {
    let repo_path = manager.get_repo_path()?;
    operations::get_blame(&repo_path, &file_path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_manager_creation() {
        let manager = GitManager::new();
        assert!(manager.get_repo_path().is_err());
    }

    #[test]
    fn test_git_manager_set_path() {
        let manager = GitManager::new();
        manager.set_repo_path(PathBuf::from("/test/path"));
        assert!(manager.get_repo_path().is_ok());
        assert_eq!(
            manager.get_repo_path().unwrap(),
            PathBuf::from("/test/path")
        );
    }
}
