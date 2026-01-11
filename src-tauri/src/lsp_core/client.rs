//! LSP Client Implementation
//! Manages LSP server process and communication

use super::types::*;
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// LSP Client
pub struct LspClient {
    language: String,
    process: Option<Child>,
    stdin: Option<ChildStdin>,
    stdout: Option<BufReader<ChildStdout>>,
    request_id: Arc<AtomicU64>,
    capabilities: ServerCapabilities,
}

impl LspClient {
    /// Create new LSP client for a language
    pub fn new(language: &str, root_uri: &str) -> Result<Self, String> {
        let mut client = Self {
            language: language.to_string(),
            process: None,
            stdin: None,
            stdout: None,
            request_id: Arc::new(AtomicU64::new(1)),
            capabilities: ServerCapabilities::default(),
        };

        // Start language server process
        client.start_server()?;

        // Initialize
        client.initialize(root_uri)?;

        Ok(client)
    }

    /// Start language server process
    fn start_server(&mut self) -> Result<(), String> {
        let (command, args) = Self::get_server_command(&self.language)?;

        eprintln!("[LSP] Starting server: command={:?}, args={:?}", command, args);

        let mut process = Command::new(&command)
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped()) // ✅ FIX: Capture stderr to see rust-analyzer errors
            .spawn()
            .map_err(|e| format!("Failed to start LSP server '{}': {}", command, e))?;

        let stdin = process.stdin.take().ok_or("Failed to get stdin")?;

        let stdout = process.stdout.take().ok_or("Failed to get stdout")?;

