//! Git repository handling

use git2::{Repository, StatusOptions, DiffOptions, Commit, Oid};
use std::path::{Path, PathBuf};
use crate::berrycode::Result;
use crate::berrycode::exceptions::AiderError;

pub struct GitRepo {
    repo: Repository,
    root: PathBuf,
}

impl GitRepo {
    /// Open or create a Git repository
    pub fn new(path: Option<&Path>) -> Result<Self> {
        let repo = if let Some(path) = path {
            Repository::open(path)
                .map_err(|e| AiderError::GitError(format!("Failed to open repo: {}", e)))?
        } else {
            Repository::open_from_env()
                .map_err(|e| AiderError::GitError(format!("Failed to open repo: {}", e)))?
        };

        let root = repo
            .workdir()
            .ok_or_else(|| AiderError::GitError("No working directory found".to_string()))?
            .to_path_buf();

        Ok(Self { repo, root })
    }

    /// Initialize a new Git repository
    pub fn init(path: &Path) -> Result<Self> {
        let repo = Repository::init(path)
            .map_err(|e| AiderError::GitError(format!("Failed to init repo: {}", e)))?;

        let root = repo
            .workdir()
            .ok_or_else(|| AiderError::GitError("No working directory found".to_string()))?
            .to_path_buf();

        Ok(Self { repo, root })
    }

    /// Get the repository root directory
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Check if a path is ignored by git
    pub fn is_ignored(&self, path: &Path) -> Result<bool> {
        Ok(self.repo.is_path_ignored(path)?)
    }

    /// Get the status of files in the repository
    pub fn status(&self) -> Result<Vec<(String, String)>> {
        let mut opts = StatusOptions::new();
        opts.include_untracked(true);
        opts.recurse_untracked_dirs(true);

        let statuses = self.repo.statuses(Some(&mut opts))?;
        let mut result = Vec::new();

        for entry in statuses.iter() {
            if let Some(path) = entry.path() {
                let status = entry.status();
                let status_str = if status.is_index_new() || status.is_wt_new() {
                    "new"
                } else if status.is_index_modified() || status.is_wt_modified() {
                    "modified"
                } else if status.is_index_deleted() || status.is_wt_deleted() {
                    "deleted"
                } else {
                    "unknown"
                };

                result.push((path.to_string(), status_str.to_string()));
            }
        }

        Ok(result)
    }

    /// Add files to the git index
    pub fn add(&self, paths: &[&Path]) -> Result<()> {
        let mut index = self.repo.index()?;

        for path in paths {
            let rel_path = path.strip_prefix(&self.root).unwrap_or(path);
            index.add_path(rel_path)?;
        }

        index.write()?;
        Ok(())
    }

    /// Create a commit
    pub fn commit(&self, message: &str) -> Result<Oid> {
        let mut index = self.repo.index()?;
        let tree_id = index.write_tree()?;
        let tree = self.repo.find_tree(tree_id)?;

        let signature = self.repo.signature()?;

        let parent_commit = self.get_head_commit_obj()?;
        let parents = if let Some(ref commit) = parent_commit {
            vec![commit]
        } else {
            vec![]
        };

        let oid = self.repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &parents,
        )?;

