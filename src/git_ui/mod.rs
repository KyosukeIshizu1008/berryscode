//! Git UI Components
//!
//! IntelliJ-style Git integration UI

pub mod source_control_panel;
pub mod diff_view;
pub mod blame_view;
pub mod commit_history;
pub mod branch_manager;

pub use source_control_panel::SourceControlPanel;
pub use diff_view::DiffView;
pub use blame_view::BlameView;
pub use commit_history::CommitHistoryPanel;
pub use branch_manager::BranchManagerPanel;
