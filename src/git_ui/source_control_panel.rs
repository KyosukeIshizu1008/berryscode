//! Source Control Panel
//!
//! IntelliJ-style Git source control panel with staging, commits, and branch management

use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use crate::common::async_bridge::TauriBridge;
use crate::common::ui_components::Panel;

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
}

/// Source Control Panel Component
#[component]
pub fn SourceControlPanel() -> impl IntoView {
    let files = RwSignal::new(Vec::<FileStatus>::new());
    let branches = RwSignal::new(Vec::<BranchInfo>::new());
    let commit_message = RwSignal::new(String::new());
    let current_branch = RwSignal::new(String::from("main"));
    let loading = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);

    // Load initial status
    Effect::new(move || {
        spawn_local(async move {
            if let Err(e) = refresh_status(files, branches, current_branch).await {
                error.set(Some(format!("Failed to load Git status: {}", e)));
            }
        });
    });

    // Refresh handler
    let handle_refresh = move || {
        loading.set(true);
        spawn_local(async move {
            match refresh_status(files, branches, current_branch).await {
                Ok(_) => {
                    error.set(None);
                    loading.set(false);
                }
                Err(e) => {
                    error.set(Some(format!("Failed to refresh: {}", e)));
                    loading.set(false);
                }
            }
        });
    };

    // Stage file handler
    let handle_stage = move |path: String| {
        spawn_local(async move {
            match stage_file(&path).await {
                Ok(_) => {
                    // Refresh status
                    let _ = refresh_status(files, branches, current_branch).await;
                }
                Err(e) => {
                    error.set(Some(format!("Failed to stage: {}", e)));
                }
            }
        });
    };

    // Unstage file handler
    let handle_unstage = move |path: String| {
        spawn_local(async move {
            match unstage_file(&path).await {
                Ok(_) => {
                    // Refresh status
                    let _ = refresh_status(files, branches, current_branch).await;
                }
                Err(e) => {
                    error.set(Some(format!("Failed to unstage: {}", e)));
                }
            }
        });
    };

    // Commit handler
    let handle_commit = move || {
        let message = commit_message.get_untracked();
        if message.is_empty() {
            error.set(Some("Commit message cannot be empty".to_string()));
            return;
        }

        spawn_local(async move {
            match commit_changes(&message).await {
                Ok(_) => {
                    commit_message.set(String::new());
                    error.set(None);
                    // Refresh status
                    let _ = refresh_status(files, branches, current_branch).await;
                }
                Err(e) => {
                    error.set(Some(format!("Failed to commit: {}", e)));
                }
            }
        });
    };

    // Branch switch handler
    let handle_switch_branch = move |branch_name: String| {
        spawn_local(async move {
            match switch_branch(&branch_name).await {
                Ok(_) => {
                    current_branch.set(branch_name);
                    error.set(None);
                    // Refresh status
                    let _ = refresh_status(files, branches, current_branch).await;
                }
                Err(e) => {
                    error.set(Some(format!("Failed to switch branch: {}", e)));
                }
            }
        });
    };

    view! {
        <Panel title="Source Control">
            <div class="berry-git-panel">
                // Header with branch selector and refresh
                <div class="berry-git-header">
                    <select
                        class="berry-git-branch-select"
                        on:change=move |ev| {
                            let value = event_target_value(&ev);
                            handle_switch_branch(value);
                        }
                    >
                        {move || {
                            let current = current_branch.get();
                            branches.get().iter().map(|branch| {
                                let selected = branch.name == current;
                                view! {
                                    <option value=branch.name.clone() selected=selected>
                                        {branch.name.clone()}
                                    </option>
                                }
                            }).collect::<Vec<_>>()
                        }}
                    </select>

                    <button
                        class="berry-git-refresh-btn"
                        on:click=move |_| handle_refresh()
                        disabled=move || loading.get()
                    >
                        {move || if loading.get() { "..." } else { "⟳" }}
                    </button>
                </div>

                // Error display
                {move || {
                    error.get().map(|err| {
                        view! {
                            <div class="berry-git-error">
                                {err}
                            </div>
                        }
                    })
                }}

                // Changes section
                <div class="berry-git-changes">
                    <div class="berry-git-section-title">"CHANGES"</div>

                    {move || {
                        let current_files = files.get();
                        let unstaged: Vec<_> = current_files.iter()
                            .filter(|f| !f.is_staged)
                            .cloned()
                            .collect();

                        if unstaged.is_empty() {
                            view! {
                                <div class="berry-git-empty">"No changes"</div>
                            }.into_any()
                        } else {
                            unstaged.iter().map(|file| {
                                let path = file.path.clone();
                                let status = file.status.clone();
                                let path_for_stage = path.clone();

                                view! {
                                    <div class="berry-git-file">
                                        <span class="berry-git-file-status">{status}</span>
                                        <span class="berry-git-file-path">{path}</span>
                                        <button
                                            class="berry-git-stage-btn"
                                            on:click=move |_| handle_stage(path_for_stage.clone())
                                        >
                                            "+"
                                        </button>
                                    </div>
                                }
                            }).collect::<Vec<_>>().into_any()
                        }
                    }}
                </div>

                // Staged changes section
                <div class="berry-git-staged">
                    <div class="berry-git-section-title">"STAGED CHANGES"</div>

                    {move || {
                        let current_files = files.get();
                        let staged: Vec<_> = current_files.iter()
                            .filter(|f| f.is_staged)
                            .cloned()
                            .collect();

                        if staged.is_empty() {
                            view! {
                                <div class="berry-git-empty">"No staged changes"</div>
                            }.into_any()
                        } else {
                            staged.iter().map(|file| {
                                let path = file.path.clone();
                                let status = file.status.clone();
                                let path_for_unstage = path.clone();

                                view! {
                                    <div class="berry-git-file">
                                        <span class="berry-git-file-status">{status}</span>
                                        <span class="berry-git-file-path">{path}</span>
                                        <button
                                            class="berry-git-unstage-btn"
                                            on:click=move |_| handle_unstage(path_for_unstage.clone())
                                        >
                                            "-"
                                        </button>
                                    </div>
                                }
                            }).collect::<Vec<_>>().into_any()
                        }
                    }}
                </div>

                // Commit section
                <div class="berry-git-commit">
                    <textarea
                        class="berry-git-commit-message"
                        placeholder="Commit message..."
                        prop:value=move || commit_message.get()
                        on:input=move |ev| {
                            commit_message.set(event_target_value(&ev));
                        }
                    />

                    <button
                        class="berry-git-commit-btn"
                        on:click=move |_| handle_commit()
                        disabled=move || commit_message.get().is_empty()
                    >
                        "Commit"
                    </button>
                </div>
            </div>
        </Panel>
    }
}