        Ok(oid)
    }

    /// Get the HEAD commit
    fn get_head_commit_obj(&self) -> Result<Option<Commit<'_>>> {
        match self.repo.head() {
            Ok(head) => {
                let commit = head.peel_to_commit()?;
                Ok(Some(commit))
            }
            Err(e) if e.code() == git2::ErrorCode::UnbornBranch => Ok(None),
            Err(e) => Err(AiderError::GitError(format!("Failed to get HEAD: {}", e)).into()),
        }
    }

    /// Get the diff of uncommitted changes (staged + unstaged)
    pub fn diff(&self) -> Result<String> {
        use git2::DiffFormat;

        let mut diff_opts = DiffOptions::new();
        diff_opts.context_lines(3);
        diff_opts.include_untracked(false);

        let mut combined_diff = String::new();

        // Get HEAD tree
        let head_tree = match self.repo.head() {
            Ok(head_ref) => {
                tracing::debug!("HEAD reference found");
                let commit = head_ref.peel_to_commit()?;
                Some(commit.tree()?)
            }
            Err(e) => {
                tracing::debug!("No HEAD reference: {} (new repository?)", e);
                None
            }
        };

        if let Some(tree) = head_tree {
            // Get index
            let index = self.repo.index()?;

            // 1. Staged changes: diff between HEAD tree and index
            tracing::debug!("Getting staged changes (HEAD -> index)");
            let staged_diff = self.repo.diff_tree_to_index(
                Some(&tree),
                Some(&index),
                Some(&mut diff_opts)
            )?;

            let staged_stats = staged_diff.stats()?;
            tracing::debug!(
                "Staged: {} files, +{} -{} ",
                staged_stats.files_changed(),
                staged_stats.insertions(),
                staged_stats.deletions()
            );

            staged_diff.print(DiffFormat::Patch, |_delta, _hunk, line| {
                let content = String::from_utf8_lossy(line.content());
                combined_diff.push_str(&content);
                true
            })?;

            // 2. Unstaged changes: diff between index and working directory
            tracing::debug!("Getting unstaged changes (index -> workdir)");
            let unstaged_diff = self.repo.diff_index_to_workdir(
                Some(&index),
                Some(&mut diff_opts)
            )?;

            let unstaged_stats = unstaged_diff.stats()?;
            tracing::debug!(
                "Unstaged: {} files, +{} -{} ",
                unstaged_stats.files_changed(),
                unstaged_stats.insertions(),
                unstaged_stats.deletions()
            );

            unstaged_diff.print(DiffFormat::Patch, |_delta, _hunk, line| {
                let content = String::from_utf8_lossy(line.content());
                combined_diff.push_str(&content);
                true
            })?;
        } else {
            // No HEAD - show all tracked files as new
            tracing::debug!("No HEAD, showing all tracked files");
            let diff = self.repo.diff_tree_to_workdir_with_index(None, Some(&mut diff_opts))?;

            diff.print(DiffFormat::Patch, |_delta, _hunk, line| {
                let content = String::from_utf8_lossy(line.content());
                combined_diff.push_str(&content);
                true
            })?;
        }

        tracing::debug!("Total diff text: {} bytes", combined_diff.len());
        Ok(combined_diff)
    }

    /// Check if the working directory is dirty
    pub fn is_dirty(&self) -> Result<bool> {
        let statuses = self.repo.statuses(None)?;
        Ok(!statuses.is_empty())
    }

    /// Get the current branch name
    pub fn current_branch(&self) -> Result<String> {
        let head = self.repo.head()?;
        if let Some(name) = head.shorthand() {
            Ok(name.to_string())
        } else {
            Err(AiderError::GitError("Failed to get branch name".to_string()).into())
        }
    }

    /// Get git config value
    pub fn get_config(&self, key: &str) -> Result<Option<String>> {
        let config = self.repo.config()?;
        match config.get_string(key) {
            Ok(value) => Ok(Some(value)),
            Err(_) => Ok(None),
        }
    }

    /// Set git config value
    pub fn set_config(&self, key: &str, value: &str) -> Result<()> {
        let mut config = self.repo.config()?;
        config.set_str(key, value)?;
        Ok(())
    }

    /// Reset to a specific commit (undo commits)
    pub fn reset_to_commit(&self, commit_oid: Oid, reset_type: git2::ResetType) -> Result<()> {
        let commit = self.repo.find_commit(commit_oid)?;
        let object = commit.as_object();
        self.repo.reset(object, reset_type, None)?;
        Ok(())
    }

    /// Get the previous commit (HEAD~1)
    pub fn get_previous_commit(&self) -> Result<Oid> {
        let head = self.repo.head()?;
        let commit = head.peel_to_commit()?;

        if commit.parent_count() == 0 {
            return Err(anyhow::anyhow!("No previous commit (this is the initial commit)"));
        }

        let parent = commit.parent(0)?;
        Ok(parent.id())
    }

    /// Get HEAD commit
    pub fn get_head_commit(&self) -> Result<Oid> {
        let head = self.repo.head()?;
        let commit = head.peel_to_commit()?;
        Ok(commit.id())
    }

    /// Check if repository has uncommitted changes
    pub fn has_changes(&self) -> Result<bool> {
        let statuses = self.repo.statuses(None)?;
        Ok(!statuses.is_empty())
    }
}

