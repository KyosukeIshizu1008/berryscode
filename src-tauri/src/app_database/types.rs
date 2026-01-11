use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub id: String,
    pub project_root: PathBuf,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentProject {
    pub project_root: PathBuf,
    pub last_accessed: DateTime<Utc>,
    pub session_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecution {
    pub id: String,
    pub session_id: String,
    pub pipeline_id: String,
    pub pipeline_name: String,
    pub status: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub loop_count: usize,
    pub error_message: Option<String>,
    pub execution_log: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSnapshot {
    pub snapshot_id: String,
    pub execution_id: String,
    pub node_id: String,
    pub node_name: String,
    pub snapshot_data: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSettings {
    pub design: String,
    pub implementation: String,
    pub review: String,
    pub test: String,
    pub debug: String,
}

impl Default for ModelSettings {
    fn default() -> Self {
        Self {
            design: "gpt-5.1-high".to_string(),
            implementation: "gpt-5.1-high".to_string(),
            review: "claude-4.5-sonnet".to_string(),
            test: "grok-4-fast".to_string(),
            debug: "gemini-2.5-flash-lite".to_string(),
        }
    }
}

impl ModelSettings {
    pub fn to_hashmap(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert("design".to_string(), self.design.clone());
        map.insert("implementation".to_string(), self.implementation.clone());
        map.insert("review".to_string(), self.review.clone());
        map.insert("test".to_string(), self.test.clone());
        map.insert("debug".to_string(), self.debug.clone());
        map
    }

    pub fn from_hashmap(map: &HashMap<String, String>) -> Self {
        let defaults = Self::default();
        Self {
            design: map.get("design").cloned().unwrap_or(defaults.design),
            implementation: map.get("implementation").cloned().unwrap_or(defaults.implementation),
            review: map.get("review").cloned().unwrap_or(defaults.review),
            test: map.get("test").cloned().unwrap_or(defaults.test),
            debug: map.get("debug").cloned().unwrap_or(defaults.debug),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSessionData {
    pub id: String,
    pub session_id: String,
    pub title: String,
    pub created_at: String,
    pub last_message_at: String,
    pub message_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessageData {
    pub id: i64,
    pub chat_session_id: String,
    pub role: String,
    pub content: String,
    pub timestamp: String,
}
