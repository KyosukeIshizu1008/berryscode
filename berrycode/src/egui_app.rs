//! egui-based main application structure
//! Replaces Dioxus components with egui immediate-mode UI

use crate::buffer::TextBuffer;
use crate::focus_stack::FocusLayer;
use crate::native;
use crate::native::fs::DirEntry;
use crate::syntax::{SyntaxHighlighter, TokenType}; // Regex-based highlighting with One Dark colors
use std::collections::HashSet;
use tokio::sync::mpsc;

// ===== Syntax Highlighting Color Palette =====
// VS Code Dark+ inspired color scheme for Rust syntax highlighting

mod syntax_colors {
    use egui::Color32;

    // Keywords: fn, let, mut, const, if, else, match, etc.
    pub const KEYWORD: Color32 = Color32::from_rgb(234, 147, 71);      // #EA9347 Orange

    // Function names: main, println, calculate, etc.
    pub const FUNCTION: Color32 = Color32::from_rgb(84, 166, 224);     // #54A6E0 Sky Blue

    // Types: u32, String, Vec, Result, etc.
    pub const TYPE: Color32 = Color32::from_rgb(232, 194, 82);         // #E8C252 Yellow

    // String literals: "hello", r#"raw"#
    pub const STRING: Color32 = Color32::from_rgb(184, 214, 84);       // #B8D654 Lime Green

    // Number literals: 42, 3.14, 0xFF
    pub const NUMBER: Color32 = Color32::from_rgb(181, 206, 168);      // #B5CEA8 Light Green

    // Normal comments: //
    pub const COMMENT: Color32 = Color32::from_rgb(128, 128, 128);     // #808080 Gray

    // Doc comments: //!, ///
    pub const DOC_COMMENT: Color32 = Color32::from_rgb(106, 153, 85);  // #6A9955 Green

    // Macros: println!, vec!, format!
    pub const MACRO: Color32 = Color32::from_rgb(84, 166, 224);        // #54A6E0 Sky Blue

    // Attributes: #[derive], #[test], #[cfg]
    pub const ATTRIBUTE: Color32 = Color32::from_rgb(197, 134, 192);   // #C586C0 Pink

    // Constants: MAX_SIZE, PI, DEFAULT_PORT
    pub const CONSTANT: Color32 = Color32::from_rgb(197, 134, 192);    // #C586C0 Pink

    // Lifetimes: 'a, 'static
    pub const LIFETIME: Color32 = Color32::from_rgb(78, 201, 176);     // #4EC9B0 Cyan

    // Namespaces/modules: std, crate, super
    pub const NAMESPACE: Color32 = Color32::from_rgb(212, 212, 212);   // #D4D4D4 White

    // Variables: x, count, buffer
    pub const VARIABLE: Color32 = Color32::from_rgb(212, 212, 212);    // デフォルト白と同じ

    // Operators: +, -, *, /, ::, =>
    pub const OPERATOR: Color32 = Color32::from_rgb(212, 212, 212);    // #D4D4D4 White
}

// ===== UI Color Palette =====

mod ui_colors {
    use egui::Color32;

    // Sidebar and panel background
    pub const SIDEBAR_BG: Color32 = Color32::from_rgb(25, 26, 28);     // #191A1C Dark Gray

    // Editor background (same as sidebar for unified look)
    pub const EDITOR_BG: Color32 = Color32::from_rgb(25, 26, 28);      // #191A1C Dark Gray

    // Default text color (UI elements, file names, etc.)
    pub const TEXT_DEFAULT: Color32 = Color32::from_rgb(212, 212, 212); // #D4D4D4 Light Gray

    // Window border stroke
    pub const BORDER: Color32 = Color32::from_rgb(54, 57, 59);         // #36393B Medium Gray
}

// ===== File Icon Color Palette =====
// Language-specific colors for file tree icons

mod file_icon_colors {
    use egui::Color32;

    // Rust files (.rs, Cargo.toml)
    pub const RUST_ORANGE: Color32 = Color32::from_rgb(255, 152, 0);   // #FF9800

    // Configuration files (.toml, .yaml, .json)
    pub const CONFIG_GRAY: Color32 = Color32::from_rgb(128, 128, 128); // #808080
    pub const JSON_YELLOW: Color32 = Color32::from_rgb(255, 203, 0);   // #FFCB00

    // Documentation files (.md)
    pub const MARKDOWN_BLUE: Color32 = Color32::from_rgb(66, 165, 245); // #42A5F5

    // JavaScript/TypeScript
    pub const JAVASCRIPT_YELLOW: Color32 = Color32::from_rgb(247, 223, 30); // #F7DF1E
    pub const TYPESCRIPT_BLUE: Color32 = Color32::from_rgb(41, 127, 214);  // #297FD6

    // Python
    pub const PYTHON_GREEN: Color32 = Color32::from_rgb(52, 168, 83);   // #34A853

    // Shell scripts
    pub const SHELL_GREEN: Color32 = Color32::from_rgb(76, 175, 80);    // #4CAF50

    // Web files (HTML, CSS, XML)
    pub const HTML_ORANGE: Color32 = Color32::from_rgb(229, 115, 0);    // #E57300
    pub const CSS_BLUE: Color32 = Color32::from_rgb(66, 165, 245);      // #42A5F5

    // Images and media
    pub const IMAGE_PURPLE: Color32 = Color32::from_rgb(156, 39, 176);  // #9C27B0
    pub const SVG_AMBER: Color32 = Color32::from_rgb(255, 179, 0);      // #FFB300

    // Git files
    pub const GIT_ORANGE: Color32 = Color32::from_rgb(240, 98, 35);     // #F06223

    // Protocol buffers
    pub const PROTO_PURPLE: Color32 = Color32::from_rgb(156, 39, 176);  // #9C27B0
}

/// Simple EditorTab structure (replaces core::virtual_editor::EditorTab)
#[derive(Clone)]
pub struct EditorTab {
    pub file_path: String,
    pub buffer: TextBuffer,
    pub cursor_line: usize,
    pub cursor_col: usize,
    pub is_dirty: bool,
    pub is_readonly: bool,  // NEW: Read-only flag for stdlib files
    pub pending_cursor_jump: Option<(usize, usize)>,  // NEW: (line, col) for programmatic cursor movement
}

impl EditorTab {
    pub fn new(file_path: String, content: String) -> Self {
        Self {
            file_path,
            buffer: TextBuffer::from_str(&content),
            cursor_line: 0,
            cursor_col: 0,
            is_dirty: false,
            is_readonly: false,
            pending_cursor_jump: None,
        }
    }

    pub fn mark_dirty(&mut self) {
        self.is_dirty = true;
    }
}

/// Active panel in the sidebar
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePanel {
    Explorer,
    Search,
    Git,
    Chat,
    Database,
    Workflow,
    Wiki,
    Terminal,
    VirtualOffice,
    Settings,
}

/// Settings Tab Categories (RustRover Style)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    Appearance,
    EditorColor,
    Plugins,
    Slack,
    GitHub,
}

// ===== NEW: Git UI Tabs (SourceTree-compatible) =====

/// Git panel tabs (6 tabs: Status, History, Branches, Remotes, Tags, Stash)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitTab {
    Status,
    History,
    Branches,
    Remotes,
    Tags,
    Stash,
}

/// State for the History tab (commit graph visualization)
#[derive(Debug, Clone)]
pub struct GitHistoryState {
    pub graph_nodes: Vec<native::git::GitGraphNode>,
    pub selected_commit_id: Option<String>,
    pub commit_details: Option<native::git::GitCommitDetail>,
    pub show_all_branches: bool,
    pub filter_author: String,
    pub filter_message: String,
    pub page_limit: usize, // Number of commits to load per page
    pub loaded_count: usize, // Number of commits currently loaded
}

impl Default for GitHistoryState {
    fn default() -> Self {
        Self {
            graph_nodes: Vec::new(),
            selected_commit_id: None,
            commit_details: None,
            show_all_branches: true,
            filter_author: String::new(),
            filter_message: String::new(),
            page_limit: 100,
            loaded_count: 0,
        }
    }
}

/// State for the Branches tab
#[derive(Debug, Clone)]
pub struct GitBranchState {
    pub local_branches: Vec<native::git::GitBranch>,
    pub remote_branches: Vec<native::git::GitBranch>,
    pub new_branch_name: String,
    pub merge_target: Option<String>,
}

impl Default for GitBranchState {
    fn default() -> Self {
        Self {
            local_branches: Vec::new(),
            remote_branches: Vec::new(),
            new_branch_name: String::new(),
            merge_target: None,
        }
    }
}

/// State for the Remotes tab
#[derive(Debug, Clone)]
pub struct GitRemoteState {
    pub remotes: Vec<native::git::GitRemote>,
    pub new_remote_name: String,
    pub new_remote_url: String,
}

impl Default for GitRemoteState {
    fn default() -> Self {
        Self {
            remotes: Vec::new(),
            new_remote_name: String::new(),
            new_remote_url: String::new(),
        }
    }
}

/// State for the Tags tab
#[derive(Debug, Clone)]
pub struct GitTagState {
    pub tags: Vec<native::git::GitTag>,
    pub new_tag_name: String,
    pub new_tag_message: String,
    pub annotated: bool,
}

impl Default for GitTagState {
    fn default() -> Self {
        Self {
            tags: Vec::new(),
            new_tag_name: String::new(),
            new_tag_message: String::new(),
            annotated: false,
        }
    }
}

/// State for the Stash tab
#[derive(Debug, Clone)]
pub struct GitStashState {
    pub stashes: Vec<native::git::GitStash>,
    pub new_stash_message: String,
    pub include_untracked: bool,
}

impl Default for GitStashState {
    fn default() -> Self {
        Self {
            stashes: Vec::new(),
            new_stash_message: String::new(),
            include_untracked: false,
        }
    }
}

/// State for Git Diff Viewer
#[derive(Debug, Clone)]
pub struct GitDiffState {
    pub selected_file: Option<String>,
    pub diff: Option<native::git::GitDiff>,
}

impl Default for GitDiffState {
    fn default() -> Self {
        Self {
            selected_file: None,
            diff: None,
        }
    }
}

/// Panel definition for data-driven Activity Bar
struct SidebarPanel {
    variant: ActivePanel,
    icon: &'static str,
    _name: &'static str, // For tooltip/accessibility
}

/// Main panels in the Activity Bar
const MAIN_PANELS: &[SidebarPanel] = &[
    SidebarPanel {
        variant: ActivePanel::Explorer,
        icon: "\u{ea83}",  // codicon-folder
        _name: "Explorer",
    },
    SidebarPanel {
        variant: ActivePanel::Search,
        icon: "\u{ea6d}",  // codicon-search
        _name: "Search",
    },
    SidebarPanel {
        variant: ActivePanel::Git,
        icon: "\u{ea84}",  // codicon-github
        _name: "Git",
    },
    SidebarPanel {
        variant: ActivePanel::Chat,
        icon: "\u{ea6b}",  // codicon-comment
        _name: "Chat",
    },
    SidebarPanel {
        variant: ActivePanel::Database,
        icon: "\u{eace}",  // codicon-database (U+EACE)
        _name: "Database",
    },
    SidebarPanel {
        variant: ActivePanel::Workflow,
        icon: "\u{ebb2}",  // codicon-tasklist
        _name: "Workflow",
    },
    SidebarPanel {
        variant: ActivePanel::Wiki,
        icon: "\u{ea88}",  // codicon-file-text (document)
        _name: "Wiki",
    },
    SidebarPanel {
        variant: ActivePanel::Terminal,
        icon: "\u{ea85}",  // codicon-terminal
        _name: "Terminal",
    },
];

/// Terminal line with style
#[derive(Debug, Clone)]
pub struct TerminalLine {
    pub text: String,
    pub style: TerminalStyle,
}

#[derive(Debug, Clone, Copy)]
pub enum TerminalStyle {
    Command,
    Output,
    Error,
}

/// Search match result
#[derive(Debug, Clone)]
pub struct SearchMatch {
    pub file_path: Option<String>, // For project-wide search
    pub line_number: usize,
    pub start_col: usize,
    pub end_col: usize,
    pub line_text: String,
}

/// Workflow definition (n8n-style visual workflow)
#[derive(Debug, Clone)]
pub struct Workflow {
    pub name: String,
    pub description: String,
    pub nodes: Vec<WorkflowNode>,
    pub connections: Vec<WorkflowConnection>,
    pub enabled: bool,
}

/// Workflow node (visual node on canvas)
#[derive(Debug, Clone)]
pub struct WorkflowNode {
    pub id: String,
    pub name: String,
    pub node_type: WorkflowNodeType,
    pub command: String,
    pub working_dir: Option<String>,
    pub position: egui::Pos2,  // Canvas position
    pub enabled: bool,
}

/// Workflow node types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowNodeType {
    Start,
    Command,
    Condition,
    End,
}

/// Connection between two nodes
#[derive(Debug, Clone)]
pub struct WorkflowConnection {
    pub from_node_id: String,
    pub to_node_id: String,
    pub label: Option<String>,  // e.g., "success", "failure"
}

impl Workflow {
    pub fn new(name: String) -> Self {
        Self {
            name,
            description: String::new(),
            nodes: Vec::new(),
            connections: Vec::new(),
            enabled: true,
        }
    }
}

impl WorkflowNode {
    pub fn new(id: String, name: String, node_type: WorkflowNodeType, position: egui::Pos2) -> Self {
        Self {
            id,
            name,
            node_type,
            command: String::new(),
            working_dir: None,
            position,
            enabled: true,
        }
    }

    /// Get input port position (left side, center)
    pub fn get_input_port_pos(&self, canvas_offset: egui::Vec2) -> egui::Pos2 {
        self.position + egui::vec2(0.0, 30.0) + canvas_offset
    }

    /// Get output port position (right side, center)
    pub fn get_output_port_pos(&self, canvas_offset: egui::Vec2) -> egui::Pos2 {
        self.position + egui::vec2(120.0, 30.0) + canvas_offset
    }
}

/// Workflow execution log entry
#[derive(Debug, Clone)]
pub struct WorkflowLogEntry {
    pub timestamp: String,
    pub node_id: String,
    pub node_name: String,
    pub message: String,
    pub log_type: WorkflowLogType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowLogType {
    Info,
    Success,
    Error,
    Warning,
}

/// Port type for workflow node connections
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortType {
    Input,
    Output,
}

// ===== Slack-like Chat System =====

/// Chat channel (like Slack channels)
#[derive(Debug, Clone)]
pub struct ChatChannel {
    pub id: String,
    pub name: String,
    pub description: String,
    pub channel_type: ChannelType,
    pub messages: Vec<ChatMessage>,
    pub unread_count: usize,
    pub is_archived: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelType {
    Public,       // # public channels
    Private,      // 🔒 private channels
    DirectMessage, // DM with another user
}

/// Chat message (supports threading like Slack)
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub id: String,
    pub channel_id: String,
    pub user_id: String,
    pub user_name: String,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub edited: bool,
    pub thread_replies: Vec<ChatMessage>, // Thread replies
    pub reactions: Vec<MessageReaction>,
    pub mentioned_users: Vec<String>, // @mentions
    pub is_pinned: bool,
}

/// Message reaction (emoji reactions like Slack)
#[derive(Debug, Clone)]
pub struct MessageReaction {
    pub emoji: String,
    pub user_ids: Vec<String>, // Users who reacted
}

impl ChatChannel {
    pub fn new(id: String, name: String, channel_type: ChannelType) -> Self {
        Self {
            id,
            name,
            description: String::new(),
            channel_type,
            messages: Vec::new(),
            unread_count: 0,
            is_archived: false,
        }
    }
}

impl ChatMessage {
    pub fn new(channel_id: String, user_id: String, user_name: String, content: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            channel_id,
            user_id,
            user_name,
            content,
            timestamp: chrono::Utc::now(),
            edited: false,
            thread_replies: Vec::new(),
            reactions: Vec::new(),
            mentioned_users: Vec::new(),
            is_pinned: false,
        }
    }
}

// ===== AI Chat System =====

/// gRPC AI Chat message
#[derive(Debug, Clone)]
pub struct GrpcMessage {
    pub content: String,
    pub is_user: bool,
}

// ===== Wiki System =====

/// Wiki page
#[derive(Debug, Clone)]
pub struct WikiPage {
    pub id: String,
    pub title: String,
    pub content: String, // Markdown format
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub tags: Vec<String>,
}

impl WikiPage {
    pub fn new(title: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title,
            content: String::new(),
            created_at: now,
            updated_at: now,
            tags: Vec::new(),
        }
    }
}

/// LSP async response messages
#[derive(Debug, Clone)]
pub enum LspResponse {
    Connected,  // NEW: LSP connection established
    Diagnostics(Vec<LspDiagnostic>),
    Hover(Option<LspHoverInfo>),
    Completions(Vec<LspCompletionItem>),
    Definition(Vec<LspLocation>),
    References(Vec<LspLocation>),  // NEW: Find references results
}

/// gRPC response types
#[derive(Debug, Clone)]
pub enum GrpcResponse {
    SessionStarted(String),  // Session ID
    ChatChunk(String),  // Streaming chat response chunk
    ChatStreamCompleted,  // Stream finished
}

/// Slack API responses
#[derive(Debug, Clone)]
pub enum SlackResponse {
    Authenticated,
    ChannelsList(Vec<native::slack::SlackChannel>),
    MessagesList(Vec<native::slack::SlackMessage>),
    MessageSent,
    Error(String),
}

/// Simplified LSP diagnostic
#[derive(Debug, Clone)]
pub struct LspDiagnostic {
    pub line: usize,
    pub column: usize,
    pub message: String,
    pub severity: DiagnosticSeverity,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

/// Simplified LSP hover info
#[derive(Debug, Clone)]
pub struct LspHoverInfo {
    pub contents: String,
    pub line: usize,
    pub column: usize,
}

/// Simplified LSP completion item
#[derive(Debug, Clone)]
pub struct LspCompletionItem {
    pub label: String,
    pub detail: Option<String>,
    pub kind: String,
}

/// Simplified LSP location
#[derive(Debug, Clone)]
pub struct LspLocation {
    pub file_path: String,
    pub line: usize,
    pub column: usize,
}

/// Pending go-to-definition request (for fallback)
#[derive(Debug, Clone)]
struct PendingGotoDefinition {
    word: String,
    original_text: String,
}

/// AI Chat mode
#[derive(Debug, Clone, Copy, PartialEq)]
enum AIChatMode {
    Chat,       // 対話式（通常のチャット）
    Autonomous, // 自動実行モード（dangerously-skip-permissions）
}

/// Main application state
pub struct BerryCodeApp {
    // === Project State ===
    root_path: String,
    selected_file: Option<(String, String)>, // (path, content)

    // === UI State ===
    active_panel: ActivePanel,
    sidebar_width: f32,

    // === Editor State ===
    editor_tabs: Vec<EditorTab>,
    active_tab_idx: usize,
    syntax_highlighter: SyntaxHighlighter, // Regex-based highlighter

    // === File Tree State ===
    file_tree_cache: Vec<DirEntry>, // Cached directory tree
    file_tree_load_pending: bool,
    expanded_dirs: HashSet<String>, // Set of expanded directory paths

    // === Terminal State ===
    terminal_output: Vec<TerminalLine>,
    terminal_input: String,
    terminal_visible: bool,
    terminal_history: Vec<String>,
    terminal_history_index: Option<usize>,
    terminal_working_dir: String,

    // === Search State ===
    search_query: String,
    search_dialog_open: bool,
    search_case_sensitive: bool,
    current_search_index: usize,
    search_results: Vec<SearchMatch>,
    // Replace功能
    replace_query: String,
    show_replace: bool, // Ctrl+H opens replace mode

    // === Git State ===
    git_current_branch: String,
    git_status: Vec<native::git::GitStatus>,
    git_commit_message: String,
    git_initialized: bool, // NEW: Track if Git has been initialized
    // NEW: SourceTree-compatible Git UI state
    git_active_tab: GitTab,
    git_history_state: GitHistoryState,
    git_branch_state: GitBranchState,
    git_remote_state: GitRemoteState,
    git_tag_state: GitTagState,
    git_stash_state: GitStashState,
    git_diff_state: GitDiffState,

    // === LSP State (Phase 6: Async integration) ===
    lsp_runtime: std::sync::Arc<tokio::runtime::Runtime>,  // NEW: Tokio runtime for async LSP
    lsp_client: Option<std::sync::Arc<native::lsp::LspClient>>,  // NEW: LSP client
    lsp_response_tx: Option<mpsc::UnboundedSender<LspResponse>>,  // NEW: Send LSP responses
    lsp_connected: bool,
    lsp_diagnostics: Vec<LspDiagnostic>,
    lsp_hover_info: Option<LspHoverInfo>,
    lsp_completions: Vec<LspCompletionItem>,
    lsp_show_completions: bool,
    lsp_show_hover: bool,
    lsp_response_rx: Option<mpsc::UnboundedReceiver<LspResponse>>,

    // === Status Message ===
    status_message: String,  // NEW: Status bar message
    status_message_timestamp: Option<std::time::Instant>,  // NEW: Message auto-clear timer

    // === Go-to-Definition State ===
    pending_goto_definition: Option<PendingGotoDefinition>,  // NEW: Fallback context
    definition_picker_locations: Vec<LspLocation>,  // NEW: Multiple definition picker
    show_definition_picker: bool,  // NEW: Show definition picker UI

    // === Find References State ===
    lsp_references: Vec<LspLocation>,  // NEW: Find references results
    show_references_panel: bool,  // NEW: Show references panel UI

    // === Slack-like Chat State ===
    pub chat_channels: Vec<ChatChannel>,
    pub selected_channel_id: Option<String>,
    pub chat_input: String,
    pub selected_message_for_thread: Option<String>, // Message ID for thread view
    pub show_thread_panel: bool,
    pub chat_search_query: String,
    pub show_channel_browser: bool,
    pub new_channel_name: String,
    pub current_user_id: String, // Current user ID
    pub current_user_name: String, // Current user name

    // === Slack Integration ===
    slack_client: native::slack::SlackClient,
    slack_token_input: String,
    slack_authenticated: bool,
    slack_channels: Vec<native::slack::SlackChannel>,
    slack_messages: Vec<native::slack::SlackMessage>,
    slack_response_tx: Option<mpsc::UnboundedSender<SlackResponse>>,
    slack_response_rx: Option<mpsc::UnboundedReceiver<SlackResponse>>,
    show_slack_settings: bool,

    // === Database Integration ===
    database_client: native::database::DatabaseClient,
    database_connections: Vec<native::database::DatabaseConnection>,
    selected_connection_id: Option<String>,
    database_tables: Vec<String>,
    selected_table: Option<String>,
    sql_query: String,
    query_result: Option<native::database::QueryResult>,
    show_add_connection_dialog: bool,
    new_db_name: String,
    new_db_comment: String,
    new_db_type: native::database::DatabaseType,
    new_db_connection_type: String,
    new_db_host: String,
    new_db_port: String,
    new_db_database: String,
    new_db_authentication: String,
    new_db_username: String,
    new_db_password: String,
    new_db_save_mode: String,
    new_db_url: String,
    new_db_active_tab: usize,
    new_db_test_result: Option<String>,

    // gRPC for AI integration (optional)
    grpc_client: native::grpc::GrpcClient,
    grpc_session_id: Option<String>,
    grpc_connected: bool,
    grpc_response_tx: Option<mpsc::UnboundedSender<GrpcResponse>>,
    grpc_response_rx: Option<mpsc::UnboundedReceiver<GrpcResponse>>,
    grpc_streaming_message: Option<String>,

    // AI Chat Panel State
    ai_chat_mode: AIChatMode,
    grpc_messages: Vec<GrpcMessage>,
    grpc_input: String,
    grpc_streaming: bool,
    grpc_current_response: String,

    // === Settings ===
    show_settings: bool,
    active_settings_tab: SettingsTab,

    // === Theme (Customizable Syntax Colors) ===
    show_theme_editor: bool,
    keyword_color: egui::Color32,
    function_color: egui::Color32,
    type_color: egui::Color32,
    string_color: egui::Color32,
    number_color: egui::Color32,
    comment_color: egui::Color32,       // Normal comments: //
    doc_comment_color: egui::Color32,   // Doc comments: //!, ///
    macro_color: egui::Color32,
    attribute_color: egui::Color32,
    constant_color: egui::Color32,
    lifetime_color: egui::Color32,
    namespace_color: egui::Color32,
    variable_color: egui::Color32,
    operator_color: egui::Color32,

    // === Focus Management ===
    active_focus: FocusLayer,

    // === Workflow State ===
    workflows: Vec<Workflow>,
    selected_workflow_idx: Option<usize>,
    workflow_editor_open: bool,
    new_workflow_name: String,

    // Workflow Canvas State
    workflow_canvas_offset: egui::Vec2,
    workflow_canvas_zoom: f32,
    dragging_node_id: Option<String>,
    dragging_from_port: Option<(String, PortType)>, // (node_id, port_type)
    selected_node_id: Option<String>,
    drag_connection_end: Option<egui::Pos2>, // For drawing connection preview while dragging

    // Workflow Execution State
    workflow_logs: Vec<WorkflowLogEntry>,
    workflow_running: bool,

    // Node Editor
    node_editor_open: bool,
    editing_node_id: Option<String>,
    new_node_name: String,
    new_node_command: String,

    // === Wiki State ===
    wiki_pages: Vec<WikiPage>,
    selected_wiki_page_id: Option<String>,
    wiki_editing: bool,
    wiki_search_query: String,
    new_wiki_title: String,

    // === File Watcher ===
    file_watcher: Option<native::watcher::FileWatcher>,  // Real-time file system monitoring
}

/// Color theme for syntax highlighting
#[derive(Debug, Clone, Copy)]
struct ColorTheme {
    keyword: egui::Color32,
    function: egui::Color32,
    type_: egui::Color32,
    string: egui::Color32,
    number: egui::Color32,
    comment: egui::Color32,       // Normal comments: //
    doc_comment: egui::Color32,   // Doc comments: //!, ///
    macro_: egui::Color32,
    attribute: egui::Color32,
    constant: egui::Color32,
    lifetime: egui::Color32,
    namespace: egui::Color32,
    variable: egui::Color32,
    operator: egui::Color32,
}

impl BerryCodeApp {
    /// Create new application instance
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Configure One Dark theme using named color constants
        let mut style = egui::Style::default();
        let mut visuals = egui::Visuals::dark();
        // CRITICAL: グローバルでは override_text_color を使わない（エディタのシンタックスハイライトが効かなくなる）
        visuals.override_text_color = None;
        // デフォルトのテキスト色を明るい白に設定
        visuals.text_cursor.stroke.color = ui_colors::TEXT_DEFAULT;
        visuals.window_fill = ui_colors::SIDEBAR_BG;
        visuals.panel_fill = ui_colors::SIDEBAR_BG;
        visuals.extreme_bg_color = ui_colors::EDITOR_BG;
        visuals.window_stroke = egui::Stroke::new(1.0, ui_colors::BORDER);
        style.visuals = visuals;

        // テキストスタイルのデフォルト色を明るい白に設定
        style.visuals.widgets.noninteractive.fg_stroke.color = ui_colors::TEXT_DEFAULT;
        style.visuals.widgets.inactive.fg_stroke.color = ui_colors::TEXT_DEFAULT;
        style.visuals.widgets.hovered.fg_stroke.color = ui_colors::TEXT_DEFAULT;
        style.visuals.widgets.active.fg_stroke.color = ui_colors::TEXT_DEFAULT;

        cc.egui_ctx.set_style(style);

        // Get project root directory
        let root_path = native::fs::get_current_dir().unwrap_or_else(|e| {
            tracing::warn!("⚠️  Failed to get current directory: {}, using fallback", e);
            ".".to_string()
        });

        tracing::info!("📁 Project root: {}", root_path);

        let terminal_working_dir = root_path.clone();

        // Create Tokio runtime for async LSP operations
        let lsp_runtime = std::sync::Arc::new(
            tokio::runtime::Runtime::new()
                .expect("Failed to create Tokio runtime for LSP")
        );

        // Create LSP client (connects to berry-api-server on port 50051)
        // Use [::1] for IPv6 localhost to match berry-api-server
        let lsp_client = std::sync::Arc::new(native::lsp::LspClient::new("http://[::1]:50051"));

        // Create gRPC client (connects to berry-api-server on port 50051)
        let grpc_client = native::grpc::GrpcClient::new("http://[::1]:50051");

        // Create LSP response channel
        let (lsp_tx, lsp_rx) = mpsc::unbounded_channel();

        // Create gRPC response channel
        let (grpc_tx, grpc_rx) = mpsc::unbounded_channel();

        // Create Slack response channel
        let (slack_tx, slack_rx) = mpsc::unbounded_channel();

        // Create file watcher for real-time file system monitoring
        let file_watcher = match native::watcher::FileWatcher::new() {
            Ok(mut watcher) => {
                // Watch the project root directory recursively
                if let Err(e) = watcher.watch(&root_path) {
                    tracing::warn!("⚠️  Failed to start file watching for {}: {}", root_path, e);
                    None
                } else {
                    tracing::info!("👁  File watcher started for: {}", root_path);
                    Some(watcher)
                }
            }
            Err(e) => {
                tracing::warn!("⚠️  Failed to create file watcher: {}", e);
                None
            }
        };

        // Spawn LSP connection task
        let client_clone = lsp_client.clone();
        let root_path_clone = root_path.clone();
        let tx_clone = lsp_tx.clone();

        lsp_runtime.spawn(async move {
            match client_clone.connect().await {
                Ok(_) => {
                    tracing::info!("✅ LSP client connected to berry-api-server");

                    // Initialize for Rust language
                    match client_clone.initialize(
                        "rust",
                        format!("file://{}", root_path_clone),
                        Some(root_path_clone.clone())
                    ).await {
                        Ok(response) => {
                            tracing::info!("🔧 LSP initialized for Rust: {:?}", response);
                            // Notify UI that LSP is connected
                            let _ = tx_clone.send(LspResponse::Connected);
                        }
                        Err(e) => {
                            tracing::error!("❌ LSP initialization failed: {:#}", e);
                            tracing::error!("   Root path: {}", root_path_clone);
                        }
                    }
                }
                Err(e) => tracing::warn!("⚠️  LSP connection failed: {} (will use fallback)", e),
            }
        });

        // Spawn gRPC connection and session initialization task
        let runtime_clone = lsp_runtime.clone();
        let root_path_for_grpc = root_path.clone();
        let grpc_tx_clone = grpc_tx.clone();
        let grpc_client_clone = grpc_client.clone();
        runtime_clone.spawn(async move {
            match grpc_client_clone.connect().await {
                Ok(_) => {
                    tracing::info!("✅ gRPC client connected to berry-api-server");
                    // Start chat session (autonomous: true = auto-continue with tools)
                    match grpc_client_clone.start_session(root_path_for_grpc, true).await {
                        Ok(session_id) => {
                            tracing::info!("🎯 gRPC chat session started: {}", session_id);
                            // Send session ID to UI
                            let _ = grpc_tx_clone.send(GrpcResponse::SessionStarted(session_id));
                        }
                        Err(e) => {
                            tracing::error!("❌ Failed to start gRPC session: {:#}", e);
                        }
                    }
                }
                Err(e) => tracing::warn!("⚠️  gRPC connection failed: {}", e),
            }
        });

        Self {
            root_path,
            selected_file: None,
            active_panel: ActivePanel::Explorer,
            sidebar_width: 300.0,
            editor_tabs: Vec::new(),
            active_tab_idx: 0,
            syntax_highlighter: SyntaxHighlighter::new(), // Regex-based highlighter
            file_tree_cache: Vec::new(),
            file_tree_load_pending: true,
            expanded_dirs: HashSet::new(),
            terminal_output: Vec::new(),
            terminal_input: String::new(),
            terminal_visible: true,
            terminal_history: Vec::new(),
            terminal_history_index: None,
            terminal_working_dir,
            search_query: String::new(),
            search_dialog_open: false,
            search_case_sensitive: false,
            current_search_index: 0,
            search_results: Vec::new(),
            replace_query: String::new(),
            show_replace: false,
            git_current_branch: String::from("(unknown)"),
            git_status: Vec::new(),
            git_commit_message: String::new(),
            git_initialized: false,
            git_active_tab: GitTab::Status,
            git_history_state: GitHistoryState::default(),
            git_branch_state: GitBranchState::default(),
            git_remote_state: GitRemoteState::default(),
            git_tag_state: GitTagState::default(),
            git_stash_state: GitStashState::default(),
            git_diff_state: GitDiffState::default(),
            lsp_runtime,
            lsp_client: Some(lsp_client),
            lsp_response_tx: Some(lsp_tx),
            lsp_connected: false,
            lsp_diagnostics: Vec::new(),
            lsp_hover_info: None,
            lsp_completions: Vec::new(),
            lsp_show_completions: false,
            lsp_show_hover: false,
            lsp_response_rx: Some(lsp_rx),
            status_message: String::new(),
            status_message_timestamp: None,
            pending_goto_definition: None,
            definition_picker_locations: Vec::new(),
            show_definition_picker: false,
            lsp_references: Vec::new(),
            show_references_panel: false,

            // === Slack-like Chat ===
            chat_channels: vec![],  // Start with empty, will be populated from Slack API
            selected_channel_id: None,
            chat_input: String::new(),
            selected_message_for_thread: None,
            show_thread_panel: false,
            chat_search_query: String::new(),
            show_channel_browser: false,
            new_channel_name: String::new(),
            current_user_id: "user_1".to_string(),
            current_user_name: "Developer".to_string(),

            grpc_client,
            grpc_session_id: None,
            grpc_connected: false,
            grpc_response_tx: Some(grpc_tx),
            grpc_response_rx: Some(grpc_rx),
            grpc_streaming_message: None,
            ai_chat_mode: AIChatMode::Chat,
            grpc_messages: Vec::new(),
            grpc_input: String::new(),
            grpc_streaming: false,
            grpc_current_response: String::new(),
            show_settings: false,
            active_settings_tab: SettingsTab::EditorColor,
            show_theme_editor: false,
            // Use named color constants from syntax_colors module
            keyword_color: syntax_colors::KEYWORD,
            function_color: syntax_colors::FUNCTION,
            type_color: syntax_colors::TYPE,
            string_color: syntax_colors::STRING,
            number_color: syntax_colors::NUMBER,
            comment_color: syntax_colors::COMMENT,
            doc_comment_color: syntax_colors::DOC_COMMENT,
            macro_color: syntax_colors::MACRO,
            attribute_color: syntax_colors::ATTRIBUTE,
            constant_color: syntax_colors::CONSTANT,
            lifetime_color: syntax_colors::LIFETIME,
            namespace_color: syntax_colors::NAMESPACE,
            variable_color: syntax_colors::VARIABLE,
            operator_color: syntax_colors::OPERATOR,
            active_focus: FocusLayer::Editor,

            // === Workflow State ===
            workflows: Vec::new(),
            selected_workflow_idx: None,
            workflow_editor_open: false,
            new_workflow_name: String::new(),

            // Workflow Canvas State
            workflow_canvas_offset: egui::Vec2::ZERO,
            workflow_canvas_zoom: 1.0,
            dragging_node_id: None,
            dragging_from_port: None,
            selected_node_id: None,
            drag_connection_end: None,

            // Workflow Execution State
            workflow_logs: Vec::new(),
            workflow_running: false,

            // Node Editor
            node_editor_open: false,
            editing_node_id: None,
            new_node_name: String::new(),
            new_node_command: String::new(),

            // === Wiki State ===
            wiki_pages: vec![
                WikiPage::new("Welcome to Project Wiki".to_string()),
            ],
            selected_wiki_page_id: None,
            wiki_editing: false,
            wiki_search_query: String::new(),
            new_wiki_title: String::new(),

            // === Slack Integration ===
            slack_client: native::slack::SlackClient::new(),
            slack_token_input: String::new(),
            slack_authenticated: false,
            slack_channels: Vec::new(),
            slack_messages: Vec::new(),
            slack_response_tx: Some(slack_tx),
            slack_response_rx: Some(slack_rx),
            show_slack_settings: false,

            // === Database Integration ===
            database_client: native::database::DatabaseClient::new(),
            database_connections: Vec::new(),
            selected_connection_id: None,
            database_tables: Vec::new(),
            selected_table: None,
            sql_query: String::new(),
            query_result: None,
            show_add_connection_dialog: false,
            new_db_name: String::new(),
            new_db_comment: String::new(),
            new_db_type: native::database::DatabaseType::PostgreSQL,
            new_db_connection_type: "default".to_string(),
            new_db_host: "localhost".to_string(),
            new_db_port: "5432".to_string(),
            new_db_database: "postgres".to_string(),
            new_db_authentication: "User & Password".to_string(),
            new_db_username: String::new(),
            new_db_password: String::new(),
            new_db_save_mode: "Forever".to_string(),
            new_db_url: "jdbc:postgresql://localhost:5432/postgres".to_string(),
            new_db_active_tab: 0,
            new_db_test_result: None,

            // === File Watcher ===
            file_watcher,
        }
    }

