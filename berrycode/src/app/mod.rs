//! egui-based main application structure
//! Replaces Dioxus components with egui immediate-mode UI

use crate::focus_stack::FocusLayer;
use crate::native;
use crate::native::fs::DirEntry;
use crate::syntax::SyntaxHighlighter;
use std::collections::HashSet;
use tokio::sync::mpsc;

// ===== Submodules =====
mod ai_chat;
pub(crate) mod ansi;
mod asset_browser;
mod bevy_templates;
mod cargo_completion;
mod code_actions;
mod custom_snippets;
mod debugger;
pub(crate) mod demo_capture;
pub(crate) mod dock;
mod ecs_inspector;
mod editor;
mod events;
mod file_tree;
mod folding;
mod game_view;
mod git;
mod header;
pub(crate) mod i18n;
mod image_preview;
mod inlay_hints;
pub(crate) mod keymap;
pub(crate) mod live_collab;
mod lsp;
mod macro_expand;
mod minimap;
mod model_preview;
pub(crate) mod new_project;
mod peek;
pub(crate) mod plugin_system;
pub(crate) mod preview_3d;
pub(crate) mod remote_dev;
mod rename;
mod run_panel;
pub(crate) mod scene_editor;
mod scene_preview;
mod search;
mod settings;
mod shortcuts;
mod sidebar;
pub(crate) mod snippets;
mod status_bar;
mod terminal;
pub(crate) mod terminal_emulator;
pub(crate) mod test_runner;
pub mod types;
pub(crate) mod utils;
pub(crate) mod vim_mode;

// Re-export public types
pub use types::*;

// ===== Syntax Highlighting Color Palette =====
// VS Code Dark+ inspired color scheme for Rust syntax highlighting

pub(crate) mod syntax_colors {
    use egui::Color32;

    pub const KEYWORD: Color32 = Color32::from_rgb(234, 147, 71); // #EA9347 Orange
    pub const FUNCTION: Color32 = Color32::from_rgb(84, 166, 224); // #54A6E0 Sky Blue
    pub const TYPE: Color32 = Color32::from_rgb(232, 194, 82); // #E8C252 Yellow
    pub const STRING: Color32 = Color32::from_rgb(184, 214, 84); // #B8D654 Lime Green
    pub const NUMBER: Color32 = Color32::from_rgb(181, 206, 168); // #B5CEA8 Light Green
    pub const COMMENT: Color32 = Color32::from_rgb(128, 128, 128); // #808080 Gray
    pub const DOC_COMMENT: Color32 = Color32::from_rgb(106, 153, 85); // #6A9955 Green
    pub const MACRO: Color32 = Color32::from_rgb(84, 166, 224); // #54A6E0 Sky Blue
    pub const ATTRIBUTE: Color32 = Color32::from_rgb(197, 134, 192); // #C586C0 Pink
    pub const CONSTANT: Color32 = Color32::from_rgb(197, 134, 192); // #C586C0 Pink
    pub const LIFETIME: Color32 = Color32::from_rgb(78, 201, 176); // #4EC9B0 Cyan
    pub const NAMESPACE: Color32 = Color32::from_rgb(212, 212, 212); // #D4D4D4 White
    pub const VARIABLE: Color32 = Color32::from_rgb(212, 212, 212); // デフォルト白と同じ
    pub const OPERATOR: Color32 = Color32::from_rgb(212, 212, 212); // #D4D4D4 White
}

// ===== UI Color Palette =====

pub(crate) mod ui_colors {
    use egui::Color32;

    pub const SIDEBAR_BG: Color32 = Color32::from_rgb(25, 26, 28); // #191A1C Dark Gray
    pub const EDITOR_BG: Color32 = Color32::from_rgb(25, 26, 28); // #191A1C Dark Gray
    pub const TEXT_DEFAULT: Color32 = Color32::from_rgb(212, 212, 212); // #D4D4D4 Light Gray
    pub const BORDER: Color32 = Color32::from_rgb(54, 57, 59); // #36393B Medium Gray
}

// ===== File Icon Color Palette =====

pub(crate) mod file_icon_colors {
    use egui::Color32;

    pub const RUST_ORANGE: Color32 = Color32::from_rgb(255, 152, 0); // #FF9800
    pub const CONFIG_GRAY: Color32 = Color32::from_rgb(128, 128, 128); // #808080
    pub const JSON_YELLOW: Color32 = Color32::from_rgb(255, 203, 0); // #FFCB00
    pub const MARKDOWN_BLUE: Color32 = Color32::from_rgb(66, 165, 245); // #42A5F5
    pub const JAVASCRIPT_YELLOW: Color32 = Color32::from_rgb(247, 223, 30); // #F7DF1E
    pub const TYPESCRIPT_BLUE: Color32 = Color32::from_rgb(41, 127, 214); // #297FD6
    pub const PYTHON_GREEN: Color32 = Color32::from_rgb(52, 168, 83); // #34A853
    pub const SHELL_GREEN: Color32 = Color32::from_rgb(76, 175, 80); // #4CAF50
    pub const HTML_ORANGE: Color32 = Color32::from_rgb(229, 115, 0); // #E57300
    pub const CSS_BLUE: Color32 = Color32::from_rgb(66, 165, 245); // #42A5F5
    pub const IMAGE_PURPLE: Color32 = Color32::from_rgb(156, 39, 176); // #9C27B0
    pub const SVG_AMBER: Color32 = Color32::from_rgb(255, 179, 0); // #FFB300
    pub const GIT_ORANGE: Color32 = Color32::from_rgb(240, 98, 35); // #F06223
    pub const PROTO_PURPLE: Color32 = Color32::from_rgb(156, 39, 176); // #9C27B0
}

/// Main panels in the Activity Bar
const MAIN_PANELS: &[SidebarPanel] = &[
    SidebarPanel {
        variant: ActivePanel::Explorer,
        icon: "\u{ea83}", // codicon-folder
        _name: "Explorer",
    },
    SidebarPanel {
        variant: ActivePanel::Search,
        icon: "\u{ea6d}", // codicon-search
        _name: "Search",
    },
    SidebarPanel {
        variant: ActivePanel::Git,
        icon: "\u{ea84}", // codicon-github
        _name: "Git",
    },
    SidebarPanel {
        variant: ActivePanel::Terminal,
        icon: "\u{ea85}", // codicon-terminal
        _name: "Terminal",
    },
    SidebarPanel {
        variant: ActivePanel::EcsInspector,
        icon: "\u{ea92}", // codicon-inspect
        _name: "ECS Inspector",
    },
    SidebarPanel {
        variant: ActivePanel::BevyTemplates,
        icon: "\u{ea61}", // codicon-symbol-misc
        _name: "Bevy Templates",
    },
    SidebarPanel {
        variant: ActivePanel::AssetBrowser,
        icon: "\u{eb64}", // codicon-folder-library
        _name: "Asset Browser",
    },
    SidebarPanel {
        variant: ActivePanel::SceneEditor,
        icon: "\u{ea9a}", // codicon-symbol-structure
        _name: "Scene Editor",
    },
    SidebarPanel {
        variant: ActivePanel::GameView,
        icon: "\u{ebb5}", // codicon-play
        _name: "Game View",
    },
];

/// Main application state
pub struct BerryCodeApp {
    // === Project State ===
    pub(crate) root_path: String,
    pub(crate) selected_file: Option<(String, String)>, // (path, content)
    /// Whether the project picker should be shown (no project loaded yet)
    pub(crate) show_project_picker: bool,
    /// Path being typed in the project picker dialog
    pub(crate) project_picker_path: String,
    /// Recently opened projects for quick access
    pub(crate) recent_projects: Vec<String>,

    // === UI State ===
    pub(crate) active_panel: ActivePanel,
    pub(crate) sidebar_width: f32,

    // === Editor State ===
    pub(crate) editor_tabs: Vec<EditorTab>,
    pub(crate) active_tab_idx: usize,
    pub(crate) syntax_highlighter: SyntaxHighlighter, // Regex-based highlighter

    // === File Tree State ===
    pub(crate) file_tree_cache: Vec<DirEntry>, // Cached directory tree
    pub(crate) file_tree_load_pending: bool,
    pub(crate) expanded_dirs: HashSet<String>, // Set of expanded directory paths

    // === Terminal State (iTerm2-style PTY emulator) ===
    pub(crate) terminal: terminal_emulator::TerminalEmulator,

    // === Search State ===
    pub(crate) search_query: String,
    pub(crate) search_dialog_open: bool,
    pub(crate) search_case_sensitive: bool,
    pub(crate) current_search_index: usize,
    pub(crate) search_results: Vec<SearchMatch>,
    pub(crate) replace_query: String,
    pub(crate) show_replace: bool,

    // === Git State ===
    pub(crate) git_current_branch: String,
    pub(crate) git_status: Vec<native::git::GitStatus>,
    pub(crate) git_commit_message: String,
    pub(crate) git_initialized: bool,
    pub(crate) git_active_tab: GitTab,
    pub(crate) git_history_state: GitHistoryState,
    pub(crate) git_branch_state: GitBranchState,
    pub(crate) git_remote_state: GitRemoteState,
    pub(crate) git_tag_state: GitTagState,
    pub(crate) git_stash_state: GitStashState,
    pub(crate) git_diff_state: GitDiffState,

    // === LSP State ===
    pub(crate) lsp_runtime: std::sync::Arc<tokio::runtime::Runtime>,
    pub(crate) lsp_native_client: Option<std::sync::Arc<native::lsp_native::NativeLspClient>>,
    pub(crate) lsp_response_tx: Option<mpsc::UnboundedSender<LspResponse>>,
    pub(crate) lsp_connected: bool,
    pub(crate) lsp_diagnostics: Vec<LspDiagnostic>,
    pub(crate) lsp_hover_info: Option<LspHoverInfo>,
    pub(crate) lsp_completions: Vec<LspCompletionItem>,
    pub(crate) lsp_show_completions: bool,
    pub(crate) lsp_show_hover: bool,
    pub(crate) lsp_response_rx: Option<mpsc::UnboundedReceiver<LspResponse>>,
    pub(crate) lsp_diagnostics_rx:
        Option<mpsc::UnboundedReceiver<native::lsp_native::PublishedDiagnostics>>,

