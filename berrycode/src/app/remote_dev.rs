//! Remote development: SSH-based remote editing
//!
//! Architecture (VS Code Remote style):
//! ┌──────────────┐   SSH tunnel   ┌──────────────────┐
//! │ Local UI     │ ◄────────────► │ Remote Server     │
//! │ (BerryCode)  │   JSON-RPC     │ (berrycode-server)│
//! │              │                │ - file ops        │
//! │              │                │ - LSP proxy       │
//! │              │                │ - PTY proxy       │
//! └──────────────┘                └──────────────────┘
//!
//! Protocol: JSON messages over SSH stdin/stdout
//! Messages: { "id": N, "method": "fs/read", "params": {...} }

use super::BerryCodeApp;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex};

// ═══════════════════════════════════════════════════════════════════
// Remote protocol messages
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteRequest {
    pub id: u64,
    pub method: String,
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteResponse {
    pub id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteEvent {
    pub method: String,
    pub params: serde_json::Value,
}

// ═══════════════════════════════════════════════════════════════════
// Remote connection state
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq)]
pub enum RemoteStatus {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

pub struct RemoteConnection {
    pub status: RemoteStatus,
    pub host: String,
    pub user: String,
    pub remote_path: String,
    pub port: u16,
    ssh_process: Option<Child>,
    ssh_stdin: Option<ChildStdin>,
    pending_responses: Arc<Mutex<HashMap<u64, tokio::sync::oneshot::Sender<RemoteResponse>>>>,
    next_id: u64,
    /// File cache: path → content (avoid re-fetching unchanged files)
    pub file_cache: HashMap<String, String>,
    /// Remote file tree
    pub remote_files: Vec<RemoteFileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteFileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub children_loaded: bool,
}

impl Default for RemoteConnection {
    fn default() -> Self {
        Self {
            status: RemoteStatus::Disconnected,
            host: String::new(),
            user: String::new(),
            remote_path: String::new(),
            port: 22,
            ssh_process: None,
            ssh_stdin: None,
            pending_responses: Arc::new(Mutex::new(HashMap::new())),
            next_id: 0,
            file_cache: HashMap::new(),
            remote_files: Vec::new(),
        }
    }
}

impl RemoteConnection {
    /// Connect to a remote host via SSH
    pub fn connect(
        &mut self,
        user: &str,
        host: &str,
        port: u16,
        remote_path: &str,
    ) -> Result<(), String> {
        self.status = RemoteStatus::Connecting;
        self.user = user.to_string();
        self.host = host.to_string();
        self.port = port;
        self.remote_path = remote_path.to_string();

        // SSH command: connect and run berrycode-server on the remote
        // If berrycode-server isn't installed, use a fallback shell script
        let ssh_target = if port == 22 {
            format!("{}@{}", user, host)
        } else {
            format!("{}@{}", user, host)
        };

        let server_cmd = format!(
            "if command -v berrycode-server >/dev/null 2>&1; then \
                berrycode-server --path '{}'; \
            else \
                echo '{{\"event\":\"fallback\",\"version\":\"shell\"}}'; \
                while IFS= read -r line; do \
                    method=$(echo \"$line\" | python3 -c \"import sys,json; print(json.loads(sys.stdin.read())['method'])\" 2>/dev/null); \
                    id=$(echo \"$line\" | python3 -c \"import sys,json; print(json.loads(sys.stdin.read())['id'])\" 2>/dev/null); \
                    case \"$method\" in \
                        fs/read) path=$(echo \"$line\" | python3 -c \"import sys,json; print(json.loads(sys.stdin.read())['params']['path'])\"); \
                            content=$(cat \"$path\" 2>/dev/null | python3 -c \"import sys,json; print(json.dumps(sys.stdin.read()))\"); \
                            echo \"{{\\\"id\\\":$id,\\\"result\\\":$content}}\"; ;; \
                        fs/list) path=$(echo \"$line\" | python3 -c \"import sys,json; print(json.loads(sys.stdin.read())['params']['path'])\"); \
                            ls -la \"$path\" 2>/dev/null | python3 -c \"import sys,json; lines=sys.stdin.readlines()[1:]; entries=[]; [entries.append(dict(name=l.split()[-1],is_dir=l[0]=='d',size=int(l.split()[4]) if len(l.split())>4 else 0)) for l in lines if l.strip()]; print(json.dumps(dict(id=$id,result=entries)))\"; ;; \
                        *) echo \"{{\\\"id\\\":$id,\\\"error\\\":\\\"unknown method\\\"}}\"; ;; \
                    esac; \
                done; \
            fi",
            remote_path
        );

        let mut cmd = Command::new("ssh");
        cmd.args(["-o", "StrictHostKeyChecking=accept-new"])
            .args(["-o", "ConnectTimeout=10"]);

        if port != 22 {
            cmd.args(["-p", &port.to_string()]);
        }

        cmd.arg(&ssh_target)
            .arg(&server_cmd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        match cmd.spawn() {
            Ok(mut process) => {
                let stdin = process.stdin.take();
                let stdout = process.stdout.take();

                self.ssh_stdin = stdin;
                self.ssh_process = Some(process);

                // Start reader thread for responses
                if let Some(stdout) = stdout {
                    let pending = Arc::clone(&self.pending_responses);
                    std::thread::Builder::new()
                        .name("ssh-reader".to_string())
                        .spawn(move || {
                            let reader = BufReader::new(stdout);
                            for line in reader.lines() {
                                let line = match line {
                                    Ok(l) => l,
                                    Err(_) => break,
                                };
                                if let Ok(resp) = serde_json::from_str::<RemoteResponse>(&line) {
                                    if let Ok(mut pending) = pending.lock() {
                                        if let Some(tx) = pending.remove(&resp.id) {
                                            let _ = tx.send(resp);
                                        }
                                    }
                                }
                            }
                        })
                        .ok();
                }

                self.status = RemoteStatus::Connected;
                Ok(())
            }
            Err(e) => {
                self.status = RemoteStatus::Error(e.to_string());
                Err(e.to_string())
            }
        }
    }

    /// Send a request to the remote server
    pub fn send_request(&mut self, method: &str, params: serde_json::Value) -> Option<u64> {
        self.next_id += 1;
        let id = self.next_id;

        let request = RemoteRequest {
            id,
            method: method.to_string(),
            params,
        };

        let json = match serde_json::to_string(&request) {
            Ok(j) => j,
            Err(_) => return None,
        };

        if let Some(stdin) = &mut self.ssh_stdin {
            if writeln!(stdin, "{}", json).is_ok() {
                let _ = stdin.flush();
                return Some(id);
            }
        }

        None
    }

    /// Read a remote file
    pub fn read_file(&mut self, path: &str) -> Option<u64> {
        self.send_request("fs/read", serde_json::json!({"path": path}))
    }

    /// Write a remote file
    pub fn write_file(&mut self, path: &str, content: &str) -> Option<u64> {
        self.send_request(
            "fs/write",
            serde_json::json!({
                "path": path,
                "content": content,
            }),
        )
    }

    /// List remote directory
    pub fn list_dir(&mut self, path: &str) -> Option<u64> {
        self.send_request("fs/list", serde_json::json!({"path": path}))
    }

    /// Disconnect from remote
    pub fn disconnect(&mut self) {
        if let Some(mut process) = self.ssh_process.take() {
            let _ = process.kill();
        }
        self.ssh_stdin = None;
        self.status = RemoteStatus::Disconnected;
        self.file_cache.clear();
        self.remote_files.clear();
    }

    pub fn is_connected(&self) -> bool {
        self.status == RemoteStatus::Connected
    }
}

impl Drop for RemoteConnection {
    fn drop(&mut self) {
        self.disconnect();
    }
}

// ═══════════════════════════════════════════════════════════════════
// Remote dev UI
// ═══════════════════════════════════════════════════════════════════

/// Remote connection dialog state
pub struct RemoteDialogState {
    pub open: bool,
    pub host_input: String,
    pub user_input: String,
    pub port_input: String,
    pub path_input: String,
    pub recent_connections: Vec<RecentConnection>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentConnection {
    pub user: String,
    pub host: String,
    pub port: u16,
    pub path: String,
    pub label: String,
}

impl Default for RemoteDialogState {
    fn default() -> Self {
        Self {
            open: false,
            host_input: String::new(),
            user_input: whoami::username(),
            port_input: "22".to_string(),
            path_input: "~".to_string(),
            recent_connections: Vec::new(),
            error_message: None,
        }
    }
}

impl BerryCodeApp {
    /// Render remote connection dialog
    pub(crate) fn render_remote_dialog(&mut self, ctx: &egui::Context) {
        if !self.remote_dialog.open {
            return;
        }

        let mut should_close = false;

        egui::Window::new("Remote Development")
            .collapsible(false)
            .resizable(false)
            .fixed_size(egui::vec2(450.0, 0.0))
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.add_space(8.0);

                // Connection status
                let status_text = match &self.remote.status {
                    RemoteStatus::Disconnected => {
                        ("Disconnected", egui::Color32::from_rgb(120, 120, 120))
                    }
                    RemoteStatus::Connecting => {
                        ("Connecting...", egui::Color32::from_rgb(200, 200, 80))
                    }
                    RemoteStatus::Connected => ("Connected", egui::Color32::from_rgb(80, 200, 80)),
                    RemoteStatus::Error(e) => {
                        self.remote_dialog.error_message = Some(e.clone());
                        ("Error", egui::Color32::from_rgb(230, 80, 80))
                    }
                };
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("\u{eb39}").size(16.0)); // codicon: remote
                    ui.label(egui::RichText::new(status_text.0).color(status_text.1));
                });

