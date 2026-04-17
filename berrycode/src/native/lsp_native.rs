//! Native LSP client implementation
//! Directly communicates with LSP servers (e.g., rust-analyzer) via stdio

use anyhow::{Context, Result};
use lsp_types::*;
use serde_json::Value;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

/// Diagnostic info pushed by the LSP server via textDocument/publishDiagnostics
#[derive(Debug, Clone)]
pub struct PublishedDiagnostics {
    pub uri: String,
    pub diagnostics: Vec<Diagnostic>,
}

/// Native LSP client that communicates directly with LSP servers
pub struct NativeLspClient {
    servers: Arc<RwLock<HashMap<String, Arc<RwLock<LspServer>>>>>,
    pending_responses: Arc<RwLock<HashMap<u64, mpsc::Sender<Value>>>>,
    /// Tracks which files are open and their current version (keyed by URI string)
    opened_files: Arc<RwLock<HashMap<String, i32>>>,
    /// Channel for pushing diagnostics from the server reader thread
    diagnostics_tx: Option<mpsc::UnboundedSender<PublishedDiagnostics>>,
}

struct LspServer {
    process: Child,
    stdin: ChildStdin,
    message_id: u64,
}

/// Helper: convert a file path to a file:// URI using lsp_types::Url
fn file_path_to_uri(file_path: &str) -> Result<Url> {
    Url::from_file_path(file_path)
        .map_err(|_| anyhow::anyhow!("Failed to convert path to URI: {}", file_path))
}

/// Detect the LSP server language key from a file path.
/// Returns None if no LSP server handles this file type.
pub fn detect_server_language(file_path: &str) -> Option<&'static str> {
    if file_path.ends_with(".rs") || file_path.ends_with(".toml") {
        Some("rust")
    } else if file_path.ends_with(".ts")
        || file_path.ends_with(".tsx")
        || file_path.ends_with(".js")
        || file_path.ends_with(".jsx")
    {
        Some("typescript")
    } else if file_path.ends_with(".py") {
        Some("python")
    } else {
        None // No LSP server for this file type
    }
}

/// Helper: detect LSP language ID from file extension
fn detect_language_id(file_path: &str) -> &'static str {
    if file_path.ends_with(".rs") {
        "rust"
    } else if file_path.ends_with(".ts") {
        "typescript"
    } else if file_path.ends_with(".tsx") {
        "typescriptreact"
    } else if file_path.ends_with(".js") {
        "javascript"
    } else if file_path.ends_with(".jsx") {
        "javascriptreact"
    } else if file_path.ends_with(".py") {
        "python"
    } else if file_path.ends_with(".toml") {
        "toml"
    } else if file_path.ends_with(".json") {
        "json"
    } else if file_path.ends_with(".md") {
        "markdown"
    } else if file_path.ends_with(".yaml") || file_path.ends_with(".yml") {
        "yaml"
    } else {
        "plaintext"
    }
}

impl NativeLspClient {
    /// Create a new NativeLspClient.
    ///
    /// Returns `(client, diagnostics_rx)` where `diagnostics_rx` receives
    /// `publishDiagnostics` notifications from all LSP servers.
    pub fn new() -> (Self, mpsc::UnboundedReceiver<PublishedDiagnostics>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let client = Self {
            servers: Arc::new(RwLock::new(HashMap::new())),
            pending_responses: Arc::new(RwLock::new(HashMap::new())),
            opened_files: Arc::new(RwLock::new(HashMap::new())),
            diagnostics_tx: Some(tx),
        };
        (client, rx)
    }

    /// Start an LSP server for a language
    pub async fn start_server(&self, language: &str, root_path: &str) -> Result<()> {
        let command = match language {
            "rust" => "rust-analyzer",
            "typescript" | "javascript" => "typescript-language-server",
            "python" => "pylsp",
            _ => return Err(anyhow::anyhow!("Unsupported language: {}", language)),
        };

        tracing::info!("Starting LSP server: {} for {}", command, language);

        let mut child_command = Command::new(command);
        child_command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());

        // typescript-language-server needs --stdio flag
        if language == "typescript" || language == "javascript" {
            child_command.arg("--stdio");
        }

