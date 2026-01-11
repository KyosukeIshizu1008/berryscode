//! TUI (Terminal User Interface) - VS Code-like 3-pane layout
//!
//! Layout:
//! ┌────────────┬───────────────────┬──────────────┐
//! │ File Tree  │  Editor View      │ AI Chat      │
//! │            │  (Syntax HL)      │ & Logs       │
//! │  [src/]    │                   │              │
//! │  ├─main.rs │  fn main() {      │ > AI: ...    │
//! │  ├─lib.rs  │    println!(...); │ > User: ...  │
//! │  └─tui.rs  │  }                │              │
//! └────────────┴───────────────────┴──────────────┘

use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use anyhow::Result;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
// LSP support temporarily disabled in TUI
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

/// App state for the TUI
pub struct TuiApp {
    /// Current working directory (project root)
    pub project_root: PathBuf,

    /// File tree state
    pub file_tree: FileTree,

    /// Editor state
    pub editor: EditorView,

    /// Chat/log panel
    pub chat: ChatPanel,

    /// Active pane (0: file tree, 1: editor, 2: chat)
    pub active_pane: usize,

    /// Should quit?
    pub should_quit: bool,

    /// Syntax highlighting
    syntax_set: Arc<SyntaxSet>,
    theme_set: Arc<ThemeSet>,
}

impl TuiApp {
    /// Create new TUI app
    pub fn new(project_root: PathBuf) -> Result<Self> {
        let file_tree = FileTree::new(&project_root)?;

        // Load syntax highlighting assets
        let syntax_set = Arc::new(SyntaxSet::load_defaults_newlines());
        let theme_set = Arc::new(ThemeSet::load_defaults());

        Ok(Self {
            project_root: project_root.clone(),
            file_tree,
            editor: EditorView::new(),
            chat: ChatPanel::new(),
            active_pane: 0,
            should_quit: false,
            syntax_set,
            theme_set,
        })
    }

    /// Handle keyboard input
    pub fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            // Global shortcuts
            KeyCode::Char('q') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            KeyCode::Tab => {
                // Cycle through panes
                self.active_pane = (self.active_pane + 1) % 3;
            }

            // Pane-specific shortcuts
            _ => {
                match self.active_pane {
                    0 => {
                        // Handle file tree - check for Enter key to open file
                        if key.code == KeyCode::Enter {
                            if let Some(selected_path) = self.file_tree.get_selected_file_path() {
                                if selected_path.is_file() {
                                    // Load file into editor
                                    let _ = self.editor.load_file(&selected_path);
                                    // Switch to editor pane
                                    self.active_pane = 1;
                                }
                            }
                        }
                        self.file_tree.handle_key(key)?;
                    }
                    1 => self.editor.handle_key(key)?,
                    2 => self.chat.handle_key(key)?,
                    _ => {}
                }
            }
        }

        Ok(())
    }

    /// Render the UI
    pub fn render(&mut self, frame: &mut Frame) {
        let size = frame.area();

        // Split screen into 3 columns: File Tree | Editor | Chat
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(20),  // File tree: 20%
                Constraint::Percentage(50),  // Editor: 50%
                Constraint::Percentage(30),  // Chat: 30%
            ])
            .split(size);

        // Render each pane
        self.file_tree.render(frame, chunks[0], self.active_pane == 0);
        self.editor.render(
            frame,
            chunks[1],
            self.active_pane == 1,
            &self.syntax_set,
            &self.theme_set,
        );
        self.chat.render(frame, chunks[2], self.active_pane == 2);
    }
}

/// File tree pane
pub struct FileTree {
    items: Vec<FileTreeItem>,
    selected_index: usize,
    #[allow(dead_code)]
    root: PathBuf,
}

#[derive(Clone)]
struct FileTreeItem {
    #[allow(dead_code)]
    path: PathBuf,
    name: String,
    is_dir: bool,
    depth: usize,
    expanded: bool,
}

impl FileTree {
    fn new(root: &PathBuf) -> Result<Self> {
        let items = Self::scan_directory(root, 0)?;

        Ok(Self {
            items,
            selected_index: 0,
            root: root.clone(),
        })
    }

