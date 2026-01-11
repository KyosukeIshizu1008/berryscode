//! Blame View Component
//!
//! Git blame information displayed in gutter or as overlay

use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use crate::common::async_bridge::TauriBridge;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlameLineInfo {
    pub line_number: usize,
    pub commit_id: String,
    pub author: String,
    pub timestamp: i64,
    pub content: String,
}

/// Blame View Component
#[component]
pub fn BlameView(
    /// File path to show blame for
    file_path: String,
    /// File content lines
    file_lines: Vec<String>,
) -> impl IntoView {
    let blame_info = RwSignal::new(Vec::<BlameLineInfo>::new());
    let loading = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let selected_line = RwSignal::new(None::<usize>);

    // Load blame on mount
    Effect::new(move || {
        let path = file_path.clone();
        let lines = file_lines.clone();
        loading.set(true);

        spawn_local(async move {
            match load_blame(&path, &lines).await {
                Ok(info) => {
                    blame_info.set(info);
                    error.set(None);
                    loading.set(false);
                }
                Err(e) => {
                    error.set(Some(format!("Failed to load blame: {}", e)));
                    loading.set(false);
                }
            }
        });
    });

    view! {
        <div class="berry-blame-view">
            {move || {
                if loading.get() {
                    view! {
                        <div class="berry-blame-loading">"Loading blame..."</div>
                    }.into_any()
                } else if let Some(err) = error.get() {
                    view! {
                        <div class="berry-blame-error">{err}</div>
                    }.into_any()
                } else {
                    let info = blame_info.get();
                    info.iter().map(|line_info| {
                        let line_num = line_info.line_number;
                        let is_selected = selected_line.get() == Some(line_num);

                        view! {
                            <BlameLineView
                                line_info=line_info.clone()
                                selected=is_selected
                                on_click=move || selected_line.set(Some(line_num))
                            />
                        }
                    }).collect::<Vec<_>>().into_any()
                }
            }}
        </div>
    }
}

/// Single blame line view
#[component]
fn BlameLineView(
    /// Blame information for this line
    line_info: BlameLineInfo,
    /// Whether this line is selected
    selected: bool,
    /// Click handler
    on_click: impl Fn() + 'static,
) -> impl IntoView {
    let class = if selected {
        "berry-blame-line selected"
    } else {
        "berry-blame-line"
    };

    // Format timestamp as relative time
    let time_ago = format_relative_time(line_info.timestamp);

    // Shorten commit ID
    let short_commit = line_info.commit_id.chars().take(8).collect::<String>();

    view! {
        <div
            class=class
            on:click=move |_| on_click()
        >
            <div class="berry-blame-gutter">
                <span class="berry-blame-commit" title=line_info.commit_id.clone()>
                    {short_commit}
                </span>
                <span class="berry-blame-author">
                    {line_info.author.clone()}
                </span>
                <span class="berry-blame-time">
                    {time_ago}
                </span>
            </div>

            <div class="berry-blame-content">
                <span class="berry-blame-lineno">
                    {line_info.line_number}
                </span>
                <span class="berry-blame-text">
                    {line_info.content.clone()}
                </span>
            </div>
        </div>
    }
}

/// Blame Gutter Component (for inline display in editor)
#[component]
pub fn BlameGutter(
    /// Line number
    line_number: usize,
    /// Blame information (if available)
    blame_info: Option<BlameLineInfo>,
) -> impl IntoView {
    view! {
        <div class="berry-blame-gutter-inline">
            {move || {
                if let Some(info) = blame_info.clone() {
                    let short_commit = info.commit_id.chars().take(7).collect::<String>();
                    let time_ago = format_relative_time(info.timestamp);
                    let author = info.author.clone();
                    let commit_id = info.commit_id.clone();

                    view! {
                        <div
                            class="berry-blame-gutter-content"
                            title=format!(
                                "Commit: {}\nAuthor: {}\nDate: {}",
                                commit_id,
                                author,
                                time_ago
                            )
                        >
                            <span class="berry-blame-gutter-commit">{short_commit}</span>
                            <span class="berry-blame-gutter-author">{info.author}</span>
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <div class="berry-blame-gutter-empty"></div>
                    }.into_any()
                }
            }}
        </div>
    }
}