        let mut process = child_command
            .spawn()
            .context(format!("Failed to start {} - is it installed?", command))?;

        let stdin = process.stdin.take().context("Failed to get stdin")?;
        let stdout = process.stdout.take().context("Failed to get stdout")?;

        let server = Arc::new(RwLock::new(LspServer {
            process,
            stdin,
            message_id: 0,
        }));

        // Start reading responses in background
        self.start_reader(language.to_string(), stdout).await;

        // Build root URI
        let root_uri = Url::from_file_path(root_path)
            .map_err(|_| anyhow::anyhow!("Invalid root path: {}", root_path))?;

        // Bevy-optimized initialization options for rust-analyzer
        let initialization_options = if language == "rust" {
            Some(serde_json::json!({
                "procMacro": { "enable": true, "attributes": { "enable": true } },
                "cargo": { "features": "all" },
                "diagnostics": { "disabled": ["unresolved-proc-macro"] },
                "check": { "command": "clippy" }
            }))
        } else {
            None
        };

        let init_params = InitializeParams {
            process_id: Some(std::process::id()),
            #[allow(deprecated)]
            root_uri: Some(root_uri),
            initialization_options,
            capabilities: ClientCapabilities {
                text_document: Some(TextDocumentClientCapabilities {
                    completion: Some(CompletionClientCapabilities {
                        completion_item: Some(CompletionItemCapability {
                            snippet_support: Some(true),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }),
                    hover: Some(HoverClientCapabilities {
                        content_format: Some(vec![MarkupKind::Markdown, MarkupKind::PlainText]),
                        ..Default::default()
                    }),
                    definition: Some(GotoCapability {
                        dynamic_registration: Some(false),
                        link_support: Some(false),
                    }),
                    references: Some(DynamicRegistrationClientCapabilities {
                        dynamic_registration: Some(false),
                    }),
                    publish_diagnostics: Some(PublishDiagnosticsClientCapabilities {
                        related_information: Some(true),
                        tag_support: None,
                        version_support: Some(true),
                        code_description_support: Some(true),
                        data_support: Some(false),
                    }),
                    code_action: Some(CodeActionClientCapabilities {
                        code_action_literal_support: Some(CodeActionLiteralSupport {
                            code_action_kind: CodeActionKindLiteralSupport {
                                value_set: vec![
                                    "quickfix".to_string(),
                                    "refactor".to_string(),
                                    "refactor.extract".to_string(),
                                    "refactor.inline".to_string(),
                                    "source".to_string(),
                                    "source.organizeImports".to_string(),
                                ],
                            },
                        }),
                        ..Default::default()
                    }),
                    rename: Some(RenameClientCapabilities {
                        dynamic_registration: Some(false),
                        prepare_support: Some(true),
                        ..Default::default()
                    }),
                    inlay_hint: Some(InlayHintClientCapabilities {
                        dynamic_registration: Some(false),
                        resolve_support: None,
                    }),
                    formatting: Some(DynamicRegistrationClientCapabilities {
                        dynamic_registration: Some(false),
                    }),
                    signature_help: Some(SignatureHelpClientCapabilities {
                        signature_information: Some(SignatureInformationSettings {
                            documentation_format: Some(vec![MarkupKind::Markdown, MarkupKind::PlainText]),
                            parameter_information: Some(ParameterInformationSettings {
                                label_offset_support: Some(true),
                            }),
                            active_parameter_support: Some(true),
                        }),
                        ..Default::default()
                    }),
                    synchronization: Some(TextDocumentSyncClientCapabilities {
                        dynamic_registration: Some(false),
                        will_save: Some(false),
                        will_save_wait_until: Some(false),
                        did_save: Some(true),
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        };

        let _response = self
            .send_request(server.clone(), "initialize", init_params)
            .await?;

        tracing::info!("LSP server initialized: {}", language);

        // Send initialized notification
        self.send_notification(server.clone(), "initialized", InitializedParams {})
            .await?;

        self.servers
            .write()
            .await
            .insert(language.to_string(), server);

        Ok(())
    }

    async fn start_reader(&self, language: String, stdout: ChildStdout) {
        let pending_responses = self.pending_responses.clone();
        let diagnostics_tx = self.diagnostics_tx.clone();

        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut line_buffer = String::new();

            loop {
                line_buffer.clear();

                // Read Content-Length header
                let bytes_read = match reader.read_line(&mut line_buffer) {
                    Ok(0) => break, // EOF
                    Ok(n) => n,
                    Err(e) => {
                        tracing::error!("LSP read error: {}", e);
                        break;
                    }
                };

                if bytes_read == 0 {
                    break;
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

                // Skip headers until empty line
                loop {
                    line_buffer.clear();
                    if reader.read_line(&mut line_buffer).is_err() {
                        break;
                    }
                    if line_buffer.trim().is_empty() {
                        break;
                    }
                }

                // Read content
                if let Some(len) = content_length {
                    let mut content = vec![0u8; len];
                    if let Ok(()) = reader.read_exact(&mut content) {
                        if let Ok(text) = String::from_utf8(content) {
                            if let Ok(value) = serde_json::from_str::<Value>(&text) {
                                // Check if this is a response (has "id") or a notification (no "id")
                                if let Some(id) = value.get("id").and_then(|id| id.as_u64()) {
                                    // This is a response to a request we sent
                                    tracing::debug!("LSP ({}) response id={}", language, id);
                                    let mut pending = pending_responses.write().await;
                                    if let Some(sender) = pending.remove(&id) {
                                        let _ = sender.send(value).await;
                                    }
                                } else if let Some(method) = value.get("method").and_then(|m| m.as_str()) {
                                    // This is a server notification
                                    match method {
                                        "textDocument/publishDiagnostics" => {
                                            if let Some(params) = value.get("params") {
                                                let uri = params.get("uri")
                                                    .and_then(|u| u.as_str())
                                                    .unwrap_or("")
                                                    .to_string();
                                                let diags: Vec<Diagnostic> = params.get("diagnostics")
                                                    .and_then(|d| serde_json::from_value(d.clone()).ok())
                                                    .unwrap_or_default();
                                                tracing::info!("LSP ({}) publishDiagnostics: {} diagnostics for {}",
                                                    language, diags.len(), uri);
                                                if let Some(tx) = &diagnostics_tx {
                                                    let _ = tx.send(PublishedDiagnostics {
                                                        uri,
                                                        diagnostics: diags,
                                                    });
                                                }
                                            }
                                        }
                                        "window/logMessage" => {
                                            if let Some(params) = value.get("params") {
                                                let msg = params.get("message")
                                                    .and_then(|m| m.as_str())
                                                    .unwrap_or("");
                                                let msg_type = params.get("type")
                                                    .and_then(|t| t.as_u64())
                                                    .unwrap_or(4);
                                                match msg_type {
                                                    1 => tracing::error!("LSP ({}) server: {}", language, msg),
                                                    2 => tracing::warn!("LSP ({}) server: {}", language, msg),
                                                    3 => tracing::info!("LSP ({}) server: {}", language, msg),
                                                    _ => tracing::debug!("LSP ({}) server: {}", language, msg),
                                                }
                                            }
                                        }
                                        "window/showMessage" => {
                                            if let Some(params) = value.get("params") {
                                                let msg = params.get("message")
                                                    .and_then(|m| m.as_str())
                                                    .unwrap_or("");
                                                tracing::info!("LSP ({}) showMessage: {}", language, msg);
                                            }
                                        }
                                        _ => {
                                            tracing::debug!("LSP ({}) notification: {}", language, method);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            tracing::info!("LSP reader for {} stopped", language);
        });
    }

    async fn send_request<P: serde::Serialize>(
        &self,
        server: Arc<RwLock<LspServer>>,
        method: &str,
        params: P,
    ) -> Result<Value> {
        let id = {
            let mut srv = server.write().await;
            srv.message_id += 1;
            srv.message_id
        };

        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        let content = serde_json::to_string(&request)?;
        let message = format!("Content-Length: {}\r\n\r\n{}", content.len(), content);

        tracing::debug!("LSP request [{}]: {}", method, content);

        // Create response channel
        let (tx, mut rx) = mpsc::channel(1);
        self.pending_responses.write().await.insert(id, tx);

        // Send request
        {
            let mut srv = server.write().await;
            srv.stdin
                .write_all(message.as_bytes())
                .context("Failed to write to LSP server")?;
            srv.stdin.flush()?;
        }

        // Wait for response with timeout
        let response = tokio::time::timeout(std::time::Duration::from_secs(30), rx.recv())
            .await
            .context("LSP request timed out")?
            .context("Failed to receive response")?;

        // Remove from pending if not already removed
        self.pending_responses.write().await.remove(&id);

        Ok(response)
    }

    async fn send_notification<P: serde::Serialize>(
        &self,
        server: Arc<RwLock<LspServer>>,
        method: &str,
        params: P,
    ) -> Result<()> {
        let notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });

        let content = serde_json::to_string(&notification)?;
        let message = format!("Content-Length: {}\r\n\r\n{}", content.len(), content);

        let mut srv = server.write().await;
        srv.stdin
            .write_all(message.as_bytes())
            .context("Failed to write notification to LSP server")?;
        srv.stdin.flush()?;

        Ok(())
    }

    // =========================================================================
    // File lifecycle: didOpen / didChange / didClose
    // =========================================================================

    /// Notify LSP that a file has been opened. Sends textDocument/didOpen once
    /// per file. Subsequent calls for the same file are no-ops.
    pub async fn open_file(&self, language: &str, file_path: &str, content: &str) -> Result<()> {
        let uri = file_path_to_uri(file_path)?;
        let uri_str = uri.to_string();

        // Check if already open
        {
            let opened = self.opened_files.read().await;
            if opened.contains_key(&uri_str) {
                tracing::debug!("LSP: file already open, skipping didOpen: {}", file_path);
                return Ok(());
            }
        }

        let servers = self.servers.read().await;
        let server = servers
            .get(language)
            .context("LSP server not started for this language")?
            .clone();

        let language_id = detect_language_id(file_path);
        let version = 1;

        let params = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri,
                language_id: language_id.to_string(),
                version,
                text: content.to_string(),
            },
        };

        self.send_notification(server, "textDocument/didOpen", params)
            .await?;

        self.opened_files
            .write()
            .await
            .insert(uri_str, version);

        tracing::info!("LSP: didOpen sent for {} (version {})", file_path, version);
        Ok(())
    }

    /// Notify LSP that a file's content has changed. Sends textDocument/didChange
    /// with full content sync (TextDocumentSyncKind::Full). Increments the version.
    pub async fn notify_change(&self, language: &str, file_path: &str, content: &str) -> Result<()> {
        let uri = file_path_to_uri(file_path)?;
        let uri_str = uri.to_string();

        let new_version = {
            let mut opened = self.opened_files.write().await;
            match opened.get_mut(&uri_str) {
                Some(version) => {
                    *version += 1;
                    *version
                }
                None => {
                    // File was not opened via didOpen yet -- open it first
                    drop(opened);
                    self.open_file(language, file_path, content).await?;
                    return Ok(());
                }
            }
        };

        let servers = self.servers.read().await;
        let server = servers
            .get(language)
            .context("LSP server not started for this language")?
            .clone();

        let params = DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri,
                version: new_version,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,        // None = full document sync
                range_length: None,
                text: content.to_string(),
            }],
        };

        self.send_notification(server, "textDocument/didChange", params)
            .await?;

        tracing::debug!("LSP: didChange sent for {} (version {})", file_path, new_version);
        Ok(())
    }

    /// Notify LSP that a file has been closed. Sends textDocument/didClose and
    /// removes the file from the opened_files tracker.
    pub async fn close_file(&self, language: &str, file_path: &str) -> Result<()> {
        let uri = file_path_to_uri(file_path)?;
        let uri_str = uri.to_string();

        {
            let mut opened = self.opened_files.write().await;
            if opened.remove(&uri_str).is_none() {
                // File was not tracked as open
                return Ok(());
            }
        }

        let servers = self.servers.read().await;
        let server = servers
            .get(language)
            .context("LSP server not started for this language")?
            .clone();

        let params = DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier { uri },
        };

        self.send_notification(server, "textDocument/didClose", params)
            .await?;

        tracing::info!("LSP: didClose sent for {}", file_path);
        Ok(())
    }

    // =========================================================================
    // LSP feature requests (completions, hover, goto-def, etc.)
    // =========================================================================

    /// Get code completions at a position
    pub async fn get_completions(
        &self,
        language: &str,
        file_path: String,
        line: u32,
        character: u32,
    ) -> Result<Vec<CompletionItem>> {
        let servers = self.servers.read().await;
        let server = servers
            .get(language)
            .context("LSP server not started for this language")?
            .clone();

        let file_uri = file_path_to_uri(&file_path)?;

        let params = CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: file_uri,
                },
                position: Position { line, character },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: None,
        };

        let response = self
            .send_request(server, "textDocument/completion", params)
            .await?;

        if let Some(result) = response.get("result") {
            if let Some(items) = result.get("items") {
                let completions: Vec<CompletionItem> = serde_json::from_value(items.clone())?;
                return Ok(completions);
            } else if let Ok(completions) = serde_json::from_value::<Vec<CompletionItem>>(result.clone()) {
                return Ok(completions);
            }
        }

        Ok(vec![])
    }

    /// Go to definition at a position
    pub async fn goto_definition(
        &self,
        language: &str,
        file_path: String,
        line: u32,
        character: u32,
    ) -> Result<Vec<Location>> {
        let servers = self.servers.read().await;
        let server = servers
            .get(language)
            .context("LSP server not started for this language")?
            .clone();

        let file_uri = file_path_to_uri(&file_path)?;

        let params = GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: file_uri,
                },
                position: Position { line, character },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };

        let response = self
            .send_request(server, "textDocument/definition", params)
            .await?;

        if let Some(result) = response.get("result") {
            // LSP can return Location | Location[] | LocationLink[]
            if let Ok(location) = serde_json::from_value::<Location>(result.clone()) {
                return Ok(vec![location]);
            } else if let Ok(locations) = serde_json::from_value::<Vec<Location>>(result.clone()) {
                return Ok(locations);
            }
        }

        Ok(vec![])
    }

    /// Find references at a position
    pub async fn find_references(
        &self,
        language: &str,
        file_path: String,
        line: u32,
        character: u32,
        include_declaration: bool,
    ) -> Result<Vec<Location>> {
        let servers = self.servers.read().await;
        let server = servers
            .get(language)
            .context("LSP server not started for this language")?
            .clone();

        let file_uri = file_path_to_uri(&file_path)?;

        let params = ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: file_uri,
                },
                position: Position { line, character },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: ReferenceContext {
                include_declaration,
            },
        };

