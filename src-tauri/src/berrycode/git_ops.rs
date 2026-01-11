//! Git Operations - Git統合機能
//! BerryFlowの"Git-Native"設計を実現

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use serde::{Deserialize, Serialize};

/// Git操作の結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

/// コミット情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    pub hash: String,
    pub message: String,
    pub author: String,
    pub timestamp: String,
    pub files_changed: Vec<String>,
}

/// ブランチ情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchInfo {
    pub name: String,
    pub is_current: bool,
    pub upstream: Option<String>,
    pub ahead: usize,
    pub behind: usize,
}

/// Git差分情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffInfo {
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
    pub diff: String,
}

pub struct GitOps {
    repo_path: PathBuf,
}

impl GitOps {
    /// 新しいGitOpsインスタンスを作成
    pub fn new(repo_path: PathBuf) -> Result<Self> {
        if !repo_path.join(".git").exists() {
            anyhow::bail!("Not a git repository: {:?}", repo_path);
        }
        Ok(Self { repo_path })
    }

    /// Gitコマンドを実行
    fn run_git(&self, args: &[&str]) -> Result<GitResult> {
        let output = Command::new("git")
            .current_dir(&self.repo_path)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .context("Failed to execute git command")?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(GitResult {
            success: output.status.success(),
            output: stdout,
            error: if stderr.is_empty() { None } else { Some(stderr) },
        })
    }

    /// 現在のブランチ名を取得
    pub fn current_branch(&self) -> Result<String> {
        let result = self.run_git(&["rev-parse", "--abbrev-ref", "HEAD"])?;
        if !result.success {
            anyhow::bail!("Failed to get current branch: {:?}", result.error);
        }
        Ok(result.output.trim().to_string())
    }

    /// リポジトリがクリーンか確認（未コミットの変更がないか）
    pub fn is_clean(&self) -> Result<bool> {
        let result = self.run_git(&["status", "--porcelain"])?;
        Ok(result.success && result.output.trim().is_empty())
    }