    /// Render top header bar (tab bar under native title)
    fn render_top_header(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_header")
            .exact_height(32.0)
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(50, 51, 54)) // Dark gray background #323336
                    .inner_margin(egui::Margin::symmetric(8.0, 4.0))
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(4.0);

                    // Purple tab with project info
                    let tab_rect_size = egui::vec2(160.0, 24.0);
                    let (tab_rect, _response) = ui.allocate_exact_size(tab_rect_size, egui::Sense::click());

                    // Draw purple background
                    ui.painter().rect_filled(
                        tab_rect,
                        4.0, // Rounded corners
                        egui::Color32::from_rgb(126, 89, 161), // Purple #7E59A1
                    );

                    // Draw badge with "0"
                    let badge_center = egui::pos2(tab_rect.left() + 16.0, tab_rect.center().y);
                    ui.painter().circle_filled(
                        badge_center,
                        9.0,
                        egui::Color32::from_rgba_premultiplied(255, 255, 255, 60),
                    );
                    ui.painter().text(
                        badge_center,
                        egui::Align2::CENTER_CENTER,
                        "0",
                        egui::FontId::proportional(12.0),
                        egui::Color32::WHITE,
                    );

                    // Project name
                    let project_name = self.root_path
                        .split('/')
                        .last()
                        .unwrap_or("oracleberry");

                    let text_pos = egui::pos2(tab_rect.left() + 34.0, tab_rect.center().y);
                    ui.painter().text(
                        text_pos,
                        egui::Align2::LEFT_CENTER,
                        project_name,
                        egui::FontId::proportional(12.0),
                        egui::Color32::WHITE,
                    );

