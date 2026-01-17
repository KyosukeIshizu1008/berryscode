//! Slack API client for BerryCode
//! Provides chat functionality with Slack workspaces

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Slack message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackMessage {
    pub user: String,
    pub text: String,
    pub timestamp: String,
    pub thread_ts: Option<String>,
}

/// Slack channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackChannel {
    pub id: String,
    pub name: String,
    pub is_member: bool,
}

/// Slack API response for conversations.list
#[derive(Debug, Deserialize)]
struct ConversationsListResponse {
    ok: bool,
    channels: Option<Vec<ChannelInfo>>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChannelInfo {
    id: String,
    name: String,
    is_member: bool,
}

/// Slack API response for conversations.history
#[derive(Debug, Deserialize)]
struct ConversationsHistoryResponse {
    ok: bool,
    messages: Option<Vec<MessageInfo>>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MessageInfo {
    user: Option<String>,
    text: String,
    ts: String,
    thread_ts: Option<String>,
}

/// Slack API response for chat.postMessage
#[derive(Debug, Deserialize)]
struct PostMessageResponse {
    ok: bool,
    error: Option<String>,
}

/// Slack client
#[derive(Clone)]
pub struct SlackClient {
    token: Arc<RwLock<Option<String>>>,
    http_client: reqwest::Client,
}

impl SlackClient {
    /// Create a new Slack client
    pub fn new() -> Self {
        Self {
            token: Arc::new(RwLock::new(None)),
            http_client: reqwest::Client::new(),
        }
    }

    /// Set Slack bot token
    pub async fn set_token(&self, token: String) {
        *self.token.write().await = Some(token);
    }

    /// Check if token is set
    pub async fn is_authenticated(&self) -> bool {
        self.token.read().await.is_some()
    }

    /// Get list of channels
    pub async fn list_channels(&self) -> Result<Vec<SlackChannel>> {
        let token = self.token.read().await;
        let token = token.as_ref().context("Slack token not set")?;

        let response = self.http_client
            .get("https://slack.com/api/conversations.list")
            .header("Authorization", format!("Bearer {}", token))
            .query(&[("types", "public_channel,private_channel")])
            .send()
            .await
            .context("Failed to fetch channels")?;

        let data: ConversationsListResponse = response.json::<ConversationsListResponse>().await
            .context("Failed to parse channels response")?;

        if !data.ok {
            anyhow::bail!("Slack API error: {:?}", data.error);
        }

        let channels = data.channels.unwrap_or_default()
            .into_iter()
            .map(|ch| SlackChannel {
                id: ch.id,
                name: ch.name,
                is_member: ch.is_member,
            })
            .collect();

        Ok(channels)
    }

    /// Get messages from a channel
    pub async fn get_messages(&self, channel_id: &str, limit: usize) -> Result<Vec<SlackMessage>> {
        let token = self.token.read().await;
        let token = token.as_ref().context("Slack token not set")?;

        let response = self.http_client
            .get("https://slack.com/api/conversations.history")
            .header("Authorization", format!("Bearer {}", token))
            .query(&[
                ("channel", channel_id),
                ("limit", &limit.to_string()),
            ])
            .send()
            .await
            .context("Failed to fetch messages")?;

        let data: ConversationsHistoryResponse = response.json::<ConversationsHistoryResponse>().await
            .context("Failed to parse messages response")?;

        if !data.ok {
            anyhow::bail!("Slack API error: {:?}", data.error);
        }

        let messages = data.messages.unwrap_or_default()
            .into_iter()
            .map(|msg| SlackMessage {
                user: msg.user.unwrap_or_else(|| "unknown".to_string()),
                text: msg.text,
                timestamp: msg.ts,
                thread_ts: msg.thread_ts,
            })
            .collect();

        Ok(messages)
    }

    /// Send a message to a channel
    pub async fn send_message(&self, channel_id: &str, text: &str, thread_ts: Option<&str>) -> Result<()> {
        let token = self.token.read().await;
        let token = token.as_ref().context("Slack token not set")?;

        let mut params = vec![
            ("channel", channel_id),
            ("text", text),
        ];

        if let Some(ts) = thread_ts {
            params.push(("thread_ts", ts));
        }

        let response = self.http_client
            .post("https://slack.com/api/chat.postMessage")
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "channel": channel_id,
                "text": text,
                "thread_ts": thread_ts,
            }))
            .send()
            .await
            .context("Failed to send message")?;

        let data: PostMessageResponse = response.json::<PostMessageResponse>().await
            .context("Failed to parse send message response")?;

        if !data.ok {
            anyhow::bail!("Slack API error: {:?}", data.error);
        }

        Ok(())
    }

    /// Get thread messages
    pub async fn get_thread_messages(&self, channel_id: &str, thread_ts: &str) -> Result<Vec<SlackMessage>> {
        let token = self.token.read().await;
        let token = token.as_ref().context("Slack token not set")?;

        let response = self.http_client
            .get("https://slack.com/api/conversations.replies")
            .header("Authorization", format!("Bearer {}", token))
            .query(&[
                ("channel", channel_id),
                ("ts", thread_ts),
            ])
            .send()
            .await
            .context("Failed to fetch thread messages")?;

        let data: ConversationsHistoryResponse = response.json::<ConversationsHistoryResponse>().await
            .context("Failed to parse thread messages response")?;

        if !data.ok {
            anyhow::bail!("Slack API error: {:?}", data.error);
        }

        let messages = data.messages.unwrap_or_default()
            .into_iter()
            .map(|msg| SlackMessage {
                user: msg.user.unwrap_or_else(|| "unknown".to_string()),
                text: msg.text,
                timestamp: msg.ts,
                thread_ts: msg.thread_ts,
            })
            .collect();

        Ok(messages)
    }
}

/// Global Slack client instance
static SLACK_CLIENT: once_cell::sync::Lazy<SlackClient> = once_cell::sync::Lazy::new(|| {
    SlackClient::new()
});

/// Get the global Slack client
pub fn get_client() -> &'static SlackClient {
    &SLACK_CLIENT
}

/// Set Slack token
pub async fn set_token(token: String) {
    get_client().set_token(token).await;
}

/// Check if authenticated
pub async fn is_authenticated() -> bool {
    get_client().is_authenticated().await
}

/// List channels
pub async fn list_channels() -> Result<Vec<SlackChannel>> {
    get_client().list_channels().await
}

/// Get messages from a channel
pub async fn get_messages(channel_id: &str, limit: usize) -> Result<Vec<SlackMessage>> {
    get_client().get_messages(channel_id, limit).await
}

/// Send a message
pub async fn send_message(channel_id: &str, text: &str, thread_ts: Option<&str>) -> Result<()> {
    get_client().send_message(channel_id, text, thread_ts).await
}

/// Get thread messages
pub async fn get_thread_messages(channel_id: &str, thread_ts: &str) -> Result<Vec<SlackMessage>> {
    get_client().get_thread_messages(channel_id, thread_ts).await
}
