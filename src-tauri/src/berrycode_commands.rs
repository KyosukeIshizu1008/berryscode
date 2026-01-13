//! Tauri commands for BerryCode CLI integration
//!
//! This module exposes BerryCode CLI functionality to the Tauri frontend.
//! Uses gRPC client to communicate with berry-api-server on port 50051.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{Emitter, State};
use crate::app_database::{AppDatabase, ChatSessionData, ChatMessageData};

// Include generated gRPC client code (only for native/Tauri, not WASM)
#[cfg(not(target_arch = "wasm32"))]
pub mod berry_api {
    tonic::include_proto!("berry_api");
}

#[cfg(not(target_arch = "wasm32"))]
use berry_api::berry_code_cli_service_client::BerryCodeCliServiceClient;
#[cfg(not(target_arch = "wasm32"))]
use berry_api::berry_code_service_client::BerryCodeServiceClient;
#[cfg(not(target_arch = "wasm32"))]
use tonic::transport::Channel;

/// Global BerryCode session state
pub struct BerryCodeState {
    /// Project root directory
    pub project_root: Mutex<Option<PathBuf>>,

    /// gRPC API session ID
    pub session_id: Mutex<Option<String>>,

    /// Skip all permission prompts (DANGEROUS!)
    /// When true, all file writes, deletions, and git operations
    /// are executed immediately without user confirmation.
    pub dangerously_skip_permissions: bool,
}

impl Default for BerryCodeState {
    fn default() -> Self {
        Self {
            project_root: Mutex::new(None),
            session_id: Mutex::new(None),
            dangerously_skip_permissions: false, // Safe by default
        }
    }
}

impl BerryCodeState {
    /// Create a new BerryCodeState with the given configuration
    pub fn new(project_root: Option<PathBuf>, dangerously_skip_permissions: bool) -> Self {
        Self {
            project_root: Mutex::new(project_root),
            session_id: Mutex::new(None),
            dangerously_skip_permissions,
        }
    }

    /// Check if operations should skip permission prompts
    pub fn should_skip_permissions(&self) -> bool {
        self.dangerously_skip_permissions
    }
}

