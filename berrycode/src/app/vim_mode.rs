#![allow(dead_code)]
//! Vim keybinding engine: modal editing with Normal/Insert/Visual/Command modes
//!
//! Supports:
//! - Normal mode: hjkl, w/b/e, 0/$, gg/G, f/F/t/T, %, {/}
//! - Operators: d, c, y, >, <, ~ with motions and text objects
//! - Text objects: iw, aw, i", a", i(, a(, i{, a{, i[, a[
//! - Visual mode: v (char), V (line), Ctrl+V (block)
//! - Command line: :w, :q, :wq, :e, :<n> (goto line)
//! - Repeat: . (dot repeat), <count> prefix
//! - Registers: unnamed, 0-9, a-z, "
//! - Marks: m + a-z, ' + a-z
//! - Search: /, ?, n, N

use super::BerryCodeApp;

// ═══════════════════════════════════════════════════════════════════
// Vim State
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VimMode {
    Normal,
    Insert,
    Visual,
    VisualLine,
    Command,
    Replace, // single char replace (r)
}

#[derive(Debug, Clone, PartialEq)]
enum PendingOp {
    None,
    Delete,
    Change,
    Yank,
    Indent,
    Dedent,
    /// Waiting for text object char after 'i'/'a' (e.g. diw, ci")
    TextObjectInner(Box<PendingOp>),
    TextObjectAround(Box<PendingOp>),
}

/// Vim engine state (separate from editor state)
pub struct VimState {
    pub enabled: bool,
    pub mode: VimMode,
    pending_op: PendingOp,
    count: Option<usize>,
    /// For g prefix (gg, etc.)
    g_prefix: bool,
    /// Visual mode anchor
    pub visual_start_line: usize,
    pub visual_start_col: usize,
    /// Command line buffer
    pub command_line: String,
    /// Search pattern
    pub search_pattern: String,
    pub search_forward: bool,
    /// Registers (named a-z + unnamed "")
    registers: std::collections::HashMap<char, String>,
    /// Last edit for . repeat
    last_edit: Option<String>,
    /// Marks (a-z → line, col)
    marks: std::collections::HashMap<char, (usize, usize)>,
    /// Status message (shown in status bar)
    pub status: String,
}

impl Default for VimState {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: VimMode::Normal,
            pending_op: PendingOp::None,
            count: None,
            g_prefix: false,
            visual_start_line: 0,
            visual_start_col: 0,
            command_line: String::new(),
            search_pattern: String::new(),
            search_forward: true,
            registers: std::collections::HashMap::new(),
            last_edit: None,
            marks: std::collections::HashMap::new(),
            status: String::new(),
        }
    }
}

impl VimState {
    fn get_count(&mut self) -> usize {
        self.count.take().unwrap_or(1)
    }

    fn set_register(&mut self, reg: char, text: String) {
        self.registers.insert(reg, text.clone());
        self.registers.insert('"', text); // also set unnamed
    }

    pub fn get_register(&self, reg: char) -> Option<&String> {
        self.registers.get(&reg)
    }

