//! Commit History View
//!
//! Display commit log with details

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use crate::common::async_bridge::TauriBridge;
use crate::common::ui_components::Panel;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CommitInfo {
    pub hash: String,
    pub short_hash: String,
    pub message: String,
    pub author: String,
    pub email: String,
    pub timestamp: i64,
    pub parents: Vec<String>,
}

/// Commit History Panel
#[component]
pub fn CommitHistoryPanel() -> Element {
    let mut commits = use_signal(|| Vec::<CommitInfo>::new());
    let mut selected_commit = use_signal(|| Option::<String>::None);
    let mut loading = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    // Load commits on mount
    use_effect(move || {
        spawn(async move {
            *loading.write() = true;
            match load_commits().await {
                Ok(commit_list) => {
                    *commits.write() = commit_list;
                    *error.write() = None;
                }
                Err(e) => {
                    *error.write() = Some(format!("Failed to load commits: {}", e));
                }
            }
            *loading.write() = false;
        });
    });

    rsx! {
        Panel { title: "Commit History",
            div { class: "berry-commit-history",
                {
                    if *loading.read() {
                        rsx! {
                            div { class: "berry-git-loading", "Loading..." }
                        }
                    } else if let Some(ref err) = *error.read() {
                        rsx! {
                            div { class: "berry-git-error", "{err}" }
                        }
                    } else {
                        let commit_list = commits.read().clone();

                        if commit_list.is_empty() {
                            rsx! {
                                div { class: "berry-git-empty", "No commits" }
                            }
                        } else {
                            rsx! {
                                for commit in commit_list {
                                    {
                                        let hash = commit.hash.clone();
                                        let is_selected = selected_commit.read().as_ref() == Some(&hash);

                                        rsx! {
                                            CommitItem {
                                                commit: commit.clone(),
                                                selected: is_selected,
                                                on_select: move |_| *selected_commit.write() = Some(hash.clone())
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Single commit item props
#[derive(Props, Clone, PartialEq)]
struct CommitItemProps {
    commit: CommitInfo,
    selected: bool,
    on_select: EventHandler<()>,
}

/// Single commit item
#[component]
fn CommitItem(props: CommitItemProps) -> Element {
    let commit = props.commit;
    let selected = props.selected;
    let on_select = props.on_select;

    let class = if selected {
        "berry-commit-item berry-commit-item-selected"
    } else {
        "berry-commit-item"
    };

    // Format timestamp
    let timestamp = commit.timestamp;
    let datetime = chrono::DateTime::from_timestamp(timestamp, 0)
        .unwrap_or_else(|| chrono::Utc::now().into());
    let time_str = datetime.format("%Y-%m-%d %H:%M").to_string();

    // Extract first line of commit message
    let first_line = commit.message.lines().next().unwrap_or("").to_string();

    rsx! {
        div {
            class: "{class}",
            onclick: move |_| on_select.call(()),

            div { class: "berry-commit-header",
                span { class: "berry-commit-hash", "{commit.short_hash}" }
                span { class: "berry-commit-time", "{time_str}" }
            }
            div { class: "berry-commit-message", "{first_line}" }
            div { class: "berry-commit-author", "{commit.author}" }
        }
    }
}

async fn load_commits() -> anyhow::Result<Vec<CommitInfo>> {
    #[derive(Serialize)]
    struct LogRequest {
        limit: Option<usize>,
    }

    let commits: Vec<CommitInfo> = TauriBridge::invoke("git_log", LogRequest {
        limit: Some(100),
    }).await?;

    Ok(commits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commit_info_creation() {
        let commit = CommitInfo {
            hash: "abc123".to_string(),
            short_hash: "abc".to_string(),
            message: "Test commit".to_string(),
            author: "Test Author".to_string(),
            email: "test@example.com".to_string(),
            timestamp: 1234567890,
            parents: vec![],
        };

        assert_eq!(commit.hash, "abc123");
        assert_eq!(commit.short_hash, "abc");
    }
}