    // === Status Message ===
    pub(crate) status_message: String,
    pub(crate) status_message_timestamp: Option<std::time::Instant>,

    // === Go-to-Definition State ===
    pub(crate) pending_goto_definition: Option<PendingGotoDefinition>,
    pub(crate) definition_picker_locations: Vec<LspLocation>,
    pub(crate) show_definition_picker: bool,

    // === Find References State ===
    pub(crate) lsp_references: Vec<LspLocation>,
    pub(crate) show_references_panel: bool,

    // === Inlay Hints State ===
    pub(crate) lsp_inlay_hints: Vec<LspInlayHint>,
    pub(crate) inlay_hints_enabled: bool,
    pub(crate) inlay_hints_last_request: Option<std::time::Instant>,

    // === Code Actions State ===
    pub(crate) lsp_code_actions: Vec<LspCodeAction>,
    pub(crate) show_code_actions: bool,
    pub(crate) code_action_line: usize,

    // === Snippet State ===
    pub(crate) snippet_session: Option<SnippetSession>,

    // === Rename Symbol State ===
    pub(crate) rename_dialog_open: bool,
    pub(crate) rename_new_name: String,

    // gRPC for AI integration
    pub(crate) grpc_client: native::grpc::GrpcClient,
    pub(crate) grpc_session_id: Option<String>,
    pub(crate) grpc_connected: bool,
    pub(crate) grpc_response_tx: Option<mpsc::UnboundedSender<GrpcResponse>>,
    pub(crate) grpc_response_rx: Option<mpsc::UnboundedReceiver<GrpcResponse>>,
    pub(crate) grpc_streaming_message: Option<String>,

    // AI Chat Panel State
    pub(crate) ai_chat_mode: AIChatMode,
    pub(crate) grpc_messages: Vec<GrpcMessage>,
    pub(crate) grpc_input: String,
    pub(crate) grpc_streaming: bool,
    pub(crate) grpc_current_response: String,
    pub(crate) chat_attachment: Option<String>,

    // === Settings ===
    pub(crate) show_settings: bool,
    pub(crate) active_settings_tab: SettingsTab,
    pub(crate) ui_language: UiLanguage,

    // === Theme (Customizable Syntax Colors) ===
    pub(crate) show_theme_editor: bool,
    pub(crate) keyword_color: egui::Color32,
    pub(crate) function_color: egui::Color32,
    pub(crate) type_color: egui::Color32,
    pub(crate) string_color: egui::Color32,
    pub(crate) number_color: egui::Color32,
    pub(crate) comment_color: egui::Color32,
    pub(crate) doc_comment_color: egui::Color32,
    pub(crate) macro_color: egui::Color32,
    pub(crate) attribute_color: egui::Color32,
    pub(crate) constant_color: egui::Color32,
    pub(crate) lifetime_color: egui::Color32,
    pub(crate) namespace_color: egui::Color32,
    pub(crate) variable_color: egui::Color32,
    pub(crate) operator_color: egui::Color32,

    // === Multi-cursor State ===
    pub(crate) multi_cursors: Vec<usize>, // additional cursor char positions (besides the primary egui cursor)

    // === Peek Definition ===
    pub(crate) peek_definition: Option<PeekDefinition>,

    // === Focus Management ===
    pub(crate) active_focus: FocusLayer,

    // === New File/Folder/Project Dialog ===
    pub(crate) new_file_dialog_open: bool,
    pub(crate) new_file_name: String,
    pub(crate) new_folder_dialog_open: bool,
    pub(crate) new_folder_name: String,
    pub(crate) new_project_dialog_open: bool,
    pub(crate) new_project_name: String,
    pub(crate) new_project_path: String,
    pub(crate) new_project_template: new_project::ProjectTemplate,

    // === Git Blame Cache ===
    pub(crate) blame_cache_line: usize,
    pub(crate) blame_cache_text: String,
    pub(crate) blame_cache_file: String,

    // === File Watcher ===
    pub(crate) file_watcher: Option<native::watcher::FileWatcher>,

    // === ECS Inspector ===
    pub(crate) ecs_inspector: crate::bevy_ide::inspector::ecs_state::EcsInspectorState,
    pub(crate) ecs_inspector_tab: ecs_inspector::EcsInspectorTab,

    // === Scene Preview ===
    pub(crate) scene_preview: crate::bevy_ide::scene_preview::parser::ScenePreviewState,

    // === Asset Browser ===
    pub(crate) asset_browser: crate::bevy_ide::assets::scanner::AssetBrowserState,

    // === Bevy Templates ===
    pub(crate) template_type: bevy_templates::TemplateType,
    pub(crate) template_name: String,
    pub(crate) template_fields: Vec<(String, String)>,
    pub(crate) template_params: Vec<String>,
    pub(crate) template_variants: Vec<String>,

    // === Debug State ===
    pub(crate) debug_state: debugger::DebugState,
    pub(crate) dap_client: Option<crate::native::dap::DapClient>,
    pub(crate) dap_event_rx:
        Option<tokio::sync::mpsc::UnboundedReceiver<crate::native::dap::DapEvent>>,

    // === Test Runner State ===
    pub(crate) test_runner: test_runner::TestRunnerState,

    // === User Snippets ===
    pub(crate) user_snippets: Vec<custom_snippets::LoadedSnippet>,

    // === Vim Mode ===
    pub(crate) vim: vim_mode::VimState,

    // === Plugin System ===
    pub(crate) plugin_manager: plugin_system::PluginManager,

    // === Remote Development ===
    pub(crate) remote: remote_dev::RemoteConnection,
    pub(crate) remote_dialog: remote_dev::RemoteDialogState,

    // === Live Collaboration ===
    pub(crate) collab: live_collab::CollabState,
    pub(crate) collab_dialog: live_collab::CollabDialogState,

    // === Context Menu State ===
    pub(crate) context_menu_path: Option<String>,
    pub(crate) context_menu_is_dir: bool,
    pub(crate) context_menu_pos: egui::Pos2,
    pub(crate) rename_file_dialog_open: bool,
    pub(crate) rename_file_old_path: String,
    pub(crate) rename_file_new_name: String,

    // === Run Bevy Project State ===
    pub(crate) run_output: Vec<String>,
    pub(crate) run_process: Option<std::process::Child>,
    pub(crate) run_panel_open: bool,
    pub(crate) run_release_mode: bool,
    pub(crate) run_output_rx: Option<std::sync::mpsc::Receiver<String>>,

    // === Phase Q: Console filter state ===
    pub(crate) console_filter_text: String,
    pub(crate) console_show_info: bool,
    pub(crate) console_show_warning: bool,
    pub(crate) console_show_error: bool,
    pub(crate) console_auto_scroll: bool,

    // === Play in Editor (Game View) State ===
    pub(crate) game_view_open: bool,
    pub(crate) game_view_texture: Option<egui::TextureHandle>,
    pub(crate) game_view_last_capture: Option<std::time::Instant>,
    pub(crate) game_view_window_hidden: bool,

    // === Scene Editor (Unity-like) ===
    pub(crate) scene_model: scene_editor::model::SceneModel,
    /// The most recently clicked entity in a multi-select context. The inspector
    /// displays this entity's properties.
    pub(crate) primary_selected_id: Option<u64>,
    pub(crate) scene_view_texture_id: Option<egui::TextureId>,
    pub(crate) scene_needs_sync: bool,
    /// Clipboard for copy/paste of components in the Inspector.
    pub(crate) component_clipboard: Option<scene_editor::model::ComponentData>,
    /// Filter text for the Add Component search popup.
    pub(crate) add_component_filter: String,
    /// Whether the Add Component search popup is open.
    pub(crate) add_component_popup_open: bool,
    pub(crate) scene_orbit_yaw: f32,
    pub(crate) scene_orbit_pitch: f32,
    pub(crate) scene_orbit_distance: f32,
    pub(crate) scene_orbit_target: [f32; 3],
    pub(crate) scene_ortho: bool,
    pub(crate) scene_ortho_scale: f32,
    pub(crate) scene_shadows_enabled: bool,
    pub(crate) scene_bloom_enabled: bool,
    pub(crate) scene_bloom_intensity: f32,
    pub(crate) scene_tonemapping: u8,
    pub(crate) scene_ssao_enabled: bool,
    pub(crate) scene_taa_enabled: bool,
    pub(crate) scene_fog_enabled: bool,
    pub(crate) scene_fog_color: [f32; 3],
    pub(crate) scene_fog_start: f32,
    pub(crate) scene_fog_end: f32,
    pub(crate) scene_dof_enabled: bool,
    pub(crate) scene_dof_focus_distance: f32,
    pub(crate) scene_dof_aperture: f32,
    pub(crate) fly_mode_active: bool,
    pub(crate) fly_camera_speed: f32,
    pub(crate) gizmo_mode: scene_editor::gizmo::GizmoMode,
    /// Currently dragged gizmo handle. None when not dragging.
    pub(crate) gizmo_dragging: Option<scene_editor::gizmo::GizmoDrag>,
    /// Start position of an in-progress box selection drag (screen coords).
    pub(crate) box_select_start: Option<egui::Pos2>,

    // === Scene Editor: Hierarchy panel state ===
    pub(crate) hierarchy_filter: String,
    pub(crate) hierarchy_dragged: Option<u64>,
    /// Drop target while a hierarchy drag is in progress. `Some(None)` means
    /// "drop on root", `Some(Some(id))` means "make a child of id".
    pub(crate) hierarchy_drop_target: Option<Option<u64>>,
    pub(crate) renaming_entity_id: Option<u64>,
    pub(crate) rename_buffer: String,

