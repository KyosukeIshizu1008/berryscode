//! CSS Class Name Constants
//!
//! This module provides type-safe constants for all CSS class names used in BerryEditor.
//! Using constants prevents typos and enables IDE autocomplete.
//!
//! ## Design Philosophy
//! - All class names follow BEM-like naming: `.berry-{component}-{element}-{modifier}`
//! - Constants are organized by component for easy discovery
//! - Each constant matches exactly one CSS class in the stylesheets

// ============================================================================
// Layout Components
// ============================================================================

/// Main application container
pub const APP: &str = "berry-editor-app";

/// Activity bar (left sidebar with icons)
pub const ACTIVITY_BAR: &str = "berry-editor-activity-bar";
pub const ACTIVITY_ICON: &str = "berry-editor-activity-icon";
pub const ACTIVITY_ICON_ACTIVE: &str = "active";

/// Sidebar (file tree, git panel, etc.)
pub const SIDEBAR: &str = "berry-editor-sidebar";
pub const SIDEBAR_PANEL_HEADER: &str = "berry-sidebar-panel-header";
pub const SIDEBAR_RESIZER: &str = "berry-editor-sidebar-resizer";
pub const SIDEBAR_RESIZER_ACTIVE: &str = "active";
pub const SIDEBAR_RESIZER_HOVER: &str = "hover";

/// Main editor area
pub const MAIN_AREA: &str = "berry-editor-main-area";
pub const EDITOR_PANE: &str = "berry-editor-pane";

/// Status bar
pub const STATUS_BAR: &str = "berry-editor-status-bar";
pub const STATUS_LEFT: &str = "berry-editor-status-left";
pub const STATUS_RIGHT: &str = "berry-editor-status-right";

// ============================================================================
// File Tree
// ============================================================================

pub const FILE_TREE: &str = "berry-editor-file-tree";
pub const FILE_ITEM: &str = "berry-editor-file-item";
pub const FILE_ITEM_SELECTED: &str = "selected";
pub const FILE_ITEM_ACTIVE: &str = "active";
pub const FOLDER_ICON: &str = "berry-editor-folder-icon";
pub const FOLDER_EXPANDED: &str = "expanded";
pub const PROJECT_ROOT: &str = "berry-project-root";
pub const PROJECT_NAME: &str = "berry-project-name";
pub const PROJECT_ACTIONS: &str = "berry-project-actions";
pub const PROJECT_REMOVE_BTN: &str = "berry-project-remove-btn";

// ============================================================================
// Editor Tabs
// ============================================================================

pub const TAB_BAR: &str = "berry-editor-tab-bar";
pub const TAB: &str = "berry-editor-tab";
pub const TAB_ACTIVE: &str = "active";
pub const TAB_CLOSE: &str = "berry-editor-tab-close";

// ============================================================================
// Command Palette
// ============================================================================

pub const COMMAND_PALETTE: &str = "berry-command-palette";
pub const COMMAND_PALETTE_OVERLAY: &str = "berry-command-palette-overlay";
pub const COMMAND_PALETTE_INPUT: &str = "berry-command-palette-input";
pub const PALETTE_ITEM: &str = "berry-palette-item";
pub const PALETTE_ITEM_SELECTED: &str = "berry-palette-item-selected";
pub const PALETTE_ITEM_ICON: &str = "berry-palette-item-icon";
pub const PALETTE_ITEM_TEXT: &str = "berry-palette-item-text";
pub const PALETTE_ITEM_SHORTCUT: &str = "berry-palette-item-shortcut";

// ============================================================================
// Completion Widget (LSP)
// ============================================================================

pub const COMPLETION_WIDGET: &str = "berry-completion-widget";
pub const COMPLETION_ITEM: &str = "berry-completion-item";
pub const COMPLETION_ITEM_SELECTED: &str = "berry-completion-item-selected";
pub const COMPLETION_ITEM_ICON: &str = "berry-completion-item-icon";
pub const COMPLETION_ITEM_TEXT: &str = "berry-completion-item-text";
pub const COMPLETION_ITEM_TYPE: &str = "berry-completion-item-type";

// ============================================================================
// Diagnostics Panel
// ============================================================================

pub const DIAGNOSTICS_PANEL: &str = "berry-diagnostics-panel";
pub const DIAGNOSTIC_ERROR: &str = "berry-diagnostic-error";
pub const DIAGNOSTIC_WARNING: &str = "berry-diagnostic-warning";
pub const DIAGNOSTIC_INFO: &str = "berry-diagnostic-info";
pub const DIAGNOSTIC_HINT: &str = "berry-diagnostic-hint";

// ============================================================================
// Git UI
// ============================================================================

pub const GIT_PANEL: &str = "berry-git-panel";
pub const GIT_HEADER: &str = "berry-git-header";
pub const GIT_BRANCH_SELECT: &str = "berry-git-branch-select";
pub const GIT_REFRESH_BTN: &str = "berry-git-refresh-btn";

// Git Changes
pub const GIT_CHANGES: &str = "berry-git-changes";
pub const GIT_STAGED: &str = "berry-git-staged";
pub const GIT_SECTION_TITLE: &str = "berry-git-section-title";
pub const GIT_FILE: &str = "berry-git-file";
pub const GIT_FILE_STATUS: &str = "berry-git-file-status";
pub const GIT_FILE_PATH: &str = "berry-git-file-path";
pub const GIT_STAGE_BTN: &str = "berry-git-stage-btn";
pub const GIT_UNSTAGE_BTN: &str = "berry-git-unstage-btn";

