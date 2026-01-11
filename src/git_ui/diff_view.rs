//! Diff View Component
//!
//! Side-by-side or unified diff view for Git changes

use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use crate::common::async_bridge::TauriBridge;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffLine {
    pub origin: char,
    pub content: String,
    pub old_lineno: Option<u32>,
    pub new_lineno: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffHunk {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub header: String,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    pub old_path: Option<String>,
    pub new_path: Option<String>,
    pub status: String,
    pub hunks: Vec<DiffHunk>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DiffViewMode {
    SideBySide,
    Unified,
}

/// Diff View Component
#[component]
pub fn DiffView(
    /// File path to show diff for
    file_path: String,
    /// Initial view mode
    #[prop(default = DiffViewMode::SideBySide)]
    mode: DiffViewMode,
) -> impl IntoView {
    let diff = RwSignal::new(None::<FileDiff>);
    let view_mode = RwSignal::new(mode);
    let loading = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);

    // Load diff on mount
    let file_path_clone = file_path.clone();
    Effect::new(move || {
        let path = file_path_clone.clone();
        loading.set(true);
        spawn_local(async move {
            match load_diff(&path).await {
                Ok(file_diff) => {
                    diff.set(Some(file_diff));
                    error.set(None);
                    loading.set(false);
                }
                Err(e) => {
                    error.set(Some(format!("Failed to load diff: {}", e)));
                    loading.set(false);
                }
            }
        });
    });

    view! {
        <div class="berry-diff-view">
            // Header with mode toggle
            <div class="berry-diff-header">
                <div class="berry-diff-file-path">
                    {
                        let fp = file_path.clone();
                        move || {
                            diff.get().and_then(|d| d.new_path.clone())
                                .unwrap_or_else(|| fp.clone())
                        }
                    }
                </div>

                <div class="berry-diff-mode-toggle">
                    <button
                        class=move || if view_mode.get() == DiffViewMode::SideBySide {
                            "berry-diff-mode-btn active"
                        } else {
                            "berry-diff-mode-btn"
                        }
                        on:click=move |_| view_mode.set(DiffViewMode::SideBySide)
                    >
                        "Side by Side"
                    </button>
                    <button
                        class=move || if view_mode.get() == DiffViewMode::Unified {
                            "berry-diff-mode-btn active"
                        } else {
                            "berry-diff-mode-btn"
                        }
                        on:click=move |_| view_mode.set(DiffViewMode::Unified)
                    >
                        "Unified"
                    </button>
                </div>
            </div>

            // Content
            <div class="berry-diff-content">
                {move || {
                    if loading.get() {
                        view! {
                            <div class="berry-diff-loading">"Loading diff..."</div>
                        }.into_any()
                    } else if let Some(err) = error.get() {
                        view! {
                            <div class="berry-diff-error">{err}</div>
                        }.into_any()
                    } else if let Some(file_diff) = diff.get() {
                        if view_mode.get() == DiffViewMode::SideBySide {
                            view! {
                                <DiffSideBySide diff=file_diff />
                            }.into_any()
                        } else {
                            view! {
                                <DiffUnified diff=file_diff />
                            }.into_any()
                        }
                    } else {
                        view! {
                            <div class="berry-diff-empty">"No changes"</div>
                        }.into_any()
                    }
                }}
            </div>
        </div>
    }
}

/// Side-by-side diff view
#[component]
fn DiffSideBySide(diff: FileDiff) -> impl IntoView {
    let hunks = diff.hunks;
    view! {
        <div class="berry-diff-side-by-side">
            {hunks.into_iter().map(|hunk| {
                let lines_old = hunk.lines.clone();
                let lines_new = hunk.lines;
                view! {
                    <div class="berry-diff-hunk">
                        <div class="berry-diff-hunk-header">{hunk.header.clone()}</div>

                        <div class="berry-diff-hunk-content">
                            <div class="berry-diff-column berry-diff-old">
                                {lines_old.into_iter().filter_map(|line| {
                                    if line.origin == '+' {
                                        None // Skip additions in left column
                                    } else {
                                        let origin = line.origin;
                                        let old_lineno = line.old_lineno;
                                        let content = line.content;
                                        Some(view! {
                                            <div class=format!("berry-diff-line berry-diff-line-{}",
                                                if origin == '-' { "removed" } else { "context" }
                                            )>
                                                <span class="berry-diff-lineno">
                                                    {old_lineno.map(|n| n.to_string()).unwrap_or_default()}
                                                </span>
                                                <span class="berry-diff-content">{content}</span>
                                            </div>
                                        })
                                    }
                                }).collect::<Vec<_>>()}
                            </div>

                            <div class="berry-diff-column berry-diff-new">
                                {lines_new.into_iter().filter_map(|line| {
                                    if line.origin == '-' {
                                        None // Skip deletions in right column
                                    } else {
                                        let origin = line.origin;
                                        let new_lineno = line.new_lineno;
                                        let content = line.content;
                                        Some(view! {
                                            <div class=format!("berry-diff-line berry-diff-line-{}",
                                                if origin == '+' { "added" } else { "context" }
                                            )>
                                                <span class="berry-diff-lineno">
                                                    {new_lineno.map(|n| n.to_string()).unwrap_or_default()}
                                                </span>
                                                <span class="berry-diff-content">{content}</span>
                                            </div>
                                        })
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                        </div>
                    </div>
                }
            }).collect::<Vec<_>>()}
        </div>
    }
}

/// Unified diff view
#[component]
fn DiffUnified(diff: FileDiff) -> impl IntoView {
    let hunks = diff.hunks;
    view! {
        <div class="berry-diff-unified">
            {hunks.into_iter().map(|hunk| {
                let lines = hunk.lines;
                view! {
                    <div class="berry-diff-hunk">
                        <div class="berry-diff-hunk-header">{hunk.header.clone()}</div>

                        <div class="berry-diff-hunk-content">
                            {lines.into_iter().map(|line| {
                                let class_suffix = match line.origin {
                                    '+' => "added",
                                    '-' => "removed",
                                    _ => "context",
                                };
                                let origin = line.origin;
                                let old_lineno = line.old_lineno;
                                let new_lineno = line.new_lineno;
                                let content = line.content;

                                view! {
                                    <div class=format!("berry-diff-line berry-diff-line-{}", class_suffix)>
                                        <span class="berry-diff-lineno berry-diff-lineno-old">
                                            {old_lineno.map(|n| n.to_string()).unwrap_or_default()}
                                        </span>
                                        <span class="berry-diff-lineno berry-diff-lineno-new">
                                            {new_lineno.map(|n| n.to_string()).unwrap_or_default()}
                                        </span>
                                        <span class="berry-diff-origin">{origin}</span>
                                        <span class="berry-diff-content">{content}</span>
                                    </div>
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    </div>
                }
            }).collect::<Vec<_>>()}
        </div>
    }
}

// Helper functions

async fn load_diff(file_path: &str) -> anyhow::Result<FileDiff> {
    #[derive(Serialize)]
    struct DiffRequest {
        file_path: String,
    }

    TauriBridge::invoke("git_diff", DiffRequest {
        file_path: file_path.to_string(),
    }).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_line() {
        let line = DiffLine {
            origin: '+',
            content: "new line".to_string(),
            old_lineno: None,
            new_lineno: Some(10),
        };

        assert_eq!(line.origin, '+');
        assert_eq!(line.new_lineno, Some(10));
    }

    #[test]
    fn test_diff_hunk() {
        let hunk = DiffHunk {
            old_start: 1,
            old_lines: 5,
            new_start: 1,
            new_lines: 6,
            header: "@@ -1,5 +1,6 @@".to_string(),
            lines: vec![],
        };

        assert_eq!(hunk.old_lines, 5);
        assert_eq!(hunk.new_lines, 6);
    }

    #[test]
    fn test_file_diff() {
        let diff = FileDiff {
            old_path: Some("old.rs".to_string()),
            new_path: Some("new.rs".to_string()),
            status: "modified".to_string(),
            hunks: vec![],
        };

        assert_eq!(diff.status, "modified");
    }
}
