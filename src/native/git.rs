//! Native Git operations using git2
//! Replaces tauri_bindings git commands

use anyhow::{Context, Result};
use git2::{Repository, Status, StatusOptions};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatus {
    pub path: String,
    pub status: String,  // "modified", "added", "deleted", "untracked"
    pub is_staged: bool,  // Whether the file is staged in the index
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBranch {
    pub name: String,
    pub is_current: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitCommit {
    pub id: String,
    pub message: String,
    pub author: String,
    pub date: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitDiffLine {
    pub origin: char,
    pub content: String,
    pub old_lineno: Option<u32>,
    pub new_lineno: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitDiffHunk {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub header: String,
    pub lines: Vec<GitDiffLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitDiff {
    pub old_path: Option<String>,
    pub new_path: Option<String>,
    pub status: String,
    pub hunks: Vec<GitDiffHunk>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBlame {
    pub line_number: u32,
    pub commit_id: String,
    pub author: String,
    pub date: u64,
    pub content: String,
}

/// Open a Git repository
fn open_repo(path: impl AsRef<Path>) -> Result<Repository> {
    Repository::discover(path.as_ref()).context("Failed to open Git repository")
}

/// Get current branch name
pub fn get_current_branch(repo_path: impl AsRef<Path>) -> Result<String> {
    let repo = open_repo(repo_path)?;
    let head = repo.head().context("Failed to get HEAD")?;

    if let Some(name) = head.shorthand() {
        Ok(name.to_string())
    } else {
        Ok("(detached HEAD)".to_string())
    }
}

/// Get Git status for all files
pub fn get_status(repo_path: impl AsRef<Path>) -> Result<Vec<GitStatus>> {
    let repo = open_repo(repo_path)?;

    let mut opts = StatusOptions::new();
    opts.include_untracked(true);
    opts.recurse_untracked_dirs(true);

    let statuses = repo.statuses(Some(&mut opts)).context("Failed to get status")?;

    let mut results = Vec::new();

    for entry in statuses.iter() {
        let path = entry.path().unwrap_or("").to_string();
        let status_flags = entry.status();

        // Determine status based on working tree or index changes
        let status = if status_flags.contains(Status::INDEX_NEW) || status_flags.contains(Status::WT_NEW) {
            "added"
        } else if status_flags.contains(Status::INDEX_MODIFIED) || status_flags.contains(Status::WT_MODIFIED) {
            "modified"
        } else if status_flags.contains(Status::INDEX_DELETED) || status_flags.contains(Status::WT_DELETED) {
            "deleted"
        } else {
            "untracked"
        };

        // File is staged if it has INDEX_ flags
        let is_staged = status_flags.intersects(
            Status::INDEX_NEW
                | Status::INDEX_MODIFIED
                | Status::INDEX_DELETED
                | Status::INDEX_RENAMED
                | Status::INDEX_TYPECHANGE,
        );

        results.push(GitStatus {
            path,
            status: status.to_string(),
            is_staged,
        });
    }

    Ok(results)
}

/// Stage a file
pub fn stage_file(repo_path: impl AsRef<Path>, file_path: &str) -> Result<()> {
    let repo = open_repo(repo_path)?;
    let mut index = repo.index().context("Failed to get index")?;

    index
        .add_path(Path::new(file_path))
        .context("Failed to stage file")?;

    index.write().context("Failed to write index")?;

    Ok(())
}

/// Stage all files
pub fn stage_all(repo_path: impl AsRef<Path>) -> Result<()> {
    let repo = open_repo(repo_path)?;
    let mut index = repo.index().context("Failed to get index")?;

    index
        .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
        .context("Failed to stage all")?;

    index.write().context("Failed to write index")?;

    Ok(())
}

/// Unstage a file (remove from index but keep working tree changes)
pub fn unstage_file(repo_path: impl AsRef<Path>, file_path: &str) -> Result<()> {
    let repo = open_repo(repo_path)?;
    let mut index = repo.index().context("Failed to get index")?;

    // Get the HEAD commit
    let head = repo.head().context("Failed to get HEAD")?;
    let head_commit = head.peel_to_commit().context("Failed to peel to commit")?;
    let head_tree = head_commit.tree().context("Failed to get tree")?;

    // Reset the file in the index to the HEAD version
    let entry = head_tree
        .get_path(Path::new(file_path))
        .context("File not found in HEAD")?;

    index.add(&git2::IndexEntry {
        ctime: git2::IndexTime::new(0, 0),
        mtime: git2::IndexTime::new(0, 0),
        dev: 0,
        ino: 0,
        mode: entry.filemode() as u32,
        uid: 0,
        gid: 0,
        file_size: 0,
        id: entry.id(),
        flags: 0,
        flags_extended: 0,
        path: file_path.as_bytes().to_vec(),
    }).context("Failed to unstage file")?;

    index.write().context("Failed to write index")?;

    Ok(())
}

/// Create a commit
pub fn commit(repo_path: impl AsRef<Path>, message: &str) -> Result<String> {
    let repo = open_repo(repo_path)?;
    let mut index = repo.index().context("Failed to get index")?;
    let tree_id = index.write_tree().context("Failed to write tree")?;
    let tree = repo.find_tree(tree_id).context("Failed to find tree")?;

    let signature = repo.signature().context("Failed to get signature")?;
    let parent_commit = repo.head()?.peel_to_commit()?;

    let commit_id = repo
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &[&parent_commit],
        )
        .context("Failed to create commit")?;

    Ok(commit_id.to_string())
}

/// List all branches
pub fn list_branches(repo_path: impl AsRef<Path>) -> Result<Vec<GitBranch>> {
    let repo = open_repo(repo_path)?;
    let branches = repo.branches(None).context("Failed to list branches")?;

    let mut results = Vec::new();

    for branch in branches {
        let (branch, _) = branch.context("Failed to get branch")?;
        let name = branch
            .name()
            .context("Failed to get branch name")?
            .unwrap_or("")
            .to_string();
        let is_current = branch.is_head();

        results.push(GitBranch { name, is_current });
    }

    Ok(results)
}

/// Checkout a branch
pub fn checkout_branch(repo_path: impl AsRef<Path>, branch_name: &str) -> Result<()> {
    let repo = open_repo(repo_path)?;

    let (object, reference) = repo
        .revparse_ext(branch_name)
        .context("Failed to parse branch name")?;

    repo.checkout_tree(&object, None)
        .context("Failed to checkout tree")?;

    match reference {
        Some(gref) => repo
            .set_head(gref.name().unwrap())
            .context("Failed to set HEAD")?,
        None => repo
            .set_head_detached(object.id())
            .context("Failed to set detached HEAD")?,
    }

    Ok(())
}

/// Create a new branch
pub fn create_branch(repo_path: impl AsRef<Path>, branch_name: &str) -> Result<()> {
    let repo = open_repo(repo_path)?;
    let head = repo.head().context("Failed to get HEAD")?;
    let commit = head.peel_to_commit().context("Failed to get commit")?;

    repo.branch(branch_name, &commit, false)
        .context("Failed to create branch")?;

    Ok(())
}

/// Delete a branch
pub fn delete_branch(repo_path: impl AsRef<Path>, branch_name: &str) -> Result<()> {
    let repo = open_repo(repo_path)?;
    let mut branch = repo.find_branch(branch_name, git2::BranchType::Local)
        .context("Failed to find branch")?;

    branch.delete().context("Failed to delete branch")?;

    Ok(())
}

/// Get commit log
pub fn get_log(repo_path: impl AsRef<Path>, limit: usize) -> Result<Vec<GitCommit>> {
    let repo = open_repo(repo_path)?;
    let mut revwalk = repo.revwalk().context("Failed to create revwalk")?;

    revwalk
        .push_head()
        .context("Failed to push HEAD to revwalk")?;

    let mut results = Vec::new();

    for (i, oid) in revwalk.enumerate() {
        if i >= limit {
            break;
        }

        let oid = oid.context("Failed to get commit OID")?;
        let commit = repo.find_commit(oid).context("Failed to find commit")?;

        results.push(GitCommit {
            id: oid.to_string(),
            message: commit.message().unwrap_or("").to_string(),
            author: commit.author().name().unwrap_or("").to_string(),
            date: commit.time().seconds() as u64,
        });
    }

    Ok(results)
}

/// Get diff for a file (simplified version - shows working tree vs HEAD)
pub fn get_diff(repo_path: impl AsRef<Path>, file_path: &str) -> Result<GitDiff> {
    let repo = open_repo(repo_path)?;

    // For simplicity, return a basic diff structure
    // Full implementation would require more complex git2 diff parsing
    Ok(GitDiff {
        old_path: Some(file_path.to_string()),
        new_path: Some(file_path.to_string()),
        status: String::from("modified"),
        hunks: Vec::new(), // TODO: Implement full diff parsing
    })
}

/// Get blame information for a file
pub fn get_blame(repo_path: impl AsRef<Path>, file_path: &str) -> Result<Vec<GitBlame>> {
    let repo = open_repo(repo_path)?;

    let mut blame_options = git2::BlameOptions::new();
    let blame = repo.blame_file(Path::new(file_path), Some(&mut blame_options))
        .context("Failed to get blame")?;

    let mut results = Vec::new();

    for i in 0..blame.len() {
        let hunk = blame.get_index(i).context("Failed to get blame hunk")?;
        let commit = repo.find_commit(hunk.final_commit_id())
            .context("Failed to find commit")?;

        // Get file content for this line (simplified - just showing line number)
        let line_number = hunk.final_start_line() as u32;

        results.push(GitBlame {
            line_number,
            commit_id: hunk.final_commit_id().to_string(),
            author: commit.author().name().unwrap_or("").to_string(),
            date: commit.time().seconds() as u64,
            content: String::new(), // Would need to read file content
        });
    }

    Ok(results)
}