    /// 変更されたファイルのリストを取得
    pub fn get_changed_files(&self) -> Result<Vec<String>> {
        let result = self.run_git(&["status", "--porcelain"])?;
        if !result.success {
            anyhow::bail!("Failed to get changed files: {:?}", result.error);
        }

        let files: Vec<String> = result
            .output
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    Some(parts[1].to_string())
                } else {
                    None
                }
            })
            .collect();

        Ok(files)
    }

    /// 差分情報を取得
    pub fn get_diff(&self, staged: bool) -> Result<DiffInfo> {
        let args = if staged {
            vec!["diff", "--cached", "--stat"]
        } else {
            vec!["diff", "--stat"]
        };

        let stat_result = self.run_git(&args)?;
        if !stat_result.success {
            anyhow::bail!("Failed to get diff stats: {:?}", stat_result.error);
        }

        // 詳細な差分を取得
        let diff_args = if staged {
            vec!["diff", "--cached"]
        } else {
            vec!["diff"]
        };
        let diff_result = self.run_git(&diff_args)?;

        // 統計情報をパース
        let (files_changed, insertions, deletions) = Self::parse_diff_stats(&stat_result.output);

        Ok(DiffInfo {
            files_changed,
            insertions,
            deletions,
            diff: diff_result.output,
        })
    }

    /// 差分統計をパース
    fn parse_diff_stats(output: &str) -> (usize, usize, usize) {
        let mut files = 0;
        let mut insertions = 0;
        let mut deletions = 0;

        for line in output.lines() {
            if line.contains("file") && line.contains("changed") {
                // "3 files changed, 45 insertions(+), 12 deletions(-)" のような行をパース
                let parts: Vec<&str> = line.split(',').collect();
                for part in parts {
                    if part.contains("file") {
                        if let Some(num) = part.split_whitespace().next() {
                            files = num.parse().unwrap_or(0);
                        }
                    } else if part.contains("insertion") {
                        if let Some(num) = part.split_whitespace().next() {
                            insertions = num.parse().unwrap_or(0);
                        }
                    } else if part.contains("deletion") {
                        if let Some(num) = part.split_whitespace().next() {
                            deletions = num.parse().unwrap_or(0);
                        }
                    }
                }
            }
        }

        (files, insertions, deletions)
    }

    /// ファイルをステージングエリアに追加
    pub fn stage_files(&self, files: &[String]) -> Result<GitResult> {
        let mut args = vec!["add"];
        args.extend(files.iter().map(|s| s.as_str()));
        self.run_git(&args)
    }

    /// すべての変更をステージング
    pub fn stage_all(&self) -> Result<GitResult> {
        self.run_git(&["add", "."])
    }

    /// コミットを作成
    pub fn commit(&self, message: &str) -> Result<CommitInfo> {
        // コミット実行
        let result = self.run_git(&["commit", "-m", message])?;
        if !result.success {
            anyhow::bail!("Failed to commit: {:?}", result.error);
        }

        // 最新のコミット情報を取得
        self.get_last_commit()
    }

    /// 自動コミット（変更を分析してメッセージを生成）
    pub async fn auto_commit(&self, context: &str) -> Result<CommitInfo> {
        // 変更されたファイルを取得
        let changed_files = self.get_changed_files()?;
        if changed_files.is_empty() {
            anyhow::bail!("No changes to commit");
        }

        // 差分を取得
        let diff = self.get_diff(false)?;

        // コミットメッセージを生成（TODO: LLMで生成）
        let commit_message = self.generate_commit_message(&changed_files, &diff, context)?;

        // ステージング
        self.stage_all()?;

        // コミット
        self.commit(&commit_message)
    }

    /// コミットメッセージを生成
    fn generate_commit_message(
        &self,
        changed_files: &[String],
        diff: &DiffInfo,
        context: &str,
    ) -> Result<String> {
        // TODO: LLMを使って差分から意味のあるコミットメッセージを生成
        // 現在は簡易的な実装
        let file_summary = if changed_files.len() <= 3 {
            changed_files.join(", ")
        } else {
            format!("{} files", changed_files.len())
        };

        let message = format!(
            "feat: {}\n\n- Modified: {}\n- {} insertions, {} deletions\n\n{}",
            context, file_summary, diff.insertions, diff.deletions, "🤖 Generated by BerryFlow"
        );

        Ok(message)
    }

    /// 最新のコミット情報を取得
    pub fn get_last_commit(&self) -> Result<CommitInfo> {
        let hash_result = self.run_git(&["rev-parse", "HEAD"])?;
        let msg_result = self.run_git(&["log", "-1", "--pretty=%s"])?;
        let author_result = self.run_git(&["log", "-1", "--pretty=%an <%ae>"])?;
        let time_result = self.run_git(&["log", "-1", "--pretty=%ai"])?;
        let files_result = self.run_git(&["diff-tree", "--no-commit-id", "--name-only", "-r", "HEAD"])?;

        Ok(CommitInfo {
            hash: hash_result.output.trim().to_string(),
            message: msg_result.output.trim().to_string(),
            author: author_result.output.trim().to_string(),
            timestamp: time_result.output.trim().to_string(),
            files_changed: files_result
                .output
                .lines()
                .map(|s| s.to_string())
                .collect(),
        })
    }

    /// 新しいブランチを作成
    pub fn create_branch(&self, branch_name: &str) -> Result<GitResult> {
        self.run_git(&["checkout", "-b", branch_name])
    }

    /// ブランチを切り替え
    pub fn checkout(&self, branch_name: &str) -> Result<GitResult> {
        self.run_git(&["checkout", branch_name])
    }

    /// すべてのブランチ情報を取得
    pub fn list_branches(&self) -> Result<Vec<BranchInfo>> {
        let result = self.run_git(&["branch", "-vv"])?;
        if !result.success {
            anyhow::bail!("Failed to list branches: {:?}", result.error);
        }

        let mut branches = Vec::new();
        for line in result.output.lines() {
            let is_current = line.starts_with('*');
            let line = line.trim_start_matches('*').trim();
            
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            let name = parts[0].to_string();
            
            // upstream情報をパース（[origin/main: ahead 2, behind 1]のような形式）
            let (upstream, ahead, behind) = if let Some(bracket_start) = line.find('[') {
                if let Some(bracket_end) = line.find(']') {
                    let upstream_info = &line[bracket_start + 1..bracket_end];
                    Self::parse_upstream_info(upstream_info)
                } else {
                    (None, 0, 0)
                }
            } else {
                (None, 0, 0)
            };

            branches.push(BranchInfo {
                name,
                is_current,
                upstream,
                ahead,
                behind,
            });
        }

        Ok(branches)
    }

    /// upstream情報をパース
    fn parse_upstream_info(info: &str) -> (Option<String>, usize, usize) {
        let parts: Vec<&str> = info.split(':').collect();
        if parts.is_empty() {
            return (None, 0, 0);
        }

        let upstream = Some(parts[0].trim().to_string());
        let mut ahead = 0;
        let mut behind = 0;

        if parts.len() > 1 {
            let status = parts[1];
            for part in status.split(',') {
                if part.contains("ahead") {
                    if let Some(num_str) = part.split_whitespace().nth(1) {
                        ahead = num_str.parse().unwrap_or(0);
                    }
                } else if part.contains("behind") {
                    if let Some(num_str) = part.split_whitespace().nth(1) {
                        behind = num_str.parse().unwrap_or(0);
                    }
                }
            }
        }

        (upstream, ahead, behind)
    }

    /// リモートにプッシュ
    pub fn push(&self, remote: &str, branch: &str, set_upstream: bool) -> Result<GitResult> {
        let args = if set_upstream {
            vec!["push", "-u", remote, branch]
        } else {
            vec!["push", remote, branch]
        };
        self.run_git(&args)
    }

    /// リモートからプル
    pub fn pull(&self, remote: &str, branch: &str) -> Result<GitResult> {
        self.run_git(&["pull", remote, branch])
    }

    /// GitHub CLIでPRを作成
    pub async fn create_pr(
        &self,
        title: &str,
        body: &str,
        base: Option<&str>,
    ) -> Result<String> {
        let mut args = vec!["pr", "create", "--title", title, "--body", body];
        
        if let Some(base_branch) = base {
            args.push("--base");
            args.push(base_branch);
        }

        let output = Command::new("gh")
            .current_dir(&self.repo_path)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .context("Failed to execute gh command. Is GitHub CLI installed?")?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to create PR: {}", error);
        }

        let pr_url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(pr_url)
    }

    /// ワークフロー完了後の自動Git操作
    /// - 変更をコミット
    /// - ブランチにプッシュ
    /// - オプションでPR作成
    pub async fn workflow_finalize(
        &self,
        workflow_name: &str,
        create_pr: bool,
        pr_base: Option<&str>,
    ) -> Result<String> {
        // 1. 変更をコミット
        let commit_info = self.auto_commit(workflow_name).await?;
        tracing::info!("✅ Committed: {}", commit_info.message);

        // 2. 現在のブランチを取得
        let current_branch = self.current_branch()?;

        // 3. プッシュ
        let push_result = self.push("origin", &current_branch, true)?;
        if !push_result.success {
            tracing::warn!("⚠️ Failed to push: {:?}", push_result.error);
        } else {
            tracing::info!("✅ Pushed to origin/{}", current_branch);
        }

        // 4. PR作成（オプション）
        if create_pr {
            let pr_title = format!("[BerryFlow] {}", workflow_name);
            let pr_body = format!(
                "## Automated by BerryFlow\n\nWorkflow: {}\n\n### Changes\n{}\n\n🤖 This PR was automatically generated by BerryFlow",
                workflow_name,
                commit_info.message
            );

            match self.create_pr(&pr_title, &pr_body, pr_base).await {
                Ok(pr_url) => {
                    tracing::info!("✅ Created PR: {}", pr_url);
                    return Ok(pr_url);
                }
                Err(e) => {
                    tracing::warn!("⚠️ Failed to create PR: {}", e);
                }
            }
        }

        Ok(format!("Committed: {}", commit_info.hash))
    }

    /// リポジトリの統計情報を取得
    pub fn get_stats(&self) -> Result<RepoStats> {
        // コミット数
        let commit_count_result = self.run_git(&["rev-list", "--count", "HEAD"])?;
        let commit_count: usize = commit_count_result
            .output
            .trim()
            .parse()
            .unwrap_or(0);

        // 貢献者数
        let contributors_result = self.run_git(&["shortlog", "-sn", "--all"])?;
        let contributor_count = contributors_result.output.lines().count();

        // ファイル数
        let file_count_result = self.run_git(&["ls-files"])?;
        let file_count = file_count_result.output.lines().count();

        Ok(RepoStats {
            commit_count,
            contributor_count,
            file_count,
        })
    }
}