/// Blame Detail Panel (shows full info for selected line)
#[component]
pub fn BlameDetailPanel(
    /// Selected blame info
    blame_info: Option<BlameLineInfo>,
) -> impl IntoView {
    view! {
        <div class="berry-blame-detail-panel">
            {move || {
                if let Some(info) = blame_info.clone() {
                    let formatted_time = format_timestamp(info.timestamp);

                    view! {
                        <div class="berry-blame-detail">
                            <div class="berry-blame-detail-section">
                                <div class="berry-blame-detail-label">"Commit:"</div>
                                <div class="berry-blame-detail-value">{info.commit_id}</div>
                            </div>

                            <div class="berry-blame-detail-section">
                                <div class="berry-blame-detail-label">"Author:"</div>
                                <div class="berry-blame-detail-value">{info.author}</div>
                            </div>

                            <div class="berry-blame-detail-section">
                                <div class="berry-blame-detail-label">"Date:"</div>
                                <div class="berry-blame-detail-value">{formatted_time}</div>
                            </div>

                            <div class="berry-blame-detail-section">
                                <div class="berry-blame-detail-label">"Line:"</div>
                                <div class="berry-blame-detail-value">{info.line_number}</div>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <div class="berry-blame-detail-empty">
                            "Select a line to see blame information"
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}

// Helper functions

async fn load_blame(file_path: &str, file_lines: &[String]) -> anyhow::Result<Vec<BlameLineInfo>> {
    #[derive(Serialize)]
    struct BlameRequest {
        file_path: String,
    }

    let mut blame_info: Vec<BlameLineInfo> = TauriBridge::invoke("git_blame", BlameRequest {
        file_path: file_path.to_string(),
    }).await?;

    // Fill in content from file lines
    for (idx, info) in blame_info.iter_mut().enumerate() {
        if let Some(line) = file_lines.get(idx) {
            info.content = line.clone();
        }
    }

    Ok(blame_info)
}

fn format_relative_time(timestamp: i64) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let diff = now - timestamp;

    if diff < 60 {
        "just now".to_string()
    } else if diff < 3600 {
        format!("{} minutes ago", diff / 60)
    } else if diff < 86400 {
        format!("{} hours ago", diff / 3600)
    } else if diff < 2592000 {
        format!("{} days ago", diff / 86400)
    } else if diff < 31536000 {
        format!("{} months ago", diff / 2592000)
    } else {
        format!("{} years ago", diff / 31536000)
    }
}

fn format_timestamp(timestamp: i64) -> String {
    use chrono::{DateTime, Utc};
    use std::time::{SystemTime, UNIX_EPOCH};

    let duration = std::time::Duration::from_secs(timestamp as u64);
    let system_time = UNIX_EPOCH + duration;
    let datetime: DateTime<Utc> = system_time.into();

    datetime.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blame_line_info() {
        let info = BlameLineInfo {
            line_number: 10,
            commit_id: "abc123def456".to_string(),
            author: "Test User".to_string(),
            timestamp: 1234567890,
            content: "let x = 42;".to_string(),
        };

        assert_eq!(info.line_number, 10);
        assert_eq!(info.author, "Test User");
    }

    #[test]
    fn test_format_relative_time() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // Test recent time
        let recent = now - 30;
        assert_eq!(format_relative_time(recent), "just now");

        // Test minutes ago
        let minutes_ago = now - 300; // 5 minutes
        assert_eq!(format_relative_time(minutes_ago), "5 minutes ago");
    }

    #[test]
    fn test_format_timestamp() {
        let timestamp = 1234567890;
        let formatted = format_timestamp(timestamp);
        assert!(formatted.contains("2009"));
    }
}