                    // Dropdown arrow
                    let arrow_pos = egui::pos2(tab_rect.right() - 12.0, tab_rect.center().y);
                    ui.painter().text(
                        arrow_pos,
                        egui::Align2::CENTER_CENTER,
                        "▼",
                        egui::FontId::proportional(9.0),
                        egui::Color32::from_rgb(200, 200, 200),
                    );
                });
            });
    }

    /// Render Activity Bar (left-most 48px panel with icons)
    fn render_activity_bar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("activity_bar")
            .exact_width(48.0)
            .resizable(false)
            .frame(
                egui::Frame::none()
                    .fill(ui_colors::SIDEBAR_BG) // #191A1C
                    .inner_margin(egui::Margin::same(4.0))
            )
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(8.0);

                    // Increase icon size for Activity Bar
                    ui.style_mut().text_styles.insert(
                        egui::TextStyle::Button,
                        egui::FontId::proportional(20.0), // Increased from default
                    );

                    for panel in MAIN_PANELS {
                        let is_selected = self.active_panel == panel.variant;

                        // Use selectable_label with custom color
                        let icon_text = egui::RichText::new(panel.icon)
                            .size(20.0);  // Explicit size for icons
                        if ui.selectable_label(is_selected, icon_text).clicked() {
                            tracing::info!("📍 Panel changed to: {:?}", panel.variant);
                            self.active_panel = panel.variant;
                        }

                        ui.add_space(4.0);
                    }
                });
            });
    }

    /// Render Sidebar (file tree, chat, terminal, etc.)
    fn render_sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("sidebar")
            .default_width(self.sidebar_width)
            .width_range(100.0..=600.0)
            .resizable(true)
            .frame(
                egui::Frame::none()
                    .fill(ui_colors::SIDEBAR_BG) // #191A1C
                    .inner_margin(egui::Margin::same(8.0))
            )
            .show(ctx, |ui| {
                // Update sidebar width from UI
                self.sidebar_width = ui.available_width();

                // Render content based on active panel
                // NOTE: Chat panel is now permanently on the right side
                match self.active_panel {
                    ActivePanel::Explorer => self.render_file_tree(ui),
                    ActivePanel::Search => self.render_search_panel(ui),
                    ActivePanel::Git => self.render_git_panel(ui),
                    ActivePanel::Chat => { /* Chat handled separately in update() */ }
                    ActivePanel::Terminal => self.render_terminal(ui),
                    ActivePanel::Database => {
                        ui.heading("Database");
                        ui.label("Database panel - 未実装");
                    }
                    ActivePanel::Workflow => self.render_workflow(ui),
                    ActivePanel::Wiki => self.render_wiki_sidebar(ui),
                    ActivePanel::VirtualOffice => {
                        ui.heading("Virtual Office");
                        ui.label("Virtual Office panel - 未実装");
                    }
                    ActivePanel::Settings => {
                        self.render_settings_panel(ui);
                    }
                }
            });
    }

    /// Render File Tree panel (Phase 2: full implementation)
    fn render_file_tree(&mut self, ui: &mut egui::Ui) {
        // Project name dropdown
        let project_name = self.root_path
            .split('/')
            .last()
            .unwrap_or("oracleberry");

        ui.horizontal(|ui| {
            // Folder icon
            ui.label(
                egui::RichText::new("\u{ea83}") // codicon-folder
                    .size(16.0)
                    .color(ui_colors::TEXT_DEFAULT)
            );

            ui.add_space(4.0);

            // Project name with dropdown
            let response = ui.button(
                egui::RichText::new(format!("{} ▼", project_name.to_uppercase()))
                    .size(11.0)
                    .strong()
            );

            // TODO: Show dropdown menu when clicked
            if response.clicked() {
                // Future: Show project switcher menu
            }
        });

        ui.separator();

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
            // Use larger font for label
            ui.style_mut().text_styles.insert(
                egui::TextStyle::Body,
                egui::FontId::proportional(14.0),
            );

            // Load file tree on first render
            if self.file_tree_cache.is_empty() && self.file_tree_load_pending {
                ui.label("読み込み中...");

                // Load root directory (max_depth: 1)
                match native::fs::read_dir(&self.root_path, Some(1)) {
                    Ok(entries) => {
                        tracing::info!("✅ Loaded {} entries from {}", entries.len(), self.root_path);
                        self.file_tree_cache = entries;
                        self.file_tree_load_pending = false;
                    }
                    Err(e) => {
                        ui.colored_label(egui::Color32::RED, format!("エラー: {}", e));
                        self.file_tree_load_pending = false;
                    }
                }
            }

            // Create root node representing the project folder
            let root_name = self.root_path.split('/').last().unwrap_or(&self.root_path);
            let is_root_expanded = self.expanded_dirs.contains(&self.root_path);
            let root_icon = if is_root_expanded { "\u{ea7c}" } else { "\u{ea83}" }; // codicon-folder-opened / codicon-folder

            // Render root folder
            let root_label = format!("{} {}", root_icon, root_name);
            let response = ui.add(
                egui::Label::new(root_label)
                .sense(egui::Sense::click())
            )
            .on_hover_cursor(egui::CursorIcon::Default);
            if response.clicked() {
                if is_root_expanded {
                    self.expanded_dirs.remove(&self.root_path);
                } else {
                    self.expanded_dirs.insert(self.root_path.clone());
                }
            }

            // Render children if root is expanded
            if is_root_expanded {
                for entry in self.file_tree_cache.clone() {
                    self.render_tree_node(ui, &entry, 1); // Start at depth 1
                }
            }
        });
    }

    /// Render a single tree node (file or directory) recursively
    fn render_tree_node(&mut self, ui: &mut egui::Ui, node: &DirEntry, depth: usize) {
        let indent = depth as f32 * 20.0;

        // Render current node
        ui.horizontal(|ui| {
            ui.add_space(indent);

            // Use larger font for file tree
            ui.style_mut().text_styles.insert(
                egui::TextStyle::Button,
                egui::FontId::proportional(15.0), // Larger font size
            );

            if node.is_dir {
                // Directory node
                let is_expanded = self.expanded_dirs.contains(&node.path);
                // Codicon folder icons: closed=\u{ea83}, open=\u{ea7c}
                let icon = if is_expanded { "\u{ea7c}" } else { "\u{ea83}" };

                let dir_label = format!("{} {}", icon, node.name);
                let response = ui.add(
                    egui::Label::new(dir_label)
                    .sense(egui::Sense::click())
                )
                .on_hover_cursor(egui::CursorIcon::Default);

                if response.clicked() {
                    // Toggle expansion
                    if is_expanded {
                        self.expanded_dirs.remove(&node.path);
                        tracing::info!("📁 Collapsed: {}", node.path);
                    } else {
                        self.expanded_dirs.insert(node.path.clone());
                        tracing::info!("📂 Expanded: {}", node.path);

                        // Load children if not already loaded
                        if node.children.is_none() {
                            self.load_directory_children(&node.path);
                        }
                    }
                }
            } else {
                // File node with colored icon
                let (icon, color) = Self::get_file_icon_with_color(&node.name);

                let response = ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    ui.label(egui::RichText::new(icon).color(color));
                    ui.add(egui::Label::new(&node.name).sense(egui::Sense::click()))
                }).inner;

                if response.clicked() {
                    self.open_file_from_path(&node.path);
                }

                response.on_hover_cursor(egui::CursorIcon::Default);
            }
        });

        // Render children OUTSIDE of horizontal layout (so they appear on separate lines)
        if node.is_dir {
            let is_expanded = self.expanded_dirs.contains(&node.path);
            if is_expanded {
                if let Some(children) = &node.children {
                    for child in children {
                        self.render_tree_node(ui, child, depth + 1);
                    }
                }
            }
        }
    }

    /// Open a file from a given path (used by file tree and search results)
    fn open_file_from_path(&mut self, file_path: &str) {
        tracing::info!("📄 Opening file: {}", file_path);

        match native::fs::read_file(file_path) {
            Ok(content) => {
                // Language detection is done in render_editor_panel for each frame
                // Check if file is already open
                if let Some(idx) = self.editor_tabs.iter().position(|tab| tab.file_path == file_path) {
                    // Switch to existing tab
                    self.active_tab_idx = idx;
                    tracing::info!("✅ Switched to existing tab: {}", file_path);
                } else {
                    // Create new editor tab
                    let tab = EditorTab::new(file_path.to_string(), content.clone());
                    self.editor_tabs.push(tab);
                    self.active_tab_idx = self.editor_tabs.len() - 1;
                    tracing::info!("✅ File loaded in new tab: {} ({} bytes)", file_path, content.len());
                }

                self.selected_file = Some((file_path.to_string(), content));
            }
            Err(e) => {
                tracing::error!("❌ Failed to read file {}: {}", file_path, e);
            }
        }
    }

    /// Load children for a specific directory
    fn load_directory_children(&mut self, dir_path: &str) {
        match native::fs::read_dir(dir_path, Some(1)) {
            Ok(children) => {
                tracing::info!("✅ Loaded {} children for {}", children.len(), dir_path);

                // Update the cache by finding the directory and updating its children
                Self::update_dir_entry_children(&mut self.file_tree_cache, dir_path, children);
            }
            Err(e) => {
                tracing::error!("❌ Failed to load directory {}: {}", dir_path, e);
            }
        }
    }

    /// Recursively update a directory entry's children in the cache
    fn update_dir_entry_children(entries: &mut Vec<DirEntry>, target_path: &str, new_children: Vec<DirEntry>) {
        for entry in entries.iter_mut() {
            if entry.path == target_path {
                entry.children = Some(new_children);
                return;
            }

            if let Some(children) = &mut entry.children {
                Self::update_dir_entry_children(children, target_path, new_children.clone());
            }
        }
    }

    /// Get file icon based on file extension (static version for use in closures)
    fn get_file_icon_static(filename: &str) -> &'static str {
        // Codicon icons (using Unicode code points)
        if filename.ends_with(".rs") {
            "\u{eb8b}" // codicon-file-code (Rust)
        } else if filename.ends_with(".toml") {
            "\u{ea7e}" // codicon-settings-gear (Config)
        } else if filename.ends_with(".md") {
            "\u{ea82}" // codicon-markdown (Markdown)
        } else if filename.ends_with(".json") {
            "\u{ead1}" // codicon-json (JSON)
        } else if filename.ends_with(".yaml") || filename.ends_with(".yml") {
            "\u{ea7e}" // codicon-settings-gear (YAML)
        } else if filename.ends_with(".js") {
            "\u{ea7a}" // codicon-symbol-method (JavaScript)
        } else if filename.ends_with(".ts") {
            "\u{ea7a}" // codicon-symbol-method (TypeScript)
        } else if filename.ends_with(".html") {
            "\u{eb7e}" // codicon-code (HTML)
        } else if filename.ends_with(".css") {
            "\u{eb7e}" // codicon-code (CSS)
        } else if filename.ends_with(".py") {
            "\u{eb8b}" // codicon-file-code (Python)
        } else if filename.ends_with(".sh") {
            "\u{ea85}" // codicon-terminal (Shell script)
        } else if filename.ends_with(".txt") {
            "\u{ea7b}" // codicon-file (Text)
        } else if filename.ends_with(".lock") {
            "\u{ea7f}" // codicon-lock (Lock file)
        } else if filename.ends_with(".proto") {
            "\u{eb8b}" // codicon-file-code (Protocol buffers)
        } else if filename.ends_with(".xml") {
            "\u{eb7e}" // codicon-code (XML)
        } else if filename.ends_with(".svg") {
            "\u{eaf0}" // codicon-file-media (SVG)
        } else if filename.ends_with(".png") || filename.ends_with(".jpg") || filename.ends_with(".jpeg") {
            "\u{eaf0}" // codicon-file-media (Images)
        } else if filename.ends_with(".gitignore") || filename.ends_with(".gitattributes") {
            "\u{ea84}" // codicon-git-branch (Git)
        } else if filename == "Cargo.toml" || filename == "Cargo.lock" {
            "\u{ea7e}" // codicon-settings-gear (Cargo)
        } else if filename == "package.json" {
            "\u{ead1}" // codicon-json (npm)
        } else if filename == "README.md" {
            "\u{ea82}" // codicon-markdown (README)
        } else {
            "\u{ea7b}" // codicon-file (Default)
        }
    }

    /// Get file icon with color based on file extension
    fn get_file_icon_with_color(filename: &str) -> (&'static str, egui::Color32) {
        // Use named color constants from file_icon_colors module
        if filename.ends_with(".rs") {
            ("\u{eb8b}", file_icon_colors::RUST_ORANGE)
        } else if filename.ends_with(".toml") {
            ("\u{ea7e}", file_icon_colors::CONFIG_GRAY)
        } else if filename.ends_with(".md") {
            ("\u{ea82}", file_icon_colors::MARKDOWN_BLUE)
        } else if filename.ends_with(".json") {
            ("\u{ead1}", file_icon_colors::JSON_YELLOW)
        } else if filename.ends_with(".yaml") || filename.ends_with(".yml") {
            ("\u{ea7e}", file_icon_colors::CONFIG_GRAY)
        } else if filename.ends_with(".js") {
            ("\u{ea7a}", file_icon_colors::JAVASCRIPT_YELLOW)
        } else if filename.ends_with(".ts") {
            ("\u{ea7a}", file_icon_colors::TYPESCRIPT_BLUE)
        } else if filename.ends_with(".html") {
            ("\u{eb7e}", file_icon_colors::HTML_ORANGE)
        } else if filename.ends_with(".css") {
            ("\u{eb7e}", file_icon_colors::CSS_BLUE)
        } else if filename.ends_with(".py") {
            ("\u{eb8b}", file_icon_colors::PYTHON_GREEN)
        } else if filename.ends_with(".sh") {
            ("\u{ea85}", file_icon_colors::SHELL_GREEN)
        } else if filename.ends_with(".txt") {
            ("\u{ea7b}", ui_colors::TEXT_DEFAULT)
        } else if filename.ends_with(".lock") {
            ("\u{ea7f}", file_icon_colors::CONFIG_GRAY)
        } else if filename.ends_with(".proto") {
            ("\u{eb8b}", file_icon_colors::PROTO_PURPLE)
        } else if filename.ends_with(".xml") {
            ("\u{eb7e}", file_icon_colors::HTML_ORANGE)
        } else if filename.ends_with(".svg") {
            ("\u{eaf0}", file_icon_colors::SVG_AMBER)
        } else if filename.ends_with(".png") || filename.ends_with(".jpg") || filename.ends_with(".jpeg") {
            ("\u{eaf0}", file_icon_colors::IMAGE_PURPLE)
        } else if filename.ends_with(".gitignore") || filename.ends_with(".gitattributes") {
            ("\u{ea84}", file_icon_colors::GIT_ORANGE)
        } else if filename == "Cargo.toml" || filename == "Cargo.lock" {
            ("\u{ea7e}", file_icon_colors::RUST_ORANGE)
        } else if filename == "package.json" {
            ("\u{ead1}", file_icon_colors::JSON_YELLOW)
        } else if filename == "README.md" {
            ("\u{ea82}", file_icon_colors::MARKDOWN_BLUE)
        } else {
            ("\u{ea7b}", ui_colors::TEXT_DEFAULT)
        }
    }

    /// Render Workflow panel (left sidebar)
    fn render_workflow(&mut self, ui: &mut egui::Ui) {
        ui.heading(format!("{} Workflows", "\u{ebb2}")); // codicon-tasklist
        ui.separator();

        // New Workflow Button - creates empty canvas
        if ui.button("➕ New Workflow").clicked() {
            let new_workflow = Workflow::new(format!("Untitled Workflow {}", self.workflows.len() + 1));
            self.workflows.push(new_workflow);
            self.selected_workflow_idx = Some(self.workflows.len() - 1);
        }

        ui.add_space(8.0);

        // Workflow List
        egui::ScrollArea::vertical()
            .id_source("workflow_list")
            .show(ui, |ui| {
                let mut workflow_to_delete: Option<usize> = None;
                let mut workflow_to_rename: Option<usize> = None;

                for (idx, workflow) in self.workflows.iter().enumerate() {
                    let is_selected = self.selected_workflow_idx == Some(idx);

                    egui::Frame::none()
                        .fill(if is_selected {
                            egui::Color32::from_rgb(60, 80, 120)
                        } else {
                            egui::Color32::from_rgb(35, 35, 35)
                        })
                        .inner_margin(8.0)
                        .rounding(4.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                // Workflow name
                                let response = ui.selectable_label(is_selected, &workflow.name);
                                if response.clicked() {
                                    self.selected_workflow_idx = Some(idx);
                                }

                                // Double-click to rename
                                if response.double_clicked() {
                                    workflow_to_rename = Some(idx);
                                }

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    // Delete button
                                    if ui.small_button("🗑").clicked() {
                                        workflow_to_delete = Some(idx);
                                    }
                                });
                            });

                            // Show node and connection count
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(format!("{} nodes", workflow.nodes.len()))
                                    .size(10.0)
                                    .color(egui::Color32::from_rgb(120, 120, 120)));
                                ui.label(egui::RichText::new("•").size(10.0).color(egui::Color32::from_rgb(80, 80, 80)));
                                ui.label(egui::RichText::new(format!("{} connections", workflow.connections.len()))
                                    .size(10.0)
                                    .color(egui::Color32::from_rgb(120, 120, 120)));
                            });
                        });

                    ui.add_space(4.0);
                }

                // Handle actions
                if let Some(idx) = workflow_to_delete {
                    self.workflows.remove(idx);
                    if self.selected_workflow_idx == Some(idx) {
                        self.selected_workflow_idx = None;
                    }
                }

                if let Some(idx) = workflow_to_rename {
                    if let Some(workflow) = self.workflows.get(idx) {
                        self.new_workflow_name = workflow.name.clone();
                        self.workflow_editor_open = true;
                        self.selected_workflow_idx = Some(idx);
                    }
                }
            });

        ui.add_space(8.0);

        // Instructions
        ui.separator();
        ui.label(egui::RichText::new("💡 Tips:")
            .size(11.0)
            .color(egui::Color32::from_rgb(150, 150, 150)));
        ui.label(egui::RichText::new("• Create workflow")
            .size(10.0)
            .color(egui::Color32::from_rgb(120, 120, 120)));
        ui.label(egui::RichText::new("• Add nodes with ➕")
            .size(10.0)
            .color(egui::Color32::from_rgb(120, 120, 120)));
        ui.label(egui::RichText::new("• Drag to move")
            .size(10.0)
            .color(egui::Color32::from_rgb(120, 120, 120)));
        ui.label(egui::RichText::new("• Right-click → Connect")
            .size(10.0)
            .color(egui::Color32::from_rgb(120, 120, 120)));
    }

    /// Render workflow canvas (center panel when Workflow is active)
    fn render_workflow_canvas(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(30, 30, 30))
                    .inner_margin(0.0)
            )
            .show(ctx, |ui| {
                let Some(workflow_idx) = self.selected_workflow_idx else {
                    // No workflow selected - show selection UI
                    ui.centered_and_justified(|ui| {
                        ui.label(egui::RichText::new("← Select a workflow from the left panel")
                            .size(16.0)
                            .color(egui::Color32::from_rgb(150, 150, 150)));
                    });
                    return;
                };

                // Clone workflow data to avoid borrow conflicts
                let (workflow_name, mut workflow_nodes, mut workflow_connections) = {
                    let workflow = match self.workflows.get(workflow_idx) {
                        Some(w) => w,
                        None => return,
                    };
                    (workflow.name.clone(), workflow.nodes.clone(), workflow.connections.clone())
                };

                // Track modifications
                let mut add_node: Option<(String, String, egui::Pos2)> = None;
                let mut run_workflow = false;
                let mut clear_logs = false;

                // Top toolbar
                egui::TopBottomPanel::top("workflow_toolbar")
                    .frame(egui::Frame::none()
                        .fill(egui::Color32::from_rgb(40, 40, 40))
                        .inner_margin(8.0))
                    .show_inside(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(&workflow_name).strong().size(16.0));
                            ui.separator();

                            // Add Node button
                            if ui.button("➕ Command Node").clicked() {
                                let node_id = format!("node_{}", workflow_nodes.len());
                                let position = egui::pos2(100.0 + workflow_nodes.len() as f32 * 150.0, 200.0);
                                add_node = Some((node_id, format!("Node {}", workflow_nodes.len() + 1), position));
                            }

                            ui.separator();

                            // Run button
                            let run_text = if self.workflow_running { "⏸ Stop" } else { "▶️ Run" };
                            if ui.button(run_text).clicked() {
                                run_workflow = true;
                            }

                            // Clear logs button
                            if ui.button("🗑 Clear Logs").clicked() {
                                clear_logs = true;
                            }

                            ui.separator();

                            // Connection mode indicator
                            if self.dragging_from_port.is_some() {
                                ui.label(egui::RichText::new("🔗 Connecting... (drag to input port or release to cancel)")
                                    .color(egui::Color32::from_rgb(100, 150, 255))
                                    .strong());
                            }
                        });
                    });

                // Handle toolbar actions
                if let Some((node_id, node_name, position)) = add_node {
                    workflow_nodes.push(WorkflowNode::new(
                        node_id.clone(),
                        node_name,
                        WorkflowNodeType::Command,
                        position,
                    ));
                    self.selected_node_id = Some(node_id);
                }

                if run_workflow {
                    if !self.workflow_running {
                        self.run_workflow_visual(workflow_idx);
                    } else {
                        self.workflow_running = false;
                    }
                }

                if clear_logs {
                    self.workflow_logs.clear();
                }

                // Canvas area
                let canvas_response = ui.allocate_response(
                    ui.available_size(),
                    egui::Sense::click_and_drag()
                );

                let painter = ui.painter_at(canvas_response.rect);
                let canvas_rect = canvas_response.rect;

                // Draw grid
                self.draw_grid(&painter, canvas_rect);

                // Handle canvas panning - only if not dragging node or port
                if canvas_response.dragged() && self.dragging_node_id.is_none() && self.dragging_from_port.is_none() {
                    self.workflow_canvas_offset += canvas_response.drag_delta();
                }

                // Update drag connection preview position
                if self.dragging_from_port.is_some() {
                    if let Some(pointer_pos) = ui.input(|i| i.pointer.hover_pos()) {
                        self.drag_connection_end = Some(pointer_pos);
                    }
                }

                // Draw connection preview while dragging from a port
                if let Some((from_node_id, PortType::Output)) = &self.dragging_from_port {
                    if let Some(from_node) = workflow_nodes.iter().find(|n| &n.id == from_node_id) {
                        if let Some(end_pos) = self.drag_connection_end {
                            let from_pos = canvas_rect.min + from_node.position.to_vec2() + self.workflow_canvas_offset + egui::vec2(120.0, 30.0);

                            // Bezier curve preview
                            let ctrl1 = from_pos + egui::vec2(50.0, 0.0);
                            let ctrl2 = end_pos - egui::vec2(50.0, 0.0);

                            painter.add(egui::Shape::CubicBezier(egui::epaint::CubicBezierShape {
                                points: [from_pos, ctrl1, ctrl2, end_pos],
                                closed: false,
                                fill: egui::Color32::TRANSPARENT,
                                stroke: egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 150, 255)).into(),
                            }));
                        }
                    }
                }

                // Draw connections
                for conn in &workflow_connections {
                    if let (Some(from_node), Some(to_node)) = (
                        workflow_nodes.iter().find(|n| n.id == conn.from_node_id),
                        workflow_nodes.iter().find(|n| n.id == conn.to_node_id),
                    ) {
                        self.draw_connection(&painter, canvas_rect, from_node, to_node, conn);
                    }
                }

                // Draw nodes
                let mut node_to_delete: Option<String> = None;
                let nodes_to_draw: Vec<_> = workflow_nodes.clone();

                for node in nodes_to_draw {
                    let node_response = self.draw_node(&painter, ui, canvas_rect, &node);

                    // Get mouse pointer position
                    let pointer_pos = ui.input(|i| i.pointer.hover_pos());

                    // Check if pointer is over a port
                    let over_input_port = pointer_pos.map_or(false, |p| self.is_point_in_port(canvas_rect, node.position, PortType::Input, p));
                    let over_output_port = pointer_pos.map_or(false, |p| self.is_point_in_port(canvas_rect, node.position, PortType::Output, p));

                    // Priority 1: Port interaction (drag from output port to create connection)
                    if over_output_port && node_response.drag_started() {
                        self.dragging_from_port = Some((node.id.clone(), PortType::Output));
                        tracing::info!("🔗 Started dragging from output port: {}", node.id);
                        continue; // Skip other interactions
                    }

                    // Priority 2: Drop on input port to complete connection
                    if over_input_port {
                        if let Some((from_node_id, PortType::Output)) = &self.dragging_from_port {
                            if ui.input(|i| i.pointer.any_released()) && from_node_id != &node.id {
                                let from_node_id_clone = from_node_id.clone();
                                workflow_connections.push(WorkflowConnection {
                                    from_node_id: from_node_id_clone.clone(),
                                    to_node_id: node.id.clone(),
                                    label: None,
                                });
                                self.dragging_from_port = None;
                                self.drag_connection_end = None;
                                tracing::info!("🔗 Connected: {} -> {}", from_node_id_clone, node.id);
                                continue; // Skip other interactions
                            }
                        }
                    }

                    // Cancel connection on release outside any input port
                    if self.dragging_from_port.is_some() && ui.input(|i| i.pointer.any_released()) {
                        self.dragging_from_port = None;
                        self.drag_connection_end = None;
                    }

                    // Priority 3: Drag node (only if not over a port)
                    if node_response.drag_started() && !over_input_port && !over_output_port {
                        self.dragging_node_id = Some(node.id.clone());
                    }

                    if node_response.dragged() && self.dragging_node_id == Some(node.id.clone()) {
                        // Update node position
                        if let Some(workflow_node) = workflow_nodes.iter_mut().find(|n| n.id == node.id) {
                            workflow_node.position += node_response.drag_delta();
                        }
                    }

                    if node_response.drag_stopped() {
                        self.dragging_node_id = None;
                    }

                    // Priority 4: Double-click to edit
                    if node_response.double_clicked() {
                        self.selected_node_id = Some(node.id.clone());
                        self.editing_node_id = Some(node.id.clone());
                        self.node_editor_open = true;
                        self.new_node_name = node.name.clone();
                        self.new_node_command = node.command.clone();
                    }

                    // Priority 5: Right-click menu
                    node_response.context_menu(|ui| {
                        if ui.button("✏️ Edit").clicked() {
                            self.editing_node_id = Some(node.id.clone());
                            self.node_editor_open = true;
                            self.new_node_name = node.name.clone();
                            self.new_node_command = node.command.clone();
                            ui.close_menu();
                        }
                        if ui.button("🗑 Delete").clicked() {
                            node_to_delete = Some(node.id.clone());
                            ui.close_menu();
                        }
                    });
                }

                // Delete node if requested
                if let Some(node_id) = node_to_delete {
                    workflow_nodes.retain(|n| n.id != node_id);
                    workflow_connections.retain(|c| c.from_node_id != node_id && c.to_node_id != node_id);
                    if self.selected_node_id.as_ref() == Some(&node_id) {
                        self.selected_node_id = None;
                    }
                }

                // Cancel connection on ESC
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    self.dragging_from_port = None;
                    self.drag_connection_end = None;
                }

                // Write back modified data to workflow
                if let Some(workflow) = self.workflows.get_mut(workflow_idx) {
                    workflow.nodes = workflow_nodes;
                    workflow.connections = workflow_connections;
                }
            });

        // Node editor dialog
        if self.node_editor_open {
            egui::Window::new("✏️ Edit Node")
                .default_width(450.0)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Node Name:");
                        ui.text_edit_singleline(&mut self.new_node_name);
                    });

                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        ui.label("Command:");
                        ui.text_edit_singleline(&mut self.new_node_command);
                    });

                    ui.add_space(4.0);

                    ui.label(egui::RichText::new("Examples:")
                        .size(11.0)
                        .color(egui::Color32::from_rgb(150, 150, 150)));
                    ui.label(egui::RichText::new("  cargo build")
                        .monospace()
                        .size(10.0)
                        .color(egui::Color32::from_rgb(120, 120, 120)));
                    ui.label(egui::RichText::new("  npm test")
                        .monospace()
                        .size(10.0)
                        .color(egui::Color32::from_rgb(120, 120, 120)));
                    ui.label(egui::RichText::new("  git push")
                        .monospace()
                        .size(10.0)
                        .color(egui::Color32::from_rgb(120, 120, 120)));

                    ui.separator();

                    ui.horizontal(|ui| {
                        if ui.button("💾 Save").clicked() {
                            if let Some(node_id) = &self.editing_node_id {
                                if let Some(workflow) = self.workflows.get_mut(self.selected_workflow_idx.unwrap()) {
                                    if let Some(node) = workflow.nodes.iter_mut().find(|n| n.id == *node_id) {
                                        node.name = self.new_node_name.clone();
                                        node.command = self.new_node_command.clone();
                                    }
                                }
                            }
                            self.node_editor_open = false;
                            self.editing_node_id = None;
                        }

                        if ui.button("❌ Cancel").clicked() {
                            self.node_editor_open = false;
                            self.editing_node_id = None;
                        }
                    });
                });
        }

        // Workflow rename dialog
        if self.workflow_editor_open {
            egui::Window::new("✏️ Rename Workflow")
                .default_width(400.0)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Workflow Name:");
                        ui.text_edit_singleline(&mut self.new_workflow_name);
                    });

                    ui.separator();

                    ui.horizontal(|ui| {
                        if ui.button("💾 Save").clicked() {
                            if let Some(idx) = self.selected_workflow_idx {
                                if let Some(workflow) = self.workflows.get_mut(idx) {
                                    workflow.name = self.new_workflow_name.clone();
                                }
                            }
                            self.workflow_editor_open = false;
                            self.new_workflow_name.clear();
                        }

                        if ui.button("❌ Cancel").clicked() {
                            self.workflow_editor_open = false;
                            self.new_workflow_name.clear();
                        }
                    });
                });
        }
    }

    /// Draw grid on canvas
    fn draw_grid(&self, painter: &egui::Painter, rect: egui::Rect) {
        let grid_spacing = 50.0;
        let grid_color = egui::Color32::from_rgb(45, 45, 45);

        // Vertical lines
        let mut x = (rect.min.x - self.workflow_canvas_offset.x) % grid_spacing;
        while x < rect.max.x {
            painter.vline(x, rect.y_range(), egui::Stroke::new(1.0, grid_color));
            x += grid_spacing;
        }

        // Horizontal lines
        let mut y = (rect.min.y - self.workflow_canvas_offset.y) % grid_spacing;
        while y < rect.max.y {
            painter.hline(rect.x_range(), y, egui::Stroke::new(1.0, grid_color));
            y += grid_spacing;
        }
    }

    /// Draw a workflow node with input/output ports
    fn draw_node(&self, painter: &egui::Painter, ui: &mut egui::Ui, canvas_rect: egui::Rect, node: &WorkflowNode) -> egui::Response {
        let node_size = egui::vec2(120.0, 60.0);
        let pos = canvas_rect.min + node.position.to_vec2() + self.workflow_canvas_offset;
        let node_rect = egui::Rect::from_min_size(pos, node_size);

        // Node color based on type
        let node_color = match node.node_type {
            WorkflowNodeType::Start => egui::Color32::from_rgb(80, 150, 80),
            WorkflowNodeType::Command => egui::Color32::from_rgb(80, 120, 200),
            WorkflowNodeType::Condition => egui::Color32::from_rgb(200, 150, 80),
            WorkflowNodeType::End => egui::Color32::from_rgb(180, 80, 80),
        };

        // Draw node background
        painter.rect_filled(node_rect, 8.0, node_color);

        // Draw node border
        let (border_color, border_width) = if self.selected_node_id.as_ref() == Some(&node.id) {
            // Selected node
            (egui::Color32::from_rgb(255, 255, 255), 2.0)
        } else {
            // Default
            (egui::Color32::from_rgb(100, 100, 100), 2.0)
        };
        painter.rect_stroke(node_rect, 8.0, egui::Stroke::new(border_width, border_color));

        // Draw node text
        let text_pos = pos + egui::vec2(10.0, 10.0);
        painter.text(
            text_pos,
            egui::Align2::LEFT_TOP,
            &node.name,
            egui::FontId::proportional(12.0),
            egui::Color32::WHITE,
        );

        // Command preview (truncated)
        if !node.command.is_empty() {
            let cmd_preview = if node.command.len() > 15 {
                format!("{}...", &node.command[..15])
            } else {
                node.command.clone()
            };
            let cmd_pos = pos + egui::vec2(10.0, 30.0);
            painter.text(
                cmd_pos,
                egui::Align2::LEFT_TOP,
                cmd_preview,
                egui::FontId::monospace(9.0),
                egui::Color32::from_rgb(200, 200, 200),
            );
        }

        // Draw input port (left side, center)
        let input_port_pos = pos + egui::vec2(0.0, 30.0);
        let input_port_radius = 6.0;
        painter.circle_filled(input_port_pos, input_port_radius, egui::Color32::from_rgb(100, 200, 100));
        painter.circle_stroke(input_port_pos, input_port_radius, egui::Stroke::new(2.0, egui::Color32::WHITE));

        // Draw output port (right side, center)
        let output_port_pos = pos + egui::vec2(120.0, 30.0);
        let output_port_radius = 6.0;
        painter.circle_filled(output_port_pos, output_port_radius, egui::Color32::from_rgb(255, 150, 100));
        painter.circle_stroke(output_port_pos, output_port_radius, egui::Stroke::new(2.0, egui::Color32::WHITE));

        ui.interact(node_rect, egui::Id::new(&node.id), egui::Sense::click_and_drag())
    }

    /// Draw connection between two nodes
    fn draw_connection(&self, painter: &egui::Painter, canvas_rect: egui::Rect, from: &WorkflowNode, to: &WorkflowNode, _conn: &WorkflowConnection) {
        let from_pos = canvas_rect.min + from.position.to_vec2() + self.workflow_canvas_offset + egui::vec2(120.0, 30.0);
        let to_pos = canvas_rect.min + to.position.to_vec2() + self.workflow_canvas_offset + egui::vec2(0.0, 30.0);

        // Bezier curve for smooth connection
        let ctrl1 = from_pos + egui::vec2(50.0, 0.0);
        let ctrl2 = to_pos - egui::vec2(50.0, 0.0);

        painter.add(egui::Shape::CubicBezier(egui::epaint::CubicBezierShape {
            points: [from_pos, ctrl1, ctrl2, to_pos],
            closed: false,
            fill: egui::Color32::TRANSPARENT,
            stroke: egui::Stroke::new(2.0, egui::Color32::from_rgb(150, 150, 150)).into(),
        }));

        // Arrow head
        let arrow_dir = (to_pos - ctrl2).normalized();
        let arrow_size = 8.0;
        let arrow_p1 = to_pos - arrow_dir * arrow_size + arrow_dir.rot90() * (arrow_size * 0.5);
        let arrow_p2 = to_pos - arrow_dir * arrow_size - arrow_dir.rot90() * (arrow_size * 0.5);
        painter.add(egui::Shape::convex_polygon(
            vec![to_pos, arrow_p1, arrow_p2],
            egui::Color32::from_rgb(150, 150, 150),
            egui::Stroke::NONE,
        ));
    }

    /// Check if a point is within a port's clickable area
    fn is_point_in_port(&self, canvas_rect: egui::Rect, node_pos: egui::Pos2, port_type: PortType, point: egui::Pos2) -> bool {
        let port_offset = match port_type {
            PortType::Input => egui::vec2(0.0, 30.0),
            PortType::Output => egui::vec2(120.0, 30.0),
        };
        let port_pos = canvas_rect.min + node_pos.to_vec2() + self.workflow_canvas_offset + port_offset;
        let port_radius = 8.0; // Slightly larger than visual radius for easier clicking
        point.distance(port_pos) <= port_radius
    }

    /// Render workflow logs panel (right side when Workflow is active)
    fn render_workflow_logs_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("workflow_logs")
            .default_width(400.0)
            .width_range(200.0..=800.0)
            .resizable(true)
            .frame(
                egui::Frame::none()
                    .fill(ui_colors::SIDEBAR_BG)
                    .inner_margin(8.0)
            )
            .show(ctx, |ui| {
                ui.heading("📋 Workflow Logs");
                ui.separator();

                egui::ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for log in &self.workflow_logs {
                            let color = match log.log_type {
                                WorkflowLogType::Info => egui::Color32::from_rgb(200, 200, 200),
                                WorkflowLogType::Success => egui::Color32::from_rgb(80, 200, 120),
                                WorkflowLogType::Error => egui::Color32::from_rgb(255, 100, 100),
                                WorkflowLogType::Warning => egui::Color32::from_rgb(255, 200, 100),
                            };

                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(&log.timestamp).size(9.0).color(egui::Color32::GRAY));
                                ui.label(egui::RichText::new(format!("[{}]", log.node_name)).size(10.0).monospace().color(egui::Color32::from_rgb(150, 150, 200)));
                                ui.label(egui::RichText::new(&log.message).color(color));
                            });
                        }
                    });
            });
    }

    /// Run a workflow (visual node-based execution)
    fn run_workflow_visual(&mut self, idx: usize) {
        if let Some(workflow) = self.workflows.get(idx) {
            self.workflow_running = true;
            self.workflow_logs.clear();

            tracing::info!("▶️ Running workflow: {}", workflow.name);

            self.workflow_logs.push(WorkflowLogEntry {
                timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                node_id: "system".to_string(),
                node_name: "System".to_string(),
                message: format!("Starting workflow: {}", workflow.name),
                log_type: WorkflowLogType::Info,
            });

            // Execute nodes in topological order (simplified: just iterate)
            for node in &workflow.nodes {
                if !node.enabled || node.command.is_empty() {
                    continue;
                }

                self.workflow_logs.push(WorkflowLogEntry {
                    timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                    node_id: node.id.clone(),
                    node_name: node.name.clone(),
                    message: format!("Executing: {}", node.command),
                    log_type: WorkflowLogType::Info,
                });

                let working_dir = node.working_dir.as_ref().unwrap_or(&self.terminal_working_dir);
                match native::terminal::execute_command(&node.command, working_dir) {
                    Ok(output) => {
                        for line in output.lines().take(10) {
                            self.workflow_logs.push(WorkflowLogEntry {
                                timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                                node_id: node.id.clone(),
                                node_name: node.name.clone(),
                                message: line.to_string(),
                                log_type: WorkflowLogType::Info,
                            });
                        }
                        self.workflow_logs.push(WorkflowLogEntry {
                            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                            node_id: node.id.clone(),
                            node_name: node.name.clone(),
                            message: "✓ Completed".to_string(),
                            log_type: WorkflowLogType::Success,
                        });
                    }
                    Err(e) => {
                        self.workflow_logs.push(WorkflowLogEntry {
                            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                            node_id: node.id.clone(),
                            node_name: node.name.clone(),
                            message: format!("✗ Error: {}", e),
                            log_type: WorkflowLogType::Error,
                        });
                        break;
                    }
                }
            }

            self.workflow_logs.push(WorkflowLogEntry {
                timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                node_id: "system".to_string(),
                node_name: "System".to_string(),
                message: "Workflow completed".to_string(),
                log_type: WorkflowLogType::Success,
            });

            self.workflow_running = false;
        }
    }

    /// Render Search panel (Phase 5.2: project-wide search)
    fn render_search_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("🔍 Search in Files");
        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Search:");
            let response = ui.text_edit_singleline(&mut self.search_query);

            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.perform_project_search();
            }

            if ui.button("Go").clicked() {
                self.perform_project_search();
            }
        });

        ui.checkbox(&mut self.search_case_sensitive, "Case sensitive");

        ui.separator();

        // Display search results
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
            if !self.search_results.is_empty() {
                ui.label(format!("Found {} matches:", self.search_results.len()));
                ui.add_space(4.0);

                // Clone results to avoid borrowing issues
                let results = self.search_results.clone();
                for (idx, result) in results.iter().enumerate() {
                    let is_selected = idx == self.current_search_index;

                    // Prepare display text and file path outside closure
                    let display_text = if let Some(ref file_path) = result.file_path {
                        // Extract filename from path
                        let filename = std::path::Path::new(file_path)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or(file_path);
                        format!("{}:{} - {}", filename, result.line_number + 1, result.line_text.trim())
                    } else {
                        // In-file search, just show line number
                        format!("Line {}: {}", result.line_number + 1, result.line_text.trim())
                    };
                    let file_path_clone = result.file_path.clone();

                    ui.horizontal(|ui| {
                        if ui.selectable_label(is_selected, display_text).clicked() {
                            self.current_search_index = idx;

                            // If clicking on a project-wide search result, open the file
                            if let Some(file_path) = file_path_clone {
                                self.open_file_from_path(&file_path);
                            }
                            // TODO: Jump to line in editor
                        }
                    });
                }
            } else if !self.search_query.is_empty() {
                ui.label("No results found");
            }
        });
    }

    /// Render Git panel (Phase 5.3)
    fn render_git_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("🔀 Git");
        ui.separator();

        // Tab bar
        self.render_git_tab_bar(ui);

        ui.separator();

        // Render the active tab
        match self.git_active_tab {
            GitTab::Status => self.render_git_status_tab(ui),
            GitTab::History => self.render_git_history_tab(ui),
            GitTab::Branches => self.render_git_branches_tab(ui),
            GitTab::Remotes => self.render_git_remotes_tab(ui),
            GitTab::Tags => self.render_git_tags_tab(ui),
            GitTab::Stash => self.render_git_stash_tab(ui),
        }
    }

    /// Render Git tab bar (6 tabs)
    fn render_git_tab_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.git_active_tab, GitTab::Status, "Status");
            ui.selectable_value(&mut self.git_active_tab, GitTab::History, "History");
            ui.selectable_value(&mut self.git_active_tab, GitTab::Branches, "Branches");
            ui.selectable_value(&mut self.git_active_tab, GitTab::Remotes, "Remotes");
            ui.selectable_value(&mut self.git_active_tab, GitTab::Tags, "Tags");
            ui.selectable_value(&mut self.git_active_tab, GitTab::Stash, "Stash");
        });
    }

    /// Render Status tab (existing functionality with grouping)
    fn render_git_status_tab(&mut self, ui: &mut egui::Ui) {
        // Refresh button and branch info
        ui.horizontal(|ui| {
            if ui.button("🔄 Refresh").clicked() {
                self.refresh_git_status();
            }
            ui.label(format!("Branch: {}", self.git_current_branch));
        });

        ui.add_space(8.0);

        // Commit message input
        ui.horizontal(|ui| {
            ui.label("Message:");
            ui.text_edit_singleline(&mut self.git_commit_message);
        });

        ui.horizontal(|ui| {
            if ui.button("✅ Commit").clicked() {
                self.perform_git_commit();
            }

            if ui.button("➕ Stage All").clicked() {
                self.perform_git_stage_all();
            }
        });

        ui.separator();

        // Changed files list with grouping
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
            if self.git_status.is_empty() {
                ui.label("No changes");
            } else {
                // Group files by staged/unstaged
                let git_statuses = self.git_status.clone();
                let staged: Vec<_> = git_statuses.iter().filter(|s| s.is_staged).collect();
                let unstaged: Vec<_> = git_statuses.iter().filter(|s| !s.is_staged).collect();

                // Staged changes
                if !staged.is_empty() {
                    ui.heading(format!("Staged Changes ({})", staged.len()));
                    ui.add_space(4.0);
                    for status in staged {
                        self.render_file_status_row(ui, status);
                    }
                    ui.add_space(8.0);
                }

                // Unstaged changes
                if !unstaged.is_empty() {
                    ui.heading(format!("Unstaged Changes ({})", unstaged.len()));
                    ui.add_space(4.0);
                    for status in unstaged {
                        self.render_file_status_row(ui, status);
                    }
                }
            }
        });
    }

    /// Helper function to render a file status row
    fn render_file_status_row(&mut self, ui: &mut egui::Ui, status: &native::git::GitStatus) {
        let (icon, color) = match status.status.as_str() {
            "modified" => ("📝", egui::Color32::from_rgb(255, 198, 109)),
            "added" => ("➕", egui::Color32::from_rgb(106, 180, 89)),
            "deleted" => ("🗑️", egui::Color32::from_rgb(255, 100, 100)),
            _ => ("❓", egui::Color32::LIGHT_GRAY),
        };
        let is_staged = status.is_staged;
        let path = status.path.clone();

        ui.horizontal(|ui| {
            ui.colored_label(color, icon);

            // File path - click to show diff
            if ui.button(&path).clicked() {
                self.load_git_diff(&path);
            }

            // Stage/Unstage button
            if is_staged {
                if ui.small_button("Unstage").clicked() {
                    self.perform_git_unstage(&path);
                }
            } else {
                if ui.small_button("Stage").clicked() {
                    self.perform_git_stage(&path);
                }
            }
        });
    }

    /// Load git diff for a file and display in center panel
    fn load_git_diff(&mut self, file_path: &str) {
        tracing::info!("🔍 Loading diff for: {}", file_path);
        match native::git::get_diff(&self.root_path, file_path) {
            Ok(diff) => {
                self.git_diff_state.selected_file = Some(file_path.to_string());
                self.git_diff_state.diff = Some(diff);
                tracing::info!("✅ Diff loaded for: {}", file_path);
            }
            Err(e) => {
                tracing::error!("❌ Failed to load diff for {}: {}", file_path, e);
                self.git_diff_state.selected_file = None;
                self.git_diff_state.diff = None;
            }
        }
    }

    /// Render History tab (commit graph)
    fn render_git_history_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("🔄 Refresh").clicked() {
                self.refresh_git_history();
            }

            ui.checkbox(&mut self.git_history_state.show_all_branches, "All branches");

            if ui.button("Load More").clicked() {
                self.git_history_state.page_limit += 100;
                self.refresh_git_history();
            }
        });

        ui.separator();

        // Filter inputs
        ui.horizontal(|ui| {
            ui.label("Author:");
            ui.text_edit_singleline(&mut self.git_history_state.filter_author);
            ui.label("Message:");
            ui.text_edit_singleline(&mut self.git_history_state.filter_message);
        });

        ui.separator();

        // 3-pane layout: Graph | Commit List | Details
        ui.horizontal(|ui| {
            // Left: Commit graph and list (60% width)
            ui.vertical(|ui| {
                ui.set_width(ui.available_width() * 0.6);

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if self.git_history_state.graph_nodes.is_empty() {
                            ui.label("No commits. Click Refresh to load.");
                        } else {
                            self.render_commit_graph(ui);
                        }
                    });
            });

            ui.separator();

            // Right: Commit details (40% width)
            ui.vertical(|ui| {
                ui.set_width(ui.available_width());
                self.render_commit_details(ui);
            });
        });
    }

    /// Render commit graph with egui painter
    fn render_commit_graph(&mut self, ui: &mut egui::Ui) {
        let nodes = self.git_history_state.graph_nodes.clone();
        let selected_id = self.git_history_state.selected_commit_id.clone();

        const NODE_RADIUS: f32 = 4.0;
        const COLUMN_WIDTH: f32 = 16.0;
        const ROW_HEIGHT: f32 = 24.0;

        // 8-color palette for branches
        let colors = [
            egui::Color32::from_rgb(106, 180, 89),   // Green
            egui::Color32::from_rgb(100, 181, 246),  // Blue
            egui::Color32::from_rgb(255, 198, 109),  // Yellow
            egui::Color32::from_rgb(239, 83, 80),    // Red
            egui::Color32::from_rgb(171, 128, 255),  // Purple
            egui::Color32::from_rgb(255, 138, 128),  // Coral
            egui::Color32::from_rgb(128, 222, 234),  // Cyan
            egui::Color32::from_rgb(255, 171, 64),   // Orange
        ];

        for (idx, node) in nodes.iter().enumerate() {
            let y_offset = idx as f32 * ROW_HEIGHT;

            ui.horizontal(|ui| {
                // Graph column (left side)
                let (graph_rect, graph_response) = ui.allocate_exact_size(
                    egui::vec2(COLUMN_WIDTH * 8.0, ROW_HEIGHT),
                    egui::Sense::click(),
                );

                if ui.is_rect_visible(graph_rect) {
                    let painter = ui.painter();

                    // Draw graph lines
                    for line in &node.graph_lines {
                        let from_pos = graph_rect.min + egui::vec2(
                            line.from_column as f32 * COLUMN_WIDTH + COLUMN_WIDTH / 2.0,
                            NODE_RADIUS,
                        );
                        let to_pos = graph_rect.min + egui::vec2(
                            line.to_column as f32 * COLUMN_WIDTH + COLUMN_WIDTH / 2.0,
                            ROW_HEIGHT,
                        );

                        let color = colors[line.color_index % colors.len()];

                        if line.line_type == native::git::GraphLineType::Direct {
                            // Straight line
                            painter.line_segment([from_pos, to_pos], egui::Stroke::new(2.0, color));
                        } else {
                            // Bezier curve for merge
                            painter.add(egui::Shape::CubicBezier(egui::epaint::CubicBezierShape::from_points_stroke(
                                [
                                    from_pos,
                                    from_pos + egui::vec2(0.0, ROW_HEIGHT * 0.3),
                                    to_pos - egui::vec2(0.0, ROW_HEIGHT * 0.3),
                                    to_pos,
                                ],
                                false,
                                egui::Color32::TRANSPARENT,
                                egui::Stroke::new(2.0, color),
                            )));
                        }
                    }

                    // Draw node circle
                    let node_pos = graph_rect.min + egui::vec2(
                        node.graph_column as f32 * COLUMN_WIDTH + COLUMN_WIDTH / 2.0,
                        NODE_RADIUS,
                    );
                    let node_color = colors[node.graph_column % colors.len()];
                    painter.circle_filled(node_pos, NODE_RADIUS, node_color);
                }

                // Commit info (right side)
                let is_selected = selected_id.as_ref() == Some(&node.commit.id);
                let text_color = if is_selected {
                    egui::Color32::from_rgb(0xAB, 0xB2, 0xBF)
                } else {
                    egui::Color32::from_rgb(180, 180, 180)
                };

                if ui.add(egui::Button::new(&node.commit.message).fill(
                    if is_selected {
                        egui::Color32::from_rgb(60, 60, 80)
                    } else {
                        egui::Color32::TRANSPARENT
                    }
                )).clicked() {
                    self.git_history_state.selected_commit_id = Some(node.commit.id.clone());
                    // Load commit details
                    if let Ok(detail) = native::git::get_commit_detail(&self.root_path, &node.commit.id) {
                        self.git_history_state.commit_details = Some(detail);
                    }
                }

                ui.colored_label(egui::Color32::GRAY, &node.commit.author);

                // Branch/tag badges
                for branch_name in &node.branch_names {
                    ui.colored_label(egui::Color32::from_rgb(106, 180, 89), format!(" [{}]", branch_name));
                }
                for tag_name in &node.tag_names {
                    ui.colored_label(egui::Color32::from_rgb(255, 198, 109), format!(" 🏷{}", tag_name));
                }
            });
        }
    }

    /// Render commit details panel
    fn render_commit_details(&mut self, ui: &mut egui::Ui) {
        if let Some(detail) = &self.git_history_state.commit_details {
            ui.heading("Commit Details");
            ui.separator();

            ui.label(format!("ID: {}", detail.commit.id));
            ui.label(format!("Author: {}", detail.commit.author));
            ui.label(format!("Date: {}", Self::format_timestamp(detail.commit.date)));
            ui.label(format!("Message: {}", detail.commit.message));

            ui.add_space(8.0);
            ui.label(format!("Stats: +{} -{}", detail.total_additions, detail.total_deletions));

            ui.separator();
            ui.heading(format!("Changed Files ({})", detail.changed_files.len()));

            egui::ScrollArea::vertical().show(ui, |ui| {
                for file in &detail.changed_files {
                    let (icon, color) = match file.status.as_str() {
                        "added" => ("➕", egui::Color32::from_rgb(106, 180, 89)),
                        "modified" => ("📝", egui::Color32::from_rgb(255, 198, 109)),
                        "deleted" => ("🗑️", egui::Color32::from_rgb(255, 100, 100)),
                        _ => ("❓", egui::Color32::GRAY),
                    };

                    ui.horizontal(|ui| {
                        ui.colored_label(color, icon);
                        ui.label(&file.path);
                        ui.colored_label(egui::Color32::GREEN, format!("+{}", file.additions));
                        ui.colored_label(egui::Color32::RED, format!("-{}", file.deletions));
                    });
                }
            });
        } else {
            ui.label("Select a commit to view details");
        }
    }

    /// Render Branches tab
    fn render_git_branches_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("🔄 Refresh").clicked() {
                self.refresh_git_branches();
            }
        });

        ui.separator();

        // Create new branch
        ui.horizontal(|ui| {
            ui.label("New branch:");
            ui.text_edit_singleline(&mut self.git_branch_state.new_branch_name);
            if ui.button("➕ Create").clicked() && !self.git_branch_state.new_branch_name.is_empty() {
                self.perform_create_branch();
            }
        });

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            // Local branches
            ui.heading(format!("Local Branches ({})", self.git_branch_state.local_branches.len()));
            ui.add_space(4.0);

            let local_branches = self.git_branch_state.local_branches.clone();
            for branch in &local_branches {
                ui.horizontal(|ui| {
                    let icon = if branch.is_current { "✓" } else { " " };
                    let color = if branch.is_current {
                        egui::Color32::from_rgb(106, 180, 89)
                    } else {
                        egui::Color32::LIGHT_GRAY
                    };

                    ui.colored_label(color, icon);
                    ui.label(&branch.name);

                    if !branch.is_current {
                        if ui.small_button("Checkout").clicked() {
                            self.perform_checkout_branch(&branch.name);
                        }
                        if ui.small_button("Merge").clicked() {
                            self.perform_merge_branch(&branch.name);
                        }
                        if ui.small_button("Delete").clicked() {
                            self.perform_delete_branch(&branch.name);
                        }
                    }
                });
            }

            ui.add_space(8.0);

            // Remote branches
            ui.heading(format!("Remote Branches ({})", self.git_branch_state.remote_branches.len()));
            ui.add_space(4.0);

            let remote_branches = self.git_branch_state.remote_branches.clone();
            for branch in &remote_branches {
                ui.horizontal(|ui| {
                    ui.colored_label(egui::Color32::from_rgb(100, 181, 246), "📡");
                    ui.label(&branch.name);
                });
            }
        });
    }

    /// Render Remotes tab
    fn render_git_remotes_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("🔄 Refresh").clicked() {
                self.refresh_git_remotes();
            }
        });

        ui.separator();

        // Add new remote
        ui.horizontal(|ui| {
            ui.label("Name:");
            ui.text_edit_singleline(&mut self.git_remote_state.new_remote_name);
            ui.label("URL:");
            ui.text_edit_singleline(&mut self.git_remote_state.new_remote_url);
            if ui.button("➕ Add").clicked() && !self.git_remote_state.new_remote_name.is_empty() {
                self.perform_add_remote();
            }
        });

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            if self.git_remote_state.remotes.is_empty() {
                ui.label("No remotes configured");
            } else {
                let remotes = self.git_remote_state.remotes.clone();
                for remote in &remotes {
                    ui.group(|ui| {
                        ui.heading(&remote.name);
                        ui.label(format!("Fetch URL: {}", remote.fetch_url));
                        ui.label(format!("Push URL: {}", remote.push_url));

                        ui.horizontal(|ui| {
                            if ui.button("Fetch").clicked() {
                                self.perform_fetch(&remote.name);
                            }
                            if ui.button("Pull").clicked() {
                                self.perform_pull(&remote.name);
                            }
                            if ui.button("Push").clicked() {
                                self.perform_push(&remote.name);
                            }
                            if ui.button("Remove").clicked() {
                                self.perform_remove_remote(&remote.name);
                            }
                        });
                    });
                    ui.add_space(8.0);
                }
            }
        });
    }

    /// Render Tags tab
    fn render_git_tags_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("🔄 Refresh").clicked() {
                self.refresh_git_tags();
            }
        });

        ui.separator();

        // Create new tag
        ui.horizontal(|ui| {
            ui.label("Tag name:");
            ui.text_edit_singleline(&mut self.git_tag_state.new_tag_name);
            ui.checkbox(&mut self.git_tag_state.annotated, "Annotated");
        });

        if self.git_tag_state.annotated {
            ui.horizontal(|ui| {
                ui.label("Message:");
                ui.text_edit_singleline(&mut self.git_tag_state.new_tag_message);
            });
        }

        if ui.button("➕ Create Tag").clicked() && !self.git_tag_state.new_tag_name.is_empty() {
            self.perform_create_tag();
        }

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            if self.git_tag_state.tags.is_empty() {
                ui.label("No tags");
            } else {
                ui.heading(format!("Tags ({})", self.git_tag_state.tags.len()));
                ui.add_space(4.0);

                let tags = self.git_tag_state.tags.clone();
                for tag in &tags {
                    ui.horizontal(|ui| {
                        ui.colored_label(egui::Color32::from_rgb(255, 198, 109), "🏷");
                        ui.label(&tag.name);

                        if tag.message.is_some() {
                            ui.colored_label(egui::Color32::GRAY, "(annotated)");
                        }

                        ui.label(format!("→ {}", &tag.commit_id[..7]));

                        if ui.small_button("Delete").clicked() {
                            self.perform_delete_tag(&tag.name);
                        }
                    });

                    if let Some(message) = &tag.message {
                        ui.label(format!("  {}", message));
                    }
                }
            }
        });
    }

    /// Render Stash tab
    fn render_git_stash_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("🔄 Refresh").clicked() {
                self.refresh_git_stashes();
            }
        });

        ui.separator();

        // Create new stash
        ui.horizontal(|ui| {
            ui.label("Message:");
            ui.text_edit_singleline(&mut self.git_stash_state.new_stash_message);
            ui.checkbox(&mut self.git_stash_state.include_untracked, "Include untracked");
        });

        if ui.button("💾 Save Stash").clicked() {
            self.perform_stash_save();
        }

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            if self.git_stash_state.stashes.is_empty() {
                ui.label("No stashes");
            } else {
                ui.heading(format!("Stashes ({})", self.git_stash_state.stashes.len()));
                ui.add_space(4.0);

                let stashes = self.git_stash_state.stashes.clone();
                for stash in &stashes {
                    ui.group(|ui| {
                        ui.heading(format!("stash@{{{}}}", stash.index));
                        ui.label(&stash.message);
                        ui.label(format!("Commit: {}", &stash.commit_id[..7]));

                        ui.horizontal(|ui| {
                            if ui.button("Apply").clicked() {
                                self.perform_stash_apply(stash.index);
                            }
                            if ui.button("Pop").clicked() {
                                self.perform_stash_pop(stash.index);
                            }
                            if ui.button("Drop").clicked() {
                                self.perform_stash_drop(stash.index);
                            }
                        });
                    });
                    ui.add_space(8.0);
                }
            }
        });
    }

    /// Helper function to format Unix timestamp
    fn format_timestamp(timestamp: u64) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let duration = std::time::Duration::from_secs(timestamp);
        let datetime = UNIX_EPOCH + duration;

        // Simple formatting (could use chrono crate for better formatting)
        if let Ok(elapsed) = SystemTime::now().duration_since(datetime) {
            let secs = elapsed.as_secs();
            if secs < 60 {
                return format!("{} seconds ago", secs);
            } else if secs < 3600 {
                return format!("{} minutes ago", secs / 60);
            } else if secs < 86400 {
                return format!("{} hours ago", secs / 3600);
            } else {
                return format!("{} days ago", secs / 86400);
            }
        }

        format!("Timestamp: {}", timestamp)
    }

    /// Render Wiki sidebar (page list)
    fn render_wiki_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.heading(format!("{} Wiki", "\u{ea88}")); // codicon-file-text (document)
        ui.separator();

        // Search box
        ui.horizontal(|ui| {
            ui.label("🔍");
            ui.text_edit_singleline(&mut self.wiki_search_query);
        });
        ui.add_space(4.0);

        // New Page button
        if ui.button("➕ New Page").clicked() {
            let new_page = WikiPage::new(format!("Untitled {}", self.wiki_pages.len() + 1));
            self.wiki_pages.push(new_page.clone());
            self.selected_wiki_page_id = Some(new_page.id);
            self.wiki_editing = true;
        }
        ui.add_space(8.0);

        // Page list
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let search_lower = self.wiki_search_query.to_lowercase();
                let mut page_to_delete: Option<String> = None;

                for page in &self.wiki_pages {
                    // Filter by search query
                    if !search_lower.is_empty() && !page.title.to_lowercase().contains(&search_lower) {
                        continue;
                    }

                    let is_selected = self.selected_wiki_page_id.as_ref() == Some(&page.id);

                    egui::Frame::none()
                        .fill(if is_selected {
                            egui::Color32::from_rgb(60, 80, 120)
                        } else {
                            egui::Color32::from_rgb(35, 35, 35)
                        })
                        .inner_margin(8.0)
                        .rounding(4.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                // Page title
                                let response = ui.selectable_label(is_selected, &page.title);
                                if response.clicked() {
                                    self.selected_wiki_page_id = Some(page.id.clone());
                                    self.wiki_editing = false;
                                }

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    // Delete button
                                    if ui.small_button("🗑").clicked() {
                                        page_to_delete = Some(page.id.clone());
                                    }
                                });
                            });

                            // Tags and timestamp
                            ui.horizontal(|ui| {
                                if !page.tags.is_empty() {
                                    ui.label(egui::RichText::new(format!("🏷 {}", page.tags.join(", ")))
                                        .size(9.0)
                                        .color(egui::Color32::from_rgb(120, 120, 120)));
                                }
                                ui.label(egui::RichText::new(format!("📅 {}", page.updated_at.format("%Y-%m-%d")))
                                    .size(9.0)
                                    .color(egui::Color32::from_rgb(100, 100, 100)));
                            });
                        });

                    ui.add_space(4.0);
                }

                // Handle deletion
                if let Some(page_id) = page_to_delete {
                    self.wiki_pages.retain(|p| p.id != page_id);
                    if self.selected_wiki_page_id.as_ref() == Some(&page_id) {
                        self.selected_wiki_page_id = None;
                    }
                }
            });
    }

    /// Render Wiki content (center panel)
    fn render_wiki_content(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(30, 30, 30))
                    .inner_margin(16.0)
            )
            .show(ctx, |ui| {
                let Some(page_id) = self.selected_wiki_page_id.clone() else {
                    // No page selected
                    ui.centered_and_justified(|ui| {
                        ui.label(egui::RichText::new("← Select a page from the left panel or create a new one")
                            .size(16.0)
                            .color(egui::Color32::from_rgb(150, 150, 150)));
                    });
                    return;
                };

                // Find the page (clone to avoid borrow issues)
                let page_opt = self.wiki_pages.iter().find(|p| p.id == page_id).cloned();
                let Some(mut page) = page_opt else {
                    ui.label("Page not found");
                    return;
                };

                // Top toolbar
                egui::TopBottomPanel::top("wiki_toolbar")
                    .frame(egui::Frame::none()
                        .fill(egui::Color32::from_rgb(40, 40, 40))
                        .inner_margin(8.0))
                    .show_inside(ui, |ui| {
                        ui.horizontal(|ui| {
                            // Title editor
                            ui.label("📄");
                            if self.wiki_editing {
                                ui.text_edit_singleline(&mut page.title);
                            } else {
                                ui.label(egui::RichText::new(&page.title).strong().size(18.0));
                            }

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                // Edit/Save button
                                if self.wiki_editing {
                                    if ui.button("💾 Save").clicked() {
                                        page.updated_at = chrono::Utc::now();
                                        self.wiki_editing = false;
                                        // Update the page in the list
                                        if let Some(idx) = self.wiki_pages.iter().position(|p| p.id == page_id) {
                                            self.wiki_pages[idx] = page.clone();
                                        }
                                    }
                                    if ui.button("✖ Cancel").clicked() {
                                        self.wiki_editing = false;
                                    }
                                } else {
                                    if ui.button("✏ Edit").clicked() {
                                        self.wiki_editing = true;
                                    }
                                }
                            });
                        });
                    });

                // Content area
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add_space(8.0);

                        if self.wiki_editing {
                            // Edit mode: Markdown editor
                            ui.label(egui::RichText::new("Content (Markdown)")
                                .size(12.0)
                                .color(egui::Color32::from_rgb(150, 150, 150)));
                            ui.add_space(4.0);

                            let text_edit = egui::TextEdit::multiline(&mut page.content)
                                .font(egui::TextStyle::Monospace)
                                .desired_width(f32::INFINITY)
                                .desired_rows(20);
                            ui.add(text_edit);

                            ui.add_space(16.0);

                            // Tags editor
                            ui.horizontal(|ui| {
                                ui.label("🏷 Tags:");
                                let tags_text = page.tags.join(", ");
                                let mut tags_buffer = tags_text.clone();
                                if ui.text_edit_singleline(&mut tags_buffer).changed() {
                                    page.tags = tags_buffer
                                        .split(',')
                                        .map(|s| s.trim().to_string())
                                        .filter(|s| !s.is_empty())
                                        .collect();
                                }
                            });

                            // Update the page in self for real-time editing
                            if let Some(idx) = self.wiki_pages.iter().position(|p| p.id == page_id) {
                                self.wiki_pages[idx] = page.clone();
                            }
                        } else {
                            // View mode: Render Markdown
                            self.render_markdown_wiki(ui, &page.content);

                            ui.add_space(16.0);

                            // Show tags
                            if !page.tags.is_empty() {
                                ui.horizontal(|ui| {
                                    ui.label("🏷");
                                    for tag in &page.tags {
                                        ui.label(egui::RichText::new(format!("#{}", tag))
                                            .size(11.0)
                                            .color(egui::Color32::from_rgb(100, 150, 200)));
                                    }
                                });
                            }

                            // Show metadata
                            ui.separator();
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(format!("Created: {}", page.created_at.format("%Y-%m-%d %H:%M")))
                                    .size(10.0)
                                    .color(egui::Color32::from_rgb(120, 120, 120)));
                                ui.label("•");
                                ui.label(egui::RichText::new(format!("Updated: {}", page.updated_at.format("%Y-%m-%d %H:%M")))
                                    .size(10.0)
                                    .color(egui::Color32::from_rgb(120, 120, 120)));
                            });
                        }
                    });
            });
    }

    /// Render Markdown content for Wiki (simple implementation)
    fn render_markdown_wiki(&self, ui: &mut egui::Ui, content: &str) {
        for line in content.lines() {
            if line.starts_with("# ") {
                ui.heading(egui::RichText::new(&line[2..]).size(24.0));
            } else if line.starts_with("## ") {
                ui.heading(egui::RichText::new(&line[3..]).size(20.0));
            } else if line.starts_with("### ") {
                ui.heading(egui::RichText::new(&line[4..]).size(16.0));
            } else if line.starts_with("- ") || line.starts_with("* ") {
                ui.horizontal(|ui| {
                    ui.label("  •");
                    ui.label(&line[2..]);
                });
            } else if line.starts_with("```") {
                // Code block marker - skip for now
                continue;
            } else if line.trim().is_empty() {
                ui.add_space(8.0);
            } else {
                ui.label(line);
            }
        }
    }

    /// Render Slack-like Chat panel (takes full center panel)
    fn render_chat_panel(&mut self, ctx: &egui::Context) {
        // Check Slack authentication
        if !self.slack_authenticated {
            // Show Slack connection UI
            egui::CentralPanel::default()
                .frame(egui::Frame::none().fill(egui::Color32::from_rgb(40, 40, 45)))
                .show(ctx, |ui| {
                    // Debug info at top
                    ui.label(egui::RichText::new(format!("Debug: Authenticated = {}", self.slack_authenticated))
                        .size(10.0)
                        .color(egui::Color32::from_rgb(100, 255, 100)));

                    // Add padding from top and center content
                    ui.add_space(80.0);

                    ui.vertical_centered(|ui| {
                        ui.heading(egui::RichText::new("📱 Connect to Slack")
                            .size(28.0)
                            .color(egui::Color32::WHITE));

                        ui.add_space(30.0);

                        ui.label(egui::RichText::new("Enter your Slack Bot Token to get started")
                            .size(15.0)
                            .color(egui::Color32::from_rgb(200, 200, 200)));

                        ui.add_space(25.0);

                        // Token input
                        ui.horizontal(|ui| {
                            ui.add_space(50.0);
                            let _response = ui.add(
                                egui::TextEdit::singleline(&mut self.slack_token_input)
                                    .desired_width(500.0)
                                    .password(true)
                                    .hint_text("xoxb-your-slack-bot-token")
                                    .font(egui::TextStyle::Monospace)
                            );

                            if ui.add_sized([100.0, 30.0], egui::Button::new("🔗 Connect")).clicked()
                                && !self.slack_token_input.is_empty() {
                                let token = self.slack_token_input.clone();
                                self.set_slack_token(token);
                                self.load_slack_channels();
                            }
                        });

                        ui.add_space(40.0);
                        ui.separator();
                        ui.add_space(20.0);

                        ui.label(egui::RichText::new("How to get a Slack Bot Token:")
                            .size(14.0)
                            .strong()
                            .color(egui::Color32::from_rgb(180, 180, 180)));

                        ui.add_space(10.0);

                        ui.horizontal(|ui| {
                            ui.add_space(100.0);
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new("1. Visit https://api.slack.com/apps")
                                    .size(12.0)
                                    .color(egui::Color32::from_rgb(150, 150, 150)));
                                ui.label(egui::RichText::new("2. Create a new app or select existing")
                                    .size(12.0)
                                    .color(egui::Color32::from_rgb(150, 150, 150)));
                                ui.label(egui::RichText::new("3. Navigate to 'OAuth & Permissions'")
                                    .size(12.0)
                                    .color(egui::Color32::from_rgb(150, 150, 150)));
                                ui.label(egui::RichText::new("4. Add Bot Token Scopes:")
                                    .size(12.0)
                                    .color(egui::Color32::from_rgb(150, 150, 150)));
                                ui.horizontal(|ui| {
                                    ui.add_space(20.0);
                                    ui.label(egui::RichText::new("• channels:read")
                                        .size(11.0)
                                        .color(egui::Color32::from_rgb(120, 200, 255)));
                                });
                                ui.horizontal(|ui| {
                                    ui.add_space(20.0);
                                    ui.label(egui::RichText::new("• chat:write")
                                        .size(11.0)
                                        .color(egui::Color32::from_rgb(120, 200, 255)));
                                });
                                ui.horizontal(|ui| {
                                    ui.add_space(20.0);
                                    ui.label(egui::RichText::new("• channels:history")
                                        .size(11.0)
                                        .color(egui::Color32::from_rgb(120, 200, 255)));
                                });
                                ui.label(egui::RichText::new("5. Install app to workspace")
                                    .size(12.0)
                                    .color(egui::Color32::from_rgb(150, 150, 150)));
                                ui.label(egui::RichText::new("6. Copy the 'Bot User OAuth Token'")
                                    .size(12.0)
                                    .color(egui::Color32::from_rgb(150, 150, 150)));
                            });
                        });
                    });
                });
            return;
        }

        // Slack connected - show 3-column layout
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::from_rgb(40, 40, 45)))
            .show(ctx, |ui| {
                // Debug info
                ui.label(egui::RichText::new(format!("Debug: Connected, {} channels, selected: {:?}",
                    self.slack_channels.len(),
                    self.selected_channel_id.as_ref().map(|s| s.as_str()).unwrap_or("none")))
                    .size(10.0)
                    .color(egui::Color32::from_rgb(100, 255, 100)));

                ui.horizontal(|ui| {
                    // Left: Channel list (200px)
                    egui::SidePanel::left("channel_list")
                        .exact_width(250.0)
                        .resizable(false)
                        .frame(egui::Frame::none().fill(egui::Color32::from_rgb(50, 50, 55)))
                        .show_inside(ui, |ui| {
                            ui.heading(egui::RichText::new("📋 Channels")
                                .color(egui::Color32::WHITE));
                            ui.separator();

                            egui::ScrollArea::vertical().show(ui, |ui| {
                                if self.slack_channels.is_empty() {
                                    ui.label(egui::RichText::new("No channels found")
                                        .color(egui::Color32::from_rgb(150, 150, 150)));
                                }

                                for channel in &self.slack_channels.clone() {
                                    let is_selected = self.selected_channel_id.as_ref() == Some(&channel.id);

                                    let button_text = egui::RichText::new(format!("# {}", channel.name))
                                        .color(if is_selected {
                                            egui::Color32::WHITE
                                        } else {
                                            egui::Color32::from_rgb(180, 180, 180)
                                        });

                                    let button = egui::Button::new(button_text)
                                        .fill(if is_selected {
                                            egui::Color32::from_rgb(60, 80, 120)
                                        } else {
                                            egui::Color32::TRANSPARENT
                                        });

                                    if ui.add(button).clicked() {
                                        self.selected_channel_id = Some(channel.id.clone());
                                        self.load_slack_messages(&channel.id);
                                    }
                                }
                            });
                        });

                    // Center: Message area
                    egui::CentralPanel::default()
                        .frame(egui::Frame::none().fill(egui::Color32::from_rgb(35, 35, 40)))
                        .show_inside(ui, |ui| {
                            if let Some(channel_id) = &self.selected_channel_id.clone() {
                                // Channel header
                                ui.horizontal(|ui| {
                                    ui.heading(egui::RichText::new(format!("# Channel"))
                                        .color(egui::Color32::WHITE));
                                });
                                ui.separator();

                                // Show messages
                                egui::ScrollArea::vertical()
                                    .stick_to_bottom(true)
                                    .max_height(ui.available_height() - 60.0)
                                    .show(ui, |ui| {
                                        if self.slack_messages.is_empty() {
                                            ui.label(egui::RichText::new("No messages yet. Start the conversation!")
                                                .color(egui::Color32::from_rgb(150, 150, 150)));
                                        } else {
                                            for msg in &self.slack_messages {
                                                ui.horizontal(|ui| {
                                                    ui.label(egui::RichText::new(&msg.user)
                                                        .strong()
                                                        .color(egui::Color32::from_rgb(120, 180, 255)));
                                                    ui.label(egui::RichText::new(&msg.text)
                                                        .color(egui::Color32::from_rgb(220, 220, 220)));
                                                });
                                                ui.add_space(8.0);
                                            }
                                        }
                                    });

                                ui.separator();

                                // Message input
                                ui.horizontal(|ui| {
                                    let text_edit = egui::TextEdit::singleline(&mut self.chat_input)
                                        .desired_width(ui.available_width() - 100.0)
                                        .hint_text("Type a message...");

                                    ui.add(text_edit);

                                    if ui.button("📤 Send").clicked() {
                                        if !self.chat_input.is_empty() {
                                            let text = self.chat_input.clone();
                                            self.send_slack_message(channel_id, &text, None);
                                        }
                                    }
                                });
                            } else {
                                ui.vertical_centered(|ui| {
                                    ui.add_space(150.0);
                                    ui.label(egui::RichText::new("👈 Select a channel to start chatting")
                                        .size(18.0)
                                        .color(egui::Color32::from_rgb(180, 180, 180)));
                                });
                            }
                        });
                });
            });
    }

    /// Render Database Management Panel
    fn render_database_panel(&mut self, ctx: &egui::Context) {
        // Left sidebar: Connection list
        egui::SidePanel::left("database_connections")
            .default_width(250.0)
            .width_range(200.0..=400.0)
            .resizable(true)
            .frame(egui::Frame::none().fill(egui::Color32::from_rgb(30, 30, 35)))
            .show(ctx, |ui| {
                ui.add_space(8.0);

                // Header with Add button
                ui.horizontal(|ui| {
                    ui.heading(egui::RichText::new("🗄️ Connections")
                        .size(16.0)
                        .color(egui::Color32::WHITE));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button(egui::RichText::new("➕").size(14.0)).clicked() {
                            self.show_add_connection_dialog = true;
                        }
                    });
                });

                ui.separator();
                ui.add_space(8.0);

                // Connection list
                egui::ScrollArea::vertical()
                    .show(ui, |ui| {
                        if self.database_connections.is_empty() {
                            ui.vertical_centered(|ui| {
                                ui.add_space(50.0);
                                ui.label(egui::RichText::new("No connections")
                                    .size(13.0)
                                    .color(egui::Color32::from_rgb(150, 150, 150)));
                                ui.label(egui::RichText::new("Click ➕ to add")
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(120, 120, 120)));
                            });
                        } else {
                            for conn in self.database_connections.clone() {
                                let is_selected = self.selected_connection_id.as_ref() == Some(&conn.id);
                                let bg_color = if is_selected {
                                    egui::Color32::from_rgb(60, 80, 120)
                                } else {
                                    egui::Color32::from_rgb(40, 40, 45)
                                };

                                let response = ui.add(
                                    egui::Button::new(
                                        egui::RichText::new(format!("{} {}",
                                            match conn.db_type {
                                                native::database::DatabaseType::SQLite => "📁",
                                                native::database::DatabaseType::PostgreSQL => "🐘",
                                                native::database::DatabaseType::MySQL => "🐬",
                                            },
                                            conn.name
                                        ))
                                        .size(13.0)
                                        .color(egui::Color32::WHITE)
                                    )
                                    .fill(bg_color)
                                    .frame(false)
                                    .min_size(egui::vec2(ui.available_width(), 36.0))
                                );

                                if response.clicked() {
                                    self.selected_connection_id = Some(conn.id.clone());
                                    self.load_database_tables(&conn.id);
                                }

                                ui.add_space(2.0);
                            }
                        }
                    });
            });

        // Center panel: SQL editor and results
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::from_rgb(40, 40, 45)))
            .show(ctx, |ui| {
                if self.selected_connection_id.is_none() {
                    // No connection selected
                    ui.vertical_centered(|ui| {
                        ui.add_space(100.0);
                        ui.heading(egui::RichText::new("Select a connection")
                            .size(24.0)
                            .color(egui::Color32::WHITE));
                        ui.add_space(10.0);
                        ui.label(egui::RichText::new("Choose a database connection from the sidebar")
                            .size(14.0)
                            .color(egui::Color32::from_rgb(180, 180, 180)));
                    });
                    return;
                }

                ui.add_space(8.0);

                // Connection info header
                if let Some(conn_id) = &self.selected_connection_id.clone() {
                    if let Some(conn) = self.database_client.get_connection(conn_id) {
                        ui.horizontal(|ui| {
                            ui.heading(egui::RichText::new(format!("📊 {}", conn.name))
                                .size(18.0)
                                .color(egui::Color32::WHITE));

                            ui.label(egui::RichText::new(format!("({} • {})",
                                conn.db_type.as_str(),
                                conn.database
                            ))
                                .size(12.0)
                                .color(egui::Color32::from_rgb(150, 150, 150)));
                        });

                        ui.separator();
                        ui.add_space(8.0);
                    }
                }

                // Tables list (horizontal)
                if !self.database_tables.is_empty() {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Tables:")
                            .size(12.0)
                            .color(egui::Color32::from_rgb(180, 180, 180)));

                        egui::ScrollArea::horizontal().show(ui, |ui| {
                            ui.horizontal(|ui| {
                                for table in &self.database_tables.clone() {
                                    let is_selected = self.selected_table.as_ref() == Some(table);
                                    let button_color = if is_selected {
                                        egui::Color32::from_rgb(60, 100, 140)
                                    } else {
                                        egui::Color32::from_rgb(50, 50, 55)
                                    };

                                    if ui.add(egui::Button::new(
                                        egui::RichText::new(format!("📋 {}", table))
                                            .size(11.0)
                                            .color(egui::Color32::WHITE)
                                    ).fill(button_color)).clicked() {
                                        self.selected_table = Some(table.clone());
                                        self.sql_query = format!("SELECT * FROM {} LIMIT 100;", table);
                                    }
                                }
                            });
                        });
                    });

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);
                }

                // SQL Query Editor
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("SQL Query:")
                        .size(13.0)
                        .strong()
                        .color(egui::Color32::WHITE));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(egui::RichText::new("▶ Execute").size(12.0)).clicked() {
                            if let Some(conn_id) = &self.selected_connection_id.clone() {
                                self.execute_sql_query(conn_id);
                            }
                        }
                    });
                });

                ui.add_space(4.0);

                ui.add(
                    egui::TextEdit::multiline(&mut self.sql_query)
                        .desired_width(f32::INFINITY)
                        .desired_rows(8)
                        .font(egui::TextStyle::Monospace)
                        .frame(true)
                );

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                // Results area
                ui.label(egui::RichText::new("Results:")
                    .size(13.0)
                    .strong()
                    .color(egui::Color32::WHITE));

                ui.add_space(4.0);

                if let Some(result) = &self.query_result {
                    if !result.columns.is_empty() {
                        // Results table
                        egui::ScrollArea::both()
                            .max_height(ui.available_height() - 50.0)
                            .show(ui, |ui| {
                                egui::Grid::new("query_results")
                                    .striped(true)
                                    .spacing([10.0, 4.0])
                                    .show(ui, |ui| {
                                        // Header row
                                        for col in &result.columns {
                                            ui.label(egui::RichText::new(col)
                                                .size(12.0)
                                                .strong()
                                                .color(egui::Color32::from_rgb(200, 200, 255)));
                                        }
                                        ui.end_row();

                                        // Data rows
                                        for row in &result.rows {
                                            for value in &row.values {
                                                ui.label(egui::RichText::new(value)
                                                    .size(11.0)
                                                    .color(egui::Color32::from_rgb(220, 220, 220)));
                                            }
                                            ui.end_row();
                                        }
                                    });
                            });

                        ui.add_space(8.0);
                        ui.label(egui::RichText::new(format!("{} rows • {} rows affected",
                            result.rows.len(),
                            result.rows_affected
                        ))
                            .size(11.0)
                            .color(egui::Color32::from_rgb(150, 150, 150)));
                    } else {
                        ui.label(egui::RichText::new(format!("✅ Query executed successfully • {} rows affected",
                            result.rows_affected
                        ))
                            .size(12.0)
                            .color(egui::Color32::from_rgb(100, 200, 100)));
                    }
                } else {
                    ui.vertical_centered(|ui| {
                        ui.add_space(50.0);
                        ui.label(egui::RichText::new("💡 Write a SQL query and click Execute")
                            .size(13.0)
                            .color(egui::Color32::from_rgb(150, 150, 150)));
                    });
                }
            });

        // Add Connection Dialog (IntelliJ DataGrip style)
        if self.show_add_connection_dialog {
            egui::Window::new("Data Sources and Drivers")
                .collapsible(false)
                .resizable(true)
                .default_width(900.0)
                .default_height(600.0)
                .show(ctx, |ui| {
                    // Top section: Name and Comment
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Name:")
                            .size(13.0)
                            .color(egui::Color32::from_rgb(180, 180, 180)));
                        ui.add(egui::TextEdit::singleline(&mut self.new_db_name)
                            .desired_width(300.0)
                            .hint_text("postgres@localhost"));
                    });

                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Comment:")
                            .size(13.0)
                            .color(egui::Color32::from_rgb(180, 180, 180)));
                        ui.add(egui::TextEdit::singleline(&mut self.new_db_comment)
                            .desired_width(400.0));
                    });

                    ui.add_space(12.0);

                    // Tab buttons
                    ui.horizontal(|ui| {
                        let tab_names = ["General", "Options", "SSH/SSL", "Schemas", "Advanced"];
                        for (idx, name) in tab_names.iter().enumerate() {
                            let is_selected = self.new_db_active_tab == idx;
                            let button_color = if is_selected {
                                egui::Color32::from_rgb(60, 80, 120)
                            } else {
                                egui::Color32::from_rgb(45, 45, 50)
                            };

                            if ui.add(egui::Button::new(egui::RichText::new(*name)
                                .size(13.0)
                                .color(egui::Color32::WHITE))
                                .fill(button_color)
                                .min_size(egui::vec2(80.0, 28.0)))
                                .clicked() {
                                self.new_db_active_tab = idx;
                            }
                        }
                    });

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(12.0);

                    // Tab content
                    egui::ScrollArea::vertical()
                        .max_height(400.0)
                        .show(ui, |ui| {
                            if self.new_db_active_tab == 0 {
                                // General Tab
                                self.render_db_general_tab(ui);
                            } else if self.new_db_active_tab == 1 {
                                // Options Tab
                                ui.vertical_centered(|ui| {
                                    ui.add_space(50.0);
                                    ui.label(egui::RichText::new("Options settings")
                                        .size(13.0)
                                        .color(egui::Color32::from_rgb(150, 150, 150)));
                                    ui.label("(Not implemented yet)");
                                });
                            } else if self.new_db_active_tab == 2 {
                                // SSH/SSL Tab
                                ui.vertical_centered(|ui| {
                                    ui.add_space(50.0);
                                    ui.label(egui::RichText::new("SSH/SSL settings")
                                        .size(13.0)
                                        .color(egui::Color32::from_rgb(150, 150, 150)));
                                    ui.label("(Not implemented yet)");
                                });
                            } else if self.new_db_active_tab == 3 {
                                // Schemas Tab
                                ui.vertical_centered(|ui| {
                                    ui.add_space(50.0);
                                    ui.label(egui::RichText::new("Schema settings")
                                        .size(13.0)
                                        .color(egui::Color32::from_rgb(150, 150, 150)));
                                    ui.label("(Not implemented yet)");
                                });
                            } else {
                                // Advanced Tab
                                ui.vertical_centered(|ui| {
                                    ui.add_space(50.0);
                                    ui.label(egui::RichText::new("Advanced settings")
                                        .size(13.0)
                                        .color(egui::Color32::from_rgb(150, 150, 150)));
                                    ui.label("(Not implemented yet)");
                                });
                            }
                        });

                    ui.add_space(8.0);

                    // Test Connection button
                    if self.new_db_active_tab == 0 {
                        ui.horizontal(|ui| {
                            if ui.button(egui::RichText::new("Test Connection")
                                .size(13.0)
                                .color(egui::Color32::from_rgb(100, 150, 255)))
                                .clicked() {
                                self.test_database_connection();
                            }

                            if let Some(result) = &self.new_db_test_result {
                                ui.label(egui::RichText::new(result)
                                    .size(12.0)
                                    .color(if result.contains("✅") {
                                        egui::Color32::from_rgb(100, 200, 100)
                                    } else {
                                        egui::Color32::from_rgb(255, 100, 100)
                                    }));
                            }
                        });

                        ui.add_space(8.0);
                    }

                    ui.separator();
                    ui.add_space(8.0);

                    // Bottom buttons
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            // Check if all required fields are filled
                            let is_valid = !self.new_db_name.is_empty() && !self.new_db_database.is_empty();

                            if ui.add(egui::Button::new(egui::RichText::new("OK").size(13.0))
                                .fill(egui::Color32::from_rgb(60, 100, 200))
                                .min_size(egui::vec2(80.0, 28.0)))
                                .clicked() && is_valid {
                                tracing::info!("🔵 OK button clicked - adding connection");
                                self.add_database_connection();
                                self.show_add_connection_dialog = false;
                            }

                            if ui.add(egui::Button::new(egui::RichText::new("Apply").size(13.0))
                                .fill(egui::Color32::from_rgb(50, 50, 55))
                                .min_size(egui::vec2(80.0, 28.0)))
                                .clicked() && is_valid {
                                tracing::info!("🟢 Apply button clicked - adding connection");
                                self.add_database_connection();
                            }

                            if ui.add(egui::Button::new(egui::RichText::new("Cancel").size(13.0))
                                .fill(egui::Color32::from_rgb(50, 50, 55))
                                .min_size(egui::vec2(80.0, 28.0)))
                                .clicked() {
                                self.show_add_connection_dialog = false;
                            }

                            // Show validation message if invalid
                            if !is_valid {
                                ui.label(egui::RichText::new("⚠ Name and Database/Path are required")
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(255, 150, 100)));
                            }
                        });
                    });
                });
        }
    }

    /// Render BerryCode AI chat (legacy - kept for AI features)
    #[allow(dead_code)]
    /// Render AI Chat panel (right side of editor)
    fn render_ai_chat_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("ai_chat_panel")
            .exact_width(400.0)  // Fixed width to prevent resizing
            .resizable(false)    // Disable resizing
            .frame(
                egui::Frame::none()
                    .fill(ui_colors::SIDEBAR_BG)  // Match sidebar background
                    .inner_margin(0.0)
            )
            .show(ctx, |ui| {
                // Header bar with dark background
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(20, 21, 23))
                    .inner_margin(egui::Margin::symmetric(16.0, 12.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // Left: AI Chat title
                            ui.label(egui::RichText::new("AI Chat")
                                .color(ui_colors::TEXT_DEFAULT)
                                .size(15.0));

                            // Connection status indicator
                            let (status_text, status_color) = if self.grpc_connected {
                                ("●", egui::Color32::from_rgb(0, 200, 0))  // Green dot
                            } else {
                                ("●", egui::Color32::from_rgb(150, 150, 150))  // Gray dot
                            };
                            ui.label(egui::RichText::new(status_text)
                                .color(status_color)
                                .size(12.0));

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                // Minimize button
                                if ui.button(egui::RichText::new("−").size(16.0)).clicked() {
                                    // TODO: Minimize panel
                                }

                                // Menu button (3 dots)
                                if ui.button(egui::RichText::new("⋮").size(16.0)).clicked() {
                                    // TODO: Show menu
                                }

                                // History/Clock button
                                if ui.button(egui::RichText::new("🕐").size(14.0)).clicked() {
                                    // TODO: Show history
                                }

                                // New Chat button
                                if ui.button(egui::RichText::new("+ New Chat").size(13.0)).clicked() {
                                    self.grpc_messages.clear();
                                    self.grpc_input.clear();
                                }
                            });
                        });
                    });

                ui.add_space(0.0);

                // Chat history area with markdown rendering
                egui::ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        ui.add_space(12.0);

                        if self.grpc_messages.is_empty() {
                            ui.add_space(100.0);
                            ui.vertical_centered(|ui| {
                                ui.add_space(20.0);

                                // Feature list
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("Multiline code completion")
                                        .color(egui::Color32::from_rgb(150, 150, 150))
                                        .size(14.0));
                                    ui.label(egui::RichText::new("⌘")
                                        .monospace()
                                        .color(egui::Color32::from_rgb(100, 100, 100))
                                        .size(12.0));
                                });

                                ui.add_space(12.0);

                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("Code generation in the editor")
                                        .color(egui::Color32::from_rgb(150, 150, 150))
                                        .size(14.0));
                                    ui.label(egui::RichText::new("⌘⌥")
                                        .monospace()
                                        .color(egui::Color32::from_rgb(100, 100, 100))
                                        .size(12.0));
                                });

                                ui.add_space(12.0);

                                ui.label(egui::RichText::new("AI actions in the editor's context menu")
                                    .color(egui::Color32::from_rgb(150, 150, 150))
                                    .size(14.0));

                                ui.add_space(12.0);

                                ui.hyperlink_to(
                                    egui::RichText::new("All features")
                                        .color(egui::Color32::from_rgb(100, 150, 255))
                                        .size(14.0),
                                    "https://github.com/anthropics/claude-code"
                                );
                            });
                        } else {
                            for msg in &self.grpc_messages.clone() {
                                if msg.is_user {
                                    // User message - simple style
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                                        egui::Frame::none()
                                            .fill(egui::Color32::from_rgb(35, 40, 50))
                                            .inner_margin(12.0)
                                            .rounding(8.0)
                                            .show(ui, |ui| {
                                                ui.set_max_width(300.0);  // Fixed width
                                                ui.label(egui::RichText::new(&msg.content)
                                                    .color(egui::Color32::from_rgb(220, 220, 220))
                                                    .size(14.0));
                                            });
                                    });
                                } else {
                                    // AI message - render as markdown
                                    ui.add_space(12.0);
                                    ui.vertical(|ui| {
                                        ui.set_max_width(360.0);  // Match panel width
                                        Self::render_markdown(ui, &msg.content);
                                    });
                                }

                                ui.add_space(16.0);
                            }
                        }

                        // Show streaming message if present
                        if self.grpc_streaming {
                            ui.add_space(12.0);
                            ui.vertical(|ui| {
                                ui.set_max_width(360.0);  // Match panel width
                                Self::render_markdown(ui, &self.grpc_current_response);
                                ui.add_space(8.0);
                                ui.spinner();
                            });
                        }

                        ui.add_space(12.0);
                    });

                ui.add_space(8.0);

                // Input area at bottom
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    // Bottom footer: "Share feedback" link
                    ui.horizontal(|ui| {
                        ui.add_space(140.0);  // Fixed spacing for centering
                        ui.hyperlink_to(
                            egui::RichText::new("Share feedback ↗")
                                .size(12.0)
                                .color(egui::Color32::from_rgb(150, 150, 150)),
                            "https://github.com/anthropics/claude-code/issues"
                        );
                    });

                    ui.add_space(8.0);

                    // Input box with controls (blue border like Claude Code)
                    let input_focused = ui.memory(|mem| mem.focused().is_some());
                    let border_color = if input_focused {
                        egui::Color32::from_rgb(70, 130, 255) // Blue when focused
                    } else {
                        egui::Color32::from_rgb(60, 60, 60) // Gray when not focused
                    };

                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(35, 36, 38))
                        .inner_margin(12.0)
                        .rounding(8.0)
                        .stroke(egui::Stroke::new(2.0, border_color))
                        .show(ui, |ui| {
                            // Text input area
                            let hint_text = "Ask AI Assistant, use @mentions or /commands";
                            let text_edit = egui::TextEdit::multiline(&mut self.grpc_input)
                                .desired_width(350.0)  // Fixed width
                                .desired_rows(3)
                                .hint_text(hint_text)
                                .font(egui::FontId::proportional(14.0));

                            let response = ui.add(text_edit);

                            ui.add_space(8.0);

                            // Attached files/context (like "📎 CLAUDE.md Current ×")
                            ui.horizontal(|ui| {
                                egui::Frame::none()
                                    .fill(egui::Color32::from_rgb(50, 51, 53))
                                    .inner_margin(egui::Margin::symmetric(8.0, 4.0))
                                    .rounding(4.0)
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.spacing_mut().item_spacing.x = 6.0;
                                            ui.label(egui::RichText::new("📎")
                                                .size(12.0));
                                            ui.label(egui::RichText::new("CLAUDE.md")
                                                .color(egui::Color32::from_rgb(100, 150, 255))
                                                .size(12.0));
                                            ui.label(egui::RichText::new("Current")
                                                .color(egui::Color32::from_rgb(150, 150, 150))
                                                .size(11.0));
                                            if ui.small_button("×").clicked() {
                                                // TODO: Remove attachment
                                            }
                                        });
                                    });
                            });

                            ui.add_space(8.0);

                            // Bottom controls row (matching Claude Code design)
                            ui.horizontal(|ui| {
                                // Left side: "+" button and "Chat ▼"
                                if ui.button(egui::RichText::new("+").size(16.0)).clicked() {
                                    // TODO: Attach file or add context
                                }

                                ui.add_space(4.0);

                                // Chat mode dropdown
                                egui::ComboBox::from_id_salt("chat_mode_selector")
                                    .selected_text(
                                        egui::RichText::new(match self.ai_chat_mode {
                                            AIChatMode::Chat => "💬 Chat",
                                            AIChatMode::Autonomous => "⚡ Autonomous",
                                        })
                                        .color(egui::Color32::from_rgb(200, 200, 200))
                                        .size(13.0)
                                    )
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut self.ai_chat_mode, AIChatMode::Chat, "💬 Chat");
                                        ui.selectable_value(&mut self.ai_chat_mode, AIChatMode::Autonomous, "⚡ Autonomous");
                                    });

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    // Right side: Send button
                                    let send_enabled = !self.grpc_input.trim().is_empty() && !self.grpc_streaming;

                                    if ui.add_enabled(send_enabled,
                                        egui::Button::new(egui::RichText::new("▶").size(16.0))
                                    ).clicked() || (response.has_focus() && ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Enter))) {
                                        self.send_grpc_message();
                                    }

                                    ui.add_space(8.0);

                                    // Auto mode dropdown (placeholder)
                                    egui::ComboBox::from_id_salt("auto_mode_selector")
                                        .selected_text(
                                            egui::RichText::new("Auto")
                                                .color(egui::Color32::from_rgb(150, 150, 150))
                                                .size(13.0)
                                        )
                                        .show_ui(ui, |ui| {
                                            ui.label("Normal");
                                            ui.label("Creative");
                                            ui.label("Precise");
                                        });

                                    if self.grpc_streaming {
                                        ui.add_space(4.0);
                                        ui.spinner();
                                    }
                                });
                            });
                        });

                    ui.add_space(24.0);  // Increased bottom margin to prevent cutoff
                });
            });
    }

    fn render_berrycode_ai_chat(&mut self, ui: &mut egui::Ui) {
        ui.label("AI Chat - Use right panel instead.");
    }


    /// Simple markdown renderer for AI chat responses
    fn render_markdown(ui: &mut egui::Ui, content: &str) {
        let mut in_code_block = false;
        let mut code_lang = String::new();
        let mut code_lines = Vec::new();

        for line in content.lines() {
            // Code block detection
            if line.trim().starts_with("```") {
                if in_code_block {
                    // End code block - render it
                    let code_text = code_lines.join("\n");
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(35, 35, 35))
                        .inner_margin(8.0)
                        .rounding(4.0)
                        .show(ui, |ui| {
                            ui.add(egui::Label::new(
                                egui::RichText::new(&code_text)
                                    .monospace()
                                    .color(egui::Color32::from_rgb(0xAB, 0xB2, 0xBF))
                            ).selectable(true));
                        });
                    code_lines.clear();
                    in_code_block = false;
                } else {
                    // Start code block
                    code_lang = line.trim().strip_prefix("```").unwrap_or("").to_string();
                    in_code_block = true;
                }
                continue;
            }

            if in_code_block {
                code_lines.push(line);
                continue;
            }

            // Heading detection
            if line.trim().starts_with("# ") {
                ui.heading(egui::RichText::new(line.trim_start_matches("# ")).color(egui::Color32::from_rgb(0xAB, 0xB2, 0xBF)));
                continue;
            }
            if line.trim().starts_with("## ") {
                ui.label(egui::RichText::new(line.trim_start_matches("## ")).size(16.0).strong().color(egui::Color32::from_rgb(0xAB, 0xB2, 0xBF)));
                continue;
            }
            if line.trim().starts_with("### ") {
                ui.label(egui::RichText::new(line.trim_start_matches("### ")).size(14.0).strong().color(egui::Color32::from_rgb(0xAB, 0xB2, 0xBF)));
                continue;
            }

            // List detection (bullets)
            if line.trim().starts_with("- ") || line.trim().starts_with("* ") {
                ui.horizontal(|ui| {
                    ui.label("•");
                    let text = line.trim_start_matches("- ").trim_start_matches("* ");
                    Self::render_inline_formatting(ui, text);
                });
                continue;
            }

            // List detection (numbered)
            if let Some(rest) = line.trim().strip_prefix(|c: char| c.is_ascii_digit()) {
                if rest.starts_with(". ") {
                    let number = line.trim().chars().take_while(|c| c.is_ascii_digit()).collect::<String>();
                    ui.horizontal(|ui| {
                        ui.label(format!("{}.", number));
                        let text = rest.trim_start_matches(". ");
                        Self::render_inline_formatting(ui, text);
                    });
                    continue;
                }
            }

            // Regular text - handle inline formatting
            if !line.trim().is_empty() {
                Self::render_inline_formatting(ui, line);
            } else {
                ui.add_space(4.0);
            }
        }

        // Handle unclosed code block
        if in_code_block && !code_lines.is_empty() {
            let code_text = code_lines.join("\n");
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(35, 35, 35))
                .inner_margin(8.0)
                .rounding(4.0)
                .show(ui, |ui| {
                    ui.add(egui::Label::new(
                        egui::RichText::new(&code_text)
                            .monospace()
                            .color(egui::Color32::from_rgb(0xAB, 0xB2, 0xBF))
                    ).selectable(true));
                });
        }
    }

    /// Render inline markdown formatting (bold, italic, code, links)
    /// Uses flowing layout instead of horizontal_wrapped to avoid vertical text splitting
    fn render_inline_formatting(ui: &mut egui::Ui, text: &str) {
        let unified_white = egui::Color32::from_rgb(0xAB, 0xB2, 0xBF);
        let code_bg = egui::Color32::from_rgb(45, 45, 45);

        // Parse inline markdown into segments
        #[derive(Debug)]
        enum Segment {
            Text(String),
            Code(String),
            Bold(String),
            Italic(String),
            Link { text: String, url: String },
        }

        let mut segments = Vec::new();
        let mut chars = text.chars().peekable();
        let mut current_text = String::new();

        while let Some(ch) = chars.next() {
            match ch {
                '`' => {
                    // Save accumulated text
                    if !current_text.is_empty() {
                        segments.push(Segment::Text(current_text.clone()));
                        current_text.clear();
                    }
                    // Extract code
                    let mut code_text = String::new();
                    while let Some(&next_ch) = chars.peek() {
                        if next_ch == '`' {
                            chars.next();
                            break;
                        }
                        code_text.push(chars.next().unwrap());
                    }
                    segments.push(Segment::Code(code_text));
                }
                '*' if chars.peek() == Some(&'*') => {
                    chars.next();
                    // Save accumulated text
                    if !current_text.is_empty() {
                        segments.push(Segment::Text(current_text.clone()));
                        current_text.clear();
                    }
                    // Extract bold
                    let mut bold_text = String::new();
                    let mut found_closing = false;
                    while let Some(&next_ch) = chars.peek() {
                        if next_ch == '*' {
                            chars.next();
                            if chars.peek() == Some(&'*') {
                                chars.next();
                                found_closing = true;
                                break;
                            } else {
                                bold_text.push('*');
                            }
                        } else {
                            bold_text.push(chars.next().unwrap());
                        }
                    }
                    if found_closing {
                        segments.push(Segment::Bold(bold_text));
                    } else {
                        current_text.push_str("**");
                        current_text.push_str(&bold_text);
                    }
                }
                '*' => {
                    // Save accumulated text
                    if !current_text.is_empty() {
                        segments.push(Segment::Text(current_text.clone()));
                        current_text.clear();
                    }
                    // Extract italic
                    let mut italic_text = String::new();
                    let mut found_closing = false;
                    while let Some(&next_ch) = chars.peek() {
                        if next_ch == '*' {
                            chars.next();
                            found_closing = true;
                            break;
                        }
                        italic_text.push(chars.next().unwrap());
                    }
                    if found_closing {
                        segments.push(Segment::Italic(italic_text));
                    } else {
                        current_text.push('*');
                        current_text.push_str(&italic_text);
                    }
                }
                '[' => {
                    // Save accumulated text
                    if !current_text.is_empty() {
                        segments.push(Segment::Text(current_text.clone()));
                        current_text.clear();
                    }
                    // Extract link
                    let mut link_text = String::new();
                    let mut found_text_end = false;
                    while let Some(&next_ch) = chars.peek() {
                        if next_ch == ']' {
                            chars.next();
                            found_text_end = true;
                            break;
                        }
                        link_text.push(chars.next().unwrap());
                    }
                    if found_text_end && chars.peek() == Some(&'(') {
                        chars.next();
                        let mut url = String::new();
                        let mut found_url_end = false;
                        while let Some(&next_ch) = chars.peek() {
                            if next_ch == ')' {
                                chars.next();
                                found_url_end = true;
                                break;
                            }
                            url.push(chars.next().unwrap());
                        }
                        if found_url_end {
                            segments.push(Segment::Link { text: link_text, url });
                        } else {
                            current_text.push('[');
                            current_text.push_str(&link_text);
                            current_text.push_str("](");
                            current_text.push_str(&url);
                        }
                    } else {
                        current_text.push('[');
                        current_text.push_str(&link_text);
                        if found_text_end {
                            current_text.push(']');
                        }
                    }
                }
                _ => {
                    current_text.push(ch);
                }
            }
        }

        // Save remaining text
        if !current_text.is_empty() {
            segments.push(Segment::Text(current_text));
        }

        // Render segments with word wrapping enabled
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0; // No spacing between segments

            for segment in segments {
                match segment {
                    Segment::Text(s) => {
                        ui.label(egui::RichText::new(s).color(unified_white));
                    }
                    Segment::Code(s) => {
                        egui::Frame::none()
                            .fill(code_bg)
                            .inner_margin(egui::Margin::symmetric(3.0, 1.0))
                            .rounding(2.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new(s).monospace().color(unified_white));
                            });
                    }
                    Segment::Bold(s) => {
                        ui.label(egui::RichText::new(s).strong().color(unified_white));
                    }
                    Segment::Italic(s) => {
                        ui.label(egui::RichText::new(s).italics().color(unified_white));
                    }
                    Segment::Link { text, url } => {
                        ui.hyperlink_to(text, url);
                    }
                }
            }
        });
    }

    /// Render Terminal panel (Phase 4: full implementation)
    fn render_terminal(&mut self, ui: &mut egui::Ui) {
        ui.heading("🖥️ Terminal");
        ui.separator();

        // Display working directory
        ui.horizontal(|ui| {
            ui.label("📁");
            ui.label(&self.terminal_working_dir);
        });

        ui.add_space(4.0);

        ui.vertical(|ui| {
            // Output area with scrolling
            let scroll_area = egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .stick_to_bottom(true)
                .max_height(ui.available_height() - 35.0);

            scroll_area.show(ui, |ui| {
                if self.terminal_output.is_empty() {
                    ui.label("ターミナル出力がここに表示されます");
                } else {
                    for line in &self.terminal_output {
                        let color = match line.style {
                            TerminalStyle::Command => egui::Color32::from_rgb(106, 135, 89), // Green
                            TerminalStyle::Output => egui::Color32::LIGHT_GRAY,
                            TerminalStyle::Error => egui::Color32::from_rgb(255, 100, 100), // Red
                        };

                        ui.colored_label(
                            color,
                            egui::RichText::new(&line.text).font(egui::FontId::monospace(12.0)),
                        );
                    }
                }
            });

            ui.add_space(4.0);

            // Input area
            ui.horizontal(|ui| {
                ui.label("$");

                let mut response = ui.text_edit_singleline(&mut self.terminal_input);

                // Handle Enter key
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    self.execute_terminal_command();
                    response.request_focus();
                }

                // Handle arrow keys for history (check input directly)
                let arrow_up_pressed = ui.input(|i| i.key_pressed(egui::Key::ArrowUp));
                let arrow_down_pressed = ui.input(|i| i.key_pressed(egui::Key::ArrowDown));

                if arrow_up_pressed && response.has_focus() {
                    self.navigate_history_up();
                    response.request_focus();
                }
                if arrow_down_pressed && response.has_focus() {
                    self.navigate_history_down();
                    response.request_focus();
                }
            });
        });
    }

    /// Render full-screen iTerm2-like terminal
    fn render_terminal_fullscreen(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(0, 0, 0)) // Pure black background like iTerm2
                    .inner_margin(16.0)
            )
            .show(ctx, |ui| {
                // Terminal title bar (optional, like iTerm2 tabs)
                egui::TopBottomPanel::top("terminal_titlebar")
                    .frame(egui::Frame::none()
                        .fill(egui::Color32::from_rgb(30, 30, 30))
                        .inner_margin(8.0))
                    .show_inside(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("🖥️ Terminal")
                                .color(egui::Color32::from_rgb(200, 200, 200))
                                .size(12.0));

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(egui::RichText::new(&self.terminal_working_dir)
                                    .color(egui::Color32::from_rgb(120, 120, 120))
                                    .size(11.0)
                                    .monospace());
                            });
                        });
                    });

                // Terminal output area
                let scroll_area = egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .stick_to_bottom(true);

                scroll_area.show(ui, |ui| {
                    ui.vertical(|ui| {
                        // Show terminal output
                        if self.terminal_output.is_empty() {
                            ui.label(egui::RichText::new("Last login: Fri Jan 17 09:00:00 on ttys000")
                                .color(egui::Color32::from_rgb(150, 150, 150))
                                .font(egui::FontId::monospace(13.0)));
                            ui.add_space(8.0);
                        } else {
                            for line in &self.terminal_output {
                                let color = match line.style {
                                    TerminalStyle::Command => egui::Color32::from_rgb(106, 200, 120), // Bright green for commands
                                    TerminalStyle::Output => egui::Color32::from_rgb(220, 220, 220), // Bright white for output
                                    TerminalStyle::Error => egui::Color32::from_rgb(255, 100, 100), // Red for errors
                                };

                                ui.label(egui::RichText::new(&line.text)
                                    .color(color)
                                    .font(egui::FontId::monospace(13.0)));
                            }
                        }

                        ui.add_space(4.0);

                        // Current command prompt
                        ui.horizontal(|ui| {
                            // Prompt (user@hostname:path $)
                            let username = std::env::var("USER").unwrap_or_else(|_| "user".to_string());
                            let hostname = "localhost"; // Simplified
                            let path = self.terminal_working_dir.replace(&std::env::var("HOME").unwrap_or_default(), "~");

                            ui.label(egui::RichText::new(format!("{}@{}:", username, hostname))
                                .color(egui::Color32::from_rgb(106, 200, 120)) // Green
                                .font(egui::FontId::monospace(13.0)));

                            ui.label(egui::RichText::new(&path)
                                .color(egui::Color32::from_rgb(100, 150, 255)) // Blue for path
                                .font(egui::FontId::monospace(13.0)));

                            ui.label(egui::RichText::new("$")
                                .color(egui::Color32::from_rgb(220, 220, 220)) // White
                                .font(egui::FontId::monospace(13.0)));

                            // Command input
                            let text_edit = egui::TextEdit::singleline(&mut self.terminal_input)
                                .font(egui::FontId::monospace(13.0))
                                .text_color(egui::Color32::from_rgb(220, 220, 220))
                                .desired_width(f32::INFINITY)
                                .frame(false); // No border, like a real terminal

                            let mut response = ui.add(text_edit);

                            // Handle Enter key to execute command
                            if ui.input(|i| i.key_pressed(egui::Key::Enter)) && response.has_focus() {
                                self.execute_terminal_command();
                                response.request_focus();
                            }

                            // Handle arrow keys for history (check input directly)
                            let arrow_up_pressed = ui.input(|i| i.key_pressed(egui::Key::ArrowUp));
                            let arrow_down_pressed = ui.input(|i| i.key_pressed(egui::Key::ArrowDown));

                            if arrow_up_pressed && response.has_focus() {
                                self.navigate_history_up();
                                response.request_focus();
                            }
                            if arrow_down_pressed && response.has_focus() {
                                self.navigate_history_down();
                                response.request_focus();
                            }
                        });
                    });
                });
            });
    }

    /// Execute terminal command
    fn execute_terminal_command(&mut self) {
        let cmd = self.terminal_input.trim().to_string();

        if cmd.is_empty() {
            return;
        }

        // Add to history
        if !self.terminal_history.contains(&cmd) || self.terminal_history.last() != Some(&cmd) {
            self.terminal_history.push(cmd.clone());
        }
        self.terminal_history_index = None;

        // Display command
        self.terminal_output.push(TerminalLine {
            text: format!("$ {}", cmd),
            style: TerminalStyle::Command,
        });

        // Handle built-in commands
        if cmd.starts_with("cd ") {
            let path = cmd[3..].trim();
            self.change_directory(path);
        } else if cmd == "clear" {
            self.terminal_output.clear();
        } else {
            // Execute external command
            self.execute_external_command(&cmd);
        }

        // Clear input
        self.terminal_input.clear();
    }

    /// Change terminal working directory
    fn change_directory(&mut self, path: &str) {
        use std::path::Path;

        let new_path = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("{}/{}", self.terminal_working_dir, path)
        };

        let normalized_path = Path::new(&new_path).canonicalize();

        match normalized_path {
            Ok(p) => {
                self.terminal_working_dir = p.to_string_lossy().to_string();
                tracing::info!("📁 Changed directory to: {}", self.terminal_working_dir);
            }
            Err(e) => {
                self.terminal_output.push(TerminalLine {
                    text: format!("cd: {}: {}", path, e),
                    style: TerminalStyle::Error,
                });
            }
        }
    }

    /// Execute external command via native::terminal
    fn execute_external_command(&mut self, cmd: &str) {
        let terminal_manager = native::terminal::TerminalManager::new();

        match terminal_manager.execute_command(cmd, &self.terminal_working_dir) {
            Ok(output) => {
                // Split output into lines and add to terminal
                for line in output.output.lines() {
                    self.terminal_output.push(TerminalLine {
                        text: line.to_string(),
                        style: TerminalStyle::Output,
                    });
                }

                // Show exit code if non-zero
                if let Some(code) = output.exit_code {
                    if code != 0 {
                        self.terminal_output.push(TerminalLine {
                            text: format!("Exit code: {}", code),
                            style: TerminalStyle::Error,
                        });
                    }
                }
            }
            Err(e) => {
                self.terminal_output.push(TerminalLine {
                    text: format!("Error: {}", e),
                    style: TerminalStyle::Error,
                });
            }
        }
    }

    /// Navigate command history up
    fn navigate_history_up(&mut self) {
        if self.terminal_history.is_empty() {
            return;
        }

        let new_index = match self.terminal_history_index {
            None => Some(self.terminal_history.len() - 1),
            Some(0) => Some(0),
            Some(i) => Some(i - 1),
        };

        if let Some(idx) = new_index {
            self.terminal_history_index = Some(idx);
            self.terminal_input = self.terminal_history[idx].clone();
        }
    }

    /// Navigate command history down
    fn navigate_history_down(&mut self) {
        if self.terminal_history.is_empty() {
            return;
        }

        let new_index = match self.terminal_history_index {
            None => None,
            Some(i) if i >= self.terminal_history.len() - 1 => {
                self.terminal_input.clear();
                None
            }
            Some(i) => Some(i + 1),
        };

        if let Some(idx) = new_index {
            self.terminal_history_index = Some(idx);
            self.terminal_input = self.terminal_history[idx].clone();
        } else {
            self.terminal_history_index = None;
        }
    }

    /// Render Git Diff Viewer in center panel (split view: Graph above, Diff below)
    fn render_git_diff_viewer(&mut self, ctx: &egui::Context) {
        // Top panel: Git commit graph (30% of height)
        egui::TopBottomPanel::top("git_graph_panel")
            .default_height(250.0)
            .resizable(true)
            .frame(
                egui::Frame::none()
                    .fill(ui_colors::SIDEBAR_BG)
                    .inner_margin(egui::Margin::same(8.0))
            )
            .show(ctx, |ui| {
                ui.heading("📊 Commit Graph");
                ui.separator();

                // Render commit graph (reuse existing logic from History tab)
                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        self.render_git_graph_compact(ui);
                    });
            });

        // Bottom panel: Diff viewer
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(ui_colors::SIDEBAR_BG)
                    .inner_margin(egui::Margin::same(8.0))
            )
            .show(ctx, |ui| {
                if let Some(diff) = &self.git_diff_state.diff {
                    let file_path = self.git_diff_state.selected_file.as_ref()
                        .map(|s| s.as_str())
                        .unwrap_or("Unknown file");

                    // Header with file path and status
                    ui.horizontal(|ui| {
                        ui.heading(format!("📝 Diff: {}", file_path));
                        let status_color = match diff.status.as_str() {
                            "added" => egui::Color32::from_rgb(100, 255, 100),
                            "deleted" => egui::Color32::from_rgb(255, 100, 100),
                            "modified" => egui::Color32::from_rgb(100, 180, 255),
                            _ => ui_colors::TEXT_DEFAULT,
                        };
                        ui.label(egui::RichText::new(&diff.status.to_uppercase())
                            .color(status_color)
                            .strong());
                    });
                    ui.separator();

                    // Scroll area for diff content
                    egui::ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            // Render each hunk
                            for hunk in &diff.hunks {
                                // Hunk header
                                ui.label(egui::RichText::new(&hunk.header)
                                    .color(egui::Color32::from_rgb(100, 180, 255))
                                    .family(egui::FontFamily::Monospace)
                                    .size(13.0));

                                ui.add_space(4.0);

                                // Render lines in the hunk
                                for line in &hunk.lines {
                                    let (bg_color, fg_color, prefix) = match line.origin {
                                        '+' => (
                                            egui::Color32::from_rgb(20, 60, 20),  // Dark green background
                                            ui_colors::TEXT_DEFAULT, // White text (same as file tree)
                                            "+ "
                                        ),
                                        '-' => (
                                            egui::Color32::from_rgb(60, 20, 20),  // Dark red background
                                            ui_colors::TEXT_DEFAULT, // White text (same as file tree)
                                            "- "
                                        ),
                                        _ => (
                                            ui_colors::SIDEBAR_BG,  // Default background
                                            ui_colors::TEXT_DEFAULT, // Default text
                                            "  "
                                        ),
                                    };

                                    // Line number + content
                                    let line_num = if line.origin == '+' {
                                        line.new_lineno.map(|n| format!("{:>4} ", n)).unwrap_or_else(|| "     ".to_string())
                                    } else if line.origin == '-' {
                                        line.old_lineno.map(|n| format!("{:>4} ", n)).unwrap_or_else(|| "     ".to_string())
                                    } else {
                                        line.new_lineno.map(|n| format!("{:>4} ", n)).unwrap_or_else(|| "     ".to_string())
                                    };

                                    let text = format!("{}{}{}", line_num, prefix, line.content.trim_end());

                                    egui::Frame::none()
                                        .fill(bg_color)
                                        .inner_margin(egui::Margin::symmetric(4.0, 2.0))
                                        .show(ui, |ui| {
                                            ui.label(egui::RichText::new(text)
                                                .color(fg_color)
                                                .family(egui::FontFamily::Monospace)
                                                .size(13.0));
                                        });
                                }

                                ui.add_space(8.0);
                            }
                        });
                } else {
                    // No diff selected
                    ui.vertical_centered(|ui| {
                        ui.add_space(100.0);
                        ui.label("📝 Select a file to view diff");
                    });
                }
            });
    }

    /// Render compact Git commit graph (for split view)
    fn render_git_graph_compact(&mut self, ui: &mut egui::Ui) {
        if self.git_history_state.graph_nodes.is_empty() {
            ui.label("No commits. Click Refresh in Git Status tab.");
            return;
        }

        let nodes = self.git_history_state.graph_nodes.clone();

        const NODE_RADIUS: f32 = 4.0;
        const COLUMN_WIDTH: f32 = 16.0;
        const ROW_HEIGHT: f32 = 24.0;

        // 8-color palette for branches (same as full graph)
        let colors = [
            egui::Color32::from_rgb(106, 180, 89),   // Green
            egui::Color32::from_rgb(100, 181, 246),  // Blue
            egui::Color32::from_rgb(255, 198, 109),  // Yellow
            egui::Color32::from_rgb(239, 83, 80),    // Red
            egui::Color32::from_rgb(171, 128, 255),  // Purple
            egui::Color32::from_rgb(255, 138, 128),  // Coral
            egui::Color32::from_rgb(128, 222, 234),  // Cyan
            egui::Color32::from_rgb(255, 171, 64),   // Orange
        ];

        // Display recent 10 commits in compact form with graph
        for node in nodes.iter().take(10) {
            ui.horizontal(|ui| {
                // Graph column (left side)
                let (graph_rect, _graph_response) = ui.allocate_exact_size(
                    egui::vec2(COLUMN_WIDTH * 8.0, ROW_HEIGHT),
                    egui::Sense::hover(),
                );

                if ui.is_rect_visible(graph_rect) {
                    let painter = ui.painter();

                    // Draw graph lines
                    for line in &node.graph_lines {
                        let from_pos = graph_rect.min + egui::vec2(
                            line.from_column as f32 * COLUMN_WIDTH + COLUMN_WIDTH / 2.0,
                            NODE_RADIUS,
                        );
                        let to_pos = graph_rect.min + egui::vec2(
                            line.to_column as f32 * COLUMN_WIDTH + COLUMN_WIDTH / 2.0,
                            ROW_HEIGHT,
                        );

                        let color = colors[line.color_index % colors.len()];

                        if line.line_type == native::git::GraphLineType::Direct {
                            // Straight line
                            painter.line_segment([from_pos, to_pos], egui::Stroke::new(2.0, color));
                        } else {
                            // Bezier curve for merge
                            painter.add(egui::Shape::CubicBezier(egui::epaint::CubicBezierShape::from_points_stroke(
                                [
                                    from_pos,
                                    from_pos + egui::vec2(0.0, ROW_HEIGHT * 0.3),
                                    to_pos - egui::vec2(0.0, ROW_HEIGHT * 0.3),
                                    to_pos,
                                ],
                                false,
                                egui::Color32::TRANSPARENT,
                                egui::Stroke::new(2.0, color),
                            )));
                        }
                    }

                    // Draw node circle
                    let node_pos = graph_rect.min + egui::vec2(
                        node.graph_column as f32 * COLUMN_WIDTH + COLUMN_WIDTH / 2.0,
                        NODE_RADIUS,
                    );
                    let node_color = colors[node.graph_column % colors.len()];
                    painter.circle_filled(node_pos, NODE_RADIUS, node_color);
                }

                // Commit message (truncated)
                let msg = node.commit.message.lines().next().unwrap_or("");
                let truncated = if msg.len() > 40 {
                    format!("{}...", &msg[..40])
                } else {
                    msg.to_string()
                };
                ui.label(egui::RichText::new(truncated)
                    .family(egui::FontFamily::Monospace)
                    .size(12.0));

                // Author
                ui.label(egui::RichText::new(&node.commit.author)
                    .color(egui::Color32::from_rgb(150, 150, 150))
                    .size(11.0));
            });
        }
    }

    /// Render Editor area (Phase 3: full implementation with TextEdit)
    fn render_editor_area(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(ui_colors::SIDEBAR_BG) // #191A1C - Match sidebar background
                    .inner_margin(egui::Margin::same(8.0))
            )
            .show(ctx, |ui| {
            if self.editor_tabs.is_empty() {
                // No file open - show placeholder
                ui.vertical_centered(|ui| {
                    ui.add_space(100.0);
                    ui.heading("BerryCode Editor");
                    ui.add_space(16.0);
                    ui.label("ファイルツリーからファイルを選択してください");
                    ui.add_space(8.0);
                    ui.label(format!("プロジェクト: {}", self.root_path));
                });
                return;
            }

            // Tab bar with close buttons
            let mut tab_to_close: Option<usize> = None;

            ui.horizontal(|ui| {
                // Larger font for tabs
                ui.style_mut().text_styles.insert(
                    egui::TextStyle::Body,
                    egui::FontId::proportional(14.0),
                );

                // Collect tab info first to avoid borrow checker issues
                let tab_info: Vec<(usize, String, &'static str, egui::Color32)> = self.editor_tabs.iter().enumerate().map(|(idx, t)| {
                    let filename = t.file_path.split('/').last().unwrap_or(&t.file_path).to_string();
                    let (icon, color) = Self::get_file_icon_with_color(&filename);
                    (idx, filename, icon, color)
                }).collect();

                for (idx, filename, file_icon, icon_color) in tab_info {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 4.0;

                            // Colored icon
                            ui.label(egui::RichText::new(file_icon).color(icon_color));

                            // Tab label (clickable to switch)
                            let filename_text = egui::RichText::new(&filename)
                                .color(egui::Color32::from_rgb(0xD4, 0xD4, 0xD4));
                            if ui.selectable_label(idx == self.active_tab_idx, filename_text).clicked() {
                                self.active_tab_idx = idx;
                            }

                            // Close button - Codicon: \u{ea76} = codicon-close
                            if ui.small_button("\u{ea76}").clicked() {
                                tab_to_close = Some(idx);
                            }
                        });
                    });
                }
            });

            // Close tab if requested (after the loop to avoid borrow issues)
            if let Some(close_idx) = tab_to_close {
                self.editor_tabs.remove(close_idx);

                // Adjust active tab index
                if self.editor_tabs.is_empty() {
                    self.active_tab_idx = 0;
                } else if self.active_tab_idx >= self.editor_tabs.len() {
                    self.active_tab_idx = self.editor_tabs.len() - 1;
                } else if close_idx <= self.active_tab_idx && self.active_tab_idx > 0 {
                    self.active_tab_idx -= 1;
                }

                tracing::info!("✅ Closed tab at index {}", close_idx);
            }

            // Early return if all tabs are closed
            if self.editor_tabs.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(100.0);
                    ui.heading("BerryCode Editor");
                    ui.add_space(16.0);
                    ui.label("ファイルツリーからファイルを選択してください");
                });
                return;
            }

            ui.separator();

            // Get active tab (after tab bar to avoid borrowing issues)
            let tab = &mut self.editor_tabs[self.active_tab_idx];

            // Editor content
            // Convert Rope to String for egui::TextEdit
            let mut text = tab.buffer.to_string();
            let original_text = text.clone();

            // Detect language from file extension (syntect uses extension, not language name)
            let extension = if tab.file_path.ends_with(".rs") {
                "rs"
            } else if tab.file_path.ends_with(".toml") {
                "toml"
            } else if tab.file_path.ends_with(".md") {
                "md"
            } else if tab.file_path.ends_with(".js") {
                "js"
            } else if tab.file_path.ends_with(".ts") {
                "ts"
            } else if tab.file_path.ends_with(".py") {
                "py"
            } else if tab.file_path.ends_with(".json") {
                "json"
            } else if tab.file_path.ends_with(".yaml") || tab.file_path.ends_with(".yml") {
                "yaml"
            } else {
                "txt"
            };

            // Set language for syntax highlighter (directly on self, not clone)
            match self.syntax_highlighter.set_language(extension) {
                Ok(_) => {
                    tracing::info!("🎨 Syntax highlighting enabled for: {} ({})", tab.file_path, extension);
                }
                Err(e) => {
                    tracing::warn!("⚠️  Failed to set syntax language for {}: {}", extension, e);
                }
            }

            // Clone highlighter AFTER setting the language
            let highlighter = self.syntax_highlighter.clone();

            // Copy color theme (to avoid borrowing issues in layouter closure)
            let color_theme = ColorTheme {
                keyword: self.keyword_color,
                function: self.function_color,
                type_: self.type_color,
                string: self.string_color,
                number: self.number_color,
                comment: self.comment_color,
                doc_comment: self.doc_comment_color,
                macro_: self.macro_color,
                attribute: self.attribute_color,
                constant: self.constant_color,
                lifetime: self.lifetime_color,
                namespace: self.namespace_color,
                variable: self.variable_color,
                operator: self.operator_color,
            };

            // Read-only warning banner
            let is_readonly = tab.is_readonly;
            if is_readonly {
                ui.colored_label(
                    egui::Color32::from_rgb(255, 200, 0),
                    "⚠️ This file is read-only (standard library source)"
                );
                ui.add_space(4.0);
            }

            // Check for pending cursor jump
            let (cursor_range_to_set, scroll_to_y) = if let Some((jump_line, jump_col)) = tab.pending_cursor_jump {
                // Calculate character offset from line/column
                let char_offset = {
                    let mut offset = 0;
                    for (line_idx, line) in text.lines().enumerate() {
                        if line_idx == jump_line {
                            offset += jump_col.min(line.len());
                            break;
                        }
                        offset += line.len() + 1; // +1 for newline
                    }
                    offset
                };

                // Calculate Y position for scrolling
                // Approximate line height (will be refined by TextEdit rendering)
                const APPROX_LINE_HEIGHT: f32 = 19.5; // 13 * 1.5
                let target_y = jump_line as f32 * APPROX_LINE_HEIGHT;

                tracing::info!("📍 Jumping to line {} col {} (char offset: {}, y: {})", jump_line, jump_col, char_offset, target_y);

                // Create cursor range for both primary and secondary cursors at the same position
                (Some(egui::text::CCursorRange::one(egui::text::CCursor::new(char_offset))), Some(target_y))
            } else {
                (None, None)
            };

            // Use ScrollArea for horizontal scrolling
            let scroll_area = egui::ScrollArea::both()
                .auto_shrink([false; 2]);

            let scroll_output = scroll_area.show(ui, |ui| {
                    // Set background color to match sidebar (#191A1C)
                    ui.style_mut().visuals.extreme_bg_color = ui_colors::SIDEBAR_BG;
                    ui.style_mut().visuals.widgets.noninteractive.bg_fill = ui_colors::SIDEBAR_BG;

                    // CRITICAL: Disable text color override to allow syntax highlighting
                    ui.style_mut().visuals.override_text_color = None;

                    let output = egui::TextEdit::multiline(&mut text)
                        .code_editor()
                        .desired_width(f32::INFINITY)
                        .desired_rows(50)
                        .lock_focus(true)
                        .interactive(!is_readonly)
                        .layouter(&mut |ui, text, _wrap_width| {
                            let mut job = Self::syntax_highlight_layouter(ui, text, &highlighter, &color_theme);
                            job.wrap.max_width = f32::INFINITY;
                            ui.fonts(|f| f.layout_job(job))
                        })
                        .show(ui);

                    // Sync changes back to Rope buffer (only if not read-only)
                    if !is_readonly && output.response.changed() && text != original_text {
                        tab.buffer = crate::buffer::TextBuffer::from_str(&text);
                        tracing::info!("✏️ Buffer modified: {} chars", text.len());
                    }

                    // FIX #1: Cmd+Click検出の改善（interact()を使用）
                    let mut go_to_def_data = None;

                    // Cmd/Ctrl+Clickを検出
                    if output.response.interact(egui::Sense::click()).clicked() {
                        if ui.input(|i| i.modifiers.command) {
                            tracing::info!("🖱️ Cmd+Click detected via interact()");

                            if let Some(cr) = output.cursor_range {
                                tracing::info!("📍 Cursor position: {}", cr.primary.ccursor.index);
                                go_to_def_data = Some((text.clone(), cr.primary.ccursor.index));
                            } else {
                                tracing::warn!("⚠️ Cursor range not available");
                            }
                        }
                    }

                    // 代替方法: グローバルinput()でチェック（フォールバック）
                    if go_to_def_data.is_none() {
                        ui.input(|i| {
                            if i.modifiers.command && i.pointer.primary_clicked() {
                                if let Some(pos) = i.pointer.interact_pos() {
                                    if output.response.rect.contains(pos) {
                                        tracing::info!("🖱️ Cmd+Click detected via global input at {:?}", pos);
                                        if let Some(cr) = output.cursor_range {
                                            go_to_def_data = Some((text.clone(), cr.primary.ccursor.index));
                                        }
                                    }
                                }
                            }
                        });
                    }

                    // Sync cursor position (simplified for MVP)
                    if let Some(cursor_range) = output.cursor_range {
                        tracing::debug!("Cursor range: {:?}", cursor_range);
                    }

                    // Manually set cursor if we have a pending jump
                    // Do this AFTER all other operations on output
                    if let Some(cursor_range) = cursor_range_to_set {
                        let response_id = output.response.id;
                        let mut state = output.state.clone();
                        state.cursor.set_char_range(Some(cursor_range));
                        state.store(ui.ctx(), response_id);

                        // Request focus to ensure the TextEdit scrolls to cursor
                        output.response.request_focus();

                        // Force scroll to cursor position
                        if let Some(y) = scroll_to_y {
                            const APPROX_LINE_HEIGHT: f32 = 19.5;
                            // Create a rect at the cursor position
                            let cursor_rect = egui::Rect::from_min_size(
                                egui::pos2(0.0, y),
                                egui::vec2(100.0, APPROX_LINE_HEIGHT * 3.0) // Show a few lines around cursor
                            );
                            // Scroll to make this rect visible
                            ui.scroll_to_rect(cursor_rect, Some(egui::Align::Center));
                            tracing::info!("📜 Scrolling to rect at y={}", y);
                        }
                    }

                    (output, go_to_def_data)
                });

            // If we had a scroll target, ensure we scroll there
            if let Some(y) = scroll_to_y {
                // Force another repaint to ensure scroll takes effect
                ctx.request_repaint();
            }

            // Clear pending cursor jump after rendering
            if let Some(tab) = self.editor_tabs.get_mut(self.active_tab_idx) {
                if tab.pending_cursor_jump.is_some() {
                    tab.pending_cursor_jump = None;
                }
            }

            // Handle go-to-definition outside the closure
            if let Some((text, cursor_pos)) = scroll_output.inner.1 {
                tracing::info!("🔍 Triggering go-to-definition at position {}", cursor_pos);
                self.handle_go_to_definition(&text, cursor_pos);
            }

            // LSP Status bar at bottom
            ui.separator();
            ui.horizontal(|ui| {
                // Connection status
                let status_text = if self.lsp_connected {
                    "🟢 LSP: Connected"
                } else {
                    "🔴 LSP: Disconnected"
                };
                ui.label(status_text);

                ui.separator();

                // Diagnostics count
                ui.label(format!("⚠️ Diagnostics: {}", self.lsp_diagnostics.len()));

                ui.separator();

                // Completion trigger button
                if ui.button("💡 Show Completions (Ctrl+Space)").clicked() {
                    self.trigger_lsp_completions();
                }
            });
        });

        // Handle keyboard shortcuts for LSP
        self.handle_lsp_shortcuts(ctx);

        // Render completion popup
        if self.lsp_show_completions && !self.lsp_completions.is_empty() {
            self.render_lsp_completions(ctx);
        }
    }

    /// Syntax highlighting layouter for egui::TextEdit
    /// Regex-based syntax highlighting with One Dark theme
    fn syntax_highlight_layouter(
        _ui: &egui::Ui,
        text: &str,
        highlighter: &SyntaxHighlighter,
        color_theme: &ColorTheme,
    ) -> egui::text::LayoutJob {
        let mut job = egui::text::LayoutJob::default();

        // Font size: 13px for optimal readability
        const FONT_SIZE: f32 = 13.0;
        // Default color unified white (#D4D4D4)
        let default_color = ui_colors::TEXT_DEFAULT;

        for line in text.lines() {
            // Get tokens from regex-based highlighter
            let tokens = highlighter.highlight_line(line);

            if tokens.is_empty() {
                // No tokens, just add the whole line in default color
                job.append(line, 0.0, egui::TextFormat {
                    font_id: egui::FontId::monospace(FONT_SIZE),
                    color: default_color,
                    ..Default::default()
                });
            } else {
                let mut pos = 0;

                for token in tokens {
                    // Add any text before this token (whitespace, punctuation, etc.)
                    if token.start > pos {
                        let before = &line[pos..token.start];
                        job.append(before, 0.0, egui::TextFormat {
                            font_id: egui::FontId::monospace(FONT_SIZE),
                            color: default_color,
                            ..Default::default()
                        });
                    }

                    // Map TokenType to VS Code Dark+ color scheme
                    let color = match token.token_type {
                        TokenType::Keyword => color_theme.keyword,        // #C586C0 Pink/Purple
                        TokenType::Function => color_theme.function,      // #DCDCAA Yellow
                        TokenType::Type => color_theme.type_,             // #4EC9B0 Cyan/Teal
                        TokenType::String => color_theme.string,          // #CE9178 Orange/Brown
                        TokenType::Number => color_theme.number,          // #B5CEA8 Light Green
                        TokenType::Comment => color_theme.comment,        // #808080 Gray (normal //)
                        TokenType::DocComment => color_theme.doc_comment, // #6A9955 Green (//!, ///)
                        TokenType::Macro => color_theme.macro_,           // #DCDCAA Yellow
                        TokenType::Attribute => color_theme.attribute,    // #C586C0 Pink
                        TokenType::Constant => color_theme.constant,      // #C586C0 Pink
                        TokenType::Lifetime => color_theme.lifetime,      // #4EC9B0 Cyan
                        TokenType::Identifier => color_theme.variable,    // #9CDCFE Light Blue (identifiers as variables)
                        TokenType::Namespace => default_color,            // #D4D4D4 White
                        TokenType::Operator => color_theme.operator,      // #D4D4D4 White
                        TokenType::EscapeSequence => color_theme.string,  // #CE9178 Orange/Brown
                    };

                    job.append(&token.text, 0.0, egui::TextFormat {
                        font_id: egui::FontId::monospace(FONT_SIZE),
                        color,
                        ..Default::default()
                    });

                    pos = token.end;
                }

                // Add any remaining text at the end of the line
                if pos < line.len() {
                    let remaining = &line[pos..];
                    job.append(remaining, 0.0, egui::TextFormat {
                        font_id: egui::FontId::monospace(FONT_SIZE),
                        color: default_color,
                        ..Default::default()
                    });
                }
            }

            // Add newline
            job.append("\n", 0.0, egui::TextFormat {
                font_id: egui::FontId::monospace(FONT_SIZE),
                color: default_color,
                ..Default::default()
            });
        }

        job
    }

    /// Render Status Bar (bottom)
    fn render_status_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status_bar")
            .exact_height(24.0)
            .frame(
                egui::Frame::none()
                    .fill(ui_colors::SIDEBAR_BG) // #191A1C
                    .inner_margin(egui::Margin::symmetric(8.0, 2.0))
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("BerryEditor");
                    ui.separator();
                    ui.label(format!("📁 {}", self.root_path));
                    ui.separator();
                    ui.label(format!("ファイル数: {}", self.file_tree_cache.len()));

                    // LSP connection status
                    ui.separator();
                    let status_text = if self.lsp_connected {
                        "🟢 LSP: Connected | F12: Definition | Shift+F12: References | Cmd+Click: Jump"
                    } else {
                        "🔴 LSP: Disconnected | Regex search only"
                    };
                    ui.label(status_text);

                    // Diagnostics count
                    if !self.lsp_diagnostics.is_empty() {
                        ui.separator();
                        ui.label(format!("⚠️ {}", self.lsp_diagnostics.len()));
                    }

                    // Status message display (auto-clear after 3 seconds)
                    if !self.status_message.is_empty() {
                        if let Some(timestamp) = self.status_message_timestamp {
                            if timestamp.elapsed().as_secs() < 3 {
                                ui.separator();
                                ui.label(&self.status_message);
                            } else {
                                self.status_message.clear();
                                self.status_message_timestamp = None;
                            }
                        }
                    }

                    // Read-only warning
                    if let Some(tab) = self.editor_tabs.get(self.active_tab_idx) {
                        if tab.is_readonly {
                            ui.separator();
                            ui.label(egui::RichText::new("📖 READ-ONLY")
                                .color(egui::Color32::from_rgb(255, 200, 0)));
                        }

                        ui.separator();

                        // Language indicator
                        let lang = if tab.file_path.ends_with(".rs") {
                            "Rust"
                        } else if tab.file_path.ends_with(".toml") {
                            "TOML"
                        } else if tab.file_path.ends_with(".md") {
                            "Markdown"
                        } else {
                            "Plain Text"
                        };
                        ui.label(format!("言語: {}", lang));

                        // Format button (only for supported languages)
                        if tab.file_path.ends_with(".rs") {
                            ui.separator();
                            if ui.button("Format (Cmd+Shift+F)").clicked() {
                                self.format_current_file();
                            }
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label("egui 0.29 + Native");
                    });
                });
            });
    }

    /// RustRover-style Settings Panel
    fn render_settings_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("⚙️ Settings");
        ui.separator();

        ui.horizontal_top(|ui| {
            // --- Left Navigation (150px width) ---
            ui.vertical(|ui| {
                ui.set_width(150.0);
                ui.add_space(8.0);

                ui.selectable_value(&mut self.active_settings_tab, SettingsTab::Appearance, "Appearance");
                ui.selectable_value(&mut self.active_settings_tab, SettingsTab::EditorColor, "Editor > Color Scheme");

                ui.add_space(12.0);
                ui.label(egui::RichText::new("Plugins").small().color(egui::Color32::GRAY));
                ui.selectable_value(&mut self.active_settings_tab, SettingsTab::Slack, "Slack API");
                ui.selectable_value(&mut self.active_settings_tab, SettingsTab::GitHub, "GitHub Review");
                ui.selectable_value(&mut self.active_settings_tab, SettingsTab::Plugins, "Other Plugins");
            });

            ui.separator();

            // --- Right Content Area ---
            ui.vertical(|ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        match self.active_settings_tab {
                            SettingsTab::EditorColor => {
                                self.render_color_scheme_settings(ui);
                            }
                            SettingsTab::Appearance => {
                                ui.heading("Appearance");
                                ui.label("Window theme, font settings, etc.");
                                ui.label("Coming soon...");
                            }
                            SettingsTab::Slack => {
                                ui.heading("Slack Integration");
                                ui.label("Backup folder features integrated.");
                                ui.label("Token inputs coming soon...");
                            }
                            SettingsTab::GitHub => {
                                ui.heading("GitHub Code Review");
                                ui.label("Pull request review features.");
                                ui.label("Coming soon...");
                            }
                            SettingsTab::Plugins => {
                                ui.heading("Other Plugins");
                                ui.label("Additional plugin configurations.");
                                ui.label("Coming soon...");
                            }
                        }
                    });
            });
        });
    }

    /// Color Scheme Settings (RustRover Darcula)
    fn render_color_scheme_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("Color Scheme: Darcula (Customized)");
        ui.label("Customize syntax highlighting colors:");
        ui.add_space(8.0);

        // Color edit rows (inline to avoid borrowing issues)
        ui.horizontal(|ui| {
            ui.color_edit_button_srgba(&mut self.keyword_color);
            ui.label("Keyword (fn, let, match)");
        });
        ui.horizontal(|ui| {
            ui.color_edit_button_srgba(&mut self.function_color);
            ui.label("Function / Macro");
        });
        ui.horizontal(|ui| {
            ui.color_edit_button_srgba(&mut self.type_color);
            ui.label("Type (struct, enum)");
        });
        ui.horizontal(|ui| {
            ui.color_edit_button_srgba(&mut self.string_color);
            ui.label("String");
        });
        ui.horizontal(|ui| {
            ui.color_edit_button_srgba(&mut self.number_color);
            ui.label("Number");
        });
        ui.horizontal(|ui| {
            ui.color_edit_button_srgba(&mut self.comment_color);
            ui.label("Comment");
        });
        ui.horizontal(|ui| {
            ui.color_edit_button_srgba(&mut self.macro_color);
            ui.label("Macro (println!)");
        });
        ui.horizontal(|ui| {
            ui.color_edit_button_srgba(&mut self.attribute_color);
            ui.label("Attribute (#[derive])");
        });
        ui.horizontal(|ui| {
            ui.color_edit_button_srgba(&mut self.constant_color);
            ui.label("Constant (STATIC)");
        });
        ui.horizontal(|ui| {
            ui.color_edit_button_srgba(&mut self.lifetime_color);
            ui.label("Lifetime ('a, 'static)");
        });

        ui.add_space(20.0);
        ui.separator();
        ui.label(egui::RichText::new("Live Preview:").strong());
        ui.add_space(8.0);
        self.render_color_preview(ui);

        ui.add_space(16.0);
        if ui.button("🔄 Reset to Darcula Defaults").clicked() {
            self.reset_colors_to_darcula();
        }
    }

    /// Live preview of syntax colors
    fn render_color_preview(&self, ui: &mut egui::Ui) {
        let frame = egui::Frame::none()
            .fill(ui_colors::SIDEBAR_BG) // #191A1C - Match sidebar background
            .inner_margin(12.0)
            .rounding(4.0);

        frame.show(ui, |ui| {
            ui.style_mut().override_font_id = Some(egui::FontId::monospace(13.0));

            // Line 1: fn main() {
            ui.horizontal(|ui| {
                ui.colored_label(self.keyword_color, "fn");
                ui.label(" ");
                ui.colored_label(self.function_color, "main");
                ui.label("() {");
            });

            // Line 2: let x: u32 = 42;
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.colored_label(self.keyword_color, "let");
                ui.label(" x: ");
                ui.colored_label(self.type_color, "u32");
                ui.label(" = ");
                ui.colored_label(self.number_color, "42");
                ui.label(";");
            });

            // Line 3: // Comment
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.colored_label(self.comment_color, "// Hello World");
            });

            // Line 4: println!("Ready!");
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.colored_label(self.macro_color, "println!");
                ui.label("(");
                ui.colored_label(self.string_color, "\"Ready!\"");
                ui.label(");");
            });

            // Line 5: const MAX: usize = 100;
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.colored_label(self.keyword_color, "const");
                ui.label(" ");
                ui.colored_label(self.constant_color, "MAX");
                ui.label(": ");
                ui.colored_label(self.type_color, "usize");
                ui.label(" = ");
                ui.colored_label(self.number_color, "100");
                ui.label(";");
            });

            // Line 6: }
            ui.label("}");
        });
    }

    /// Reset colors to VS Code Dark+ defaults
    fn reset_colors_to_darcula(&mut self) {
        // Use named color constants from syntax_colors module
        self.keyword_color = syntax_colors::KEYWORD;
        self.function_color = syntax_colors::FUNCTION;
        self.type_color = syntax_colors::TYPE;
        self.string_color = syntax_colors::STRING;
        self.number_color = syntax_colors::NUMBER;
        self.comment_color = syntax_colors::COMMENT;
        self.doc_comment_color = syntax_colors::DOC_COMMENT;
        self.macro_color = syntax_colors::MACRO;
        self.attribute_color = syntax_colors::ATTRIBUTE;
        self.constant_color = syntax_colors::CONSTANT;
        self.lifetime_color = syntax_colors::LIFETIME;
        self.namespace_color = syntax_colors::NAMESPACE;
        self.variable_color = syntax_colors::VARIABLE;
        self.operator_color = syntax_colors::OPERATOR;
        tracing::info!("🎨 Reset colors to VS Code Dark+ defaults");
    }
}

