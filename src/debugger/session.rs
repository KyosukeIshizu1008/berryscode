//! Debug Session Management
//!
//! Manages debugging sessions via DAP (Debug Adapter Protocol).

use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use crate::common::async_bridge::TauriBridge;

/// Debug session state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DebugState {
    Stopped,
    Running,
    Paused,
    Stepping,
}

/// Breakpoint information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Breakpoint {
    pub id: String,
    pub file: PathBuf,
    pub line: u32,
    pub condition: Option<String>,
    pub verified: bool,
}

/// Stack frame information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackFrame {
    pub id: i64,
    pub name: String,
    pub file: Option<PathBuf>,
    pub line: Option<u32>,
    pub column: Option<u32>,
}

/// Variable scope (Local, Closure, Global, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scope {
    pub name: String,
    pub variables: Vec<Variable>,
}

/// Variable information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    pub name: String,
    pub value: String,
    pub type_name: Option<String>,
    pub children: Option<Vec<Variable>>,
}

/// Debug session manager
#[derive(Clone, Copy)]
pub struct DebugSession {
    pub session_id: RwSignal<Option<String>>,
    pub state: RwSignal<DebugState>,
    pub breakpoints: RwSignal<HashMap<PathBuf, Vec<Breakpoint>>>,
    pub stack_frames: RwSignal<Vec<StackFrame>>,
    pub scopes: RwSignal<Vec<Scope>>,
}

impl Default for DebugSession {
    fn default() -> Self {
        Self::new()
    }
}

impl DebugSession {
    /// Create a new debug session
    pub fn new() -> Self {
        Self {
            session_id: RwSignal::new(None),
            state: RwSignal::new(DebugState::Stopped),
            breakpoints: RwSignal::new(HashMap::new()),
            stack_frames: RwSignal::new(Vec::new()),
            scopes: RwSignal::new(Vec::new()),
        }
    }

    /// Start a debug session
    pub async fn start(&self, program_path: String) -> Result<String, String> {
        #[derive(Serialize)]
        struct StartArgs {
            program_path: String,
        }

        let session_id: String = TauriBridge::invoke("debug_start_session", StartArgs { program_path })
            .await
            .map_err(|e| e.to_string())?;

        self.session_id.set(Some(session_id.clone()));
        self.state.set(DebugState::Running);

        Ok(session_id)
    }

    /// Stop the debug session
    pub async fn stop(&self) -> Result<(), String> {
        if let Some(session_id) = self.session_id.get_untracked() {
            #[derive(Serialize)]
            struct StopArgs {
                session_id: String,
            }

            TauriBridge::invoke::<_, ()>("debug_stop_session", StopArgs { session_id })
                .await
                .map_err(|e| e.to_string())?;
        }

        self.session_id.set(None);
        self.state.set(DebugState::Stopped);
        self.stack_frames.set(Vec::new());
        self.scopes.set(Vec::new());

        Ok(())
    }

    /// Set a breakpoint
    pub async fn set_breakpoint(&self, file: PathBuf, line: u32, condition: Option<String>) -> Result<Breakpoint, String> {
        let session_id = self.session_id.get_untracked()
            .ok_or("No active debug session")?;

        #[derive(Serialize)]
        struct SetBreakpointArgs {
            session_id: String,
            file: String,
            line: u32,
            condition: Option<String>,
        }

        let breakpoint: Breakpoint = TauriBridge::invoke(
            "debug_set_breakpoint",
            SetBreakpointArgs {
                session_id,
                file: file.to_string_lossy().to_string(),
                line,
                condition,
            }
        )
        .await
        .map_err(|e| e.to_string())?;

        // Update local breakpoints
        self.breakpoints.update(|bps| {
            bps.entry(file.clone()).or_insert_with(Vec::new).push(breakpoint.clone());
        });

        Ok(breakpoint)
    }

    /// Remove a breakpoint
    pub async fn remove_breakpoint(&self, breakpoint_id: String) -> Result<(), String> {
        let session_id = self.session_id.get_untracked()
            .ok_or("No active debug session")?;

        #[derive(Serialize)]
        struct RemoveBreakpointArgs {
            session_id: String,
            breakpoint_id: String,
        }

        TauriBridge::invoke::<_, ()>(
            "debug_remove_breakpoint",
            RemoveBreakpointArgs { session_id, breakpoint_id: breakpoint_id.clone() }
        )
        .await
        .map_err(|e| e.to_string())?;

        // Update local breakpoints
        self.breakpoints.update(|bps| {
            for breakpoints in bps.values_mut() {
                breakpoints.retain(|bp| bp.id != breakpoint_id);
            }
        });

        Ok(())
    }

    /// Continue execution
    pub async fn continue_execution(&self) -> Result<(), String> {
        let session_id = self.session_id.get_untracked()
            .ok_or("No active debug session")?;

        #[derive(Serialize)]
        struct ContinueArgs {
            session_id: String,
        }

        TauriBridge::invoke::<_, ()>("debug_continue", ContinueArgs { session_id })
            .await
            .map_err(|e| e.to_string())?;
        self.state.set(DebugState::Running);

        Ok(())
    }

    /// Step over
    pub async fn step_over(&self) -> Result<(), String> {
        let session_id = self.session_id.get_untracked()
            .ok_or("No active debug session")?;

        #[derive(Serialize)]
        struct StepOverArgs {
            session_id: String,
        }

        TauriBridge::invoke::<_, ()>("debug_step_over", StepOverArgs { session_id })
            .await
            .map_err(|e| e.to_string())?;
        self.state.set(DebugState::Stepping);

        Ok(())
    }