        // ✅ FIX: Spawn a thread to capture and log stderr
        if let Some(stderr) = process.stderr.take() {
            let language = self.language.clone();
            std::thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines() {
                    if let Ok(line) = line {
                        eprintln!("[LSP stderr:{}] {}", language, line);
                    }
                }
            });
        }

        eprintln!("[LSP] Server process started successfully");

        self.process = Some(process);
        self.stdin = Some(stdin);
        self.stdout = Some(BufReader::new(stdout));

        Ok(())
    }

    /// Get server command for language
    fn get_server_command(language: &str) -> Result<(String, Vec<String>), String> {
        match language {
            "rust" => {
                // Try to find rust-analyzer in common locations
                let rust_analyzer = Self::find_executable("rust-analyzer")
                    .ok_or_else(|| "rust-analyzer not found. Install with: rustup component add rust-analyzer".to_string())?;
                Ok((rust_analyzer, vec![]))
            }
            "typescript" | "javascript" => {
                let ts_server = Self::find_executable("typescript-language-server")
                    .unwrap_or_else(|| "typescript-language-server".to_string());
                Ok((ts_server, vec!["--stdio".to_string()]))
            }
            "python" => {
                let pyright = Self::find_executable("pyright-langserver")
                    .unwrap_or_else(|| "pyright-langserver".to_string());
                Ok((pyright, vec!["--stdio".to_string()]))
            }
            _ => Err(format!("Unsupported language: {}", language)),
        }
    }

    /// Find executable in PATH or common locations
    fn find_executable(name: &str) -> Option<String> {
        eprintln!("[LSP] Finding executable: {}", name);

        // For rust-analyzer, check common locations FIRST (more reliable than which)
        if name == "rust-analyzer" {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/Users/default".to_string());
            let cargo_path = format!("{}/.cargo/bin/rust-analyzer", home);

            let common_paths = vec![
                "/opt/homebrew/bin/rust-analyzer",  // Homebrew on Apple Silicon (MOST COMMON)
                cargo_path.as_str(),                 // Cargo install
                "/usr/local/bin/rust-analyzer",      // Homebrew on Intel Mac
                "/usr/bin/rust-analyzer",            // System install
            ];

            for path in &common_paths {
                eprintln!("[LSP] Checking path: {}", path);
                if std::path::Path::new(path).exists() {
                    eprintln!("[LSP] ✅ Found rust-analyzer at: {}", path);
                    return Some(path.to_string());
                }
            }
            eprintln!("[LSP] ❌ rust-analyzer not found in any common location");
            eprintln!("[LSP] Searched: {:?}", common_paths);
        }

        // Try using `which` command as fallback
        if let Ok(output) = std::process::Command::new("which")
            .arg(name)
            .output()
        {
            if output.status.success() {
                if let Ok(path) = String::from_utf8(output.stdout) {
                    let path = path.trim();
                    if !path.is_empty() {
                        eprintln!("[LSP] Found via which: {}", path);
                        return Some(path.to_string());
                    }
                }
            }
        }

        // Fallback to just the name (will search PATH)
        Some(name.to_string())
    }

    /// Initialize LSP server
    fn initialize(&mut self, root_uri: &str) -> Result<(), String> {
        eprintln!("[LSP] Initializing server with root_uri: {}", root_uri);

        let params = serde_json::json!({
            "processId": std::process::id(),
            "rootUri": root_uri,
            "capabilities": {
                "textDocument": {
                    "completion": {
                        "completionItem": {
                            "snippetSupport": true
                        }
                    },
                    "hover": {
                        "contentFormat": ["markdown", "plaintext"]
                    }
                }
            }
        });

        eprintln!("[LSP] Sending initialize request...");
        let id = self.next_request_id();
        let request = LspRequest::new(id, "initialize", Some(params));

        let response = self.send_request(request)?;
        eprintln!("[LSP] Received initialize response");

        // Parse server capabilities
        if let Some(result) = response.result {
            if let Some(caps) = result.get("capabilities") {
                self.capabilities = serde_json::from_value(caps.clone()).unwrap_or_default();
            }
        }

        // Send initialized notification
        let notification = LspNotification::new("initialized", Some(serde_json::json!({})));
        self.send_notification(notification)?;

        Ok(())
    }

    /// Send request and wait for response
    fn send_request(&mut self, request: LspRequest) -> Result<LspResponse, String> {
        let message = serde_json::to_string(&request)
            .map_err(|e| format!("Failed to serialize request: {}", e))?;

        self.write_message(&message)?;

        // Read response (simplified - in production, use async/await)
        self.read_response(request.id)
    }

    /// Send notification (no response expected)
    fn send_notification(&mut self, notification: LspNotification) -> Result<(), String> {
        let message = serde_json::to_string(&notification)
            .map_err(|e| format!("Failed to serialize notification: {}", e))?;

        self.write_message(&message)
    }

    /// Write message to stdin
    fn write_message(&mut self, message: &str) -> Result<(), String> {
        let stdin = self.stdin.as_mut().ok_or("No stdin available")?;

        let content_length = message.len();
        let header = format!("Content-Length: {}\r\n\r\n", content_length);

        stdin
            .write_all(header.as_bytes())
            .map_err(|e| format!("Failed to write header: {}", e))?;

        stdin
            .write_all(message.as_bytes())
            .map_err(|e| format!("Failed to write message: {}", e))?;

        stdin
            .flush()
            .map_err(|e| format!("Failed to flush: {}", e))?;

        Ok(())
    }

    /// Read response from stdout
    fn read_response(&mut self, expected_id: u64) -> Result<LspResponse, String> {
        let stdout = self.stdout.as_mut().ok_or("No stdout available")?;

        // Safety: Limit iterations to prevent infinite loop
        const MAX_RESPONSE_ATTEMPTS: usize = 1000;
        let mut attempt_count = 0;

        loop {
            attempt_count += 1;
            if attempt_count > MAX_RESPONSE_ATTEMPTS {
                return Err(format!("Timeout: No response with ID {} after {} attempts", expected_id, MAX_RESPONSE_ATTEMPTS));
            }

            // Read headers
            let mut headers = Vec::new();
            let mut line = String::new();

            // Safety: Limit header line reads to prevent infinite loop on EOF
            const MAX_HEADER_LINES: usize = 100;
            let mut header_line_count = 0;

            loop {
                header_line_count += 1;
                if header_line_count > MAX_HEADER_LINES {
                    return Err("Too many header lines - possible protocol error".to_string());
                }

                line.clear();
                let bytes_read = stdout
                    .read_line(&mut line)
                    .map_err(|e| format!("Failed to read line: {}", e))?;

                // Safety: Detect EOF (0 bytes read)
                if bytes_read == 0 {
                    return Err("LSP server closed connection (EOF)".to_string());
                }

                if line == "\r\n" || line == "\n" {
                    break;
                }

                headers.push(line.clone());
            }

            // Parse Content-Length
            let content_length: usize = headers
                .iter()
                .find(|h| h.starts_with("Content-Length:"))
                .and_then(|h| h.split(':').nth(1))
                .and_then(|s| s.trim().parse().ok())
                .ok_or("No Content-Length header")?;

            // Read content
            let mut buffer = vec![0u8; content_length];
            std::io::Read::read_exact(stdout, &mut buffer)
                .map_err(|e| format!("Failed to read content: {}", e))?;

            let content = String::from_utf8(buffer).map_err(|e| format!("Invalid UTF-8: {}", e))?;

            // Parse message
            let message: LspMessage = serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse message: {}", e))?;

            match message {
                LspMessage::Response(response) => {
                    if response.id == expected_id {
                        return Ok(response);
                    }
                    // Wrong response ID, continue reading
                }
                LspMessage::Notification(_notification) => {
                    // Handle notification (e.g., diagnostics) - for now, ignore
                    continue;
                }
                LspMessage::Request(_request) => {
                    // Server sent a request - for now, ignore
                    continue;
                }
            }
        }
    }

    /// Get next request ID
    fn next_request_id(&self) -> u64 {
        self.request_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Get completions at position
    pub fn get_completions(
        &mut self,
        file_uri: &str,
        line: u32,
        character: u32,
    ) -> Result<Vec<CompletionItem>, String> {
        let params = serde_json::json!({
            "textDocument": {
                "uri": file_uri
            },
            "position": {
                "line": line,
                "character": character
            }
        });

        let id = self.next_request_id();
        let request = LspRequest::new(id, "textDocument/completion", Some(params));

        let response = self.send_request(request)?;

        if let Some(result) = response.result {
            // Result can be CompletionList or Vec<CompletionItem>
            if let Some(items) = result.get("items") {
                // CompletionList
                return serde_json::from_value(items.clone())
                    .map_err(|e| format!("Failed to parse completion items: {}", e));
            } else {
                // Direct Vec<CompletionItem>
                return serde_json::from_value(result)
                    .map_err(|e| format!("Failed to parse completion items: {}", e));
            }
        }

        Ok(Vec::new())
    }

    /// Get hover information at position
    pub fn get_hover(
        &mut self,
        file_uri: &str,
        line: u32,
        character: u32,
    ) -> Result<Option<Hover>, String> {
        let params = serde_json::json!({
            "textDocument": {
                "uri": file_uri
            },
            "position": {
                "line": line,
                "character": character
            }
        });

        let id = self.next_request_id();
        let request = LspRequest::new(id, "textDocument/hover", Some(params));

        let response = self.send_request(request)?;

        if let Some(result) = response.result {
            if result.is_null() {
                return Ok(None);
            }

            let hover: Hover = serde_json::from_value(result)
                .map_err(|e| format!("Failed to parse hover: {}", e))?;

            return Ok(Some(hover));
        }

        Ok(None)
    }

    /// Go to definition at position
    pub fn goto_definition(
        &mut self,
        file_uri: &str,
        line: u32,
        character: u32,
    ) -> Result<Option<Location>, String> {
        println!("[LSP Client] goto_definition: uri={}, line={}, char={}", file_uri, line, character);

        let params = serde_json::json!({
            "textDocument": {
                "uri": file_uri
            },
            "position": {
                "line": line,
                "character": character
            }
        });

        let id = self.next_request_id();
        let request = LspRequest::new(id, "textDocument/definition", Some(params));

        println!("[LSP Client] Sending request to language server...");
        let response = self.send_request(request)?;
        println!("[LSP Client] Received response from language server");

        if let Some(result) = response.result {
            if result.is_null() {
                println!("[LSP Client] Response is null - no definition found");
                return Ok(None);
            }

            println!("[LSP Client] Response result: {:?}", result);

            // Result can be Location, Vec<Location>, or LocationLink[]
            // Try to parse as single Location first
            if let Ok(location) = serde_json::from_value::<Location>(result.clone()) {
                println!("[LSP Client] Parsed as single Location: uri={}, line={}",
                    location.uri, location.range.start.line);
                return Ok(Some(location));
            }

            // Try to parse as Vec<Location>
            if let Ok(locations) = serde_json::from_value::<Vec<Location>>(result.clone()) {
                println!("[LSP Client] Parsed as Vec<Location> with {} items", locations.len());
                if let Some(first) = locations.into_iter().next() {
                    println!("[LSP Client] Using first location: uri={}, line={}",
                        first.uri, first.range.start.line);
                    return Ok(Some(first));
                }
            }

            println!("[LSP Client] ERROR: Failed to parse definition response");
            return Err(format!("Failed to parse definition response: {:?}", result));
        }

        println!("[LSP Client] No result in response");
        Ok(None)
    }

    /// Shutdown LSP server
    pub fn shutdown(&mut self) -> Result<(), String> {
        // Send shutdown request
        let id = self.next_request_id();
        let request = LspRequest::new(id, "shutdown", None);
        let _ = self.send_request(request); // Ignore errors

        // Send exit notification
        let notification = LspNotification::new("exit", None);
        let _ = self.send_notification(notification); // Ignore errors

        // Kill process
        if let Some(mut process) = self.process.take() {
            let _ = process.kill();
            let _ = process.wait();
        }

        Ok(())
    }

    /// Get server capabilities
    pub fn capabilities(&self) -> &ServerCapabilities {
        &self.capabilities
    }
}

