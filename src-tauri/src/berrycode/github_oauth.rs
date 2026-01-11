//! GitHub OAuth Integration
//!
//! Handles OAuth authentication flow and GitHub API interactions

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::env;

/// GitHub OAuth configuration
#[derive(Clone, Debug)]
pub struct GitHubOAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

impl GitHubOAuthConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            client_id: env::var("GITHUB_CLIENT_ID")
                .unwrap_or_else(|_| "your_github_client_id".to_string()),
            client_secret: env::var("GITHUB_CLIENT_SECRET")
                .unwrap_or_else(|_| "your_github_client_secret".to_string()),
            redirect_uri: env::var("GITHUB_REDIRECT_URI")
                .unwrap_or_else(|_| "http://localhost:7778/auth/github/callback".to_string()),
        })
    }

    /// Generate authorization URL
    pub fn authorization_url(&self, state: &str) -> String {
        format!(
            "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&scope=repo,user&state={}",
            self.client_id,
            urlencoding::encode(&self.redirect_uri),
            state
        )
    }
}

/// OAuth token response
#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub scope: String,
}

/// GitHub repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubRepo {
    pub id: u64,
    pub name: String,
    pub full_name: String,
    pub description: Option<String>,
    pub html_url: String,
    pub clone_url: String,
    pub ssh_url: String,
    pub private: bool,
    pub language: Option<String>,
    pub stargazers_count: u64,
    pub updated_at: String,
    pub owner: GitHubUser,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubUser {
    pub login: String,
    pub avatar_url: String,
}

/// GitHub Issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubIssue {
    pub id: u64,
    pub number: u64,
    pub title: String,
    pub body: Option<String>,
    pub state: String,
    pub html_url: String,
    pub user: GitHubUser,
    pub labels: Vec<GitHubLabel>,
    pub assignees: Vec<GitHubUser>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubLabel {
    pub id: u64,
    pub name: String,
    pub color: String,
    pub description: Option<String>,
}

/// GitHub Pull Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubPullRequest {
    pub id: u64,
    pub number: u64,
    pub title: String,
    pub body: Option<String>,
    pub state: String,
    pub html_url: String,
    pub user: GitHubUser,
    pub head: GitHubBranch,
    pub base: GitHubBranch,
    pub created_at: String,
    pub updated_at: String,
    pub mergeable: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubBranch {
    pub label: String,
    #[serde(rename = "ref")]
    pub ref_name: String,
    pub sha: String,
}

/// Create issue request
#[derive(Debug, Serialize)]
pub struct CreateIssueRequest {
    pub title: String,
    pub body: Option<String>,
    pub labels: Option<Vec<String>>,
    pub assignees: Option<Vec<String>>,
}

/// Create pull request request
#[derive(Debug, Serialize)]
pub struct CreatePullRequestRequest {
    pub title: String,
    pub body: Option<String>,
    pub head: String,
    pub base: String,
}

/// GitHub OAuth client
#[derive(Clone)]
pub struct GitHubOAuthClient {
    config: GitHubOAuthConfig,
    client: reqwest::Client,
}

impl GitHubOAuthClient {
    /// Create new GitHub OAuth client
    pub fn new(config: GitHubOAuthConfig) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("BerryCode/1.0")
            .build()
            .unwrap();