    /// Step into
    pub async fn step_into(&self) -> Result<(), String> {
        let session_id = self.session_id.get_untracked()
            .ok_or("No active debug session")?;

        #[derive(Serialize)]
        struct StepIntoArgs {
            session_id: String,
        }

        TauriBridge::invoke::<_, ()>("debug_step_into", StepIntoArgs { session_id })
            .await
            .map_err(|e| e.to_string())?;
        self.state.set(DebugState::Stepping);

        Ok(())
    }

    /// Step out
    pub async fn step_out(&self) -> Result<(), String> {
        let session_id = self.session_id.get_untracked()
            .ok_or("No active debug session")?;

        #[derive(Serialize)]
        struct StepOutArgs {
            session_id: String,
        }

        TauriBridge::invoke::<_, ()>("debug_step_out", StepOutArgs { session_id })
            .await
            .map_err(|e| e.to_string())?;
        self.state.set(DebugState::Stepping);

        Ok(())
    }

    /// Get stack trace
    pub async fn get_stack_trace(&self) -> Result<Vec<StackFrame>, String> {
        let session_id = self.session_id.get_untracked()
            .ok_or("No active debug session")?;

        #[derive(Serialize)]
        struct GetStackTraceArgs {
            session_id: String,
        }

        let frames: Vec<StackFrame> = TauriBridge::invoke(
            "debug_get_stack_trace",
            GetStackTraceArgs { session_id }
        )
        .await
        .map_err(|e| e.to_string())?;

        self.stack_frames.set(frames.clone());

        Ok(frames)
    }

    /// Get variables for a frame
    pub async fn get_variables(&self, frame_id: i64) -> Result<Vec<Scope>, String> {
        let session_id = self.session_id.get_untracked()
            .ok_or("No active debug session")?;

        #[derive(Serialize)]
        struct GetVariablesArgs {
            session_id: String,
            frame_id: i64,
        }

        let scopes: Vec<Scope> = TauriBridge::invoke(
            "debug_get_variables",
            GetVariablesArgs { session_id, frame_id }
        )
        .await
        .map_err(|e| e.to_string())?;

        self.scopes.set(scopes.clone());

        Ok(scopes)
    }

    /// Evaluate expression in debug context
    pub async fn evaluate(&self, expression: String, frame_id: Option<i64>) -> Result<String, String> {
        let session_id = self.session_id.get_untracked()
            .ok_or("No active debug session")?;

        #[derive(Serialize)]
        struct EvaluateArgs {
            session_id: String,
            expression: String,
            frame_id: Option<i64>,
        }

        let result: String = TauriBridge::invoke(
            "debug_evaluate",
            EvaluateArgs { session_id, expression, frame_id }
        )
        .await
        .map_err(|e| e.to_string())?;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_session_new() {
        let session = DebugSession::new();
        assert_eq!(session.state.get_untracked(), DebugState::Stopped);
        assert!(session.session_id.get_untracked().is_none());
        assert_eq!(session.breakpoints.get_untracked().len(), 0);
        assert_eq!(session.stack_frames.get_untracked().len(), 0);
    }

    #[test]
    fn test_debug_state_equality() {
        assert_eq!(DebugState::Stopped, DebugState::Stopped);
        assert_ne!(DebugState::Stopped, DebugState::Running);
        assert_eq!(DebugState::Paused, DebugState::Paused);
        assert_eq!(DebugState::Stepping, DebugState::Stepping);
    }

    #[test]
    fn test_breakpoint_equality() {
        let bp1 = Breakpoint {
            id: "1".to_string(),
            file: PathBuf::from("test.rs"),
            line: 10,
            condition: None,
            verified: true,
        };
        let bp2 = Breakpoint {
            id: "1".to_string(),
            file: PathBuf::from("test.rs"),
            line: 10,
            condition: None,
            verified: true,
        };
        let bp3 = Breakpoint {
            id: "2".to_string(),
            file: PathBuf::from("test.rs"),
            line: 10,
            condition: None,
            verified: true,
        };

        assert_eq!(bp1, bp2);
        assert_ne!(bp1, bp3);
    }

    #[test]
    fn test_stack_frame_creation() {
        let frame = StackFrame {
            id: 1,
            name: "main".to_string(),
            file: Some(PathBuf::from("main.rs")),
            line: Some(42),
            column: Some(10),
        };

        assert_eq!(frame.id, 1);
        assert_eq!(frame.name, "main");
        assert_eq!(frame.line, Some(42));
    }

    #[test]
    fn test_variable_with_children() {
        let child1 = Variable {
            name: "x".to_string(),
            value: "10".to_string(),
            type_name: Some("i32".to_string()),
            children: None,
        };

        let parent = Variable {
            name: "struct_var".to_string(),
            value: "MyStruct".to_string(),
            type_name: Some("MyStruct".to_string()),
            children: Some(vec![child1]),
        };

        assert!(parent.children.is_some());
        assert_eq!(parent.children.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_scope_creation() {
        let scope = Scope {
            name: "Local".to_string(),
            variables: vec![
                Variable {
                    name: "x".to_string(),
                    value: "42".to_string(),
                    type_name: Some("i32".to_string()),
                    children: None,
                },
            ],
        };

        assert_eq!(scope.name, "Local");
        assert_eq!(scope.variables.len(), 1);
    }
}
