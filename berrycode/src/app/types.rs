//! Supporting types for BerryCodeApp

use crate::buffer::TextBuffer;
use crate::native;

/// UI language setting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiLanguage {
    English,
    Japanese,
}

impl UiLanguage {
    pub fn label(&self) -> &'static str {
        match self {
            UiLanguage::English => "English",
            UiLanguage::Japanese => "日本語",
        }
    }
}

/// Simple EditorTab structure (replaces core::virtual_editor::EditorTab)
#[derive(Clone)]
pub struct EditorTab {
    pub file_path: String,
    pub buffer: TextBuffer,
    pub cursor_line: usize,
    pub cursor_col: usize,
    pub is_dirty: bool,
    pub is_readonly: bool,
    pub pending_cursor_jump: Option<(usize, usize)>,
    /// Cached string representation (avoids Rope→String conversion every frame)
    pub text_cache: String,
    /// Buffer version when cache was last updated
    pub text_cache_version: u64,
    /// Git line-level changes for gutter markers
    pub git_line_changes: Vec<crate::native::git::LineChange>,
    /// Whether git line changes have been loaded
    pub git_changes_loaded: bool,
    /// Set of folded line ranges (start_line, end_line) - exclusive end
    pub folded_regions: Vec<(usize, usize)>,
    /// Whether this file is an image (not a text file)
    pub is_image: bool,
    /// If this tab is an image, store the texture handle
    pub image_texture: Option<egui::TextureHandle>,
    /// Whether this file is a 3D model (GLTF/GLB)
    pub is_model: bool,
    /// 3D model preview data (for GLTF/GLB files)
    pub model_data: Option<super::model_preview::ModelPreviewData>,
    /// 3D camera rotation Y (yaw) in radians
    pub model_rot_y: f32,
    /// 3D camera rotation X (pitch) in radians
    pub model_rot_x: f32,
    /// 3D camera zoom factor
    pub model_zoom: f32,
    /// GPU-rendered preview texture ID (for GLB/GLTF via Bevy's PBR renderer)
    pub gpu_preview_texture_id: Option<egui::TextureId>,
}

impl EditorTab {
    pub fn new(file_path: String, content: String) -> Self {
        let buffer = TextBuffer::from_str(&content);
        let version = buffer.version();
        Self {
            file_path,
            buffer,
            cursor_line: 0,
            cursor_col: 0,
            is_dirty: false,
            is_readonly: false,
            pending_cursor_jump: None,
            text_cache: content,
            text_cache_version: version,
            git_line_changes: Vec::new(),
            git_changes_loaded: false,
            folded_regions: Vec::new(),
            is_image: false,
            image_texture: None,
            is_model: false,
            model_data: None,
            model_rot_y: std::f32::consts::PI * 0.25, // 45 degrees
            model_rot_x: std::f32::consts::PI * 0.15, // slight tilt
            model_zoom: 1.0,
            gpu_preview_texture_id: None,
        }
    }

    /// Get the text content, using cache when possible
    pub fn get_text(&mut self) -> &str {
        let current_version = self.buffer.version();
        if current_version != self.text_cache_version {
            self.text_cache = self.buffer.to_string();
            self.text_cache_version = current_version;
        }
        &self.text_cache
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
    Terminal,
    Settings,
    EcsInspector,
    BevyTemplates,
    AssetBrowser,
    SceneEditor,
    GameView,
}

/// Settings Tab Categories (RustRover Style)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    Appearance,
    EditorColor,
    Keybindings,
    Language,
    Plugins,
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
    pub page_limit: usize,   // Number of commits to load per page
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
pub(crate) struct SidebarPanel {
    pub variant: ActivePanel,
    pub icon: &'static str,
    pub _name: &'static str, // For tooltip/accessibility
}

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
    Separator,
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

/// gRPC AI Chat message
#[derive(Debug, Clone)]
pub struct GrpcMessage {
    pub content: String,
    pub is_user: bool,
}

/// LSP async response messages
#[derive(Debug, Clone)]
pub enum LspResponse {
    Connected,
    Diagnostics(Vec<LspDiagnostic>),
    Hover(Option<LspHoverInfo>),
    Completions(Vec<LspCompletionItem>),
    Definition(Vec<LspLocation>),
    References(Vec<LspLocation>),
    InlayHints(Vec<LspInlayHint>),
    CodeActions(Vec<LspCodeAction>),
    MacroExpansion(String, String), // (macro_name, expanded_text)
}

/// Event produced by file tree rendering (one per frame at most).
/// Collected during the read-only render pass and applied afterwards.
#[derive(Debug)]
pub(crate) enum FileTreeEvent {
    ExpandDir(String, bool), // (path, needs_fs_load)
    CollapseDir(String),
    OpenFile(String),
    ContextMenu(String, bool), // (path, is_dir)
    StartAssetDrag(String),    // asset path (3D model dropped into Scene View)
}

/// gRPC response types
#[derive(Debug, Clone)]
pub enum GrpcResponse {
    SessionStarted(String), // Session ID
    ChatChunk(String),      // Streaming chat response chunk
    ChatStreamCompleted,    // Stream finished
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
    /// LSP snippet text (e.g. "fn ${1:name}($2) {\n\t$0\n}")
    pub insert_text: Option<String>,
    /// Whether insert_text is a snippet (has $1, $2 etc.)
    pub is_snippet: bool,
}

/// Inlay hint (type annotation, parameter name, etc.)
#[derive(Debug, Clone)]
pub struct LspInlayHint {
    pub line: usize,
    pub column: usize,
    pub label: String,
    /// "type" or "parameter"
    pub kind: &'static str,
}

/// Code action from LSP (quick fix, refactor, etc.)
#[derive(Debug, Clone)]
pub struct LspCodeAction {
    pub title: String,
    pub kind: Option<String>,
    /// JSON-serialized workspace edit (applied when selected)
    pub edit_json: Option<String>,
    /// JSON-serialized command (executed when selected)
    pub command_json: Option<String>,
}

/// Snippet session: active snippet being expanded with tab stops
#[derive(Debug, Clone)]
pub struct SnippetSession {
    /// Tab stop positions: (line, col, placeholder_len)
    pub tab_stops: Vec<(usize, usize, usize)>,
    /// Current tab stop index
    pub current_stop: usize,
    /// The line where snippet was inserted
    pub start_line: usize,
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
#[allow(dead_code)]
pub(crate) struct PendingGotoDefinition {
    pub word: String,
    pub original_text: String,
}

/// AI Chat mode
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum AIChatMode {
    Chat,       // 対話式（通常のチャット）
    Autonomous, // 自動実行モード（dangerously-skip-permissions）
}

/// Peek definition inline view
#[derive(Debug, Clone)]
pub struct PeekDefinition {
    pub file_path: String,
    pub line: usize,
    pub content_preview: String, // few lines around the definition
    pub anchor_line: usize,      // line in current editor where peek is shown
}

/// Color theme for syntax highlighting
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub(crate) struct ColorTheme {
    pub keyword: egui::Color32,
    pub function: egui::Color32,
    pub type_: egui::Color32,
    pub string: egui::Color32,
    pub number: egui::Color32,
    pub comment: egui::Color32,     // Normal comments: //
    pub doc_comment: egui::Color32, // Doc comments: //!, ///
    pub macro_: egui::Color32,
    pub attribute: egui::Color32,
    pub constant: egui::Color32,
    pub lifetime: egui::Color32,
    pub namespace: egui::Color32,
    pub variable: egui::Color32,
    pub operator: egui::Color32,
}
