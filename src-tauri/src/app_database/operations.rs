use super::schema::*;
use super::types::*;
use anyhow::Result;
use chrono::Utc;
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct AppDatabase {
    conn: Arc<Mutex<Connection>>,
}

impl AppDatabase {
    /// Initialize database with schema
    pub fn new<P: AsRef<Path>>(database_path: P) -> Result<Self> {
        let conn = Connection::open(database_path)?;

        // Create tables
        conn.execute(CREATE_SESSIONS_TABLE, [])?;
        conn.execute(CREATE_RECENT_PROJECTS_TABLE, [])?;
        conn.execute(CREATE_MODEL_SETTINGS_TABLE, [])?;
        conn.execute(CREATE_API_KEYS_TABLE, [])?;
        conn.execute(CREATE_WORKFLOW_EXECUTIONS_TABLE, [])?;
        conn.execute(CREATE_WORKFLOW_SNAPSHOTS_TABLE, [])?;
        conn.execute(CREATE_CHAT_SESSIONS_TABLE, [])?;
        conn.execute(CREATE_CHAT_MESSAGES_TABLE, [])?;

        // Create indexes
        for index_sql in CREATE_INDEXES {
            conn.execute(index_sql, [])?;
        }

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    // ============================================================================
    // Session Management
    // ============================================================================

    pub fn create_session(&self, session_id: &str, project_root: &PathBuf) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO sessions (id, project_root, created_at, last_activity) VALUES (?1, ?2, ?3, ?4)",
            params![session_id, project_root.to_string_lossy().as_ref(), &now, &now],
        )?;

        // Update recent projects
        drop(conn);
        self.update_recent_project(project_root)?;

