//! Git operations using git2-rs

use super::types::*;
use anyhow::{Context, Result};
use git2::{Branch, BranchType, Commit, DiffOptions, Repository, Status, StatusOptions};
use std::cell::RefCell;
use std::path::Path;

/// Get Git repository status
pub fn get_status(repo_path: &Path) -> Result<Vec<FileStatus>> {
    let repo = Repository::open(repo_path)?;
    let mut opts = StatusOptions::new();
    opts.include_untracked(true);
    opts.recurse_untracked_dirs(true);

    let statuses = repo.statuses(Some(&mut opts))?;

    let mut result = Vec::new();

    for entry in statuses.iter() {
        let path = entry.path().unwrap_or("").to_string();
        let git_status = entry.status();

        // Determine status string
        let status_str = if git_status.contains(Status::INDEX_NEW) {
            "A"
        } else if git_status.contains(Status::INDEX_MODIFIED) {
            "M"
        } else if git_status.contains(Status::INDEX_DELETED) {
            "D"
        } else if git_status.contains(Status::WT_NEW) {
            "U"
        } else if git_status.contains(Status::WT_MODIFIED) {
            "M"
        } else if git_status.contains(Status::WT_DELETED) {
            "D"
        } else {
            "?"
        };

        // Determine if staged
        let is_staged = git_status.contains(Status::INDEX_NEW)
            || git_status.contains(Status::INDEX_MODIFIED)
            || git_status.contains(Status::INDEX_DELETED);

        result.push(FileStatus {
            path,
            status: status_str.to_string(),
            is_staged,
        });
    }

    Ok(result)
}

/// List all branches
pub fn list_branches(repo_path: &Path) -> Result<Vec<BranchInfo>> {
    let repo = Repository::open(repo_path)?;
    let branches = repo.branches(Some(BranchType::Local))?;

    let mut result = Vec::new();

    for branch_result in branches {
        let (branch, _) = branch_result?;
        let name = branch.name()?.unwrap_or("").to_string();
        let is_head = branch.is_head();

        let upstream = branch
            .upstream()
            .ok()
            .and_then(|u| u.name().ok().flatten().map(|s| s.to_string()));

        // Calculate ahead/behind (simplified - would need more complex logic)
        let ahead = 0;
        let behind = 0;

        result.push(BranchInfo {
            name,
            is_head,
            upstream,
            ahead,
            behind,
        });
    }

    Ok(result)
}

/// Get current branch name
pub fn current_branch(repo_path: &Path) -> Result<String> {
    let repo = Repository::open(repo_path)?;
    let head = repo.head()?;

    if let Some(name) = head.shorthand() {
        Ok(name.to_string())
    } else {
        Ok("HEAD (detached)".to_string())
    }
}

/// Stage a file
pub fn stage_file(repo_path: &Path, file_path: &str) -> Result<()> {
    let repo = Repository::open(repo_path)?;
    let mut index = repo.index()?;

    index.add_path(Path::new(file_path))?;
    index.write()?;

    Ok(())
}

/// Unstage a file
pub fn unstage_file(repo_path: &Path, file_path: &str) -> Result<()> {
    let repo = Repository::open(repo_path)?;
    let head_commit = repo.head()?.peel_to_commit()?;
    let head_tree = head_commit.tree()?;

    repo.reset_default(Some(&head_tree.into_object()), &[Path::new(file_path)])?;

    Ok(())
}

/// Stage all files
pub fn stage_all(repo_path: &Path) -> Result<()> {
    let repo = Repository::open(repo_path)?;
    let mut index = repo.index()?;

    index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
    index.write()?;

    Ok(())
}

/// Create a commit
pub fn commit(repo_path: &Path, message: &str) -> Result<String> {
    let repo = Repository::open(repo_path)?;

    // Get signature
    let signature = repo.signature()?;

    // Get tree from index
    let mut index = repo.index()?;
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;

    // Get parent commit
    let parent_commit = repo.head()?.peel_to_commit()?;

    // Create commit
    let commit_id = repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        message,
        &tree,
        &[&parent_commit],
    )?;

    Ok(commit_id.to_string())
}

/// Checkout a branch
pub fn checkout_branch(repo_path: &Path, branch_name: &str) -> Result<()> {
    let repo = Repository::open(repo_path)?;

    let (object, reference) = repo.revparse_ext(branch_name)?;

    repo.checkout_tree(&object, None)?;

    match reference {
        Some(gref) => repo.set_head(gref.name().unwrap())?,
        None => repo.set_head_detached(object.id())?,
    }

    Ok(())
}

/// Create a new branch
pub fn create_branch(repo_path: &Path, branch_name: &str) -> Result<()> {
    let repo = Repository::open(repo_path)?;
    let head_commit = repo.head()?.peel_to_commit()?;

    repo.branch(branch_name, &head_commit, false)?;

    Ok(())
}

/// Delete a branch
pub fn delete_branch(repo_path: &Path, branch_name: &str) -> Result<()> {
    let repo = Repository::open(repo_path)?;
    let mut branch = repo.find_branch(branch_name, BranchType::Local)?;

    branch.delete()?;

    Ok(())
}

