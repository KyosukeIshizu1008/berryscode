// Database schema for Tauri app persistent storage

pub const CREATE_SESSIONS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    project_root TEXT NOT NULL,
    created_at DATETIME NOT NULL,
    last_activity DATETIME NOT NULL
)
"#;

pub const CREATE_RECENT_PROJECTS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS recent_projects (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_root TEXT NOT NULL UNIQUE,
    last_accessed DATETIME NOT NULL,
    session_count INTEGER NOT NULL DEFAULT 1
)
"#;

pub const CREATE_MODEL_SETTINGS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS model_settings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    task_type TEXT NOT NULL,
    model_name TEXT NOT NULL,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
    UNIQUE(session_id, task_type)
)
"#;

pub const CREATE_API_KEYS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS api_keys (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    provider TEXT NOT NULL,
    encrypted_key TEXT NOT NULL,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
    UNIQUE(session_id, provider)
)
"#;

pub const CREATE_WORKFLOW_EXECUTIONS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS workflow_executions (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    pipeline_id TEXT NOT NULL,
    pipeline_name TEXT NOT NULL,
    status TEXT NOT NULL,
    start_time DATETIME NOT NULL,
    end_time DATETIME,
    loop_count INTEGER NOT NULL DEFAULT 0,
    error_message TEXT,
    execution_log TEXT,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
)
"#;

pub const CREATE_WORKFLOW_SNAPSHOTS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS workflow_snapshots (
    snapshot_id TEXT PRIMARY KEY,
    execution_id TEXT NOT NULL,
    node_id TEXT NOT NULL,
    node_name TEXT NOT NULL,
    snapshot_data TEXT NOT NULL,
    timestamp DATETIME NOT NULL,
    FOREIGN KEY (execution_id) REFERENCES workflow_executions(id) ON DELETE CASCADE
)
"#;

pub const CREATE_CHAT_SESSIONS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS chat_sessions (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    title TEXT NOT NULL DEFAULT 'New Chat',
    created_at DATETIME NOT NULL,
    last_message_at DATETIME NOT NULL,
    message_count INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
)
"#;

pub const CREATE_CHAT_MESSAGES_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS chat_messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    chat_session_id TEXT NOT NULL,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    timestamp DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (chat_session_id) REFERENCES chat_sessions(id) ON DELETE CASCADE
)
"#;

pub const CREATE_INDEXES: &[&str] = &[
    "CREATE INDEX IF NOT EXISTS idx_model_settings_session ON model_settings(session_id)",
    "CREATE INDEX IF NOT EXISTS idx_api_keys_session ON api_keys(session_id)",
    "CREATE INDEX IF NOT EXISTS idx_workflow_executions_session ON workflow_executions(session_id)",
    "CREATE INDEX IF NOT EXISTS idx_workflow_snapshots_execution ON workflow_snapshots(execution_id)",
    "CREATE INDEX IF NOT EXISTS idx_chat_messages_session ON chat_messages(chat_session_id)",
    "CREATE INDEX IF NOT EXISTS idx_chat_sessions_berrycode_session ON chat_sessions(session_id)",
];
