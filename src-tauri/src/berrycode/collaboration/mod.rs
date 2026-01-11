//! Collaboration module for Live Share functionality
//!
//! Provides real-time collaborative editing with:
//! - Session management (create, join, leave)
//! - Operational Transformation for conflict-free text synchronization
//! - Cursor position sharing
//! - User presence tracking

pub mod session_manager;
pub mod ot_engine;

pub use session_manager::{CollaborationSession, SessionManager, User, UserRole};
pub use ot_engine::{Operation, OperationType, OTEngine};