    // === Scene Editor: Undo/Redo history (Phase 62: command-pattern overlay) ===
    pub(crate) command_history: scene_editor::history::CommandHistory,

    // === Scene Editor: Snapping ===
    pub(crate) snap_enabled: bool,
    pub(crate) snap_value: f32,

    // === Scene Editor: Asset drag & drop (Phase H) ===
    /// Path of the asset currently being dragged from the file tree.
    /// Set when the user starts dragging a droppable file; cleared on drop or release.
    pub(crate) dragged_asset_path: Option<String>,

    // === Scene Editor: Profiler panel (Phase P) ===
    pub(crate) profiler: scene_editor::profiler::ProfilerState,

    // === Scene Editor: Particle preview (Phase M) ===
    /// Editor-only live particle simulation state, advanced each frame and
    /// drawn as 2D dots over the Scene View.
    pub(crate) particle_preview: scene_editor::particle_preview::ParticlePreview,

    // === Scene Editor: Animation playback (Phase K) ===
    /// Editor-only per-entity animation playback state. Drives the Timeline
    /// window and applies sampled transforms during scene sync.
    pub(crate) animation_playback: scene_editor::animation::AnimationPlayback,
    /// Whether the floating Timeline window is currently visible.
    pub(crate) timeline_open: bool,
    /// Whether the floating Dopesheet / Curve Editor window is visible.
    pub(crate) dopesheet_open: bool,
    /// Whether the curve overlay is shown in the dopesheet.
    pub(crate) dopesheet_show_curves: bool,
    /// Whether the Animator Editor window is currently visible.
    pub(crate) animator_editor_open: bool,
    /// The animator controller being edited (if any).
    pub(crate) editing_animator: Option<scene_editor::animator::AnimatorController>,
    /// File path of the animator controller being edited.
    pub(crate) editing_animator_path: String,
    /// Index of the animator state node currently being dragged (Phase 3).
    pub(crate) animator_dragging_state: Option<usize>,
    /// Source state index for a pending "Add Transition From Here" action (Phase 3).
    pub(crate) pending_transition_from: Option<usize>,
    /// Clipboard for entity copy/paste in the scene hierarchy (Phase 4).
    pub(crate) entity_clipboard: Option<scene_editor::prefab::PrefabFile>,
    /// Whether the quad-view mode is active in the Scene View.
    pub(crate) quad_view_enabled: bool,
    /// Per-quadrant independent camera states (Phase 10).
    pub(crate) quad_camera_states: [scene_editor::scene_view::QuadCameraState; 4],
    /// Index of the currently active quadrant (0..3). The main camera
    /// parameters mirror this quadrant's state.
    pub(crate) active_quad_idx: usize,
    /// Whether an audio preview is currently playing in the inspector.
    pub(crate) audio_preview_playing: bool,
    /// Path of the audio file currently being previewed.
    pub(crate) audio_preview_path: String,

    // === Scene Editor: Material Preview GPU texture (Phase 8) ===
    /// egui texture id for the GPU-rendered material preview sphere.
    /// Updated each frame from `MaterialPreviewRender` in `berry_ui_system`.
    pub(crate) material_preview_texture_id: Option<egui::TextureId>,
    /// PBR values to push to the material preview sphere each frame.
    /// Written by the inspector, consumed by `berry_ui_system`.
    pub(crate) material_preview_color: [f32; 3],
    pub(crate) material_preview_metallic: f32,
    pub(crate) material_preview_roughness: f32,
    pub(crate) material_preview_emissive: [f32; 3],
    /// Dirty flag: set true by the inspector when PBR values change.
    pub(crate) material_preview_dirty: bool,

    // === Scene Editor: Play Mode (Phase 15) ===
    pub(crate) play_mode: scene_editor::play_mode::PlayModeState,
    pub(crate) play_mode_snapshot: Option<scene_editor::model::SceneModel>,

    // === Scene Editor: Physics Simulation (Phase 16) ===
    pub(crate) physics_state: scene_editor::physics_sim::PhysicsState,

    // === Scene Editor: Build Settings (Phase 18) ===
    pub(crate) build_settings_open: bool,
    pub(crate) build_settings: scene_editor::build_settings::BuildSettings,
    pub(crate) player_settings: scene_editor::build_settings::PlayerSettings,

    // === Customizable Keyboard Shortcuts ===
    pub(crate) keymap: keymap::Keymap,

    // === Dockable Tool Panel (Phase 3) ===
    pub(crate) tool_panel_open: bool,
    pub(crate) active_tool_tab: dock::ToolTab,

    // === Asset Thumbnails (Phase 11) ===
    pub(crate) thumbnail_cache: scene_editor::thumbnail_cache::ThumbnailCache,

    // === Multiple Scene Tabs (Phase 8) ===
    pub(crate) scene_tabs: Vec<scene_editor::scene_tabs::SceneTab>,
    pub(crate) active_scene_tab: usize,

    // === Asset Dependency Tracking (Phase 12) ===
    pub(crate) asset_dependencies: Option<scene_editor::asset_deps::AssetDependencies>,

    // === Terrain Brush (Phase 66) ===
    pub(crate) terrain_brush: scene_editor::terrain::TerrainBrushState,

    // === Visual Script Editor (Phase 72) ===
    pub(crate) visual_script_editor_open: bool,
    pub(crate) editing_visual_script: Option<scene_editor::visual_script::VisualScript>,

    // === Shader Graph Editor (Phase 74) ===
    pub(crate) shader_graph_editor_open: bool,
    pub(crate) editing_shader_graph: Option<scene_editor::shader_graph::ShaderGraph>,

    // === Hot Reload (Phase 75) ===
    pub(crate) hot_reload: scene_editor::hot_reload::HotReloadState,

    // === Build Pipeline (Phase 76) ===
    pub(crate) build_output: Vec<String>,
    pub(crate) build_process: Option<std::process::Child>,
    pub(crate) build_output_rx: Option<std::sync::mpsc::Receiver<String>>,

    // === Save-time Cargo Check ===
    pub(crate) cargo_check_rx: Option<std::sync::mpsc::Receiver<String>>,

    // === Test Mode CLI ===
    pub(crate) test_mode: bool,
    pub(crate) test_command_rx: Option<std::sync::mpsc::Receiver<String>>,

    // === Demo Capture (screenshots + video) ===
    pub(crate) demo_capture: demo_capture::DemoCapture,

    // === Scanned User Components (bidirectional sync) ===
    pub(crate) scanned_user_components: Vec<scene_editor::script_scan::ScannedComponent>,

    // === Scene Merge (Phase 64) ===
    pub(crate) merge_panel_open: bool,
    pub(crate) merge_base_path: String,
    pub(crate) merge_ours_path: String,
    pub(crate) merge_theirs_path: String,
    pub(crate) merge_result: Option<scene_editor::scene_merge::MergeResult>,

    // === Bevy System Graph ===
    pub(crate) system_graph_open: bool,
    pub(crate) system_graph: scene_editor::system_graph::SystemGraph,

    // === Bevy Event Monitor ===
    pub(crate) event_monitor_open: bool,
    pub(crate) event_log: Vec<scene_editor::event_monitor::EventEntry>,
    pub(crate) event_filter_text: String,
    pub(crate) event_filter_types: std::collections::HashSet<String>,

    // === Bevy Query Visualizer ===
    pub(crate) query_viz_open: bool,
    pub(crate) queries: Vec<scene_editor::query_viz::QueryDef>,

    // === Bevy States Editor ===
    pub(crate) state_editor_open: bool,
    pub(crate) state_graph: scene_editor::state_editor::StateGraph,

    // === Bevy Plugin Browser ===
    pub(crate) plugin_browser_open: bool,
    pub(crate) plugin_search_query: String,
    pub(crate) plugin_search_results: Vec<scene_editor::plugin_browser::CrateResult>,

    // === Bevy Version Management ===
    pub(crate) bevy_version: Option<String>,
}

impl BerryCodeApp {
    /// Apply the BerryCode egui style — IntelliJ-inspired dark theme
    pub fn setup_egui_style(ctx: &egui::Context) {
        let mut style = egui::Style::default();
        let mut visuals = egui::Visuals::dark();

        // === Colors ===
        let bg_dark = egui::Color32::from_rgb(30, 31, 34); // main background
        let bg_panel = egui::Color32::from_rgb(43, 45, 48); // sidebar/panel
        let bg_input = egui::Color32::from_rgb(50, 52, 56); // input fields
        let bg_hover = egui::Color32::from_rgb(55, 57, 61); // hover state
        let bg_active = egui::Color32::from_rgb(65, 68, 74); // active/pressed
        let bg_selected = egui::Color32::from_rgb(38, 79, 140); // brighter blue selection
        let border = egui::Color32::from_rgb(60, 63, 68); // borders
        let border_focus = egui::Color32::from_rgb(75, 110, 175); // focused border (accent)
        let text = egui::Color32::from_rgb(205, 207, 213); // primary text
        let text_dim = egui::Color32::from_rgb(140, 143, 150); // secondary text

        visuals.override_text_color = None;
        visuals.window_fill = bg_panel;
        visuals.panel_fill = bg_dark;
        visuals.extreme_bg_color = bg_input;
        visuals.code_bg_color = egui::Color32::from_rgb(35, 36, 40);
        visuals.faint_bg_color = egui::Color32::from_rgb(38, 40, 43);

        // Window
        visuals.window_stroke = egui::Stroke::new(1.0, border);
        visuals.window_shadow = egui::epaint::Shadow {
            offset: egui::vec2(0.0, 4.0),
            blur: 12.0,
            spread: 0.0,
            color: egui::Color32::from_black_alpha(80),
        };
        visuals.window_rounding = egui::Rounding::same(8.0);
        visuals.menu_rounding = egui::Rounding::same(6.0);

        // Selection
        visuals.selection.bg_fill = bg_selected;
        visuals.selection.stroke = egui::Stroke::new(0.0, egui::Color32::TRANSPARENT);

        // Text cursor
        visuals.text_cursor.stroke.color = egui::Color32::from_rgb(180, 190, 220);

        // === Widget Styles ===

        // Non-interactive (labels, separators)
        visuals.widgets.noninteractive.bg_fill = bg_dark;
        visuals.widgets.noninteractive.weak_bg_fill = bg_dark;
        visuals.widgets.noninteractive.bg_stroke =
            egui::Stroke::new(0.0, egui::Color32::TRANSPARENT);
        visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, text);
        visuals.widgets.noninteractive.rounding = egui::Rounding::same(6.0);

