//! Git types for serialization

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileStatus {
    pub path: String,
    pub status: String,
    pub is_staged: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchInfo {
    pub name: String,
    pub is_head: bool,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    pub hash: String,
    pub short_hash: String,
    pub message: String,
    pub author: String,
    pub email: String,
    pub timestamp: i64,
    pub parents: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitDetail {
    pub info: CommitInfo,
    pub files: Vec<FileDiff>,
    pub stats: DiffStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    pub path: String,
    pub old_path: Option<String>,
    pub status: String,
    pub hunks: Vec<DiffHunk>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffHunk {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffLine {
    pub line_type: String,
    pub content: String,
    pub old_line_no: Option<u32>,
    pub new_line_no: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlameLine {
    pub line_no: u32,
    pub commit_hash: String,
    pub author: String,
    pub timestamp: i64,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteInfo {
    pub name: String,
    pub url: String,
    pub fetch_url: Option<String>,
    pub push_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffStats {
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_status_creation() {
        let status = FileStatus {
            path: "test.rs".to_string(),
            status: "M".to_string(),
            is_staged: false,
        };
        assert_eq!(status.path, "test.rs");
        assert_eq!(status.status, "M");
        assert!(!status.is_staged);
    }

    #[test]
    fn test_branch_info_creation() {
        let branch = BranchInfo {
            name: "main".to_string(),
            is_head: true,
            upstream: Some("origin/main".to_string()),
            ahead: 0,
            behind: 0,
        };
        assert!(branch.is_head);
        assert_eq!(branch.name, "main");
    }

    #[test]
    fn test_commit_info_creation() {
        let commit = CommitInfo {
            hash: "abc123".to_string(),
            short_hash: "abc".to_string(),
            message: "Initial commit".to_string(),
            author: "Test User".to_string(),
            email: "test@example.com".to_string(),
            timestamp: 1234567890,
            parents: vec![],
        };
        assert_eq!(commit.hash, "abc123");
        assert!(commit.parents.is_empty());
    }
}
