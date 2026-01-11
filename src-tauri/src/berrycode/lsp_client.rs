//! LSP (Language Server Protocol) client for intelligent code navigation
//!
//! This module provides IDE-level code intelligence by communicating with
//! language servers like rust-analyzer, typescript-language-server, etc.
//!
//! Benefits:
//! - Go to Definition: Find where a symbol is defined (instant, no grep)
//! - Find References: Find all usages of a symbol
//! - Hover: Get type information and documentation
//! - No hallucinations: Only returns real symbols that exist

use crate::berrycode::Result;
use anyhow::anyhow;
use lsp_types::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::io::{BufRead, BufReader, Write};
use lsp_types::InlayHint;
use lsp_types::CodeLens;

/// LSP client that manages language server processes
#[derive(Clone)]
pub struct LspClient {
    /// Root directory of the project
    project_root: PathBuf,
    /// Active language servers (by language ID)
    servers: Arc<Mutex<HashMap<String, LanguageServer>>>,
    /// Diagnostics cache (by file URI)
    diagnostics: Arc<Mutex<HashMap<String, Vec<Diagnostic>>>>,
}

/// A running language server instance
struct LanguageServer {
    /// The server process
    process: Child,
    /// Next request ID
    next_id: i64,
    /// Initialized flag
    #[allow(dead_code)]
    initialized: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: i64,
    method: String,
    params: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

impl LspClient {
    /// Create a new LSP client for the given project
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            project_root,
            servers: Arc::new(Mutex::new(HashMap::new())),
            diagnostics: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Get diagnostics for a file
    pub fn get_diagnostics(&self, file_path: &Path) -> Result<Vec<Diagnostic>> {
        let uri = Url::from_file_path(file_path)
            .map_err(|_| anyhow!("Invalid file path"))?;
        let uri_str = uri.to_string();

        let diagnostics = self.diagnostics.lock().unwrap();
        Ok(diagnostics.get(&uri_str).cloned().unwrap_or_default())
    }

    /// Start a language server for the given language
    pub fn start_server(&self, language: &str) -> Result<()> {
        let command = match language {
            // Systems programming
            "rust" => vec!["rust-analyzer"],
            "c" | "cpp" | "c++" => vec!["clangd"],
            "go" => vec!["gopls"],

            // Web & scripting
            "typescript" | "javascript" | "tsx" | "jsx" => vec!["typescript-language-server", "--stdio"],
            "python" => vec!["pylsp"],
            "ruby" => vec!["solargraph", "stdio"],
            "php" => vec!["intelephense", "--stdio"],

            // Web frameworks
            "vue" => vec!["vue-language-server", "--stdio"],
            "svelte" => vec!["svelteserver", "--stdio"],
            "astro" => vec!["astro-ls", "--stdio"],

            // JVM languages
            "java" => vec!["jdtls"],
            "kotlin" => vec!["kotlin-language-server"],
            "scala" => vec!["metals"],

            // .NET
            "csharp" | "cs" => vec!["omnisharp", "--languageserver"],

            // Functional languages
            "haskell" => vec!["haskell-language-server-wrapper", "--lsp"],
            "elixir" => vec!["elixir-ls"],
            "ocaml" => vec!["ocamllsp"],

            // Other popular languages
            "lua" => vec!["lua-language-server"],
            "swift" => vec!["sourcekit-lsp"],
            "dart" => vec!["dart", "language-server"],
            "zig" => vec!["zls"],

            // Shell & config
            "shell" | "bash" | "zsh" => vec!["bash-language-server", "start"],

            // Markup & data languages
            "html" => vec!["vscode-html-language-server", "--stdio"],
            "css" | "scss" | "sass" | "less" => vec!["vscode-css-language-server", "--stdio"],
            "json" => vec!["vscode-json-language-server", "--stdio"],
            "yaml" | "yml" => vec!["yaml-language-server", "--stdio"],
            "xml" => vec!["lemminx"],
            "toml" => vec!["taplo", "lsp", "stdio"],
            "markdown" | "md" => vec!["marksman", "server"],

            // SQL
            "sql" => vec!["sql-language-server", "up", "--method", "stdio"],

            _ => return Err(anyhow!("No language server available for: {}", language)),
        };

        let mut process = Command::new(command[0])
            .args(&command[1..])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| anyhow!("Failed to start {} language server: {}. Make sure it's installed.", language, e))?;