impl eframe::App for BerryCodeApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {

        // Initialize Git repository on first update
        if !self.git_initialized {
            self.git_initialized = true;
            self.refresh_git_status();
            self.refresh_git_history();
            self.refresh_git_branches();
            self.refresh_git_remotes();
            self.refresh_git_tags();
            self.refresh_git_stashes();
            tracing::info!("✅ Git repository initialized for {}", self.root_path);
        }

        // Poll LSP responses (non-blocking)
        self.poll_lsp_responses();

        // Poll gRPC responses (non-blocking)
        self.poll_grpc_responses();

        // Poll Slack responses (non-blocking)
        self.poll_slack_responses();

        // Poll file watcher events (non-blocking)
        self.poll_file_watcher_events();

        // Handle keyboard shortcuts
        self.handle_editor_shortcuts(ctx);
        self.handle_goto_definition_shortcut(ctx);
        self.handle_find_references_shortcut(ctx);
        self.handle_settings_shortcuts(ctx);

        // Render top header bar (VS Code style)
        self.render_top_header(ctx);

        // Render UI panels
        self.render_activity_bar(ctx);

        // Conditional panels based on active panel
        if self.active_panel == ActivePanel::Chat {
            // Chat mode: Full Slack-like interface (no sidebar, no editor)
            self.render_chat_panel(ctx);
        } else if self.active_panel == ActivePanel::Workflow {
            // Workflow mode: Sidebar + Canvas (center) + Logs (right)
            self.render_sidebar(ctx);
            self.render_workflow_canvas(ctx);
            self.render_workflow_logs_panel(ctx);
        } else if self.active_panel == ActivePanel::Wiki {
            // Wiki mode: Sidebar (page list) + Center (wiki content)
            self.render_sidebar(ctx);
            self.render_wiki_content(ctx);
        } else if self.active_panel == ActivePanel::Terminal {
            // Terminal mode: Full-screen iTerm2-like terminal (no sidebar)
            self.render_terminal_fullscreen(ctx);
        } else if self.active_panel == ActivePanel::Database {
            // Database mode: Full database management interface
            self.render_database_panel(ctx);
        } else if self.active_panel == ActivePanel::Git {
            // Git mode: Sidebar (status/tree) + Center (diff viewer) + AI Chat (optional)
            self.render_sidebar(ctx);
            self.render_git_diff_viewer(ctx);
        } else {
            // Normal mode: Sidebar + Editor (center) + AI Chat (right panel)
            self.render_sidebar(ctx);
            self.render_ai_chat_panel(ctx);
            self.render_editor_area(ctx);
        }