/// リポジトリ統計
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoStats {
    pub commit_count: usize,
    pub contributor_count: usize,
    pub file_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_diff_stats() {
        let output = "3 files changed, 45 insertions(+), 12 deletions(-)";
        let (files, ins, del) = GitOps::parse_diff_stats(output);
        assert_eq!(files, 3);
        assert_eq!(ins, 45);
        assert_eq!(del, 12);
    }

    #[test]
    fn test_parse_upstream_info() {
        let info = "origin/main: ahead 2, behind 1";
        let (upstream, ahead, behind) = GitOps::parse_upstream_info(info);
        assert_eq!(upstream, Some("origin/main".to_string()));
        assert_eq!(ahead, 2);
        assert_eq!(behind, 1);
    }

    #[test]
    fn test_parse_upstream_info_no_ahead_behind() {
        let info = "origin/main";
        let (upstream, ahead, behind) = GitOps::parse_upstream_info(info);
        assert_eq!(upstream, Some("origin/main".to_string()));
        assert_eq!(ahead, 0);
        assert_eq!(behind, 0);
    }

    #[test]
    fn test_parse_diff_stats_no_changes() {
        let output = "";
        let (files, ins, del) = GitOps::parse_diff_stats(output);
        assert_eq!(files, 0);
        assert_eq!(ins, 0);
        assert_eq!(del, 0);
    }

    #[test]
    fn test_parse_diff_stats_only_insertions() {
        let output = "2 files changed, 30 insertions(+)";
        let (files, ins, del) = GitOps::parse_diff_stats(output);
        assert_eq!(files, 2);
        assert_eq!(ins, 30);
        assert_eq!(del, 0);
    }

    #[test]
    fn test_parse_diff_stats_only_deletions() {
        let output = "1 file changed, 10 deletions(-)";
        let (files, ins, del) = GitOps::parse_diff_stats(output);
        assert_eq!(files, 1);
        assert_eq!(ins, 0);
        assert_eq!(del, 10);
    }

    #[test]
    fn test_git_result_structure() {
        let result = GitResult {
            success: true,
            output: "test output".to_string(),
            error: None,
        };
        assert_eq!(result.success, true);
        assert_eq!(result.output, "test output");
        assert!(result.error.is_none());
    }

    #[test]
    fn test_commit_info_structure() {
        let commit = CommitInfo {
            hash: "abc123".to_string(),
            message: "Test commit".to_string(),
            author: "Test User <test@example.com>".to_string(),
            timestamp: "2025-01-15 10:00:00".to_string(),
            files_changed: vec!["file1.rs".to_string(), "file2.rs".to_string()],
        };
        assert_eq!(commit.hash, "abc123");
        assert_eq!(commit.files_changed.len(), 2);
    }

    #[test]
    fn test_branch_info_structure() {
        let branch = BranchInfo {
            name: "main".to_string(),
            is_current: true,
            upstream: Some("origin/main".to_string()),
            ahead: 3,
            behind: 1,
        };
        assert_eq!(branch.name, "main");
        assert_eq!(branch.is_current, true);
        assert_eq!(branch.ahead, 3);
    }

    #[test]
    fn test_repo_stats_structure() {
        let stats = RepoStats {
            commit_count: 100,
            contributor_count: 5,
            file_count: 50,
        };
        assert_eq!(stats.commit_count, 100);
        assert_eq!(stats.contributor_count, 5);
        assert_eq!(stats.file_count, 50);
    }
}
