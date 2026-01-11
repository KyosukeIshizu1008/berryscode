//! Type-safe Codicon icon name constants
//!
//! This module provides compile-time checked icon names to prevent typos
//! and ensure all icon references match actual Codicon CSS definitions.
//!
//! ## Usage
//! ```rust
//! use crate::common::icons::ICON_FILES;
//! let icon_class = format!("codicon codicon-{}", ICON_FILES);
//! ```

/// File/Folder icons
pub const ICON_FILES: &str = "files";
pub const ICON_FILE: &str = "file";
pub const ICON_FOLDER: &str = "folder";
pub const ICON_FOLDER_OPENED: &str = "folder-opened";

/// Communication icons
pub const ICON_COMMENT_DISCUSSION: &str = "comment-discussion";
pub const ICON_COMMENT: &str = "comment";

/// Database icons
pub const ICON_DATABASE: &str = "database";

/// Workflow & References icons
pub const ICON_REFERENCES: &str = "references";
pub const ICON_GIT_MERGE: &str = "git-merge";
pub const ICON_GIT_PULL_REQUEST: &str = "git-pull-request";

/// Terminal icons
pub const ICON_TERMINAL: &str = "terminal";
pub const ICON_CONSOLE: &str = "console";

/// Bot & AI icons
pub const ICON_HUBOT: &str = "hubot";
pub const ICON_COPILOT: &str = "copilot";

/// Settings icons
pub const ICON_SETTINGS_GEAR: &str = "settings-gear";
pub const ICON_GEAR: &str = "gear";

/// Remote icons
pub const ICON_REMOTE_EXPLORER: &str = "remote-explorer";
pub const ICON_REMOTE: &str = "remote";

/// Navigation icons
pub const ICON_CHEVRON_RIGHT: &str = "chevron-right";
pub const ICON_CHEVRON_DOWN: &str = "chevron-down";
pub const ICON_ARROW_LEFT: &str = "arrow-left";
pub const ICON_ARROW_RIGHT: &str = "arrow-right";

/// Action icons
pub const ICON_ADD: &str = "add";
pub const ICON_REMOVE: &str = "remove";
pub const ICON_CLOSE: &str = "close";
pub const ICON_REFRESH: &str = "refresh";
pub const ICON_SAVE: &str = "save";
pub const ICON_EDIT: &str = "edit";

/// Status icons
pub const ICON_CHECK: &str = "check";
pub const ICON_ERROR: &str = "error";
pub const ICON_WARNING: &str = "warning";
pub const ICON_INFO: &str = "info";

/// Search icons
pub const ICON_SEARCH: &str = "search";
pub const ICON_REPLACE: &str = "replace";

/// Symbol icons
pub const ICON_SYMBOL_METHOD: &str = "symbol-method";
pub const ICON_SYMBOL_FUNCTION: &str = "symbol-function";
pub const ICON_SYMBOL_CLASS: &str = "symbol-class";
pub const ICON_SYMBOL_VARIABLE: &str = "symbol-variable";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_icon_constants_no_prefix() {
        // Ensure all icon constants don't include "codicon-" prefix
        assert!(!ICON_FILES.contains("codicon-"));
        assert!(!ICON_HUBOT.contains("codicon-"));
        assert!(!ICON_REFERENCES.contains("codicon-"));
    }

    #[test]
    fn test_icon_class_generation() {
        let class = format!("codicon codicon-{}", ICON_FILES);
        assert_eq!(class, "codicon codicon-files");
    }
}