        // ✅ Phase 6.2: Render diagnostics panel (before status bar so it appears above)
        if !self.lsp_diagnostics.is_empty() {
            self.render_diagnostics_panel(ctx);
        }

        self.render_status_bar(ctx);

        // Render search dialog if open
        if self.search_dialog_open {
            self.render_search_dialog(ctx);
        }

        // Render settings dialog
        if self.show_settings {
            self.render_settings_dialog(ctx);
        }

        // Render theme editor
        if self.show_theme_editor {
            self.render_theme_editor(ctx);
        }

        // Render LSP hover tooltip
        if self.lsp_show_hover {
            self.render_lsp_hover(ctx);
        }

        // Render definition picker window
        if self.show_definition_picker {
            self.render_definition_picker(ctx);
        }

        // Render references panel
        if self.show_references_panel {
            self.render_references_panel(ctx);
        }

        // FIX #3: Reactive Mode - ステータスメッセージがある場合のみ再描画
        if self.status_message_timestamp.is_some() {
            // ステータスメッセージは3秒で消えるので、その間だけ再描画
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }
}

impl BerryCodeApp {
    /// Handle global keyboard shortcuts
    fn handle_editor_shortcuts(&mut self, ctx: &egui::Context) {
        // Only handle shortcuts when editor is focused
        if self.active_focus != FocusLayer::Editor {
            return;
        }

        // Skip if no tabs open
        if self.editor_tabs.is_empty() {
            return;
        }

        ctx.input(|i| {
            // Ctrl+F / Cmd+F: Open search dialog
            if i.modifiers.command && !i.modifiers.shift && i.key_pressed(egui::Key::F) {
                self.search_dialog_open = true;
                self.show_replace = false;
                self.search_results.clear();
            }

            // Ctrl+H / Cmd+H: Open replace dialog
            if i.modifiers.command && i.key_pressed(egui::Key::H) {
                self.search_dialog_open = true;
                self.show_replace = true;
                self.search_results.clear();
            }

            // Ctrl+S / Cmd+S: Save file
            if i.modifiers.command && i.key_pressed(egui::Key::S) {
                self.save_current_file();
            }

            // Ctrl+Shift+F / Cmd+Shift+F: Format file
            if i.modifiers.command && i.modifiers.shift && i.key_pressed(egui::Key::F) {
                self.format_current_file();
            }

            // Ctrl+Z / Cmd+Z: Undo (not Shift)
            if i.modifiers.command && !i.modifiers.shift && i.key_pressed(egui::Key::Z) {
                if let Some(tab) = self.editor_tabs.get_mut(self.active_tab_idx) {
                    // Note: EditorTab::undo() is private, so we'll implement simplified undo
                    // For MVP, just log for now - full undo/redo requires EditorAction integration
                    tracing::info!("⏪ Undo requested (full implementation in later phase)");
                }
            }

            // Ctrl+Y / Cmd+Shift+Z: Redo
            if (i.modifiers.command && i.key_pressed(egui::Key::Y))
                || (i.modifiers.command && i.modifiers.shift && i.key_pressed(egui::Key::Z))
            {
                if let Some(tab) = self.editor_tabs.get_mut(self.active_tab_idx) {
                    tracing::info!("⏩ Redo requested (full implementation in later phase)");
                }
            }

            // Note: Ctrl+C/V/X are handled by egui::TextEdit automatically
        });
    }