impl Drop for LspClient {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_server_command_rust() {
        let result = LspClient::get_server_command("rust");
        // Should either find rust-analyzer or return an error with install instructions
        match result {
            Ok((cmd, args)) => {
                assert!(cmd.contains("rust-analyzer"), "Command should contain rust-analyzer, got: {}", cmd);
                assert_eq!(args.len(), 0);
            }
            Err(e) => {
                assert!(e.contains("rustup component add"), "Error should mention installation: {}", e);
            }
        }
    }

    #[test]
    fn test_find_executable_rust_analyzer() {
        let result = LspClient::find_executable("rust-analyzer");
        // Should find rust-analyzer if installed
        if let Some(path) = result {
            assert!(path.contains("rust-analyzer"), "Path should contain rust-analyzer: {}", path);
            assert!(
                std::path::Path::new(&path).exists() || path == "rust-analyzer",
                "Path should exist or be a bare command name: {}",
                path
            );
        }
    }

    #[test]
    fn test_get_server_command_typescript() {
        let (cmd, args) = LspClient::get_server_command("typescript").unwrap();
        assert_eq!(cmd, "typescript-language-server");
        assert!(args.contains(&"--stdio".to_string()));
    }

    #[test]
    fn test_get_server_command_unsupported() {
        let result = LspClient::get_server_command("unknown");
        assert!(result.is_err());
    }

    #[test]
    fn test_next_request_id() {
        let client = LspClient {
            language: "rust".to_string(),
            process: None,
            stdin: None,
            stdout: None,
            request_id: Arc::new(AtomicU64::new(1)),
            capabilities: ServerCapabilities::default(),
        };

        assert_eq!(client.next_request_id(), 1);
        assert_eq!(client.next_request_id(), 2);
        assert_eq!(client.next_request_id(), 3);
    }
}
