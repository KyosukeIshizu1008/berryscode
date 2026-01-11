//! Notification system for BerryFlow
//!
//! Slack and email notifications for workflow completion

use anyhow::Result;
use serde_json::json;

/// Notifier for sending workflow notifications
#[derive(Clone)]
pub struct Notifier {
    pub slack_webhook: Option<String>,
}

impl Notifier {
    /// Create new notifier from optional webhook URL
    pub fn new(slack_webhook: Option<String>) -> Self {
        Self { slack_webhook }
    }

    /// Send workflow completion notification
    pub async fn send_workflow_complete(
        &self,
        workflow_name: &str,
        success: bool,
        duration: &str,
        loop_count: usize,
    ) -> Result<()> {
        if let Some(webhook) = &self.slack_webhook {
            let icon = if success { "✅" } else { "❌" };
            let status = if success { "完了" } else { "失敗" };

            let payload = json!({
                "text": format!("{} Workflow '{}' {}", icon, workflow_name, status),
                "attachments": [{
                    "color": if success { "good" } else { "danger" },
                    "fields": [
                        {
                            "title": "実行時間",
                            "value": duration,
                            "short": true
                        },
                        {
                            "title": "ループ回数",
                            "value": loop_count.to_string(),
                            "short": true
                        }
                    ]
                }]
            });

            reqwest::Client::new()
                .post(webhook)
                .json(&payload)
                .send()
                .await?;

            tracing::info!("Slack notification sent for workflow: {}", workflow_name);
        }
        Ok(())
    }

    /// Send workflow error notification
    pub async fn send_workflow_error(
        &self,
        workflow_name: &str,
        error_message: &str,
    ) -> Result<()> {
        if let Some(webhook) = &self.slack_webhook {
            let payload = json!({
                "text": format!("❌ Workflow '{}' エラー", workflow_name),
                "attachments": [{
                    "color": "danger",
                    "fields": [
                        {
                            "title": "エラー内容",
                            "value": error_message,
                        }
                    ]
                }]
            });

            reqwest::Client::new()
                .post(webhook)
                .json(&payload)
                .send()
                .await?;

            tracing::info!("Error notification sent for workflow: {}", workflow_name);
        }
        Ok(())
    }
}