    /// Save current file
    fn save_current_file(&mut self) {
        if let Some(tab) = self.editor_tabs.get(self.active_tab_idx) {
            let content = tab.buffer.to_string();
            match native::fs::write_file(&tab.file_path, &content) {
                Ok(_) => {
                    tracing::info!("💾 File saved: {} ({} bytes)", tab.file_path, content.len());
                    // TODO: Update dirty state

                    // NOTE: Diagnostics disabled - requires Tokio runtime
                    // self.request_diagnostics();
                }
                Err(e) => {
                    tracing::error!("❌ Failed to save file {}: {}", tab.file_path, e);
                }
            }
        }
    }

    /// Format current file using language-specific formatter
    fn format_current_file(&mut self) {
        if let Some(tab) = self.editor_tabs.get_mut(self.active_tab_idx) {
            tracing::info!("✨ Formatting file: {}", tab.file_path);

            // Save file first
            let content = tab.buffer.to_string();
            if let Err(e) = native::fs::write_file(&tab.file_path, &content) {
                tracing::error!("❌ Failed to save before formatting: {}", e);
                return;
            }

            // Run formatter based on file extension
            let formatter_result = if tab.file_path.ends_with(".rs") {
                // Use rustfmt
                std::process::Command::new("rustfmt")
                    .arg(&tab.file_path)
                    .output()
            } else {
                tracing::warn!("⚠️  No formatter configured for {}", tab.file_path);
                return;
            };

            match formatter_result {
                Ok(output) => {
                    if output.status.success() {
                        // Reload formatted file
                        match native::fs::read_file(&tab.file_path) {
                            Ok(formatted_content) => {
                                tab.buffer = crate::buffer::TextBuffer::from_str(&formatted_content);
                                tracing::info!("✅ File formatted successfully");

                                self.terminal_output.push(TerminalLine {
                                    text: format!("✅ Formatted: {}", tab.file_path),
                                    style: TerminalStyle::Output,
                                });
                            }
                            Err(e) => {
                                tracing::error!("❌ Failed to reload formatted file: {}", e);
                            }
                        }
                    } else {
                        let error_msg = String::from_utf8_lossy(&output.stderr);
                        tracing::error!("❌ Formatter error: {}", error_msg);

                        self.terminal_output.push(TerminalLine {
                            text: format!("❌ Format error: {}", error_msg),
                            style: TerminalStyle::Error,
                        });
                    }
                }
                Err(e) => {
                    tracing::error!("❌ Failed to run formatter: {}", e);
                    self.terminal_output.push(TerminalLine {
                        text: format!("❌ Failed to run rustfmt: {}", e),
                        style: TerminalStyle::Error,
                    });
                }
            }
        }
    }

