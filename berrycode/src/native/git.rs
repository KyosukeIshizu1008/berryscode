//! Native Git operations using git2
//! Replaces tauri_bindings git commands

use anyhow::{Context, Result};
use git2::{Repository, Status, StatusOptions};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatus {
    pub path: String,
    pub status: String,  // "modified", "added", "deleted", "untracked"
    pub is_staged: bool, // Whether the file is staged in the index
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

// ===== NEW: SourceTree-Compatible Data Structures =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitRemote {
    pub name: String,
    pub url: String,
    pub fetch_url: String,
    pub push_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitTag {
    pub name: String,
    pub commit_id: String,
    pub message: Option<String>, // None for lightweight tags
    pub tagger: Option<String>,
    pub date: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStash {
    pub index: usize,
    pub message: String,
    pub commit_id: String,
    pub date: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitFileChange {
    pub path: String,
    pub old_path: Option<String>, // For renames
    pub status: String,           // "added", "modified", "deleted", "renamed"
    pub additions: u32,
    pub deletions: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitCommitDetail {
    pub commit: GitCommit,
    pub parents: Vec<String>,
    pub changed_files: Vec<GitFileChange>,
    pub total_additions: u32,
    pub total_deletions: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GraphLineType {
    Direct, // Straight line (parent-child)
    Merge,  // Bezier curve (merge line)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphLine {
    pub from_column: usize,
    pub to_column: usize,
    pub line_type: GraphLineType,
    pub color_index: usize, // Index into color palette (0-7)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitGraphNode {
    pub commit: GitCommit,
    pub parents: Vec<String>,
    pub children: Vec<String>,
    pub branch_names: Vec<String>,
    pub tag_names: Vec<String>,
    pub graph_column: usize,         // Column position in the graph (0-based)
    pub graph_lines: Vec<GraphLine>, // Lines to draw from this node
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

    let statuses = repo
        .statuses(Some(&mut opts))
        .context("Failed to get status")?;

    let mut results = Vec::new();

    for entry in statuses.iter() {
        let path = entry.path().unwrap_or("").to_string();
        let status_flags = entry.status();

        // Determine status based on working tree or index changes
        let status =
            if status_flags.contains(Status::INDEX_NEW) || status_flags.contains(Status::WT_NEW) {
                "added"
            } else if status_flags.contains(Status::INDEX_MODIFIED)
                || status_flags.contains(Status::WT_MODIFIED)
            {
                "modified"
            } else if status_flags.contains(Status::INDEX_DELETED)
                || status_flags.contains(Status::WT_DELETED)
            {
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

    index
        .add(&git2::IndexEntry {
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
        })
        .context("Failed to unstage file")?;

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
    let mut branch = repo
        .find_branch(branch_name, git2::BranchType::Local)
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
    let repo = open_repo(repo_path.as_ref())?;

    // Check if file is untracked (new file)
    let full_path = repo_path.as_ref().join(file_path);
    let is_new_file = if full_path.exists() {
        // Check if file is in the index
        let index = repo.index()?;
        index.get_path(Path::new(file_path), 0).is_none()
    } else {
        false
    };

    // For new files, create a synthetic diff showing all lines as additions
    if is_new_file {
        let content = std::fs::read_to_string(&full_path)
            .unwrap_or_else(|_| String::from("(Binary file or unreadable)"));

        let mut lines = Vec::new();
        for (idx, line) in content.lines().enumerate() {
            lines.push(GitDiffLine {
                origin: '+',
                content: format!("{}\n", line),
                old_lineno: None,
                new_lineno: Some((idx + 1) as u32),
            });
        }

        let hunk = GitDiffHunk {
            old_start: 0,
            old_lines: 0,
            new_start: 1,
            new_lines: lines.len() as u32,
            header: format!("@@ -0,0 +1,{} @@ New file\n", lines.len()),
            lines,
        };

        return Ok(GitDiff {
            old_path: None,
            new_path: Some(file_path.to_string()),
            status: String::from("added"),
            hunks: vec![hunk],
        });
    }

    let head_tree = repo.head()?.peel_to_tree()?;

    // Get diff between HEAD and working directory
    let mut diff_options = git2::DiffOptions::new();
    diff_options.pathspec(file_path);
    diff_options.context_lines(3);

    let diff = repo.diff_tree_to_workdir_with_index(Some(&head_tree), Some(&mut diff_options))?;

    let old_path = RefCell::new(Some(file_path.to_string()));
    let new_path = RefCell::new(Some(file_path.to_string()));
    let status = RefCell::new(String::from("modified"));
    let hunks = RefCell::new(Vec::new());
    let current_hunk = RefCell::new(None::<GitDiffHunk>);

    // Parse diff hunks and lines
    diff.foreach(
        &mut |delta, _progress| {
            *old_path.borrow_mut() = delta
                .old_file()
                .path()
                .map(|p| p.to_string_lossy().to_string());
            *new_path.borrow_mut() = delta
                .new_file()
                .path()
                .map(|p| p.to_string_lossy().to_string());
            *status.borrow_mut() = match delta.status() {
                git2::Delta::Added => "added",
                git2::Delta::Deleted => "deleted",
                git2::Delta::Modified => "modified",
                git2::Delta::Renamed => "renamed",
                _ => "modified",
            }
            .to_string();
            true
        },
        None,
        Some(&mut |_delta, hunk| {
            // Save previous hunk if exists
            if let Some(h) = current_hunk.borrow_mut().take() {
                hunks.borrow_mut().push(h);
            }

            // Create new hunk
            *current_hunk.borrow_mut() = Some(GitDiffHunk {
                old_start: hunk.old_start(),
                old_lines: hunk.old_lines(),
                new_start: hunk.new_start(),
                new_lines: hunk.new_lines(),
                header: String::from_utf8_lossy(hunk.header()).to_string(),
                lines: Vec::new(),
            });
            true
        }),
        Some(&mut |_delta, _hunk, line| {
            if let Some(ref mut hunk) = *current_hunk.borrow_mut() {
                hunk.lines.push(GitDiffLine {
                    origin: line.origin(),
                    content: String::from_utf8_lossy(line.content()).to_string(),
                    old_lineno: line.old_lineno(),
                    new_lineno: line.new_lineno(),
                });
            }
            true
        }),
    )?;

    // Save last hunk if exists
    if let Some(h) = current_hunk.borrow_mut().take() {
        hunks.borrow_mut().push(h);
    }

    Ok(GitDiff {
        old_path: old_path.into_inner(),
        new_path: new_path.into_inner(),
        status: status.into_inner(),
        hunks: hunks.into_inner(),
    })
}

/// Get blame information for a file
pub fn get_blame(repo_path: impl AsRef<Path>, file_path: &str) -> Result<Vec<GitBlame>> {
    let repo = open_repo(repo_path)?;

    let mut blame_options = git2::BlameOptions::new();
    let blame = repo
        .blame_file(Path::new(file_path), Some(&mut blame_options))
        .context("Failed to get blame")?;

    let mut results = Vec::new();

    for i in 0..blame.len() {
        let hunk = blame.get_index(i).context("Failed to get blame hunk")?;
        let commit = repo
            .find_commit(hunk.final_commit_id())
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

// ===== NEW: SourceTree-Compatible Git Functions =====

/// Get detailed commit log with graph structure for visualization
pub fn get_detailed_log(
    repo_path: impl AsRef<Path>,
    limit: usize,
    all_branches: bool,
) -> Result<Vec<GitGraphNode>> {
    let repo = open_repo(repo_path)?;
    let mut revwalk = repo.revwalk().context("Failed to create revwalk")?;

    // Configure revwalk
    revwalk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)?;

    if all_branches {
        // Include all references (branches and tags)
        revwalk.push_glob("refs/heads/*")?;
        revwalk.push_glob("refs/remotes/*")?;
        revwalk.push_glob("refs/tags/*")?;
    } else {
        // Only current branch
        revwalk.push_head()?;
    }

    let mut nodes = Vec::new();
    let mut column_map: std::collections::HashMap<git2::Oid, usize> =
        std::collections::HashMap::new();
    let mut next_column = 0;

    // Collect branch and tag references
    let branches = repo.branches(None)?;
    let mut branch_map: std::collections::HashMap<git2::Oid, Vec<String>> =
        std::collections::HashMap::new();
    for branch_result in branches {
        let (branch, _) = branch_result?;
        if let Some(oid) = branch.get().target() {
            let name = branch.name()?.unwrap_or("").to_string();
            branch_map.entry(oid).or_insert_with(Vec::new).push(name);
        }
    }

    let tags = repo.tag_names(None)?;
    let mut tag_map: std::collections::HashMap<git2::Oid, Vec<String>> =
        std::collections::HashMap::new();
    for tag_name in tags.iter().flatten() {
        if let Ok(reference) = repo.find_reference(&format!("refs/tags/{}", tag_name)) {
            if let Some(oid) = reference.target() {
                tag_map
                    .entry(oid)
                    .or_insert_with(Vec::new)
                    .push(tag_name.to_string());
            }
        }
    }

    for (i, oid_result) in revwalk.enumerate() {
        if i >= limit {
            break;
        }

        let oid = oid_result?;
        let commit = repo.find_commit(oid)?;

        // Get parent IDs
        let parents: Vec<String> = commit.parents().map(|p| p.id().to_string()).collect();

        // Assign column for this commit
        let column = if let Some(&col) = column_map.get(&oid) {
            col
        } else {
            let col = next_column;
            next_column += 1;
            column_map.insert(oid, col);
            col
        };

        // Create graph lines
        let mut graph_lines = Vec::new();
        for (idx, parent_id_str) in parents.iter().enumerate() {
            if let Ok(parent_oid) = parent_id_str.parse::<git2::Oid>() {
                let parent_column = if idx == 0 {
                    // First parent: same column (straight line)
                    column
                } else {
                    // Additional parents: new column (merge line)
                    let col = next_column;
                    next_column += 1;
                    col
                };

                column_map.insert(parent_oid, parent_column);

                graph_lines.push(GraphLine {
                    from_column: column,
                    to_column: parent_column,
                    line_type: if idx == 0 {
                        GraphLineType::Direct
                    } else {
                        GraphLineType::Merge
                    },
                    color_index: column % 8, // 8 color palette
                });
            }
        }

        nodes.push(GitGraphNode {
            commit: GitCommit {
                id: oid.to_string(),
                message: commit.message().unwrap_or("").to_string(),
                author: commit.author().name().unwrap_or("").to_string(),
                date: commit.time().seconds() as u64,
            },
            parents: parents.clone(),
            children: Vec::new(), // Will be populated in a second pass if needed
            branch_names: branch_map.get(&oid).cloned().unwrap_or_default(),
            tag_names: tag_map.get(&oid).cloned().unwrap_or_default(),
            graph_column: column,
            graph_lines,
        });
    }

    Ok(nodes)
}

/// Get detailed information about a specific commit
pub fn get_commit_detail(repo_path: impl AsRef<Path>, commit_id: &str) -> Result<GitCommitDetail> {
    let repo = open_repo(repo_path)?;
    let oid = git2::Oid::from_str(commit_id).context("Invalid commit ID")?;
    let commit = repo.find_commit(oid).context("Commit not found")?;

    // Get parents
    let parents: Vec<String> = commit.parents().map(|p| p.id().to_string()).collect();

    // Get diff stats
    let tree = commit.tree()?;
    let parent_tree = if commit.parent_count() > 0 {
        Some(commit.parent(0)?.tree()?)
    } else {
        None
    };

    let mut diff_options = git2::DiffOptions::new();
    let diff =
        repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(&mut diff_options))?;

    let mut changed_files = Vec::new();
    let mut total_additions = 0;
    let mut total_deletions = 0;

    for delta in diff.deltas() {
        let old_path = delta
            .old_file()
            .path()
            .and_then(|p| p.to_str())
            .map(String::from);
        let new_path = delta
            .new_file()
            .path()
            .and_then(|p| p.to_str())
            .map(String::from);

        let status = match delta.status() {
            git2::Delta::Added => "added",
            git2::Delta::Deleted => "deleted",
            git2::Delta::Modified => "modified",
            git2::Delta::Renamed => "renamed",
            _ => "unknown",
        };

        // Get stats for this file
        let stats = diff.stats()?;
        let additions = stats.insertions() as u32;
        let deletions = stats.deletions() as u32;

        total_additions += additions;
        total_deletions += deletions;

        changed_files.push(GitFileChange {
            path: new_path
                .clone()
                .unwrap_or_else(|| old_path.clone().unwrap_or_default()),
            old_path,
            status: status.to_string(),
            additions,
            deletions,
        });
    }

    // Extract commit information before creating the result
    let message = commit.message().unwrap_or("").to_string();
    let author = commit.author().name().unwrap_or("").to_string();
    let date = commit.time().seconds() as u64;

    Ok(GitCommitDetail {
        commit: GitCommit {
            id: commit_id.to_string(),
            message,
            author,
            date,
        },
        parents,
        changed_files,
        total_additions,
        total_deletions,
    })
}

/// Get diff between two commits
pub fn get_commit_diff(
    repo_path: impl AsRef<Path>,
    old_id: &str,
    new_id: &str,
) -> Result<Vec<GitDiff>> {
    let repo = open_repo(repo_path)?;
    let old_oid = git2::Oid::from_str(old_id)?;
    let new_oid = git2::Oid::from_str(new_id)?;

    let old_commit = repo.find_commit(old_oid)?;
    let new_commit = repo.find_commit(new_oid)?;

    let old_tree = old_commit.tree()?;
    let new_tree = new_commit.tree()?;

    let mut diff_options = git2::DiffOptions::new();
    let diff = repo.diff_tree_to_tree(Some(&old_tree), Some(&new_tree), Some(&mut diff_options))?;

    let mut results = Vec::new();

    for delta in diff.deltas() {
        let old_path = delta
            .old_file()
            .path()
            .and_then(|p| p.to_str())
            .map(String::from);
        let new_path = delta
            .new_file()
            .path()
            .and_then(|p| p.to_str())
            .map(String::from);

        let status = match delta.status() {
            git2::Delta::Added => "added",
            git2::Delta::Deleted => "deleted",
            git2::Delta::Modified => "modified",
            git2::Delta::Renamed => "renamed",
            _ => "unknown",
        };

        // Parse hunks
        let mut hunks = Vec::new();
        diff.foreach(
            &mut |_, _| true,
            None,
            Some(&mut |delta_ref, hunk| {
                if delta_ref.old_file().path() == delta.old_file().path() {
                    hunks.push(GitDiffHunk {
                        old_start: hunk.old_start(),
                        old_lines: hunk.old_lines(),
                        new_start: hunk.new_start(),
                        new_lines: hunk.new_lines(),
                        header: String::from_utf8_lossy(hunk.header()).to_string(),
                        lines: Vec::new(), // Lines would be populated in line callback
                    });
                }
                true
            }),
            None,
        )?;

        results.push(GitDiff {
            old_path,
            new_path,
            status: status.to_string(),
            hunks,
        });
    }

    Ok(results)
}

// ===== Remote Operations =====

/// List all remotes
pub fn list_remotes(repo_path: impl AsRef<Path>) -> Result<Vec<GitRemote>> {
    let repo = open_repo(repo_path)?;
    let remotes = repo.remotes()?;

    let mut results = Vec::new();

    for name in remotes.iter().flatten() {
        if let Ok(remote) = repo.find_remote(name) {
            let url = remote.url().unwrap_or("").to_string();
            let fetch_url = remote.url().unwrap_or("").to_string();
            let push_url = remote
                .pushurl()
                .unwrap_or(remote.url().unwrap_or(""))
                .to_string();

            results.push(GitRemote {
                name: name.to_string(),
                url,
                fetch_url,
                push_url,
            });
        }
    }

    Ok(results)
}

/// Add a new remote
pub fn add_remote(repo_path: impl AsRef<Path>, name: &str, url: &str) -> Result<()> {
    let repo = open_repo(repo_path)?;
    repo.remote(name, url).context("Failed to add remote")?;
    Ok(())
}

/// Remove a remote
pub fn remove_remote(repo_path: impl AsRef<Path>, name: &str) -> Result<()> {
    let repo = open_repo(repo_path)?;
    repo.remote_delete(name)
        .context("Failed to remove remote")?;
    Ok(())
}

/// Fetch from a remote
pub fn fetch(repo_path: impl AsRef<Path>, remote_name: &str) -> Result<()> {
    let repo = open_repo(repo_path)?;
    let mut remote = repo.find_remote(remote_name).context("Remote not found")?;

    // Fetch with default refspecs
    remote
        .fetch(&[] as &[&str], None, None)
        .context("Failed to fetch")?;

    Ok(())
}

/// Pull from a remote branch
pub fn pull(repo_path: impl AsRef<Path>, remote_name: &str, branch_name: &str) -> Result<()> {
    let repo = open_repo(&repo_path)?;

    // Fetch first
    fetch(&repo_path, remote_name)?;

    // Get the remote branch reference
    let fetch_head = repo
        .find_reference("FETCH_HEAD")
        .context("FETCH_HEAD not found")?;
    let fetch_commit = repo.reference_to_annotated_commit(&fetch_head)?;

    // Perform merge
    let analysis = repo.merge_analysis(&[&fetch_commit])?;

    if analysis.0.is_up_to_date() {
        return Ok(());
    } else if analysis.0.is_fast_forward() {
        // Fast-forward merge
        let refname = format!("refs/heads/{}", branch_name);
        let mut reference = repo.find_reference(&refname)?;
        reference.set_target(fetch_commit.id(), "Fast-forward")?;
        repo.set_head(&refname)?;
        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
    } else {
        // Normal merge
        repo.merge(&[&fetch_commit], None, None)?;
    }

    Ok(())
}

/// Push to a remote
pub fn push(repo_path: impl AsRef<Path>, remote_name: &str, refspecs: &[&str]) -> Result<()> {
    let repo = open_repo(repo_path)?;
    let mut remote = repo.find_remote(remote_name).context("Remote not found")?;

    remote.push(refspecs, None).context("Failed to push")?;

    Ok(())
}

// ===== Tag Operations =====

/// List all tags
pub fn list_tags(repo_path: impl AsRef<Path>) -> Result<Vec<GitTag>> {
    let repo = open_repo(repo_path)?;
    let tag_names = repo.tag_names(None)?;

    let mut results = Vec::new();

    for tag_name in tag_names.iter().flatten() {
        let reference = repo.find_reference(&format!("refs/tags/{}", tag_name))?;

        if let Some(target_oid) = reference.target() {
            // Check if it's an annotated tag
            if let Ok(tag_obj) = repo.find_tag(target_oid) {
                // Annotated tag
                results.push(GitTag {
                    name: tag_name.to_string(),
                    commit_id: tag_obj.target_id().to_string(),
                    message: Some(tag_obj.message().unwrap_or("").to_string()),
                    tagger: tag_obj.tagger().and_then(|t| t.name().map(String::from)),
                    date: tag_obj.tagger().map(|t| t.when().seconds() as u64),
                });
            } else {
                // Lightweight tag
                results.push(GitTag {
                    name: tag_name.to_string(),
                    commit_id: target_oid.to_string(),
                    message: None,
                    tagger: None,
                    date: None,
                });
            }
        }
    }

    Ok(results)
}

/// Create a lightweight tag
pub fn create_tag(
    repo_path: impl AsRef<Path>,
    tag_name: &str,
    commit_id: Option<&str>,
) -> Result<()> {
    let repo = open_repo(repo_path)?;

    let target_oid = if let Some(id) = commit_id {
        git2::Oid::from_str(id)?
    } else {
        repo.head()?.target().context("Failed to get HEAD target")?
    };

    let target = repo.find_object(target_oid, None)?;

    repo.tag_lightweight(tag_name, &target, false)
        .context("Failed to create tag")?;

    Ok(())
}

/// Create an annotated tag
pub fn create_annotated_tag(
    repo_path: impl AsRef<Path>,
    tag_name: &str,
    message: &str,
    commit_id: Option<&str>,
) -> Result<()> {
    let repo = open_repo(repo_path)?;

    let target_oid = if let Some(id) = commit_id {
        git2::Oid::from_str(id)?
    } else {
        repo.head()?.target().context("Failed to get HEAD target")?
    };

    let target = repo.find_object(target_oid, None)?;
    let signature = repo.signature()?;

    repo.tag(tag_name, &target, &signature, message, false)
        .context("Failed to create annotated tag")?;

    Ok(())
}

/// Delete a tag
pub fn delete_tag(repo_path: impl AsRef<Path>, tag_name: &str) -> Result<()> {
    let repo = open_repo(repo_path)?;
    repo.tag_delete(tag_name).context("Failed to delete tag")?;
    Ok(())
}

// ===== Stash Operations =====

/// List all stashes
pub fn list_stashes(repo_path: impl AsRef<Path>) -> Result<Vec<GitStash>> {
    let mut repo = open_repo(repo_path)?;

    let mut results = Vec::new();

    repo.stash_foreach(|idx, message, oid| {
        results.push(GitStash {
            index: idx,
            message: message.to_string(),
            commit_id: oid.to_string(),
            date: 0, // git2 doesn't provide stash date easily
        });
        true
    })?;

    Ok(results)
}

/// Save a stash
pub fn stash_save(
    repo_path: impl AsRef<Path>,
    message: Option<&str>,
    include_untracked: bool,
) -> Result<()> {
    let mut repo = open_repo(repo_path)?;
    let signature = repo.signature()?;

    let mut flags = git2::StashFlags::DEFAULT;
    if include_untracked {
        flags |= git2::StashFlags::INCLUDE_UNTRACKED;
    }

    repo.stash_save(&signature, message.unwrap_or(""), Some(flags))
        .context("Failed to save stash")?;

    Ok(())
}

/// Apply a stash
pub fn stash_apply(repo_path: impl AsRef<Path>, index: usize) -> Result<()> {
    let mut repo = open_repo(repo_path)?;

    let mut options = git2::StashApplyOptions::new();
    repo.stash_apply(index, Some(&mut options))
        .context("Failed to apply stash")?;

    Ok(())
}

/// Pop a stash (apply and remove)
pub fn stash_pop(repo_path: impl AsRef<Path>, index: usize) -> Result<()> {
    let mut repo = open_repo(repo_path)?;

    let mut options = git2::StashApplyOptions::new();
    repo.stash_pop(index, Some(&mut options))
        .context("Failed to pop stash")?;

    Ok(())
}

/// Drop a stash
pub fn stash_drop(repo_path: impl AsRef<Path>, index: usize) -> Result<()> {
    let mut repo = open_repo(repo_path)?;
    repo.stash_drop(index).context("Failed to drop stash")?;
    Ok(())
}

// ===== Advanced Branch Operations =====

/// List remote branches
pub fn list_remote_branches(repo_path: impl AsRef<Path>) -> Result<Vec<GitBranch>> {
    let repo = open_repo(repo_path)?;
    let branches = repo.branches(Some(git2::BranchType::Remote))?;

    let mut results = Vec::new();

    for branch in branches {
        let (branch, _) = branch?;
        let name = branch.name()?.unwrap_or("").to_string();
        let is_current = branch.is_head();

        results.push(GitBranch { name, is_current });
    }

    Ok(results)
}

/// Merge a branch into the current branch
pub fn merge_branch(repo_path: impl AsRef<Path>, branch_name: &str) -> Result<()> {
    let repo = open_repo(repo_path)?;

    // Find the branch
    let branch = repo.find_branch(branch_name, git2::BranchType::Local)?;
    let branch_ref = branch.get();
    let annotated_commit = repo.reference_to_annotated_commit(branch_ref)?;

    // Perform merge analysis
    let analysis = repo.merge_analysis(&[&annotated_commit])?;

    if analysis.0.is_up_to_date() {
        return Ok(());
    } else if analysis.0.is_fast_forward() {
        // Fast-forward merge
        let refname = "HEAD";
        let mut reference = repo.find_reference(refname)?;
        reference.set_target(annotated_commit.id(), "Fast-forward")?;
        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
    } else {
        // Normal merge
        repo.merge(&[&annotated_commit], None, None)?;

        // Check for conflicts
        let mut index = repo.index()?;
        if index.has_conflicts() {
            anyhow::bail!("Merge conflicts detected. Please resolve conflicts manually.");
        }

        // Create merge commit
        let signature = repo.signature()?;
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;
        let head = repo.head()?;
        let head_commit = head.peel_to_commit()?;
        let merge_commit = repo.find_commit(annotated_commit.id())?;

        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            &format!("Merge branch '{}'", branch_name),
            &tree,
            &[&head_commit, &merge_commit],
        )?;

        // Cleanup merge state
        repo.cleanup_state()?;
    }

    Ok(())
}

// ===== Line-level diff for gutter markers =====

#[derive(Debug, Clone)]
pub struct LineChange {
    pub line: usize, // 0-indexed line number in the current file
    pub change_type: LineChangeType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LineChangeType {
    Added,
    Modified,
    Deleted,
}

/// Get line-level diff between HEAD and working copy for a file (for gutter markers)
pub fn get_line_changes(root_path: &str, file_path: &str) -> Result<Vec<LineChange>> {
    let repo = Repository::open(root_path)
        .or_else(|_| Repository::discover(root_path))
        .context("Failed to open git repository")?;

    // Get relative path
    let relative = file_path
        .strip_prefix(root_path)
        .unwrap_or(file_path)
        .trim_start_matches('/');

    let head_tree = match repo.head() {
        Ok(head) => match head.peel_to_tree() {
            Ok(tree) => tree,
            Err(_) => {
                // No commits yet, all lines are Added
                let content = std::fs::read_to_string(file_path)?;
                let count = content.lines().count();
                return Ok((0..count)
                    .map(|i| LineChange {
                        line: i,
                        change_type: LineChangeType::Added,
                    })
                    .collect());
            }
        },
        Err(_) => {
            let content = std::fs::read_to_string(file_path)?;
            let count = content.lines().count();
            return Ok((0..count)
                .map(|i| LineChange {
                    line: i,
                    change_type: LineChangeType::Added,
                })
                .collect());
        }
    };

    let entry = match head_tree.get_path(Path::new(relative)) {
        Ok(e) => e,
        Err(_) => {
            // File not in HEAD, all lines are Added
            let content = std::fs::read_to_string(file_path)?;
            let count = content.lines().count();
            return Ok((0..count)
                .map(|i| LineChange {
                    line: i,
                    change_type: LineChangeType::Added,
                })
                .collect());
        }
    };

    let blob = repo.find_blob(entry.id())?;
    let old_content = std::str::from_utf8(blob.content()).unwrap_or("");
    let new_content = std::fs::read_to_string(file_path)?;

    let old_lines: Vec<&str> = old_content.lines().collect();
    let new_lines: Vec<&str> = new_content.lines().collect();

    let mut changes = Vec::new();
    let max_len = old_lines.len().max(new_lines.len());

    for i in 0..max_len {
        match (old_lines.get(i), new_lines.get(i)) {
            (Some(old), Some(new)) => {
                if old != new {
                    changes.push(LineChange {
                        line: i,
                        change_type: LineChangeType::Modified,
                    });
                }
            }
            (None, Some(_)) => {
                changes.push(LineChange {
                    line: i,
                    change_type: LineChangeType::Added,
                });
            }
            (Some(_), None) => {
                changes.push(LineChange {
                    line: i.min(new_lines.len().saturating_sub(1)),
                    change_type: LineChangeType::Deleted,
                });
            }
            (None, None) => {}
        }
    }

    Ok(changes)
}

// ===== Inline blame info =====

#[derive(Debug, Clone)]
pub struct BlameInfo {
    pub author: String,
    pub timestamp: i64,
    pub message: String,
}

/// Get blame info for a specific line (0-indexed)
pub fn get_line_blame(root_path: &str, file_path: &str, line: usize) -> Result<Option<BlameInfo>> {
    let repo = Repository::open(root_path)
        .or_else(|_| Repository::discover(root_path))
        .context("Failed to open git repository")?;

    let relative = file_path
        .strip_prefix(root_path)
        .unwrap_or(file_path)
        .trim_start_matches('/');

    let blame = repo
        .blame_file(Path::new(relative), None)
        .context("Failed to get blame")?;

    // git blame get_line is 1-indexed
    if let Some(hunk) = blame.get_line(line + 1) {
        let sig = hunk.final_signature();
        let name = sig.name().unwrap_or("unknown").to_string();
        let time = sig.when();
        let commit_id = hunk.final_commit_id();

        let message = repo
            .find_commit(commit_id)
            .map(|c| c.summary().unwrap_or("").to_string())
            .unwrap_or_default();

        Ok(Some(BlameInfo {
            author: name,
            timestamp: time.seconds(),
            message,
        }))
    } else {
        Ok(None)
    }
}

/// Rebase current branch onto target branch
pub fn rebase_branch(repo_path: impl AsRef<Path>, target_branch: &str) -> Result<()> {
    let repo = open_repo(repo_path)?;

    // Find the target branch
    let branch = repo.find_branch(target_branch, git2::BranchType::Local)?;
    let branch_ref = branch.get();
    let target_annotated = repo.reference_to_annotated_commit(branch_ref)?;

    // Get current HEAD as annotated commit
    let head_ref = repo.head()?;
    let head_annotated = repo.reference_to_annotated_commit(&head_ref)?;

    // Perform rebase
    let mut rebase = repo.rebase(Some(&head_annotated), Some(&target_annotated), None, None)?;

    // Iterate through rebase operations
    while let Some(op) = rebase.next() {
        let _op = op?;
        // Commit the rebased operation
        let signature = repo.signature()?;
        rebase.commit(None, &signature, None)?;
    }

    // Finish rebase
    rebase.finish(None)?;

    Ok(())
}