        Self { config, client }
    }

    /// Exchange code for access token
    pub async fn exchange_code(&self, code: &str) -> Result<String> {
        #[derive(Serialize)]
        struct TokenRequest {
            client_id: String,
            client_secret: String,
            code: String,
            redirect_uri: String,
        }

        let request = TokenRequest {
            client_id: self.config.client_id.clone(),
            client_secret: self.config.client_secret.clone(),
            code: code.to_string(),
            redirect_uri: self.config.redirect_uri.clone(),
        };

        let response = self
            .client
            .post("https://github.com/login/oauth/access_token")
            .header("Accept", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to exchange code: {}",
                response.status()
            ));
        }

        let token_response: TokenResponse = response.json().await?;
        Ok(token_response.access_token)
    }

    /// Fetch user's repositories
    pub async fn fetch_repositories(&self, access_token: &str) -> Result<Vec<GitHubRepo>> {
        let response = self
            .client
            .get("https://api.github.com/user/repos")
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Accept", "application/vnd.github.v3+json")
            .query(&[("per_page", "100"), ("sort", "updated")])
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to fetch repositories: {}",
                response.status()
            ));
        }

        let repos: Vec<GitHubRepo> = response.json().await?;
        Ok(repos)
    }

    /// Search repositories
    pub async fn search_repositories(
        &self,
        access_token: &str,
        query: &str,
    ) -> Result<Vec<GitHubRepo>> {
        let repos = self.fetch_repositories(access_token).await?;

        let filtered: Vec<GitHubRepo> = repos
            .into_iter()
            .filter(|repo| {
                let query_lower = query.to_lowercase();
                repo.name.to_lowercase().contains(&query_lower)
                    || repo.full_name.to_lowercase().contains(&query_lower)
                    || repo
                        .description
                        .as_ref()
                        .map(|d| d.to_lowercase().contains(&query_lower))
                        .unwrap_or(false)
            })
            .collect();

        Ok(filtered)
    }

    /// Fetch issues from a repository
    pub async fn fetch_issues(
        &self,
        access_token: &str,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<GitHubIssue>> {
        let url = format!("https://api.github.com/repos/{}/{}/issues", owner, repo);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Accept", "application/vnd.github.v3+json")
            .query(&[("state", "all"), ("per_page", "100")])
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to fetch issues: {}",
                response.status()
            ));
        }

        let issues: Vec<GitHubIssue> = response.json().await?;
        Ok(issues)
    }

    /// Create a new issue
    pub async fn create_issue(
        &self,
        access_token: &str,
        owner: &str,
        repo: &str,
        request: CreateIssueRequest,
    ) -> Result<GitHubIssue> {
        let url = format!("https://api.github.com/repos/{}/{}/issues", owner, repo);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Accept", "application/vnd.github.v3+json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to create issue: {}",
                response.status()
            ));
        }

        let issue: GitHubIssue = response.json().await?;
        Ok(issue)
    }

    /// Fetch pull requests from a repository
    pub async fn fetch_pull_requests(
        &self,
        access_token: &str,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<GitHubPullRequest>> {
        let url = format!("https://api.github.com/repos/{}/{}/pulls", owner, repo);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Accept", "application/vnd.github.v3+json")
            .query(&[("state", "all"), ("per_page", "100")])
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to fetch pull requests: {}",
                response.status()
            ));
        }

        let prs: Vec<GitHubPullRequest> = response.json().await?;
        Ok(prs)
    }

    /// Create a new pull request
    pub async fn create_pull_request(
        &self,
        access_token: &str,
        owner: &str,
        repo: &str,
        request: CreatePullRequestRequest,
    ) -> Result<GitHubPullRequest> {
        let url = format!("https://api.github.com/repos/{}/{}/pulls", owner, repo);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Accept", "application/vnd.github.v3+json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to create pull request: {}",
                response.status()
            ));
        }

        let pr: GitHubPullRequest = response.json().await?;
        Ok(pr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_from_env() {
        let config = GitHubOAuthConfig::from_env();
        assert!(config.is_ok());
    }

    #[test]
    fn test_authorization_url() {
        let config = GitHubOAuthConfig {
            client_id: "test_client_id".to_string(),
            client_secret: "test_secret".to_string(),
            redirect_uri: "http://localhost:7778/callback".to_string(),
        };

        let url = config.authorization_url("test_state");
        assert!(url.contains("client_id=test_client_id"));
        assert!(url.contains("state=test_state"));
    }
}