        // Send initialize request
        let root_uri = Url::from_file_path(&self.project_root)
            .map_err(|_| anyhow!("Invalid project root path"))?;
        let init_params = InitializeParams {
            process_id: Some(std::process::id()),
            root_uri: None, // Deprecated - using workspace_folders instead
            workspace_folders: Some(vec![WorkspaceFolder {
                uri: root_uri,
                name: self.project_root.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("project")
                    .to_string(),
            }]),
            capabilities: ClientCapabilities {
                text_document: Some(TextDocumentClientCapabilities {
                    definition: Some(GotoCapability {
                        dynamic_registration: Some(false),
                        link_support: Some(false),
                    }),
                    references: Some(DynamicRegistrationClientCapabilities {
                        dynamic_registration: Some(false),
                    }),
                    hover: Some(HoverClientCapabilities {
                        dynamic_registration: Some(false),
                        content_format: Some(vec![MarkupKind::PlainText]),
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        };

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "initialize".to_string(),
            params: serde_json::to_value(init_params)?,
        };

        // Send request to server
        if let Some(stdin) = process.stdin.as_mut() {
            let request_str = serde_json::to_string(&request)?;
            let content_length = request_str.len();

            writeln!(stdin, "Content-Length: {}\r\n\r\n{}", content_length, request_str)?;
            stdin.flush()?;
        }

        let server = LanguageServer {
            process,
            next_id: 2,
            initialized: true,
        };

        self.servers.lock().unwrap().insert(language.to_string(), server);

        tracing::info!("Started {} language server", language);
        Ok(())
    }

    /// Go to definition of a symbol
    /// Returns the file path and line number where the symbol is defined
    pub fn goto_definition(&self, file_path: &Path, line: u32, character: u32) -> Result<Option<Location>> {
        let language = self.detect_language(file_path)?;

        // Ensure server is started
        if !self.servers.lock().unwrap().contains_key(&language) {
            self.start_server(&language)?;
        }

        let params = GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: Url::from_file_path(file_path)
                        .map_err(|_| anyhow!("Invalid file path"))?,
                },
                position: Position { line, character },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };

        let response = self.send_request(&language, "textDocument/definition", params)?;

        // Parse response
        if let Some(result) = response.result {
            if let Ok(location) = serde_json::from_value::<Location>(result.clone()) {
                return Ok(Some(location));
            }
            if let Ok(locations) = serde_json::from_value::<Vec<Location>>(result) {
                return Ok(locations.into_iter().next());
            }
        }

        Ok(None)
    }

    /// Find all references to a symbol
    pub fn find_references(&self, file_path: &Path, line: u32, character: u32) -> Result<Vec<Location>> {
        let language = self.detect_language(file_path)?;

        // Ensure server is started
        if !self.servers.lock().unwrap().contains_key(&language) {
            self.start_server(&language)?;
        }

        let params = ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: Url::from_file_path(file_path)
                        .map_err(|_| anyhow!("Invalid file path"))?,
                },
                position: Position { line, character },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: ReferenceContext {
                include_declaration: true,
            },
        };

        let response = self.send_request(&language, "textDocument/references", params)?;

        if let Some(result) = response.result {
            if let Ok(locations) = serde_json::from_value::<Vec<Location>>(result) {
                return Ok(locations);
            }
        }

        Ok(Vec::new())
    }

