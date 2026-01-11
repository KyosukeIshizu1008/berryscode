// Application Database Integration Tests
// Tests for session management, settings persistence, and workflow logging

use berry_editor_tauri::app_database::*;
use chrono::Utc;
use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::NamedTempFile;

#[test]
fn test_create_and_get_session() {
    let temp_file = NamedTempFile::new().unwrap();
    let db = AppDatabase::new(temp_file.path()).unwrap();

    let session_id = "test-session-1";
    let project_root = PathBuf::from("/test/project");

    // Create session
    db.create_session(session_id, &project_root).unwrap();

    // Get session
    let session = db.get_session(session_id).unwrap().unwrap();
    assert_eq!(session.id, session_id);
    assert_eq!(session.project_root, project_root);
}

#[test]
fn test_list_sessions() {
    let temp_file = NamedTempFile::new().unwrap();
    let db = AppDatabase::new(temp_file.path()).unwrap();

    // Create multiple sessions
    db.create_session("session-1", &PathBuf::from("/project1"))
        .unwrap();
    db.create_session("session-2", &PathBuf::from("/project2"))
        .unwrap();
    db.create_session("session-3", &PathBuf::from("/project3"))
        .unwrap();

    // List sessions
    let sessions = db.list_sessions().unwrap();
    assert_eq!(sessions.len(), 3);
}

#[test]
fn test_update_session_activity() {
    let temp_file = NamedTempFile::new().unwrap();
    let db = AppDatabase::new(temp_file.path()).unwrap();

    let session_id = "test-session";
    db.create_session(session_id, &PathBuf::from("/test")).unwrap();

    let initial = db.get_session(session_id).unwrap().unwrap();

    // Sleep to ensure time difference
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Update activity
    db.update_session_activity(session_id).unwrap();

    let updated = db.get_session(session_id).unwrap().unwrap();
    assert!(updated.last_activity > initial.last_activity);
}

#[test]
fn test_recent_projects() {
    let temp_file = NamedTempFile::new().unwrap();
    let db = AppDatabase::new(temp_file.path()).unwrap();

    // Create sessions (which updates recent projects)
    db.create_session("s1", &PathBuf::from("/project1")).unwrap();
    db.create_session("s2", &PathBuf::from("/project2")).unwrap();
    db.create_session("s3", &PathBuf::from("/project1")).unwrap(); // Duplicate project

    // Get recent projects
    let projects = db.get_recent_projects(10).unwrap();
    assert_eq!(projects.len(), 2); // Only 2 unique projects

    // Most recent should be project1 (accessed twice)
    assert_eq!(projects[0].project_root, PathBuf::from("/project1"));
    assert_eq!(projects[0].session_count, 2);
}

#[test]
fn test_model_settings() {
    let temp_file = NamedTempFile::new().unwrap();
    let db = AppDatabase::new(temp_file.path()).unwrap();

    let session_id = "test-session";
    db.create_session(session_id, &PathBuf::from("/test")).unwrap();

    // Initialize defaults
    db.init_default_model_settings(session_id).unwrap();

    // Get settings
    let settings = db.get_model_settings(session_id).unwrap();
    assert_eq!(settings.len(), 5);
    assert_eq!(settings.get("design"), Some(&"gpt-5.1-high".to_string()));
    assert_eq!(settings.get("test"), Some(&"grok-4-fast".to_string()));
}

#[test]
fn test_save_model_settings() {
    let temp_file = NamedTempFile::new().unwrap();
    let db = AppDatabase::new(temp_file.path()).unwrap();

    let session_id = "test-session";
    db.create_session(session_id, &PathBuf::from("/test")).unwrap();

    // Save custom settings
    let mut custom_settings = HashMap::new();
    custom_settings.insert("design".to_string(), "gpt-4o".to_string());
    custom_settings.insert("implementation".to_string(), "claude-3-5-sonnet-20241022".to_string());
    custom_settings.insert("review".to_string(), "gpt-4o".to_string());
    custom_settings.insert("test".to_string(), "gpt-4o-mini".to_string());
    custom_settings.insert("debug".to_string(), "claude-3-haiku-20240307".to_string());

    db.save_model_settings(session_id, &custom_settings).unwrap();

    // Verify saved settings
    let retrieved = db.get_model_settings(session_id).unwrap();
    assert_eq!(retrieved.get("design"), Some(&"gpt-4o".to_string()));
    assert_eq!(retrieved.get("implementation"), Some(&"claude-3-5-sonnet-20241022".to_string()));
}