        let response = self
            .send_request(server, "textDocument/references", params)
            .await?;

        if let Some(result) = response.get("result") {
            if let Ok(locations) = serde_json::from_value::<Vec<Location>>(result.clone()) {
                return Ok(locations);
            }
        }

        Ok(vec![])
    }

    /// Get hover information at a position
    pub async fn get_hover(
        &self,
        language: &str,
        file_path: String,
        line: u32,
        character: u32,
    ) -> Result<Option<Hover>> {
        let servers = self.servers.read().await;
        let server = servers
            .get(language)
            .context("LSP server not started for this language")?
            .clone();

        let file_uri = file_path_to_uri(&file_path)?;

        let params = HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: file_uri,
                },
                position: Position { line, character },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
        };

        let response = self
            .send_request(server, "textDocument/hover", params)
            .await?;

        if let Some(result) = response.get("result") {
            if !result.is_null() {
                if let Ok(hover) = serde_json::from_value::<Hover>(result.clone()) {
                    return Ok(Some(hover));
                }
            }
        }

        Ok(None)
    }

    /// Format a file using textDocument/formatting
    pub async fn format_file(
        &self,
        language: &str,
        file_path: &str,
    ) -> Result<Vec<TextEdit>> {
        let servers = self.servers.read().await;
        let server = servers
            .get(language)
            .context("LSP server not started for this language")?
            .clone();

        let file_uri = file_path_to_uri(file_path)?;

        let params = DocumentFormattingParams {
            text_document: TextDocumentIdentifier {
                uri: file_uri,
            },
            options: FormattingOptions {
                tab_size: 4,
                insert_spaces: true,
                ..Default::default()
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
        };

        let response = self
            .send_request(server, "textDocument/formatting", params)
            .await?;

        if let Some(result) = response.get("result") {
            if !result.is_null() {
                if let Ok(edits) = serde_json::from_value::<Vec<TextEdit>>(result.clone()) {
                    return Ok(edits);
                }
            }
        }

        Ok(vec![])
    }

    /// Get diagnostics for a file.
    ///
    /// Note: In LSP, diagnostics are pushed by the server via
    /// textDocument/publishDiagnostics notifications. These are received by the
    /// reader thread and forwarded through the `diagnostics_tx` channel. This
    /// method exists for API compatibility but always returns an empty vec.
    /// Use the diagnostics channel receiver instead.
    pub async fn get_diagnostics(
        &self,
        _language: &str,
        _file_path: String,
    ) -> Result<Vec<Diagnostic>> {
        // Diagnostics are pushed via publishDiagnostics notification.
        // Poll the diagnostics_rx channel from the app layer instead.
        Ok(vec![])
    }

    /// Notify LSP that a file has been saved. Sends textDocument/didSave.
    pub async fn save_file(&self, language: &str, file_path: &str) -> Result<()> {
        let uri = file_path_to_uri(file_path)?;
        let uri_str = uri.to_string();

        let version = {
            let opened = self.opened_files.read().await;
            opened.get(&uri_str).copied()
        };

        if version.is_none() {
            return Ok(()); // File not tracked, skip
        }

        let servers = self.servers.read().await;
        let server = match servers.get(language) {
            Some(s) => s.clone(),
            None => return Ok(()),
        };

        let params = DidSaveTextDocumentParams {
            text_document: TextDocumentIdentifier { uri },
            text: Some(std::fs::read_to_string(file_path).unwrap_or_default()),
        };

        self.send_notification(server, "textDocument/didSave", params)
            .await?;
        tracing::debug!("LSP: didSave sent for {}", file_path);
        Ok(())
    }

    /// Get signature help at a position
    pub async fn get_signature_help(
        &self,
        language: &str,
        file_path: String,
        line: u32,
        character: u32,
    ) -> Result<Option<SignatureHelp>> {
        let servers = self.servers.read().await;
        let server = servers
            .get(language)
            .context("LSP server not started for this language")?
            .clone();

        let file_uri = file_path_to_uri(&file_path)?;

        let params = SignatureHelpParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: file_uri },
                position: Position { line, character },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            context: None,
        };

        let response = self
            .send_request(server, "textDocument/signatureHelp", params)
            .await?;

        if let Some(result) = response.get("result") {
            if !result.is_null() {
                if let Ok(sig) = serde_json::from_value::<SignatureHelp>(result.clone()) {
                    return Ok(Some(sig));
                }
            }
        }

        Ok(None)
    }

    /// Request code actions (quick fixes) for a range
    pub async fn get_code_actions(
        &self,
        language: &str,
        file_path: String,
        start_line: u32,
        start_character: u32,
        end_line: u32,
        end_character: u32,
        diagnostics: Vec<Diagnostic>,
    ) -> Result<Vec<CodeActionOrCommand>> {
        let servers = self.servers.read().await;
        let server = servers
            .get(language)
            .context("LSP server not started")?
            .clone();

        let file_uri = file_path_to_uri(&file_path)?;

        let params = CodeActionParams {
            text_document: TextDocumentIdentifier { uri: file_uri },
            range: Range {
                start: Position { line: start_line, character: start_character },
                end: Position { line: end_line, character: end_character },
            },
            context: CodeActionContext {
                diagnostics,
                only: None,
                trigger_kind: Some(CodeActionTriggerKind::INVOKED),
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };

        let response = self.send_request(server, "textDocument/codeAction", params).await?;

        if let Some(result) = response.get("result") {
            if !result.is_null() {
                if let Ok(actions) = serde_json::from_value::<Vec<CodeActionOrCommand>>(result.clone()) {
                    return Ok(actions);
                }
            }
        }

        Ok(vec![])
    }

    /// Rename a symbol across the project
    pub async fn rename_symbol(
        &self,
        language: &str,
        file_path: String,
        line: u32,
        character: u32,
        new_name: &str,
    ) -> Result<Option<WorkspaceEdit>> {
        let servers = self.servers.read().await;
        let server = servers
            .get(language)
            .context("LSP server not started")?
            .clone();

        let file_uri = file_path_to_uri(&file_path)?;

        let params = RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: file_uri },
                position: Position { line, character },
            },
            new_name: new_name.to_string(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        };

        let response = self.send_request(server, "textDocument/rename", params).await?;

        if let Some(result) = response.get("result") {
            if !result.is_null() {
                if let Ok(edit) = serde_json::from_value::<WorkspaceEdit>(result.clone()) {
                    return Ok(Some(edit));
                }
            }
        }

        Ok(None)
    }

    /// Prepare rename (check if rename is possible)
    pub async fn prepare_rename(
        &self,
        language: &str,
        file_path: String,
        line: u32,
        character: u32,
    ) -> Result<Option<PrepareRenameResponse>> {
        let servers = self.servers.read().await;
        let server = servers
            .get(language)
            .context("LSP server not started")?
            .clone();

        let file_uri = file_path_to_uri(&file_path)?;

        let params = TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: file_uri },
            position: Position { line, character },
        };

        let response = self.send_request(server, "textDocument/prepareRename", params).await?;

        if let Some(result) = response.get("result") {
            if !result.is_null() {
                if let Ok(resp) = serde_json::from_value::<PrepareRenameResponse>(result.clone()) {
                    return Ok(Some(resp));
                }
            }
        }

        Ok(None)
    }

    /// Get inlay hints for a range
    pub async fn get_inlay_hints(
        &self,
        language: &str,
        file_path: String,
        start_line: u32,
        end_line: u32,
    ) -> Result<Vec<InlayHint>> {
        let servers = self.servers.read().await;
        let server = servers
            .get(language)
            .context("LSP server not started")?
            .clone();

        let file_uri = file_path_to_uri(&file_path)?;

        let params = InlayHintParams {
            text_document: TextDocumentIdentifier { uri: file_uri },
            range: Range {
                start: Position { line: start_line, character: 0 },
                end: Position { line: end_line, character: u32::MAX },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
        };

        let response = self.send_request(server, "textDocument/inlayHint", params).await?;

        if let Some(result) = response.get("result") {
            if !result.is_null() {
                if let Ok(hints) = serde_json::from_value::<Vec<InlayHint>>(result.clone()) {
                    return Ok(hints);
                }
            }
        }

        Ok(vec![])
    }

    /// Shutdown a language server
    pub async fn shutdown(&self, language: &str) -> Result<()> {
        let mut servers = self.servers.write().await;
        if let Some(server) = servers.remove(language) {
            let _ = self.send_request(server.clone(), "shutdown", ()).await;
            let _ = self.send_notification(server.clone(), "exit", ()).await;

            let mut srv = server.write().await;
            let _ = srv.process.kill();

            tracing::info!("LSP server shutdown: {}", language);
        }

        // Clear opened files tracking since server is gone
        self.opened_files.write().await.clear();

        Ok(())
    }

    /// Shutdown all language servers
    pub async fn shutdown_all(&self) -> Result<()> {
        let languages: Vec<String> = self.servers.read().await.keys().cloned().collect();

        for language in languages {
            self.shutdown(&language).await?;
        }

        tracing::info!("All LSP servers shutdown");
        Ok(())
    }
}

impl Drop for LspServer {
    fn drop(&mut self) {
        let _ = self.process.kill();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_native_lsp_client_creation() {
        let (client, _rx) = NativeLspClient::new();
        assert_eq!(client.servers.read().await.len(), 0);
    }

    #[test]
    fn test_file_path_to_uri() {
        let uri = file_path_to_uri("/tmp/test.rs").unwrap();
        assert_eq!(uri.scheme(), "file");
        assert!(uri.path().ends_with("/tmp/test.rs"));
    }

    #[test]
    fn test_detect_language_id() {
        assert_eq!(detect_language_id("/foo/bar.rs"), "rust");
        assert_eq!(detect_language_id("/foo/bar.ts"), "typescript");
        assert_eq!(detect_language_id("/foo/bar.py"), "python");
        assert_eq!(detect_language_id("/foo/bar.txt"), "plaintext");
    }
}