/// Comprehensive project context for AI
/// This provides all necessary information for the AI to understand
/// the current state of the project and make informed decisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContext {
    /// Project root directory (absolute path)
    pub project_root: String,

    /// List of all relevant files in the project
    pub files: Vec<String>,

    /// Current git status (branch, uncommitted changes, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_status: Option<GitStatus>,

    /// LSP diagnostics (errors, warnings from language servers)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<Vec<DiagnosticInfo>>,

    /// Symbol index (functions, structs, etc. from LSP)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbols: Option<Vec<SymbolInfo>>,

    /// Recently modified files (last 10)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recent_files: Option<Vec<String>>,

    /// Number of files by language
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_stats: Option<FileStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatus {
    pub branch: String,
    pub uncommitted_changes: usize,
    pub untracked_files: usize,
    pub ahead: usize,
    pub behind: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticInfo {
    pub file_path: String,
    pub line: u32,
    pub column: u32,
    pub severity: String, // "error", "warning", "info", "hint"
    pub message: String,
    pub source: String, // e.g., "rust-analyzer", "typescript"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: String, // "function", "struct", "enum", "trait", etc.
    pub file_path: String,
    pub line: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileStats {
    pub total: usize,
    pub rust: usize,
    pub javascript: usize,
    pub typescript: usize,
    pub python: usize,
    pub other: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String, // "user" or "assistant"
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub provider: String,
    pub description: String,
}

/// Initialize a new BerryCode session via gRPC (Native only)
#[cfg(not(target_arch = "wasm32"))]
#[tauri::command]
pub async fn berrycode_init(
    model: Option<String>,
    mode: Option<String>,
    project_root: Option<String>,
    autonomous: Option<bool>,
    state: State<'_, BerryCodeState>,
    app_db: State<'_, Arc<AppDatabase>>,
) -> Result<String, String> {
    eprintln!("[BerryCode Init] Starting gRPC initialization...");

    // Determine project root
    let root_path_buf = if let Some(root) = &project_root {
        let path = PathBuf::from(root);
        if path.exists() {
            *state.project_root.lock()
                .map_err(|e| format!("Failed to lock project_root: {}", e))? = Some(path.clone());
            eprintln!("[BerryCode Init] Project root set to: {}", root);
            path
        } else {
            eprintln!("[BerryCode Init] Warning: Project root does not exist: {}", root);
            PathBuf::from(".")
        }
    } else {
        PathBuf::from(".")
    };

    let root_path = root_path_buf.to_string_lossy().to_string();

    // Connect to gRPC server (use localhost which resolves to both IPv4 and IPv6)
    let channel = Channel::from_static("http://localhost:50051")
        .connect()
        .await
        .map_err(|e| format!("Failed to connect to BerryCode gRPC server: {}", e))?;

    let mut client = BerryCodeServiceClient::new(channel);

    // Prepare request (use StartSession for BerryCodeService instead of InitializeCLI)
    let request = tonic::Request::new(berry_api::StartSessionRequest {
        model: model.clone(),
        mode: mode.clone(),
        files: vec![], // Empty files list initially
        project_path: Some(root_path.clone()),
        git_enabled: Some(true),
        api_key: None, // Will use environment variable
        api_base: None, // Will use default
        autonomous: autonomous, // Pass through the autonomous mode flag
    });

    eprintln!("[BerryCode Init] Calling StartSession gRPC method...");

    // Call StartSession (for BerryCodeService, not CLI)
    let response = client.start_session(request)
        .await
        .map_err(|e| format!("gRPC StartSession failed: {}", e))?
        .into_inner();

    eprintln!("[BerryCode Init] Session started: {}", response.session_id);

    // Store session ID in memory
    *state.session_id.lock()
        .map_err(|e| format!("Failed to lock session_id: {}", e))? = Some(response.session_id.clone());

    eprintln!("[BerryCode Init] ✅ Session ID stored in state: {}", response.session_id);

    // Store session in database (for foreign key constraint)
    app_db.create_session(&response.session_id, &root_path_buf)
        .map_err(|e| format!("Failed to save session to database: {}", e))?;

    eprintln!("[BerryCode Init] ✅ Session initialized: {}", response.session_id);
    Ok(format!("BerryCode session initialized: {}", response.session_id))
}

/// Initialize a new BerryCode session (WASM stub - not supported)
#[cfg(target_arch = "wasm32")]
#[tauri::command]
pub async fn berrycode_init(
    _model: Option<String>,
    _mode: Option<String>,
    _project_root: Option<String>,
    _autonomous: Option<bool>,
    _state: State<'_, BerryCodeState>,
    _app_db: State<'_, Arc<AppDatabase>>,
) -> Result<String, String> {
    Err("BerryCode AI chat is only available in the native desktop app, not in web mode.".to_string())
}

/// Send a chat message to the AI via gRPC streaming (Native only)
/// Streams chunks in real-time via Tauri events
#[cfg(not(target_arch = "wasm32"))]
#[tauri::command]
pub async fn berrycode_chat(
    message: String,
    state: State<'_, BerryCodeState>,
    window: tauri::Window,
) -> Result<(), String> {
    use futures_util::StreamExt;

    eprintln!("[BerryCode Chat] Sending message: {}", message);

    // Get session ID
    let session_id = {
        let locked = state.session_id.lock()
            .map_err(|e| format!("Failed to lock session_id: {}", e))?;
        eprintln!("[BerryCode Chat] Locked state, session_id = {:?}", *locked);
        locked.clone()
            .ok_or_else(|| {
                eprintln!("[BerryCode Chat] ❌ No session ID in state!");
                "No active BerryCode session. Call berrycode_init first.".to_string()
            })?
    };

    eprintln!("[BerryCode Chat] ✅ Using session: {}", session_id);

    // Connect to gRPC server
    let channel = Channel::from_static("http://localhost:50051")
        .connect()
        .await
        .map_err(|e| format!("Failed to connect to BerryCode gRPC server: {}", e))?;

    let mut client = BerryCodeServiceClient::new(channel);

    // Prepare ChatRequest
    let request = tonic::Request::new(berry_api::ChatRequest {
        session_id,
        message: message.clone(),
        stream: Some(true),
    });

    eprintln!("[BerryCode Chat] Calling Chat gRPC method...");

    // Call Chat (streaming response)
    let mut stream = client.chat(request)
        .await
        .map_err(|e| format!("gRPC Chat failed: {}", e))?
        .into_inner();

    // Stream chunks in real-time via Tauri events
    while let Some(result) = stream.next().await {
        match result {
            Ok(chat_chunk) => {
                if !chat_chunk.content.is_empty() {
                    eprintln!("[BerryCode Chat] Stream chunk: {}", chat_chunk.content);

                    // Emit chunk to frontend
                    window.emit("berrycode-stream-chunk", &chat_chunk.content)
                        .map_err(|e| format!("Failed to emit event: {}", e))?;
                }

                if chat_chunk.is_final {
                    eprintln!("[BerryCode Chat] ✅ Stream completed");

                    // Emit completion event
                    window.emit("berrycode-stream-end", ())
                        .map_err(|e| format!("Failed to emit end event: {}", e))?;
                    break;
                }
            }
            Err(e) => {
                eprintln!("[BerryCode Chat] ❌ Stream error: {}", e);

                // Emit error event
                window.emit("berrycode-stream-error", format!("Stream error: {}", e))
                    .map_err(|e2| format!("Failed to emit error event: {}", e2))?;

                return Err(format!("Stream error: {}", e));
            }
        }
    }

    Ok(())
}

/// Send a chat message to the AI (WASM stub - not supported)
#[cfg(target_arch = "wasm32")]
#[tauri::command]
pub async fn berrycode_chat(
    _message: String,
    _state: State<'_, BerryCodeState>,
) -> Result<String, String> {
    Err("BerryCode AI chat is only available in the native desktop app, not in web mode.".to_string())
}

/// Add a file to the chat context
#[tauri::command]
pub async fn berrycode_add_file(
    file_path: String,
    _state: State<'_, BerryCodeState>,
) -> Result<String, String> {
    let path = PathBuf::from(&file_path);
    if !path.exists() {
        return Err(format!("File not found: {}", file_path));
    }
    Ok(format!("Added {} to context (placeholder)", file_path))
}

/// Remove a file from the chat context
#[tauri::command]
pub async fn berrycode_drop_file(
    file_path: String,
    _state: State<'_, BerryCodeState>,
) -> Result<String, String> {
    Ok(format!("Removed {} from context (placeholder)", file_path))
}

/// List all files in the current context
#[tauri::command]
pub async fn berrycode_list_files(
    _state: State<'_, BerryCodeState>,
) -> Result<Vec<String>, String> {
    // Get current working directory (project root)
    let current_dir = std::env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?;

    eprintln!("[BerryCode] Listing files in: {:?}", current_dir);

    let mut files = Vec::new();

    // Excluded directories (matching .gitignore patterns)
    let exclude_dirs = vec![
        "target",       // Rust build artifacts
        "dist",         // Build output
        "node_modules", // Node.js dependencies
        ".git",         // Git metadata
        ".next",        // Next.js cache
        ".vscode",      // IDE settings
        ".idea",        // JetBrains IDE
        "build",        // Generic build dir
        "tmp",          // Temporary files
        "temp",         // Temporary files
        ".cache",       // Cache directory
        "data",         // Data directory (often large)
        "static",       // Static assets
    ];

    // File extensions to include (code files only)
    let include_extensions = vec![
        "rs", "toml", "md", "txt",                           // Rust/Docs
        "js", "ts", "jsx", "tsx", "mjs", "cjs",              // JavaScript/TypeScript
        "py", "pyx", "pyi",                                  // Python
        "go", "mod", "sum",                                  // Go
        "java", "kt", "scala",                               // JVM languages
        "cpp", "c", "h", "hpp", "cc", "cxx",                 // C/C++
        "html", "css", "scss", "sass", "less",               // Web
        "json", "yaml", "yml", "xml", "toml", "ini", "conf", // Config
        "sh", "bash", "zsh", "fish",                         // Shell scripts
        "sql", "graphql", "proto",                           // Data/API
        "vue", "svelte", "astro",                            // Modern frameworks
    ];

    match visit_dirs(&current_dir, &current_dir, &mut files, &exclude_dirs, &include_extensions) {
        Ok(_) => {
            eprintln!("[BerryCode] Found {} files", files.len());
            Ok(files)
        }
        Err(e) => Err(format!("Failed to list files: {}", e)),
    }
}

/// Recursively visit directories and collect file paths
fn visit_dirs(
    dir: &PathBuf,
    project_root: &PathBuf,
    files: &mut Vec<String>,
    exclude_dirs: &[&str],
    include_extensions: &[&str],
) -> Result<(), String> {
    if !dir.is_dir() {
        return Ok(());
    }

    let entries = fs::read_dir(dir).map_err(|e| format!("Failed to read dir {:?}: {}", dir, e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path();

        // Get file/dir name
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        // Skip hidden files and directories (starting with .)
        if name.starts_with('.') {
            continue;
        }

        if path.is_dir() {
            // Skip excluded directories
            if exclude_dirs.contains(&name) {
                continue;
            }

            // Recursively visit subdirectory
            visit_dirs(&path, project_root, files, exclude_dirs, include_extensions)?;
        } else {
            // Check file extension
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                if include_extensions.contains(&ext) {
                    // Convert to relative path from project root
                    let relative_path = path
                        .strip_prefix(project_root)
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .to_string();

                    files.push(relative_path);
                }
            }
        }
    }

    Ok(())
}

/// Create a new chat session
#[tauri::command]
pub async fn berrycode_create_chat_session(
    title: Option<String>,
    state: State<'_, BerryCodeState>,
    app_db: State<'_, Arc<AppDatabase>>,
) -> Result<String, String> {
    // Get BerryCode session ID
    let session_id = state.session_id.lock()
        .map_err(|e| format!("Failed to lock session_id: {}", e))?
        .clone()
        .ok_or_else(|| "No active BerryCode session. Call berrycode_init first.".to_string())?;

    // Generate unique chat session ID
    let chat_id = uuid::Uuid::new_v4().to_string();

    // Create chat session in database
    app_db.create_chat_session(&chat_id, &session_id, title.as_deref())
        .map_err(|e| format!("Failed to create chat session: {}", e))?;

    eprintln!("[BerryCode] ✅ Created chat session: {}", chat_id);
    Ok(chat_id)
}

/// List all chat sessions for current project
#[tauri::command]
pub async fn berrycode_list_chat_sessions(
    state: State<'_, BerryCodeState>,
    app_db: State<'_, Arc<AppDatabase>>,
) -> Result<Vec<ChatSessionData>, String> {
    // Get BerryCode session ID
    let session_id = state.session_id.lock()
        .map_err(|e| format!("Failed to lock session_id: {}", e))?
        .clone()
        .ok_or_else(|| "No active BerryCode session.".to_string())?;

    // List chat sessions from database
    let sessions = app_db.list_chat_sessions(&session_id)
        .map_err(|e| format!("Failed to list chat sessions: {}", e))?;

    eprintln!("[BerryCode] Found {} chat sessions", sessions.len());
    Ok(sessions)
}

/// Load messages for a specific chat session
#[tauri::command]
pub async fn berrycode_load_chat_messages(
    chat_session_id: String,
    app_db: State<'_, Arc<AppDatabase>>,
) -> Result<Vec<ChatMessage>, String> {
    // Load messages from database
    let messages = app_db.load_chat_messages(&chat_session_id)
        .map_err(|e| format!("Failed to load chat messages: {}", e))?;

    // Convert ChatMessageData to ChatMessage
    let chat_messages: Vec<ChatMessage> = messages.into_iter().map(|m| ChatMessage {
        role: m.role,
        content: m.content,
    }).collect();

    eprintln!("[BerryCode] Loaded {} messages for session {}", chat_messages.len(), chat_session_id);
    Ok(chat_messages)
}

/// Save a message (called after sending user message or receiving AI response)
#[tauri::command]
pub async fn berrycode_save_message(
    chat_session_id: String,
    role: String,
    content: String,
    app_db: State<'_, Arc<AppDatabase>>,
) -> Result<i64, String> {
    // Save message to database
    let message_id = app_db.save_chat_message(&chat_session_id, &role, &content)
        .map_err(|e| format!("Failed to save message: {}", e))?;

    eprintln!("[BerryCode] Saved {} message (ID: {})", role, message_id);
    Ok(message_id)
}

/// Delete a chat session
#[tauri::command]
pub async fn berrycode_delete_chat_session(
    chat_session_id: String,
    app_db: State<'_, Arc<AppDatabase>>,
) -> Result<(), String> {
    // Delete chat session from database (cascade deletes messages)
    app_db.delete_chat_session(&chat_session_id)
        .map_err(|e| format!("Failed to delete chat session: {}", e))?;

    eprintln!("[BerryCode] Deleted chat session: {}", chat_session_id);
    Ok(())
}

/// Update chat session title
#[tauri::command]
pub async fn berrycode_update_chat_title(
    chat_session_id: String,
    title: String,
    app_db: State<'_, Arc<AppDatabase>>,
) -> Result<(), String> {
    // Update title in database
    app_db.update_chat_session(&chat_session_id, Some(&title))
        .map_err(|e| format!("Failed to update chat title: {}", e))?;

    eprintln!("[BerryCode] Updated chat session title: {}", title);
    Ok(())
}

/// Get chat history (returns messages from most recent chat session)
#[tauri::command]
pub async fn berrycode_get_history(
    state: State<'_, BerryCodeState>,
    app_db: State<'_, Arc<AppDatabase>>,
) -> Result<Vec<ChatMessage>, String> {
    // Get BerryCode session ID
    let session_id = state.session_id.lock()
        .map_err(|e| format!("Failed to lock session_id: {}", e))?
        .clone()
        .ok_or_else(|| "No active BerryCode session.".to_string())?;

    // Get most recent chat session
    let sessions = app_db.list_chat_sessions(&session_id)
        .map_err(|e| format!("Failed to list chat sessions: {}", e))?;

    if let Some(latest_session) = sessions.first() {
        // Load messages from most recent session
        let messages = app_db.load_chat_messages(&latest_session.id)
            .map_err(|e| format!("Failed to load messages: {}", e))?;

        let chat_messages: Vec<ChatMessage> = messages.into_iter().map(|m| ChatMessage {
            role: m.role,
            content: m.content,
        }).collect();

        Ok(chat_messages)
    } else {
        Ok(vec![])
    }
}

/// Clear chat history (deletes all chat sessions for current project)
#[tauri::command]
pub async fn berrycode_clear_history(
    state: State<'_, BerryCodeState>,
    app_db: State<'_, Arc<AppDatabase>>,
) -> Result<String, String> {
    // Get BerryCode session ID
    let session_id = state.session_id.lock()
        .map_err(|e| format!("Failed to lock session_id: {}", e))?
        .clone()
        .ok_or_else(|| "No active BerryCode session.".to_string())?;

    // Get all chat sessions
    let sessions = app_db.list_chat_sessions(&session_id)
        .map_err(|e| format!("Failed to list chat sessions: {}", e))?;

    // Delete each session
    for session in &sessions {
        app_db.delete_chat_session(&session.id)
            .map_err(|e| format!("Failed to delete session {}: {}", session.id, e))?;
    }

    eprintln!("[BerryCode] Cleared {} chat sessions", sessions.len());
    Ok(format!("Cleared {} chat sessions", sessions.len()))
}

/// List available models
#[tauri::command]
pub async fn berrycode_list_models() -> Result<Vec<ModelInfo>, String> {
    let models = vec![
        ModelInfo {
            name: "gpt-4".to_string(),
            provider: "OpenAI".to_string(),
            description: "Most capable GPT-4 model".to_string(),
        },
        ModelInfo {
            name: "gpt-4-turbo".to_string(),
            provider: "OpenAI".to_string(),
            description: "Faster GPT-4 with 128k context".to_string(),
        },
        ModelInfo {
            name: "gpt-3.5-turbo".to_string(),
            provider: "OpenAI".to_string(),
            description: "Fast and cost-effective".to_string(),
        },
        ModelInfo {
            name: "claude-3-opus".to_string(),
            provider: "Anthropic".to_string(),
            description: "Most capable Claude model".to_string(),
        },
        ModelInfo {
            name: "claude-3-sonnet".to_string(),
            provider: "Anthropic".to_string(),
            description: "Balanced performance".to_string(),
        },
        ModelInfo {
            name: "claude-3-haiku".to_string(),
            provider: "Anthropic".to_string(),
            description: "Fast and lightweight".to_string(),
        },
        ModelInfo {
            name: "deepseek-chat".to_string(),
            provider: "DeepSeek".to_string(),
            description: "General purpose chat model".to_string(),
        },
        ModelInfo {
            name: "deepseek-coder".to_string(),
            provider: "DeepSeek".to_string(),
            description: "Specialized for coding".to_string(),
        },
    ];

    Ok(models)
}

/// Test result from cargo check/test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub success: bool,
    pub output: String,
    pub errors: Vec<CompilationError>,
    pub warnings: Vec<CompilationWarning>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilationError {
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub message: String,
    pub code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilationWarning {
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub message: String,
}

/// Run cargo check and return structured results
#[tauri::command]
pub async fn berrycode_cargo_check(
    _state: State<'_, BerryCodeState>,
) -> Result<TestResult, String> {
    use std::process::Command;
    use std::time::Instant;

    eprintln!("[BerryCode] 🔍 Running cargo check...");

    let start = Instant::now();

    let output = Command::new("cargo")
        .arg("check")
        .arg("--message-format=json")
        .current_dir(std::env::current_dir().map_err(|e| e.to_string())?)
        .output()
        .map_err(|e| format!("Failed to run cargo check: {}", e))?;

    let duration_ms = start.elapsed().as_millis() as u64;

    let success = output.status.success();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    // Parse JSON output from cargo
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    for line in stdout.lines() {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
            if json["reason"] == "compiler-message" {
                if let Some(message) = json["message"].as_object() {
                    let level = message["level"].as_str().unwrap_or("");
                    let msg_text = message["message"].as_str().unwrap_or("").to_string();

                    // Extract file location
                    if let Some(spans) = message["spans"].as_array() {
                        if let Some(span) = spans.first() {
                            let file = span["file_name"].as_str().unwrap_or("").to_string();
                            let line = span["line_start"].as_u64().unwrap_or(0) as u32;
                            let column = span["column_start"].as_u64().unwrap_or(0) as u32;
                            let code = message["code"]
                                .as_object()
                                .and_then(|c| c["code"].as_str())
                                .map(|s| s.to_string());

                            match level {
                                "error" => errors.push(CompilationError {
                                    file: file.clone(),
                                    line,
                                    column,
                                    message: msg_text.clone(),
                                    code,
                                }),
                                "warning" => warnings.push(CompilationWarning {
                                    file: file.clone(),
                                    line,
                                    column,
                                    message: msg_text.clone(),
                                }),
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }

    let summary = if success {
        format!("✅ cargo check passed ({} warnings)", warnings.len())
    } else {
        format!("❌ cargo check failed ({} errors, {} warnings)", errors.len(), warnings.len())
    };

    eprintln!("[BerryCode] {}", summary);

    Ok(TestResult {
        success,
        output: stdout,
        errors,
        warnings,
        duration_ms,
    })
}

/// Run cargo test and return results
#[tauri::command]
pub async fn berrycode_cargo_test(
    _state: State<'_, BerryCodeState>,
) -> Result<TestResult, String> {
    use std::process::Command;
    use std::time::Instant;

    eprintln!("[BerryCode] 🧪 Running cargo test...");

    let start = Instant::now();

    let output = Command::new("cargo")
        .arg("test")
        .arg("--")
        .arg("--nocapture")
        .current_dir(std::env::current_dir().map_err(|e| e.to_string())?)
        .output()
        .map_err(|e| format!("Failed to run cargo test: {}", e))?;

    let duration_ms = start.elapsed().as_millis() as u64;

    let success = output.status.success();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let combined_output = format!("{}\n{}", stdout, stderr);

    let summary = if success {
        "✅ cargo test passed"
    } else {
        "❌ cargo test failed"
    };

    eprintln!("[BerryCode] {} ({}ms)", summary, duration_ms);

    Ok(TestResult {
        success,
        output: combined_output,
        errors: Vec::new(),  // Test failures are in output text
        warnings: Vec::new(),
        duration_ms,
    })
}

/// Get comprehensive project context for AI
/// This is the foundation for RAG (Retrieval-Augmented Generation)
#[tauri::command]
pub async fn berrycode_get_context(
    state: State<'_, BerryCodeState>,
) -> Result<ProjectContext, String> {
    eprintln!("[BerryCode] 📊 Building project context...");

    // Get current directory as project root
    let project_root = std::env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?
        .to_string_lossy()
        .to_string();

    eprintln!("[BerryCode] Project root: {}", project_root);

    // Get file list
    let files = berrycode_list_files(state.clone()).await?;
    eprintln!("[BerryCode] Found {} files", files.len());

    // Calculate file statistics
    let file_stats = calculate_file_stats(&files);

    // Get git status (if in a git repository)
    let git_status = get_git_status().await.ok();

    // TODO: Get LSP diagnostics (Phase 2)
    let diagnostics = None;

    // TODO: Get symbol index (Phase 2)
    let symbols = None;

    // Get recently modified files (last 10)
    let recent_files = get_recent_files(&files).await.ok();

    let context = ProjectContext {
        project_root: project_root.clone(),
        files,
        git_status,
        diagnostics,
        symbols,
        recent_files,
        file_stats: Some(file_stats),
    };

    eprintln!(
        "[BerryCode] ✅ Context built: {} files, git={}, stats={:?}",
        context.files.len(),
        context.git_status.is_some(),
        context.file_stats
    );

    Ok(context)
}

/// Calculate file statistics by extension
fn calculate_file_stats(files: &[String]) -> FileStats {
    let mut stats = FileStats {
        total: files.len(),
        rust: 0,
        javascript: 0,
        typescript: 0,
        python: 0,
        other: 0,
    };

    for file in files {
        if file.ends_with(".rs") {
            stats.rust += 1;
        } else if file.ends_with(".js") || file.ends_with(".jsx") || file.ends_with(".mjs") {
            stats.javascript += 1;
        } else if file.ends_with(".ts") || file.ends_with(".tsx") {
            stats.typescript += 1;
        } else if file.ends_with(".py") {
            stats.python += 1;
        } else {
            stats.other += 1;
        }
    }

    stats
}

/// Get git status for the current repository
async fn get_git_status() -> Result<GitStatus, String> {
    use std::process::Command;

    // Check if we're in a git repository
    let is_git_repo = Command::new("git")
        .args(&["rev-parse", "--is-inside-work-tree"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !is_git_repo {
        return Err("Not in a git repository".to_string());
    }

    // Get current branch
    let branch_output = Command::new("git")
        .args(&["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .map_err(|e| format!("Failed to get git branch: {}", e))?;

    let branch = String::from_utf8_lossy(&branch_output.stdout)
        .trim()
        .to_string();

    // Count uncommitted changes
    let status_output = Command::new("git")
        .args(&["status", "--porcelain"])
        .output()
        .map_err(|e| format!("Failed to get git status: {}", e))?;

    let status_lines = String::from_utf8_lossy(&status_output.stdout);
    let uncommitted_changes = status_lines.lines().count();
    let untracked_files = status_lines
        .lines()
        .filter(|l| l.starts_with("??"))
        .count();

    // Get ahead/behind counts (if tracking remote)
    let (ahead, behind) = get_ahead_behind().await.unwrap_or((0, 0));

    Ok(GitStatus {
        branch,
        uncommitted_changes,
        untracked_files,
        ahead,
        behind,
    })
}

/// Get ahead/behind counts relative to remote
async fn get_ahead_behind() -> Result<(usize, usize), String> {
    use std::process::Command;

    let output = Command::new("git")
        .args(&["rev-list", "--left-right", "--count", "HEAD...@{upstream}"])
        .output()
        .map_err(|e| format!("Failed to get ahead/behind: {}", e))?;

    if !output.status.success() {
        return Ok((0, 0)); // No upstream tracking
    }

    let result = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = result.trim().split('\t').collect();

    let ahead = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
    let behind = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);

    Ok((ahead, behind))
}

/// Get recently modified files (last 10)
async fn get_recent_files(all_files: &[String]) -> Result<Vec<String>, String> {
    use std::fs;
    use std::time::SystemTime;

    let mut file_times: Vec<(String, SystemTime)> = Vec::new();

    // Get current directory as base
    let current_dir = std::env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?;

    for file in all_files {
        let full_path = current_dir.join(file);
        if let Ok(metadata) = fs::metadata(&full_path) {
            if let Ok(modified) = metadata.modified() {
                file_times.push((file.clone(), modified));
            }
        }
    }

    // Sort by modification time (most recent first)
    file_times.sort_by(|a, b| b.1.cmp(&a.1));

    // Take top 10
    let recent: Vec<String> = file_times.into_iter().take(10).map(|(f, _)| f).collect();

    Ok(recent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_berrycode_list_files_returns_vec() {
        let state = BerryCodeState::default();
        let result = berrycode_list_files(State::from(&state)).await;

        // Should return Ok with a Vec (may be empty or filled depending on cwd)
        assert!(result.is_ok());
        let files = result.unwrap();
        println!("Found {} files in current directory", files.len());

        // If we're in the project directory, we should find at least Cargo.toml
        if files.iter().any(|f| f.contains("Cargo.toml")) {
            println!("✅ Found Cargo.toml - we're in the project directory");
        }
    }

    #[test]
    fn test_visit_dirs_excludes_hidden_files() {
        use std::path::PathBuf;

        let mut files = Vec::new();
        let exclude_dirs = vec!["target", "node_modules"];
        let include_extensions = vec!["rs", "toml"];

        // Test that hidden directories are skipped
        // This is a conceptual test - actual filesystem needed for real test
        let test_path = PathBuf::from(".");

        // If current directory exists and is readable
        if test_path.exists() && test_path.is_dir() {
            let result = visit_dirs(
                &test_path,
                &test_path,
                &mut files,
                &exclude_dirs,
                &include_extensions,
            );

            assert!(result.is_ok());

            // Verify no hidden files in results
            for file in &files {
                let path = PathBuf::from(file);
                if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                    assert!(
                        !name.starts_with('.'),
                        "Found hidden file: {}",
                        file
                    );
                }
            }

            println!("✅ No hidden files found (tested {} files)", files.len());
        }
    }

    #[test]
    fn test_visit_dirs_filters_by_extension() {
        use std::path::PathBuf;

        let mut files = Vec::new();
        let exclude_dirs = vec!["target", "node_modules", ".git"];
        let include_extensions = vec!["rs"]; // Only Rust files

        let test_path = PathBuf::from(".");

        if test_path.exists() && test_path.is_dir() {
            let result = visit_dirs(
                &test_path,
                &test_path,
                &mut files,
                &exclude_dirs,
                &include_extensions,
            );

            assert!(result.is_ok());

            // All returned files should be .rs files
            for file in &files {
                assert!(
                    file.ends_with(".rs"),
                    "Non-.rs file found: {}",
                    file
                );
            }

            println!("✅ All {} files have .rs extension", files.len());
        }
    }

    #[test]
    fn test_berrycode_state_default() {
        let state = BerryCodeState::default();
        assert!(!state.should_skip_permissions());
    }

    #[test]
    fn test_berrycode_state_with_skip_permissions() {
        let state = BerryCodeState::new(None, true);
        assert!(state.should_skip_permissions());

        let safe_state = BerryCodeState::new(None, false);
        assert!(!safe_state.should_skip_permissions());
    }

    #[tokio::test]
    async fn test_execute_command_blocks_dangerous_operations() {
        let state = BerryCodeState::default(); // skip_permissions = false

        // Dangerous commands should be blocked
        let result = berrycode_execute_command(
            "/delete some_file".to_string(),
            State::from(&state),
        ).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Permission denied"));
    }

    #[tokio::test]
    async fn test_execute_command_allows_with_flag() {
        let state = BerryCodeState::new(None, true); // skip_permissions = true

        // Dangerous commands should be allowed with flag
        let result = berrycode_execute_command(
            "/delete some_file".to_string(),
            State::from(&state),
        ).await;

        assert!(result.is_ok());
    }
}
