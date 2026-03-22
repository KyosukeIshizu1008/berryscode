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

/// Native LSP client that communicates directly with LSP servers
pub struct NativeLspClient {
    servers: Arc<RwLock<HashMap<String, Arc<RwLock<LspServer>>>>>,
    pending_responses: Arc<RwLock<HashMap<u64, mpsc::Sender<Value>>>>,
}

struct LspServer {
    process: Child,
    stdin: ChildStdin,
    message_id: u64,
}

impl NativeLspClient {
    pub fn new() -> Self {
        Self {
            servers: Arc::new(RwLock::new(HashMap::new())),
            pending_responses: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start an LSP server for a language
    pub async fn start_server(&self, language: &str, root_path: &str) -> Result<()> {
        let command = match language {
            "rust" => "rust-analyzer",
            "typescript" | "javascript" => "typescript-language-server",
            "python" => "pylsp",
            _ => return Err(anyhow::anyhow!("Unsupported language: {}", language)),
        };

        tracing::info!("🚀 Starting LSP server: {} for {}", command, language);

        let mut process = Command::new(command)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
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

        // Send initialize request
        let root_uri = if root_path.starts_with('/') {
            format!("file://{}", root_path)
        } else {
            format!("file:///{}", root_path)
        };

        let init_params = InitializeParams {
            process_id: Some(std::process::id()),
            root_uri: Some(Url::parse(&root_uri)?),
            initialization_options: None,
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
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        };

        let _response = self
            .send_request(server.clone(), "initialize", init_params)
            .await?;

        tracing::info!("✅ LSP server initialized: {}", language);

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
                        tracing::error!("❌ LSP read error: {}", e);
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
                                tracing::debug!("📨 LSP ({}) response: {}", language, text);

                                if let Some(id) = value.get("id").and_then(|id| id.as_u64()) {
                                    // This is a response to a request
                                    let mut pending = pending_responses.write().await;
                                    if let Some(sender) = pending.remove(&id) {
                                        let _ = sender.send(value).await;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            tracing::info!("📤 LSP reader for {} stopped", language);
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

        tracing::debug!("📤 LSP request: {}", message);

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

        let file_uri = if file_path.starts_with('/') {
            format!("file://{}", file_path)
        } else {
            format!("file:///{}", file_path)
        };

        // Send textDocument/didOpen notification first
        let did_open_params = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: Url::parse(&file_uri)?,
                language_id: language.to_string(),
                version: 1,
                text: std::fs::read_to_string(&file_path).unwrap_or_default(),
            },
        };
        let _ = self.send_notification(server.clone(), "textDocument/didOpen", did_open_params).await;

        let params = CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: Url::parse(&file_uri)?,
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

        let file_uri = if file_path.starts_with('/') {
            format!("file://{}", file_path)
        } else {
            format!("file:///{}", file_path)
        };

        let params = GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: Url::parse(&file_uri)?,
                },
                position: Position { line, character },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };

        // First, send textDocument/didOpen notification to ensure server knows about the file
        let did_open_params = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: Url::parse(&file_uri)?,
                language_id: language.to_string(),
                version: 1,
                text: std::fs::read_to_string(&file_path).unwrap_or_default(),
            },
        };
        let _ = self.send_notification(server.clone(), "textDocument/didOpen", did_open_params).await;

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

        let file_uri = if file_path.starts_with('/') {
            format!("file://{}", file_path)
        } else {
            format!("file:///{}", file_path)
        };

        let params = ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: Url::parse(&file_uri)?,
                },
                position: Position { line, character },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: ReferenceContext {
                include_declaration,
            },
        };

        // Send textDocument/didOpen notification first
        let did_open_params = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: Url::parse(&file_uri)?,
                language_id: language.to_string(),
                version: 1,
                text: std::fs::read_to_string(&file_path).unwrap_or_default(),
            },
        };
        let _ = self.send_notification(server.clone(), "textDocument/didOpen", did_open_params).await;

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

        let file_uri = if file_path.starts_with('/') {
            format!("file://{}", file_path)
        } else {
            format!("file:///{}", file_path)
        };

        let params = HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: Url::parse(&file_uri)?,
                },
                position: Position { line, character },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
        };

        // Send textDocument/didOpen notification first
        let did_open_params = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: Url::parse(&file_uri)?,
                language_id: language.to_string(),
                version: 1,
                text: std::fs::read_to_string(&file_path).unwrap_or_default(),
            },
        };
        let _ = self.send_notification(server.clone(), "textDocument/didOpen", did_open_params).await;

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

    /// Get diagnostics for a file
    /// Note: LSP diagnostics are typically pushed by the server via publishDiagnostics notification
    /// This method is for compatibility but may not return real-time diagnostics
    pub async fn get_diagnostics(
        &self,
        language: &str,
        file_path: String,
    ) -> Result<Vec<Diagnostic>> {
        // In LSP, diagnostics are pushed by the server via textDocument/publishDiagnostics notification
        // There's no request method to pull diagnostics
        // For now, we return an empty vec
        // To properly handle diagnostics, we would need to:
        // 1. Listen for publishDiagnostics notifications in the reader
        // 2. Store them in a shared HashMap
        // 3. Return them from this method
        tracing::warn!("⚠️ get_diagnostics is not fully implemented for native LSP");
        tracing::warn!("   LSP diagnostics are pushed by server via notifications, not pulled");
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

            tracing::info!("✅ LSP server shutdown: {}", language);
        }
        Ok(())
    }

    /// Shutdown all language servers
    pub async fn shutdown_all(&self) -> Result<()> {
        let languages: Vec<String> = self.servers.read().await.keys().cloned().collect();

        for language in languages {
            self.shutdown(&language).await?;
        }

        tracing::info!("✅ All LSP servers shutdown");
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
        let client = NativeLspClient::new();
        assert_eq!(client.servers.read().await.len(), 0);
    }
}