#[test]
fn test_get_model_for_task() {
    let temp_file = NamedTempFile::new().unwrap();
    let db = AppDatabase::new(temp_file.path()).unwrap();

    let session_id = "test-session";
    db.create_session(session_id, &PathBuf::from("/test")).unwrap();
    db.init_default_model_settings(session_id).unwrap();

    let model = db.get_model_for_task(session_id, "design").unwrap();
    assert_eq!(model, Some("gpt-5.1-high".to_string()));

    let nonexistent = db.get_model_for_task(session_id, "invalid-task").unwrap();
    assert_eq!(nonexistent, None);
}

#[test]
fn test_api_keys() {
    let temp_file = NamedTempFile::new().unwrap();
    let db = AppDatabase::new(temp_file.path()).unwrap();

    let session_id = "test-session";
    db.create_session(session_id, &PathBuf::from("/test")).unwrap();

    // Save API keys
    db.save_api_key(session_id, "openai", "encrypted-key-123").unwrap();
    db.save_api_key(session_id, "anthropic", "encrypted-key-456").unwrap();

    // Get single key
    let openai_key = db.get_api_key(session_id, "openai").unwrap();
    assert_eq!(openai_key, Some("encrypted-key-123".to_string()));

    // Get all keys
    let all_keys = db.get_all_api_keys(session_id).unwrap();
    assert_eq!(all_keys.len(), 2);
    assert!(all_keys.contains_key("openai"));
    assert!(all_keys.contains_key("anthropic"));

    // Delete key
    db.delete_api_key(session_id, "openai").unwrap();
    let deleted = db.get_api_key(session_id, "openai").unwrap();
    assert_eq!(deleted, None);
}

#[test]
fn test_workflow_execution() {
    let temp_file = NamedTempFile::new().unwrap();
    let db = AppDatabase::new(temp_file.path()).unwrap();

    let session_id = "test-session";
    db.create_session(session_id, &PathBuf::from("/test")).unwrap();

    // Create workflow execution
    let execution = WorkflowExecution {
        id: "exec-1".to_string(),
        session_id: session_id.to_string(),
        pipeline_id: "tdd-loop".to_string(),
        pipeline_name: "TDD Loop".to_string(),
        status: "running".to_string(),
        start_time: Utc::now(),
        end_time: None,
        loop_count: 0,
        error_message: None,
        execution_log: Some("Started execution".to_string()),
    };

    db.create_workflow_execution(&execution).unwrap();

    // Get executions
    let executions = db.get_workflow_executions(session_id, 10).unwrap();
    assert_eq!(executions.len(), 1);
    assert_eq!(executions[0].pipeline_id, "tdd-loop");
    assert_eq!(executions[0].status, "running");
}

#[test]
fn test_update_workflow_execution_status() {
    let temp_file = NamedTempFile::new().unwrap();
    let db = AppDatabase::new(temp_file.path()).unwrap();

    let session_id = "test-session";
    db.create_session(session_id, &PathBuf::from("/test")).unwrap();

    let execution = WorkflowExecution {
        id: "exec-1".to_string(),
        session_id: session_id.to_string(),
        pipeline_id: "test".to_string(),
        pipeline_name: "Test".to_string(),
        status: "running".to_string(),
        start_time: Utc::now(),
        end_time: None,
        loop_count: 0,
        error_message: None,
        execution_log: None,
    };

    db.create_workflow_execution(&execution).unwrap();

    // Update status to completed
    let end_time = Utc::now();
    db.update_workflow_execution_status("exec-1", "completed", Some(end_time), None)
        .unwrap();

    // Verify update
    let executions = db.get_workflow_executions(session_id, 10).unwrap();
    assert_eq!(executions[0].status, "completed");
    assert!(executions[0].end_time.is_some());
}