        // Inactive (buttons, checkboxes at rest)
        visuals.widgets.inactive.bg_fill = bg_panel;
        visuals.widgets.inactive.weak_bg_fill = bg_panel;
        visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, border);
        visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, text);
        visuals.widgets.inactive.rounding = egui::Rounding::same(6.0);
        visuals.widgets.inactive.expansion = 0.0;

        // Hovered
        visuals.widgets.hovered.bg_fill = bg_hover;
        visuals.widgets.hovered.weak_bg_fill = bg_hover;
        visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, border_focus);
        visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
        visuals.widgets.hovered.rounding = egui::Rounding::same(6.0);
        visuals.widgets.hovered.expansion = 1.0;

        // Active (pressed)
        visuals.widgets.active.bg_fill = bg_active;
        visuals.widgets.active.weak_bg_fill = bg_active;
        visuals.widgets.active.bg_stroke = egui::Stroke::new(1.5, border_focus);
        visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
        visuals.widgets.active.rounding = egui::Rounding::same(6.0);
        visuals.widgets.active.expansion = 0.0;

        // Open (combo boxes, menus open state)
        visuals.widgets.open.bg_fill = bg_active;
        visuals.widgets.open.weak_bg_fill = bg_active;
        visuals.widgets.open.bg_stroke = egui::Stroke::new(1.0, border_focus);
        visuals.widgets.open.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
        visuals.widgets.open.rounding = egui::Rounding::same(6.0);

        // Popup shadow
        visuals.popup_shadow = egui::epaint::Shadow {
            offset: egui::vec2(0.0, 6.0),
            blur: 16.0,
            spread: 2.0,
            color: egui::Color32::from_black_alpha(100),
        };

        // Striped backgrounds (tables)
        visuals.striped = true;

        // Separator
        visuals.widgets.noninteractive.bg_stroke =
            egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 47, 50));

        style.visuals = visuals;

        // === Spacing ===
        style.spacing.item_spacing = egui::vec2(8.0, 6.0); // more breathing room
        style.spacing.button_padding = egui::vec2(14.0, 6.0); // wider, taller buttons
        style.spacing.window_margin = egui::Margin::same(12.0); // window inner padding
        style.spacing.menu_margin = egui::Margin::same(8.0);
        style.spacing.indent = 18.0; // tree indent
        style.spacing.interact_size = egui::vec2(40.0, 24.0); // minimum interactive element size
        style.spacing.slider_width = 160.0;
        style.spacing.combo_width = 160.0;
        style.spacing.text_edit_width = 200.0;
        style.spacing.scroll = egui::style::ScrollStyle {
            bar_width: 8.0,
            ..Default::default()
        };

        // === Text (even pixel sizes for crisp rendering) ===
        use egui::FontId;
        style
            .text_styles
            .insert(egui::TextStyle::Heading, FontId::proportional(18.0));
        style
            .text_styles
            .insert(egui::TextStyle::Body, FontId::proportional(14.0));
        style
            .text_styles
            .insert(egui::TextStyle::Small, FontId::proportional(12.0));
        style
            .text_styles
            .insert(egui::TextStyle::Button, FontId::proportional(14.0));
        style
            .text_styles
            .insert(egui::TextStyle::Monospace, FontId::monospace(14.0));

        // === Interaction ===
        style.interaction.show_tooltips_only_when_still = false;

        ctx.set_style(style);
    }

    /// Open a native OS folder selection dialog (cross-platform via rfd).
    /// Returns the selected folder path, or None if cancelled.
    fn native_folder_dialog() -> Option<String> {
        let folder = rfd::FileDialog::new()
            .set_title("Select Bevy Project Folder")
            .pick_folder()?;
        let path = folder.to_string_lossy().to_string();
        let path = path.trim_end_matches(['/', '\\']).to_string();
        if path.is_empty() {
            None
        } else {
            Some(path)
        }
    }

    /// Resolve the project path: CLI arg > env > prompt user
    fn resolve_project_path() -> String {
        // 1. Check command-line arguments: berrycode /path/to/project
        let args: Vec<String> = std::env::args().collect();
        if args.len() > 1 {
            let path = &args[1];
            if std::path::Path::new(path).is_dir() {
                tracing::info!("Project path from CLI arg: {}", path);
                return path.clone();
            }
        }

        // 2. Check BERRYCODE_PROJECT env var
        if let Ok(path) = std::env::var("BERRYCODE_PROJECT") {
            if std::path::Path::new(&path).is_dir() {
                tracing::info!("Project path from env: {}", path);
                return path;
            }
        }

        // 3. No project specified — use empty placeholder; the picker will show
        String::new()
    }

    /// Load recent projects from ~/.berrycode/recent_projects.txt
    fn load_recent_projects() -> Vec<String> {
        let path = dirs::home_dir()
            .map(|h| format!("{}/.berrycode/recent_projects.txt", h.display()))
            .unwrap_or_default();
        std::fs::read_to_string(&path)
            .unwrap_or_default()
            .lines()
            .filter(|l| !l.is_empty() && std::path::Path::new(l).is_dir())
            .map(|l| l.to_string())
            .collect()
    }

    /// Save a project to recent projects list
    fn save_to_recent_projects(project_path: &str) {
        let config_dir = dirs::home_dir()
            .map(|h| format!("{}/.berrycode", h.display()))
            .unwrap_or_default();
        let _ = std::fs::create_dir_all(&config_dir);
        let file_path = format!("{}/recent_projects.txt", config_dir);
        let mut projects = Self::load_recent_projects();
        projects.retain(|p| p != project_path);
        projects.insert(0, project_path.to_string());
        projects.truncate(10); // Keep last 10
        let _ = std::fs::write(&file_path, projects.join("\n"));
    }

    /// Open a project: set root_path, refresh file tree, start LSP, etc.
    pub(crate) fn open_project(&mut self, path: &str) {
        self.root_path = path.to_string();
        self.show_project_picker = false;
        self.file_tree_cache.clear();
        self.file_tree_load_pending = true;
        self.expanded_dirs.clear();
        self.editor_tabs.clear();
        self.active_tab_idx = 0;
        self.git_initialized = false;

        // Start file watcher for new project
        if let Ok(mut watcher) = crate::native::watcher::FileWatcher::new() {
            let _ = watcher.watch(&self.root_path);
            self.file_watcher = Some(watcher);
        }

        // Save to recent projects
        Self::save_to_recent_projects(path);

        // Auto-import from main.rs if scene is empty
        let main_path = format!("{}/src/main.rs", path);
        if self.scene_model.entities.is_empty() {
            if let Ok(code) = crate::native::fs::read_file(&main_path) {
                let imported = crate::app::scene_editor::code_import::import_scene_from_code(&code);
                if !imported.entities.is_empty() {
                    self.scene_model = imported;
                    self.scene_needs_sync = true;
                    tracing::info!(
                        "Auto-imported {} entities from main.rs",
                        self.scene_model.entities.len()
                    );
                }
            }
        }

        self.status_message = format!("Opened project: {}", path);
        self.status_message_timestamp = Some(std::time::Instant::now());
        tracing::info!("Opened project: {}", path);
    }

    /// Render the project picker screen (shown when no project is loaded)
    pub(crate) fn render_project_picker(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::from_rgb(25, 27, 31)))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(80.0);

                    // Logo / Title
                    ui.label(
                        egui::RichText::new("BerryCode")
                            .size(48.0)
                            .color(egui::Color32::from_rgb(126, 89, 161))
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("Bevy Game Engine IDE")
                            .size(16.0)
                            .color(egui::Color32::from_gray(140)),
                    );

                    ui.add_space(40.0);

                    // Open project section
                    ui.group(|ui| {
                        ui.set_width(500.0);
                        ui.vertical(|ui| {
                            ui.heading("Open Project");
                            ui.add_space(8.0);

                            ui.horizontal(|ui| {
                                ui.label("Path:");
                                ui.add_sized(
                                    [300.0, 22.0],
                                    egui::TextEdit::singleline(&mut self.project_picker_path)
                                        .hint_text("/path/to/your/bevy/project"),
                                );
                                if ui.button("Browse...").clicked() {
                                    // Open native folder picker dialog
                                    if let Some(path) = Self::native_folder_dialog() {
                                        self.project_picker_path = path;
                                    }
                                }
                                if ui.button("Open").clicked()
                                    && !self.project_picker_path.is_empty()
                                {
                                    let path = self.project_picker_path.clone();
                                    if std::path::Path::new(&path).is_dir() {
                                        self.open_project(&path);
                                    } else {
                                        self.status_message =
                                            format!("Directory not found: {}", path);
                                        self.status_message_timestamp =
                                            Some(std::time::Instant::now());
                                    }
                                }
                            });

                            ui.add_space(8.0);

                            // New Bevy Project button
                            if ui.button("+ New Bevy Project").clicked() {
                                self.new_project_dialog_open = true;
                            }
                        });
                    });

                    ui.add_space(20.0);

                    // Recent projects
                    if !self.recent_projects.is_empty() {
                        ui.group(|ui| {
                            ui.set_width(500.0);
                            ui.vertical(|ui| {
                                ui.heading("Recent Projects");
                                ui.add_space(4.0);

                                let recent = self.recent_projects.clone();
                                for project in &recent {
                                    let name = project.rsplit('/').next().unwrap_or(project);
                                    ui.horizontal(|ui| {
                                        if ui
                                            .add(
                                                egui::Button::new(
                                                    egui::RichText::new(name).size(14.0),
                                                )
                                                .frame(false),
                                            )
                                            .clicked()
                                        {
                                            self.open_project(project);
                                        }
                                        ui.label(
                                            egui::RichText::new(project)
                                                .size(11.0)
                                                .color(egui::Color32::from_gray(120)),
                                        );
                                    });
                                }
                            });
                        });
                    }

                    ui.add_space(40.0);
                    ui.label(
                        egui::RichText::new("v0.2.0 | Bevy 0.15 | 285 tests | 31MB binary")
                            .size(11.0)
                            .color(egui::Color32::from_gray(100)),
                    );
                });
            });
    }

    /// Create new application instance
    pub fn new() -> Self {
        // Check command-line args for project path, otherwise show picker
        let root_path = Self::resolve_project_path();

        tracing::info!("📁 Project root: {}", root_path);

        let terminal_working_dir = root_path.clone();

        // Create Tokio runtime for async LSP operations
        let lsp_runtime = std::sync::Arc::new(
            tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime for LSP"),
        );

        // Create native LSP client (returns client + diagnostics receiver)
        let (lsp_native_client_inner, lsp_diagnostics_rx) =
            native::lsp_native::NativeLspClient::new();
        let lsp_native_client = std::sync::Arc::new(lsp_native_client_inner);

        // Create gRPC client
        let grpc_client = native::grpc::GrpcClient::new("http://[::1]:50051");

        // Create LSP response channel
        let (lsp_tx, lsp_rx) = mpsc::unbounded_channel();

        // Create gRPC response channel
        let (grpc_tx, grpc_rx) = mpsc::unbounded_channel();

        // Create file watcher
        let file_watcher = match native::watcher::FileWatcher::new() {
            Ok(mut watcher) => {
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

        // Spawn native LSP initialization task
        let client_clone = lsp_native_client.clone();
        let root_path_clone = root_path.clone();
        let tx_clone = lsp_tx.clone();

        lsp_runtime.spawn(async move {
            match client_clone.start_server("rust", &root_path_clone).await {
                Ok(_) => {
                    tracing::info!("✅ Native LSP (rust-analyzer) started");
                    let _ = tx_clone.send(LspResponse::Connected);
                }
                Err(e) => {
                    tracing::warn!("⚠️  Native LSP startup failed: {} (will use fallback)", e);
                }
            }
        });

        // Spawn gRPC connection and session initialization task
        let runtime_clone = lsp_runtime.clone();
        let root_path_for_grpc = root_path.clone();
        let grpc_tx_clone = grpc_tx.clone();
        let grpc_client_clone = grpc_client.clone();
        runtime_clone.spawn(async move {
            for attempt in 1..=24u32 {
                match grpc_client_clone.connect().await {
                    Ok(_) => {
                        tracing::info!("✅ gRPC client connected to berry-api-server");
                        match grpc_client_clone.start_session(root_path_for_grpc.clone(), true).await {
                            Ok(session_id) => {
                                tracing::info!("🎯 gRPC chat session started: {}", session_id);
                                let _ = grpc_tx_clone.send(GrpcResponse::SessionStarted(session_id));
                            }
                            Err(e) => {
                                tracing::error!("❌ Failed to start gRPC session: {:#}", e);
                            }
                        }
                        return;
                    }
                    Err(_) => {
                        tracing::info!("⏳ berry-api-server not ready, retrying in 5s (attempt {}/24)...", attempt);
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    }
                }
            }
            tracing::warn!("⚠️  berry-api-server not found after 2 minutes. Start it with: cd berry_api && cargo run --bin berry-api-server");
        });

        let bevy_version = scene_editor::bevy_version::detect_bevy_version(&root_path);

        let show_picker = root_path.is_empty();
        let recent = Self::load_recent_projects();
        let home = dirs::home_dir()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_default();
        let picker_path = home.clone();
        if !root_path.is_empty() {
            Self::save_to_recent_projects(&root_path);
        }
        // Keep root_path empty if no project specified — picker will handle it
        let root_path = if root_path.is_empty() {
            String::new()
        } else {
            root_path
        };
        let root_path_ref = root_path.clone();

        let mut app = Self {
            root_path,
            selected_file: None,
            show_project_picker: show_picker,
            project_picker_path: picker_path,
            recent_projects: recent,
            active_panel: ActivePanel::Explorer,
            sidebar_width: 300.0,
            editor_tabs: {
                // Auto-open src/main.rs if it exists
                let main_path = format!("{}/src/main.rs", root_path_ref);
                if std::path::Path::new(&main_path).exists() {
                    if let Ok(content) = crate::native::fs::read_file(&main_path) {
                        vec![types::EditorTab::new(main_path, content)]
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                }
            },
            active_tab_idx: 0,
            syntax_highlighter: SyntaxHighlighter::new(),
            file_tree_cache: Vec::new(),
            file_tree_load_pending: true,
            expanded_dirs: {
                let mut dirs = HashSet::new();
                // Auto-expand src/ directory
                let src_dir = format!("{}/src", root_path_ref);
                if std::path::Path::new(&src_dir).is_dir() {
                    dirs.insert(root_path_ref.clone());
                    dirs.insert(src_dir);
                }
                dirs
            },
            terminal: terminal_emulator::TerminalEmulator::new(&terminal_working_dir),
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
            lsp_native_client: Some(lsp_native_client),
            lsp_response_tx: Some(lsp_tx),
            lsp_connected: false,
            lsp_diagnostics: Vec::new(),
            lsp_hover_info: None,
            lsp_completions: Vec::new(),
            lsp_show_completions: false,
            lsp_show_hover: false,
            lsp_response_rx: Some(lsp_rx),
            lsp_diagnostics_rx: Some(lsp_diagnostics_rx),
            status_message: String::new(),
            status_message_timestamp: None,
            pending_goto_definition: None,
            definition_picker_locations: Vec::new(),
            show_definition_picker: false,
            lsp_references: Vec::new(),
            show_references_panel: false,

            lsp_inlay_hints: Vec::new(),
            inlay_hints_enabled: true,
            inlay_hints_last_request: None,

            lsp_code_actions: Vec::new(),
            show_code_actions: false,
            code_action_line: 0,

            snippet_session: None,

            rename_dialog_open: false,
            rename_new_name: String::new(),

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
            chat_attachment: None,
            show_settings: false,
            active_settings_tab: SettingsTab::EditorColor,
            ui_language: UiLanguage::English,
            show_theme_editor: false,
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
            multi_cursors: Vec::new(),
            peek_definition: None,
            active_focus: FocusLayer::Editor,

            new_file_dialog_open: false,
            new_file_name: String::new(),
            new_folder_dialog_open: false,
            new_folder_name: String::new(),
            new_project_dialog_open: false,
            new_project_name: String::new(),
            new_project_path: dirs::home_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| ".".to_string()),
            new_project_template: new_project::ProjectTemplate::Empty2D,

            blame_cache_line: usize::MAX,
            blame_cache_text: String::new(),
            blame_cache_file: String::new(),

            file_watcher,

            ecs_inspector: Default::default(),
            ecs_inspector_tab: ecs_inspector::EcsInspectorTab::default(),

            scene_preview: Default::default(),

            asset_browser: Default::default(),

            template_type: bevy_templates::TemplateType::default(),
            template_name: String::new(),
            template_fields: Vec::new(),
            template_params: Vec::new(),
            template_variants: Vec::new(),

            debug_state: Default::default(),
            dap_client: None,
            dap_event_rx: None,
            test_runner: Default::default(),
            user_snippets: custom_snippets::load_user_snippets(),
            vim: Default::default(),
            plugin_manager: Default::default(),
            remote: Default::default(),
            remote_dialog: Default::default(),
            collab: Default::default(),
            collab_dialog: Default::default(),

            context_menu_path: None,
            context_menu_is_dir: false,
            context_menu_pos: egui::Pos2::ZERO,
            rename_file_dialog_open: false,
            rename_file_old_path: String::new(),
            rename_file_new_name: String::new(),

            run_output: Vec::new(),
            run_process: None,
            run_panel_open: false,
            run_release_mode: false,
            run_output_rx: None,

            console_filter_text: String::new(),
            console_show_info: true,
            console_show_warning: true,
            console_show_error: true,
            console_auto_scroll: true,

            game_view_open: false,
            game_view_texture: None,
            game_view_last_capture: None,
            game_view_window_hidden: false,

            scene_model: {
                // Auto-import entities from main.rs if project has one
                let main_path = format!("{}/src/main.rs", root_path_ref);
                if let Ok(code) = crate::native::fs::read_file(&main_path) {
                    let imported = scene_editor::code_import::import_scene_from_code(&code);
                    if !imported.entities.is_empty() {
                        tracing::info!(
                            "Auto-imported {} entities from main.rs",
                            imported.entities.len()
                        );
                        imported
                    } else {
                        scene_editor::model::SceneModel::new()
                    }
                } else {
                    scene_editor::model::SceneModel::new()
                }
            },
            primary_selected_id: None,
            scene_view_texture_id: None,
            scene_needs_sync: false,
            component_clipboard: None,
            add_component_filter: String::new(),
            add_component_popup_open: false,
            scene_orbit_yaw: std::f32::consts::FRAC_PI_4,
            scene_orbit_pitch: 0.5,
            scene_orbit_distance: 8.0,
            scene_orbit_target: [0.0, 0.0, 0.0],
            scene_ortho: false,
            scene_ortho_scale: 8.0,
            scene_shadows_enabled: true,
            scene_bloom_enabled: false,
            scene_bloom_intensity: 0.3,
            scene_tonemapping: 3,
            scene_ssao_enabled: false,
            scene_taa_enabled: false,
            scene_fog_enabled: false,
            scene_fog_color: [0.7, 0.8, 1.0],
            scene_fog_start: 50.0,
            scene_fog_end: 200.0,
            scene_dof_enabled: false,
            scene_dof_focus_distance: 5.0,
            scene_dof_aperture: 0.02,
            fly_mode_active: false,
            fly_camera_speed: 5.0,
            gizmo_mode: scene_editor::gizmo::GizmoMode::Move,
            gizmo_dragging: None,
            box_select_start: None,

            hierarchy_filter: String::new(),
            hierarchy_dragged: None,
            hierarchy_drop_target: None,
            renaming_entity_id: None,
            rename_buffer: String::new(),

            command_history: scene_editor::history::CommandHistory::new(),

            snap_enabled: false,
            snap_value: 0.5,

            dragged_asset_path: None,

            profiler: {
                let mut p = scene_editor::profiler::ProfilerState::default();
                p.open = false;
                p
            },

            particle_preview: scene_editor::particle_preview::ParticlePreview::default(),

            animation_playback: {
                let mut ap = scene_editor::animation::AnimationPlayback::default();
                ap.playing = true;
                ap
            },
            timeline_open: false,
            dopesheet_open: false,
            dopesheet_show_curves: true,
            animator_editor_open: false,
            editing_animator: Some({
                let mut c = scene_editor::animator::AnimatorController::default();
                c.states.push(scene_editor::animator::AnimState {
                    name: "Walk".into(),
                    clip_name: "walk".into(),
                    speed: 1.0,
                    looped: true,
                    position: [300.0, 100.0],
                });
                c.states.push(scene_editor::animator::AnimState {
                    name: "Run".into(),
                    clip_name: "run".into(),
                    speed: 1.5,
                    looped: true,
                    position: [300.0, 250.0],
                });
                c.transitions.push(scene_editor::animator::AnimTransition {
                    from_state: 0,
                    to_state: 1,
                    condition: scene_editor::animator::TransitionCondition::BoolParam {
                        name: "is_running".into(),
                        value: true,
                    },
                    blend_duration: 0.2,
                });
                c.transitions.push(scene_editor::animator::AnimTransition {
                    from_state: 1,
                    to_state: 0,
                    condition: scene_editor::animator::TransitionCondition::BoolParam {
                        name: "is_running".into(),
                        value: false,
                    },
                    blend_duration: 0.3,
                });
                c.parameters.push(scene_editor::animator::AnimParam::Bool {
                    name: "is_running".into(),
                    value: false,
                });
                c.parameters.push(scene_editor::animator::AnimParam::Float {
                    name: "speed".into(),
                    value: 1.0,
                });
                c
            }),
            editing_animator_path: String::new(),
            animator_dragging_state: None,
            pending_transition_from: None,
            entity_clipboard: None,
            quad_view_enabled: false,
            quad_camera_states: scene_editor::scene_view::QuadCameraState::defaults(),
            active_quad_idx: 0,
            audio_preview_playing: false,
            audio_preview_path: String::new(),

            material_preview_texture_id: None,
            material_preview_color: [0.8, 0.8, 0.8],
            material_preview_metallic: 0.0,
            material_preview_roughness: 0.5,
            material_preview_emissive: [0.0, 0.0, 0.0],
            material_preview_dirty: false,

            play_mode: scene_editor::play_mode::PlayModeState::Stopped,
            play_mode_snapshot: None,

            physics_state: scene_editor::physics_sim::PhysicsState::default(),

            build_settings_open: false,
            build_settings: scene_editor::build_settings::BuildSettings::default(),
            player_settings: scene_editor::build_settings::PlayerSettings::default(),

            keymap: keymap::Keymap::load(),

            tool_panel_open: false,
            active_tool_tab: dock::ToolTab::Console,

            thumbnail_cache: scene_editor::thumbnail_cache::ThumbnailCache::new(),

            scene_tabs: vec![],
            active_scene_tab: 0,

            asset_dependencies: None,

            terrain_brush: scene_editor::terrain::TerrainBrushState::default(),

            visual_script_editor_open: false,
            editing_visual_script: Some({
                let mut s = scene_editor::visual_script::VisualScript::default();
                s.nodes.push(scene_editor::visual_script::ScriptNode {
                    id: 2,
                    node_type: scene_editor::visual_script::NodeType::Print {
                        message: "Hello World".into(),
                    },
                    position: [300.0, 80.0],
                });
                s.nodes.push(scene_editor::visual_script::ScriptNode {
                    id: 3,
                    node_type: scene_editor::visual_script::NodeType::Branch,
                    position: [200.0, 200.0],
                });
                s.nodes.push(scene_editor::visual_script::ScriptNode {
                    id: 4,
                    node_type: scene_editor::visual_script::NodeType::Delay { seconds: 1.0 },
                    position: [400.0, 200.0],
                });
                s.edges.push(scene_editor::visual_script::ScriptEdge {
                    from_node: 1,
                    from_pin: 0,
                    to_node: 2,
                    to_pin: 0,
                });
                s.edges.push(scene_editor::visual_script::ScriptEdge {
                    from_node: 2,
                    from_pin: 0,
                    to_node: 3,
                    to_pin: 0,
                });
                s
            }),

            shader_graph_editor_open: false,
            editing_shader_graph: Some(scene_editor::shader_graph::ShaderGraph::default()),

            hot_reload: scene_editor::hot_reload::HotReloadState::default(),

            build_output: Vec::new(),
            build_process: None,
            build_output_rx: None,

            cargo_check_rx: None,

            test_mode: false,
            test_command_rx: None,

            demo_capture: demo_capture::DemoCapture::new(),

            scanned_user_components: Vec::new(),

            merge_panel_open: false,
            merge_base_path: String::new(),
            merge_ours_path: String::new(),
            merge_theirs_path: String::new(),
            merge_result: None,

            system_graph_open: false,
            system_graph: scene_editor::system_graph::SystemGraph::default(),

            event_monitor_open: false,
            event_log: Vec::new(),
            event_filter_text: String::new(),
            event_filter_types: std::collections::HashSet::new(),

            query_viz_open: false,
            queries: Vec::new(),

            state_editor_open: false,
            state_graph: scene_editor::state_editor::StateGraph::default_game_states(),

            plugin_browser_open: false,
            plugin_search_query: String::new(),
            plugin_search_results: Vec::new(),

            bevy_version,
        };

        // === Test Mode CLI: --test-mode ===
        if std::env::args().any(|a| a == "--test-mode") {
            app.test_mode = true;
            let (tx, rx) = std::sync::mpsc::channel();
            app.test_command_rx = Some(rx);
            std::thread::spawn(move || {
                let listener = match std::net::TcpListener::bind("127.0.0.1:17171") {
                    Ok(l) => l,
                    Err(e) => {
                        tracing::error!("Failed to bind test mode port 17171: {}", e);
                        return;
                    }
                };
                tracing::info!("Test mode: listening on 127.0.0.1:17171");
                for stream in listener.incoming().flatten() {
                    use std::io::{BufRead, BufReader};
                    let reader = BufReader::new(&stream);
                    for line in reader.lines().flatten() {
                        if tx.send(line).is_err() {
                            return;
                        }
                    }
                }
            });
        }

        app
    }

    /// Take a snapshot of the current scene model so the next destructive edit
    /// can be undone. Call this BEFORE the mutation.
    ///
    /// This is the backward-compatible wrapper that records a [`SceneCommand::Generic`].
    /// Prefer calling `self.command_history.execute(specific_command, &self.scene_model)`
    /// directly for operations that have a dedicated [`SceneCommand`] variant.
    pub(crate) fn scene_snapshot(&mut self) {
        self.command_history.execute(
            scene_editor::history::SceneCommand::Generic("edit".into()),
            &self.scene_model,
        );
    }
}

/// Recursively copy a directory and all its contents
fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// Bevy startup system: configure egui fonts and style
pub fn setup_egui_fonts_and_style(mut egui_ctx: bevy_egui::EguiContexts) {
    let ctx = egui_ctx.ctx_mut();

    // Setup fonts with Japanese support
    let mut fonts = egui::FontDefinitions::default();

    // Add Codicon font for icons (embedded at compile time)
    const CODICON_FONT_BYTES: &[u8] = include_bytes!("../../assets/codicon.ttf");
    tracing::info!("Loaded Codicon font: {} bytes", CODICON_FONT_BYTES.len());
    fonts.font_data.insert(
        "codicon".to_owned(),
        egui::FontData::from_static(CODICON_FONT_BYTES).into(),
    );

    // Add Nerd Font Symbols for terminal glyphs (powerline, devicons, etc.)
    const NERD_FONT_BYTES: &[u8] = include_bytes!("../../assets/nerd-symbols.ttf");
    fonts.font_data.insert(
        "nerd-symbols".to_owned(),
        egui::FontData::from_static(NERD_FONT_BYTES).into(),
    );

    // Create a custom font family for Codicon icons
    fonts.families.insert(
        egui::FontFamily::Name("codicon".into()),
        vec!["codicon".to_owned()],
    );

    // Also add to Proportional family as fallback
    fonts
        .families
        .get_mut(&egui::FontFamily::Proportional)
        .unwrap()
        .insert(0, "codicon".to_owned());

    // Add Nerd Font Symbols as fallback for Monospace and Proportional
    fonts
        .families
        .get_mut(&egui::FontFamily::Monospace)
        .unwrap()
        .push("nerd-symbols".to_owned());
    fonts
        .families
        .get_mut(&egui::FontFamily::Proportional)
        .unwrap()
        .push("nerd-symbols".to_owned());
    tracing::info!("Codicon + Nerd Font Symbols loaded");

    // Add Japanese font (try monospace fonts first for better baseline alignment)
    let japanese_font_paths = vec![
        "/System/Library/Fonts/Osaka.ttf",
        "/System/Library/Fonts/ヒラギノ角ゴシック W3.ttc",
        "/System/Library/Fonts/Hiragino Sans GB.ttc",
        "/Library/Fonts/ヒラギノ角ゴ ProN W3.otf",
    ];

    for path in japanese_font_paths {
        if let Ok(font_data) = std::fs::read(path) {
            let mut font_data_with_tweak = egui::FontData::from_owned(font_data);
            font_data_with_tweak.tweak.y_offset_factor = 0.15;
            font_data_with_tweak.tweak.y_offset = 2.0;

            fonts
                .font_data
                .insert("japanese".to_owned(), font_data_with_tweak.into());

            fonts
                .families
                .get_mut(&egui::FontFamily::Proportional)
                .unwrap()
                .push("japanese".to_owned());

            fonts
                .families
                .get_mut(&egui::FontFamily::Monospace)
                .unwrap()
                .push("japanese".to_owned());

            tracing::info!("Loaded Japanese font: {} (with baseline tweak)", path);
            break;
        }
    }

    ctx.set_fonts(fonts);

    // Custom dark theme with #191a1c background and unified #D4D4D4 white
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(egui::Color32::from_rgb(212, 212, 212));
    visuals.panel_fill = egui::Color32::from_rgb(25, 26, 28);
    visuals.window_fill = egui::Color32::from_rgb(25, 26, 28);
    visuals.extreme_bg_color = egui::Color32::from_rgb(25, 26, 28);
    visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(25, 26, 28);
    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(25, 26, 28);
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(45, 47, 50);
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(60, 63, 65);
    visuals.selection.bg_fill = egui::Color32::from_rgb(38, 79, 140);
    visuals.selection.stroke = egui::Stroke::new(0.0, egui::Color32::TRANSPARENT);
    visuals.code_bg_color = egui::Color32::from_rgb(25, 26, 28);
    ctx.set_visuals(visuals);

    // Also apply the One Dark style
    BerryCodeApp::setup_egui_style(ctx);

    tracing::info!("egui fonts and style configured");
}

/// Main UI update system for Bevy
pub fn berry_ui_system(
    mut app: bevy::ecs::system::NonSendMut<BerryCodeApp>,
    mut egui_ctx: bevy_egui::EguiContexts,
    mut drop_events: bevy::ecs::event::EventReader<bevy::window::FileDragAndDrop>,
    mut preview_scene: bevy::ecs::system::ResMut<preview_3d::ModelPreviewScene>,
    mut scene_render: bevy::ecs::system::ResMut<scene_editor::bevy_render::SceneEditorRender>,
    mut mat_preview: bevy::ecs::system::ResMut<
        scene_editor::material_preview::MaterialPreviewRender,
    >,
) {
    // Handle drag-and-drop files from OS (via Bevy's FileDragAndDrop event)
    for event in drop_events.read() {
        if let bevy::window::FileDragAndDrop::DroppedFile { path_buf, .. } = event {
            let path = path_buf;
            let path_str = path.to_string_lossy().to_string();
            if path.is_file() {
                let file_name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let dest = format!("{}/{}", app.root_path, file_name);
                match std::fs::copy(&path_str, &dest) {
                    Ok(_) => {
                        app.status_message = format!("Imported: {}", file_name);
                        app.status_message_timestamp = Some(std::time::Instant::now());
                        app.file_tree_cache.clear();
                        app.file_tree_load_pending = true;
                        app.open_file_from_path(&dest);
                    }
                    Err(e) => {
                        app.status_message = format!("Import failed: {}", e);
                        app.status_message_timestamp = Some(std::time::Instant::now());
                    }
                }
            } else if path.is_dir() {
                let dir_name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let dest = format!("{}/{}", app.root_path, dir_name);
                if let Err(e) = copy_dir_recursive(path, std::path::Path::new(&dest)) {
                    app.status_message = format!("Import failed: {}", e);
                    app.status_message_timestamp = Some(std::time::Instant::now());
                } else {
                    app.status_message = format!("Imported folder: {}", dir_name);
                    app.status_message_timestamp = Some(std::time::Instant::now());
                    app.file_tree_cache.clear();
                    app.file_tree_load_pending = true;
                }
            }
        }
    }

    {
        let ctx = egui_ctx.ctx_mut();

        // Global panel switching: Ctrl+1..9 — processed BEFORE any panel rendering
        // so it works regardless of which widget has focus
        ctx.input(|i| {
            if i.modifiers.command {
                if i.key_pressed(egui::Key::Num1) {
                    app.active_panel = types::ActivePanel::Explorer;
                }
                if i.key_pressed(egui::Key::Num2) {
                    app.active_panel = types::ActivePanel::Search;
                }
                if i.key_pressed(egui::Key::Num3) {
                    app.active_panel = types::ActivePanel::Git;
                }
                if i.key_pressed(egui::Key::Num4) {
                    app.active_panel = types::ActivePanel::Terminal;
                }
                if i.key_pressed(egui::Key::Num5) {
                    app.active_panel = types::ActivePanel::EcsInspector;
                }
                if i.key_pressed(egui::Key::Num6) {
                    app.active_panel = types::ActivePanel::BevyTemplates;
                }
                if i.key_pressed(egui::Key::Num7) {
                    app.active_panel = types::ActivePanel::AssetBrowser;
                }
                if i.key_pressed(egui::Key::Num8) {
                    app.active_panel = types::ActivePanel::SceneEditor;
                }
                if i.key_pressed(egui::Key::Num9) {
                    app.active_panel = types::ActivePanel::GameView;
                }
            }
        });

        // Show project picker if no project loaded
        if app.show_project_picker {
            app.render_project_picker(ctx);
            // Still render the New Project dialog if open
            app.render_new_project_dialog(ctx);
            return;
        }

        // Initialize Git repository on first update
        if !app.git_initialized {
            app.git_initialized = true;
            app.refresh_git_status();
            app.refresh_git_history();
            app.refresh_git_branches();
            app.refresh_git_remotes();
            app.refresh_git_tags();
            app.refresh_git_stashes();
            tracing::info!("Git repository initialized for {}", app.root_path);
        }

        // Poll LSP responses (non-blocking)
        app.poll_lsp_responses();

        // Poll inlay hints (periodic, throttled)
        app.poll_inlay_hints();

        // Poll test runner results (non-blocking)
        app.poll_test_results();

        // Poll DAP events (non-blocking)
        app.poll_dap_events();

        // Poll remote development responses
        app.poll_remote_responses();

        // Poll collaboration state
        app.poll_collab();

        // Poll gRPC responses (non-blocking)
        app.poll_grpc_responses();

        // Poll file watcher events (non-blocking)
        app.poll_file_watcher_events();

        // Poll run process output (non-blocking)
        app.poll_run_output();

        // Poll cargo check results (non-blocking)
        app.poll_cargo_check();

        // Poll test mode commands (non-blocking)
        app.poll_test_commands();

        // Update game view texture (captures running game window)
        app.update_game_view(ctx);

        // Handle keyboard shortcuts
        app.handle_editor_shortcuts(ctx);
        app.handle_goto_definition_shortcut(ctx);
        app.handle_find_references_shortcut(ctx);
        app.handle_code_action_shortcut(ctx);
        app.handle_macro_expand_shortcut(ctx);
        app.handle_debug_shortcuts(ctx);
        app.handle_settings_shortcuts(ctx);

        // Render top header bar (VS Code style)
        app.render_top_header(ctx);

        // Render UI panels
        app.render_activity_bar(ctx);

        // Render dockable tool panel (bottom, must reserve space before CentralPanel)
        app.render_tool_panel(ctx);

        // Conditional panels based on active panel
        if app.active_panel == ActivePanel::Terminal {
            app.render_terminal_fullscreen(ctx);
        } else if app.active_panel == ActivePanel::Git {
            app.render_sidebar(ctx);
            app.render_git_diff_viewer(ctx);
        } else if app.active_panel == ActivePanel::GameView {
            // Game View: sidebar (file tree) + central game view
            app.render_sidebar(ctx);
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::none()
                        .fill(ui_colors::EDITOR_BG)
                        .inner_margin(egui::Margin::same(8.0)),
                )
                .show(ctx, |ui| {
                    app.render_game_view_central(ui);
                });
        } else if app.active_panel == ActivePanel::SceneEditor {
            // Unity-style 3-column layout:
            //   Left   = Hierarchy  (handled by render_sidebar)
            //   Right  = Inspector  (dedicated SidePanel::right, shown BEFORE CentralPanel)
            //   Center = Scene View (CentralPanel)
            app.render_sidebar(ctx);
            egui::SidePanel::right("scene_inspector")
                .default_width(300.0)
                .width_range(240.0..=500.0)
                .resizable(true)
                .frame(
                    egui::Frame::none()
                        .fill(ui_colors::SIDEBAR_BG)
                        .inner_margin(egui::Margin::same(8.0)),
                )
                .show(ctx, |ui| {
                    app.render_scene_inspector(ui);
                });
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::none()
                        .fill(ui_colors::EDITOR_BG)
                        .inner_margin(egui::Margin::same(8.0)),
                )
                .show(ctx, |ui| {
                    app.render_scene_view(ui);
                });
        } else {
            app.render_sidebar(ctx);
            app.render_ai_chat_panel(ctx);
            app.render_editor_area(ctx);
        }

        // Render scene preview panel for .scn.ron files
        app.render_scene_preview(ctx);

        // Render debug panel (bottom panel when debugging)
        app.render_debug_panel(ctx);

        // Render run output panel (bottom panel when running cargo)
        app.render_run_panel(ctx);

        // Render Play in Editor (Game View) window
        app.render_game_view(ctx);

        // Render diagnostics panel
        if !app.lsp_diagnostics.is_empty() {
            app.render_diagnostics_panel(ctx);
        }

        app.render_status_bar(ctx);

        // Render search dialog if open
        if app.search_dialog_open {
            app.render_search_dialog(ctx);
        }

        // Render settings dialog
        if app.show_settings {
            app.render_settings_dialog(ctx);
        }

        // Render theme editor
        if app.show_theme_editor {
            app.render_theme_editor(ctx);
        }

        // Render LSP hover tooltip
        if app.lsp_show_hover {
            app.render_lsp_hover(ctx);
        }

        // Render definition picker window
        if app.show_definition_picker {
            app.render_definition_picker(ctx);
        }

        // Render references panel
        if app.show_references_panel {
            app.render_references_panel(ctx);
        }

        // Render rename dialog
        app.render_rename_dialog(ctx);

        // Render new file/folder dialogs
        app.render_new_file_dialog(ctx);
        app.render_new_folder_dialog(ctx);
        app.render_new_project_dialog(ctx);

        // Render file tree context menu and rename dialog
        app.render_file_context_menu(ctx);
        app.render_rename_file_dialog(ctx);

        // Phase P: editor-side profiler (FPS / frame time / entity count).
        app.render_profiler(ctx);

        // Phase K: floating timeline window for animation keyframe editing.
        app.render_timeline(ctx);

        // Phase 10: floating dopesheet / curve editor window.
        app.render_dopesheet(ctx);

        // Phase 13: floating animator controller editor window.
        app.render_animator_editor(ctx);

        // Phase 18: floating build settings window.
        app.render_build_settings(ctx);

        // Phase 64: floating scene merge panel.
        app.render_merge_panel(ctx);

        // Phase 72: floating visual script editor.
        app.render_visual_script_editor(ctx);

        // Phase 74: floating shader graph editor.
        app.render_shader_graph_editor(ctx);

        // Bevy-specific: System Execution Graph.
        app.render_system_graph(ctx);

        // Bevy-specific: Event Monitor.
        app.render_event_monitor(ctx);

        // Bevy-specific: Query Visualizer.
        app.render_query_viz(ctx);

        // Bevy-specific: States Editor.
        app.render_state_editor(ctx);

        // Bevy-specific: Plugin Browser.
        app.render_plugin_browser(ctx);

        // Phase 75: hot reload polling.
        {
            let root = app.root_path.clone();
            if let Some(msg) = app.hot_reload.poll(&root) {
                app.status_message = msg;
                app.status_message_timestamp = Some(std::time::Instant::now());
            }
        }

        // Reactive Mode: only repaint when status message is active
        if app.status_message_timestamp.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    } // end of ctx borrow scope

    // GPU 3D preview: if active tab is GLTF/GLB, update preview scene and assign texture
    {
        let idx = app.active_tab_idx;
        let mut wants_gpu = false;
        let mut model_path: Option<String> = None;
        let mut orbit_yaw = 0.0f32;
        let mut orbit_pitch = 0.0f32;
        let mut orbit_zoom = 1.0f32;

        if !app.editor_tabs.is_empty() && idx < app.editor_tabs.len() {
            let tab = &app.editor_tabs[idx];
            if tab.is_model {
                let ext = tab
                    .file_path
                    .rsplit('.')
                    .next()
                    .unwrap_or("")
                    .to_lowercase();
                if ext == "glb" || ext == "gltf" {
                    wants_gpu = true;
                    model_path = Some(tab.file_path.clone());
                    orbit_yaw = tab.model_rot_y;
                    orbit_pitch = tab.model_rot_x;
                    orbit_zoom = tab.model_zoom;
                }
            }
        }

        if wants_gpu {
            if let Some(ref path) = model_path {
                if preview_scene.requested_model_path.as_ref() != Some(path) {
                    preview_scene.requested_model_path = Some(path.clone());
                }
            }
            preview_scene.orbit_yaw = orbit_yaw;
            preview_scene.orbit_pitch = orbit_pitch;
            preview_scene.orbit_distance = orbit_zoom * 3.0;

            if let Some(handle) = preview_scene.render_target.clone() {
                let texture_id = egui_ctx.add_image(handle);
                app.editor_tabs[idx].gpu_preview_texture_id = Some(texture_id);
            }
        } else {
            if preview_scene.loaded_model_path.is_some() {
                preview_scene.requested_model_path = None;
            }
            if !app.editor_tabs.is_empty() && idx < app.editor_tabs.len() {
                app.editor_tabs[idx].gpu_preview_texture_id = None;
            }
        }
    }

    // === Scene Editor render-target plumbing ===
    // 1) Push the current orbit parameters from the UI state into the Bevy
    //    resource so `update_scene_editor_camera` can pick them up.
    // 2) Re-register the render-target image with egui every frame and stash
    //    the texture id back on the app so the Scene View panel can draw it.
    {
        scene_render.orbit_yaw = app.scene_orbit_yaw;
        scene_render.orbit_pitch = app.scene_orbit_pitch;
        scene_render.orbit_distance = app.scene_orbit_distance;
        scene_render.orbit_target = app.scene_orbit_target;
        scene_render.ortho = app.scene_ortho;
        scene_render.ortho_scale = app.scene_ortho_scale;
        scene_render.shadows_enabled = app.scene_shadows_enabled;
        scene_render.bloom_enabled = app.scene_bloom_enabled;
        scene_render.bloom_intensity = app.scene_bloom_intensity;
        scene_render.tonemapping = app.scene_tonemapping;
        scene_render.ssao_enabled = app.scene_ssao_enabled;
        scene_render.taa_enabled = app.scene_taa_enabled;
        scene_render.fog_enabled = app.scene_fog_enabled;
        scene_render.fog_color = app.scene_fog_color;
        scene_render.fog_start = app.scene_fog_start;
        scene_render.fog_end = app.scene_fog_end;
        scene_render.dof_enabled = app.scene_dof_enabled;
        scene_render.dof_focus_distance = app.scene_dof_focus_distance;
        scene_render.dof_aperture = app.scene_dof_aperture;

        if let Some(handle) = scene_render.render_target.clone() {
            let tex_id = egui_ctx.add_image(handle);
            scene_render.egui_texture_id = Some(tex_id);
            app.scene_view_texture_id = Some(tex_id);
        }
    }

    // === Material Preview render-target plumbing (Phase 8) ===
    // Push PBR values from the inspector to the Bevy resource, then
    // re-register the render target texture with egui.
    {
        if app.material_preview_dirty {
            mat_preview.current_color = app.material_preview_color;
            mat_preview.current_metallic = app.material_preview_metallic;
            mat_preview.current_roughness = app.material_preview_roughness;
            mat_preview.current_emissive = app.material_preview_emissive;
            mat_preview.dirty = true;
            app.material_preview_dirty = false;
        }

        if let Some(handle) = mat_preview.render_target.clone() {
            let tex_id = egui_ctx.add_image(handle);
            mat_preview.egui_texture_id = Some(tex_id);
            app.material_preview_texture_id = Some(tex_id);
        }
    }
}

