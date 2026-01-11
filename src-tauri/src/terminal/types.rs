use serde::{Deserialize, Serialize};

/// Terminal command request
#[derive(Debug, Deserialize)]
pub struct TerminalCommandRequest {
    pub command: String,
    pub background: Option<bool>,
}

/// Terminal command response
#[derive(Debug, Serialize)]
pub struct TerminalCommandResponse {
    pub output: String,
    pub success: bool,
    pub process_id: Option<String>, // For background processes
}

/// Background process info
#[derive(Debug, Serialize, Clone)]
pub struct BackgroundProcessInfo {
    pub id: String,
    pub command: String,
    pub pid: u32,
    pub status: String,
    pub output_lines: Vec<String>,
}

/// Change directory request
#[derive(Debug, Deserialize)]
pub struct ChangeDirRequest {
    pub path: String,
}

/// Kill process request
#[derive(Debug, Deserialize)]
pub struct KillProcessRequest {
    pub process_id: String,
}

/// Process status (matches persistent_terminal.rs)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProcessStatus {
    Running,
    Completed(i32), // Exit code
    Failed(String),
}
