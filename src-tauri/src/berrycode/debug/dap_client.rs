//! DAP (Debug Adapter Protocol) client for debugging support
//!
//! This module provides debugging capabilities by communicating with
//! debug adapters like lldb-vscode, codelldb, etc.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

/// DAP client that manages debug adapter processes
#[derive(Clone)]
pub struct DapClient {
    /// Root directory of the project
    project_root: PathBuf,
    /// Active debug sessions (by session ID)
    sessions: Arc<Mutex<HashMap<String, DebugSession>>>,
    /// Next request ID
    next_id: Arc<Mutex<i64>>,
}

/// A running debug session
pub struct DebugSession {
    /// The debug adapter process
    pub process: Child,
    /// Current thread ID
    pub thread_id: Option<i64>,
    /// Breakpoints (file path -> line numbers)
    pub breakpoints: HashMap<PathBuf, Vec<i64>>,
    /// Conditional breakpoints (file path -> (line, condition))
    pub conditional_breakpoints: HashMap<PathBuf, Vec<(i64, String)>>,
    /// Logpoints (file path -> (line, message))
    pub logpoints: HashMap<PathBuf, Vec<(i64, String)>>,
    /// Watch expressions
    pub watch_expressions: Vec<WatchExpression>,
    /// Stack frames for current thread
    pub stack_frames: Vec<StackFrame>,
    /// Variables scopes
    pub scopes: Vec<Scope>,
    /// Is the debugger stopped (at breakpoint or step)?
    pub is_stopped: bool,
    /// Current stopped reason
    pub stop_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackFrame {
    pub id: i64,
    pub name: String,
    pub source: Option<Source>,
    pub line: i64,
    pub column: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub name: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scope {
    pub name: String,
    pub variables_reference: i64,
    pub expensive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    pub name: String,
    pub value: String,
    #[serde(rename = "type")]
    pub var_type: Option<String>,
    pub variables_reference: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchExpression {
    pub id: String,
    pub expression: String,
    pub value: Option<String>,
    #[serde(rename = "type")]
    pub var_type: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationResult {
    pub result: String,
    #[serde(rename = "type")]
    pub var_type: Option<String>,
    pub variables_reference: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct DapRequest {
    seq: i64,
    #[serde(rename = "type")]
    msg_type: String,
    command: String,
    arguments: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DapResponse {
    seq: i64,
    #[serde(rename = "type")]
    msg_type: String,
    request_seq: i64,
    success: bool,
    command: String,
    message: Option<String>,
    body: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DapEvent {
    seq: i64,
    #[serde(rename = "type")]
    msg_type: String,
    event: String,
    body: Option<serde_json::Value>,
}

impl DapClient {
    /// Create a new DAP client for the given project
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            project_root,
            sessions: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(1)),
        }
    }

    /// Get next request ID
    fn next_request_id(&self) -> i64 {
        let mut id = self.next_id.lock().unwrap();
        let current = *id;
        *id += 1;
        current
    }

    /// Start a debug session for Rust (using lldb-vscode or codelldb)
    pub fn start_rust_debug(&self, session_id: &str, program_path: &Path) -> Result<()> {
        // Try lldb-vscode first, fall back to codelldb
        let adapter_cmd = if Command::new("lldb-vscode").arg("--version").output().is_ok() {
            vec!["lldb-vscode"]
        } else if Command::new("codelldb").arg("--version").output().is_ok() {
            vec!["codelldb", "--port", "0"]
        } else {
            return Err(anyhow!("No LLDB debug adapter found. Install lldb-vscode or codelldb."));
        };

        let mut process = Command::new(adapter_cmd[0])
            .args(&adapter_cmd[1..])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| anyhow!("Failed to start debug adapter: {}", e))?;

        // Send initialize request
        self.send_request(
            &mut process,
            "initialize",
            Some(serde_json::json!({
                "clientID": "berrycode",
                "clientName": "BerryCode",
                "adapterID": "lldb",
                "linesStartAt1": true,
                "columnsStartAt1": true,
                "pathFormat": "path",
                "supportsVariableType": true,
                "supportsVariablePaging": false,
                "supportsRunInTerminalRequest": false,
            })),
        )?;

        // Read initialize response
        self.read_response(&mut process)?;

        // Send launch request
        self.send_request(
            &mut process,
            "launch",
            Some(serde_json::json!({
                "program": program_path.to_string_lossy(),
                "cwd": self.project_root.to_string_lossy(),
                "stopOnEntry": false,
            })),
        )?;

        // Create session
        let session = DebugSession {
            process,
            thread_id: None,
            breakpoints: HashMap::new(),
            conditional_breakpoints: HashMap::new(),
            logpoints: HashMap::new(),
            watch_expressions: Vec::new(),
            stack_frames: Vec::new(),
            scopes: Vec::new(),
            is_stopped: false,
            stop_reason: None,
        };

        self.sessions
            .lock()
            .unwrap()
            .insert(session_id.to_string(), session);

        Ok(())
    }

    /// Set breakpoints in a file
    pub fn set_breakpoints(
        &self,
        session_id: &str,
        file_path: &Path,
        lines: Vec<i64>,
    ) -> Result<Vec<i64>> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| anyhow!("Debug session not found"))?;

        // Send setBreakpoints request
        self.send_request(
            &mut session.process,
            "setBreakpoints",
            Some(serde_json::json!({
                "source": {
                    "path": file_path.to_string_lossy(),
                },
                "breakpoints": lines.iter().map(|line| {
                    serde_json::json!({ "line": line })
                }).collect::<Vec<_>>(),
            })),
        )?;

        // Read response
        let response = self.read_response(&mut session.process)?;

        // Store breakpoints
        session.breakpoints.insert(file_path.to_path_buf(), lines.clone());

        // Extract verified breakpoints from response
        if let Some(body) = response.body {
            if let Some(breakpoints) = body.get("breakpoints") {
                if let Some(bps) = breakpoints.as_array() {
                    let verified_lines: Vec<i64> = bps
                        .iter()
                        .filter_map(|bp| {
                            if bp.get("verified")?.as_bool()? {
                                bp.get("line")?.as_i64()
                            } else {
                                None
                            }
                        })
                        .collect();
                    return Ok(verified_lines);
                }
            }
        }

        Ok(lines)
    }

    /// Continue execution
    pub fn continue_execution(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| anyhow!("Debug session not found"))?;

        let thread_id = session
            .thread_id
            .ok_or_else(|| anyhow!("No active thread"))?;

        self.send_request(
            &mut session.process,
            "continue",
            Some(serde_json::json!({
                "threadId": thread_id,
            })),
        )?;

        session.is_stopped = false;
        session.stop_reason = None;

        Ok(())
    }