/// Get commit log
pub fn get_log(repo_path: &Path, limit: usize) -> Result<Vec<CommitInfo>> {
    let repo = Repository::open(repo_path)?;
    let mut revwalk = repo.revwalk()?;

    revwalk.push_head()?;
    revwalk.set_sorting(git2::Sort::TIME)?;

    let mut result = Vec::new();

    for (idx, oid_result) in revwalk.enumerate() {
        if idx >= limit {
            break;
        }

        let oid = oid_result?;
        let commit = repo.find_commit(oid)?;

        let hash = commit.id().to_string();
        let short_hash = commit
            .as_object()
            .short_id()?
            .as_str()
            .unwrap_or("")
            .to_string();
        let message = commit.message().unwrap_or("").to_string();
        let author = commit.author();
        let author_name = author.name().unwrap_or("").to_string();
        let email = author.email().unwrap_or("").to_string();
        let timestamp = commit.time().seconds();

        let parents = commit.parent_ids().map(|id| id.to_string()).collect();

        result.push(CommitInfo {
            hash,
            short_hash,
            message,
            author: author_name,
            email,
            timestamp,
            parents,
        });
    }

    Ok(result)
}

/// Get file diff
pub fn get_file_diff(repo_path: &Path, file_path: &str) -> Result<FileDiff> {
    let repo = Repository::open(repo_path)?;

    let head_commit = repo.head()?.peel_to_commit()?;
    let head_tree = head_commit.tree()?;

    let mut diff_opts = DiffOptions::new();
    diff_opts.pathspec(file_path);

    let diff = repo.diff_tree_to_workdir(Some(&head_tree), Some(&mut diff_opts))?;

    let file_diff = RefCell::new(FileDiff {
        path: file_path.to_string(),
        old_path: None,
        status: "M".to_string(),
        hunks: Vec::new(),
    });

    diff.foreach(
        &mut |_delta, _progress| true,
        None,
        Some(&mut |_delta, hunk| {
            let old_start = hunk.old_start();
            let old_lines = hunk.old_lines();
            let new_start = hunk.new_start();
            let new_lines = hunk.new_lines();

            file_diff.borrow_mut().hunks.push(DiffHunk {
                old_start,
                old_lines,
                new_start,
                new_lines,
                lines: Vec::new(),
            });

            true
        }),
        Some(&mut |_delta, _hunk, line| {
            let content = String::from_utf8_lossy(line.content()).to_string();
            let line_type = match line.origin() {
                '+' => "add",
                '-' => "delete",
                _ => "context",
            };

            if let Some(last_hunk) = file_diff.borrow_mut().hunks.last_mut() {
                last_hunk.lines.push(DiffLine {
                    line_type: line_type.to_string(),
                    content,
                    old_line_no: line.old_lineno(),
                    new_line_no: line.new_lineno(),
                });
            }

            true
        }),
    )?;

    Ok(file_diff.into_inner())
}

/// Get blame for a file
pub fn get_blame(repo_path: &Path, file_path: &str) -> Result<Vec<BlameLine>> {
    let repo = Repository::open(repo_path)?;
    let blame = repo.blame_file(Path::new(file_path), None)?;

    let mut result = Vec::new();

    // Read file content
    let file_content = std::fs::read_to_string(repo_path.join(file_path))?;

    for (line_no, line_content) in file_content.lines().enumerate() {
        let hunk = blame
            .get_line(line_no + 1)
            .context("Failed to get blame hunk for line")?;
        let commit_id = hunk.final_commit_id();
        let commit = repo.find_commit(commit_id)?;

        let author = commit.author();

        result.push(BlameLine {
            line_no: (line_no + 1) as u32,
            commit_hash: commit_id.to_string(),
            author: author.name().unwrap_or("").to_string(),
            timestamp: commit.time().seconds(),
            content: line_content.to_string(),
        });
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_repo() -> Result<TempDir> {
        let temp_dir = TempDir::new()?;
        let repo = Repository::init(temp_dir.path())?;

        // Create initial commit
        let signature = repo.signature()?;
        let tree_id = {
            let mut index = repo.index()?;
            index.write_tree()?
        };
        let tree = repo.find_tree(tree_id)?;

        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Initial commit",
            &tree,
            &[],
        )?;

        Ok(temp_dir)
    }

    #[test]
    fn test_get_status_empty() {
        let temp_dir = create_test_repo().unwrap();
        let status = get_status(temp_dir.path()).unwrap();
        assert_eq!(status.len(), 0);
    }

    #[test]
    fn test_current_branch() {
        let temp_dir = create_test_repo().unwrap();
        let branch = current_branch(temp_dir.path()).unwrap();
        assert!(branch.contains("main") || branch.contains("master"));
    }

    #[test]
    fn test_list_branches() {
        let temp_dir = create_test_repo().unwrap();
        let branches = list_branches(temp_dir.path()).unwrap();
        assert!(!branches.is_empty());
        assert!(branches.iter().any(|b| b.is_head));
    }
}
