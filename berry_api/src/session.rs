use crate::berry_api::StartSessionRequest;
use std::collections::HashMap;

/// Sessions inactive longer than this are eligible for cleanup.
const SESSION_TTL_HOURS: i64 = 24;

pub struct Session {
    pub id: String,
    pub request: StartSessionRequest,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub struct SessionManager {
    sessions: HashMap<String, Session>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    pub fn create_session(&mut self, session_id: String, request: StartSessionRequest) {
        let session = Session {
            id: session_id.clone(),
            request,
            created_at: chrono::Utc::now(),
        };

        self.sessions.insert(session_id, session);
    }

    pub fn get_session(&self, session_id: &str) -> Option<&Session> {
        self.sessions.get(session_id)
    }

    pub fn remove_session(&mut self, session_id: &str) -> Option<Session> {
        self.sessions.remove(session_id)
    }

    /// Remove sessions older than SESSION_TTL_HOURS.
    /// Called on each new session creation so no background task is needed.
    pub fn cleanup_expired(&mut self) {
        let ttl = chrono::Duration::hours(SESSION_TTL_HOURS);
        let now = chrono::Utc::now();
        let before = self.sessions.len();
        self.sessions.retain(|_, s| now - s.created_at < ttl);
        let removed = before - self.sessions.len();
        if removed > 0 {
            tracing::info!("🗑 Cleaned up {} expired session(s)", removed);
        }
    }
}