// Git Commit
pub const GIT_COMMIT: &str = "berry-git-commit";
pub const GIT_COMMIT_MESSAGE: &str = "berry-git-commit-message";
pub const GIT_COMMIT_BTN: &str = "berry-git-commit-btn";

// Git Commit History
pub const COMMIT_HISTORY: &str = "berry-commit-history";
pub const COMMIT_ITEM: &str = "berry-commit-item";
pub const COMMIT_ITEM_SELECTED: &str = "berry-commit-item-selected";
pub const COMMIT_HEADER: &str = "berry-commit-header";
pub const COMMIT_HASH: &str = "berry-commit-hash";
pub const COMMIT_TIME: &str = "berry-commit-time";
pub const COMMIT_MESSAGE: &str = "berry-commit-message";
pub const COMMIT_AUTHOR: &str = "berry-commit-author";

// Git Branch Manager
pub const BRANCH_MANAGER: &str = "berry-branch-manager";
pub const BRANCH_HEADER: &str = "berry-branch-header";
pub const BRANCH_CREATE_DIALOG: &str = "berry-branch-create-dialog";
pub const BRANCH_LIST: &str = "berry-branch-list";
pub const BRANCH_ITEM: &str = "berry-branch-item";
pub const BRANCH_NAME: &str = "berry-branch-name";
pub const BRANCH_CHECKOUT_BTN: &str = "berry-branch-checkout-btn";
pub const BRANCH_DELETE_BTN: &str = "berry-branch-delete-btn";

// Git Status Messages
pub const GIT_ERROR: &str = "berry-git-error";
pub const GIT_LOADING: &str = "berry-git-loading";
pub const GIT_EMPTY: &str = "berry-git-empty";

// ============================================================================
// Settings Panel
// ============================================================================

pub const SETTINGS_SIDEBAR: &str = "berry-settings-sidebar";
pub const SETTINGS_CONTENT: &str = "berry-settings-content";
pub const SETTINGS_SECTION: &str = "berry-settings-section";
pub const SETTINGS_SECTION_TITLE: &str = "berry-settings-section-title";
pub const SETTINGS_CONTROLS: &str = "berry-settings-controls";
pub const SETTINGS_CONTROL_ROW: &str = "berry-settings-control-row";
pub const SETTINGS_CONTROL_LABEL: &str = "berry-settings-control-label";
pub const SETTINGS_INPUT: &str = "berry-settings-input";
pub const SETTINGS_SELECT: &str = "berry-settings-select";
pub const SETTINGS_CHECKBOX: &str = "berry-settings-checkbox";

// ============================================================================
// Common UI Elements
// ============================================================================

pub const BUTTON: &str = "berry-button";
pub const BUTTON_PRIMARY: &str = "berry-button-primary";
pub const BUTTON_SECONDARY: &str = "berry-button-secondary";
pub const ICON_BUTTON: &str = "berry-icon-button";

// ============================================================================
// Syntax Highlighting
// ============================================================================

pub const SYNTAX_KEYWORD: &str = "syntax-keyword";
pub const SYNTAX_FUNCTION: &str = "syntax-function";
pub const SYNTAX_TYPE: &str = "syntax-type";
pub const SYNTAX_STRING: &str = "syntax-string";
pub const SYNTAX_NUMBER: &str = "syntax-number";
pub const SYNTAX_COMMENT: &str = "syntax-comment";
pub const SYNTAX_OPERATOR: &str = "syntax-operator";
pub const SYNTAX_IDENTIFIER: &str = "syntax-identifier";
pub const SYNTAX_ATTRIBUTE: &str = "syntax-attribute";
pub const SYNTAX_CONSTANT: &str = "syntax-constant";

// ============================================================================
// Search Dialog
// ============================================================================

pub const SEARCH_DIALOG: &str = "berry-search-dialog";
pub const SEARCH_DIALOG_OVERLAY: &str = "berry-search-dialog-overlay";
pub const SEARCH_INPUT: &str = "berry-search-input";
pub const SEARCH_RESULTS: &str = "berry-search-results";
pub const SEARCH_RESULT_ITEM: &str = "berry-search-result-item";
pub const SEARCH_RESULT_SELECTED: &str = "berry-search-result-selected";
pub const SEARCH_RESULT_PATH: &str = "berry-search-result-path";
pub const SEARCH_RESULT_TEXT: &str = "berry-search-result-text";

// ============================================================================
// Helper Functions
// ============================================================================

/// Combine class names with a space separator
///
/// # Example
/// ```
/// use berry_editor::css_classes::{combine, FILE_ITEM, FILE_ITEM_SELECTED};
///
/// let classes = combine(&[FILE_ITEM, FILE_ITEM_SELECTED]);
/// assert_eq!(classes, "berry-editor-file-item selected");
/// ```
pub fn combine(classes: &[&str]) -> String {
    classes.join(" ")
}

/// Conditionally add a class name
///
/// # Example
/// ```
/// use berry_editor::css_classes::{conditional, FILE_ITEM_SELECTED};
///
/// let is_selected = true;
/// let class = conditional(FILE_ITEM_SELECTED, is_selected);
/// assert_eq!(class, Some("selected"));
/// ```
pub fn conditional(class: &str, condition: bool) -> Option<&str> {
    if condition {
        Some(class)
    } else {
        None
    }
}
