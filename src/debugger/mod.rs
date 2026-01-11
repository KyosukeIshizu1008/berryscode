//! Debugger Module
//!
//! IntelliJ-style debugger UI components with DAP integration.

pub mod session;
pub mod breakpoint_gutter;
pub mod debug_toolbar;
pub mod variables_panel;
pub mod call_stack_panel;
pub mod watch_panel;
pub mod debug_console;

pub use session::DebugSession;
pub use breakpoint_gutter::BreakpointGutter;
pub use debug_toolbar::DebugToolbar;
pub use variables_panel::VariablesPanel;
pub use call_stack_panel::CallStackPanel;
pub use watch_panel::WatchPanel;
pub use debug_console::DebugConsole;