#[test]
fn test_workflow_snapshots() {
    let temp_file = NamedTempFile::new().unwrap();
    let db = AppDatabase::new(temp_file.path()).unwrap();

    let session_id = "test-session";
    db.create_session(session_id, &PathBuf::from("/test")).unwrap();

    // Create execution first
    let execution = WorkflowExecution {
        id: "exec-1".to_string(),
        session_id: session_id.to_string(),
        pipeline_id: "test".to_string(),
        pipeline_name: "Test".to_string(),
        status: "running".to_string(),
        start_time: Utc::now(),
        end_time: None,
        loop_count: 0,
        error_message: None,
        execution_log: None,
    };

    db.create_workflow_execution(&execution).unwrap();

    // Save snapshots
    let snapshot1 = WorkflowSnapshot {
        snapshot_id: "snap-1".to_string(),
        execution_id: "exec-1".to_string(),
        node_id: "node-1".to_string(),
        node_name: "Design".to_string(),
        snapshot_data: r#"{"files": []}"#.to_string(),
        timestamp: Utc::now(),
    };

    let snapshot2 = WorkflowSnapshot {
        snapshot_id: "snap-2".to_string(),
        execution_id: "exec-1".to_string(),
        node_id: "node-2".to_string(),
        node_name: "Implement".to_string(),
        snapshot_data: r#"{"files": ["main.rs"]}"#.to_string(),
        timestamp: Utc::now(),
    };

    db.save_workflow_snapshot(&snapshot1).unwrap();
    db.save_workflow_snapshot(&snapshot2).unwrap();

    // Get snapshots
    let snapshots = db.get_workflow_snapshots("exec-1").unwrap();
    assert_eq!(snapshots.len(), 2);
    assert_eq!(snapshots[0].node_name, "Design");
    assert_eq!(snapshots[1].node_name, "Implement");
}

#[test]
fn test_model_settings_default() {
    let defaults = ModelSettings::default();
    assert_eq!(defaults.design, "gpt-5.1-high");
    assert_eq!(defaults.implementation, "gpt-5.1-high");
    assert_eq!(defaults.review, "claude-4.5-sonnet");
    assert_eq!(defaults.test, "grok-4-fast");
    assert_eq!(defaults.debug, "gemini-2.5-flash-lite");
}

#[test]
fn test_model_settings_to_hashmap() {
    let settings = ModelSettings::default();
    let map = settings.to_hashmap();
    assert_eq!(map.len(), 5);
    assert_eq!(map.get("design"), Some(&"gpt-5.1-high".to_string()));
}

#[test]
fn test_model_settings_from_hashmap() {
    let mut map = HashMap::new();
    map.insert("design".to_string(), "custom-model".to_string());
    map.insert("test".to_string(), "test-model".to_string());

    let settings = ModelSettings::from_hashmap(&map);
    assert_eq!(settings.design, "custom-model");
    assert_eq!(settings.test, "test-model");
    // Others should fallback to defaults
    assert_eq!(settings.implementation, "gpt-5.1-high");
}

#[test]
fn test_session_isolation() {
    let temp_file = NamedTempFile::new().unwrap();
    let db = AppDatabase::new(temp_file.path()).unwrap();

    // Create two sessions
    db.create_session("session-1", &PathBuf::from("/project1"))
        .unwrap();
    db.create_session("session-2", &PathBuf::from("/project2"))
        .unwrap();

    // Set different settings for each session
    let mut settings1 = HashMap::new();
    settings1.insert("design".to_string(), "gpt-4o".to_string());
    db.save_model_settings("session-1", &settings1).unwrap();

    let mut settings2 = HashMap::new();
    settings2.insert("design".to_string(), "claude-4.5-sonnet".to_string());
    db.save_model_settings("session-2", &settings2).unwrap();

    // Verify isolation
    let s1_settings = db.get_model_settings("session-1").unwrap();
    let s2_settings = db.get_model_settings("session-2").unwrap();

    assert_eq!(s1_settings.get("design"), Some(&"gpt-4o".to_string()));
    assert_eq!(s2_settings.get("design"), Some(&"claude-4.5-sonnet".to_string()));
}