    /// Step over (next line in current function)
    pub fn step_over(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| anyhow!("Debug session not found"))?;

        let thread_id = session
            .thread_id
            .ok_or_else(|| anyhow!("No active thread"))?;

        self.send_request(
            &mut session.process,
            "next",
            Some(serde_json::json!({
                "threadId": thread_id,
            })),
        )?;

        Ok(())
    }

    /// Step into (step into function call)
    pub fn step_into(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| anyhow!("Debug session not found"))?;

        let thread_id = session
            .thread_id
            .ok_or_else(|| anyhow!("No active thread"))?;

        self.send_request(
            &mut session.process,
            "stepIn",
            Some(serde_json::json!({
                "threadId": thread_id,
            })),
        )?;

        Ok(())
    }

    /// Step out (step out of current function)
    pub fn step_out(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| anyhow!("Debug session not found"))?;

        let thread_id = session
            .thread_id
            .ok_or_else(|| anyhow!("No active thread"))?;

        self.send_request(
            &mut session.process,
            "stepOut",
            Some(serde_json::json!({
                "threadId": thread_id,
            })),
        )?;

        Ok(())
    }

    /// Get stack trace for current thread
    pub fn get_stack_trace(&self, session_id: &str) -> Result<Vec<StackFrame>> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| anyhow!("Debug session not found"))?;

        let thread_id = session
            .thread_id
            .ok_or_else(|| anyhow!("No active thread"))?;

        self.send_request(
            &mut session.process,
            "stackTrace",
            Some(serde_json::json!({
                "threadId": thread_id,
            })),
        )?;

        let response = self.read_response(&mut session.process)?;

        if let Some(body) = response.body {
            if let Some(stack_frames) = body.get("stackFrames") {
                let frames: Vec<StackFrame> = serde_json::from_value(stack_frames.clone())?;
                session.stack_frames = frames.clone();
                return Ok(frames);
            }
        }

        Ok(Vec::new())
    }

    /// Get scopes for a stack frame
    pub fn get_scopes(&self, session_id: &str, frame_id: i64) -> Result<Vec<Scope>> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| anyhow!("Debug session not found"))?;

        self.send_request(
            &mut session.process,
            "scopes",
            Some(serde_json::json!({
                "frameId": frame_id,
            })),
        )?;

        let response = self.read_response(&mut session.process)?;

        if let Some(body) = response.body {
            if let Some(scopes) = body.get("scopes") {
                let scope_list: Vec<Scope> = serde_json::from_value(scopes.clone())?;
                session.scopes = scope_list.clone();
                return Ok(scope_list);
            }
        }

        Ok(Vec::new())
    }

    /// Get variables for a scope
    pub fn get_variables(
        &self,
        session_id: &str,
        variables_reference: i64,
    ) -> Result<Vec<Variable>> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| anyhow!("Debug session not found"))?;

        self.send_request(
            &mut session.process,
            "variables",
            Some(serde_json::json!({
                "variablesReference": variables_reference,
            })),
        )?;

        let response = self.read_response(&mut session.process)?;

        if let Some(body) = response.body {
            if let Some(variables) = body.get("variables") {
                let var_list: Vec<Variable> = serde_json::from_value(variables.clone())?;
                return Ok(var_list);
            }
        }

        Ok(Vec::new())
    }

    /// Set conditional breakpoint
    pub fn set_conditional_breakpoint(
        &self,
        session_id: &str,
        file_path: &Path,
        line: i64,
        condition: String,
    ) -> Result<()> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| anyhow!("Debug session not found"))?;

        // Add to conditional breakpoints
        session
            .conditional_breakpoints
            .entry(file_path.to_path_buf())
            .or_insert_with(Vec::new)
            .push((line, condition.clone()));

        // Build breakpoints array with all types
        let mut all_breakpoints = Vec::new();

        // Regular breakpoints
        if let Some(regular_bps) = session.breakpoints.get(file_path) {
            for &line in regular_bps {
                all_breakpoints.push(serde_json::json!({ "line": line }));
            }
        }

        // Conditional breakpoints
        if let Some(cond_bps) = session.conditional_breakpoints.get(file_path) {
            for (bp_line, cond) in cond_bps {
                all_breakpoints.push(serde_json::json!({
                    "line": bp_line,
                    "condition": cond,
                }));
            }
        }

        // Logpoints
        if let Some(log_bps) = session.logpoints.get(file_path) {
            for (bp_line, msg) in log_bps {
                all_breakpoints.push(serde_json::json!({
                    "line": bp_line,
                    "logMessage": msg,
                }));
            }
        }

        // Send setBreakpoints request with all breakpoints
        self.send_request(
            &mut session.process,
            "setBreakpoints",
            Some(serde_json::json!({
                "source": {
                    "path": file_path.to_string_lossy(),
                },
                "breakpoints": all_breakpoints,
            })),
        )?;

        Ok(())
    }

    /// Set logpoint (non-breaking breakpoint that logs)
    pub fn set_logpoint(
        &self,
        session_id: &str,
        file_path: &Path,
        line: i64,
        message: String,
    ) -> Result<()> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| anyhow!("Debug session not found"))?;

        // Add to logpoints
        session
            .logpoints
            .entry(file_path.to_path_buf())
            .or_insert_with(Vec::new)
            .push((line, message.clone()));

        // Build breakpoints array with all types
        let mut all_breakpoints = Vec::new();

        // Regular breakpoints
        if let Some(regular_bps) = session.breakpoints.get(file_path) {
            for &line in regular_bps {
                all_breakpoints.push(serde_json::json!({ "line": line }));
            }
        }

        // Conditional breakpoints
        if let Some(cond_bps) = session.conditional_breakpoints.get(file_path) {
            for (bp_line, cond) in cond_bps {
                all_breakpoints.push(serde_json::json!({
                    "line": bp_line,
                    "condition": cond,
                }));
            }
        }

        // Logpoints
        if let Some(log_bps) = session.logpoints.get(file_path) {
            for (bp_line, msg) in log_bps {
                all_breakpoints.push(serde_json::json!({
                    "line": bp_line,
                    "logMessage": msg,
                }));
            }
        }

        // Send setBreakpoints request with all breakpoints
        self.send_request(
            &mut session.process,
            "setBreakpoints",
            Some(serde_json::json!({
                "source": {
                    "path": file_path.to_string_lossy(),
                },
                "breakpoints": all_breakpoints,
            })),
        )?;

        Ok(())
    }

    /// Remove conditional breakpoint
    pub fn remove_conditional_breakpoint(
        &self,
        session_id: &str,
        file_path: &Path,
        line: i64,
    ) -> Result<()> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| anyhow!("Debug session not found"))?;

        // Remove from conditional breakpoints
        if let Some(cond_bps) = session.conditional_breakpoints.get_mut(file_path) {
            cond_bps.retain(|(bp_line, _)| *bp_line != line);
        }

        Ok(())
    }

    /// Remove logpoint
    pub fn remove_logpoint(
        &self,
        session_id: &str,
        file_path: &Path,
        line: i64,
    ) -> Result<()> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| anyhow!("Debug session not found"))?;

        // Remove from logpoints
        if let Some(log_bps) = session.logpoints.get_mut(file_path) {
            log_bps.retain(|(bp_line, _)| *bp_line != line);
        }

        Ok(())
    }

    /// Add watch expression
    pub fn add_watch_expression(
        &self,
        session_id: &str,
        expression: String,
    ) -> Result<WatchExpression> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| anyhow!("Debug session not found"))?;

        let watch = WatchExpression {
            id: uuid::Uuid::new_v4().to_string(),
            expression,
            value: None,
            var_type: None,
            error: None,
        };

        session.watch_expressions.push(watch.clone());
        Ok(watch)
    }

    /// Remove watch expression
    pub fn remove_watch_expression(&self, session_id: &str, watch_id: &str) -> Result<()> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| anyhow!("Debug session not found"))?;

        session.watch_expressions.retain(|w| w.id != watch_id);
        Ok(())
    }

    /// Get all watch expressions
    pub fn get_watch_expressions(&self, session_id: &str) -> Result<Vec<WatchExpression>> {
        let sessions = self.sessions.lock().unwrap();
        let session = sessions
            .get(session_id)
            .ok_or_else(|| anyhow!("Debug session not found"))?;

        Ok(session.watch_expressions.clone())
    }

    /// Evaluate expression in debug context
    pub fn evaluate_expression(
        &self,
        session_id: &str,
        expression: &str,
        frame_id: Option<i64>,
    ) -> Result<EvaluationResult> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| anyhow!("Debug session not found"))?;

        // Use the provided frame_id or the first stack frame
        let eval_frame_id = frame_id.or_else(|| {
            session.stack_frames.first().map(|f| f.id)
        }).unwrap_or(0);

        self.send_request(
            &mut session.process,
            "evaluate",
            Some(serde_json::json!({
                "expression": expression,
                "frameId": eval_frame_id,
                "context": "watch",
            })),
        )?;

        let response = self.read_response(&mut session.process)?;

        if let Some(body) = response.body {
            let result = EvaluationResult {
                result: body.get("result")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                var_type: body.get("type")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                variables_reference: body.get("variablesReference")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0),
            };
            return Ok(result);
        }

        Err(anyhow!("Evaluation failed"))
    }

    /// Evaluate all watch expressions
    pub fn evaluate_all_watches(&self, session_id: &str) -> Result<Vec<WatchExpression>> {
        let watches = self.get_watch_expressions(session_id)?;
        let mut results = Vec::new();

        for watch in watches {
            let mut updated_watch = watch.clone();

            match self.evaluate_expression(session_id, &watch.expression, None) {
                Ok(eval_result) => {
                    updated_watch.value = Some(eval_result.result);
                    updated_watch.var_type = eval_result.var_type;
                    updated_watch.error = None;
                }
                Err(e) => {
                    updated_watch.error = Some(e.to_string());
                }
            }

            results.push(updated_watch);
        }

        // Update the session's watch expressions
        let mut sessions = self.sessions.lock().unwrap();
        if let Some(session) = sessions.get_mut(session_id) {
            session.watch_expressions = results.clone();
        }

        Ok(results)
    }

    /// Stop debugging session
    pub fn stop_debug(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.sessions.lock().unwrap();
        if let Some(mut session) = sessions.remove(session_id) {
            self.send_request(&mut session.process, "disconnect", None)?;
            let _ = session.process.kill();
        }
        Ok(())
    }

    /// Send a DAP request
    fn send_request(
        &self,
        process: &mut Child,
        command: &str,
        arguments: Option<serde_json::Value>,
    ) -> Result<()> {
        let request = DapRequest {
            seq: self.next_request_id(),
            msg_type: "request".to_string(),
            command: command.to_string(),
            arguments,
        };

        let content = serde_json::to_string(&request)?;
        let message = format!("Content-Length: {}\r\n\r\n{}", content.len(), content);

        if let Some(stdin) = process.stdin.as_mut() {
            stdin.write_all(message.as_bytes())?;
            stdin.flush()?;
        }

        Ok(())
    }

    /// Read a DAP response
    fn read_response(&self, process: &mut Child) -> Result<DapResponse> {
        if let Some(stdout) = process.stdout.as_mut() {
            let mut reader = BufReader::new(stdout);
            let mut header = String::new();

            // Read headers
            loop {
                header.clear();
                reader.read_line(&mut header)?;
                if header.trim().is_empty() {
                    break;
                }
            }

            // Extract content length
            let content_length: usize = header
                .lines()
                .find(|line| line.starts_with("Content-Length:"))
                .and_then(|line| line.split(':').nth(1))
                .and_then(|s| s.trim().parse().ok())
                .ok_or_else(|| anyhow!("Invalid Content-Length header"))?;

            // Read content
            let mut content = vec![0u8; content_length];
            std::io::Read::read_exact(&mut reader, &mut content)?;

            let response: DapResponse = serde_json::from_slice(&content)?;
            Ok(response)
        } else {
            Err(anyhow!("No stdout available"))
        }
    }
}
