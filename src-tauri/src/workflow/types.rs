use serde::{Deserialize, Serialize};

/// Workflow preset information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowPreset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub nodes_count: usize,
}

/// Workflow execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStatus {
    pub execution_id: String,
    pub preset_id: String,
    pub status: ExecutionStatus,
    pub current_step: Option<String>,
    pub progress: f32, // 0.0 - 1.0
    pub started_at: i64,
    pub completed_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionStatus {
    Running,
    Paused,
    Completed,
    Failed,
}

/// Start workflow request
#[derive(Debug, Deserialize)]
pub struct StartWorkflowRequest {
    pub preset_id: String,
    pub initial_prompt: String,
}