        Ok(())
    }

    pub fn get_session(&self, session_id: &str) -> Result<Option<SessionData>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, project_root, created_at, last_activity FROM sessions WHERE id = ?1"
        )?;

        let mut rows = stmt.query(params![session_id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(SessionData {
                id: row.get(0)?,
                project_root: PathBuf::from(row.get::<_, String>(1)?),
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(2)?)?
                    .with_timezone(&Utc),
                last_activity: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)?
                    .with_timezone(&Utc),
            }))
        } else {
            Ok(None)
        }
    }

    pub fn update_session_activity(&self, session_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE sessions SET last_activity = ?1 WHERE id = ?2",
            params![Utc::now().to_rfc3339(), session_id],
        )?;
        Ok(())
    }

    pub fn list_sessions(&self) -> Result<Vec<SessionData>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, project_root, created_at, last_activity FROM sessions ORDER BY last_activity DESC"
        )?;

        let sessions = stmt.query_map([], |row| {
            Ok(SessionData {
                id: row.get(0)?,
                project_root: PathBuf::from(row.get::<_, String>(1)?),
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(2)?)
                    .unwrap()
                    .with_timezone(&Utc),
                last_activity: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                    .unwrap()
                    .with_timezone(&Utc),
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(sessions)
    }

    // ============================================================================
    // Recent Projects
    // ============================================================================

    fn update_recent_project(&self, project_root: &PathBuf) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let project_str = project_root.to_string_lossy();
        let now = Utc::now().to_rfc3339();

        // Try to update existing record
        let affected = conn.execute(
            "UPDATE recent_projects SET last_accessed = ?1, session_count = session_count + 1 WHERE project_root = ?2",
            params![&now, &*project_str],
        )?;

        // If no rows affected, insert new record
        if affected == 0 {
            conn.execute(
                "INSERT INTO recent_projects (project_root, last_accessed, session_count) VALUES (?1, ?2, 1)",
                params![&*project_str, &now],
            )?;
        }

        Ok(())
    }

    pub fn get_recent_projects(&self, limit: usize) -> Result<Vec<RecentProject>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT project_root, last_accessed, session_count FROM recent_projects ORDER BY last_accessed DESC LIMIT ?1"
        )?;

        let projects = stmt.query_map(params![limit], |row| {
            Ok(RecentProject {
                project_root: PathBuf::from(row.get::<_, String>(0)?),
                last_accessed: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(1)?)
                    .unwrap()
                    .with_timezone(&Utc),
                session_count: row.get(2)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(projects)
    }

    // ============================================================================
    // Model Settings
    // ============================================================================

    pub fn init_default_model_settings(&self, session_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let defaults = ModelSettings::default();
        let settings_map = defaults.to_hashmap();

        for (task_type, model_name) in settings_map {
            // Check if already exists
            let exists: i64 = conn.query_row(
                "SELECT COUNT(*) FROM model_settings WHERE session_id = ?1 AND task_type = ?2",
                params![session_id, &task_type],
                |row| row.get(0),
            )?;

            if exists == 0 {
                conn.execute(
                    "INSERT INTO model_settings (session_id, task_type, model_name) VALUES (?1, ?2, ?3)",
                    params![session_id, &task_type, &model_name],
                )?;
            }
        }

        Ok(())
    }

    pub fn get_model_settings(&self, session_id: &str) -> Result<HashMap<String, String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT task_type, model_name FROM model_settings WHERE session_id = ?1"
        )?;

        let mut settings = HashMap::new();
        let rows = stmt.query_map(params![session_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        for row in rows {
            let (task_type, model_name) = row?;
            settings.insert(task_type, model_name);
        }

        // If empty, return defaults
        if settings.is_empty() {
            settings = ModelSettings::default().to_hashmap();
        }

        Ok(settings)
    }

    pub fn save_model_settings(
        &self,
        session_id: &str,
        settings: &HashMap<String, String>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();

        for (task_type, model_name) in settings {
            conn.execute(
                "INSERT INTO model_settings (session_id, task_type, model_name, updated_at)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(session_id, task_type) DO UPDATE SET
                 model_name = excluded.model_name,
                 updated_at = excluded.updated_at",
                params![session_id, task_type, model_name, &now],
            )?;
        }

        Ok(())
    }

    pub fn get_model_for_task(&self, session_id: &str, task_type: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let result = conn.query_row(
            "SELECT model_name FROM model_settings WHERE session_id = ?1 AND task_type = ?2",
            params![session_id, task_type],
            |row| row.get(0),
        );

        match result {
            Ok(model_name) => Ok(Some(model_name)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    // ============================================================================
    // API Keys (encrypted storage)
    // ============================================================================

    pub fn save_api_key(
        &self,
        session_id: &str,
        provider: &str,
        encrypted_key: &str,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO api_keys (session_id, provider, encrypted_key, updated_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(session_id, provider) DO UPDATE SET
             encrypted_key = excluded.encrypted_key,
             updated_at = excluded.updated_at",
            params![session_id, provider, encrypted_key, &now],
        )?;

        Ok(())
    }

    pub fn get_api_key(&self, session_id: &str, provider: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let result = conn.query_row(
            "SELECT encrypted_key FROM api_keys WHERE session_id = ?1 AND provider = ?2",
            params![session_id, provider],
            |row| row.get(0),
        );

        match result {
            Ok(key) => Ok(Some(key)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn get_all_api_keys(&self, session_id: &str) -> Result<HashMap<String, String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT provider, encrypted_key FROM api_keys WHERE session_id = ?1"
        )?;

        let mut keys = HashMap::new();
        let rows = stmt.query_map(params![session_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        for row in rows {
            let (provider, encrypted_key) = row?;
            keys.insert(provider, encrypted_key);
        }

        Ok(keys)
    }

    pub fn delete_api_key(&self, session_id: &str, provider: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM api_keys WHERE session_id = ?1 AND provider = ?2",
            params![session_id, provider],
        )?;
        Ok(())
    }

    // ============================================================================
    // Workflow Executions
    // ============================================================================

    pub fn create_workflow_execution(&self, execution: &WorkflowExecution) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO workflow_executions (id, session_id, pipeline_id, pipeline_name, status, start_time, end_time, loop_count, error_message, execution_log)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                &execution.id,
                &execution.session_id,
                &execution.pipeline_id,
                &execution.pipeline_name,
                &execution.status,
                execution.start_time.to_rfc3339(),
                execution.end_time.as_ref().map(|dt| dt.to_rfc3339()),
                execution.loop_count as i64,
                &execution.error_message,
                &execution.execution_log
            ],
        )?;

        Ok(())
    }

    pub fn update_workflow_execution_status(
        &self,
        execution_id: &str,
        status: &str,
        end_time: Option<chrono::DateTime<Utc>>,
        error_message: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE workflow_executions SET status = ?1, end_time = ?2, error_message = ?3 WHERE id = ?4",
            params![status, end_time.as_ref().map(|dt| dt.to_rfc3339()), error_message, execution_id],
        )?;

        Ok(())
    }

    pub fn get_workflow_executions(
        &self,
        session_id: &str,
        limit: usize,
    ) -> Result<Vec<WorkflowExecution>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, session_id, pipeline_id, pipeline_name, status, start_time, end_time, loop_count, error_message, execution_log
             FROM workflow_executions
             WHERE session_id = ?1
             ORDER BY start_time DESC
             LIMIT ?2"
        )?;

        let executions = stmt.query_map(params![session_id, limit], |row| {
            Ok(WorkflowExecution {
                id: row.get(0)?,
                session_id: row.get(1)?,
                pipeline_id: row.get(2)?,
                pipeline_name: row.get(3)?,
                status: row.get(4)?,
                start_time: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                    .unwrap()
                    .with_timezone(&Utc),
                end_time: row.get::<_, Option<String>>(6)?
                    .map(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                    .flatten()
                    .map(|dt| dt.with_timezone(&Utc)),
                loop_count: row.get::<_, i64>(7)? as usize,
                error_message: row.get(8)?,
                execution_log: row.get(9)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(executions)
    }

    // ============================================================================
    // Workflow Snapshots
    // ============================================================================

    pub fn save_workflow_snapshot(&self, snapshot: &WorkflowSnapshot) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO workflow_snapshots (snapshot_id, execution_id, node_id, node_name, snapshot_data, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                &snapshot.snapshot_id,
                &snapshot.execution_id,
                &snapshot.node_id,
                &snapshot.node_name,
                &snapshot.snapshot_data,
                snapshot.timestamp.to_rfc3339()
            ],
        )?;

        Ok(())
    }

    pub fn get_workflow_snapshots(&self, execution_id: &str) -> Result<Vec<WorkflowSnapshot>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT snapshot_id, execution_id, node_id, node_name, snapshot_data, timestamp
             FROM workflow_snapshots
             WHERE execution_id = ?1
             ORDER BY timestamp ASC"
        )?;

        let snapshots = stmt.query_map(params![execution_id], |row| {
            Ok(WorkflowSnapshot {
                snapshot_id: row.get(0)?,
                execution_id: row.get(1)?,
                node_id: row.get(2)?,
                node_name: row.get(3)?,
                snapshot_data: row.get(4)?,
                timestamp: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                    .unwrap()
                    .with_timezone(&Utc),
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(snapshots)
    }

    // ============================================================================
    // Chat History
    // ============================================================================

    /// Create a new chat session
    pub fn create_chat_session(&self, chat_id: &str, session_id: &str, title: Option<&str>) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        let title = title.unwrap_or("New Chat");

        conn.execute(
            "INSERT INTO chat_sessions (id, session_id, title, created_at, last_message_at, message_count) VALUES (?1, ?2, ?3, ?4, ?5, 0)",
            params![chat_id, session_id, title, &now, &now],
        )?;

        Ok(())
    }

    /// Get all chat sessions for a BerryCode session (ordered by last_message_at DESC)
    pub fn list_chat_sessions(&self, session_id: &str) -> Result<Vec<ChatSessionData>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, session_id, title, created_at, last_message_at, message_count
             FROM chat_sessions
             WHERE session_id = ?1
             ORDER BY last_message_at DESC"
        )?;

        let sessions = stmt.query_map(params![session_id], |row| {
            Ok(ChatSessionData {
                id: row.get(0)?,
                session_id: row.get(1)?,
                title: row.get(2)?,
                created_at: row.get(3)?,
                last_message_at: row.get(4)?,
                message_count: row.get(5)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(sessions)
    }

    /// Get a single chat session by ID
    pub fn get_chat_session(&self, chat_id: &str) -> Result<Option<ChatSessionData>> {
        let conn = self.conn.lock().unwrap();
        let result = conn.query_row(
            "SELECT id, session_id, title, created_at, last_message_at, message_count FROM chat_sessions WHERE id = ?1",
            params![chat_id],
            |row| {
                Ok(ChatSessionData {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    title: row.get(2)?,
                    created_at: row.get(3)?,
                    last_message_at: row.get(4)?,
                    message_count: row.get(5)?,
                })
            },
        );

        match result {
            Ok(session) => Ok(Some(session)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Update chat session metadata (title, last_message_at, message_count)
    pub fn update_chat_session(&self, chat_id: &str, title: Option<&str>) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();

        if let Some(title) = title {
            conn.execute(
                "UPDATE chat_sessions SET title = ?1, last_message_at = ?2 WHERE id = ?3",
                params![title, &now, chat_id],
            )?;
        } else {
            conn.execute(
                "UPDATE chat_sessions SET last_message_at = ?1, message_count = message_count + 1 WHERE id = ?2",
                params![&now, chat_id],
            )?;
        }

        Ok(())
    }

    /// Delete a chat session and all its messages (CASCADE)
    pub fn delete_chat_session(&self, chat_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM chat_sessions WHERE id = ?1",
            params![chat_id],
        )?;
        Ok(())
    }

    /// Save a single message
    pub fn save_chat_message(&self, chat_id: &str, role: &str, content: &str) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO chat_messages (chat_session_id, role, content, timestamp) VALUES (?1, ?2, ?3, ?4)",
            params![chat_id, role, content, &now],
        )?;

        let message_id = conn.last_insert_rowid();

        // Update chat session metadata
        conn.execute(
            "UPDATE chat_sessions SET last_message_at = ?1, message_count = message_count + 1 WHERE id = ?2",
            params![&now, chat_id],
        )?;

        Ok(message_id)
    }

    /// Load all messages for a chat session (ordered by timestamp ASC)
    pub fn load_chat_messages(&self, chat_id: &str) -> Result<Vec<ChatMessageData>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, chat_session_id, role, content, timestamp
             FROM chat_messages
             WHERE chat_session_id = ?1
             ORDER BY timestamp ASC"
        )?;

        let messages = stmt.query_map(params![chat_id], |row| {
            Ok(ChatMessageData {
                id: row.get(0)?,
                chat_session_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                timestamp: row.get(4)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(messages)
    }

    /// Delete all messages in a chat session
    pub fn clear_chat_messages(&self, chat_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM chat_messages WHERE chat_session_id = ?1",
            params![chat_id],
        )?;

        // Reset message count
        conn.execute(
            "UPDATE chat_sessions SET message_count = 0 WHERE id = ?1",
            params![chat_id],
        )?;

        Ok(())
    }

    /// Search messages by content (for future "find in history" feature)
    pub fn search_chat_messages(&self, session_id: &str, query: &str) -> Result<Vec<ChatMessageData>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT m.id, m.chat_session_id, m.role, m.content, m.timestamp
             FROM chat_messages m
             INNER JOIN chat_sessions s ON m.chat_session_id = s.id
             WHERE s.session_id = ?1 AND m.content LIKE ?2
             ORDER BY m.timestamp DESC"
        )?;

        let search_pattern = format!("%{}%", query);
        let messages = stmt.query_map(params![session_id, search_pattern], |row| {
            Ok(ChatMessageData {
                id: row.get(0)?,
                chat_session_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                timestamp: row.get(4)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(messages)
    }
}