    /// Get hover information (type, documentation) for a symbol
    pub fn hover(&self, file_path: &Path, line: u32, character: u32) -> Result<Option<String>> {
        let language = self.detect_language(file_path)?;

        // Ensure server is started
        if !self.servers.lock().unwrap().contains_key(&language) {
            self.start_server(&language)?;
        }

        let params = HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: Url::from_file_path(file_path)
                        .map_err(|_| anyhow!("Invalid file path"))?,
                },
                position: Position { line, character },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
        };

        let response = self.send_request(&language, "textDocument/hover", params)?;

        if let Some(result) = response.result {
            if let Ok(hover) = serde_json::from_value::<Hover>(result) {
                let text = match hover.contents {
                    HoverContents::Scalar(content) => match content {
                        MarkedString::String(s) => s,
                        MarkedString::LanguageString(ls) => ls.value,
                    },
                    HoverContents::Array(arr) => {
                        arr.into_iter()
                            .map(|ms| match ms {
                                MarkedString::String(s) => s,
                                MarkedString::LanguageString(ls) => ls.value,
                            })
                            .collect::<Vec<_>>()
                            .join("\n")
                    }
                    HoverContents::Markup(markup) => markup.value,
                };
                return Ok(Some(text));
            }
        }

        Ok(None)
    }

    /// Rename a symbol across the workspace
    /// Returns a WorkspaceEdit containing all the changes needed
    pub fn rename(&self, file_path: &Path, line: u32, character: u32, new_name: &str) -> Result<Option<WorkspaceEdit>> {
        let language = self.detect_language(file_path)?;

        // Ensure server is started
        if !self.servers.lock().unwrap().contains_key(&language) {
            self.start_server(&language)?;
        }

        let params = RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: Url::from_file_path(file_path)
                        .map_err(|_| anyhow!("Invalid file path"))?,
                },
                position: Position { line, character },
            },
            new_name: new_name.to_string(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        };

        let response = self.send_request(&language, "textDocument/rename", params)?;

        if let Some(result) = response.result {
            if let Ok(workspace_edit) = serde_json::from_value::<WorkspaceEdit>(result) {
                return Ok(Some(workspace_edit));
            }
        }

        Ok(None)
    }

    /// Get code actions (quick fixes) for a range
    pub fn code_actions(
        &self,
        file_path: &Path,
        start_line: u32,
        start_char: u32,
        end_line: u32,
        end_char: u32,
        diagnostics: Vec<Diagnostic>,
    ) -> Result<Vec<CodeActionOrCommand>> {
        let language = self.detect_language(file_path)?;

        // Ensure server is started
        if !self.servers.lock().unwrap().contains_key(&language) {
            self.start_server(&language)?;
        }

        let params = CodeActionParams {
            text_document: TextDocumentIdentifier {
                uri: Url::from_file_path(file_path)
                    .map_err(|_| anyhow!("Invalid file path"))?,
            },
            range: Range {
                start: Position {
                    line: start_line,
                    character: start_char,
                },
                end: Position {
                    line: end_line,
                    character: end_char,
                },
            },
            context: CodeActionContext {
                diagnostics,
                only: None,
                trigger_kind: None,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };

        let response = self.send_request(&language, "textDocument/codeAction", params)?;

        if let Some(result) = response.result {
            if let Ok(actions) = serde_json::from_value::<Vec<CodeActionOrCommand>>(result) {
                return Ok(actions);
            }
        }

        Ok(Vec::new())
    }

    /// Get completion items at a position
    pub fn completion(
        &self,
        file_path: &Path,
        line: u32,
        character: u32,
    ) -> Result<Vec<CompletionItem>> {
        let language = self.detect_language(file_path)?;

        // Ensure server is started
        if !self.servers.lock().unwrap().contains_key(&language) {
            self.start_server(&language)?;
        }

        let params = CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: Url::from_file_path(file_path)
                        .map_err(|_| anyhow!("Invalid file path"))?,
                },
                position: Position { line, character },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: None,
        };

        let response = self.send_request(&language, "textDocument/completion", params)?;

        if let Some(result) = response.result {
            // Try as CompletionList first
            if let Ok(list) = serde_json::from_value::<CompletionList>(result.clone()) {
                return Ok(list.items);
            }
            // Try as Vec<CompletionItem>
            if let Ok(items) = serde_json::from_value::<Vec<CompletionItem>>(result) {
                return Ok(items);
            }
        }

        Ok(Vec::new())
    }

    /// Get signature help (parameter hints) at a position
    pub fn signature_help(
        &self,
        file_path: &Path,
        line: u32,
        character: u32,
    ) -> Result<Option<SignatureHelp>> {
        let language = self.detect_language(file_path)?;

        // Ensure server is started
        if !self.servers.lock().unwrap().contains_key(&language) {
            self.start_server(&language)?;
        }

        let params = SignatureHelpParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: Url::from_file_path(file_path)
                        .map_err(|_| anyhow!("Invalid file path"))?,
                },
                position: Position { line, character },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            context: None,
        };

        let response = self.send_request(&language, "textDocument/signatureHelp", params)?;

        if let Some(result) = response.result {
            if let Ok(sig_help) = serde_json::from_value::<SignatureHelp>(result) {
                return Ok(Some(sig_help));
            }
        }

        Ok(None)
    }

    /// Get Inlay Hints for a file and a specific range
    pub fn inlay_hints(
        &self,
        file_path: &Path,
        range: lsp_types::Range
    ) -> Result<Vec<InlayHint>> {
        let language = self.detect_language(file_path)?;

        // Ensure server is started
        if !self.servers.lock().unwrap().contains_key(&language) {
            self.start_server(&language)?;
        }

        let params = InlayHintParams {
            text_document: TextDocumentIdentifier {
                uri: Url::from_file_path(file_path)
                    .map_err(|_| anyhow!("Invalid file path"))?,
            },
            range,
            work_done_progress_params: WorkDoneProgressParams::default(),
        };

        let response = self.send_request(&language, "textDocument/inlayHint", params)?;

        if let Some(result) = response.result {
            if let Ok(hints) = serde_json::from_value::<Vec<InlayHint>>(result) {
                return Ok(hints);
            }
        }

        Ok(Vec::new())
    }

    /// Get Code Lenses for a file
    pub fn code_lens(&self, file_path: &Path) -> Result<Vec<CodeLens>> {
        let language = self.detect_language(file_path)?;

        // Ensure server is started
        if !self.servers.lock().unwrap().contains_key(&language) {
            self.start_server(&language)?;
        }

        let params = CodeLensParams {
            text_document: TextDocumentIdentifier {
                uri: Url::from_file_path(file_path)
                    .map_err(|_| anyhow!("Invalid file path"))?,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };

        let response = self.send_request(&language, "textDocument/codeLens", params)?;

        if let Some(result) = response.result {
            if let Ok(lenses) = serde_json::from_value::<Vec<CodeLens>>(result) {
                return Ok(lenses);
            }
        }

        Ok(Vec::new())
    }

    /// Get document symbols (outline) for a file
    pub fn document_symbols(&self, file_path: &Path) -> Result<Vec<DocumentSymbol>> {
        let language = self.detect_language(file_path)?;

        // Ensure server is started
        if !self.servers.lock().unwrap().contains_key(&language) {
            self.start_server(&language)?;
        }

        let params = DocumentSymbolParams {
            text_document: TextDocumentIdentifier {
                uri: Url::from_file_path(file_path)
                    .map_err(|_| anyhow!("Invalid file path"))?,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };

        let response = self.send_request(&language, "textDocument/documentSymbol", params)?;

        if let Some(result) = response.result {
            // Try parsing as Vec<DocumentSymbol>
            if let Ok(symbols) = serde_json::from_value::<Vec<DocumentSymbol>>(result.clone()) {
                return Ok(symbols);
            }
            // Try parsing as Vec<SymbolInformation> (older LSP spec)
            if let Ok(symbol_info) = serde_json::from_value::<Vec<SymbolInformation>>(result) {
                // Convert SymbolInformation to DocumentSymbol
                return Ok(symbol_info.into_iter().map(|info| DocumentSymbol {
                    name: info.name,
                    detail: None,
                    kind: info.kind,
                    tags: info.tags,
                    deprecated: info.deprecated,
                    range: info.location.range,
                    selection_range: info.location.range,
                    children: None,
                }).collect());
            }
        }

        Ok(Vec::new())
    }

    /// Get semantic tokens for a file
    pub fn semantic_tokens(&self, file_path: &Path) -> Result<Option<SemanticTokens>> {
        let language = self.detect_language(file_path)?;

        // Ensure server is started
        if !self.servers.lock().unwrap().contains_key(&language) {
            self.start_server(&language)?;
        }

        let params = SemanticTokensParams {
            text_document: TextDocumentIdentifier {
                uri: Url::from_file_path(file_path)
                    .map_err(|_| anyhow!("Invalid file path"))?,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };

        let response = self.send_request(&language, "textDocument/semanticTokens/full", params)?;

        if let Some(result) = response.result {
            // Parse result as SemanticTokens or SemanticTokensResult
            if let Ok(tokens) = serde_json::from_value::<SemanticTokens>(result.clone()) {
                return Ok(Some(tokens));
            }
            if let Ok(tokens_result) = serde_json::from_value::<SemanticTokensResult>(result) {
                match tokens_result {
                    SemanticTokensResult::Tokens(tokens) => return Ok(Some(tokens)),
                    SemanticTokensResult::Partial(_) => return Ok(None),
                }
            }
        }

        Ok(None)
    }

    /// Format a specific range in a file
    pub fn document_range_formatting(
        &self,
        file_path: &Path,
        range: Range,
    ) -> Result<Vec<TextEdit>> {
        let language = self.detect_language(file_path)?;

        // Ensure server is started
        if !self.servers.lock().unwrap().contains_key(&language) {
            self.start_server(&language)?;
        }

        let params = DocumentRangeFormattingParams {
            text_document: TextDocumentIdentifier {
                uri: Url::from_file_path(file_path)
                    .map_err(|_| anyhow!("Invalid file path"))?,
            },
            range,
            options: FormattingOptions {
                tab_size: 4,
                insert_spaces: true,
                trim_trailing_whitespace: Some(true),
                insert_final_newline: Some(true),
                trim_final_newlines: Some(true),
                ..Default::default()
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
        };

        let response = self.send_request(&language, "textDocument/rangeFormatting", params)?;

        if let Some(result) = response.result {
            if let Ok(edits) = serde_json::from_value::<Vec<TextEdit>>(result) {
                return Ok(edits);
            }
        }

        Ok(Vec::new())
    }

    /// Search for symbols across the workspace
    pub fn workspace_symbols(&self, query: &str) -> Result<Vec<SymbolInformation>> {
        // Find servers that are currently started
        let languages: Vec<String> = self.servers
            .lock()
            .unwrap()
            .keys()
            .cloned()
            .collect();

        let mut all_symbols = Vec::new();

        // Try to find symbols using all started language servers
        for language in languages {
            let params = WorkspaceSymbolParams {
                query: query.to_string(),
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            };

            let response = self.send_request(&language, "workspace/symbol", params)?;

            if let Some(result) = response.result {
                // Try parsing as Vec<SymbolInformation>
                if let Ok(symbols) = serde_json::from_value::<Vec<SymbolInformation>>(result) {
                    all_symbols.extend(symbols);
                }
            }
        }

        Ok(all_symbols)
    }

    /// Send a JSON-RPC request to the language server
    fn send_request<P: Serialize>(
        &self,
        language: &str,
        method: &str,
        params: P,
    ) -> Result<JsonRpcResponse> {
        let mut servers = self.servers.lock().unwrap();
        let server = servers.get_mut(language)
            .ok_or_else(|| anyhow!("Language server not started for: {}", language))?;

        let id = server.next_id;
        server.next_id += 1;

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.to_string(),
            params: serde_json::to_value(params)?,
        };

        if let Some(stdin) = server.process.stdin.as_mut() {
            let request_str = serde_json::to_string(&request)?;
            let content_length = request_str.len();

            writeln!(stdin, "Content-Length: {}\r\n\r\n{}", content_length, request_str)?;
            stdin.flush()?;
        }

        // Read response (simplified - real implementation should handle async)
        if let Some(stdout) = server.process.stdout.as_mut() {
            let mut reader = BufReader::new(stdout);
            let mut headers = String::new();

            // Read headers
            loop {
                let mut line = String::new();
                reader.read_line(&mut line)?;
                if line == "\r\n" || line == "\n" {
                    break;
                }
                headers.push_str(&line);
            }

            // Parse content length
            let content_length: usize = headers
                .lines()
                .find(|l| l.starts_with("Content-Length:"))
                .and_then(|l| l.split(':').nth(1))
                .and_then(|l| l.trim().parse().ok())
                .unwrap_or(0);

            // Read content
            let mut content = vec![0u8; content_length];
            std::io::Read::read_exact(&mut reader, &mut content)?;

            let response: JsonRpcResponse = serde_json::from_slice(&content)?;
            return Ok(response);
        }

        Err(anyhow!("Failed to read response from language server"))
    }

    /// Detect the programming language from file extension
    fn detect_language(&self, file_path: &Path) -> Result<String> {
        let ext = file_path
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| anyhow!("Could not detect file extension"))?;

        let language = match ext {
            // Systems programming
            "rs" => "rust",
            "c" => "c",
            "cpp" | "cc" | "cxx" | "c++" | "hpp" | "hxx" | "h" => "cpp",
            "go" => "go",

            // Web & scripting
            "ts" => "typescript",
            "tsx" => "tsx",
            "js" | "mjs" | "cjs" => "javascript",
            "jsx" => "jsx",
            "py" | "pyw" | "pyi" => "python",
            "rb" => "ruby",
            "php" | "phtml" => "php",

            // Web frameworks
            "vue" => "vue",
            "svelte" => "svelte",
            "astro" => "astro",

            // JVM languages
            "java" => "java",
            "kt" | "kts" => "kotlin",
            "scala" | "sc" => "scala",

            // .NET
            "cs" => "csharp",

            // Functional languages
            "hs" | "lhs" => "haskell",
            "ex" | "exs" => "elixir",
            "ml" | "mli" => "ocaml",

            // Other popular languages
            "lua" => "lua",
            "swift" => "swift",
            "dart" => "dart",
            "zig" => "zig",

            // Shell & config
            "sh" | "bash" | "zsh" => "shell",

            // Markup & data languages
            "html" | "htm" => "html",
            "css" => "css",
            "scss" => "scss",
            "sass" => "sass",
            "less" => "less",
            "json" | "jsonc" => "json",
            "yaml" | "yml" => "yaml",
            "xml" => "xml",
            "toml" => "toml",
            "md" | "markdown" => "markdown",

            // SQL
            "sql" => "sql",

            _ => return Err(anyhow!("Unsupported file extension: {}", ext)),
        };

        Ok(language.to_string())
    }

    /// Get document links for a file
    pub fn document_links(&self, file_path: &Path) -> Result<Vec<DocumentLink>> {
        let language = self.detect_language(file_path)?;

        // Ensure server is started
        if !self.servers.lock().unwrap().contains_key(&language) {
            self.start_server(&language)?;
        }

        let params = DocumentLinkParams {
            text_document: TextDocumentIdentifier {
                uri: Url::from_file_path(file_path).map_err(|_| anyhow!("Invalid file path"))?,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };

        let response = self.send_request(&language, "textDocument/documentLink", params)?;

        if let Some(result) = response.result {
            Ok(serde_json::from_value(result)?)
        } else {
            Ok(Vec::new())
        }
    }

    /// Format an entire document
    pub fn document_formatting(&self, file_path: &Path) -> Result<Vec<TextEdit>> {
        // Stub implementation - returns empty list
        // TODO: Implement actual document formatting
        Ok(Vec::new())
    }

    /// Prepare call hierarchy
    pub fn prepare_call_hierarchy(&self, file_path: &Path, line: u32, character: u32) -> Result<Vec<CallHierarchyItem>> {
        // Stub implementation
        Ok(Vec::new())
    }

    /// Get incoming calls for a call hierarchy item
    pub fn call_hierarchy_incoming_calls(&self, _item: CallHierarchyItem) -> Result<Vec<CallHierarchyIncomingCall>> {
        // Stub implementation
        Ok(Vec::new())
    }

    /// Get outgoing calls for a call hierarchy item
    pub fn call_hierarchy_outgoing_calls(&self, _item: CallHierarchyItem) -> Result<Vec<CallHierarchyOutgoingCall>> {
        // Stub implementation
        Ok(Vec::new())
    }

    /// Prepare type hierarchy
    pub fn prepare_type_hierarchy(&self, file_path: &Path, line: u32, character: u32) -> Result<Vec<TypeHierarchyItem>> {
        // Stub implementation
        Ok(Vec::new())
    }

    /// Get supertypes for a type hierarchy item
    pub fn type_hierarchy_supertypes(&self, _item: TypeHierarchyItem) -> Result<Vec<TypeHierarchyItem>> {
        // Stub implementation
        Ok(Vec::new())
    }

    /// Get subtypes for a type hierarchy item
    pub fn type_hierarchy_subtypes(&self, _item: TypeHierarchyItem) -> Result<Vec<TypeHierarchyItem>> {
        // Stub implementation
        Ok(Vec::new())
    }

    /// Shutdown all language servers
    pub fn shutdown(&mut self) -> Result<()> {
        let mut servers = self.servers.lock().unwrap();

        for (language, mut server) in servers.drain() {
            // Send shutdown request
            let request = JsonRpcRequest {
                jsonrpc: "2.0".to_string(),
                id: server.next_id,
                method: "shutdown".to_string(),
                params: serde_json::Value::Null,
            };

            if let Some(stdin) = server.process.stdin.as_mut() {
                let request_str = serde_json::to_string(&request).unwrap();
                let content_length = request_str.len();

                let _ = writeln!(stdin, "Content-Length: {}\r\n\r\n{}", content_length, request_str);
                let _ = stdin.flush();
            }

            // Kill process
            let _ = server.process.kill();
            tracing::info!("Stopped {} language server", language);
        }

        Ok(())
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
    fn test_lsp_client_creation() {
        let client = LspClient::new(PathBuf::from("/tmp"));
        assert_eq!(client.project_root, PathBuf::from("/tmp"));
    }

    #[test]
    fn test_detect_language() {
        let client = LspClient::new(PathBuf::from("/tmp"));

        // Systems programming
        assert_eq!(client.detect_language(Path::new("test.rs")).unwrap(), "rust");
        assert_eq!(client.detect_language(Path::new("test.c")).unwrap(), "c");
        assert_eq!(client.detect_language(Path::new("test.cpp")).unwrap(), "cpp");
        assert_eq!(client.detect_language(Path::new("test.go")).unwrap(), "go");

        // Web & scripting
        assert_eq!(client.detect_language(Path::new("test.ts")).unwrap(), "typescript");
        assert_eq!(client.detect_language(Path::new("test.tsx")).unwrap(), "tsx");
        assert_eq!(client.detect_language(Path::new("test.js")).unwrap(), "javascript");
        assert_eq!(client.detect_language(Path::new("test.jsx")).unwrap(), "jsx");
        assert_eq!(client.detect_language(Path::new("test.py")).unwrap(), "python");
        assert_eq!(client.detect_language(Path::new("test.rb")).unwrap(), "ruby");
        assert_eq!(client.detect_language(Path::new("test.php")).unwrap(), "php");

        // Web frameworks
        assert_eq!(client.detect_language(Path::new("test.vue")).unwrap(), "vue");
        assert_eq!(client.detect_language(Path::new("test.svelte")).unwrap(), "svelte");
        assert_eq!(client.detect_language(Path::new("test.astro")).unwrap(), "astro");

        // JVM languages
        assert_eq!(client.detect_language(Path::new("test.java")).unwrap(), "java");
        assert_eq!(client.detect_language(Path::new("test.kt")).unwrap(), "kotlin");
        assert_eq!(client.detect_language(Path::new("test.scala")).unwrap(), "scala");

        // .NET
        assert_eq!(client.detect_language(Path::new("test.cs")).unwrap(), "csharp");

        // Shell
        assert_eq!(client.detect_language(Path::new("test.sh")).unwrap(), "shell");
        assert_eq!(client.detect_language(Path::new("test.bash")).unwrap(), "shell");
        assert_eq!(client.detect_language(Path::new("test.zsh")).unwrap(), "shell");

        // Markup & data
        assert_eq!(client.detect_language(Path::new("test.html")).unwrap(), "html");
        assert_eq!(client.detect_language(Path::new("test.css")).unwrap(), "css");
        assert_eq!(client.detect_language(Path::new("test.json")).unwrap(), "json");
        assert_eq!(client.detect_language(Path::new("test.yaml")).unwrap(), "yaml");
        assert_eq!(client.detect_language(Path::new("test.toml")).unwrap(), "toml");
        assert_eq!(client.detect_language(Path::new("test.md")).unwrap(), "markdown");
    }
}
