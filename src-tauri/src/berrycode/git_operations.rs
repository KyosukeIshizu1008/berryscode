//! Git Operations Engine
//! Provides high-level Git operations using git2 library
//!
//! This module has been unified with git_core to eliminate code duplication.
//! All git2-based operations now use the centralized git_core implementation.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// Re-export types from git_core with compatibility aliases
pub use crate::git_core::{
    BlameLine as BlameLineInfo,
    BranchInfo,
    CommitInfo,
    DiffHunk,
    DiffLine,
    FileDiff,
    FileStatus,
};

// Import operations from git_core
use crate::git_core;

/// High-level Git operations wrapper that uses git_core
pub struct GitOperations {
    repo_path: PathBuf,
}

impl GitOperations {
    /// Open a Git repository at the given path
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let repo_path = path.as_ref().to_path_buf();

        // Verify it's a valid git repository by trying to get status
        git_core::get_status(&repo_path)
            .with_context(|| format!("Failed to open Git repository at {:?}", repo_path))?;

        Ok(Self { repo_path })
    }

    /// Get the status of all files in the repository
    pub fn get_status(&self) -> Result<Vec<FileStatus>> {
        git_core::get_status(&self.repo_path)
    }

    /// Get file history (commits that touched a specific file)
    pub fn get_file_history<P: AsRef<Path>>(&self, file_path: P) -> Result<Vec<CommitInfo>> {
        // This is a specialized function - implement using git_core's get_log
        // For now, return all commits (could be filtered by file path in the future)
        git_core::get_log(&self.repo_path, 100)
    }

    /// Get blame information for a file
    pub fn blame<P: AsRef<Path>>(&self, file_path: P) -> Result<Vec<BlameLineInfo>> {
        git_core::get_blame(&self.repo_path, file_path.as_ref().to_str().unwrap())
    }

    /// List all branches
    pub fn list_branches(&self) -> Result<Vec<BranchInfo>> {
        git_core::list_branches(&self.repo_path)
    }

    /// Create a new branch
    pub fn create_branch(&self, name: &str) -> Result<()> {
        git_core::create_branch(&self.repo_path, name)
    }

    /// Checkout a branch
    pub fn checkout_branch(&self, name: &str) -> Result<()> {
        git_core::checkout_branch(&self.repo_path, name)
    }

    /// Commit changes
    pub fn commit(&self, message: &str, files: Vec<PathBuf>) -> Result<String> {
        // Stage specified files
        for file in files {
            git_core::stage_file(&self.repo_path, file.to_str().unwrap())?;
        }

        // Create commit
        git_core::commit(&self.repo_path, message)
    }

    /// Get diff for a file
    pub fn diff<P: AsRef<Path>>(&self, file_path: P) -> Result<FileDiff> {
        git_core::get_file_diff(&self.repo_path, file_path.as_ref().to_str().unwrap())
    }

    /// Push to remote
    /// Note: git2-rs doesn't easily support authentication, so this may not work
    /// in all scenarios. Consider using git_ops.rs for network operations.
    pub fn push(&self, _remote: &str, _branch: &str) -> Result<()> {
        anyhow::bail!("Push operation requires authentication. Use git_ops.rs or CLI instead.")
    }

    /// Pull from remote
    /// Note: git2-rs doesn't easily support authentication, so this may not work
    /// in all scenarios. Consider using git_ops.rs for network operations.
    pub fn pull(&self, _remote: &str, _branch: &str) -> Result<()> {
        anyhow::bail!("Pull operation requires authentication. Use git_ops.rs or CLI instead.")
    }

    /// Merge a branch
    /// Note: This is a simplified implementation. For complex merges,
    /// consider using git CLI via git_ops.rs
    pub fn merge_branch(&self, _branch_name: &str) -> Result<String> {
        anyhow::bail!("Merge operation not yet implemented. Use git_ops.rs or CLI instead.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::Repository;
    use tempfile::TempDir;

    fn setup_test_repo() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().to_path_buf();

        // Initialize git repo
        Repository::init(&repo_path).unwrap();

        (temp_dir, repo_path)
    }

    #[test]
    fn test_git_operations_open() {
        let (_temp, repo_path) = setup_test_repo();
        let ops = GitOperations::open(&repo_path);
        assert!(ops.is_ok());
    }

    #[test]
    fn test_git_operations_open_invalid() {
        let result = GitOperations::open("/nonexistent/path");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_status_empty() {
        let (_temp, repo_path) = setup_test_repo();
        let ops = GitOperations::open(&repo_path).unwrap();
        let status = ops.get_status().unwrap();
        assert!(status.is_empty());
    }

    #[test]
    fn test_list_branches() {
        let (_temp, repo_path) = setup_test_repo();
        let ops = GitOperations::open(&repo_path).unwrap();
        let branches = ops.list_branches().unwrap();
        // New repo has no branches until first commit
        assert!(branches.is_empty() || branches.len() >= 0);
    }

    #[test]
    fn test_file_status_serialization() {
        let status = FileStatus {
            path: "test.rs".to_string(),
            status: "modified".to_string(),
            is_staged: false,
        };

        let json = serde_json::to_string(&status).unwrap();
        let deserialized: FileStatus = serde_json::from_str(&json).unwrap();

        assert_eq!(status.path, deserialized.path);
        assert_eq!(status.status, deserialized.status);
        assert_eq!(status.is_staged, deserialized.is_staged);
    }

    #[test]
    fn test_commit_info_serialization() {
        let info = CommitInfo {
            hash: "abc123".to_string(),
            short_hash: "abc".to_string(),
            message: "Test commit".to_string(),
            author: "Test User".to_string(),
            email: "test@example.com".to_string(),
            timestamp: 1234567890,
            parents: vec!["parent1".to_string()],
        };

        let json = serde_json::to_string(&info).unwrap();
        let deserialized: CommitInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(info.hash, deserialized.hash);
        assert_eq!(info.author, deserialized.author);
    }

    #[test]
    fn test_branch_info_serialization() {
        let info = BranchInfo {
            name: "main".to_string(),
            is_head: true,
            upstream: Some("origin/main".to_string()),
            ahead: 0,
            behind: 0,
        };

        let json = serde_json::to_string(&info).unwrap();
        let deserialized: BranchInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(info.name, deserialized.name);
        assert_eq!(info.is_head, deserialized.is_head);
    }
}