    /// Render search dialog
    fn render_search_dialog(&mut self, ctx: &egui::Context) {
        let mut close_dialog = false;

        let window_title = if self.show_replace {
            "🔍 Find & Replace"
        } else {
            "🔍 Search"
        };

        egui::Window::new(window_title)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_TOP, [0.0, 100.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Find:");
                    let response = ui.text_edit_singleline(&mut self.search_query);

                    // Auto-focus on open
                    if self.search_results.is_empty() && !self.search_query.is_empty() {
                        response.request_focus();
                    }

                    // Search on Enter
                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.perform_search();
                        response.request_focus();
                    }

                    if ui.button("Search").clicked() {
                        self.perform_search();
                    }
                });

                // Replace input field (only show in replace mode)
                if self.show_replace {
                    ui.horizontal(|ui| {
                        ui.label("Replace:");
                        ui.text_edit_singleline(&mut self.replace_query);

                        if ui.button("Replace").clicked() {
                            self.perform_replace_current();
                        }

                        if ui.button("Replace All").clicked() {
                            self.perform_replace_all();
                        }
                    });
                }

                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.search_case_sensitive, "Case sensitive");
                });

                ui.separator();

                // Display search results
                if !self.search_results.is_empty() {
                    ui.label(format!(
                        "Found {} matches (showing {}/{})",
                        self.search_results.len(),
                        self.current_search_index + 1,
                        self.search_results.len()
                    ));

                    ui.horizontal(|ui| {
                        if ui.button("⬆ Previous").clicked() {
                            self.go_to_previous_match();
                        }
                        if ui.button("⬇ Next").clicked() {
                            self.go_to_next_match();
                        }
                    });

                    ui.separator();

                    // Show all results in a scrollable list
                    let mut clicked_index: Option<usize> = None;

                    egui::ScrollArea::vertical()
                        .max_height(300.0)
                        .show(ui, |ui| {
                            for (idx, match_result) in self.search_results.iter().enumerate() {
                                let is_current = idx == self.current_search_index;

                                // Format the display text
                                let display_text = if let Some(file_path) = &match_result.file_path {
                                    // Project-wide search: show file path and line
                                    let filename = file_path.split('/').last().unwrap_or(file_path);
                                    format!("{}:{}: {}", filename, match_result.line_number + 1, match_result.line_text.trim())
                                } else {
                                    // In-file search: just show line number
                                    format!("Line {}: {}", match_result.line_number + 1, match_result.line_text.trim())
                                };

                                // Make each result clickable
                                let response = ui.selectable_label(is_current, display_text);

                                if response.clicked() {
                                    clicked_index = Some(idx);
                                }
                            }
                        });

                    // Jump to clicked result (outside the borrow)
                    if let Some(idx) = clicked_index {
                        self.current_search_index = idx;
                        self.jump_to_current_match();
                    }
                } else if !self.search_query.is_empty() {
                    ui.label("No matches found");
                }

                ui.separator();

                if ui.button("Close").clicked() {
                    close_dialog = true;
                }

                // ESC to close
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    close_dialog = true;
                }
            });

        if close_dialog {
            self.search_dialog_open = false;
            self.search_results.clear();
            self.search_query.clear();
        }
    }

    /// Perform search in current file
    fn perform_search(&mut self) {
        self.search_results.clear();
        self.current_search_index = 0;

        if self.search_query.is_empty() || self.editor_tabs.is_empty() {
            return;
        }

        let tab = &self.editor_tabs[self.active_tab_idx];
        let content = tab.buffer.to_string();

        let query = if self.search_case_sensitive {
            self.search_query.clone()
        } else {
            self.search_query.to_lowercase()
        };

        for (line_number, line) in content.lines().enumerate() {
            let search_line = if self.search_case_sensitive {
                line.to_string()
            } else {
                line.to_lowercase()
            };

            let mut start_pos = 0;
            while let Some(pos) = search_line[start_pos..].find(&query) {
                let actual_pos = start_pos + pos;
                self.search_results.push(SearchMatch {
                    file_path: None, // In-file search doesn't need file_path
                    line_number,
                    start_col: actual_pos,
                    end_col: actual_pos + self.search_query.len(),
                    line_text: line.to_string(),
                });
                start_pos = actual_pos + 1;
            }
        }

        tracing::info!("🔍 Search found {} matches for '{}'", self.search_results.len(), self.search_query);

        // Jump to first match if any results found
        if !self.search_results.is_empty() {
            self.jump_to_current_match();
        }
    }

    /// Perform project-wide search using native::search
    fn perform_project_search(&mut self) {
        self.search_results.clear();
        self.current_search_index = 0;

        if self.search_query.is_empty() {
            return;
        }

        tracing::info!("🔍 Starting project-wide search for '{}' in {}", self.search_query, self.root_path);

        // Use native::search::search_in_files() for parallel search
        match native::search::search_in_files(
            &self.root_path,
            &self.search_query,
            self.search_case_sensitive,
            false, // use_regex: false (literal search)
        ) {
            Ok(results) => {
                // Convert native::search::SearchResult to our SearchMatch
                self.search_results = results
                    .into_iter()
                    .map(|r| SearchMatch {
                        file_path: Some(r.file_path),
                        line_number: r.line_number - 1, // native returns 1-based, we use 0-based
                        start_col: r.match_start,
                        end_col: r.match_end,
                        line_text: r.line_content,
                    })
                    .collect();

                tracing::info!(
                    "🔍 Project search found {} matches for '{}'",
                    self.search_results.len(),
                    self.search_query
                );

                // Jump to first match if any results found
                if !self.search_results.is_empty() {
                    self.jump_to_current_match();
                }
            }
            Err(e) => {
                tracing::error!("❌ Project search failed: {}", e);
                // Add error message to terminal output
                self.terminal_output.push(TerminalLine {
                    text: format!("Search error: {}", e),
                    style: TerminalStyle::Error,
                });
            }
        }
    }

    /// Go to next search match
    fn go_to_next_match(&mut self) {
        if self.search_results.is_empty() {
            return;
        }

        self.current_search_index = (self.current_search_index + 1) % self.search_results.len();
        tracing::info!("🔍 Next match: {}/{}", self.current_search_index + 1, self.search_results.len());

        // Jump to the match location
        self.jump_to_current_match();
    }

    /// Go to previous search match
    fn go_to_previous_match(&mut self) {
        if self.search_results.is_empty() {
            return;
        }

        if self.current_search_index == 0 {
            self.current_search_index = self.search_results.len() - 1;
        } else {
            self.current_search_index -= 1;
        }
        tracing::info!("🔍 Previous match: {}/{}", self.current_search_index + 1, self.search_results.len());

        // Jump to the match location
        self.jump_to_current_match();
    }

    /// Jump to the current search match location
    fn jump_to_current_match(&mut self) {
        // Clone the match result to avoid borrowing issues
        let match_result = if let Some(m) = self.search_results.get(self.current_search_index) {
            m.clone()
        } else {
            return;
        };

        // If this is a project-wide search result with a file path, open that file first
        if let Some(file_path) = &match_result.file_path {
            // Check if the file is already open
            let file_already_open = self.editor_tabs
                .iter()
                .position(|tab| tab.file_path == *file_path);

            if let Some(tab_idx) = file_already_open {
                // File is already open, just switch to it
                self.active_tab_idx = tab_idx;
            } else {
                // Open the file
                self.open_file_from_path(file_path);
            }
        }

        // Set cursor position to the match location
        if let Some(tab) = self.editor_tabs.get_mut(self.active_tab_idx) {
            tab.cursor_line = match_result.line_number;
            tab.cursor_col = match_result.start_col;

            tracing::info!(
                "⚡ Jumped to {}:{}:{}",
                tab.file_path.split('/').last().unwrap_or(&tab.file_path),
                match_result.line_number + 1,
                match_result.start_col + 1
            );
        }
    }

    /// Replace current search match
    fn perform_replace_current(&mut self) {
        if self.search_results.is_empty() || self.editor_tabs.is_empty() {
            return;
        }

        let match_result = if let Some(m) = self.search_results.get(self.current_search_index) {
            m.clone()
        } else {
            return;
        };

        // Get current tab
        if let Some(tab) = self.editor_tabs.get_mut(self.active_tab_idx) {
            let content = tab.buffer.to_string();
            let lines: Vec<&str> = content.lines().collect();

            if match_result.line_number >= lines.len() {
                return;
            }

            let line = lines[match_result.line_number];
            let before = &line[..match_result.start_col];
            let after = &line[match_result.end_col..];
            let new_line = format!("{}{}{}", before, self.replace_query, after);

            // Replace the line in the buffer
            let mut new_lines: Vec<String> = lines.iter().map(|s| s.to_string()).collect();
            new_lines[match_result.line_number] = new_line;

            let new_content = new_lines.join("\n");
            tab.buffer = TextBuffer::from_str(&new_content);
            tab.mark_dirty();

            tracing::info!(
                "✏️  Replaced '{}' with '{}' at line {}",
                self.search_query,
                self.replace_query,
                match_result.line_number + 1
            );

            // Remove this match and move to next
            self.search_results.remove(self.current_search_index);
            if !self.search_results.is_empty() && self.current_search_index >= self.search_results.len() {
                self.current_search_index = 0;
            }

            // Re-run search to update matches
            self.perform_search();
        }
    }

    /// Replace all search matches
    fn perform_replace_all(&mut self) {
        if self.search_results.is_empty() || self.editor_tabs.is_empty() {
            return;
        }

        let tab = &mut self.editor_tabs[self.active_tab_idx];
        let content = tab.buffer.to_string();

        // Perform replace using simple string replacement
        let new_content = if self.search_case_sensitive {
            content.replace(&self.search_query, &self.replace_query)
        } else {
            // Case-insensitive replacement
            let mut result = content.clone();
            let query_lower = self.search_query.to_lowercase();
            let mut start = 0;

            while let Some(pos) = result[start..].to_lowercase().find(&query_lower) {
                let actual_pos = start + pos;
                result.replace_range(actual_pos..actual_pos + self.search_query.len(), &self.replace_query);
                start = actual_pos + self.replace_query.len();
            }
            result
        };

        let count = self.search_results.len();
        tab.buffer = TextBuffer::from_str(&new_content);
        tab.mark_dirty();

        tracing::info!(
            "✏️  Replaced {} occurrences of '{}' with '{}'",
            count,
            self.search_query,
            self.replace_query
        );

        // Clear search results
        self.search_results.clear();
        self.current_search_index = 0;
    }

    /// Handle Cmd+Click go-to-definition (Hybrid: LSP priority + regex fallback)
    fn handle_go_to_definition(&mut self, text: &str, cursor_pos: usize) {
        // Extract word at cursor position
        let word = self.extract_word_at_position(text, cursor_pos);
        if word.is_empty() {
            tracing::debug!("No word found at cursor position");
            return;
        }

        tracing::info!("🔍 Looking for definition of: '{}'", word);

        // Get current file path
        let current_file = match self.editor_tabs.get(self.active_tab_idx) {
            Some(tab) => tab.file_path.clone(),
            None => return,
        };

        let (line, column) = calculate_line_column(text, cursor_pos);

        // PHASE 1: Try LSP first (if connected)
        if self.lsp_connected && self.lsp_client.is_some() {
            tracing::info!("🚀 Requesting LSP goto_definition for '{}' at {}:{}", word, line, column);
            self.spawn_goto_definition_request(current_file, line, column);

            // Save context for fallback if LSP fails
            self.pending_goto_definition = Some(PendingGotoDefinition {
                word: word.clone(),
                original_text: text.to_string(),
            });

            return;
        }

        // PHASE 2: LSP not connected → use regex fallback
        tracing::info!("📝 LSP unavailable, using local regex search");
        self.fallback_goto_definition(text, &word);
    }

    /// Regex-based local search (fallback when LSP unavailable)
    fn fallback_goto_definition(&mut self, text: &str, word: &str) {
        // Search for definition patterns in the current file first
        let patterns = vec![
            format!(r"fn\s+{}\s*\(", word),           // fn word_name(
            format!(r"pub\s+fn\s+{}\s*\(", word),     // pub fn word_name(
            format!(r"struct\s+{}\s*[{{<]", word),    // struct WordName { or struct WordName<
            format!(r"pub\s+struct\s+{}\s*[{{<]", word), // pub struct WordName
            format!(r"enum\s+{}\s*[{{<]", word),      // enum WordName
            format!(r"pub\s+enum\s+{}\s*[{{<]", word), // pub enum WordName
            format!(r"trait\s+{}\s*[{{<]", word),     // trait WordName
            format!(r"pub\s+trait\s+{}\s*[{{<]", word), // pub trait WordName
            format!(r"type\s+{}\s*=", word),          // type WordName =
            format!(r"const\s+{}\s*:", word),         // const WORD_NAME:
            format!(r"static\s+{}\s*:", word),        // static WORD_NAME:
            format!(r"impl\s+{}\s*[{{<]", word),      // impl WordName
            format!(r"impl.*for\s+{}\s*[{{<]", word), // impl Trait for WordName
        ];

        // Search in current file
        for (line_idx, line) in text.lines().enumerate() {
            for pattern in &patterns {
                if let Ok(regex) = regex::Regex::new(pattern) {
                    if regex.is_match(line) {
                        tracing::info!("✅ Found definition at line {}: {}", line_idx + 1, line.trim());

                        // Jump to the definition (using pending jump for next frame)
                        if let Some(tab) = self.editor_tabs.get_mut(self.active_tab_idx) {
                            tab.cursor_line = line_idx;
                            tab.cursor_col = 0;
                            tab.pending_cursor_jump = Some((line_idx, 0));
                            tracing::info!("⏭️ Scheduled cursor jump to line {}", line_idx);
                        }
                        return;
                    }
                }
            }
        }

        // If not found in current file, search across project
        tracing::info!("🔍 Searching in project for '{}'", word);
        self.search_definition_in_project(word);
    }

    /// Extract word at cursor position
    fn extract_word_at_position(&self, text: &str, pos: usize) -> String {
        if pos > text.len() {
            return String::new();
        }

        let chars: Vec<char> = text.chars().collect();
        if pos >= chars.len() {
            return String::new();
        }

        // Find start of word (move backwards)
        let mut start = pos;
        while start > 0 {
            let ch = chars[start - 1];
            if !ch.is_alphanumeric() && ch != '_' {
                break;
            }
            start -= 1;
        }

        // Find end of word (move forwards)
        let mut end = pos;
        while end < chars.len() {
            let ch = chars[end];
            if !ch.is_alphanumeric() && ch != '_' {
                break;
            }
            end += 1;
        }

        chars[start..end].iter().collect()
    }

    /// Search for definition across the project
    fn search_definition_in_project(&mut self, word: &str) {
        // Search for function, struct, enum, trait definitions
        // Try both pub and non-pub variants separately
        let search_patterns = vec![
            // pub variants
            format!(r"pub fn {}", word),
            format!(r"pub struct {}", word),
            format!(r"pub enum {}", word),
            format!(r"pub trait {}", word),
            format!(r"pub type {}", word),
            format!(r"pub const {}", word),
            // non-pub variants
            format!(r"fn {}", word),
            format!(r"struct {}", word),
            format!(r"enum {}", word),
            format!(r"trait {}", word),
            format!(r"type {}", word),
            format!(r"const {}", word),
        ];

        for pattern in search_patterns {
            match native::search::search_in_files(
                &self.root_path,
                &pattern,
                false, // case_sensitive
                true,  // use_regex
            ) {
                Ok(results) => {
                    if !results.is_empty() {
                        // Found definition(s), jump to the first one
                        let first_result = &results[0];

                        tracing::info!(
                            "✅ Found definition in {}: line {}",
                            first_result.file_path,
                            first_result.line_number
                        );

                        // Open file and jump to definition
                        let file_path = first_result.file_path.clone();
                        let line_number = first_result.line_number - 1; // Convert to 0-based

                        // Check if file is already open
                        let file_already_open = self.editor_tabs
                            .iter()
                            .position(|tab| tab.file_path == file_path);

                        if let Some(tab_idx) = file_already_open {
                            self.active_tab_idx = tab_idx;
                        } else {
                            self.open_file_from_path(&file_path);
                        }

                        // Set cursor to definition line (using pending jump for next frame)
                        if let Some(tab) = self.editor_tabs.get_mut(self.active_tab_idx) {
                            tab.cursor_line = line_number;
                            tab.cursor_col = 0;
                            tab.pending_cursor_jump = Some((line_number, 0));
                            tracing::info!("⏭️ Scheduled cursor jump to line {} in {}", line_number, file_path);
                        }

                        return;
                    }
                }
                Err(e) => {
                    tracing::error!("❌ Search error: {}", e);
                }
            }
        }

        tracing::warn!("⚠️ Definition not found for '{}'", word);
    }

    // ====================================================================
    // LSP Go-to-Definition Support (Phase 2)
    // ====================================================================

    /// Spawn LSP goto_definition request asynchronously
    fn spawn_goto_definition_request(&self, file_path: String, line: usize, column: usize) {
        let client = match &self.lsp_client {
            Some(c) => std::sync::Arc::clone(c),
            None => {
                tracing::warn!("⚠️ LSP client not initialized");
                return;
            }
        };

        let tx = match &self.lsp_response_tx {
            Some(t) => t.clone(),
            None => return,
        };

        let runtime = std::sync::Arc::clone(&self.lsp_runtime);

        // Spawn async task to request goto_definition from LSP server
        runtime.spawn(async move {
            tracing::info!("🚀 Requesting LSP goto_definition");
            tracing::info!("   File: {}", file_path);
            tracing::info!("   Position: line={}, column={}", line, column);

            match client.goto_definition("rust", file_path.clone(), line as u32, column as u32).await {
                Ok(locations) => {
                    tracing::info!("📍 LSP returned {} locations", locations.len());
                    for (i, loc) in locations.iter().enumerate() {
                        tracing::info!("   Location {}: {}", i + 1, loc.uri);
                    }

                    // Convert proto Location → LspLocation
                    let lsp_locations: Vec<LspLocation> = locations
                        .into_iter()
                        .filter_map(parse_lsp_location)
                        .collect();

                    if let Err(e) = tx.send(LspResponse::Definition(lsp_locations)) {
                        tracing::error!("❌ Failed to send LSP response: {}", e);
                    }
                }
                Err(e) => {
                    tracing::warn!("⚠️ LSP goto_definition failed: {} (will use fallback)", e);
                    // Send empty response to trigger fallback
                    let _ = tx.send(LspResponse::Definition(vec![]));
                }
            }
        });
    }

    /// Navigate to a specific location (file + line + column)
    fn navigate_to_location(&mut self, location: &LspLocation) {
        tracing::info!("📍 Navigating to location:");
        tracing::info!("   File: {}", location.file_path);
        tracing::info!("   Line: {}, Column: {}", location.line, location.column);

        // Detect if this is a standard library file
        let is_stdlib = location.file_path.contains("/.rustup/")
            || location.file_path.contains("\\.rustup\\");

        if is_stdlib {
            tracing::info!("📖 Detected standard library file");
        }

        // Check if file is already open
        let file_already_open = self.editor_tabs
            .iter()
            .position(|tab| tab.file_path == location.file_path);

        if let Some(tab_idx) = file_already_open {
            self.active_tab_idx = tab_idx;
        } else {
            // Open new tab
            self.open_file_from_path(&location.file_path);

            // Mark as read-only if it's stdlib
            if is_stdlib {
                if let Some(tab) = self.editor_tabs.last_mut() {
                    tab.is_readonly = true;
                    tracing::info!("📖 Opened as read-only (stdlib)");
                }
            }
        }

        // Set cursor position (using pending jump for next frame)
        if let Some(tab) = self.editor_tabs.get_mut(self.active_tab_idx) {
            tab.cursor_line = location.line;
            tab.cursor_col = location.column;
            tab.pending_cursor_jump = Some((location.line, location.column));
            tracing::info!("⏭️ Scheduled cursor jump to line {} col {}", location.line, location.column);
        }

        self.status_message = format!("✅ Jumped to {}",
            location.file_path.split('/').last().unwrap_or(""));
        self.status_message_timestamp = Some(std::time::Instant::now());
    }

    /// Spawn LSP find_references request asynchronously
    fn spawn_find_references_request(&self, file_path: String, line: usize, column: usize, include_declaration: bool) {
        let client = match &self.lsp_client {
            Some(c) => std::sync::Arc::clone(c),
            None => {
                tracing::warn!("⚠️ LSP client not initialized");
                return;
            }
        };

        let tx = match &self.lsp_response_tx {
            Some(t) => t.clone(),
            None => return,
        };

        let runtime = std::sync::Arc::clone(&self.lsp_runtime);

        // Spawn async task to request find_references from LSP server
        runtime.spawn(async move {
            tracing::info!("🔍 Requesting LSP find_references");
            tracing::info!("   File: {}", file_path);
            tracing::info!("   Position: line={}, column={}, include_decl={}", line, column, include_declaration);

            match client.find_references("rust", file_path.clone(), line as u32, column as u32, include_declaration).await {
                Ok(locations) => {
                    tracing::info!("📍 LSP returned {} references", locations.len());
                    for (i, loc) in locations.iter().enumerate() {
                        tracing::info!("   Reference {}: {}", i + 1, loc.uri);
                    }

                    // Convert proto Location → LspLocation
                    let lsp_locations: Vec<LspLocation> = locations
                        .into_iter()
                        .filter_map(parse_lsp_location)
                        .collect();

                    if let Err(e) = tx.send(LspResponse::References(lsp_locations)) {
                        tracing::error!("❌ Failed to send LSP references: {}", e);
                    }
                }
                Err(e) => {
                    tracing::warn!("⚠️ LSP find_references failed: {}", e);
                    // Send empty response
                    let _ = tx.send(LspResponse::References(vec![]));
                }
            }
        });
    }

    /// Refresh Git status (branch and changed files)
    fn refresh_git_status(&mut self) {
        tracing::info!("🔀 Refreshing Git status for {}", self.root_path);

        // Get current branch
        match native::git::get_current_branch(&self.root_path) {
            Ok(branch) => {
                self.git_current_branch = branch;
                tracing::info!("✅ Current branch: {}", self.git_current_branch);
            }
            Err(e) => {
                tracing::error!("❌ Failed to get current branch: {}", e);
                self.git_current_branch = "(error)".to_string();
            }
        }

        // Get file status
        match native::git::get_status(&self.root_path) {
            Ok(status) => {
                self.git_status = status;
                tracing::info!("✅ Git status loaded: {} files changed", self.git_status.len());
            }
            Err(e) => {
                tracing::error!("❌ Failed to get Git status: {}", e);
                self.git_status.clear();
            }
        }
    }

    /// Refresh Git history (load commit graph)
    fn refresh_git_history(&mut self) {
        tracing::info!("🔀 Refreshing Git history for {}", self.root_path);

        match native::git::get_detailed_log(
            &self.root_path,
            self.git_history_state.page_limit,
            self.git_history_state.show_all_branches,
        ) {
            Ok(nodes) => {
                self.git_history_state.graph_nodes = nodes;
                self.git_history_state.loaded_count = self.git_history_state.graph_nodes.len();
                tracing::info!("✅ Git history loaded: {} commits", self.git_history_state.loaded_count);
            }
            Err(e) => {
                tracing::error!("❌ Failed to load Git history: {}", e);
                self.git_history_state.graph_nodes.clear();
            }
        }
    }

    /// Refresh Git branches
    fn refresh_git_branches(&mut self) {
        match native::git::list_branches(&self.root_path) {
            Ok(branches) => {
                self.git_branch_state.local_branches = branches;
            }
            Err(e) => {
                tracing::error!("❌ Failed to load branches: {}", e);
            }
        }

        match native::git::list_remote_branches(&self.root_path) {
            Ok(branches) => {
                self.git_branch_state.remote_branches = branches;
            }
            Err(e) => {
                tracing::error!("❌ Failed to load remote branches: {}", e);
            }
        }
    }

    /// Refresh Git remotes
    fn refresh_git_remotes(&mut self) {
        match native::git::list_remotes(&self.root_path) {
            Ok(remotes) => {
                self.git_remote_state.remotes = remotes;
            }
            Err(e) => {
                tracing::error!("❌ Failed to load remotes: {}", e);
            }
        }
    }

    /// Refresh Git tags
    fn refresh_git_tags(&mut self) {
        match native::git::list_tags(&self.root_path) {
            Ok(tags) => {
                self.git_tag_state.tags = tags;
            }
            Err(e) => {
                tracing::error!("❌ Failed to load tags: {}", e);
            }
        }
    }

    /// Refresh Git stashes
    fn refresh_git_stashes(&mut self) {
        match native::git::list_stashes(&self.root_path) {
            Ok(stashes) => {
                self.git_stash_state.stashes = stashes;
            }
            Err(e) => {
                tracing::error!("❌ Failed to load stashes: {}", e);
            }
        }
    }

    // ===== Branch Actions =====

    fn perform_create_branch(&mut self) {
        let branch_name = self.git_branch_state.new_branch_name.clone();
        match native::git::create_branch(&self.root_path, &branch_name) {
            Ok(_) => {
                tracing::info!("✅ Created branch: {}", branch_name);
                self.git_branch_state.new_branch_name.clear();
                self.refresh_git_branches();
            }
            Err(e) => {
                tracing::error!("❌ Failed to create branch: {}", e);
            }
        }
    }

    fn perform_checkout_branch(&mut self, branch_name: &str) {
        match native::git::checkout_branch(&self.root_path, branch_name) {
            Ok(_) => {
                tracing::info!("✅ Checked out branch: {}", branch_name);
                self.refresh_git_branches();
                self.refresh_git_status();
            }
            Err(e) => {
                tracing::error!("❌ Failed to checkout branch: {}", e);
            }
        }
    }

    fn perform_delete_branch(&mut self, branch_name: &str) {
        match native::git::delete_branch(&self.root_path, branch_name) {
            Ok(_) => {
                tracing::info!("✅ Deleted branch: {}", branch_name);
                self.refresh_git_branches();
            }
            Err(e) => {
                tracing::error!("❌ Failed to delete branch: {}", e);
            }
        }
    }

    fn perform_merge_branch(&mut self, branch_name: &str) {
        match native::git::merge_branch(&self.root_path, branch_name) {
            Ok(_) => {
                tracing::info!("✅ Merged branch: {}", branch_name);
                self.refresh_git_status();
                self.refresh_git_history();
            }
            Err(e) => {
                tracing::error!("❌ Failed to merge branch: {}", e);
            }
        }
    }

    // ===== Remote Actions =====

    fn perform_add_remote(&mut self) {
        let name = self.git_remote_state.new_remote_name.clone();
        let url = self.git_remote_state.new_remote_url.clone();

        match native::git::add_remote(&self.root_path, &name, &url) {
            Ok(_) => {
                tracing::info!("✅ Added remote: {} -> {}", name, url);
                self.git_remote_state.new_remote_name.clear();
                self.git_remote_state.new_remote_url.clear();
                self.refresh_git_remotes();
            }
            Err(e) => {
                tracing::error!("❌ Failed to add remote: {}", e);
            }
        }
    }

    fn perform_remove_remote(&mut self, name: &str) {
        match native::git::remove_remote(&self.root_path, name) {
            Ok(_) => {
                tracing::info!("✅ Removed remote: {}", name);
                self.refresh_git_remotes();
            }
            Err(e) => {
                tracing::error!("❌ Failed to remove remote: {}", e);
            }
        }
    }

    fn perform_fetch(&mut self, remote_name: &str) {
        match native::git::fetch(&self.root_path, remote_name) {
            Ok(_) => {
                tracing::info!("✅ Fetched from remote: {}", remote_name);
                self.refresh_git_branches();
            }
            Err(e) => {
                tracing::error!("❌ Failed to fetch: {}", e);
            }
        }
    }

    fn perform_pull(&mut self, remote_name: &str) {
        // Use current branch name
        let branch_name = self.git_current_branch.clone();
        match native::git::pull(&self.root_path, remote_name, &branch_name) {
            Ok(_) => {
                tracing::info!("✅ Pulled from remote: {}", remote_name);
                self.refresh_git_status();
                self.refresh_git_history();
            }
            Err(e) => {
                tracing::error!("❌ Failed to pull: {}", e);
            }
        }
    }

    fn perform_push(&mut self, remote_name: &str) {
        // Push current branch
        let branch_name = self.git_current_branch.clone();
        let refspec = format!("refs/heads/{}:refs/heads/{}", branch_name, branch_name);
        match native::git::push(&self.root_path, remote_name, &[&refspec]) {
            Ok(_) => {
                tracing::info!("✅ Pushed to remote: {}", remote_name);
            }
            Err(e) => {
                tracing::error!("❌ Failed to push: {}", e);
            }
        }
    }

    // ===== Tag Actions =====

    fn perform_create_tag(&mut self) {
        let tag_name = self.git_tag_state.new_tag_name.clone();
        let result = if self.git_tag_state.annotated {
            let message = self.git_tag_state.new_tag_message.clone();
            native::git::create_annotated_tag(&self.root_path, &tag_name, &message, None)
        } else {
            native::git::create_tag(&self.root_path, &tag_name, None)
        };

        match result {
            Ok(_) => {
                tracing::info!("✅ Created tag: {}", tag_name);
                self.git_tag_state.new_tag_name.clear();
                self.git_tag_state.new_tag_message.clear();
                self.refresh_git_tags();
            }
            Err(e) => {
                tracing::error!("❌ Failed to create tag: {}", e);
            }
        }
    }

    fn perform_delete_tag(&mut self, tag_name: &str) {
        match native::git::delete_tag(&self.root_path, tag_name) {
            Ok(_) => {
                tracing::info!("✅ Deleted tag: {}", tag_name);
                self.refresh_git_tags();
            }
            Err(e) => {
                tracing::error!("❌ Failed to delete tag: {}", e);
            }
        }
    }

    // ===== Stash Actions =====

    fn perform_stash_save(&mut self) {
        let message = if self.git_stash_state.new_stash_message.is_empty() {
            None
        } else {
            Some(self.git_stash_state.new_stash_message.as_str())
        };

        match native::git::stash_save(&self.root_path, message, self.git_stash_state.include_untracked) {
            Ok(_) => {
                tracing::info!("✅ Saved stash");
                self.git_stash_state.new_stash_message.clear();
                self.refresh_git_stashes();
                self.refresh_git_status();
            }
            Err(e) => {
                tracing::error!("❌ Failed to save stash: {}", e);
            }
        }
    }

    fn perform_stash_apply(&mut self, index: usize) {
        match native::git::stash_apply(&self.root_path, index) {
            Ok(_) => {
                tracing::info!("✅ Applied stash@{}", index);
                self.refresh_git_status();
            }
            Err(e) => {
                tracing::error!("❌ Failed to apply stash: {}", e);
            }
        }
    }

    fn perform_stash_pop(&mut self, index: usize) {
        match native::git::stash_pop(&self.root_path, index) {
            Ok(_) => {
                tracing::info!("✅ Popped stash@{}", index);
                self.refresh_git_stashes();
                self.refresh_git_status();
            }
            Err(e) => {
                tracing::error!("❌ Failed to pop stash: {}", e);
            }
        }
    }

    fn perform_stash_drop(&mut self, index: usize) {
        match native::git::stash_drop(&self.root_path, index) {
            Ok(_) => {
                tracing::info!("✅ Dropped stash@{}", index);
                self.refresh_git_stashes();
            }
            Err(e) => {
                tracing::error!("❌ Failed to drop stash: {}", e);
            }
        }
    }

    /// Stage a file
    fn perform_git_stage(&mut self, file_path: &str) {
        tracing::info!("🔀 Staging file: {}", file_path);

        match native::git::stage_file(&self.root_path, file_path) {
            Ok(_) => {
                tracing::info!("✅ File staged: {}", file_path);
                self.refresh_git_status(); // Refresh to update UI
            }
            Err(e) => {
                tracing::error!("❌ Failed to stage file: {}", e);
                self.terminal_output.push(TerminalLine {
                    text: format!("Git stage error: {}", e),
                    style: TerminalStyle::Error,
                });
            }
        }
    }

    /// Unstage a file
    fn perform_git_unstage(&mut self, file_path: &str) {
        tracing::info!("🔀 Unstaging file: {}", file_path);

        match native::git::unstage_file(&self.root_path, file_path) {
            Ok(_) => {
                tracing::info!("✅ File unstaged: {}", file_path);
                self.refresh_git_status(); // Refresh to update UI
            }
            Err(e) => {
                tracing::error!("❌ Failed to unstage file: {}", e);
                self.terminal_output.push(TerminalLine {
                    text: format!("Git unstage error: {}", e),
                    style: TerminalStyle::Error,
                });
            }
        }
    }

    /// Stage all files
    fn perform_git_stage_all(&mut self) {
        tracing::info!("🔀 Staging all files");

        match native::git::stage_all(&self.root_path) {
            Ok(_) => {
                tracing::info!("✅ All files staged");
                self.refresh_git_status(); // Refresh to update UI
            }
            Err(e) => {
                tracing::error!("❌ Failed to stage all: {}", e);
                self.terminal_output.push(TerminalLine {
                    text: format!("Git stage all error: {}", e),
                    style: TerminalStyle::Error,
                });
            }
        }
    }

    /// Create a commit
    fn perform_git_commit(&mut self) {
        if self.git_commit_message.trim().is_empty() {
            tracing::warn!("⚠️  Cannot commit with empty message");
            self.terminal_output.push(TerminalLine {
                text: "Error: Commit message cannot be empty".to_string(),
                style: TerminalStyle::Error,
            });
            return;
        }

        tracing::info!("🔀 Creating commit: {}", self.git_commit_message);

        match native::git::commit(&self.root_path, &self.git_commit_message) {
            Ok(commit_id) => {
                tracing::info!("✅ Commit created: {}", commit_id);
                self.terminal_output.push(TerminalLine {
                    text: format!("✅ Commit created: {}", commit_id),
                    style: TerminalStyle::Output,
                });
                self.git_commit_message.clear(); // Clear input
                self.refresh_git_status(); // Refresh to update UI
            }
            Err(e) => {
                tracing::error!("❌ Failed to commit: {}", e);
                self.terminal_output.push(TerminalLine {
                    text: format!("Git commit error: {}", e),
                    style: TerminalStyle::Error,
                });
            }
        }
    }

    /// Send chat message (legacy - moved to Slack system)
    #[allow(dead_code)]
    fn send_chat_message(&mut self) {
        // Moved to send_message_to_channel in Slack system
    }


    /// Handle LSP keyboard shortcuts
    fn handle_lsp_shortcuts(&mut self, ctx: &egui::Context) {
        // Only handle shortcuts when editor is focused
        if self.active_focus != FocusLayer::Editor || self.editor_tabs.is_empty() {
            return;
        }

        ctx.input(|i| {
            // Ctrl+Space: Trigger completions
            if i.modifiers.command && i.key_pressed(egui::Key::Space) {
                self.trigger_lsp_completions();
            }

            // Escape: Close completions
            if i.key_pressed(egui::Key::Escape) && self.lsp_show_completions {
                self.lsp_show_completions = false;
                self.lsp_completions.clear();
            }
        });
    }

    /// Trigger LSP completions (Phase 5.4 MVP: placeholder data)
    fn trigger_lsp_completions(&mut self) {
        tracing::info!("💡 Triggering LSP completions");

        let tab = match self.editor_tabs.get(self.active_tab_idx) {
            Some(t) => t,
            None => return,
        };

        let file_path = tab.file_path.clone();

        // Get current cursor position
        let line = tab.cursor_line;
        let column = tab.cursor_col;

        let client = match &self.lsp_client {
            Some(c) => std::sync::Arc::clone(c),
            None => {
                tracing::warn!("⚠️ LSP client not initialized");
                return;
            }
        };

        let tx = match &self.lsp_response_tx {
            Some(t) => t.clone(),
            None => return,
        };

        let runtime = std::sync::Arc::clone(&self.lsp_runtime);

        // Spawn async task to request completions from LSP server
        runtime.spawn(async move {
            tracing::info!("🚀 Requesting LSP completions at {}:{}", line, column);

            match client.get_completions("rust", file_path.clone(), line as u32, column as u32).await {
                Ok(items) => {
                    tracing::info!("📋 LSP returned {} completion items", items.len());

                    // Convert proto CompletionItem → LspCompletionItem
                    let lsp_completions: Vec<LspCompletionItem> = items
                        .into_iter()
                        .map(|item| LspCompletionItem {
                            label: item.label,
                            detail: item.detail,
                            kind: match item.kind {
                                Some(1) => "text",
                                Some(2) => "method",
                                Some(3) => "function",
                                Some(4) => "constructor",
                                Some(5) => "field",
                                Some(6) => "variable",
                                Some(7) => "class",
                                Some(8) => "interface",
                                Some(9) => "module",
                                Some(10) => "property",
                                Some(11) => "unit",
                                Some(12) => "value",
                                Some(13) => "enum",
                                Some(14) => "keyword",
                                Some(15) => "snippet",
                                Some(16) => "color",
                                Some(17) => "file",
                                Some(18) => "reference",
                                Some(19) => "folder",
                                Some(20) => "enum_member",
                                Some(21) => "constant",
                                Some(22) => "struct",
                                Some(23) => "event",
                                Some(24) => "operator",
                                Some(25) => "type_parameter",
                                _ => "unknown",
                            }.to_string(),
                        })
                        .collect();

                    if let Err(e) = tx.send(LspResponse::Completions(lsp_completions)) {
                        tracing::error!("❌ Failed to send LSP completions: {}", e);
                    }
                }
                Err(e) => {
                    tracing::warn!("⚠️ LSP get_completions failed: {}", e);
                }
            }
        });

        // Show completions window immediately (will be populated when response arrives)
        self.lsp_show_completions = true;
    }

    /// Render LSP completion popup
    fn render_lsp_completions(&mut self, ctx: &egui::Context) {
        let mut close_completions = false;

        egui::Window::new("💡 Code Completions")
            .collapsible(false)
            .resizable(false)
            .default_pos([400.0, 200.0])
            .show(ctx, |ui| {
                ui.label("Ctrl+Space triggered completions (Phase 5.4 MVP)");
                ui.separator();

                // Clone items to avoid borrowing issues
                let items = self.lsp_completions.clone();

                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        for item in &items {
                            let display_text = if let Some(ref detail) = item.detail {
                                format!("{} → {}", item.label, detail)
                            } else {
                                item.label.clone()
                            };

                            if ui.selectable_label(false, display_text).clicked() {
                                tracing::info!("💡 Selected completion: {}", item.label);
                                // TODO: Insert completion into editor
                                close_completions = true;
                            }
                        }
                    });

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Close (Esc)").clicked() {
                        close_completions = true;
                    }

                    ui.label("ℹ️ Full LSP integration coming soon");
                });
            });

        // Close completions if requested
        if close_completions {
            self.lsp_show_completions = false;
            self.lsp_completions.clear();
        }
    }

    // ===== Phase 6: Advanced Features =====

    /// Poll LSP responses from background tasks (non-blocking)
    /// Poll gRPC responses from async tasks
    /// Send a message to the AI via gRPC
    fn send_grpc_message(&mut self) {
        let message = self.grpc_input.trim().to_string();
        if message.is_empty() {
            return;
        }

        // Add user message to chat history
        self.grpc_messages.push(GrpcMessage {
            content: message.clone(),
            is_user: true,
        });

        // Clear input
        self.grpc_input.clear();

        // Set streaming state
        self.grpc_streaming = true;
        self.grpc_current_response.clear();
        self.grpc_streaming_message = Some(String::new());

        // Get session ID
        let session_id = match &self.grpc_session_id {
            Some(id) => id.clone(),
            None => {
                tracing::error!("❌ No active gRPC session - berry-api-server may not be running");
                self.grpc_streaming = false;

                // Add error message to chat
                self.grpc_messages.push(GrpcMessage {
                    content: "⚠️ AI Chat is not available. Please start berry-api-server:\ncd berry_api && cargo run --bin berry-api-server".to_string(),
                    is_user: false,
                });

                self.status_message = "❌ AI Chat unavailable - start berry-api-server".to_string();
                self.status_message_timestamp = Some(std::time::Instant::now());
                return;
            }
        };

        // Send message to berry-api-server via gRPC
        let grpc_client = self.grpc_client.clone();
        let tx = self.grpc_response_tx.clone();
        let autonomous = self.ai_chat_mode == AIChatMode::Autonomous;

        tracing::info!("📤 Sending AI message (autonomous={}): {}", autonomous, message);

        // Spawn async task to handle streaming response
        self.lsp_runtime.spawn(async move {
            match grpc_client.chat_stream(session_id, message, autonomous).await {
                Ok(mut rx) => {
                    // Stream chunks back to UI
                    while let Some(chunk) = rx.recv().await {
                        if let Some(tx) = &tx {
                            let _ = tx.send(GrpcResponse::ChatChunk(chunk));
                        }
                    }
                    // Signal completion
                    if let Some(tx) = &tx {
                        let _ = tx.send(GrpcResponse::ChatStreamCompleted);
                    }
                }
                Err(e) => {
                    tracing::error!("❌ Failed to send chat message: {}", e);
                }
            }
        });
    }

    fn poll_grpc_responses(&mut self) {
        if let Some(rx) = &mut self.grpc_response_rx {
            // Try to receive all available messages without blocking
            while let Ok(response) = rx.try_recv() {
                match response {
                    GrpcResponse::SessionStarted(session_id) => {
                        tracing::info!("🎯 gRPC session ready: {}", session_id);
                        self.grpc_session_id = Some(session_id);
                        self.grpc_connected = true;
                        self.status_message = "✅ AI Chat ready".to_string();
                        self.status_message_timestamp = Some(std::time::Instant::now());
                    }
                    GrpcResponse::ChatChunk(chunk) => {
                        tracing::info!("🎨 UI received chunk: {} chars", chunk.len());

                        // Append chunk to current AI response
                        self.grpc_current_response.push_str(&chunk);

                        // Also keep legacy streaming message for compatibility
                        if let Some(streaming_msg) = &mut self.grpc_streaming_message {
                            streaming_msg.push_str(&chunk);
                            tracing::info!("📝 Accumulated message: {} chars total", streaming_msg.len());
                        } else {
                            // Initialize streaming message if not present
                            self.grpc_streaming_message = Some(String::new());
                            if let Some(streaming_msg) = &mut self.grpc_streaming_message {
                                streaming_msg.push_str(&chunk);
                            }
                        }
                    }
                    GrpcResponse::ChatStreamCompleted => {
                        tracing::info!("✅ Chat stream completed");

                        // Add completed AI message to history
                        if !self.grpc_current_response.is_empty() {
                            self.grpc_messages.push(GrpcMessage {
                                content: self.grpc_current_response.clone(),
                                is_user: false,
                            });
                            self.grpc_current_response.clear();
                        }

                        // Reset streaming state
                        self.grpc_streaming = false;
                        self.grpc_streaming_message = None;
                    }
                }
            }
        }
    }

    /// Poll Slack responses (non-blocking)
    fn poll_slack_responses(&mut self) {
        let mut should_reload_messages = false;
        let mut reload_channel_id: Option<String> = None;

        if let Some(rx) = &mut self.slack_response_rx {
            while let Ok(response) = rx.try_recv() {
                match response {
                    SlackResponse::Authenticated => {
                        tracing::info!("✅ Slack authenticated");
                        self.slack_authenticated = true;
                        self.status_message = "✅ Slack connected".to_string();
                        self.status_message_timestamp = Some(std::time::Instant::now());
                        self.show_slack_settings = false;
                    }
                    SlackResponse::ChannelsList(channels) => {
                        tracing::info!("📋 Loaded {} Slack channels", channels.len());
                        self.slack_channels = channels.clone();

                        // Convert Slack channels to Chat UI channels
                        self.chat_channels = channels.into_iter().map(|ch| {
                            ChatChannel::new(
                                ch.id.clone(),
                                ch.name,
                                ChannelType::Public,  // Slack API doesn't distinguish in this simple impl
                            )
                        }).collect();

                        // Select first channel if none selected
                        let first_channel_id = if self.selected_channel_id.is_none() && !self.chat_channels.is_empty() {
                            let channel_id = self.chat_channels[0].id.clone();
                            self.selected_channel_id = Some(channel_id.clone());
                            Some(channel_id)
                        } else {
                            None
                        };

                        // Load messages after releasing borrow
                        if let Some(channel_id) = first_channel_id {
                            should_reload_messages = true;
                            reload_channel_id = Some(channel_id);
                        }
                    }
                    SlackResponse::MessagesList(messages) => {
                        tracing::info!("💬 Loaded {} Slack messages", messages.len());
                        self.slack_messages = messages.clone();

                        // Convert Slack messages to Chat UI messages
                        if let Some(channel_id) = &self.selected_channel_id {
                            if let Some(channel) = self.chat_channels.iter_mut().find(|c| &c.id == channel_id) {
                                channel.messages = messages.into_iter().map(|msg| {
                                    // Parse timestamp or use current time as fallback
                                    let timestamp = msg.timestamp.parse::<f64>()
                                        .ok()
                                        .and_then(|ts| chrono::DateTime::from_timestamp(ts as i64, 0))
                                        .unwrap_or_else(|| chrono::Utc::now());

                                    ChatMessage {
                                        id: msg.timestamp.clone(),
                                        channel_id: channel_id.clone(),
                                        user_id: msg.user.clone(),
                                        user_name: msg.user.clone(),  // Use user ID as name for now
                                        content: msg.text,
                                        timestamp,
                                        edited: false,  // TODO: Parse from Slack metadata
                                        thread_replies: Vec::new(),  // TODO: Load thread replies
                                        reactions: Vec::new(),  // TODO: Load reactions
                                        mentioned_users: Vec::new(),  // TODO: Parse from message text
                                        is_pinned: false,  // TODO: Get from Slack metadata
                                    }
                                }).collect();
                            }
                        }
                    }
                    SlackResponse::MessageSent => {
                        tracing::info!("✅ Slack message sent");
                        self.chat_input.clear();
                        // Schedule message reload
                        should_reload_messages = true;
                        reload_channel_id = self.selected_channel_id.clone();
                    }
                    SlackResponse::Error(err) => {
                        tracing::error!("❌ Slack error: {}", err);
                        self.status_message = format!("❌ Slack error: {}", err);
                        self.status_message_timestamp = Some(std::time::Instant::now());
                    }
                }
            }
        }

        // Execute deferred actions
        if should_reload_messages {
            if let Some(channel_id) = reload_channel_id {
                self.load_slack_messages(&channel_id);
            }
        }
    }

    /// Poll file watcher events (non-blocking) and update file tree
    fn poll_file_watcher_events(&mut self) {
        if let Some(watcher) = &mut self.file_watcher {
            // Try to receive all available file system events without blocking
            while let Some(event) = watcher.try_recv() {
                match event {
                    native::watcher::FileEvent::Created(path) => {
                        tracing::debug!("📄 File created: {}", path.display());
                        // Mark file tree for reload
                        self.file_tree_load_pending = true;
                    }
                    native::watcher::FileEvent::Modified(path) => {
                        tracing::debug!("✏️  File modified: {}", path.display());
                        // If the file is currently open, we could optionally reload it
                        // For now, just log it
                    }
                    native::watcher::FileEvent::Removed(path) => {
                        tracing::debug!("🗑️  File removed: {}", path.display());
                        // Mark file tree for reload
                        self.file_tree_load_pending = true;

                        // Close the tab if the deleted file is currently open
                        let path_str = path.to_string_lossy().to_string();
                        if let Some(tab_idx) = self.editor_tabs.iter().position(|tab| tab.file_path == path_str) {
                            self.editor_tabs.remove(tab_idx);
                            // Adjust active tab index if necessary
                            if self.active_tab_idx >= self.editor_tabs.len() && !self.editor_tabs.is_empty() {
                                self.active_tab_idx = self.editor_tabs.len() - 1;
                            }
                            tracing::info!("🗑️  Closed tab for deleted file: {}", path_str);
                        }
                    }
                    native::watcher::FileEvent::Renamed { from, to } => {
                        tracing::debug!("📝 File renamed: {} -> {}", from.display(), to.display());
                        // Mark file tree for reload
                        self.file_tree_load_pending = true;

                        // Update the tab path if the renamed file is currently open
                        let from_str = from.to_string_lossy().to_string();
                        let to_str = to.to_string_lossy().to_string();
                        if let Some(tab) = self.editor_tabs.iter_mut().find(|tab| tab.file_path == from_str) {
                            tab.file_path = to_str.clone();
                            tracing::info!("📝 Updated tab path: {} -> {}", from_str, to_str);
                        }
                    }
                }
            }
        }
    }

    fn poll_lsp_responses(&mut self) {
        // Deferred actions to perform after releasing rx borrow
        enum DeferredAction {
            NavigateToLocation(LspLocation),
            ShowPicker(Vec<LspLocation>),
        }

        let mut deferred_actions: Vec<DeferredAction> = Vec::new();

        if let Some(rx) = &mut self.lsp_response_rx {
            // Try to receive all available messages without blocking
            while let Ok(response) = rx.try_recv() {
                match response {
                    LspResponse::Connected => {
                        tracing::info!("🟢 LSP connection established");
                        self.lsp_connected = true;
                        self.status_message = "✅ LSP connected".to_string();
                        self.status_message_timestamp = Some(std::time::Instant::now());
                    }
                    LspResponse::Diagnostics(diagnostics) => {
                        tracing::info!("📋 Received {} diagnostics", diagnostics.len());
                        self.lsp_diagnostics = diagnostics;
                    }
                    LspResponse::Hover(hover_info) => {
                        tracing::info!("💡 Received hover info");
                        let has_hover = hover_info.is_some();
                        self.lsp_hover_info = hover_info;
                        self.lsp_show_hover = has_hover;
                    }
                    LspResponse::Completions(completions) => {
                        tracing::info!("💡 Received {} completions", completions.len());
                        self.lsp_completions = completions;
                        self.lsp_show_completions = !self.lsp_completions.is_empty();
                    }
                    LspResponse::Definition(locations) => {
                        tracing::info!("🔍 Received {} definition locations", locations.len());

                        if locations.is_empty() {
                            // LSP returned no results → disabled fallback for performance
                            self.pending_goto_definition.take(); // Clear pending
                            self.status_message = "❌ Definition not found (LSP)".to_string();
                            self.status_message_timestamp = Some(std::time::Instant::now());

                        } else if locations.len() == 1 {
                            // Single definition → navigate directly
                            deferred_actions.push(DeferredAction::NavigateToLocation(locations[0].clone()));
                            self.pending_goto_definition = None;
                        } else {
                            // Multiple definitions → show picker UI
                            tracing::info!("📋 Multiple definitions found, showing picker");
                            deferred_actions.push(DeferredAction::ShowPicker(locations));
                            self.pending_goto_definition = None;
                        }
                    }
                    LspResponse::References(locations) => {
                        tracing::info!("🔍 Received {} references", locations.len());

                        if locations.is_empty() {
                            self.status_message = "No references found".to_string();
                            self.status_message_timestamp = Some(std::time::Instant::now());
                        } else {
                            // Show references panel
                            self.lsp_references = locations;
                            self.show_references_panel = true;
                            self.status_message = format!("Found {} references", self.lsp_references.len());
                            self.status_message_timestamp = Some(std::time::Instant::now());
                        }
                    }
                }
            }
        }

        // Process deferred actions after releasing the borrow
        for action in deferred_actions {
            match action {
                DeferredAction::NavigateToLocation(location) => {
                    self.navigate_to_location(&location);
                }
                DeferredAction::ShowPicker(locations) => {
                    self.definition_picker_locations = locations;
                    self.show_definition_picker = true;
                }
            }
        }
    }

    /// Render LSP hover tooltip
    fn render_lsp_hover(&mut self, ctx: &egui::Context) {
        // Clone to avoid borrowing issues with the closure
        if let Some(hover_info) = self.lsp_hover_info.clone() {
            let mut close_hover = false;

            egui::Window::new("💡 Hover Information")
                .collapsible(false)
                .resizable(false)
                .default_pos([400.0, 300.0])
                .show(ctx, |ui| {
                    ui.label(&hover_info.contents);

                    ui.separator();

                    if ui.button("Close (Esc)").clicked() {
                        close_hover = true;
                    }
                });

            if close_hover {
                self.lsp_show_hover = false;
                self.lsp_hover_info = None;
            }
        }
    }

    /// Render definition picker window (for multiple definitions)
    fn render_definition_picker(&mut self, ctx: &egui::Context) {
        // Clone locations to avoid borrowing issues
        let locations = self.definition_picker_locations.clone();
        let mut selected_location: Option<LspLocation> = None;
        let mut close_picker = false;

        egui::Window::new("📋 Choose Definition")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .default_size([600.0, 400.0])
            .show(ctx, |ui| {
                ui.label(format!("{} definitions found:", locations.len()));
                ui.separator();

                egui::ScrollArea::vertical()
                    .max_height(400.0)
                    .show(ui, |ui| {
                        for (idx, loc) in locations.iter().enumerate() {
                            let file_name = loc.file_path.split('/').last().unwrap_or(&loc.file_path);
                            let label = format!("{}  {}:{}  ({})",
                                idx + 1, file_name, loc.line + 1, loc.file_path);

                            if ui.button(&label).clicked() {
                                selected_location = Some(loc.clone());
                                close_picker = true;
                            }
                        }
                    });

                ui.separator();
                if ui.button("❌ Cancel").clicked() {
                    close_picker = true;
                }
            });

        // Handle selection/cancellation after window is closed
        if let Some(location) = selected_location {
            self.navigate_to_location(&location);
            self.show_definition_picker = false;
            self.definition_picker_locations.clear();
        } else if close_picker {
            self.show_definition_picker = false;
            self.definition_picker_locations.clear();
        }
    }

    /// Render References panel
    fn render_references_panel(&mut self, ctx: &egui::Context) {
        // Clone references to avoid borrowing issues
        let references = self.lsp_references.clone();
        let mut selected_location: Option<LspLocation> = None;
        let mut close_panel = false;

        egui::Window::new("🔍 References")
            .collapsible(false)
            .resizable(true)
            .anchor(egui::Align2::RIGHT_TOP, egui::Vec2::new(-10.0, 50.0))
            .default_size([600.0, 400.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(format!("{} references found", references.len()));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("❌").clicked() {
                            close_panel = true;
                        }
                    });
                });
                ui.separator();

                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        for (_idx, loc) in references.iter().enumerate() {
                            let file_name = loc.file_path.split('/').last().unwrap_or(&loc.file_path);

                            // File:line:column display as clickable link
                            let location_text = format!("{}:{}:{}", file_name, loc.line + 1, loc.column + 1);
                            if ui.link(&location_text).clicked() {
                                selected_location = Some(loc.clone());
                            }
                        }
                    });
            });

        // Handle selection/cancellation after window is closed
        if let Some(location) = selected_location {
            self.navigate_to_location(&location);
        } else if close_panel {
            self.show_references_panel = false;
            self.lsp_references.clear();
        }
    }

    /// Render Settings dialog
    fn render_settings_dialog(&mut self, ctx: &egui::Context) {
        egui::Window::new("⚙️ Settings")
            .collapsible(false)
            .resizable(true)
            .default_size([500.0, 400.0])
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.heading("Editor Settings");
                    ui.separator();

                    ui.label("Font size:");
                    ui.label("(Font size control coming soon)");

                    ui.add_space(8.0);

                    ui.label("Tab size:");
                    ui.label("(Tab size control coming soon)");

                    ui.add_space(8.0);

                    ui.label("Theme:");
                    if ui.button("Open Theme Editor").clicked() {
                        self.show_theme_editor = true;
                    }
                });

                ui.separator();

                if ui.button("Close").clicked() {
                    self.show_settings = false;
                }
            });
    }

    /// Render Theme editor
    fn render_theme_editor(&mut self, ctx: &egui::Context) {
        egui::Window::new("🎨 Theme Editor")
            .collapsible(false)
            .resizable(true)
            .default_size([600.0, 500.0])
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.heading("Color Scheme");
                    ui.separator();

                    let mut visuals = ctx.style().visuals.clone();

                    ui.label("Widget colors:");
                    egui::Grid::new("theme_grid")
                        .num_columns(2)
                        .spacing([40.0, 4.0])
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label("Background:");
                            ui.color_edit_button_srgba(&mut visuals.panel_fill);
                            ui.end_row();

                            ui.label("Text:");
                            let mut text_color = visuals.text_color();
                            ui.color_edit_button_srgba(&mut text_color);
                            // DO NOT override text color - it breaks syntax highlighting!
                            // visuals.override_text_color = Some(text_color);
                            ui.end_row();

                            ui.label("Selection:");
                            ui.color_edit_button_srgba(&mut visuals.selection.bg_fill);
                            ui.end_row();

                            ui.label("Window fill:");
                            ui.color_edit_button_srgba(&mut visuals.window_fill);
                            ui.end_row();
                        });

                    ui.add_space(8.0);

                    if ui.button("Apply Theme").clicked() {
                        ctx.set_visuals(visuals);
                        tracing::info!("🎨 Theme updated");
                    }
                });

                ui.separator();

                if ui.button("Close").clicked() {
                    self.show_theme_editor = false;
                }
            });
    }

    // ===== Phase 6.2: LSP Diagnostics =====

    /// Request diagnostics for the current file (spawns background task)
    fn request_diagnostics(&mut self) {
        let tab = match self.editor_tabs.get(self.active_tab_idx) {
            Some(t) => t,
            None => return,
        };

        let file_path = tab.file_path.clone();

        let client = match &self.lsp_client {
            Some(c) => std::sync::Arc::clone(c),
            None => {
                tracing::warn!("⚠️ LSP client not initialized");
                return;
            }
        };

        let tx = match &self.lsp_response_tx {
            Some(t) => t.clone(),
            None => return,
        };

        let runtime = std::sync::Arc::clone(&self.lsp_runtime);

        // Spawn async task to request diagnostics from LSP server
        runtime.spawn(async move {
            tracing::info!("🚀 Requesting LSP diagnostics for {}", file_path);

            match client.get_diagnostics("rust", file_path.clone()).await {
                Ok(diagnostics) => {
                    tracing::info!("📋 LSP returned {} diagnostics", diagnostics.len());

                    // Convert proto Diagnostic → LspDiagnostic
                    let lsp_diagnostics: Vec<LspDiagnostic> = diagnostics
                        .into_iter()
                        .filter_map(|diag| {
                            let range = diag.range?;
                            let start = range.start?;

                            Some(LspDiagnostic {
                                line: start.line as usize,
                                column: start.character as usize,
                                severity: match diag.severity {
                                    1 => DiagnosticSeverity::Error,
                                    2 => DiagnosticSeverity::Warning,
                                    3 => DiagnosticSeverity::Information,
                                    4 => DiagnosticSeverity::Hint,
                                    _ => DiagnosticSeverity::Error,
                                },
                                message: diag.message,
                                source: diag.source,
                            })
                        })
                        .collect();

                    if let Err(e) = tx.send(LspResponse::Diagnostics(lsp_diagnostics)) {
                        tracing::error!("❌ Failed to send LSP diagnostics: {}", e);
                    }
                }
                Err(e) => {
                    tracing::warn!("⚠️ LSP get_diagnostics failed: {}", e);
                }
            }
        });
    }

    /// Render diagnostics in the editor (gutter icons and inline messages)
    fn render_diagnostics_in_editor(&self, ui: &mut egui::Ui, line_number: usize) {
        // Find diagnostics for this line
        let diagnostics_on_line: Vec<&LspDiagnostic> = self
            .lsp_diagnostics
            .iter()
            .filter(|d| d.line == line_number)
            .collect();

        if diagnostics_on_line.is_empty() {
            return;
        }

        // Show gutter icon
        for diagnostic in &diagnostics_on_line {
            let (icon, color) = match diagnostic.severity {
                DiagnosticSeverity::Error => ("❌", egui::Color32::from_rgb(255, 80, 80)),
                DiagnosticSeverity::Warning => ("⚠️", egui::Color32::from_rgb(255, 200, 100)),
                DiagnosticSeverity::Information => ("ℹ️", egui::Color32::from_rgb(100, 150, 255)),
                DiagnosticSeverity::Hint => ("💡", egui::Color32::from_rgb(150, 150, 150)),
            };

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(icon).color(color));
                ui.label(egui::RichText::new(&diagnostic.message).color(color));
            });
        }
    }

    /// Render diagnostics panel at the bottom of the editor
    fn render_diagnostics_panel(&mut self, ctx: &egui::Context) {
        if self.lsp_diagnostics.is_empty() {
            return;
        }

        egui::TopBottomPanel::bottom("diagnostics_panel")
            .resizable(true)
            .default_height(150.0)
            .show(ctx, |ui| {
                ui.heading(format!("📋 Problems ({})", self.lsp_diagnostics.len()));
                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    let diagnostics = self.lsp_diagnostics.clone();

                    for diagnostic in diagnostics.iter() {
                        let (icon, color) = match diagnostic.severity {
                            DiagnosticSeverity::Error => ("❌", egui::Color32::from_rgb(255, 80, 80)),
                            DiagnosticSeverity::Warning => ("⚠️", egui::Color32::from_rgb(255, 200, 100)),
                            DiagnosticSeverity::Information => ("ℹ️", egui::Color32::from_rgb(100, 150, 255)),
                            DiagnosticSeverity::Hint => ("💡", egui::Color32::from_rgb(150, 150, 150)),
                        };

                        let file_path = if !self.editor_tabs.is_empty() {
                            self.editor_tabs[self.active_tab_idx].file_path.clone()
                        } else {
                            "unknown".to_string()
                        };

                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(icon).color(color));

                            let location = format!("{}:{}:{}",
                                file_path.split('/').last().unwrap_or(""),
                                diagnostic.line + 1,
                                diagnostic.column + 1
                            );

                            if ui.link(&location).clicked() {
                                // Jump to diagnostic location
                                if let Some(tab) = self.editor_tabs.get_mut(self.active_tab_idx) {
                                    tab.cursor_line = diagnostic.line;
                                    tab.cursor_col = diagnostic.column;
                                }
                            }

                            ui.label(egui::RichText::new(&diagnostic.message).color(color));
                        });

                        ui.separator();
                    }
                });
            });
    }

    // ===== Phase 6.3: LSP Hover =====

    /// Request hover information for a specific position (spawns background task)
    fn request_hover(&mut self, line: usize, column: usize) {
        let tab = match self.editor_tabs.get(self.active_tab_idx) {
            Some(t) => t,
            None => return,
        };

        let file_path = tab.file_path.clone();

        let client = match &self.lsp_client {
            Some(c) => std::sync::Arc::clone(c),
            None => {
                tracing::warn!("⚠️ LSP client not initialized");
                return;
            }
        };

        let tx = match &self.lsp_response_tx {
            Some(t) => t.clone(),
            None => return,
        };

        let runtime = std::sync::Arc::clone(&self.lsp_runtime);

        // Spawn async task to request hover from LSP server
        runtime.spawn(async move {
            tracing::info!("🚀 Requesting LSP hover at {}:{}", line, column);

            match client.get_hover("rust", file_path.clone(), line as u32, column as u32).await {
                Ok(hover_opt) => {
                    if let Some(hover) = hover_opt {
                        tracing::info!("💡 LSP returned hover info");

                        let lsp_hover = LspHoverInfo {
                            contents: hover.contents,
                            line,
                            column,
                        };

                        if let Err(e) = tx.send(LspResponse::Hover(Some(lsp_hover))) {
                            tracing::error!("❌ Failed to send LSP hover: {}", e);
                        }
                    } else {
                        tracing::info!("ℹ️ No hover info available");
                        let _ = tx.send(LspResponse::Hover(None));
                    }
                }
                Err(e) => {
                    tracing::warn!("⚠️ LSP get_hover failed: {}", e);
                }
            }
        });
    }

    /// Check if mouse is hovering over text and trigger hover request
    fn check_hover_in_editor(&mut self, _response: &egui::Response) {
        // Disabled
    }

    // ===== Phase 6.4: Go to Definition =====

    /// Request definition locations for the symbol at the current position
    /// NOTE: Disabled - requires Tokio runtime
    fn request_definition(&mut self) {
        tracing::debug!("LSP go-to-definition disabled (no Tokio runtime)");
    }

    /// Handle keyboard shortcut for Go to Definition (F12)
    fn handle_goto_definition_shortcut(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            if i.key_pressed(egui::Key::F12) && !i.modifiers.shift {
                self.trigger_goto_definition_at_cursor();
            }
        });
    }

    /// Handle keyboard shortcut for Find References (Shift+F12)
    fn handle_find_references_shortcut(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            if i.modifiers.shift && i.key_pressed(egui::Key::F12) {
                self.trigger_find_references_at_cursor();
            }
        });
    }

    /// Trigger find references at current cursor position
    fn trigger_find_references_at_cursor(&mut self) {
        if self.editor_tabs.is_empty() {
            return;
        }

        let tab = match self.editor_tabs.get(self.active_tab_idx) {
            Some(t) => t,
            None => return,
        };

        let file_path = tab.file_path.clone();
        let cursor_line = tab.cursor_line;
        let cursor_col = tab.cursor_col;

        tracing::info!("🔍 Triggering find references at {}:{}:{}",
            file_path.split('/').last().unwrap_or(&file_path),
            cursor_line + 1,
            cursor_col + 1
        );

        // Spawn async LSP request
        self.spawn_find_references_request(file_path, cursor_line, cursor_col, true);
    }

    /// Trigger go-to-definition at current cursor position
    fn trigger_goto_definition_at_cursor(&mut self) {
        let tab = match self.editor_tabs.get(self.active_tab_idx) {
            Some(t) => t,
            None => return,
        };

        let text = tab.buffer.to_string();
        let cursor_line = tab.cursor_line;
        let cursor_col = tab.cursor_col;

        // Calculate byte offset from line/column
        let cursor_pos = {
            let mut pos = 0;
            for (line_idx, line) in text.lines().enumerate() {
                if line_idx == cursor_line {
                    pos += cursor_col.min(line.len());
                    break;
                }
                pos += line.len() + 1; // +1 for newline
            }
            pos
        };

        self.handle_go_to_definition(&text, cursor_pos);
    }

    // ===== Phase 6.5 & 6.6: Settings & Theme UI =====

    /// Handle keyboard shortcuts for Settings and Theme
    fn handle_settings_shortcuts(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            // Ctrl+, (Comma): Open settings
            if i.modifiers.command && i.key_pressed(egui::Key::Comma) {
                tracing::info!("⚙️ Opening settings");
                self.show_settings = !self.show_settings;
            }

            // Ctrl+K Ctrl+T: Open theme editor (VSCode-style)
            // For simplicity, use Ctrl+Shift+T
            if i.modifiers.command && i.modifiers.shift && i.key_pressed(egui::Key::T) {
                tracing::info!("🎨 Opening theme editor");
                self.show_theme_editor = !self.show_theme_editor;
            }

            // Escape: Close settings/theme if open
            if i.key_pressed(egui::Key::Escape) {
                if self.show_settings {
                    self.show_settings = false;
                }
                if self.show_theme_editor {
                    self.show_theme_editor = false;
                }
            }
        });
    }

    // ====================================================================
    // Slack Integration Helper Methods
    // ====================================================================

    /// Set Slack bot token and authenticate
    fn set_slack_token(&mut self, token: String) {
        let slack_client = self.slack_client.clone();
        let tx = self.slack_response_tx.clone();

        self.lsp_runtime.spawn(async move {
            slack_client.set_token(token).await;

            if let Some(tx) = tx {
                let _ = tx.send(SlackResponse::Authenticated);
            }
        });
    }

    /// Load Slack channels
    fn load_slack_channels(&mut self) {
        let slack_client = self.slack_client.clone();
        let tx = self.slack_response_tx.clone();

        self.lsp_runtime.spawn(async move {
            match slack_client.list_channels().await {
                Ok(channels) => {
                    if let Some(tx) = tx {
                        let _ = tx.send(SlackResponse::ChannelsList(channels));
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to load Slack channels: {}", e);
                    if let Some(tx) = tx {
                        let _ = tx.send(SlackResponse::Error(e.to_string()));
                    }
                }
            }
        });
    }

    /// Send a Slack message
    pub fn send_slack_message(&mut self, channel_id: &str, text: &str, thread_ts: Option<&str>) {
        let slack_client = self.slack_client.clone();
        let tx = self.slack_response_tx.clone();
        let channel_id = channel_id.to_string();
        let text = text.to_string();
        let thread_ts = thread_ts.map(|s| s.to_string());

        self.lsp_runtime.spawn(async move {
            match slack_client.send_message(&channel_id, &text, thread_ts.as_deref()).await {
                Ok(_) => {
                    tracing::info!("✅ Message sent to channel {}", channel_id);
                    if let Some(tx) = tx {
                        let _ = tx.send(SlackResponse::MessageSent);
                    }
                }
                Err(e) => {
                    tracing::error!("❌ Failed to send message: {}", e);
                    if let Some(tx) = tx {
                        let _ = tx.send(SlackResponse::Error(e.to_string()));
                    }
                }
            }
        });
    }

    /// Load messages from a Slack channel
    fn load_slack_messages(&mut self, channel_id: &str) {
        let slack_client = self.slack_client.clone();
        let tx = self.slack_response_tx.clone();
        let channel_id = channel_id.to_string();

        self.lsp_runtime.spawn(async move {
            match slack_client.get_messages(&channel_id, 50).await {
                Ok(messages) => {
                    if let Some(tx) = tx {
                        let _ = tx.send(SlackResponse::MessagesList(messages));
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to load Slack messages: {}", e);
                    if let Some(tx) = tx {
                        let _ = tx.send(SlackResponse::Error(e.to_string()));
                    }
                }
            }
        });
    }

    /// Send a message to a Slack channel
    // === Database Helper Methods ===

    /// Add a new database connection
    fn add_database_connection(&mut self) {
        tracing::info!("📊 Adding database connection: name={}, type={:?}, db={}",
            self.new_db_name, self.new_db_type, self.new_db_database);

        let conn = match self.new_db_type {
            native::database::DatabaseType::SQLite => {
                native::database::DatabaseConnection::new_sqlite(
                    self.new_db_name.clone(),
                    self.new_db_database.clone(),
                )
            }
            native::database::DatabaseType::PostgreSQL => {
                let port = self.new_db_port.parse::<u16>().unwrap_or(5432);
                native::database::DatabaseConnection::new_postgresql(
                    self.new_db_name.clone(),
                    self.new_db_host.clone(),
                    port,
                    self.new_db_database.clone(),
                    self.new_db_username.clone(),
                    self.new_db_password.clone(),
                )
            }
            native::database::DatabaseType::MySQL => {
                let port = self.new_db_port.parse::<u16>().unwrap_or(3306);
                native::database::DatabaseConnection::new_mysql(
                    self.new_db_name.clone(),
                    self.new_db_host.clone(),
                    port,
                    self.new_db_database.clone(),
                    self.new_db_username.clone(),
                    self.new_db_password.clone(),
                )
            }
        };

        tracing::info!("✅ Created connection: id={}, name={}", conn.id, conn.name);

        self.database_client.add_connection(conn.clone());
        self.database_connections.push(conn.clone());

        tracing::info!("📋 Total connections: {}", self.database_connections.len());

        // Auto-select the newly added connection
        self.selected_connection_id = Some(conn.id.clone());
        self.load_database_tables(&conn.id);

        // Reset form
        self.new_db_name.clear();
        self.new_db_comment.clear();
        self.new_db_type = native::database::DatabaseType::PostgreSQL;
        self.new_db_connection_type = "default".to_string();
        self.new_db_host = "localhost".to_string();
        self.new_db_port = "5432".to_string();
        self.new_db_database = "postgres".to_string();
        self.new_db_authentication = "User & Password".to_string();
        self.new_db_username.clear();
        self.new_db_password.clear();
        self.new_db_save_mode = "Forever".to_string();
        self.new_db_url = "jdbc:postgresql://localhost:5432/postgres".to_string();
        self.new_db_active_tab = 0;
        self.new_db_test_result = None;
    }

    /// Load tables for a database connection
    fn load_database_tables(&mut self, connection_id: &str) {
        match self.database_client.get_tables(connection_id) {
            Ok(tables) => {
                self.database_tables = tables;
            }
            Err(e) => {
                tracing::error!("Failed to load tables: {}", e);
                self.database_tables.clear();
            }
        }
    }

    /// Execute SQL query
    fn execute_sql_query(&mut self, connection_id: &str) {
        match self.database_client.execute_query(connection_id, &self.sql_query) {
            Ok(result) => {
                self.query_result = Some(result);
            }
            Err(e) => {
                tracing::error!("Failed to execute query: {}", e);
                self.query_result = None;
            }
        }
    }

    /// Render General tab content for database connection dialog
    fn render_db_general_tab(&mut self, ui: &mut egui::Ui) {
        // Connection type and Driver
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Connection type:")
                .size(12.0)
                .color(egui::Color32::from_rgb(180, 180, 180)));
            ui.label(egui::RichText::new(&self.new_db_connection_type)
                .size(12.0)
                .color(egui::Color32::from_rgb(100, 150, 255)));

            ui.add_space(20.0);

            ui.label(egui::RichText::new("Driver:")
                .size(12.0)
                .color(egui::Color32::from_rgb(180, 180, 180)));

            egui::ComboBox::from_id_salt("db_driver")
                .selected_text(self.new_db_type.as_str())
                .show_ui(ui, |ui| {
                    if ui.selectable_value(&mut self.new_db_type, native::database::DatabaseType::PostgreSQL, "PostgreSQL").clicked() {
                        self.update_db_url();
                    }
                    if ui.selectable_value(&mut self.new_db_type, native::database::DatabaseType::MySQL, "MySQL").clicked() {
                        self.update_db_url();
                    }
                    if ui.selectable_value(&mut self.new_db_type, native::database::DatabaseType::SQLite, "SQLite").clicked() {
                        self.update_db_url();
                    }
                });
        });

        ui.add_space(12.0);

        // Conditional fields based on database type
        if self.new_db_type == native::database::DatabaseType::SQLite {
            // SQLite: Only File Path
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("File Path:")
                    .size(12.0)
                    .color(egui::Color32::from_rgb(180, 180, 180)));
                if ui.add(egui::TextEdit::singleline(&mut self.new_db_database)
                    .desired_width(500.0)
                    .hint_text("/path/to/database.db or :memory:")).changed() {
                    self.update_db_url();
                }
            });

            ui.add_space(8.0);
            ui.label(egui::RichText::new("💡 Tip: SQLite creates the file automatically if it doesn't exist")
                .size(11.0)
                .color(egui::Color32::from_rgb(100, 150, 200))
                .italics());

            ui.add_space(4.0);
            ui.label(egui::RichText::new("Examples:")
                .size(11.0)
                .color(egui::Color32::from_rgb(150, 150, 150)));
            ui.label(egui::RichText::new("  • /Users/username/Documents/mydb.db")
                .size(10.0)
                .color(egui::Color32::from_rgb(120, 120, 120))
                .family(egui::FontFamily::Monospace));
            ui.label(egui::RichText::new("  • ./data/local.db")
                .size(10.0)
                .color(egui::Color32::from_rgb(120, 120, 120))
                .family(egui::FontFamily::Monospace));
            ui.label(egui::RichText::new("  • :memory: (in-memory database)")
                .size(10.0)
                .color(egui::Color32::from_rgb(120, 120, 120))
                .family(egui::FontFamily::Monospace));

        } else {
            // PostgreSQL / MySQL: Server connection fields

            // Host and Port
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Host:")
                    .size(12.0)
                    .color(egui::Color32::from_rgb(180, 180, 180)));
                if ui.add(egui::TextEdit::singleline(&mut self.new_db_host)
                    .desired_width(250.0)).changed() {
                    self.update_db_url();
                }

                ui.add_space(20.0);

                ui.label(egui::RichText::new("Port:")
                    .size(12.0)
                    .color(egui::Color32::from_rgb(180, 180, 180)));
                if ui.add(egui::TextEdit::singleline(&mut self.new_db_port)
                    .desired_width(80.0)).changed() {
                    self.update_db_url();
                }
            });

            ui.add_space(12.0);

            // Authentication
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Authentication:")
                    .size(12.0)
                    .color(egui::Color32::from_rgb(180, 180, 180)));

                egui::ComboBox::from_id_salt("db_auth")
                    .selected_text(&self.new_db_authentication)
                    .width(250.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.new_db_authentication, "User & Password".to_string(), "User & Password");
                        ui.selectable_value(&mut self.new_db_authentication, "No Auth".to_string(), "No Auth");
                    });
            });

            ui.add_space(12.0);

            // User
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("User:")
                    .size(12.0)
                    .color(egui::Color32::from_rgb(180, 180, 180)));
                ui.add(egui::TextEdit::singleline(&mut self.new_db_username)
                    .desired_width(250.0));
            });

            ui.add_space(12.0);

            // Password and Save
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Password:")
                    .size(12.0)
                    .color(egui::Color32::from_rgb(180, 180, 180)));
                ui.add(egui::TextEdit::singleline(&mut self.new_db_password)
                    .password(true)
                    .desired_width(250.0)
                    .hint_text("<hidden>"));

                ui.add_space(20.0);

                ui.label(egui::RichText::new("Save:")
                    .size(12.0)
                    .color(egui::Color32::from_rgb(180, 180, 180)));

                egui::ComboBox::from_id_salt("db_save")
                    .selected_text(&self.new_db_save_mode)
                    .width(120.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.new_db_save_mode, "Forever".to_string(), "Forever");
                        ui.selectable_value(&mut self.new_db_save_mode, "Until Restart".to_string(), "Until Restart");
                        ui.selectable_value(&mut self.new_db_save_mode, "Never".to_string(), "Never");
                    });
            });

            ui.add_space(12.0);

            // Database
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Database:")
                    .size(12.0)
                    .color(egui::Color32::from_rgb(180, 180, 180)));
                if ui.add(egui::TextEdit::singleline(&mut self.new_db_database)
                    .desired_width(250.0)).changed() {
                    self.update_db_url();
                }
            });
        }

        ui.add_space(12.0);

        // URL (auto-generated)
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("URL:")
                .size(12.0)
                .color(egui::Color32::from_rgb(180, 180, 180)));
            ui.add(egui::TextEdit::singleline(&mut self.new_db_url)
                .desired_width(500.0)
                .font(egui::TextStyle::Monospace));
        });

        ui.add_space(4.0);
        ui.label(egui::RichText::new("Overrides settings above")
            .size(11.0)
            .color(egui::Color32::from_rgb(120, 120, 120))
            .italics());
    }

    /// Update database URL based on current settings
    fn update_db_url(&mut self) {
        self.new_db_url = match self.new_db_type {
            native::database::DatabaseType::PostgreSQL => {
                format!("jdbc:postgresql://{}:{}/{}",
                    self.new_db_host,
                    self.new_db_port,
                    self.new_db_database)
            }
            native::database::DatabaseType::MySQL => {
                format!("jdbc:mysql://{}:{}/{}",
                    self.new_db_host,
                    self.new_db_port,
                    self.new_db_database)
            }
            native::database::DatabaseType::SQLite => {
                format!("jdbc:sqlite:{}", self.new_db_database)
            }
        };
    }

    /// Test database connection
    fn test_database_connection(&mut self) {
        // Mock test - in real implementation, this would actually test the connection
        let success = match self.new_db_type {
            native::database::DatabaseType::SQLite => {
                !self.new_db_database.is_empty()
            }
            _ => {
                !self.new_db_host.is_empty() && !self.new_db_database.is_empty()
            }
        };

        if success {
            let message = match self.new_db_type {
                native::database::DatabaseType::SQLite => {
                    format!("✅ SQLite file path validated: {}", self.new_db_database)
                }
                _ => {
                    format!("✅ Connected to {} successfully", self.new_db_type.as_str())
                }
            };
            self.new_db_test_result = Some(message);
        } else {
            self.new_db_test_result = Some("❌ Connection failed: Missing required fields".to_string());
        }
    }
}

// ====================================================================
// Helper Functions for LSP Go-to-Definition (Phase 2)
// ====================================================================

/// Parse proto Location to LspLocation
fn parse_lsp_location(proto_loc: native::lsp::lsp_service::Location) -> Option<LspLocation> {
    // URI format: "file:///path/to/file.rs" → "/path/to/file.rs"
    let file_path = proto_loc.uri
        .strip_prefix("file://")
        .unwrap_or(&proto_loc.uri)
        .to_string();

    let range = proto_loc.range?;
    let start = range.start?;

    Some(LspLocation {
        file_path,
        line: start.line as usize,
        column: start.character as usize,
    })
}

/// Calculate line and column from byte offset in text
fn calculate_line_column(text: &str, pos: usize) -> (usize, usize) {
    let mut line = 0;
    let mut col = 0;

    for (i, ch) in text.char_indices() {
        if i >= pos {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }

    (line, col)
}