/// Find git repository starting from current directory
pub fn find_git_root(start_path: Option<&Path>) -> Result<Option<PathBuf>> {
    let search_path = start_path.unwrap_or_else(|| Path::new("."));

    match Repository::discover(search_path) {
        Ok(repo) => {
            if let Some(workdir) = repo.workdir() {
                Ok(Some(workdir.to_path_buf()))
            } else {
                Ok(None)
            }
        }
        Err(_) => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_find_git_root_current_repo() {
        // This project is a git repo, so it should find it
        let result = find_git_root(Some(Path::new(".")));
        assert!(result.is_ok());
        let root = result.unwrap();
        assert!(root.is_some());
    }

    #[test]
    fn test_find_git_root_non_repo() {
        // Create a temporary directory that's not a git repo
        let temp_dir = TempDir::new().unwrap();
        let result = find_git_root(Some(temp_dir.path()));
        // Should succeed but return None or find parent repo
        assert!(result.is_ok());
    }

    #[test]
    fn test_gitrepo_new() {
        // Open the current git repository
        let result = GitRepo::new(Some(Path::new(".")));
        assert!(result.is_ok());
        let repo = result.unwrap();
        assert!(repo.root().exists());
    }

    #[test]
    fn test_gitrepo_root() {
        let repo = GitRepo::new(Some(Path::new("."))).unwrap();
        let root = repo.root();
        assert!(root.is_dir());
        assert!(root.join(".git").exists());
    }

    #[test]
    fn test_gitrepo_init() {
        let temp_dir = TempDir::new().unwrap();
        let result = GitRepo::init(temp_dir.path());
        assert!(result.is_ok());

        let repo = result.unwrap();
        assert!(repo.root().exists());
        assert!(repo.root().join(".git").exists());
    }

    #[test]
    fn test_gitrepo_is_ignored() {
        let repo = GitRepo::new(Some(Path::new("."))).unwrap();

        // .gitignore should be tracked (not ignored)
        let gitignore_path = repo.root().join(".gitignore");
        if gitignore_path.exists() {
            let result = repo.is_ignored(&gitignore_path);
            // Result depends on repo configuration, just verify it doesn't panic
            assert!(result.is_ok());
        }

        // target/ should be ignored (if in .gitignore)
        let target_path = repo.root().join("target");
        if target_path.exists() {
            let result = repo.is_ignored(&target_path);
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_gitrepo_status() {
        let repo = GitRepo::new(Some(Path::new("."))).unwrap();
        let result = repo.status();
        assert!(result.is_ok());

        // Status returns a list of (path, status) tuples
        let statuses = result.unwrap();
        // Just verify the format is correct
        for (path, status) in statuses {
            assert!(!path.is_empty());
            assert!(["new", "modified", "deleted", "unknown"].contains(&status.as_str()));
        }
    }

    #[test]
    fn test_gitrepo_has_changes() {
        let repo = GitRepo::new(Some(Path::new("."))).unwrap();
        let result = repo.has_changes();
        assert!(result.is_ok());
        // The result depends on the current state of the repo
        let _has_changes = result.unwrap();
    }

    #[test]
    fn test_gitrepo_is_dirty() {
        let repo = GitRepo::new(Some(Path::new("."))).unwrap();
        let result = repo.is_dirty();
        assert!(result.is_ok());
        // The result depends on the current state of the repo
        let _is_dirty = result.unwrap();
    }

    #[test]
    fn test_gitrepo_current_branch() {
        let repo = GitRepo::new(Some(Path::new("."))).unwrap();
        let result = repo.current_branch();
        assert!(result.is_ok());

        let branch = result.unwrap();
        assert!(!branch.is_empty());
    }

    #[test]
    fn test_gitrepo_config() {
        let repo = GitRepo::new(Some(Path::new("."))).unwrap();

        // Try to get user.name config
        let result = repo.get_config("user.name");
        assert!(result.is_ok());
        // Value may or may not exist, just verify it doesn't panic
        let _value = result.unwrap();
    }

    #[test]
    fn test_gitrepo_diff() {
        let repo = GitRepo::new(Some(Path::new("."))).unwrap();
        let result = repo.diff();
        assert!(result.is_ok());
        // Diff may be empty or have content depending on repo state
        let _diff_text = result.unwrap();
    }

    #[test]
    fn test_gitrepo_add_and_operations() {
        // Create a temporary git repo for testing add/commit operations
        let temp_dir = TempDir::new().unwrap();
        let repo = GitRepo::init(temp_dir.path()).unwrap();

        // Set git config for the test repo
        let _ = repo.set_config("user.name", "Test User");
        let _ = repo.set_config("user.email", "test@example.com");

        // Create a test file
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "test content").unwrap();

        // Add the file - use relative path from repo root
        let relative_path = test_file.strip_prefix(temp_dir.path()).unwrap();
        let result = repo.add(&[relative_path]);

        if let Err(e) = &result {
            eprintln!("Add error: {:?}", e);
        }
        assert!(result.is_ok());

        // Commit the file
        let commit_result = repo.commit("Test commit");

        if let Err(e) = &commit_result {
            eprintln!("Commit error: {:?}", e);
        }
        assert!(commit_result.is_ok());

        // Get HEAD commit
        let head_result = repo.get_head_commit();
        assert!(head_result.is_ok());
    }

    #[test]
    fn test_gitrepo_set_config() {
        let temp_dir = TempDir::new().unwrap();
        let repo = GitRepo::init(temp_dir.path()).unwrap();

        // Set a test config value
        let result = repo.set_config("test.value", "test123");
        assert!(result.is_ok());

        // Get it back
        let get_result = repo.get_config("test.value");
        assert!(get_result.is_ok());
        assert_eq!(get_result.unwrap(), Some("test123".to_string()));
    }

    #[test]
    fn test_gitrepo_diff_unstaged_changes() {
        // Create a temporary repository
        let temp_dir = TempDir::new().unwrap();
        let repo = GitRepo::init(temp_dir.path()).unwrap();

        // Configure git user (required for commits)
        repo.set_config("user.name", "Test User").unwrap();
        repo.set_config("user.email", "test@example.com").unwrap();

        // Create a file and make initial commit
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "initial content\n").unwrap();

        // Stage and commit
        repo.repo.index().unwrap().add_path(Path::new("test.txt")).unwrap();
        repo.repo.index().unwrap().write().unwrap();

        let tree_id = repo.repo.index().unwrap().write_tree().unwrap();
        let tree = repo.repo.find_tree(tree_id).unwrap();
        let sig = git2::Signature::now("Test User", "test@example.com").unwrap();
        repo.repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            "Initial commit",
            &tree,
            &[]
        ).unwrap();

        // Modify the file (unstaged change)
        fs::write(&test_file, "initial content\nmodified line\n").unwrap();

        // Get diff
        let diff_result = repo.diff();
        assert!(diff_result.is_ok());

        let diff_text = diff_result.unwrap();
        assert!(!diff_text.is_empty(), "Diff should not be empty for unstaged changes");
        assert!(diff_text.contains("test.txt"), "Diff should mention the changed file");
        assert!(diff_text.contains("modified line"), "Diff should show added line");
    }

    #[test]
    fn test_gitrepo_diff_staged_changes() {
        // Create a temporary repository
        let temp_dir = TempDir::new().unwrap();
        let repo = GitRepo::init(temp_dir.path()).unwrap();

        // Configure git user
        repo.set_config("user.name", "Test User").unwrap();
        repo.set_config("user.email", "test@example.com").unwrap();

        // Create a file and make initial commit
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "initial content\n").unwrap();

        repo.repo.index().unwrap().add_path(Path::new("test.txt")).unwrap();
        repo.repo.index().unwrap().write().unwrap();

        let tree_id = repo.repo.index().unwrap().write_tree().unwrap();
        let tree = repo.repo.find_tree(tree_id).unwrap();
        let sig = git2::Signature::now("Test User", "test@example.com").unwrap();
        repo.repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            "Initial commit",
            &tree,
            &[]
        ).unwrap();

        // Modify and stage the file
        fs::write(&test_file, "initial content\nstaged modification\n").unwrap();
        repo.repo.index().unwrap().add_path(Path::new("test.txt")).unwrap();
        repo.repo.index().unwrap().write().unwrap();

        // Get diff (should show staged changes)
        let diff_result = repo.diff();
        assert!(diff_result.is_ok());

        let diff_text = diff_result.unwrap();
        assert!(!diff_text.is_empty(), "Diff should not be empty for staged changes");
        assert!(diff_text.contains("test.txt"), "Diff should mention the changed file");
        assert!(diff_text.contains("staged modification"), "Diff should show staged line");
    }

    #[test]
    fn test_gitrepo_diff_mixed_changes() {
        // Create a temporary repository
        let temp_dir = TempDir::new().unwrap();
        let repo = GitRepo::init(temp_dir.path()).unwrap();

        // Configure git user
        repo.set_config("user.name", "Test User").unwrap();
        repo.set_config("user.email", "test@example.com").unwrap();

        // Create a file and make initial commit
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "line 1\nline 2\nline 3\n").unwrap();

        repo.repo.index().unwrap().add_path(Path::new("test.txt")).unwrap();
        repo.repo.index().unwrap().write().unwrap();

        let tree_id = repo.repo.index().unwrap().write_tree().unwrap();
        let tree = repo.repo.find_tree(tree_id).unwrap();
        let sig = git2::Signature::now("Test User", "test@example.com").unwrap();
        repo.repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            "Initial commit",
            &tree,
            &[]
        ).unwrap();

        // Make a staged change
        fs::write(&test_file, "line 1\nstaged change\nline 3\n").unwrap();
        repo.repo.index().unwrap().add_path(Path::new("test.txt")).unwrap();
        repo.repo.index().unwrap().write().unwrap();

        // Make an unstaged change
        fs::write(&test_file, "line 1\nstaged change\nline 3\nunstaged addition\n").unwrap();

        // Get diff (should show both staged and unstaged changes)
        let diff_result = repo.diff();
        assert!(diff_result.is_ok());

        let diff_text = diff_result.unwrap();
        assert!(!diff_text.is_empty(), "Diff should not be empty");
        assert!(diff_text.contains("test.txt"), "Diff should mention the file");

        // Should contain both staged and unstaged changes
        assert!(
            diff_text.contains("staged change") || diff_text.contains("-line 2"),
            "Diff should show staged changes"
        );
        assert!(
            diff_text.contains("unstaged addition"),
            "Diff should show unstaged changes"
        );
    }

    #[test]
    fn test_gitrepo_diff_no_changes() {
        // Create a temporary repository
        let temp_dir = TempDir::new().unwrap();
        let repo = GitRepo::init(temp_dir.path()).unwrap();

        // Configure git user
        repo.set_config("user.name", "Test User").unwrap();
        repo.set_config("user.email", "test@example.com").unwrap();

        // Create a file and commit
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "content\n").unwrap();

        repo.repo.index().unwrap().add_path(Path::new("test.txt")).unwrap();
        repo.repo.index().unwrap().write().unwrap();

        let tree_id = repo.repo.index().unwrap().write_tree().unwrap();
        let tree = repo.repo.find_tree(tree_id).unwrap();
        let sig = git2::Signature::now("Test User", "test@example.com").unwrap();
        repo.repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            "Initial commit",
            &tree,
            &[]
        ).unwrap();

        // Get diff (should be empty - no changes)
        let diff_result = repo.diff();
        assert!(diff_result.is_ok());

        let diff_text = diff_result.unwrap();
        assert!(diff_text.is_empty(), "Diff should be empty when there are no changes");
    }
}
