//! Collaboration session management
//!
//! Manages collaboration sessions, users, and permissions

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// User role in a collaboration session
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    Host,
    Editor,
    ReadOnly,
}

/// User in a collaboration session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
    pub role: UserRole,
    pub color: String,
    pub cursor_position: Option<CursorPosition>,
    pub joined_at: chrono::DateTime<chrono::Utc>,
}

/// Cursor position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPosition {
    pub line: u32,
    pub column: u32,
    pub file_path: Option<String>,
}

/// Collaboration session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationSession {
    pub id: String,
    pub name: String,
    pub host_id: String,
    pub users: HashMap<String, User>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub project_root: String,
    pub invite_code: String,
    pub max_users: usize,
}

impl CollaborationSession {
    /// Create a new collaboration session
    pub fn new(name: String, host_user: User, project_root: String) -> Self {
        let id = Uuid::new_v4().to_string();
        let invite_code = generate_invite_code();
        let host_id = host_user.id.clone();
        let mut users = HashMap::new();
        users.insert(host_user.id.clone(), host_user);

        Self {
            id,
            name,
            host_id,
            users,
            created_at: chrono::Utc::now(),
            project_root,
            invite_code,
            max_users: 10,
        }
    }

    /// Add a user to the session
    pub fn add_user(&mut self, user: User) -> Result<(), String> {
        if self.users.len() >= self.max_users {
            return Err("Session is full".to_string());
        }

        if self.users.contains_key(&user.id) {
            return Err("User already in session".to_string());
        }

        self.users.insert(user.id.clone(), user);
        Ok(())
    }

    /// Remove a user from the session
    pub fn remove_user(&mut self, user_id: &str) -> Result<User, String> {
        if user_id == self.host_id {
            return Err("Cannot remove host".to_string());
        }

        self.users
            .remove(user_id)
            .ok_or_else(|| "User not found".to_string())
    }

    /// Update user cursor position
    pub fn update_cursor(&mut self, user_id: &str, cursor: CursorPosition) -> Result<(), String> {
        self.users
            .get_mut(user_id)
            .map(|user| {
                user.cursor_position = Some(cursor);
            })
            .ok_or_else(|| "User not found".to_string())
    }

    /// Change user role (host only)
    pub fn change_user_role(
        &mut self,
        requester_id: &str,
        target_user_id: &str,
        new_role: UserRole,
    ) -> Result<(), String> {
        // Only host can change roles
        if requester_id != self.host_id {
            return Err("Only host can change roles".to_string());
        }

        // Cannot change host role
        if target_user_id == self.host_id {
            return Err("Cannot change host role".to_string());
        }

        self.users
            .get_mut(target_user_id)
            .map(|user| {
                user.role = new_role;
            })
            .ok_or_else(|| "User not found".to_string())
    }

    /// Get all users except the specified user (for broadcasting)
    pub fn get_other_users(&self, user_id: &str) -> Vec<&User> {
        self.users
            .values()
            .filter(|user| user.id != user_id)
            .collect()
    }
}

