use crate::berry_api::StartSessionRequest;
use std::collections::HashMap;

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
}