    fn scan_directory(path: &PathBuf, depth: usize) -> Result<Vec<FileTreeItem>> {
        use std::fs;

        let mut items = Vec::new();

        if let Ok(entries) = fs::read_dir(path) {
            let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
            entries.sort_by_key(|e| e.path());

            for entry in entries {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();

                // Skip hidden files and common ignore patterns
                if name.starts_with('.') || name == "target" || name == "node_modules" {
                    continue;
                }

                let is_dir = path.is_dir();

                items.push(FileTreeItem {
                    path: path.clone(),
                    name,
                    is_dir,
                    depth,
                    expanded: false,
                });
            }
        }

        Ok(items)
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected_index < self.items.len().saturating_sub(1) {
                    self.selected_index += 1;
                }
            }
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                self.toggle_directory()?;
            }
            KeyCode::Left | KeyCode::Char('h') => {
                // Collapse directory
                if let Some(item) = self.items.get(self.selected_index) {
                    if item.is_dir && item.expanded {
                        self.toggle_directory()?;
                    } else if item.depth > 0 {
                        // Jump to parent directory
                        let parent_depth = item.depth - 1;
                        for i in (0..self.selected_index).rev() {
                            if self.items[i].depth == parent_depth && self.items[i].is_dir {
                                self.selected_index = i;
                                break;
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Get the path of the currently selected file/directory
    fn get_selected_file_path(&self) -> Option<PathBuf> {
        self.items.get(self.selected_index).map(|item| item.path.clone())
    }

    fn toggle_directory(&mut self) -> Result<()> {
        if self.items.is_empty() || self.selected_index >= self.items.len() {
            return Ok(());
        }

        let current_item = &self.items[self.selected_index];

        if !current_item.is_dir {
            return Ok(()); // Not a directory
        }

        let is_expanded = current_item.expanded;
        let dir_path = current_item.path.clone();
        let dir_depth = current_item.depth;

        if is_expanded {
            // Collapse: Remove all children
            self.items[self.selected_index].expanded = false;

            // Remove all items with greater depth that follow
            let mut remove_count = 0;
            for i in (self.selected_index + 1)..self.items.len() {
                if self.items[i].depth > dir_depth {
                    remove_count += 1;
                } else {
                    break;
                }
            }

            for _ in 0..remove_count {
                self.items.remove(self.selected_index + 1);
            }
        } else {
            // Expand: Add children
            self.items[self.selected_index].expanded = true;

            let children = Self::scan_directory(&dir_path, dir_depth + 1)?;

            // Insert children after current item
            for (i, child) in children.into_iter().enumerate() {
                self.items.insert(self.selected_index + 1 + i, child);
            }
        }

        Ok(())
    }

    fn render(&self, frame: &mut Frame, area: Rect, is_active: bool) {
        let items: Vec<ListItem> = self
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let indent = "  ".repeat(item.depth);

                // Icon and expansion indicator
                let (icon, expand_indicator) = if item.is_dir {
                    let indicator = if item.expanded { "▼" } else { "▶" };
                    ("📁", indicator)
                } else {
                    ("📄", " ")
                };

                let name = format!("{}{} {} {}", indent, expand_indicator, icon, item.name);

                let style = if i == self.selected_index {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                ListItem::new(name).style(style)
            })
            .collect();

        let border_style = if is_active {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        };

        let list = List::new(items)
            .block(
                Block::default()
                    .title("📂 Files")
                    .borders(Borders::ALL)
                    .border_style(border_style),
            );

        frame.render_widget(list, area);
    }
}

/// Editor mode
#[derive(Clone, Copy, PartialEq)]
enum EditorMode {
    Normal,  // Vim-like normal mode (navigation)
    Insert,  // Insert mode (typing)
    Command, // Command mode (:w, :q, etc)
}

/// Editor view pane
pub struct EditorView {
    content: Vec<String>,
    cursor_line: usize,
    cursor_col: usize,
    scroll_offset: usize,
    file_path: Option<PathBuf>,
    file_type: String,
    mode: EditorMode,
    command_buffer: String,
    undo_stack: Vec<Vec<String>>,
    redo_stack: Vec<Vec<String>>,
    modified: bool,
}

impl EditorView {
    fn new() -> Self {
        Self {
            content: vec![
                "// Welcome to BerryCode TUI Editor!".to_string(),
                "// Press 'i' to enter Insert mode".to_string(),
                "// Press ESC to return to Normal mode".to_string(),
                "// Press ':w' to save, 'Ctrl+Space' for completions".to_string(),
                "".to_string(),
                "fn main() {".to_string(),
                "    println!(\"Hello, world!\");".to_string(),
                "}".to_string(),
            ],
            cursor_line: 0,
            cursor_col: 0,
            scroll_offset: 0,
            file_path: None,
            file_type: "rust".to_string(),
            mode: EditorMode::Normal,
            command_buffer: String::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            modified: false,
        }
    }

    fn load_file(&mut self, path: &PathBuf) -> Result<()> {
        use std::fs;

        let content = fs::read_to_string(path)?;
        self.content = content.lines().map(|s| s.to_string()).collect();

        if self.content.is_empty() {
            self.content.push(String::new());
        }

        self.file_path = Some(path.clone());

        // Detect file type from extension
        if let Some(ext) = path.extension() {
            self.file_type = ext.to_string_lossy().to_string();
        }

        self.cursor_line = 0;
        self.cursor_col = 0;
        self.scroll_offset = 0;
        self.modified = false;
        self.undo_stack.clear();
        self.redo_stack.clear();

        Ok(())
    }

    fn save_file(&mut self) -> Result<String> {
        use std::fs;

        if let Some(ref path) = self.file_path {
            let content = self.content.join("\n");
            fs::write(path, content)?;
            self.modified = false;
            Ok(format!("Saved to {:?}", path))
        } else {
            Ok("No file path set".to_string())
        }
    }

    fn push_undo(&mut self) {
        self.undo_stack.push(self.content.clone());
        if self.undo_stack.len() > 100 {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    fn undo(&mut self) {
        if let Some(previous) = self.undo_stack.pop() {
            self.redo_stack.push(self.content.clone());
            self.content = previous;
            self.clamp_cursor();
        }
    }

    fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(self.content.clone());
            self.content = next;
            self.clamp_cursor();
        }
    }

    fn clamp_cursor(&mut self) {
        if self.content.is_empty() {
            self.content.push(String::new());
        }

        if self.cursor_line >= self.content.len() {
            self.cursor_line = self.content.len() - 1;
        }

        let line_len = self.content[self.cursor_line].len();
        if self.cursor_col > line_len {
            self.cursor_col = line_len;
        }
    }

    fn insert_char(&mut self, c: char) {
        self.push_undo();

        if self.cursor_line >= self.content.len() {
            self.content.push(String::new());
        }

        let line = &mut self.content[self.cursor_line];
        line.insert(self.cursor_col, c);
        self.cursor_col += 1;
        self.modified = true;
    }

    fn insert_newline(&mut self) {
        self.push_undo();

        let current_line = self.content[self.cursor_line].clone();
        let (before, after) = current_line.split_at(self.cursor_col);

        self.content[self.cursor_line] = before.to_string();
        self.content.insert(self.cursor_line + 1, after.to_string());

        self.cursor_line += 1;
        self.cursor_col = 0;
        self.modified = true;
    }

    fn backspace(&mut self) {
        if self.cursor_col > 0 {
            self.push_undo();
            self.content[self.cursor_line].remove(self.cursor_col - 1);
            self.cursor_col -= 1;
            self.modified = true;
        } else if self.cursor_line > 0 {
            self.push_undo();
            let current_line = self.content.remove(self.cursor_line);
            self.cursor_line -= 1;
            self.cursor_col = self.content[self.cursor_line].len();
            self.content[self.cursor_line].push_str(&current_line);
            self.modified = true;
        }
    }

    fn delete_char(&mut self) {
        if self.cursor_col < self.content[self.cursor_line].len() {
            self.push_undo();
            self.content[self.cursor_line].remove(self.cursor_col);
            self.modified = true;
        } else if self.cursor_line < self.content.len() - 1 {
            self.push_undo();
            let next_line = self.content.remove(self.cursor_line + 1);
            self.content[self.cursor_line].push_str(&next_line);
            self.modified = true;
        }
    }

    fn execute_command(&mut self, command: &str) -> Result<String> {
        match command {
            "w" | "write" => self.save_file(),
            "q" | "quit" => Ok("Use Ctrl+Q to quit".to_string()),
            "wq" => {
                self.save_file()?;
                Ok("Saved. Use Ctrl+Q to quit".to_string())
            }
            _ => Ok(format!("Unknown command: {}", command)),
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        match self.mode {
            EditorMode::Normal => self.handle_normal_mode(key),
            EditorMode::Insert => self.handle_insert_mode(key),
            EditorMode::Command => self.handle_command_mode(key),
        }
    }

    fn handle_normal_mode(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            // Mode switching
            KeyCode::Char('i') => {
                self.mode = EditorMode::Insert;
            }
            KeyCode::Char('I') => {
                self.cursor_col = 0;
                self.mode = EditorMode::Insert;
            }
            KeyCode::Char('a') => {
                if self.cursor_col < self.content[self.cursor_line].len() {
                    self.cursor_col += 1;
                }
                self.mode = EditorMode::Insert;
            }
            KeyCode::Char('A') => {
                self.cursor_col = self.content[self.cursor_line].len();
                self.mode = EditorMode::Insert;
            }
            KeyCode::Char('o') => {
                self.push_undo();
                self.content.insert(self.cursor_line + 1, String::new());
                self.cursor_line += 1;
                self.cursor_col = 0;
                self.mode = EditorMode::Insert;
            }
            KeyCode::Char('O') => {
                self.push_undo();
                self.content.insert(self.cursor_line, String::new());
                self.cursor_col = 0;
                self.mode = EditorMode::Insert;
            }
            KeyCode::Char(':') => {
                self.mode = EditorMode::Command;
                self.command_buffer.clear();
            }

            // Navigation
            KeyCode::Char('h') | KeyCode::Left => {
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                }
            }
            KeyCode::Char('l') | KeyCode::Right => {
                if self.cursor_col < self.content[self.cursor_line].len() {
                    self.cursor_col += 1;
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if self.cursor_line < self.content.len() - 1 {
                    self.cursor_line += 1;
                    self.clamp_cursor();
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.cursor_line > 0 {
                    self.cursor_line -= 1;
                    self.clamp_cursor();
                }
            }
            KeyCode::Char('0') => {
                self.cursor_col = 0;
            }
            KeyCode::Char('$') => {
                self.cursor_col = self.content[self.cursor_line].len();
            }
            KeyCode::Char('g') => {
                self.cursor_line = 0;
                self.cursor_col = 0;
            }
            KeyCode::Char('G') => {
                self.cursor_line = self.content.len() - 1;
                self.cursor_col = 0;
            }

            // Editing
            KeyCode::Char('x') => {
                self.delete_char();
            }
            KeyCode::Char('d') => {
                // Simple delete line (dd in Vim requires two 'd' presses)
                if self.content.len() > 1 {
                    self.push_undo();
                    self.content.remove(self.cursor_line);
                    self.clamp_cursor();
                }
            }
            KeyCode::Char('u') => {
                self.undo();
            }
            KeyCode::Char('r') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                self.redo();
            }

            _ => {}
        }

        Ok(())
    }

    fn handle_insert_mode(&mut self, key: KeyEvent) -> Result<()> {
        // LSP completion temporarily disabled

        // Normal insert mode handling
        match key.code {
            KeyCode::Esc => {
                self.mode = EditorMode::Normal;
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                }
            }
            KeyCode::Char(c) => {
                self.insert_char(c);
            }
            KeyCode::Enter => {
                self.insert_newline();
            }
            KeyCode::Backspace => {
                self.backspace();
            }
            KeyCode::Delete => {
                self.delete_char();
            }
            KeyCode::Left => {
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                }
            }
            KeyCode::Right => {
                if self.cursor_col < self.content[self.cursor_line].len() {
                    self.cursor_col += 1;
                }
            }
            KeyCode::Up => {
                if self.cursor_line > 0 {
                    self.cursor_line -= 1;
                    self.clamp_cursor();
                }
            }
            KeyCode::Down => {
                if self.cursor_line < self.content.len() - 1 {
                    self.cursor_line += 1;
                    self.clamp_cursor();
                }
            }
            _ => {}
        }

        Ok(())
    }

    #[allow(dead_code)]
    fn trigger_completion(&mut self) {
        // LSP completion temporarily disabled
    }

    fn handle_command_mode(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.mode = EditorMode::Normal;
                self.command_buffer.clear();
            }
            KeyCode::Char(c) => {
                self.command_buffer.push(c);
            }
            KeyCode::Enter => {
                let result = self.execute_command(&self.command_buffer.clone());
                self.mode = EditorMode::Normal;
                self.command_buffer.clear();

                // TODO: Display command result in status line
                if let Ok(msg) = result {
                    tracing::info!("Command result: {}", msg);
                }
            }
            KeyCode::Backspace => {
                self.command_buffer.pop();
            }
            _ => {}
        }

        Ok(())
    }

    fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        is_active: bool,
        syntax_set: &SyntaxSet,
        theme_set: &ThemeSet,
    ) {
        // Get syntax definition for current file type
        let syntax = syntax_set
            .find_syntax_by_extension(&self.file_type)
            .unwrap_or_else(|| syntax_set.find_syntax_plain_text());

        // Use Monokai theme (popular dark theme)
        let theme = &theme_set.themes["Monokai Extended"];

        // Highlight lines
        let mut highlighter = HighlightLines::new(syntax, theme);

        let lines: Vec<Line> = self
            .content
            .iter()
            .enumerate()
            .map(|(i, line_text)| {
                let line_num = format!("{:4} │ ", i + 1);

                // Highlight this line
                let highlighted = highlighter.highlight_line(line_text, syntax_set).unwrap_or_default();

                let mut spans = vec![
                    Span::styled(line_num, Style::default().fg(Color::DarkGray)),
                ];

                // Convert syntect styles to ratatui styles
                for (style, text) in highlighted {
                    let fg_color = Self::syntect_to_ratatui_color(style.foreground);
                    spans.push(Span::styled(
                        text.to_string(),
                        Style::default().fg(fg_color),
                    ));
                }

                // Add cursor line highlight
                if i == self.cursor_line {
                    Line::from(spans).style(Style::default().bg(Color::DarkGray))
                } else {
                    Line::from(spans)
                }
            })
            .collect();

        let border_style = if is_active {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        };

        let mode_str = match self.mode {
            EditorMode::Normal => "NORMAL",
            EditorMode::Insert => "INSERT",
            EditorMode::Command => &format!(":{}", self.command_buffer),
        };

        let modified_indicator = if self.modified { " [+]" } else { "" };

        let title = if let Some(ref path) = self.file_path {
            format!(
                "📝 {} | {} | {}:{}{}",
                path.file_name().unwrap_or_default().to_string_lossy(),
                mode_str,
                self.cursor_line + 1,
                self.cursor_col + 1,
                modified_indicator
            )
        } else {
            format!("📝 Editor | {} | {}:{}{}",
                mode_str,
                self.cursor_line + 1,
                self.cursor_col + 1,
                modified_indicator
            )
        };

        let paragraph = Paragraph::new(lines)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, area);
    }

    /// Convert syntect color to ratatui color
    fn syntect_to_ratatui_color(syntect_color: syntect::highlighting::Color) -> Color {
        Color::Rgb(syntect_color.r, syntect_color.g, syntect_color.b)
    }
}

