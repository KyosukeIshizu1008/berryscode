//! Commit History View
//!
//! Display commit log with details

use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use crate::common::async_bridge::TauriBridge;
use crate::common::ui_components::Panel;

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

/// Commit History Panel
#[component]
pub fn CommitHistoryPanel() -> impl IntoView {
    let commits = RwSignal::new(Vec::<CommitInfo>::new());
    let selected_commit = RwSignal::new(Option::<String>::None);
    let loading = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);

    // Load commits on mount
    Effect::new(move || {
        spawn_local(async move {
            loading.set(true);
            match load_commits().await {
                Ok(commit_list) => {
                    commits.set(commit_list);
                    error.set(None);
                }
                Err(e) => {
                    error.set(Some(format!("Failed to load commits: {}", e)));
                }
            }
            loading.set(false);
        });
    });

    view! {
        <Panel title="Commit History">
            <div class="berry-commit-history">
                {move || {
                    if loading.get() {
                        view! {
                            <div class="berry-git-loading">"Loading..."</div>
                        }.into_any()
                    } else if let Some(err) = error.get() {
                        view! {
                            <div class="berry-git-error">{err}</div>
                        }.into_any()
                    } else {
                        let commit_list = commits.get();

                        if commit_list.is_empty() {
                            view! {
                                <div class="berry-git-empty">"No commits"</div>
                            }.into_any()
                        } else {
                            commit_list.iter().map(|commit| {
                                let hash = commit.hash.clone();
                                let is_selected = selected_commit.get().as_ref() == Some(&hash);

                                view! {
                                    <CommitItem
                                        commit=commit.clone()
                                        selected=is_selected
                                        on_select=move || selected_commit.set(Some(hash.clone()))
                                    />
                                }
                            }).collect::<Vec<_>>().into_any()
                        }
                    }
                }}
            </div>
        </Panel>
    }
}

/// Single commit item
#[component]
fn CommitItem(
    commit: CommitInfo,
    selected: bool,
    on_select: impl Fn() + 'static,
) -> impl IntoView {
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

    view! {
        <div class=class on:click=move |_| on_select()>
            <div class="berry-commit-header">
                <span class="berry-commit-hash">{commit.short_hash}</span>
                <span class="berry-commit-time">{time_str}</span>
            </div>
            <div class="berry-commit-message">{first_line}</div>
            <div class="berry-commit-author">{commit.author}</div>
        </div>
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
