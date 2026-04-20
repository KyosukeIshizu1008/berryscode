//! Debug Adapter Protocol client implementation

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DapBreakpoint {
    pub line: usize,
    pub verified: bool,
    pub file_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DapStackFrame {
    pub id: u64,
    pub name: String,
    pub file_path: Option<String>,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DapVariable {
    pub name: String,
    pub value: String,
    pub var_type: Option<String>,
    pub variables_reference: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DapThread {
    pub id: u64,
    pub name: String,
}

#[derive(Debug, Clone)]
pub enum DapEvent {
    Stopped { thread_id: u64, reason: String },
    Continued { thread_id: u64 },
    Terminated,
    Output { category: String, output: String },
    Breakpoint { breakpoint: DapBreakpoint },
    Initialized,
}

/// DAP client state
pub struct DapClient {
    process: Option<Child>,
    stdin: Option<ChildStdin>,
    seq: u64,
    pending_responses: Arc<RwLock<HashMap<u64, mpsc::Sender<Value>>>>,
    event_tx: mpsc::UnboundedSender<DapEvent>,
}

impl DapClient {
    pub fn new(event_tx: mpsc::UnboundedSender<DapEvent>) -> Self {
        Self {
            process: None,
            stdin: None,
            seq: 0,
            pending_responses: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
        }
    }

    /// Launch a debug adapter process
    pub async fn launch_adapter(&mut self, adapter_command: &str, args: &[&str]) -> Result<()> {
        let mut cmd = Command::new(adapter_command);
        cmd.args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());

        let mut process = cmd.spawn().context(format!(
            "Failed to start debug adapter: {}",
            adapter_command
        ))?;

        let stdin = process.stdin.take().context("Failed to get DAP stdin")?;
        let stdout = process.stdout.take().context("Failed to get DAP stdout")?;

        self.stdin = Some(stdin);
        self.process = Some(process);

        // Start reader thread
        let pending = self.pending_responses.clone();
        let event_tx = self.event_tx.clone();

        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut line_buffer = String::new();

            loop {
                line_buffer.clear();
                match reader.read_line(&mut line_buffer) {
                    Ok(0) => break,
                    Ok(_) => {}
                    Err(_) => break,
                }

                let content_length: Option<usize> = if line_buffer.starts_with("Content-Length:") {
                    line_buffer
                        .trim_start_matches("Content-Length:")
                        .trim()
                        .parse()
                        .ok()
                } else {
                    None
                };

                if content_length.is_none() {
                    continue;
                }

                // Skip to empty line
                loop {
                    line_buffer.clear();
                    if reader.read_line(&mut line_buffer).is_err() || line_buffer.trim().is_empty()
                    {
                        break;
                    }
                }

                if let Some(len) = content_length {
                    let mut content = vec![0u8; len];
                    if reader.read_exact(&mut content).is_ok() {
                        if let Ok(text) = String::from_utf8(content) {
                            if let Ok(value) = serde_json::from_str::<Value>(&text) {
                                let msg_type =
                                    value.get("type").and_then(|t| t.as_str()).unwrap_or("");

                                match msg_type {
                                    "response" => {
                                        if let Some(seq) =
                                            value.get("request_seq").and_then(|s| s.as_u64())
                                        {
                                            let mut p = pending.write().await;
                                            if let Some(tx) = p.remove(&seq) {
                                                let _ = tx.send(value).await;
                                            }
                                        }
                                    }
                                    "event" => {
                                        let event_name = value
                                            .get("event")
                                            .and_then(|e| e.as_str())
                                            .unwrap_or("");
                                        let body =
                                            value.get("body").cloned().unwrap_or(Value::Null);

                                        let event = match event_name {
                                            "stopped" => Some(DapEvent::Stopped {
                                                thread_id: body
                                                    .get("threadId")
                                                    .and_then(|t| t.as_u64())
                                                    .unwrap_or(0),
                                                reason: body
                                                    .get("reason")
                                                    .and_then(|r| r.as_str())
                                                    .unwrap_or("unknown")
                                                    .to_string(),
                                            }),
                                            "continued" => Some(DapEvent::Continued {
                                                thread_id: body
                                                    .get("threadId")
                                                    .and_then(|t| t.as_u64())
                                                    .unwrap_or(0),
                                            }),
                                            "terminated" => Some(DapEvent::Terminated),
                                            "output" => Some(DapEvent::Output {
                                                category: body
                                                    .get("category")
                                                    .and_then(|c| c.as_str())
                                                    .unwrap_or("console")
                                                    .to_string(),
                                                output: body
                                                    .get("output")
                                                    .and_then(|o| o.as_str())
                                                    .unwrap_or("")
                                                    .to_string(),
                                            }),
                                            "initialized" => Some(DapEvent::Initialized),
                                            _ => None,
                                        };

                                        if let Some(evt) = event {
                                            let _ = event_tx.send(evt);
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        });

        // Send initialize request
        let init_args = serde_json::json!({
            "clientID": "berrycode",
            "clientName": "BerryCode",
            "adapterID": "codelldb",
            "pathFormat": "path",
            "linesStartAt1": true,
            "columnsStartAt1": true,
            "supportsVariableType": true,
            "supportsRunInTerminalRequest": false,
        });

        let _response = self.send_request("initialize", init_args).await?;
        tracing::info!("DAP adapter initialized");

        Ok(())
    }

    async fn send_request(&mut self, command: &str, arguments: Value) -> Result<Value> {
        self.seq += 1;
        let seq = self.seq;

        let request = serde_json::json!({
            "seq": seq,
            "type": "request",
            "command": command,
            "arguments": arguments,
        });

        let content = serde_json::to_string(&request)?;
        let message = format!("Content-Length: {}\r\n\r\n{}", content.len(), content);

        if let Some(stdin) = &mut self.stdin {
            stdin.write_all(message.as_bytes())?;
            stdin.flush()?;
        } else {
            anyhow::bail!("DAP adapter not running");
        }

        let (tx, mut rx) = mpsc::channel(1);
        self.pending_responses.write().await.insert(seq, tx);

        let response = tokio::time::timeout(std::time::Duration::from_secs(30), rx.recv())
            .await
            .context("DAP request timed out")?
            .context("Failed to receive DAP response")?;

        self.pending_responses.write().await.remove(&seq);
        Ok(response)
    }

    /// Set breakpoints for a file
    pub async fn set_breakpoints(
        &mut self,
        file_path: &str,
        lines: &[usize],
    ) -> Result<Vec<DapBreakpoint>> {
        let breakpoints: Vec<Value> = lines
            .iter()
            .map(|l| serde_json::json!({"line": l + 1}))
            .collect();

        let args = serde_json::json!({
            "source": { "path": file_path },
            "breakpoints": breakpoints,
        });

        let response = self.send_request("setBreakpoints", args).await?;

        let mut result = Vec::new();
        if let Some(body) = response.get("body") {
            if let Some(bps) = body.get("breakpoints").and_then(|b| b.as_array()) {
                for bp in bps {
                    result.push(DapBreakpoint {
                        line: bp.get("line").and_then(|l| l.as_u64()).unwrap_or(0) as usize - 1,
                        verified: bp
                            .get("verified")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false),
                        file_path: file_path.to_string(),
                    });
                }
            }
        }
        Ok(result)
    }

    /// Launch a program for debugging
    pub async fn launch(&mut self, program: &str, args: &[&str], cwd: &str) -> Result<()> {
        let launch_args = serde_json::json!({
            "program": program,
            "args": args,
            "cwd": cwd,
            "stopOnEntry": false,
            "type": "lldb",
        });

        let _response = self.send_request("launch", launch_args).await?;
        tracing::info!("DAP: Program launched: {}", program);
        Ok(())
    }

    pub async fn continue_execution(&mut self, thread_id: u64) -> Result<()> {
        let args = serde_json::json!({"threadId": thread_id});
        let _ = self.send_request("continue", args).await?;
        Ok(())
    }

    pub async fn step_over(&mut self, thread_id: u64) -> Result<()> {
        let args = serde_json::json!({"threadId": thread_id});
        let _ = self.send_request("next", args).await?;
        Ok(())
    }

    pub async fn step_into(&mut self, thread_id: u64) -> Result<()> {
        let args = serde_json::json!({"threadId": thread_id});
        let _ = self.send_request("stepIn", args).await?;
        Ok(())
    }

    pub async fn step_out(&mut self, thread_id: u64) -> Result<()> {
        let args = serde_json::json!({"threadId": thread_id});
        let _ = self.send_request("stepOut", args).await?;
        Ok(())
    }

    pub async fn get_threads(&mut self) -> Result<Vec<DapThread>> {
        let response = self.send_request("threads", serde_json::json!({})).await?;
        let mut threads = Vec::new();
        if let Some(body) = response.get("body") {
            if let Some(ts) = body.get("threads").and_then(|t| t.as_array()) {
                for t in ts {
                    threads.push(DapThread {
                        id: t.get("id").and_then(|i| i.as_u64()).unwrap_or(0),
                        name: t
                            .get("name")
                            .and_then(|n| n.as_str())
                            .unwrap_or("")
                            .to_string(),
                    });
                }
            }
        }
        Ok(threads)
    }

    pub async fn get_stack_frames(&mut self, thread_id: u64) -> Result<Vec<DapStackFrame>> {
        let args = serde_json::json!({"threadId": thread_id});
        let response = self.send_request("stackTrace", args).await?;
        let mut frames = Vec::new();
        if let Some(body) = response.get("body") {
            if let Some(sfs) = body.get("stackFrames").and_then(|s| s.as_array()) {
                for sf in sfs {
                    let source_path = sf
                        .get("source")
                        .and_then(|s| s.get("path"))
                        .and_then(|p| p.as_str())
                        .map(String::from);
                    frames.push(DapStackFrame {
                        id: sf.get("id").and_then(|i| i.as_u64()).unwrap_or(0),
                        name: sf
                            .get("name")
                            .and_then(|n| n.as_str())
                            .unwrap_or("")
                            .to_string(),
                        file_path: source_path,
                        line: sf.get("line").and_then(|l| l.as_u64()).unwrap_or(0) as usize - 1,
                        column: sf.get("column").and_then(|c| c.as_u64()).unwrap_or(0) as usize,
                    });
                }
            }
        }
        Ok(frames)
    }

    pub async fn get_variables(&mut self, variables_reference: u64) -> Result<Vec<DapVariable>> {
        let args = serde_json::json!({"variablesReference": variables_reference});
        let response = self.send_request("variables", args).await?;
        let mut vars = Vec::new();
        if let Some(body) = response.get("body") {
            if let Some(vs) = body.get("variables").and_then(|v| v.as_array()) {
                for v in vs {
                    vars.push(DapVariable {
                        name: v
                            .get("name")
                            .and_then(|n| n.as_str())
                            .unwrap_or("")
                            .to_string(),
                        value: v
                            .get("value")
                            .and_then(|val| val.as_str())
                            .unwrap_or("")
                            .to_string(),
                        var_type: v.get("type").and_then(|t| t.as_str()).map(String::from),
                        variables_reference: v
                            .get("variablesReference")
                            .and_then(|r| r.as_u64())
                            .unwrap_or(0),
                    });
                }
            }
        }
        Ok(vars)
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        let _ = self
            .send_request("disconnect", serde_json::json!({"terminateDebuggee": true}))
            .await;
        if let Some(mut proc) = self.process.take() {
            let _ = proc.kill();
        }
        Ok(())
    }
}
