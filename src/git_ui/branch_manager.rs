//! Branch Manager
//!
//! Create, delete, and switch branches

use dioxus::prelude::*;
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
pub fn BranchManagerPanel() -> Element {
    let mut branches = use_signal(|| Vec::<BranchInfo>::new());
    let mut new_branch_name = use_signal(|| String::new());
    let mut show_create_dialog = use_signal(|| false);
    let mut loading = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    // Load branches
    let load_branches = move || {
        spawn(async move {
            *loading.write() = true;
            match fetch_branches().await {
                Ok(branch_list) => {
                    *branches.write() = branch_list;
                    *error.write() = None;
                }
                Err(e) => {
                    *error.write() = Some(format!("Failed to load branches: {}", e));
                }
            }
            *loading.write() = false;
        });
    };

    // Initial load
    use_effect(move || {
        load_branches();
    });

    // Create branch handler
    let handle_create_branch = move || {
        let name = new_branch_name.read().clone();
        if name.is_empty() {
            *error.write() = Some("Branch name cannot be empty".to_string());
            return;
        }

        spawn(async move {
            match create_branch(&name).await {
                Ok(_) => {
                    *new_branch_name.write() = String::new();
                    *show_create_dialog.write() = false;
                    *error.write() = None;
                    load_branches();
                }
                Err(e) => {
                    *error.write() = Some(format!("Failed to create branch: {}", e));
                }
            }
        });
    };

    // Checkout branch handler
    let handle_checkout = move |branch_name: String| {
        spawn(async move {
            match checkout_branch(&branch_name).await {
                Ok(_) => {
                    *error.write() = None;
                    load_branches();
                }
                Err(e) => {
                    *error.write() = Some(format!("Failed to checkout branch: {}", e));
                }
            }
        });
    };

    // Delete branch handler
    let handle_delete = move |branch_name: String| {
        spawn(async move {
            match delete_branch(&branch_name).await {
                Ok(_) => {
                    *error.write() = None;
                    load_branches();
                }
                Err(e) => {
                    *error.write() = Some(format!("Failed to delete branch: {}", e));
                }
            }
        });
    };

    rsx! {
        Panel { title: "Branches",
            div { class: "berry-branch-manager",
                // Header with create button
                div { class: "berry-branch-header",
                    button {
                        class: "berry-button",
                        onclick: move |_| *show_create_dialog.write() = true,
                        "New Branch"
                    }
                }

                // Create branch dialog
                {
                    if *show_create_dialog.read() {
                        rsx! {
                            div { class: "berry-branch-create-dialog",
                                input {
                                    r#type: "text",
                                    class: "berry-input",
                                    placeholder: "Branch name...",
                                    value: "{new_branch_name.read()}",
                                    oninput: move |ev| *new_branch_name.write() = ev.value(),
                                    onkeydown: move |ev| {
                                        if ev.key() == Key::Enter {
                                            handle_create_branch();
                                        } else if ev.key() == Key::Escape {
                                            *show_create_dialog.write() = false;
                                        }
                                    },
                                }
                                div { class: "berry-dialog-buttons",
                                    button {
                                        class: "berry-button",
                                        onclick: move |_| handle_create_branch(),
                                        "Create"
                                    }
                                    button {
                                        class: "berry-button",
                                        onclick: move |_| *show_create_dialog.write() = false,
                                        "Cancel"
                                    }
                                }
                            }
                        }
                    } else {
                        rsx! {}
                    }
                }

                // Error display
                {
                    error.read().as_ref().map(|err| {
                        rsx! {
                            div { class: "berry-git-error", "{err}" }
                        }
                    })
                }

                // Branch list
                div { class: "berry-branch-list",
                    {
                        if *loading.read() {
                            rsx! {
                                div { class: "berry-git-loading", "Loading..." }
                            }
                        } else {
                            let branch_list = branches.read().clone();

                            if branch_list.is_empty() {
                                rsx! {
                                    div { class: "berry-git-empty", "No branches" }
                                }
                            } else {
                                rsx! {
                                    for branch in branch_list {
                                        {
                                            let name = branch.name.clone();
                                            let is_head = branch.is_head;
                                            let name_for_checkout = name.clone();
                                            let name_for_delete = name.clone();

                                            rsx! {
                                                div { class: "berry-branch-item",
                                                    span { class: "berry-branch-name",
                                                        { if is_head { "* " } else { "  " } }
                                                        "{name}"
                                                    }

                                                    {
                                                        if !is_head {
                                                            rsx! {
                                                                button {
                                                                    class: "berry-branch-checkout-btn",
                                                                    onclick: move |_| handle_checkout(name_for_checkout.clone()),
                                                                    "Checkout"
                                                                }
                                                                button {
                                                                    class: "berry-branch-delete-btn",
                                                                    onclick: move |_| handle_delete(name_for_delete.clone()),
                                                                    "Delete"
                                                                }
                                                            }
                                                        } else {
                                                            rsx! {}
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
        }
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
