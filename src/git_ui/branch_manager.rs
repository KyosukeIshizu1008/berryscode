//! Branch Manager
//!
//! Create, delete, and switch branches

use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use crate::common::async_bridge::TauriBridge;
use crate::common::ui_components::{Panel, Button};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchInfo {
    pub name: String,
    pub is_head: bool,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
}

/// Branch Manager Panel
#[component]
pub fn BranchManagerPanel() -> impl IntoView {
    let branches = RwSignal::new(Vec::<BranchInfo>::new());
    let new_branch_name = RwSignal::new(String::new());
    let show_create_dialog = RwSignal::new(false);
    let loading = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);

    // Load branches
    let load_branches = move || {
        spawn_local(async move {
            loading.set(true);
            match fetch_branches().await {
                Ok(branch_list) => {
                    branches.set(branch_list);
                    error.set(None);
                }
                Err(e) => {
                    error.set(Some(format!("Failed to load branches: {}", e)));
                }
            }
            loading.set(false);
        });
    };

    // Initial load
    Effect::new(move || {
        load_branches();
    });

    // Create branch handler
    let handle_create_branch = move || {
        let name = new_branch_name.get_untracked();
        if name.is_empty() {
            error.set(Some("Branch name cannot be empty".to_string()));
            return;
        }

        spawn_local(async move {
            match create_branch(&name).await {
                Ok(_) => {
                    new_branch_name.set(String::new());
                    show_create_dialog.set(false);
                    error.set(None);
                    load_branches();
                }
                Err(e) => {
                    error.set(Some(format!("Failed to create branch: {}", e)));
                }
            }
        });
    };

    // Checkout branch handler
    let handle_checkout = move |branch_name: String| {
        spawn_local(async move {
            match checkout_branch(&branch_name).await {
                Ok(_) => {
                    error.set(None);
                    load_branches();
                }
                Err(e) => {
                    error.set(Some(format!("Failed to checkout branch: {}", e)));
                }
            }
        });
    };

    // Delete branch handler
    let handle_delete = move |branch_name: String| {
        spawn_local(async move {
            match delete_branch(&branch_name).await {
                Ok(_) => {
                    error.set(None);
                    load_branches();
                }
                Err(e) => {
                    error.set(Some(format!("Failed to delete branch: {}", e)));
                }
            }
        });
    };

    view! {
        <Panel title="Branches">
            <div class="berry-branch-manager">
                // Header with create button
                <div class="berry-branch-header">
                    <button
                        class="berry-button"
                        on:click=move |_| show_create_dialog.set(true)
                    >
                        "New Branch"
                    </button>
                </div>

                // Create branch dialog
                {move || {
                    if show_create_dialog.get() {
                        view! {
                            <div class="berry-branch-create-dialog">
                                <input
                                    type="text"
                                    class="berry-input"
                                    placeholder="Branch name..."
                                    prop:value=move || new_branch_name.get()
                                    on:input=move |ev| {
                                        new_branch_name.set(event_target_value(&ev));
                                    }
                                    on:keydown=move |ev| {
                                        if ev.key() == "Enter" {
                                            handle_create_branch();
                                        } else if ev.key() == "Escape" {
                                            show_create_dialog.set(false);
                                        }
                                    }
                                />
                                <div class="berry-dialog-buttons">
                                    <button
                                        class="berry-button"
                                        on:click=move |_| handle_create_branch()
                                    >
                                        "Create"
                                    </button>
                                    <button
                                        class="berry-button"
                                        on:click=move |_| show_create_dialog.set(false)
                                    >
                                        "Cancel"
                                    </button>
                                </div>
                            </div>
                        }.into_any()
                    } else {
                        view! { <></> }.into_any()
                    }
                }}

                // Error display
                {move || {
                    error.get().map(|err| {
                        view! {
                            <div class="berry-git-error">{err}</div>
                        }
                    })
                }}

                // Branch list
                <div class="berry-branch-list">
                    {move || {
                        if loading.get() {
                            view! {
                                <div class="berry-git-loading">"Loading..."</div>
                            }.into_any()
                        } else {
                            let branch_list = branches.get();

                            if branch_list.is_empty() {
                                view! {
                                    <div class="berry-git-empty">"No branches"</div>
                                }.into_any()
                            } else {
                                branch_list.iter().map(|branch| {
                                    let name = branch.name.clone();
                                    let is_head = branch.is_head;
                                    let name_for_checkout = name.clone();
                                    let name_for_delete = name.clone();

                                    view! {
                                        <div class="berry-branch-item">
                                            <span class="berry-branch-name">
                                                {if is_head { "* " } else { "  " }}
                                                {name}
                                            </span>

                                            {if !is_head {
                                                view! {
                                                    <>
                                                        <button
                                                            class="berry-branch-checkout-btn"
                                                            on:click=move |_| handle_checkout(name_for_checkout.clone())
                                                        >
                                                            "Checkout"
                                                        </button>
                                                        <button
                                                            class="berry-branch-delete-btn"
                                                            on:click=move |_| handle_delete(name_for_delete.clone())
                                                        >
                                                            "Delete"
                                                        </button>
                                                    </>
                                                }.into_any()
                                            } else {
                                                view! { <></> }.into_any()
                                            }}
                                        </div>
                                    }
                                }).collect::<Vec<_>>().into_any()
                            }
                        }
                    }}
                </div>
            </div>
        </Panel>
    }
}

async fn fetch_branches() -> anyhow::Result<Vec<BranchInfo>> {
    let branches: Vec<BranchInfo> = TauriBridge::invoke("git_list_branches", ()).await?;
    Ok(branches)
}

async fn create_branch(name: &str) -> anyhow::Result<()> {
    #[derive(Serialize)]
    struct CreateRequest {
        branch_name: String,
    }

    TauriBridge::invoke("git_create_branch", CreateRequest {
        branch_name: name.to_string(),
    }).await
}

async fn checkout_branch(name: &str) -> anyhow::Result<()> {
    #[derive(Serialize)]
    struct CheckoutRequest {
        branch_name: String,
    }

    TauriBridge::invoke("git_checkout_branch", CheckoutRequest {
        branch_name: name.to_string(),
    }).await
}

async fn delete_branch(name: &str) -> anyhow::Result<()> {
    #[derive(Serialize)]
    struct DeleteRequest {
        branch_name: String,
    }

    TauriBridge::invoke("git_delete_branch", DeleteRequest {
        branch_name: name.to_string(),
    }).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_branch_info_creation() {
        let branch = BranchInfo {
            name: "feature/test".to_string(),
            is_head: false,
            upstream: Some("origin/feature/test".to_string()),
            ahead: 2,
            behind: 1,
        };

        assert_eq!(branch.name, "feature/test");
        assert!(!branch.is_head);
        assert_eq!(branch.ahead, 2);
        assert_eq!(branch.behind, 1);
    }
}