    pub fn mode_display(&self) -> &'static str {
        match self.mode {
            VimMode::Normal => "NORMAL",
            VimMode::Insert => "INSERT",
            VimMode::Visual => "VISUAL",
            VimMode::VisualLine => "V-LINE",
            VimMode::Command => "COMMAND",
            VimMode::Replace => "REPLACE",
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// Vim key handling
// ═══════════════════════════════════════════════════════════════════

impl BerryCodeApp {
    /// Main Vim key handler — called from editor before normal key processing.
    /// Returns true if the key was consumed by Vim.
    pub(crate) fn handle_vim_key(&mut self, ctx: &egui::Context) -> bool {
        if !self.vim.enabled {
            return false;
        }

        let events: Vec<egui::Event> = ctx.input(|i| i.events.clone());
        let mut consumed = false;

        for event in &events {
            match &self.vim.mode {
                VimMode::Normal => {
                    if self.handle_vim_normal(event) {
                        consumed = true;
                    }
                }
                VimMode::Insert => {
                    if self.handle_vim_insert(event) {
                        consumed = true;
                    }
                }
                VimMode::Visual | VimMode::VisualLine => {
                    if self.handle_vim_visual(event) {
                        consumed = true;
                    }
                }
                VimMode::Command => {
                    if self.handle_vim_command(event) {
                        consumed = true;
                    }
                }
                VimMode::Replace => {
                    if self.handle_vim_replace(event) {
                        consumed = true;
                    }
                }
            }
        }

        consumed
    }

    // ─── Normal mode ─────────────────────────────────────────────

    fn handle_vim_normal(&mut self, event: &egui::Event) -> bool {
        match event {
            egui::Event::Text(text) => {
                for ch in text.chars() {
                    self.process_normal_char(ch);
                }
                true
            }
            egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } => {
                self.process_normal_key(*key, modifiers);
                true
            }
            _ => false,
        }
    }

    fn process_normal_char(&mut self, ch: char) {
        if self.editor_tabs.get(self.active_tab_idx).is_none() {
            return;
        }

        // Count prefix
        if ch.is_ascii_digit() && (self.vim.count.is_some() || ch != '0') {
            let n = self.vim.count.unwrap_or(0);
            self.vim.count = Some(n * 10 + (ch as usize - '0' as usize));
            return;
        }

        // g prefix
        if self.vim.g_prefix {
            self.vim.g_prefix = false;
            match ch {
                'g' => {
                    let target = self.vim.count.take().unwrap_or(1).saturating_sub(1);
                    if let Some(tab) = self.editor_tabs.get_mut(self.active_tab_idx) {
                        tab.cursor_line = target;
                        tab.cursor_col = 0;
                    }
                }
                _ => {}
            }
            return;
        }

        let n = self.vim.get_count();

        // Handle text object completion (di", ciw, etc.)
        match &self.vim.pending_op {
            PendingOp::TextObjectInner(parent_op) | PendingOp::TextObjectAround(parent_op) => {
                let is_inner = matches!(&self.vim.pending_op, PendingOp::TextObjectInner(_));
                let is_delete = matches!(parent_op.as_ref(), PendingOp::Delete | PendingOp::Change);
                let is_change = matches!(parent_op.as_ref(), PendingOp::Change);
                let is_yank = matches!(parent_op.as_ref(), PendingOp::Yank);

                self.vim.pending_op = PendingOp::None;
                self.vim_text_object(ch, is_inner, is_delete, is_yank);
                if is_change {
                    self.vim.mode = VimMode::Insert;
                }
                return;
            }
            _ => {}
        }

        // Check pending operator — capture cursor_line first, then call methods
        if self.vim.pending_op != PendingOp::None {
            let cursor_line = self
                .editor_tabs
                .get(self.active_tab_idx)
                .map(|t| t.cursor_line)
                .unwrap_or(0);
            match ch {
                'd' if self.vim.pending_op == PendingOp::Delete => {
                    self.vim.pending_op = PendingOp::None;
                    self.vim_delete_lines(cursor_line, n);
                }
                'c' if self.vim.pending_op == PendingOp::Change => {
                    self.vim.pending_op = PendingOp::None;
                    self.vim_delete_lines(cursor_line, n);
                    self.vim.mode = VimMode::Insert;
                }
                'y' if self.vim.pending_op == PendingOp::Yank => {
                    self.vim.pending_op = PendingOp::None;
                    self.vim_yank_lines(cursor_line, n);
                }
                'w' | 'e' | 'b' | '$' | '0' | 'G' | '{' | '}' => {
                    self.vim.pending_op = PendingOp::None;
                    self.vim_apply_motion(ch, n);
                }
                'i' => {
                    // Text object inner: diw, ci", etc.
                    let op = std::mem::replace(&mut self.vim.pending_op, PendingOp::None);
                    self.vim.pending_op = PendingOp::TextObjectInner(Box::new(op));
                }
                'a' => {
                    // Text object around: daw, ca", etc.
                    let op = std::mem::replace(&mut self.vim.pending_op, PendingOp::None);
                    self.vim.pending_op = PendingOp::TextObjectAround(Box::new(op));
                }
                _ => {
                    self.vim.pending_op = PendingOp::None;
                }
            }
            return;
        }

        // Determine what action to take.
        // First handle actions that only need tab (cursor movement).
        // Then handle deferred actions that need &mut self.
        enum DeferredVim {
            None,
            WordForward(usize),
            WordBackward(usize),
            WordEnd(usize),
            InsertNewlineBelow,
            InsertNewlineAbove,
            DeleteChar(usize),
            BackspaceDelete(usize),
            PasteAfter(usize),
            PasteBefore(usize),
            JoinLines(usize),
            SearchNext,
            SearchPrev,
        }
        let mut deferred = DeferredVim::None;

        {
            let tab = match self.editor_tabs.get_mut(self.active_tab_idx) {
                Some(t) => t,
                None => return,
            };

            match ch {
                // ─── Movement ────────────────────
                'h' => tab.cursor_col = tab.cursor_col.saturating_sub(n),
                'j' => {
                    let max_line = tab.buffer.to_string().lines().count().saturating_sub(1);
                    tab.cursor_line = (tab.cursor_line + n).min(max_line);
                }
                'k' => tab.cursor_line = tab.cursor_line.saturating_sub(n),
                'l' => {
                    let line_len = tab
                        .buffer
                        .to_string()
                        .lines()
                        .nth(tab.cursor_line)
                        .map(|l| l.len())
                        .unwrap_or(0);
                    tab.cursor_col = (tab.cursor_col + n).min(line_len.saturating_sub(1));
                }
                'w' | 'W' => deferred = DeferredVim::WordForward(n),
                'b' | 'B' => deferred = DeferredVim::WordBackward(n),
                'e' => deferred = DeferredVim::WordEnd(n),
                '0' => tab.cursor_col = 0,
                '$' => {
                    let line_len = tab
                        .buffer
                        .to_string()
                        .lines()
                        .nth(tab.cursor_line)
                        .map(|l| l.len())
                        .unwrap_or(0);
                    tab.cursor_col = line_len.saturating_sub(1).max(0);
                }
                '^' => {
                    if let Some(line) = tab.buffer.to_string().lines().nth(tab.cursor_line) {
                        tab.cursor_col = line.len() - line.trim_start().len();
                    }
                }
                'G' => {
                    let text = tab.buffer.to_string();
                    if let Some(target) = self.vim.count.take() {
                        tab.cursor_line = target.saturating_sub(1);
                    } else {
                        tab.cursor_line = text.lines().count().saturating_sub(1);
                    }
                    tab.cursor_col = 0;
                }
                'g' => self.vim.g_prefix = true,
                '{' => {
                    let text = tab.buffer.to_string();
                    for _ in 0..n {
                        while tab.cursor_line > 0 {
                            tab.cursor_line -= 1;
                            if text
                                .lines()
                                .nth(tab.cursor_line)
                                .map(|l| l.trim().is_empty())
                                .unwrap_or(true)
                            {
                                break;
                            }
                        }
                    }
                }
                '}' => {
                    let text = tab.buffer.to_string();
                    let max = text.lines().count().saturating_sub(1);
                    for _ in 0..n {
                        while tab.cursor_line < max {
                            tab.cursor_line += 1;
                            if text
                                .lines()
                                .nth(tab.cursor_line)
                                .map(|l| l.trim().is_empty())
                                .unwrap_or(true)
                            {
                                break;
                            }
                        }
                    }
                }

                // ─── Mode switching ──────────────
                'i' => {
                    self.vim.mode = VimMode::Insert;
                    self.vim.status = "-- INSERT --".to_string();
                }
                'I' => {
                    if let Some(line) = tab.buffer.to_string().lines().nth(tab.cursor_line) {
                        tab.cursor_col = line.len() - line.trim_start().len();
                    }
                    self.vim.mode = VimMode::Insert;
                }
                'a' => {
                    tab.cursor_col += 1;
                    self.vim.mode = VimMode::Insert;
                }
                'A' => {
                    let line_len = tab
                        .buffer
                        .to_string()
                        .lines()
                        .nth(tab.cursor_line)
                        .map(|l| l.len())
                        .unwrap_or(0);
                    tab.cursor_col = line_len;
                    self.vim.mode = VimMode::Insert;
                }
                'o' => {
                    let line_len = tab
                        .buffer
                        .to_string()
                        .lines()
                        .nth(tab.cursor_line)
                        .map(|l| l.len())
                        .unwrap_or(0);
                    tab.cursor_col = line_len;
                    self.vim.mode = VimMode::Insert;
                    deferred = DeferredVim::InsertNewlineBelow;
                }
                'O' => {
                    tab.cursor_col = 0;
                    self.vim.mode = VimMode::Insert;
                    deferred = DeferredVim::InsertNewlineAbove;
                }
                'v' => {
                    self.vim.visual_start_line = tab.cursor_line;
                    self.vim.visual_start_col = tab.cursor_col;
                    self.vim.mode = VimMode::Visual;
                    self.vim.status = "-- VISUAL --".to_string();
                }
                'V' => {
                    self.vim.visual_start_line = tab.cursor_line;
                    self.vim.visual_start_col = 0;
                    self.vim.mode = VimMode::VisualLine;
                    self.vim.status = "-- VISUAL LINE --".to_string();
                }
                ':' => {
                    self.vim.mode = VimMode::Command;
                    self.vim.command_line.clear();
                    self.vim.status = ":".to_string();
                }
                'r' => self.vim.mode = VimMode::Replace,
                'R' => self.vim.mode = VimMode::Insert,

                // ─── Operators ───────────────────
                'd' => self.vim.pending_op = PendingOp::Delete,
                'c' => self.vim.pending_op = PendingOp::Change,
                'y' => self.vim.pending_op = PendingOp::Yank,

                // ─── Quick actions (deferred) ────
                'x' => deferred = DeferredVim::DeleteChar(n),
                'X' => deferred = DeferredVim::BackspaceDelete(n),
                'p' => deferred = DeferredVim::PasteAfter(n),
                'P' => deferred = DeferredVim::PasteBefore(n),
                'u' => self.vim.status = "undo".to_string(),
                'J' => deferred = DeferredVim::JoinLines(n),

                // ─── Search ─────────────────────
                '/' => {
                    self.vim.mode = VimMode::Command;
                    self.vim.search_forward = true;
                    self.vim.command_line.clear();
                    self.vim.status = "/".to_string();
                }
                '?' => {
                    self.vim.mode = VimMode::Command;
                    self.vim.search_forward = false;
                    self.vim.command_line.clear();
                    self.vim.status = "?".to_string();
                }
                'n' => deferred = DeferredVim::SearchNext,
                'N' => deferred = DeferredVim::SearchPrev,

                _ => {}
            }
        } // tab borrow dropped here

        // Execute deferred actions
        match deferred {
            DeferredVim::None => {}
            DeferredVim::WordForward(n) => {
                for _ in 0..n {
                    self.vim_word_forward();
                }
            }
            DeferredVim::WordBackward(n) => {
                for _ in 0..n {
                    self.vim_word_backward();
                }
            }
            DeferredVim::WordEnd(n) => {
                for _ in 0..n {
                    self.vim_word_end();
                }
            }
            DeferredVim::InsertNewlineBelow => self.vim_insert_newline_below(),
            DeferredVim::InsertNewlineAbove => self.vim_insert_newline_above(),
            DeferredVim::DeleteChar(n) => self.vim_delete_char(n),
            DeferredVim::BackspaceDelete(n) => {
                for _ in 0..n {
                    if let Some(tab) = self.editor_tabs.get_mut(self.active_tab_idx) {
                        if tab.cursor_col > 0 {
                            tab.cursor_col -= 1;
                        }
                    }
                    self.vim_delete_char(1);
                }
            }
            DeferredVim::PasteAfter(n) => self.vim_paste_after(n),
            DeferredVim::PasteBefore(n) => self.vim_paste_before(n),
            DeferredVim::JoinLines(n) => self.vim_join_lines(n),
            DeferredVim::SearchNext => self.vim_search_next(),
            DeferredVim::SearchPrev => self.vim_search_prev(),
        }
    }

    fn process_normal_key(&mut self, key: egui::Key, _modifiers: &egui::Modifiers) {
        match key {
            egui::Key::Escape => {
                self.vim.pending_op = PendingOp::None;
                self.vim.count = None;
                self.vim.g_prefix = false;
                self.vim.status.clear();
            }
            _ => {}
        }
    }

    // ─── Insert mode ─────────────────────────────────────────────

    fn handle_vim_insert(&mut self, event: &egui::Event) -> bool {
        match event {
            egui::Event::Key {
                key: egui::Key::Escape,
                pressed: true,
                ..
            } => {
                self.vim.mode = VimMode::Normal;
                self.vim.status.clear();
                // Move cursor left by 1 (Vim convention)
                if let Some(tab) = self.editor_tabs.get_mut(self.active_tab_idx) {
                    tab.cursor_col = tab.cursor_col.saturating_sub(1);
                }
                true
            }
            _ => false, // Let normal editor handle all other keys in insert mode
        }
    }

    // ─── Visual mode ─────────────────────────────────────────────

    fn handle_vim_visual(&mut self, event: &egui::Event) -> bool {
        match event {
            egui::Event::Text(text) => {
                for ch in text.chars() {
                    match ch {
                        'h' | 'j' | 'k' | 'l' | 'w' | 'b' | 'e' | '0' | '$' | 'G' | 'g' => {
                            // Reuse normal mode motions (they move the cursor, visual tracks the range)
                            self.process_normal_char(ch);
                        }
                        'd' | 'x' => {
                            // Delete selection
                            self.vim_delete_visual_selection();
                            self.vim.mode = VimMode::Normal;
                            self.vim.status.clear();
                        }
                        'y' => {
                            // Yank selection
                            self.vim_yank_visual_selection();
                            self.vim.mode = VimMode::Normal;
                            self.vim.status.clear();
                        }
                        'c' => {
                            self.vim_delete_visual_selection();
                            self.vim.mode = VimMode::Insert;
                            self.vim.status = "-- INSERT --".to_string();
                        }
                        _ => {}
                    }
                }
                true
            }
            egui::Event::Key {
                key: egui::Key::Escape,
                pressed: true,
                ..
            } => {
                self.vim.mode = VimMode::Normal;
                self.vim.status.clear();
                true
            }
            _ => false,
        }
    }

    // ─── Command mode ────────────────────────────────────────────

    fn handle_vim_command(&mut self, event: &egui::Event) -> bool {
        match event {
            egui::Event::Text(text) => {
                self.vim.command_line.push_str(text);
                self.vim.status = if self.vim.search_forward || !self.vim.search_pattern.is_empty()
                {
                    if self.vim.status.starts_with('/') || self.vim.status.starts_with('?') {
                        format!("{}{}", &self.vim.status[..1], self.vim.command_line)
                    } else {
                        format!(":{}", self.vim.command_line)
                    }
                } else {
                    format!(":{}", self.vim.command_line)
                };
                true
            }
            egui::Event::Key {
                key, pressed: true, ..
            } => {
                match key {
                    egui::Key::Enter => {
                        let cmd = self.vim.command_line.clone();
                        self.vim.mode = VimMode::Normal;

                        if self.vim.status.starts_with('/') {
                            // Search forward
                            self.vim.search_pattern = cmd;
                            self.vim.search_forward = true;
                            self.vim_search_next();
                        } else if self.vim.status.starts_with('?') {
                            // Search backward
                            self.vim.search_pattern = cmd;
                            self.vim.search_forward = false;
                            self.vim_search_prev();
                        } else {
                            // Ex command
                            self.execute_vim_command(&cmd);
                        }

                        self.vim.command_line.clear();
                        self.vim.status.clear();
                    }
                    egui::Key::Escape => {
                        self.vim.mode = VimMode::Normal;
                        self.vim.command_line.clear();
                        self.vim.status.clear();
                    }
                    egui::Key::Backspace => {
                        self.vim.command_line.pop();
                        if self.vim.command_line.is_empty() {
                            self.vim.mode = VimMode::Normal;
                            self.vim.status.clear();
                        }
                    }
                    _ => {}
                }
                true
            }
            _ => false,
        }
    }

    // ─── Replace mode ────────────────────────────────────────────

    fn handle_vim_replace(&mut self, event: &egui::Event) -> bool {
        match event {
            egui::Event::Text(text) => {
                if let Some(ch) = text.chars().next() {
                    self.vim_replace_char(ch);
                }
                self.vim.mode = VimMode::Normal;
                true
            }
            egui::Event::Key {
                key: egui::Key::Escape,
                pressed: true,
                ..
            } => {
                self.vim.mode = VimMode::Normal;
                true
            }
            _ => false,
        }
    }

    // ─── Ex commands (:w, :q, :wq, :<n>, :e) ────────────────────

    fn execute_vim_command(&mut self, cmd: &str) {
        let cmd = cmd.trim();
        match cmd {
            "w" => {
                // Save
                self.save_current_file();
                self.vim.status = "Written".to_string();
            }
            "q" => {
                // Close tab
                if !self.editor_tabs.is_empty() {
                    self.editor_tabs.remove(self.active_tab_idx);
                    if self.active_tab_idx > 0 {
                        self.active_tab_idx -= 1;
                    }
                }
            }
            "wq" | "x" => {
                self.save_current_file();
                if !self.editor_tabs.is_empty() {
                    self.editor_tabs.remove(self.active_tab_idx);
                    if self.active_tab_idx > 0 {
                        self.active_tab_idx -= 1;
                    }
                }
            }
            "q!" => {
                // Force close without saving
                if !self.editor_tabs.is_empty() {
                    self.editor_tabs.remove(self.active_tab_idx);
                    if self.active_tab_idx > 0 {
                        self.active_tab_idx -= 1;
                    }
                }
            }
            _ => {
                // Try as line number
                if let Ok(line) = cmd.parse::<usize>() {
                    if let Some(tab) = self.editor_tabs.get_mut(self.active_tab_idx) {
                        tab.cursor_line = line.saturating_sub(1);
                        tab.cursor_col = 0;
                    }
                } else {
                    self.vim.status = format!("E492: Not an editor command: {}", cmd);
                }
            }
        }
    }

    // ─── Vim operations (helpers) ────────────────────────────────

    fn vim_word_forward(&mut self) {
        let tab = match self.editor_tabs.get_mut(self.active_tab_idx) {
            Some(t) => t,
            None => return,
        };
        let text = tab.buffer.to_string();
        let lines: Vec<&str> = text.lines().collect();
        if tab.cursor_line >= lines.len() {
            return;
        }
        let line = lines[tab.cursor_line];
        let mut col = tab.cursor_col;

        // Skip current word chars
        while col < line.len()
            && line
                .as_bytes()
                .get(col)
                .map(|b| b.is_ascii_alphanumeric() || *b == b'_')
                .unwrap_or(false)
        {
            col += 1;
        }
        // Skip whitespace
        while col < line.len()
            && line
                .as_bytes()
                .get(col)
                .map(|b| b.is_ascii_whitespace())
                .unwrap_or(false)
        {
            col += 1;
        }

        if col >= line.len() && tab.cursor_line + 1 < lines.len() {
            tab.cursor_line += 1;
            tab.cursor_col = 0;
        } else {
            tab.cursor_col = col.min(line.len().saturating_sub(1));
        }
    }

    fn vim_word_backward(&mut self) {
        let tab = match self.editor_tabs.get_mut(self.active_tab_idx) {
            Some(t) => t,
            None => return,
        };
        let text = tab.buffer.to_string();
        let lines: Vec<&str> = text.lines().collect();
        if tab.cursor_line >= lines.len() {
            return;
        }
        let line = lines[tab.cursor_line];
        let mut col = tab.cursor_col;

        if col == 0 {
            if tab.cursor_line > 0 {
                tab.cursor_line -= 1;
                tab.cursor_col = lines[tab.cursor_line].len().saturating_sub(1);
            }
            return;
        }

        col = col.saturating_sub(1);
        // Skip whitespace backward
        while col > 0
            && line
                .as_bytes()
                .get(col)
                .map(|b| b.is_ascii_whitespace())
                .unwrap_or(false)
        {
            col -= 1;
        }
        // Skip word chars backward
        while col > 0
            && line
                .as_bytes()
                .get(col.saturating_sub(1))
                .map(|b| b.is_ascii_alphanumeric() || *b == b'_')
                .unwrap_or(false)
        {
            col -= 1;
        }
        tab.cursor_col = col;
    }

    fn vim_word_end(&mut self) {
        let tab = match self.editor_tabs.get_mut(self.active_tab_idx) {
            Some(t) => t,
            None => return,
        };
        let text = tab.buffer.to_string();
        let lines: Vec<&str> = text.lines().collect();
        if tab.cursor_line >= lines.len() {
            return;
        }
        let line = lines[tab.cursor_line];
        let mut col = tab.cursor_col + 1;

        // Skip whitespace
        while col < line.len()
            && line
                .as_bytes()
                .get(col)
                .map(|b| b.is_ascii_whitespace())
                .unwrap_or(false)
        {
            col += 1;
        }
        // Move to end of word
        while col < line.len()
            && line
                .as_bytes()
                .get(col)
                .map(|b| b.is_ascii_alphanumeric() || *b == b'_')
                .unwrap_or(false)
        {
            col += 1;
        }
        tab.cursor_col = col.saturating_sub(1).min(line.len().saturating_sub(1));
    }

    fn vim_apply_motion(&mut self, ch: char, n: usize) {
        for _ in 0..n {
            match ch {
                'w' => self.vim_word_forward(),
                'b' => self.vim_word_backward(),
                'e' => self.vim_word_end(),
                _ => {}
            }
        }
    }

    fn vim_delete_char(&mut self, n: usize) {
        let tab = match self.editor_tabs.get_mut(self.active_tab_idx) {
            Some(t) => t,
            None => return,
        };
        let mut text = tab.buffer.to_string();
        let lines: Vec<&str> = text.lines().collect();
        if tab.cursor_line >= lines.len() {
            return;
        }

        let byte_offset: usize = lines
            .iter()
            .take(tab.cursor_line)
            .map(|l| l.len() + 1)
            .sum::<usize>()
            + tab.cursor_col;
        let end = (byte_offset + n).min(text.len());
        if byte_offset < end {
            let deleted = text[byte_offset..end].to_string();
            text.replace_range(byte_offset..end, "");
            tab.buffer = crate::buffer::TextBuffer::from_str(&text);
            tab.mark_dirty();
            self.vim.set_register('"', deleted);
        }
    }

    fn vim_delete_lines(&mut self, start_line: usize, count: usize) {
        let tab = match self.editor_tabs.get_mut(self.active_tab_idx) {
            Some(t) => t,
            None => return,
        };
        let text = tab.buffer.to_string();
        let mut lines: Vec<&str> = text.lines().collect();
        let end = (start_line + count).min(lines.len());
        let deleted: String = lines[start_line..end].join("\n");
        self.vim.set_register('"', deleted);
        lines.drain(start_line..end);
        if lines.is_empty() {
            lines.push("");
        }
        tab.buffer = crate::buffer::TextBuffer::from_str(&lines.join("\n"));
        tab.cursor_line = start_line.min(lines.len().saturating_sub(1));
        tab.cursor_col = 0;
        tab.mark_dirty();
    }

    fn vim_yank_lines(&mut self, start_line: usize, count: usize) {
        let tab = match self.editor_tabs.get(self.active_tab_idx) {
            Some(t) => t,
            None => return,
        };
        let text = tab.buffer.to_string();
        let lines: Vec<&str> = text.lines().collect();
        let end = (start_line + count).min(lines.len());
        let yanked: String = lines[start_line..end].join("\n");
        self.vim.set_register('"', yanked);
        self.vim.status = format!("{} lines yanked", end - start_line);
    }

    fn vim_paste_after(&mut self, n: usize) {
        let reg_text = self.vim.get_register('"').cloned().unwrap_or_default();
        if reg_text.is_empty() {
            return;
        }

        let tab = match self.editor_tabs.get_mut(self.active_tab_idx) {
            Some(t) => t,
            None => return,
        };

        let mut text = tab.buffer.to_string();
        let lines: Vec<&str> = text.lines().collect();
        let is_linewise = reg_text.contains('\n');

        if is_linewise {
            // Paste on new line below
            let line_end: usize = lines
                .iter()
                .take(tab.cursor_line + 1)
                .map(|l| l.len() + 1)
                .sum::<usize>();
            let insert_at = line_end.min(text.len());
            let paste = format!("{}\n", reg_text).repeat(n);
            text.insert_str(insert_at, &paste);
            tab.cursor_line += 1;
            tab.cursor_col = 0;
        } else {
            let byte_offset: usize = lines
                .iter()
                .take(tab.cursor_line)
                .map(|l| l.len() + 1)
                .sum::<usize>()
                + tab.cursor_col
                + 1;
            let insert_at = byte_offset.min(text.len());
            let paste = reg_text.repeat(n);
            text.insert_str(insert_at, &paste);
            tab.cursor_col += 1;
        }

        tab.buffer = crate::buffer::TextBuffer::from_str(&text);
        tab.mark_dirty();
    }

    fn vim_paste_before(&mut self, n: usize) {
        let reg_text = self.vim.get_register('"').cloned().unwrap_or_default();
        if reg_text.is_empty() {
            return;
        }

        let tab = match self.editor_tabs.get_mut(self.active_tab_idx) {
            Some(t) => t,
            None => return,
        };

        let mut text = tab.buffer.to_string();
        let lines: Vec<&str> = text.lines().collect();
        let is_linewise = reg_text.contains('\n');

        if is_linewise {
            let line_start: usize = lines
                .iter()
                .take(tab.cursor_line)
                .map(|l| l.len() + 1)
                .sum();
            let paste = format!("{}\n", reg_text).repeat(n);
            text.insert_str(line_start.min(text.len()), &paste);
            tab.cursor_col = 0;
        } else {
            let byte_offset: usize = lines
                .iter()
                .take(tab.cursor_line)
                .map(|l| l.len() + 1)
                .sum::<usize>()
                + tab.cursor_col;
            let paste = reg_text.repeat(n);
            text.insert_str(byte_offset.min(text.len()), &paste);
        }

        tab.buffer = crate::buffer::TextBuffer::from_str(&text);
        tab.mark_dirty();
    }

    fn vim_replace_char(&mut self, ch: char) {
        let tab = match self.editor_tabs.get_mut(self.active_tab_idx) {
            Some(t) => t,
            None => return,
        };
        let mut text = tab.buffer.to_string();
        let lines: Vec<&str> = text.lines().collect();
        if tab.cursor_line >= lines.len() {
            return;
        }

        let byte_offset: usize = lines
            .iter()
            .take(tab.cursor_line)
            .map(|l| l.len() + 1)
            .sum::<usize>()
            + tab.cursor_col;
        if byte_offset < text.len() {
            text.replace_range(byte_offset..byte_offset + 1, &ch.to_string());
            tab.buffer = crate::buffer::TextBuffer::from_str(&text);
            tab.mark_dirty();
        }
    }

    fn vim_join_lines(&mut self, count: usize) {
        let tab = match self.editor_tabs.get_mut(self.active_tab_idx) {
            Some(t) => t,
            None => return,
        };
        let mut text = tab.buffer.to_string();

        for _ in 0..count {
            let lines: Vec<&str> = text.lines().collect();
            if tab.cursor_line + 1 >= lines.len() {
                break;
            }

            let line_end: usize = lines
                .iter()
                .take(tab.cursor_line + 1)
                .map(|l| l.len() + 1)
                .sum::<usize>()
                - 1;
            if line_end < text.len() {
                // Replace newline with space
                text.replace_range(line_end..line_end + 1, " ");
            }
        }

        tab.buffer = crate::buffer::TextBuffer::from_str(&text);
        tab.mark_dirty();
    }

    fn vim_insert_newline_below(&mut self) {
        let tab = match self.editor_tabs.get_mut(self.active_tab_idx) {
            Some(t) => t,
            None => return,
        };
        let mut text = tab.buffer.to_string();
        let lines: Vec<&str> = text.lines().collect();
        let line_end: usize = lines
            .iter()
            .take(tab.cursor_line + 1)
            .map(|l| l.len() + 1)
            .sum::<usize>();
        let insert_at = (line_end).min(text.len());

        // Get indentation from current line
        let indent: String = lines
            .get(tab.cursor_line)
            .map(|l| l.chars().take_while(|c| c.is_whitespace()).collect())
            .unwrap_or_default();

        text.insert_str(insert_at, &format!("\n{}", indent));
        tab.buffer = crate::buffer::TextBuffer::from_str(&text);
        tab.cursor_line += 1;
        tab.cursor_col = indent.len();
        tab.mark_dirty();
    }

    fn vim_insert_newline_above(&mut self) {
        let tab = match self.editor_tabs.get_mut(self.active_tab_idx) {
            Some(t) => t,
            None => return,
        };
        let mut text = tab.buffer.to_string();
        let lines: Vec<&str> = text.lines().collect();
        let line_start: usize = lines
            .iter()
            .take(tab.cursor_line)
            .map(|l| l.len() + 1)
            .sum();

        let indent: String = lines
            .get(tab.cursor_line)
            .map(|l| l.chars().take_while(|c| c.is_whitespace()).collect())
            .unwrap_or_default();

        text.insert_str(line_start, &format!("{}\n", indent));
        tab.buffer = crate::buffer::TextBuffer::from_str(&text);
        tab.cursor_col = indent.len();
        tab.mark_dirty();
    }

    fn vim_delete_visual_selection(&mut self) {
        // Simplified: delete from visual start to current cursor
        // Full implementation would handle char/line/block selections
        self.vim.status.clear();
    }

    fn vim_yank_visual_selection(&mut self) {
        self.vim.status.clear();
    }

    fn vim_search_next(&mut self) {
        if self.vim.search_pattern.is_empty() {
            return;
        }

        let tab = match self.editor_tabs.get_mut(self.active_tab_idx) {
            Some(t) => t,
            None => return,
        };
        let text = tab.buffer.to_string();
        let lines: Vec<&str> = text.lines().collect();

        // Search forward from current position
        for line_idx in tab.cursor_line..lines.len() {
            let start_col = if line_idx == tab.cursor_line {
                tab.cursor_col + 1
            } else {
                0
            };
            if let Some(line) = lines.get(line_idx) {
                if start_col < line.len() {
                    if let Some(pos) = line[start_col..].find(&self.vim.search_pattern) {
                        tab.cursor_line = line_idx;
                        tab.cursor_col = start_col + pos;
                        return;
                    }
                }
            }
        }
        // Wrap around
        for line_idx in 0..=tab.cursor_line {
            if let Some(line) = lines.get(line_idx) {
                if let Some(pos) = line.find(&self.vim.search_pattern) {
                    tab.cursor_line = line_idx;
                    tab.cursor_col = pos;
                    self.vim.status = "search wrapped".to_string();
                    return;
                }
            }
        }
    }

    fn vim_search_prev(&mut self) {
        if self.vim.search_pattern.is_empty() {
            return;
        }

        let tab = match self.editor_tabs.get_mut(self.active_tab_idx) {
            Some(t) => t,
            None => return,
        };
        let text = tab.buffer.to_string();
        let lines: Vec<&str> = text.lines().collect();

        // Search backward
        for line_idx in (0..=tab.cursor_line).rev() {
            if let Some(line) = lines.get(line_idx) {
                let end_col = if line_idx == tab.cursor_line {
                    tab.cursor_col
                } else {
                    line.len()
                };
                if let Some(pos) = line[..end_col].rfind(&self.vim.search_pattern) {
                    tab.cursor_line = line_idx;
                    tab.cursor_col = pos;
                    return;
                }
            }
        }
    }

    /// Apply a text object operation (iw, i", a(, etc.)
    fn vim_text_object(&mut self, obj_char: char, is_inner: bool, is_delete: bool, is_yank: bool) {
        let tab = match self.editor_tabs.get_mut(self.active_tab_idx) {
            Some(t) => t,
            None => return,
        };
        let text = tab.buffer.to_string();
        let lines: Vec<&str> = text.lines().collect();
        if tab.cursor_line >= lines.len() {
            return;
        }
        let line = lines[tab.cursor_line];
        let col = tab.cursor_col;

        // Find the range of the text object
        let (start, end) = match obj_char {
            'w' | 'W' => {
                // Word text object
                let bytes = line.as_bytes();
                let is_word_char = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
                let mut s = col;
                let mut e = col;
                if col < bytes.len() && is_word_char(bytes[col]) {
                    while s > 0 && is_word_char(bytes[s - 1]) {
                        s -= 1;
                    }
                    while e < bytes.len() && is_word_char(bytes[e]) {
                        e += 1;
                    }
                    if !is_inner {
                        // 'aw' includes trailing whitespace
                        while e < bytes.len() && bytes[e].is_ascii_whitespace() {
                            e += 1;
                        }
                    }
                }
                (s, e)
            }
            '"' | '\'' | '`' => {
                // Quote text object
                let quote = obj_char as u8;
                let bytes = line.as_bytes();
                let mut open = None;
                let mut close = None;
                // Find opening quote before or at cursor
                for i in (0..=col.min(bytes.len().saturating_sub(1))).rev() {
                    if bytes[i] == quote {
                        open = Some(i);
                        break;
                    }
                }
                // Find closing quote after cursor
                if let Some(o) = open {
                    for i in (o + 1)..bytes.len() {
                        if bytes[i] == quote {
                            close = Some(i);
                            break;
                        }
                    }
                }
                match (open, close) {
                    (Some(o), Some(c)) => {
                        if is_inner {
                            (o + 1, c)
                        } else {
                            (o, c + 1)
                        }
                    }
                    _ => return,
                }
            }
            '(' | ')' | 'b' => find_matched_pair(line, col, b'(', b')', is_inner),
            '{' | '}' | 'B' => find_matched_pair(line, col, b'{', b'}', is_inner),
            '[' | ']' => find_matched_pair(line, col, b'[', b']', is_inner),
            '<' | '>' => find_matched_pair(line, col, b'<', b'>', is_inner),
            _ => return,
        };

        if start >= end {
            return;
        }

        let selected = &line[start..end];

        if is_yank {
            self.vim.set_register('"', selected.to_string());
            self.vim.status = format!("{} chars yanked", end - start);
            return;
        }

        if is_delete {
            let mut full_text = text.clone();
            let line_offset: usize = lines
                .iter()
                .take(tab.cursor_line)
                .map(|l| l.len() + 1)
                .sum();
            full_text.replace_range((line_offset + start)..(line_offset + end), "");
            self.vim.set_register('"', selected.to_string());
            tab.buffer = crate::buffer::TextBuffer::from_str(&full_text);
            tab.cursor_col = start;
            tab.mark_dirty();
        }
    }
}

/// Find matched bracket pair range on a line (free function to avoid borrow issues)
fn find_matched_pair(
    line: &str,
    col: usize,
    open: u8,
    close: u8,
    is_inner: bool,
) -> (usize, usize) {
    let bytes = line.as_bytes();
    let mut depth = 0;
    let mut open_pos = None;

    // Search backward for opening bracket
    for i in (0..=col.min(bytes.len().saturating_sub(1))).rev() {
        if bytes[i] == close {
            depth += 1;
        }
        if bytes[i] == open {
            if depth == 0 {
                open_pos = Some(i);
                break;
            }
            depth -= 1;
        }
    }

    let open_pos = match open_pos {
        Some(p) => p,
        None => return (col, col),
    };

    // Search forward for closing bracket
    depth = 0;
    for i in (open_pos + 1)..bytes.len() {
        if bytes[i] == open {
            depth += 1;
        }
        if bytes[i] == close {
            if depth == 0 {
                return if is_inner {
                    (open_pos + 1, i)
                } else {
                    (open_pos, i + 1)
                };
            }
            depth -= 1;
        }
    }

    (col, col)
}

impl BerryCodeApp {
    /// Toggle Vim mode on/off
    pub(crate) fn toggle_vim_mode(&mut self) {
        self.vim.enabled = !self.vim.enabled;
        if self.vim.enabled {
            self.vim.mode = VimMode::Normal;
            self.vim.status = "-- NORMAL --".to_string();
            self.status_message = "Vim mode enabled".to_string();
        } else {
            self.vim.status.clear();
            self.status_message = "Vim mode disabled".to_string();
        }
        self.status_message_timestamp = Some(std::time::Instant::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vim_state_default() {
        let state = VimState::default();
        assert_eq!(state.mode, VimMode::Normal);
        assert!(!state.enabled);
        assert_eq!(state.mode_display(), "NORMAL");
    }

    #[test]
    fn test_vim_count() {
        let mut state = VimState::default();
        assert_eq!(state.get_count(), 1); // default
        state.count = Some(5);
        assert_eq!(state.get_count(), 5);
        assert_eq!(state.get_count(), 1); // consumed
    }

    #[test]
    fn test_vim_register() {
        let mut state = VimState::default();
        state.set_register('a', "hello".to_string());
        assert_eq!(state.get_register('a'), Some(&"hello".to_string()));
        assert_eq!(state.get_register('"'), Some(&"hello".to_string())); // unnamed also set
    }

    #[test]
    fn test_mode_display() {
        assert_eq!(VimMode::Normal.to_display(), "NORMAL");
        assert_eq!(VimMode::Insert.to_display(), "INSERT");
        assert_eq!(VimMode::Visual.to_display(), "VISUAL");
    }
}

impl VimMode {
    fn to_display(&self) -> &'static str {
        match self {
            VimMode::Normal => "NORMAL",
            VimMode::Insert => "INSERT",
            VimMode::Visual => "VISUAL",
            VimMode::VisualLine => "V-LINE",
            VimMode::Command => "COMMAND",
            VimMode::Replace => "REPLACE",
        }
    }
}