/// Chat message
#[derive(Clone)]
struct ChatMessage {
    role: String,      // "user" or "assistant"
    content: String,
}

/// Chat/log panel
pub struct ChatPanel {
    messages: Vec<ChatMessage>,
    scroll_offset: usize,
    input_buffer: String,
    is_inputting: bool,
    is_processing: bool,
}

impl ChatPanel {
    fn new() -> Self {
        let welcome_messages = vec![
            ChatMessage {
                role: "assistant".to_string(),
                content: "🤖 BerryCode AI Assistant\n\nWelcome! I can help you with:\n- Code generation\n- Bug fixes\n- Explanations\n- Refactoring\n\nPress 'i' to start typing, ESC to cancel, Enter to send.".to_string(),
            },
        ];

        Self {
            messages: welcome_messages,
            scroll_offset: 0,
            input_buffer: String::new(),
            is_inputting: false,
            is_processing: false,
        }
    }

    fn send_message(&mut self, content: String) {
        // Add user message
        self.messages.push(ChatMessage {
            role: "user".to_string(),
            content: content.clone(),
        });

        self.is_processing = true;

        // TODO: Call LLM API asynchronously
        // For now, add a placeholder response
        self.messages.push(ChatMessage {
            role: "assistant".to_string(),
            content: format!("I received your message: \"{}\"\n\n[AI integration in progress...]", content),
        });

        self.is_processing = false;
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        if self.is_inputting {
            match key.code {
                KeyCode::Esc => {
                    self.is_inputting = false;
                    self.input_buffer.clear();
                }
                KeyCode::Enter => {
                    if !self.input_buffer.trim().is_empty() {
                        self.send_message(self.input_buffer.clone());
                        self.input_buffer.clear();
                    }
                    self.is_inputting = false;
                }
                KeyCode::Char(c) => {
                    self.input_buffer.push(c);
                }
                KeyCode::Backspace => {
                    self.input_buffer.pop();
                }
                _ => {}
            }
        } else {
            match key.code {
                KeyCode::Char('i') => {
                    self.is_inputting = true;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.scroll_offset > 0 {
                        self.scroll_offset -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.scroll_offset += 1;
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn render(&self, frame: &mut Frame, area: Rect, is_active: bool) {
        // Create message lines with role indicators
        let mut lines: Vec<Line> = Vec::new();

        for msg in &self.messages {
            let role_prefix = match msg.role.as_str() {
                "user" => "👤 You: ",
                "assistant" => "🤖 AI: ",
                _ => "",
            };

            let role_color = match msg.role.as_str() {
                "user" => Color::Green,
                "assistant" => Color::Cyan,
                _ => Color::White,
            };

            // Split content by newlines
            for (i, content_line) in msg.content.lines().enumerate() {
                if i == 0 {
                    lines.push(Line::from(vec![
                        Span::styled(role_prefix, Style::default().fg(role_color).add_modifier(Modifier::BOLD)),
                        Span::raw(content_line),
                    ]));
                } else {
                    lines.push(Line::from(Span::raw(format!("    {}", content_line))));
                }
            }

            lines.push(Line::from(""));  // Empty line between messages
        }

        // Add input line if inputting
        if self.is_inputting {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("👤 You: ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::raw(&self.input_buffer),
                Span::styled("█", Style::default().fg(Color::Yellow)), // Cursor
            ]));
        }

        let border_style = if is_active {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        };

        let status = if self.is_inputting {
            " [INPUT MODE - ESC to cancel, Enter to send]"
        } else if self.is_processing {
            " [Processing...]"
        } else {
            " [Press 'i' to chat]"
        };

        let title = format!("💬 AI Chat{}", status);

        let paragraph = Paragraph::new(lines)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .wrap(Wrap { trim: false })
            .scroll((self.scroll_offset as u16, 0));

        frame.render_widget(paragraph, area);
    }
}

/// Run the TUI application
pub fn run_tui(project_root: PathBuf) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = TuiApp::new(project_root)?;

    // Run event loop
    loop {
        terminal.draw(|f| app.render(f))?;

        if let Event::Key(key) = event::read()? {
            app.handle_key(key)?;
        }

        if app.should_quit {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_tui_app_creation() {
        let temp_dir = TempDir::new().unwrap();
        let app = TuiApp::new(temp_dir.path().to_path_buf());
        assert!(app.is_ok());
    }

    #[test]
    fn test_file_tree_creation() {
        let temp_dir = TempDir::new().unwrap();
        let tree = FileTree::new(&temp_dir.path().to_path_buf());
        assert!(tree.is_ok());
    }
}