// Helper functions

async fn refresh_status(
    files: RwSignal<Vec<FileStatus>>,
    branches: RwSignal<Vec<BranchInfo>>,
    current_branch: RwSignal<String>,
) -> anyhow::Result<()> {
    // Get file status
    let file_status: Vec<FileStatus> = TauriBridge::invoke("git_status", ()).await?;
    files.set(file_status);

    // Get branches
    let branch_list: Vec<BranchInfo> = TauriBridge::invoke("git_list_branches", ()).await?;
    branches.set(branch_list.clone());

    // Update current branch
    if let Some(head) = branch_list.iter().find(|b| b.is_head) {
        current_branch.set(head.name.clone());
    }

    Ok(())
}

async fn stage_file(path: &str) -> anyhow::Result<()> {
    #[derive(Serialize)]
    struct StageRequest {
        path: String,
    }

    TauriBridge::invoke("git_stage_file", StageRequest {
        path: path.to_string(),
    }).await
}

async fn unstage_file(path: &str) -> anyhow::Result<()> {
    #[derive(Serialize)]
    struct UnstageRequest {
        path: String,
    }

    TauriBridge::invoke("git_unstage_file", UnstageRequest {
        path: path.to_string(),
    }).await
}

async fn commit_changes(message: &str) -> anyhow::Result<()> {
    #[derive(Serialize)]
    struct CommitRequest {
        message: String,
    }

    TauriBridge::invoke("git_commit", CommitRequest {
        message: message.to_string(),
    }).await
}

async fn switch_branch(branch_name: &str) -> anyhow::Result<()> {
    #[derive(Serialize)]
    struct CheckoutRequest {
        branch_name: String,
    }

    TauriBridge::invoke("git_checkout_branch", CheckoutRequest {
        branch_name: branch_name.to_string(),
    }).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_status() {
        let status = FileStatus {
            path: "test.rs".to_string(),
            status: "modified".to_string(),
            is_staged: false,
        };

        assert_eq!(status.path, "test.rs");
        assert!(!status.is_staged);
    }

    #[test]
    fn test_branch_info() {
        let branch = BranchInfo {
            name: "main".to_string(),
            is_head: true,
            upstream: Some("origin/main".to_string()),
        };

        assert!(branch.is_head);
        assert_eq!(branch.name, "main");
    }
}