/// Session manager
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, CollaborationSession>>>,
    invite_code_to_session: Arc<RwLock<HashMap<String, String>>>,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            invite_code_to_session: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new collaboration session
    pub async fn create_session(
        &self,
        name: String,
        host_user: User,
        project_root: String,
    ) -> Result<CollaborationSession, String> {
        let session = CollaborationSession::new(name, host_user, project_root);
        let session_id = session.id.clone();
        let invite_code = session.invite_code.clone();

        let mut sessions = self.sessions.write().await;
        let mut invite_map = self.invite_code_to_session.write().await;

        sessions.insert(session_id.clone(), session.clone());
        invite_map.insert(invite_code, session_id);

        Ok(session)
    }

    /// Join a session by invite code
    pub async fn join_session(
        &self,
        invite_code: &str,
        user: User,
    ) -> Result<CollaborationSession, String> {
        let invite_map = self.invite_code_to_session.read().await;
        let session_id = invite_map
            .get(invite_code)
            .ok_or_else(|| "Invalid invite code".to_string())?;

        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| "Session not found".to_string())?;

        session.add_user(user)?;
        Ok(session.clone())
    }

    /// Leave a session
    pub async fn leave_session(&self, session_id: &str, user_id: &str) -> Result<(), String> {
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| "Session not found".to_string())?;

        session.remove_user(user_id)?;

        // If no users left, remove the session
        if session.users.is_empty() {
            let invite_code = session.invite_code.clone();
            sessions.remove(session_id);
            let mut invite_map = self.invite_code_to_session.write().await;
            invite_map.remove(&invite_code);
        }

        Ok(())
    }

    /// Get a session by ID
    pub async fn get_session(&self, session_id: &str) -> Option<CollaborationSession> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).cloned()
    }

    /// Update cursor position
    pub async fn update_cursor(
        &self,
        session_id: &str,
        user_id: &str,
        cursor: CursorPosition,
    ) -> Result<(), String> {
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| "Session not found".to_string())?;

        session.update_cursor(user_id, cursor)
    }

    /// Change user role
    pub async fn change_user_role(
        &self,
        session_id: &str,
        requester_id: &str,
        target_user_id: &str,
        new_role: UserRole,
    ) -> Result<(), String> {
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| "Session not found".to_string())?;

        session.change_user_role(requester_id, target_user_id, new_role)
    }

    /// Get all sessions for a user
    pub async fn get_user_sessions(&self, user_id: &str) -> Vec<CollaborationSession> {
        let sessions = self.sessions.read().await;
        sessions
            .values()
            .filter(|session| session.users.contains_key(user_id))
            .cloned()
            .collect()
    }

    /// Clean up expired sessions (TTL: 24 hours)
    pub async fn cleanup_expired_sessions(&self) {
        let mut sessions = self.sessions.write().await;
        let mut invite_map = self.invite_code_to_session.write().await;

        let now = chrono::Utc::now();
        let expired_sessions: Vec<String> = sessions
            .iter()
            .filter(|(_, session)| {
                now.signed_duration_since(session.created_at).num_hours() > 24
            })
            .map(|(id, _)| id.clone())
            .collect();

        for session_id in expired_sessions {
            if let Some(session) = sessions.remove(&session_id) {
                invite_map.remove(&session.invite_code);
            }
        }
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate a random invite code (6 characters)
fn generate_invite_code() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut rng = rand::thread_rng();

    (0..6)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_and_join_session() {
        let manager = SessionManager::new();

        let host = User {
            id: "host1".to_string(),
            name: "Host".to_string(),
            role: UserRole::Host,
            color: "#FF0000".to_string(),
            cursor_position: None,
            joined_at: chrono::Utc::now(),
        };

        let session = manager
            .create_session("Test Session".to_string(), host, "/tmp/test".to_string())
            .await
            .unwrap();

        let invite_code = session.invite_code.clone();

        let guest = User {
            id: "guest1".to_string(),
            name: "Guest".to_string(),
            role: UserRole::Editor,
            color: "#00FF00".to_string(),
            cursor_position: None,
            joined_at: chrono::Utc::now(),
        };

        let joined_session = manager.join_session(&invite_code, guest).await.unwrap();

        assert_eq!(joined_session.users.len(), 2);
    }

    #[tokio::test]
    async fn test_leave_session() {
        let manager = SessionManager::new();

        let host = User {
            id: "host1".to_string(),
            name: "Host".to_string(),
            role: UserRole::Host,
            color: "#FF0000".to_string(),
            cursor_position: None,
            joined_at: chrono::Utc::now(),
        };

        let session = manager
            .create_session("Test Session".to_string(), host, "/tmp/test".to_string())
            .await
            .unwrap();

        let invite_code = session.invite_code.clone();
        let session_id = session.id.clone();

        let guest = User {
            id: "guest1".to_string(),
            name: "Guest".to_string(),
            role: UserRole::Editor,
            color: "#00FF00".to_string(),
            cursor_position: None,
            joined_at: chrono::Utc::now(),
        };

        manager.join_session(&invite_code, guest).await.unwrap();

        manager.leave_session(&session_id, "guest1").await.unwrap();

        let updated_session = manager.get_session(&session_id).await.unwrap();
        assert_eq!(updated_session.users.len(), 1);
    }
}