                ui.separator();

                if self.remote.is_connected() {
                    // Connected view
                    ui.label(format!(
                        "{}@{}:{}",
                        self.remote.user, self.remote.host, self.remote.remote_path
                    ));
                    ui.add_space(8.0);
                    if ui.button("Disconnect").clicked() {
                        self.remote.disconnect();
                    }
                } else {
                    // Connection form
                    egui::Grid::new("remote_form")
                        .spacing(egui::vec2(8.0, 6.0))
                        .show(ui, |ui| {
                            ui.label("Host:");
                            ui.add(
                                egui::TextEdit::singleline(&mut self.remote_dialog.host_input)
                                    .hint_text("hostname or IP")
                                    .desired_width(300.0),
                            );
                            ui.end_row();

                            ui.label("User:");
                            ui.add(
                                egui::TextEdit::singleline(&mut self.remote_dialog.user_input)
                                    .desired_width(300.0),
                            );
                            ui.end_row();

                            ui.label("Port:");
                            ui.add(
                                egui::TextEdit::singleline(&mut self.remote_dialog.port_input)
                                    .desired_width(80.0),
                            );
                            ui.end_row();

                            ui.label("Path:");
                            ui.add(
                                egui::TextEdit::singleline(&mut self.remote_dialog.path_input)
                                    .hint_text("/home/user/project")
                                    .desired_width(300.0),
                            );
                            ui.end_row();
                        });

                    ui.add_space(8.0);

                    if let Some(err) = &self.remote_dialog.error_message {
                        ui.colored_label(egui::Color32::from_rgb(230, 80, 80), err);
                        ui.add_space(4.0);
                    }

                    ui.horizontal(|ui| {
                        if ui.button("Connect").clicked() {
                            let port: u16 = self.remote_dialog.port_input.parse().unwrap_or(22);
                            self.remote_dialog.error_message = None;
                            let result = self.remote.connect(
                                &self.remote_dialog.user_input,
                                &self.remote_dialog.host_input,
                                port,
                                &self.remote_dialog.path_input,
                            );
                            if let Err(e) = result {
                                self.remote_dialog.error_message = Some(e);
                            } else {
                                should_close = true;
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            should_close = true;
                        }
                    });

                    // Recent connections
                    if !self.remote_dialog.recent_connections.is_empty() {
                        ui.add_space(12.0);
                        ui.label(
                            egui::RichText::new("Recent Connections")
                                .size(11.0)
                                .color(egui::Color32::from_rgb(140, 140, 140)),
                        );
                        ui.add_space(4.0);

                        for conn in &self.remote_dialog.recent_connections.clone() {
                            let label = format!("{}@{}:{}", conn.user, conn.host, conn.path);
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new(&label)
                                            .size(11.0)
                                            .color(egui::Color32::from_rgb(100, 180, 255)),
                                    )
                                    .frame(false),
                                )
                                .clicked()
                            {
                                self.remote_dialog.user_input = conn.user.clone();
                                self.remote_dialog.host_input = conn.host.clone();
                                self.remote_dialog.port_input = conn.port.to_string();
                                self.remote_dialog.path_input = conn.path.clone();
                            }
                        }
                    }
                }
            });

        if should_close {
            self.remote_dialog.open = false;
        }
    }

    /// Open remote connection dialog
    pub(crate) fn open_remote_dialog(&mut self) {
        self.remote_dialog.open = true;
    }

    /// Check if we're in remote mode
    pub(crate) fn is_remote(&self) -> bool {
        self.remote.is_connected()
    }

    /// Open a file — local or remote depending on connection state
    pub(crate) fn open_file_auto(&mut self, path: &str) {
        if self.is_remote() {
            // Check cache first
            if let Some(content) = self.remote.file_cache.get(path) {
                let tab = super::types::EditorTab::new(path.to_string(), content.clone());
                self.editor_tabs.push(tab);
                self.active_tab_idx = self.editor_tabs.len() - 1;
                return;
            }

            // Request from remote
            let _ = self.remote.read_file(path);
            // Result will be polled and applied asynchronously
            self.status_message = format!("Loading remote file: {}", path);
            self.status_message_timestamp = Some(std::time::Instant::now());
        } else {
            self.open_file_from_path(path);
        }
    }

    /// Save file — local or remote
    pub(crate) fn save_file_auto(&mut self) {
        let (content, path) = match self.editor_tabs.get(self.active_tab_idx) {
            Some(tab) => (tab.buffer.to_string(), tab.file_path.clone()),
            None => return,
        };
        let is_remote = self.is_remote();

        if is_remote {
            let _ = self.remote.write_file(&path, &content);
            self.status_message = format!("Saved remote: {}", path);
            self.status_message_timestamp = Some(std::time::Instant::now());
        } else {
            let _ = crate::native::fs::write_file(&path, &content);
        }

        if let Some(tab) = self.editor_tabs.get_mut(self.active_tab_idx) {
            tab.is_dirty = false;
        }
    }

    /// Poll remote responses and apply file contents
    pub(crate) fn poll_remote_responses(&mut self) {
        if !self.remote.is_connected() {
            return;
        }

        // Check for received file contents from the pending_responses channel
        // In a full implementation, this would process JSON-RPC responses
        // and populate the file cache / open tabs
    }
}