/// Bevy system for demo capture — uses Screenshot API to read GPU framebuffer.
/// Cycles through all features, taking per-feature screenshots while recording video.
pub fn demo_capture_system(
    mut app: bevy::ecs::system::NonSendMut<BerryCodeApp>,
    mut commands: bevy::ecs::system::Commands,
) {
    use bevy::render::view::screenshot::{save_to_disk, Screenshot};
    use demo_capture::{DemoAction, SetupAction};

    if !app.demo_capture.active {
        return;
    }

    let action = app.demo_capture.tick();

    match action {
        DemoAction::None => {}
        DemoAction::CaptureVideo => {
            // Capture a frame for video only
            let encoder = app.demo_capture.encoder.clone();
            commands.spawn(Screenshot::primary_window()).observe(
                move |trigger: bevy::prelude::Trigger<
                    bevy::render::view::screenshot::ScreenshotCaptured,
                >| {
                    let img = trigger.event();
                    let w = img.width();
                    let h = img.height();
                    if let Ok(mut enc) = encoder.lock() {
                        enc.feed(&img.data, w, h);
                    }
                },
            );
        }
        DemoAction::SetupUi { panel, setup } => {
            // Switch sidebar panel if specified
            if let Some(p) = panel {
                app.active_panel = p;
            }

            // Apply extra UI setup
            match setup {
                SetupAction::None => {}
                SetupAction::OpenDebugger => {
                    app.debug_state.active = true;
                    // Close other panels that might overlap
                    app.run_panel_open = false;
                    app.tool_panel_open = false;
                }
                SetupAction::OpenRunPanel => {
                    app.run_panel_open = true;
                    app.debug_state.active = false;
                    app.tool_panel_open = false;
                }
                SetupAction::OpenToolPanel => {
                    app.tool_panel_open = true;
                    app.debug_state.active = false;
                    app.run_panel_open = false;
                }
            }

            // Also capture a video frame during setup
            let encoder = app.demo_capture.encoder.clone();
            commands.spawn(Screenshot::primary_window()).observe(
                move |trigger: bevy::prelude::Trigger<
                    bevy::render::view::screenshot::ScreenshotCaptured,
                >| {
                    let img = trigger.event();
                    let w = img.width();
                    let h = img.height();
                    if let Ok(mut enc) = encoder.lock() {
                        enc.feed(&img.data, w, h);
                    }
                },
            );
        }
        DemoAction::CaptureScreenshotAndVideo(name) => {
            // Capture for both screenshot and video
            let encoder = app.demo_capture.encoder.clone();
            let output_dir = std::path::PathBuf::from("docs/demo");
            let name_clone = name.clone();

            // Save screenshot to disk
            let path = output_dir.join(&name);
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(path));

            // Also feed to video encoder
            commands.spawn(Screenshot::primary_window()).observe(
                move |trigger: bevy::prelude::Trigger<
                    bevy::render::view::screenshot::ScreenshotCaptured,
                >| {
                    let img = trigger.event();
                    let w = img.width();
                    let h = img.height();
                    if let Ok(mut enc) = encoder.lock() {
                        enc.feed(&img.data, w, h);
                    }
                    tracing::info!("📸 Saved: docs/demo/{}", name_clone);
                },
            );

            app.demo_capture.mark_screenshot(name);
        }
        DemoAction::Finish => {
            app.demo_capture.finalize();
            std::process::exit(0);
        }
    }
}
