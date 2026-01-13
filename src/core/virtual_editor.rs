//! Canvas-based Virtual Editor
//!
//! Phase 1: Basic Canvas rendering without input
//! - Display canvas element
//! - Render text from Rope buffer
//! - Draw cursor

use crate::buffer::TextBuffer;
use crate::completion_widget::CompletionWidget;
use crate::core::canvas_renderer::{CanvasRenderer, LINE_HEIGHT};
use crate::diagnostics_panel::DiagnosticsPanel;
use crate::focus_stack::{FocusStack, FocusLayer};
use crate::hover_tooltip::HoverTooltip;
use crate::lsp_ui::{CompletionItem, Diagnostic, HoverInfo, LspIntegration};
use crate::syntax::SyntaxHighlighter;
use crate::theme::EditorTheme;
use crate::types::Position;
use leptos::html::Canvas;
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;

// Undo/Redo用の状態スナップショット
#[derive(Clone)]
struct EditorSnapshot {
    buffer: TextBuffer,
    cursor_line: usize,
    cursor_col: usize,
}

/// Editor commands for testability and event abstraction
///
/// This enum represents all possible editor commands, decoupling input events from logic.
/// Benefits:
/// - Unit testable: Each command can be tested independently
/// - Event source agnostic: Same command from keyboard, mouse, or API
/// - Undo/Redo friendly: Commands can be recorded and replayed
#[derive(Debug, Clone, PartialEq)]
pub enum EditorCommand {
    // Text editing
    InsertChar(char),
    InsertText(String),
    Newline,
    Backspace,
    Delete,

    // Clipboard operations
    Copy,
    Cut,
    Paste,
    SelectAll,

    // Cursor movement (extend_selection = Shift key behavior)
    MoveCursorLeft { extend_selection: bool },
    MoveCursorRight { extend_selection: bool },
    MoveCursorUp { extend_selection: bool },
    MoveCursorDown { extend_selection: bool },
    MoveCursorHome { extend_selection: bool },
    MoveCursorEnd { extend_selection: bool },
    PageUp,
    PageDown,

    // Undo/Redo
    Undo,
    Redo,

    // File operations
    Save,

    // LSP operations
    TriggerCompletion,
    GotoDefinition,

    // Completion widget navigation (when completion popup is active)
    CompletionNext,
    CompletionPrevious,
    CompletionSelect,
    CompletionDismiss,

    // Scroll
    Scroll(f64), // delta_y

    // Mouse operations
    MouseClick { line: usize, col: usize },
    MouseDragStart { line: usize, col: usize },
    MouseDragMove { line: usize, col: usize },
    MouseDragEnd,
}

// エディタタブ（簡略版）
// Note: Ropeのcloneは O(1) なので、Rcは不要
#[derive(Clone)]
pub struct EditorTab {
    pub file_path: String,
    pub buffer: TextBuffer,
    pub cursor_line: usize,
    pub cursor_col: usize,
    pub scroll_top: f64,
    // テキスト選択範囲
    pub selection_start: Option<(usize, usize)>, // (line, col)
    pub selection_end: Option<(usize, usize)>,   // (line, col)
    // Undo/Redo履歴
    undo_stack: Vec<EditorSnapshot>,
    redo_stack: Vec<EditorSnapshot>,
    // シンタックスハイライト
    syntax_highlighter: SyntaxHighlighter,
    // ファイルの言語（拡張子から判定）
    language: Option<String>,
    // カーソルアニメーション用の前回位置
    prev_cursor_line: usize,
    prev_cursor_col: usize,
    cursor_move_timestamp: f64,
}

impl EditorTab {
    pub fn new(file_path: String, content: String) -> Self {
        // ファイル拡張子から言語を推測
        let mut syntax_highlighter = SyntaxHighlighter::new();
        let language = if file_path.ends_with(".rs") {
            let _ = syntax_highlighter.set_language("rust");
            Some("rust".to_string())
        } else if file_path.ends_with(".js") || file_path.ends_with(".jsx") {
            let _ = syntax_highlighter.set_language("javascript");
            Some("javascript".to_string())
        } else if file_path.ends_with(".py") {
            let _ = syntax_highlighter.set_language("python");
            Some("python".to_string())
        } else if file_path.ends_with(".html") || file_path.ends_with(".htm") {
            let _ = syntax_highlighter.set_language("html");
            Some("html".to_string())
        } else if file_path.ends_with(".css") {
            let _ = syntax_highlighter.set_language("css");
            Some("css".to_string())
        } else {
            None // サポートされていない拡張子
        };

        Self {
            file_path,
            buffer: TextBuffer::from_str(&content),
            cursor_line: 0,
            cursor_col: 0,
            scroll_top: 0.0,
            selection_start: None,
            selection_end: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            syntax_highlighter,
            language,
            prev_cursor_line: 0,
            prev_cursor_col: 0,
            cursor_move_timestamp: 0.0,
        }
    }

    // 現在の状態をUndoスタックに保存
    // 🚀 MEMORY OPTIMIZATION: Reduced from 100 to 30 snapshots
    // - Each snapshot contains a full Rope clone (Copy-on-Write, but still memory overhead)
    // - Large files with frequent edits can accumulate significant memory in old Rope branches
    // - 30 undo levels balances functionality with memory efficiency for large files
    fn save_undo_state(&mut self) {
        let snapshot = EditorSnapshot {
            buffer: self.buffer.clone(),
            cursor_line: self.cursor_line,
            cursor_col: self.cursor_col,
        };
        self.undo_stack.push(snapshot);

        // Limit undo stack to 30 snapshots (was 100, then 50)
        // Further reduced to prevent memory accumulation with large files
        const MAX_UNDO_HISTORY: usize = 30;
        if self.undo_stack.len() > MAX_UNDO_HISTORY {
            self.undo_stack.remove(0);
        }

        // 新しい編集が行われたらRedoスタックをクリア
        self.redo_stack.clear();
    }

    // Undo実行
    fn undo(&mut self) -> bool {
        if let Some(snapshot) = self.undo_stack.pop() {
            // 現在の状態をRedoスタックに保存
            let redo_snapshot = EditorSnapshot {
                buffer: self.buffer.clone(),
                cursor_line: self.cursor_line,
                cursor_col: self.cursor_col,
            };
            self.redo_stack.push(redo_snapshot);

            // 状態を復元
            self.buffer = snapshot.buffer;
            self.cursor_line = snapshot.cursor_line;
            self.cursor_col = snapshot.cursor_col;
            self.clear_selection();
            true
        } else {
            false
        }
    }

    // Redo実行
    fn redo(&mut self) -> bool {
        if let Some(snapshot) = self.redo_stack.pop() {
            // 現在の状態をUndoスタックに保存
            let undo_snapshot = EditorSnapshot {
                buffer: self.buffer.clone(),
                cursor_line: self.cursor_line,
                cursor_col: self.cursor_col,
            };
            self.undo_stack.push(undo_snapshot);

            // 状態を復元
            self.buffer = snapshot.buffer;
            self.cursor_line = snapshot.cursor_line;
            self.cursor_col = snapshot.cursor_col;
            self.clear_selection();
            true
        } else {
            false
        }
    }

    // 選択範囲があるかチェック
    fn has_selection(&self) -> bool {
        self.selection_start.is_some() && self.selection_end.is_some()
    }

    // 選択範囲をクリア
    fn clear_selection(&mut self) {
        self.selection_start = None;
        self.selection_end = None;
    }

    // カーソルを移動（アニメーション用のトラッキング付き）
    fn move_cursor(&mut self, new_line: usize, new_col: usize) {
        // アニメーションのために前回位置を保存
        self.prev_cursor_line = self.cursor_line;
        self.prev_cursor_col = self.cursor_col;

        // TODO: Performance API integration for smooth cursor animation
        // Currently disabled due to web_sys API compatibility
        self.cursor_move_timestamp = 0.0;

        // Update cursor position
        self.cursor_line = new_line;
        self.cursor_col = new_col;
    }

    // 選択範囲のテキストを取得
    fn get_selected_text(&self) -> Option<String> {
        if !self.has_selection() {
            return None;
        }

        let (start_line, start_col) = self.selection_start?;
        let (end_line, end_col) = self.selection_end?;

        // 選択範囲を正規化（開始 < 終了）
        let ((sl, sc), (el, ec)) = if start_line < end_line || (start_line == end_line && start_col <= end_col) {
            ((start_line, start_col), (end_line, end_col))
        } else {
            ((end_line, end_col), (start_line, start_col))
        };

        let start_char = self.buffer.line_to_char(sl) + sc;
        let end_char = self.buffer.line_to_char(el) + ec;

        self.buffer.slice(start_char, end_char)
    }

    // 選択範囲のテキストを削除
    fn delete_selection(&mut self) {
        if !self.has_selection() {
            return;
        }

        let (start_line, start_col) = self.selection_start.unwrap();
        let (end_line, end_col) = self.selection_end.unwrap();

        // 選択範囲を正規化
        let ((sl, sc), (el, ec)) = if start_line < end_line || (start_line == end_line && start_col <= end_col) {
            ((start_line, start_col), (end_line, end_col))
        } else {
            ((end_line, end_col), (start_line, start_col))
        };

        let start_char = self.buffer.line_to_char(sl) + sc;
        let end_char = self.buffer.line_to_char(el) + ec;

        self.buffer.remove(start_char, end_char);

        // カーソルを選択開始位置に移動
        self.cursor_line = sl;
        self.cursor_col = sc;
        self.clear_selection();
    }

    // カーソルが見える範囲にスクロールを調整
    pub fn scroll_into_view(&mut self, canvas_height: f64) {
        let line_height = 20.0; // LINE_HEIGHT
        let cursor_y = self.cursor_line as f64 * line_height;
        let visible_lines = (canvas_height / line_height).floor();
        let total_lines = self.buffer.len_lines();

        // カーソルが上に隠れている場合
        if cursor_y < self.scroll_top {
            self.scroll_top = cursor_y;
        }
        // カーソルが下に隠れている場合（1行分のマージン）
        else if cursor_y + line_height > self.scroll_top + canvas_height {
            self.scroll_top = cursor_y + line_height - canvas_height;
        }

        // 🎨 IntelliJ-style: スクロール範囲を拡大して最終行を画面中央に配置可能にする
        // "Scroll past end" 機能 - 最終行を読みやすい位置に配置できる
        let content_height = total_lines as f64 * line_height;
        let max_scroll = (content_height - canvas_height / 2.0).max(0.0);

        // ✅ FIX: スクロール位置を0～max_scrollの範囲内に制限
        self.scroll_top = self.scroll_top.max(0.0).min(max_scroll);
    }

    // 指定位置の単語の境界を取得
    fn get_word_bounds(&self, line: usize, col: usize) -> (usize, usize) {
        let line_text = self.buffer.line(line).unwrap_or_default();
        let chars: Vec<char> = line_text.chars().collect();

        if col >= chars.len() {
            return (chars.len(), chars.len());
        }

        // 単語の文字かどうかを判定（英数字、アンダースコア、日本語など）
        let is_word_char = |c: char| c.is_alphanumeric() || c == '_' || c > '\u{007F}';

        // クリック位置が単語文字でない場合は空の選択
        if !is_word_char(chars[col]) {
            return (col, col);
        }

        // 単語の開始位置を探す
        let mut start = col;
        while start > 0 && is_word_char(chars[start - 1]) {
            start -= 1;
        }

        // 単語の終了位置を探す
        let mut end = col;
        while end < chars.len() && is_word_char(chars[end]) {
            end += 1;
        }

        (start, end)
    }

    // カーソル位置の単語を選択
    fn select_word_at_cursor(&mut self) {
        let (start, end) = self.get_word_bounds(self.cursor_line, self.cursor_col);
        if start < end {
            self.selection_start = Some((self.cursor_line, start));
            self.selection_end = Some((self.cursor_line, end));
            self.cursor_col = end;
        }
    }

    /// Execute an editor action - Command Pattern implementation
    /// Returns true if the action modified the buffer
    pub fn execute_action(&mut self, action: &crate::core::actions::EditorAction) -> bool {
        use crate::core::actions::{Direction, EditorAction};

        // Save undo state if action requires it
        if action.requires_undo_save() {
            self.save_undo_state();
        }

        let mut buffer_modified = false;

        match action {
            // ===== Text Input =====
            EditorAction::InsertChar(ch) => {
                if self.has_selection() {
                    self.delete_selection();
                }
                let char_idx = self.buffer.line_to_char(self.cursor_line) + self.cursor_col;
                self.buffer.insert(char_idx, &ch.to_string());
                self.cursor_col += 1;
                buffer_modified = true;
            }

            EditorAction::InsertText(text) => {
                if self.has_selection() {
                    self.delete_selection();
                }
                let char_idx = self.buffer.line_to_char(self.cursor_line) + self.cursor_col;
                let chars_count = text.chars().count();
                self.buffer.insert(char_idx, text);
                self.cursor_col += chars_count;
                buffer_modified = true;
            }

            EditorAction::NewLine => {
                if self.has_selection() {
                    self.delete_selection();
                }
                let char_idx = self.buffer.line_to_char(self.cursor_line) + self.cursor_col;
                self.buffer.insert(char_idx, "\n");
                self.cursor_line += 1;
                self.cursor_col = 0;
                buffer_modified = true;
            }

            EditorAction::Backspace => {
                if self.has_selection() {
                    self.delete_selection();
                    buffer_modified = true;
                } else if self.cursor_col > 0 {
                    let char_idx = self.buffer.line_to_char(self.cursor_line) + self.cursor_col - 1;
                    self.buffer.remove(char_idx, char_idx + 1);
                    self.cursor_col -= 1;
                    buffer_modified = true;
                } else if self.cursor_line > 0 {
                    let prev_line_len = self.buffer.line(self.cursor_line - 1)
                        .map(|s| s.trim_end_matches('\n').chars().count())
                        .unwrap_or(0);
                    let char_idx = self.buffer.line_to_char(self.cursor_line) - 1;
                    self.buffer.remove(char_idx, char_idx + 1);
                    self.cursor_line -= 1;
                    self.cursor_col = prev_line_len;
                    buffer_modified = true;
                }
            }

            EditorAction::Delete => {
                if self.has_selection() {
                    self.delete_selection();
                    buffer_modified = true;
                } else {
                    let line_len = self.buffer.line(self.cursor_line)
                        .map(|s| s.trim_end_matches('\n').chars().count())
                        .unwrap_or(0);
                    if self.cursor_col < line_len {
                        let char_idx = self.buffer.line_to_char(self.cursor_line) + self.cursor_col;
                        self.buffer.remove(char_idx, char_idx + 1);
                        buffer_modified = true;
                    } else if self.cursor_line < self.buffer.len_lines() - 1 {
                        let char_idx = self.buffer.line_to_char(self.cursor_line) + self.cursor_col;
                        self.buffer.remove(char_idx, char_idx + 1);
                        buffer_modified = true;
                    }
                }
            }

            // ===== Cursor Movement =====
            EditorAction::MoveCursor(Direction::Left) => {
                let (new_line, new_col) = if self.cursor_col > 0 {
                    (self.cursor_line, self.cursor_col - 1)
                } else if self.cursor_line > 0 {
                    let line = self.cursor_line - 1;
                    let col = self.buffer.line(line)
                        .map(|s| s.trim_end_matches('\n').chars().count())
                        .unwrap_or(0);
                    (line, col)
                } else {
                    (self.cursor_line, self.cursor_col)
                };
                self.move_cursor(new_line, new_col);
                self.clear_selection();
            }

            EditorAction::MoveCursor(Direction::Right) => {
                let line_len = self.buffer.line(self.cursor_line)
                    .map(|s| s.trim_end_matches('\n').chars().count())
                    .unwrap_or(0);
                let (new_line, new_col) = if self.cursor_col < line_len {
                    (self.cursor_line, self.cursor_col + 1)
                } else if self.cursor_line < self.buffer.len_lines() - 1 {
                    (self.cursor_line + 1, 0)
                } else {
                    (self.cursor_line, self.cursor_col)
                };
                self.move_cursor(new_line, new_col);
                self.clear_selection();
            }

            EditorAction::MoveCursor(Direction::Up) => {
                let (new_line, new_col) = if self.cursor_line > 0 {
                    let line = self.cursor_line - 1;
                    let line_len = self.buffer.line(line)
                        .map(|s| s.trim_end_matches('\n').chars().count())
                        .unwrap_or(0);
                    (line, self.cursor_col.min(line_len))
                } else {
                    (self.cursor_line, self.cursor_col)
                };
                self.move_cursor(new_line, new_col);
                self.clear_selection();
            }

            EditorAction::MoveCursor(Direction::Down) => {
                let (new_line, new_col) = if self.cursor_line < self.buffer.len_lines() - 1 {
                    let line = self.cursor_line + 1;
                    let line_len = self.buffer.line(line)
                        .map(|s| s.trim_end_matches('\n').chars().count())
                        .unwrap_or(0);
                    (line, self.cursor_col.min(line_len))
                } else {
                    (self.cursor_line, self.cursor_col)
                };
                self.move_cursor(new_line, new_col);
                self.clear_selection();
            }

            EditorAction::MoveToLineStart => {
                self.move_cursor(self.cursor_line, 0);
                self.clear_selection();
            }

            EditorAction::MoveToLineEnd => {
                let line_len = self.buffer.line(self.cursor_line)
                    .map(|s| s.trim_end_matches('\n').chars().count())
                    .unwrap_or(0);
                self.move_cursor(self.cursor_line, line_len);
                self.clear_selection();
            }

            EditorAction::MoveToPosition { line, col } => {
                let new_line = (*line).min(self.buffer.len_lines() - 1);
                let line_len = self.buffer.line(new_line)
                    .map(|s| s.trim_end_matches('\n').chars().count())
                    .unwrap_or(0);
                let new_col = (*col).min(line_len);
                self.move_cursor(new_line, new_col);
                self.clear_selection();
            }

            EditorAction::PageUp => {
                self.cursor_line = self.cursor_line.saturating_sub(20);
                self.scroll_top = (self.cursor_line as f64 * 20.0).max(0.0);
                self.clear_selection();
            }

            EditorAction::PageDown => {
                self.cursor_line = (self.cursor_line + 20).min(self.buffer.len_lines() - 1);
                self.clear_selection();
            }

            // ===== Selection =====
            EditorAction::ExtendSelection(dir) => {
                if !self.has_selection() {
                    self.selection_start = Some((self.cursor_line, self.cursor_col));
                }
                // Move cursor, then set selection_end
                match dir {
                    Direction::Left => {
                        if self.cursor_col > 0 {
                            self.cursor_col -= 1;
                        } else if self.cursor_line > 0 {
                            self.cursor_line -= 1;
                            self.cursor_col = self.buffer.line(self.cursor_line)
                                .map(|s| s.trim_end_matches('\n').chars().count())
                                .unwrap_or(0);
                        }
                    }
                    Direction::Right => {
                        let line_len = self.buffer.line(self.cursor_line)
                            .map(|s| s.trim_end_matches('\n').chars().count())
                            .unwrap_or(0);
                        if self.cursor_col < line_len {
                            self.cursor_col += 1;
                        } else if self.cursor_line < self.buffer.len_lines() - 1 {
                            self.cursor_line += 1;
                            self.cursor_col = 0;
                        }
                    }
                    Direction::Up => {
                        if self.cursor_line > 0 {
                            self.cursor_line -= 1;
                            let line_len = self.buffer.line(self.cursor_line)
                                .map(|s| s.trim_end_matches('\n').chars().count())
                                .unwrap_or(0);
                            self.cursor_col = self.cursor_col.min(line_len);
                        }
                    }
                    Direction::Down => {
                        if self.cursor_line < self.buffer.len_lines() - 1 {
                            self.cursor_line += 1;
                            let line_len = self.buffer.line(self.cursor_line)
                                .map(|s| s.trim_end_matches('\n').chars().count())
                                .unwrap_or(0);
                            self.cursor_col = self.cursor_col.min(line_len);
                        }
                    }
                }
                self.selection_end = Some((self.cursor_line, self.cursor_col));
            }

            EditorAction::ExtendToLineStart => {
                if !self.has_selection() {
                    self.selection_start = Some((self.cursor_line, self.cursor_col));
                }
                self.cursor_col = 0;
                self.selection_end = Some((self.cursor_line, self.cursor_col));
            }

            EditorAction::ExtendToLineEnd => {
                if !self.has_selection() {
                    self.selection_start = Some((self.cursor_line, self.cursor_col));
                }
                let line_len = self.buffer.line(self.cursor_line)
                    .map(|s| s.trim_end_matches('\n').chars().count())
                    .unwrap_or(0);
                self.cursor_col = line_len;
                self.selection_end = Some((self.cursor_line, self.cursor_col));
            }

            EditorAction::SelectAll => {
                self.selection_start = Some((0, 0));
                let last_line = self.buffer.len_lines() - 1;
                let last_col = self.buffer.line(last_line)
                    .map(|s| s.trim_end_matches('\n').chars().count())
                    .unwrap_or(0);
                self.selection_end = Some((last_line, last_col));
                self.cursor_line = last_line;
                self.cursor_col = last_col;
            }

            EditorAction::ClearSelection => {
                self.clear_selection();
            }

            EditorAction::SetSelection { start_line, start_col, end_line, end_col } => {
                self.selection_start = Some((*start_line, *start_col));
                self.selection_end = Some((*end_line, *end_col));
            }

            // ===== Clipboard Operations =====
            // Note: Copy/Cut/Paste require clipboard access from parent component
            // These are handled separately in the event handler
            EditorAction::Copy | EditorAction::Cut | EditorAction::Paste => {
                // Handled externally (requires clipboard_text signal)
            }

            // ===== Undo/Redo =====
            EditorAction::Undo => {
                buffer_modified = self.undo();
            }

            EditorAction::Redo => {
                buffer_modified = self.redo();
            }

            // ===== File Operations =====
            // Note: Save/Close are handled by parent component
            EditorAction::Save | EditorAction::SaveAs(_) | EditorAction::Close => {
                // Handled externally (requires Tauri bridge)
            }

            // ===== Formatting & Editing =====
            // Note: These are complex operations requiring external implementations
            EditorAction::Format
            | EditorAction::ToggleComment
            | EditorAction::Indent
            | EditorAction::Dedent => {
                // TODO: Implement in future refactoring
            }

            // ===== LSP Integration =====
            // Note: LSP operations require async calls and are handled externally
            EditorAction::GotoDefinition
            | EditorAction::TriggerCompletion
            | EditorAction::ShowHover
            | EditorAction::FindReferences
            | EditorAction::Rename(_) => {
                // Handled externally (requires LSP client)
            }

            // ===== Search & Navigation =====
            EditorAction::OpenCommandPalette
            | EditorAction::GotoLine(_)
            | EditorAction::Find(_)
            | EditorAction::Replace { .. } => {
                // Handled externally (requires UI components)
            }

            // ===== View Control =====
            EditorAction::ToggleSidebar
            | EditorAction::SplitHorizontal
            | EditorAction::SplitVertical => {
                // Handled externally (requires layout management)
            }

            // ===== Special =====
            EditorAction::Batch(actions) => {
                for action in actions {
                    if self.execute_action(action) {
                        buffer_modified = true;
                    }
                }
            }

            EditorAction::None => {
                // No-op
            }
        }

        buffer_modified
    }
}

/// マウスのX座標から、テキスト内の列位置を正確に計算する
/// measureText()を使ってピクセル単位で最も近い文字位置を見つける
fn find_column_from_x_position(renderer: &CanvasRenderer, line_text: &str, target_x: f64) -> usize {
    let chars: Vec<char> = line_text.chars().collect();

    if chars.is_empty() || target_x <= 0.0 {
        return 0;
    }

    // 各文字位置の幅を測定して、最も近い位置を見つける
    for i in 0..=chars.len() {
        let text_up_to_i: String = chars[0..i].iter().collect();
        let width = renderer.measure_text(&text_up_to_i);

        if i == chars.len() {
            // 最後の文字を超えている
            return chars.len();
        }

        // 次の文字の中間位置を計算
        let text_up_to_next: String = chars[0..=i].iter().collect();
        let next_width = renderer.measure_text(&text_up_to_next);
        let mid_width = (width + next_width) / 2.0;

        // target_x が現在の文字と次の文字の中間より前なら、現在位置を返す
        if target_x < mid_width {
            return i;
        }
    }

    chars.len()
}

/// Helper: Find symbol boundaries at given position
/// Returns (start_col, end_col) if cursor is on a symbol
fn find_symbol_at_position(line_text: &str, col: usize) -> Option<(usize, usize)> {
    let chars: Vec<char> = line_text.chars().collect();

    if col >= chars.len() {
        return None;
    }

    let ch = chars[col];

    // Check if current character is part of an identifier
    if !ch.is_alphanumeric() && ch != '_' {
        return None;
    }

    // Find start of symbol
    let mut start = col;
    while start > 0 {
        let prev_ch = chars[start - 1];
        if !prev_ch.is_alphanumeric() && prev_ch != '_' {
            break;
        }
        start -= 1;
    }

    // Find end of symbol
    let mut end = col;
    while end < chars.len() {
        let curr_ch = chars[end];
        if !curr_ch.is_alphanumeric() && curr_ch != '_' {
            break;
        }
        end += 1;
    }

    if start < end {
        Some((start, end))
    } else {
        None
    }
}

/// ✅ LSP Integration: Canvas pixel → LSP position (line, column)
fn canvas_pixel_to_lsp_position(
    renderer: &CanvasRenderer,
    pixel_x: f64,
    pixel_y: f64,
    scroll_top: f64,
    buffer: &TextBuffer,
) -> Position {
    // Calculate line from Y coordinate
    let line = ((pixel_y + scroll_top) / LINE_HEIGHT) as usize;

    // Get the line text
    let line_text = if line < buffer.len_lines() {
        buffer.line(line).unwrap_or_default()
    } else {
        String::new()
    };

    // Calculate column from X coordinate using measureText
    let adjusted_x = pixel_x - renderer.gutter_width() - 15.0;
    let column = find_column_from_x_position(renderer, &line_text, adjusted_x);

    Position { line, column }
}

/// ✅ LSP Integration: LSP position (line, column) → Canvas pixel (x, y)
fn lsp_position_to_canvas_pixel(
    renderer: &CanvasRenderer,
    position: Position,
    scroll_top: f64,
    buffer: &TextBuffer,
) -> (f64, f64) {
    // Calculate Y from line
    let y = (position.line as f64) * LINE_HEIGHT - scroll_top;

    // Get line text up to cursor position
    let line_text = if position.line < buffer.len_lines() {
        buffer.line(position.line).unwrap_or_default()
    } else {
        String::new()
    };

    // Calculate X using measureText for precise positioning (handles multi-byte chars)
    let text_before_cursor: String = line_text
        .chars()
        .take(position.column)
        .collect();
    let text_width = renderer.measure_text(&text_before_cursor);

    let x = renderer.gutter_width() + 15.0 + text_width;

    (x, y)
}

#[component]
pub fn VirtualEditorPanel(
    /// ✅ FIX: Make selected_file optional - if not provided, use context
    #[prop(into, optional)] selected_file: Option<Signal<Option<(String, String)>>>,
    /// Whether this editor panel is currently active (visible). Defaults to true for backwards compatibility.
    #[prop(into, default = Signal::derive(|| true))]
    is_active: Signal<bool>,
    /// Focus stack for keyboard event routing (prevents conflicts with modals)
    #[prop(optional)]
    focus_stack: Option<RwSignal<FocusStack>>,
) -> impl IntoView {
    // ✅ FIX: Use context if not provided as prop
    let selected_file = selected_file.unwrap_or_else(|| {
        use_context::<RwSignal<Option<(String, String)>>>()
            .expect("selected_file must be provided via context")
            .into()
    });

    // Use provided focus_stack or create a local one for backwards compatibility
    let focus_stack = focus_stack.unwrap_or_else(|| RwSignal::new(FocusStack::new()));
    let canvas_ref = NodeRef::<Canvas>::new();
    let container_ref = NodeRef::<leptos::html::Div>::new();

    // タブ管理（複数タブ対応）
    let tabs = RwSignal::new(Vec::<EditorTab>::new());
    let active_tab_index = RwSignal::new(Option::<usize>::None);

    // 再描画トリガー用
    let render_trigger = RwSignal::new(0u32);

    // IME状態管理
    let is_composing = RwSignal::new(false);
    let composing_text = RwSignal::new(String::new());

    // IME用の隠しinput要素
    let ime_input_ref = NodeRef::<leptos::html::Input>::new();

    // カーソルのピクセル位置（IME候補ウィンドウの位置制御用）
    let cursor_x = RwSignal::new(0.0);
    let cursor_y = RwSignal::new(0.0);

    // Copy/Paste用のクリップボード（簡易実装）
    let clipboard_text = RwSignal::new(String::new());

    // マウスドラッグ中かどうか
    let is_dragging = RwSignal::new(false);

    // ✅ LSP Integration: Hover debounce timer
    let hover_debounce_timer = RwSignal::new(0u32);

    // ✅ LSP Integration: Completion state
    let completion_items = RwSignal::new(Vec::<CompletionItem>::new());
    let show_completion = RwSignal::new(false);
    let completion_selected_index = RwSignal::new(0usize);

    // ✅ LSP Integration: Hover state
    let hover_info = RwSignal::new(Option::<HoverInfo>::None);
    let hover_pixel_position = RwSignal::new(Option::<(f64, f64)>::None);

    // ✅ LSP Integration: Cmd+Hover symbol underline (line, start_col, end_col)
    let hover_symbol_underline = RwSignal::new(Option::<(usize, usize, usize)>::None);

    // ✅ LSP Integration: Diagnostics state
    let diagnostics = RwSignal::new(Vec::<Diagnostic>::new());

    // ✅ PERFORMANCE: Use global LSP from context (initialized once at startup)
    let lsp = use_context::<RwSignal<LspIntegration>>()
        .unwrap_or_else(|| {
            leptos::logging::warn!("⚠️  Global LSP context not found, using new instance (this is a bug!)");
            RwSignal::new(LspIntegration::new())
        });
    let lsp_initialized = use_context::<RwSignal<bool>>()
        .unwrap_or_else(|| {
            leptos::logging::warn!("⚠️  Global lsp_initialized context not found (this is a bug!)");
            RwSignal::new(false)
        });

    // ファイルが選択されたらタブを作成または切り替え
    Effect::new(move |_| {
        let current_file = selected_file.get();

        leptos::logging::log!("🔍 DEBUG: Effect triggered, current_file={:?}",
            current_file.as_ref().map(|(p, _)| p));

        if let Some((path, content)) = current_file {
            leptos::logging::log!("🔍 DEBUG: Opening file: {}", path);

            // ✅ FIX: Calculate index OUTSIDE of tabs.update() to prevent circular reference
            let new_tab_index = tabs.with_untracked(|tabs_vec| {
                // 既存のタブを探す
                if let Some(existing_index) = tabs_vec.iter().position(|t| &t.file_path == &path) {
                    Some(existing_index)
                } else {
                    None
                }
            });

            // ✅ FIX: Update tabs and active_tab_index - these are normal reactive updates
            // The FileTree uses untrack when calling set(), so this won't cause infinite loops
            if let Some(existing_index) = new_tab_index {
                // 既存のタブをアクティブにする
                leptos::logging::log!("🔍 DEBUG: Switching to existing tab at index: {}", existing_index);
                active_tab_index.set(Some(existing_index));
            } else {
                // 新しいタブを追加
                leptos::logging::log!("🔍 DEBUG: Creating new tab for: {}", path);
                let new_index = tabs.with_untracked(|tabs_vec| tabs_vec.len());
                tabs.update(|tabs_vec| {
                    tabs_vec.push(EditorTab::new(path.clone(), content.clone()));
                });
                active_tab_index.set(Some(new_index));
            }

            // ✅ PERFORMANCE: Use global LSP (initialized once at startup)
            // Only update file path, add to context, and request diagnostics
            untrack(move || {
                let lsp_client = lsp.get_untracked();
                let is_init = lsp_initialized.get_untracked();

                // Update LSP with the current file path
                lsp_client.set_file_path(path.clone());

                if is_init {
                    leptos::logging::log!("📂 LSP: File opened: {}", path);

                    // Add file to LSP context and request diagnostics
                    spawn_local(async move {
                        // 1. Add file to berry_api context for LSP
                        if let Err(e) = lsp_client.add_file_to_context(path.clone()).await {
                            leptos::logging::log!("⚠️  LSP: Failed to add file to context: {:?}", e);
                        }

                        // 2. Request diagnostics for this file
                        match lsp_client.request_diagnostics().await {
                            Ok(diags) => {
                                let count = diags.len();
                                diagnostics.set(diags);
                                leptos::logging::log!("✅ LSP: Diagnostics loaded: {} items for {}", count, path);
                            }
                            Err(e) => {
                                leptos::logging::log!("❌ LSP: Diagnostics error: {:?}", e);
                            }
                        }
                    });
                } else {
                    leptos::logging::log!("⚠️  LSP: Not initialized, skipping diagnostics for {}", path);
                }

                render_trigger.set(0);
            });
        }
    });

    // ⚠️ LSP: Buffer change detection temporarily disabled
    // This Effect was causing memory issues by creating too many spawn_local tasks
    // TODO: Implement more efficient diagnostics update mechanism
    // For now, diagnostics are only updated on file selection
    let diagnostics_debounce_timer = RwSignal::new(0u32);
    let _ = diagnostics_debounce_timer; // Suppress unused warning

    // 後方互換性：current_tabはMemoで計算される読み取り専用の値
    // 書き込みはヘルパー関数を使用
    let current_tab_memo = Signal::derive(move || {
        if let Some(index) = active_tab_index.get() {
            tabs.get().get(index).cloned()
        } else {
            None
        }
    });

    // current_tab.get() の代わり
    #[derive(Clone, Copy)]
    struct CurrentTab {
        tabs: RwSignal<Vec<EditorTab>>,
        active_index: RwSignal<Option<usize>>,
        memo: Signal<Option<EditorTab>>,
    }

    impl CurrentTab {
        fn get(&self) -> Option<EditorTab> {
            self.memo.get()
        }

        fn set(&self, new_tab: Option<EditorTab>) {
            if let Some(tab) = new_tab {
                if let Some(index) = self.active_index.get() {
                    let mut tabs_vec = self.tabs.get();
                    if index < tabs_vec.len() {
                        // 🚀 MEMORY FIX: Trim token cache before saving tab state
                        // This prevents unbounded memory growth from syntax highlighting
                        let mut updated_tab = tab;
                        let visible_start = (updated_tab.scroll_top / crate::core::canvas_renderer::LINE_HEIGHT) as usize;
                        let visible_count = 50; // Estimated visible lines, can be adjusted
                        updated_tab.buffer.trim_token_cache(visible_start, visible_start + visible_count, 20);

                        tabs_vec[index] = updated_tab;
                        self.tabs.set(tabs_vec);
                    }
                }
            }
        }
    }

    let current_tab = CurrentTab {
        tabs,
        active_index: active_tab_index,
        memo: current_tab_memo,
    };

    // ===== Keyboard Event Parsing Helper =====
    // Convert browser keyboard event to EditorAction
    fn parse_keyboard_event(ev: &leptos::ev::KeyboardEvent) -> crate::core::actions::EditorAction {
        use crate::core::actions::{Direction, EditorAction};

        let key = ev.key();
        let ctrl = ev.ctrl_key();
        let meta = ev.meta_key();
        let shift = ev.shift_key();

        match key.as_str() {
            // ===== Ctrl/Cmd Combinations =====
            "z" if (ctrl || meta) && shift => EditorAction::Redo,
            "z" if ctrl || meta => EditorAction::Undo,
            "y" if ctrl || meta => EditorAction::Redo,
            "c" if ctrl || meta => EditorAction::Copy,
            "x" if ctrl || meta => EditorAction::Cut,
            "v" if ctrl || meta => EditorAction::Paste,
            "a" if ctrl || meta => EditorAction::SelectAll,
            "s" if ctrl || meta => EditorAction::Save,

            // ===== Arrow Keys =====
            "ArrowLeft" if shift => EditorAction::ExtendSelection(Direction::Left),
            "ArrowLeft" => EditorAction::MoveCursor(Direction::Left),
            "ArrowRight" if shift => EditorAction::ExtendSelection(Direction::Right),
            "ArrowRight" => EditorAction::MoveCursor(Direction::Right),
            "ArrowUp" if shift => EditorAction::ExtendSelection(Direction::Up),
            "ArrowUp" => EditorAction::MoveCursor(Direction::Up),
            "ArrowDown" if shift => EditorAction::ExtendSelection(Direction::Down),
            "ArrowDown" => EditorAction::MoveCursor(Direction::Down),

            // ===== Home/End =====
            "Home" if shift => EditorAction::ExtendToLineStart,
            "Home" => EditorAction::MoveToLineStart,
            "End" if shift => EditorAction::ExtendToLineEnd,
            "End" => EditorAction::MoveToLineEnd,

            // ===== Special Keys =====
            "Enter" => EditorAction::NewLine,
            "Backspace" => EditorAction::Backspace,
            "Delete" => EditorAction::Delete,
            "PageUp" => EditorAction::PageUp,
            "PageDown" => EditorAction::PageDown,

            // ===== Single Character Input =====
            k if k.len() == 1 && !ctrl && !meta => {
                EditorAction::InsertChar(k.chars().next().unwrap())
            }

            // ===== Unknown Key =====
            _ => EditorAction::None,
        }
    }

    // ===== Clipboard Helper Functions =====
    // These require access to clipboard_text signal, so they're separate from execute_action()

    fn handle_copy_action(tab: &EditorTab, clipboard_text: &RwSignal<String>) {
        if let Some(selected_text) = tab.get_selected_text() {
            clipboard_text.set(selected_text);
            leptos::logging::log!("Copied selection");
        } else if let Some(line_text) = tab.buffer.line(tab.cursor_line) {
            clipboard_text.set(line_text.to_string());
            leptos::logging::log!("Copied line");
        }
    }

    fn handle_cut_action(tab: &mut EditorTab, clipboard_text: &RwSignal<String>) -> bool {
        tab.save_undo_state();

        if let Some(selected_text) = tab.get_selected_text() {
            clipboard_text.set(selected_text);
            tab.delete_selection();
            leptos::logging::log!("Cut selection");
            true // Buffer modified
        } else if let Some(line_text) = tab.buffer.line(tab.cursor_line) {
            clipboard_text.set(line_text.to_string());
            let line_start = tab.buffer.line_to_char(tab.cursor_line);
            let line_end = line_start + line_text.len();
            tab.buffer.remove(line_start, line_end);
            tab.cursor_col = 0;
            leptos::logging::log!("Cut line");
            true // Buffer modified
        } else {
            false // No modification
        }
    }

    fn handle_paste_action(tab: &mut EditorTab, clipboard_text: &RwSignal<String>) -> bool {
        let text_to_paste = clipboard_text.get();
        if text_to_paste.is_empty() {
            return false;
        }

        tab.execute_action(&crate::core::actions::EditorAction::InsertText(text_to_paste))
    }

    // キーボードイベントハンドラー
    let on_keydown = move |ev: leptos::ev::KeyboardEvent| {
        leptos::logging::log!("🎹 on_keydown called: key={}, keyCode={}, composing={}",
            ev.key(), ev.key_code(), ev.is_composing());

        // 🎯 FOCUS GUARD: Only handle keyboard events if editor has focus
        // Prevents conflicts with CommandPalette, dialogs, and other modals
        if !focus_stack.with(|stack| stack.should_handle_keys(FocusLayer::Editor)) {
            leptos::logging::log!("⛔ Editor does not have focus, ignoring keyboard event");
            return;
        }

        // IME入力中は何もしない
        if ev.is_composing() || ev.key_code() == 229 {
            leptos::logging::log!("🇯🇵 IME composing detected, skipping");
            return;
        }

        ev.prevent_default(); // ブラウザのデフォルト動作を阻止

        let Some(mut tab) = current_tab.get() else {
            return;
        };

        let key = ev.key();
        let mut buffer_changed = false;

        // ✅ LSP: Completion widget navigation (when active)
        if show_completion.get() {
            match key.as_str() {
                "ArrowDown" => {
                    completion_selected_index.update(|idx| {
                        let max = completion_items.get_untracked().len().saturating_sub(1);
                        *idx = (*idx + 1).min(max);
                    });
                    return;
                }
                "ArrowUp" => {
                    completion_selected_index.update(|idx| {
                        *idx = idx.saturating_sub(1);
                    });
                    return;
                }
                "Enter" | "Tab" => {
                    // Select current completion item
                    let selected_idx = completion_selected_index.get_untracked();
                    let items = completion_items.get_untracked();
                    if let Some(item) = items.get(selected_idx) {
                        let label = item.label.clone();
                        let char_idx = tab.buffer.line_to_char(tab.cursor_line) + tab.cursor_col;
                        tab.buffer.insert(char_idx, &label);
                        tab.cursor_col += label.len();
                        current_tab.set(Some(tab.clone()));
                        show_completion.set(false);
                        render_trigger.update(|v| *v += 1);
                    }
                    return;
                }
                "Escape" => {
                    show_completion.set(false);
                    return;
                }
                _ => {
                    // Close completion on other keys
                    show_completion.set(false);
                }
            }
        }

        // Ctrl/Cmd + Z (Undo)
        if (ev.ctrl_key() || ev.meta_key()) && key.as_str() == "z" {
            if tab.undo() {
                current_tab.set(Some(tab));
                render_trigger.update(|v| *v += 1);
                leptos::logging::log!("Undo executed");
            }
            return;
        }

        // Ctrl/Cmd + Y (Redo) または Ctrl/Cmd + Shift + Z
        if ((ev.ctrl_key() || ev.meta_key()) && key.as_str() == "y") ||
           ((ev.ctrl_key() || ev.meta_key()) && ev.shift_key() && key.as_str() == "Z") {
            if tab.redo() {
                current_tab.set(Some(tab));
                render_trigger.update(|v| *v += 1);
                leptos::logging::log!("Redo executed");
            }
            return;
        }

        // ✅ LSP: Ctrl/Cmd + Space (Trigger Code Completion)
        if (ev.ctrl_key() || ev.meta_key()) && key.as_str() == " " {
            ev.prevent_default(); // Prevent default space behavior

            leptos::logging::log!("🎯 Ctrl/Cmd+Space pressed - requesting completions");

            let position = Position::new(tab.cursor_line, tab.cursor_col);
            let lsp_client = lsp.get_untracked();

            spawn_local(async move {
                leptos::logging::log!("🔍 LSP: Requesting completions at {:?}", position);
                match lsp_client.request_completions(position).await {
                    Ok(items) if !items.is_empty() => {
                        completion_items.set(items.clone());
                        show_completion.set(true);
                        leptos::logging::log!("✅ LSP: Completion widget shown with {} items", items.len());
                    }
                    Ok(_) => {
                        leptos::logging::log!("⚠️ LSP: No completions available");
                    }
                    Err(e) => {
                        leptos::logging::log!("❌ LSP: Completion error: {:?}", e);
                    }
                }
            });
            return;
        }

        // Ctrl/Cmd + S (Save)
        if (ev.ctrl_key() || ev.meta_key()) && key.as_str() == "s" {
            let file_path = tab.file_path.clone();
            let content = tab.buffer.to_string();

            // Tauri commandを使ってファイル保存
            wasm_bindgen_futures::spawn_local(async move {
                #[cfg(target_arch = "wasm32")]
                {
                    use wasm_bindgen::prelude::*;
                    #[wasm_bindgen]
                    extern "C" {
                        #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
                        async fn invoke(cmd: &str, args: JsValue) -> JsValue;
                    }

                    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
                        "path": file_path,
                        "contents": content,
                    })).unwrap();

                    match invoke("write_file", args).await {
                        _ => {
                            leptos::logging::log!("File saved: {}", file_path);
                        }
                    }
                }
            });

            current_tab.set(Some(tab));
            return;
        }

        // Ctrl/Cmd + A (Select All)
        if (ev.ctrl_key() || ev.meta_key()) && key.as_str() == "a" {
            tab.selection_start = Some((0, 0));
            let last_line = tab.buffer.len_lines().saturating_sub(1);
            let last_col = tab.buffer.line(last_line)
                .map(|s| s.trim_end_matches('\n').chars().count())
                .unwrap_or(0);
            tab.selection_end = Some((last_line, last_col));
            current_tab.set(Some(tab));
            render_trigger.update(|v| *v += 1);
            return;
        }

        // Ctrl/Cmd + C (Copy) - 選択範囲または行全体をコピー
        if (ev.ctrl_key() || ev.meta_key()) && key.as_str() == "c" {
            if let Some(selected_text) = tab.get_selected_text() {
                clipboard_text.set(selected_text.clone());
                leptos::logging::log!("Copied selection: {}", selected_text);
            } else if let Some(line_text) = tab.buffer.line(tab.cursor_line) {
                clipboard_text.set(line_text.to_string());
                leptos::logging::log!("Copied line: {}", line_text);
            }
            current_tab.set(Some(tab));
            return;
        }

        // Ctrl/Cmd + V (Paste) - カーソル位置または選択範囲に貼り付け
        if (ev.ctrl_key() || ev.meta_key()) && key.as_str() == "v" {
            let text_to_paste = clipboard_text.get();
            if !text_to_paste.is_empty() {
                tab.save_undo_state();

                // 選択範囲があれば先に削除
                if tab.has_selection() {
                    tab.delete_selection();
                }

                let char_idx = tab.buffer.line_to_char(tab.cursor_line) + tab.cursor_col;
                tab.buffer.insert(char_idx, &text_to_paste);

                // カーソルを貼り付けたテキストの末尾に移動
                let chars_inserted = text_to_paste.chars().count();
                tab.cursor_col += chars_inserted;
                buffer_changed = true;
                leptos::logging::log!("Pasted: {}", text_to_paste);
            }
            current_tab.set(Some(tab));
            render_trigger.update(|v| *v += 1);
            return;
        }

        // Ctrl/Cmd + X (Cut) - 選択範囲または行全体をカット
        if (ev.ctrl_key() || ev.meta_key()) && key.as_str() == "x" {
            tab.save_undo_state();

            if let Some(selected_text) = tab.get_selected_text() {
                clipboard_text.set(selected_text);
                tab.delete_selection();
                buffer_changed = true;
                leptos::logging::log!("Cut selection");
            } else if let Some(line_text) = tab.buffer.line(tab.cursor_line) {
                clipboard_text.set(line_text.to_string());
                let line_start = tab.buffer.line_to_char(tab.cursor_line);
                let line_end = line_start + line_text.len();
                tab.buffer.remove(line_start, line_end);
                tab.cursor_col = 0;
                buffer_changed = true;
                leptos::logging::log!("Cut line");
            }
            current_tab.set(Some(tab));
            render_trigger.update(|v| *v += 1);
            return;
        }

        match key.as_str() {
            // 英数字・記号の入力
            k if k.len() == 1 && !ev.ctrl_key() && !ev.meta_key() => {
                tab.save_undo_state();

                // 選択範囲があれば先に削除
                if tab.has_selection() {
                    tab.delete_selection();
                }

                let char_idx = tab.buffer.line_to_char(tab.cursor_line) + tab.cursor_col;
                tab.buffer.insert(char_idx, k);
                tab.cursor_col += 1;
                buffer_changed = true;
                leptos::logging::log!("Inserted: '{}' at line={}, col={}", k, tab.cursor_line, tab.cursor_col - 1);

                // ✅ LSP: Auto-trigger completion on '.' or ':'
                if k == "." || k == ":" {
                    let position = Position::new(tab.cursor_line, tab.cursor_col);
                    let lsp_client = lsp.get_untracked();

                    spawn_local(async move {
                        match lsp_client.request_completions(position).await {
                            Ok(items) if !items.is_empty() => {
                                completion_items.set(items);
                                show_completion.set(true);
                                leptos::logging::log!("✅ LSP: Auto-completion triggered");
                            }
                            _ => {}
                        }
                    });
                }
            }

            // Backspace
            "Backspace" => {
                tab.save_undo_state();

                if tab.has_selection() {
                    tab.delete_selection();
                    buffer_changed = true;
                } else if tab.cursor_col > 0 {
                    // 同じ行内で削除
                    let char_idx = tab.buffer.line_to_char(tab.cursor_line) + tab.cursor_col - 1;
                    tab.buffer.remove(char_idx, char_idx + 1);
                    tab.cursor_col -= 1;
                    buffer_changed = true;
                } else if tab.cursor_line > 0 {
                    // 前の行と結合
                    let prev_line_len = tab.buffer.line(tab.cursor_line - 1)
                        .map(|s| s.trim_end_matches('\n').chars().count())
                        .unwrap_or(0);

                    let char_idx = tab.buffer.line_to_char(tab.cursor_line) - 1; // 改行文字
                    tab.buffer.remove(char_idx, char_idx + 1);
                    tab.cursor_line -= 1;
                    tab.cursor_col = prev_line_len;
                    buffer_changed = true;
                }
                leptos::logging::log!("Backspace: line={}, col={}", tab.cursor_line, tab.cursor_col);
            }

            // Delete
            "Delete" => {
                tab.save_undo_state();

                if tab.has_selection() {
                    tab.delete_selection();
                    buffer_changed = true;
                } else {
                    let line_len = tab.buffer.line(tab.cursor_line)
                        .map(|s| s.trim_end_matches('\n').chars().count())
                        .unwrap_or(0);

                    if tab.cursor_col < line_len {
                        // 同じ行内で削除
                        let char_idx = tab.buffer.line_to_char(tab.cursor_line) + tab.cursor_col;
                        tab.buffer.remove(char_idx, char_idx + 1);
                        buffer_changed = true;
                    } else if tab.cursor_line < tab.buffer.len_lines() - 1 {
                        // 次の行と結合
                        let char_idx = tab.buffer.line_to_char(tab.cursor_line) + tab.cursor_col;
                        tab.buffer.remove(char_idx, char_idx + 1);
                        buffer_changed = true;
                    }
                }
            }

            // Enter
            "Enter" => {
                tab.save_undo_state();

                // 選択範囲があれば先に削除
                if tab.has_selection() {
                    tab.delete_selection();
                }

                let char_idx = tab.buffer.line_to_char(tab.cursor_line) + tab.cursor_col;
                tab.buffer.insert(char_idx, "\n");
                tab.cursor_line += 1;
                tab.cursor_col = 0;
                buffer_changed = true;
                leptos::logging::log!("Enter: line={}, col={}", tab.cursor_line, tab.cursor_col);
            }

            // Home - 行頭に移動
            "Home" => {
                if ev.shift_key() {
                    // Shift+Home: 選択しながら行頭へ
                    if !tab.has_selection() {
                        tab.selection_start = Some((tab.cursor_line, tab.cursor_col));
                    }
                    tab.cursor_col = 0;
                    tab.selection_end = Some((tab.cursor_line, tab.cursor_col));
                } else {
                    tab.cursor_col = 0;
                    tab.clear_selection();
                }
            }

            // End - 行末に移動
            "End" => {
                let line_len = tab.buffer.line(tab.cursor_line)
                    .map(|s| s.trim_end_matches('\n').chars().count())
                    .unwrap_or(0);

                if ev.shift_key() {
                    // Shift+End: 選択しながら行末へ
                    if !tab.has_selection() {
                        tab.selection_start = Some((tab.cursor_line, tab.cursor_col));
                    }
                    tab.cursor_col = line_len;
                    tab.selection_end = Some((tab.cursor_line, tab.cursor_col));
                } else {
                    tab.cursor_col = line_len;
                    tab.clear_selection();
                }
            }

            // PageUp - 1ページ上へ
            "PageUp" => {
                let page_lines = 20; // 1ページ = 20行
                tab.cursor_line = tab.cursor_line.saturating_sub(page_lines);
                let line_len = tab.buffer.line(tab.cursor_line)
                    .map(|s| s.trim_end_matches('\n').chars().count())
                    .unwrap_or(0);
                tab.cursor_col = tab.cursor_col.min(line_len);
                if !ev.shift_key() {
                    tab.clear_selection();
                }
            }

            // PageDown - 1ページ下へ
            "PageDown" => {
                let page_lines = 20;
                tab.cursor_line = (tab.cursor_line + page_lines).min(tab.buffer.len_lines().saturating_sub(1));
                let line_len = tab.buffer.line(tab.cursor_line)
                    .map(|s| s.trim_end_matches('\n').chars().count())
                    .unwrap_or(0);
                tab.cursor_col = tab.cursor_col.min(line_len);
                if !ev.shift_key() {
                    tab.clear_selection();
                }
            }

            // 矢印キー - カーソル移動（Shiftキーで選択）
            "ArrowLeft" => {
                if ev.shift_key() {
                    // Shift+Left: 選択しながら左へ
                    if !tab.has_selection() {
                        tab.selection_start = Some((tab.cursor_line, tab.cursor_col));
                    }
                    if tab.cursor_col > 0 {
                        tab.cursor_col -= 1;
                    } else if tab.cursor_line > 0 {
                        tab.cursor_line -= 1;
                        tab.cursor_col = tab.buffer.line(tab.cursor_line)
                            .map(|s| s.trim_end_matches('\n').chars().count())
                            .unwrap_or(0);
                    }
                    tab.selection_end = Some((tab.cursor_line, tab.cursor_col));
                } else {
                    tab.clear_selection();
                    if tab.cursor_col > 0 {
                        tab.cursor_col -= 1;
                    } else if tab.cursor_line > 0 {
                        tab.cursor_line -= 1;
                        tab.cursor_col = tab.buffer.line(tab.cursor_line)
                            .map(|s| s.trim_end_matches('\n').chars().count())
                            .unwrap_or(0);
                    }
                }
            }

            "ArrowRight" => {
                let line_len = tab.buffer.line(tab.cursor_line)
                    .map(|s| s.trim_end_matches('\n').chars().count())
                    .unwrap_or(0);

                if ev.shift_key() {
                    // Shift+Right: 選択しながら右へ
                    if !tab.has_selection() {
                        tab.selection_start = Some((tab.cursor_line, tab.cursor_col));
                    }
                    if tab.cursor_col < line_len {
                        tab.cursor_col += 1;
                    } else if tab.cursor_line < tab.buffer.len_lines() - 1 {
                        tab.cursor_line += 1;
                        tab.cursor_col = 0;
                    }
                    tab.selection_end = Some((tab.cursor_line, tab.cursor_col));
                } else {
                    tab.clear_selection();
                    if tab.cursor_col < line_len {
                        tab.cursor_col += 1;
                    } else if tab.cursor_line < tab.buffer.len_lines() - 1 {
                        tab.cursor_line += 1;
                        tab.cursor_col = 0;
                    }
                }
            }

            "ArrowUp" => {
                if ev.shift_key() {
                    // Shift+Up: 選択しながら上へ
                    if !tab.has_selection() {
                        tab.selection_start = Some((tab.cursor_line, tab.cursor_col));
                    }
                    if tab.cursor_line > 0 {
                        tab.cursor_line -= 1;
                        let line_len = tab.buffer.line(tab.cursor_line)
                            .map(|s| s.trim_end_matches('\n').chars().count())
                            .unwrap_or(0);
                        tab.cursor_col = tab.cursor_col.min(line_len);
                    }
                    tab.selection_end = Some((tab.cursor_line, tab.cursor_col));
                } else {
                    tab.clear_selection();
                    if tab.cursor_line > 0 {
                        tab.cursor_line -= 1;
                        let line_len = tab.buffer.line(tab.cursor_line)
                            .map(|s| s.trim_end_matches('\n').chars().count())
                            .unwrap_or(0);
                        tab.cursor_col = tab.cursor_col.min(line_len);
                    }
                }
            }

            "ArrowDown" => {
                if ev.shift_key() {
                    // Shift+Down: 選択しながら下へ
                    if !tab.has_selection() {
                        tab.selection_start = Some((tab.cursor_line, tab.cursor_col));
                    }
                    if tab.cursor_line < tab.buffer.len_lines() - 1 {
                        tab.cursor_line += 1;
                        let line_len = tab.buffer.line(tab.cursor_line)
                            .map(|s| s.trim_end_matches('\n').chars().count())
                            .unwrap_or(0);
                        tab.cursor_col = tab.cursor_col.min(line_len);
                    }
                    tab.selection_end = Some((tab.cursor_line, tab.cursor_col));
                } else {
                    tab.clear_selection();
                    if tab.cursor_line < tab.buffer.len_lines() - 1 {
                        tab.cursor_line += 1;
                        let line_len = tab.buffer.line(tab.cursor_line)
                            .map(|s| s.trim_end_matches('\n').chars().count())
                            .unwrap_or(0);
                        tab.cursor_col = tab.cursor_col.min(line_len);
                    }
                }
            }

            _ => {
                leptos::logging::log!("Unhandled key: {}", key);
            }
        }

        // カーソルが見える範囲にスクロール調整
        if let Some(canvas) = canvas_ref.get() {
            let height = canvas.height() as f64;
            tab.scroll_into_view(height);
        }

        // タブを更新
        current_tab.set(Some(tab));

        // バッファが変更された場合、またはカーソルが移動した場合は再描画
        render_trigger.update(|v| *v += 1);
    };

    // IMEイベントハンドラー
    let on_composition_start = move |_ev: leptos::ev::CompositionEvent| {
        is_composing.set(true);
        leptos::logging::log!("IME composition started");
    };

    let on_composition_update = move |ev: leptos::ev::CompositionEvent| {
        if let Some(data) = ev.data() {
            composing_text.set(data);
            render_trigger.update(|v| *v += 1);
            leptos::logging::log!("IME composing: {}", composing_text.get());
        }
    };

    let on_composition_end = move |ev: leptos::ev::CompositionEvent| {
        is_composing.set(false);

        // ✅ FIX: ev.data()は空になることがあるため、IME inputの値を直接取得
        let data = if let Some(input) = ime_input_ref.get() {
            let value = input.value();
            leptos::logging::log!("🔍 compositionend: ev.data()={:?}, input.value()={}", ev.data(), value);
            value
        } else {
            ev.data().unwrap_or_default()
        };

        // 確定文字をバッファに挿入
        if !data.is_empty() {
            if let Some(mut tab) = current_tab.get() {
                let old_col = tab.cursor_col;
                let char_idx = tab.buffer.line_to_char(tab.cursor_line) + tab.cursor_col;
                tab.buffer.insert(char_idx, &data);
                tab.cursor_col += data.chars().count();
                leptos::logging::log!(
                    "✅ IME committed: '{}' at pos {}, cursor: {} -> {}",
                    data,
                    char_idx,
                    old_col,
                    tab.cursor_col
                );
                current_tab.set(Some(tab));
            }
        } else {
            leptos::logging::log!("⚠️ IME committed empty string, skipping");
        }

        composing_text.set(String::new());

        // IME inputの値をクリア（次の入力に備えて）
        if let Some(input) = ime_input_ref.get() {
            input.set_value("");
            let _ = input.focus();
        }

        render_trigger.update(|v| *v += 1);
    };


    // マウスクリックでカーソル配置（ドラッグ開始）
    let on_mousedown = move |ev: leptos::ev::MouseEvent| {
        leptos::logging::log!("🖱️ MOUSEDOWN EVENT FIRED");

        let Some(canvas) = canvas_ref.get() else {
            leptos::logging::log!("❌ Canvas ref not found");
            return;
        };

        let Some(mut tab) = current_tab.get() else {
            leptos::logging::log!("❌ Current tab not found");
            return;
        };

        let rect = canvas.get_bounding_client_rect();
        let x = ev.client_x() as f64 - rect.left();
        let y = ev.client_y() as f64 - rect.top();
        leptos::logging::log!("🖱️ Click position: x={}, y={}", x, y);

        // カーソル位置を計算
        if let Ok(renderer) = CanvasRenderer::new((*canvas).clone().unchecked_into()) {
            // ガター幅を超えているか確認
            if x > renderer.gutter_width() {
                let text_x = x - renderer.gutter_width() - 15.0;
                let clicked_line = ((y + tab.scroll_top) / LINE_HEIGHT).floor() as usize;

                // 行範囲内に制限
                let line = clicked_line.min(tab.buffer.len_lines().saturating_sub(1));

                // 行のテキストを取得
                let line_text = tab.buffer.line(line)
                    .map(|s| s.trim_end_matches('\n').to_string())
                    .unwrap_or_default();

                let line_len = line_text.chars().count();

                // 列位置を計算（measureText()を使って正確に）
                let col = find_column_from_x_position(&renderer, &line_text, text_x);

                // ✅ Cmd+Click (or Ctrl+Click) for go-to-definition
                if ev.meta_key() || ev.ctrl_key() {
                    leptos::logging::log!("🔍 Cmd/Ctrl+Click detected at line={}, col={}", line, col);

                    // ✅ FIX: Check global LSP initialization status first
                    let global_lsp_init = lsp_initialized.get();
                    leptos::logging::log!("🔍 DEBUG: lsp_initialized={}", global_lsp_init);

                    if !global_lsp_init {
                        leptos::logging::log!("❌ LSP: Not initialized yet, please wait for project to load");
                        return;
                    }

                    leptos::logging::log!("✅ LSP: Initialized, proceeding with goto_definition");

                    // Update cursor position first
                    tab.cursor_line = line;
                    tab.cursor_col = col.min(line_len);
                    current_tab.set(Some(tab.clone()));
                    render_trigger.update(|v| *v += 1);

                    // Call LSP goto_definition
                    let lsp_client = lsp.get_untracked();
                    let current_file = tab.file_path.clone();

                    let position = Position::new(line, col);

                    spawn_local(async move {
                        leptos::logging::log!("🔍 LSP: Spawned async task, calling goto_definition at {:?}", position);
                        leptos::logging::log!("🔍 LSP: About to call lsp_client.goto_definition()...");

                        match lsp_client.goto_definition(position).await {
                            Ok(location) => {
                                leptos::logging::log!("✅ LSP: Definition found at {}:{}:{}", location.uri, location.line, location.column);

                                // Check if we need to open a different file
                                if location.uri != current_file {
                                    leptos::logging::log!("📂 LSP: Opening different file: {}", location.uri);

                                    // Read the target file content
                                    match crate::tauri_bindings::read_file(&location.uri).await {
                                        Ok(content) => {
                                            // Check if tab already exists
                                            let existing_tab_index = tabs.with_untracked(|tabs_vec| {
                                                tabs_vec.iter().position(|t| t.file_path == location.uri)
                                            });

                                            if let Some(existing_idx) = existing_tab_index {
                                                // Switch to existing tab
                                                leptos::logging::log!("🔍 LSP: Switching to existing tab at index: {}", existing_idx);
                                                active_tab_index.set(Some(existing_idx));

                                                // Update cursor position
                                                tabs.update(|tabs_vec| {
                                                    if let Some(tab) = tabs_vec.get_mut(existing_idx) {
                                                        tab.cursor_line = location.line;
                                                        tab.cursor_col = location.column;
                                                        tab.scroll_into_view(canvas.client_height() as f64);
                                                    }
                                                });
                                            } else {
                                                // Create new tab
                                                leptos::logging::log!("🔍 LSP: Creating new tab for: {}", location.uri);
                                                let new_index = tabs.with_untracked(|tabs_vec| tabs_vec.len());

                                                tabs.update(|tabs_vec| {
                                                    let mut new_tab = EditorTab::new(location.uri.clone(), content);
                                                    new_tab.cursor_line = location.line;
                                                    new_tab.cursor_col = location.column;
                                                    tabs_vec.push(new_tab);
                                                });

                                                active_tab_index.set(Some(new_index));

                                                // Scroll to cursor after tab is created
                                                tabs.update(|tabs_vec| {
                                                    if let Some(tab) = tabs_vec.get_mut(new_index) {
                                                        tab.scroll_into_view(canvas.client_height() as f64);
                                                    }
                                                });
                                            }

                                            render_trigger.update(|v| *v += 1);
                                            leptos::logging::log!("✅ LSP: Jumped to {}:{}:{}", location.uri, location.line, location.column);
                                        }
                                        Err(e) => {
                                            leptos::logging::error!("❌ LSP: Failed to read file {}: {}", location.uri, e);
                                        }
                                    }
                                } else {
                                    // Same file, just move cursor
                                    tabs.update(|tabs_vec| {
                                        if let Some(active_idx) = active_tab_index.get_untracked() {
                                            if let Some(tab) = tabs_vec.get_mut(active_idx) {
                                                tab.cursor_line = location.line;
                                                tab.cursor_col = location.column;
                                                tab.scroll_into_view(canvas.client_height() as f64);
                                                leptos::logging::log!("✅ LSP: Jumped to same file at {}:{}", location.line, location.column);
                                            }
                                        }
                                    });
                                    render_trigger.update(|v| *v += 1);
                                }
                            }
                            Err(e) => {
                                leptos::logging::error!("❌ LSP: Goto definition failed: {}", e);
                            }
                        }
                    });

                    // Don't start drag selection for Cmd+Click
                    return;
                }

                tab.cursor_line = line;
                tab.cursor_col = col.min(line_len);

                // ドラッグ開始
                is_dragging.set(true);
                tab.selection_start = Some((line, col.min(line_len)));
                tab.selection_end = Some((line, col.min(line_len)));

                leptos::logging::log!("🖱️ Mouse down: line={}, col={}, selection_start=({}, {})",
                    line, col, line, col.min(line_len));

                current_tab.set(Some(tab));
                render_trigger.update(|v| *v += 1);
            }
        }
    };

    // マウス移動（ドラッグ中）
    let on_mousemove = move |ev: leptos::ev::MouseEvent| {
        let Some(canvas) = canvas_ref.get() else {
            return;
        };

        let Some(mut tab) = current_tab.get() else {
            return;
        };

        let rect = canvas.get_bounding_client_rect();
        let x = ev.client_x() as f64 - rect.left();
        let y = ev.client_y() as f64 - rect.top();

        if let Ok(renderer) = CanvasRenderer::new((*canvas).clone().unchecked_into()) {
            if x > renderer.gutter_width() {
                let text_x = x - renderer.gutter_width() - 15.0;
                let clicked_line = ((y + tab.scroll_top) / LINE_HEIGHT).floor() as usize;
                let line = clicked_line.min(tab.buffer.len_lines().saturating_sub(1));

                // 行のテキストを取得
                let line_text = tab.buffer.line(line)
                    .map(|s| s.trim_end_matches('\n').to_string())
                    .unwrap_or_default();

                let line_len = line_text.chars().count();

                // 列位置を計算（measureText()を使って正確に）
                let col = find_column_from_x_position(&renderer, &line_text, text_x);

                // ✅ LSP: Cmd+Hover underline
                // PERFORMANCE: Only update render_trigger when underline state changes
                // This prevents unnecessary redraws when Cmd is pressed/released without movement
                let is_cmd_pressed = ev.meta_key() || ev.ctrl_key();

                if is_cmd_pressed && !is_dragging.get() {
                    // Find symbol boundaries at cursor position
                    let new_underline = find_symbol_at_position(&line_text, col)
                        .map(|(start_col, end_col)| (line, start_col, end_col));

                    // Only update if underline position/state changed
                    let current_underline = hover_symbol_underline.get_untracked();
                    if current_underline != new_underline {
                        hover_symbol_underline.set(new_underline);
                        render_trigger.update(|v| *v += 1);
                    }
                } else {
                    // Clear underline when Cmd is released, but only if it was set
                    // Use update() to check and clear atomically
                    let was_set = hover_symbol_underline.try_update(|underline| {
                        if underline.is_some() {
                            *underline = None;
                            true
                        } else {
                            false
                        }
                    }).unwrap_or(false);

                    if was_set {
                        render_trigger.update(|v| *v += 1);
                    }
                }

                // ✅ LSP: Handle dragging vs hovering
                if is_dragging.get() {
                    // Dragging - update selection
                    tab.cursor_line = line;
                    tab.cursor_col = col.min(line_len);
                    tab.selection_end = Some((line, col.min(line_len)));

                    leptos::logging::log!("🖱️ Mouse move: line={}, col={}, selection_end=({}, {})",
                        line, col, line, col.min(line_len));

                    current_tab.set(Some(tab));
                    render_trigger.update(|v| *v += 1);
                } else if !show_completion.get() {
                    // ✅ LSP: Hovering - request hover info (debounced)
                    let position = canvas_pixel_to_lsp_position(&renderer, x, y, tab.scroll_top, &tab.buffer);
                    let lsp_client = lsp.get_untracked();

                    // Increment timer for debounce cancellation
                    let timer_id = hover_debounce_timer.get() + 1;
                    hover_debounce_timer.set(timer_id);

                    spawn_local(async move {
                        // Debounce: wait 300ms
                        #[cfg(target_arch = "wasm32")]
                        {
                            use wasm_bindgen_futures::JsFuture;
                            use web_sys::window;
                            if let Some(win) = window() {
                                let promise = js_sys::Promise::new(&mut |resolve, _reject| {
                                    let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 300);
                                });
                                let _ = JsFuture::from(promise).await;
                            }
                        }

                        // Check if timer was cancelled by another mousemove
                        if hover_debounce_timer.get_untracked() != timer_id {
                            return;
                        }

                        match lsp_client.request_hover(position).await {
                            Ok(Some(info)) => {
                                hover_info.set(Some(info));
                                hover_pixel_position.set(Some((x, y)));
                                leptos::logging::log!("✅ LSP: Hover info received");
                            }
                            _ => {
                                hover_info.set(None);
                                hover_pixel_position.set(None);
                            }
                        }
                    });
                }
            }
        }
    };

    // マウスボタンを離す（ドラッグ終了）
    let on_mouseup = move |_ev: leptos::ev::MouseEvent| {
        leptos::logging::log!("🖱️ Mouse up, is_dragging was: {}", is_dragging.get());
        is_dragging.set(false);

        // 選択範囲が1文字未満なら選択解除
        if let Some(tab) = current_tab.get() {
            if let (Some(start), Some(end)) = (tab.selection_start, tab.selection_end) {
                leptos::logging::log!("🖱️ Selection on mouseup: start={:?}, end={:?}", start, end);
                if start == end {
                    leptos::logging::log!("⚠️ Selection cleared (start == end)");
                    let mut tab = tab;
                    tab.clear_selection();
                    current_tab.set(Some(tab));
                    render_trigger.update(|v| *v += 1);
                } else {
                    leptos::logging::log!("✅ Selection kept (start != end)");
                }
            }
        }

        // ドラッグ終了後、IME inputに再フォーカス
        if let Some(input) = ime_input_ref.get() {
            use wasm_bindgen::JsCast;
            let input_clone = input.clone();
            let callback = wasm_bindgen::closure::Closure::once(move || {
                let _ = input_clone.focus();
                leptos::logging::log!("🔄 Re-focused IME input after mouseup");
            });
            let window = web_sys::window().unwrap();
            let _ = window.request_animation_frame(callback.as_ref().unchecked_ref());
            callback.forget();
        }
    };

    // ダブルクリックで単語選択
    let on_dblclick = move |ev: leptos::ev::MouseEvent| {
        ev.prevent_default();
        ev.stop_propagation();

        leptos::logging::log!("🖱️ DOUBLE CLICK EVENT FIRED");

        let Some(canvas) = canvas_ref.get() else {
            leptos::logging::log!("❌ Canvas ref not found");
            return;
        };

        let Some(mut tab) = current_tab.get() else {
            leptos::logging::log!("❌ Current tab not found");
            return;
        };

        let rect = canvas.get_bounding_client_rect();
        let x = ev.client_x() as f64 - rect.left();
        let y = ev.client_y() as f64 - rect.top();

        // カーソル位置を計算
        if let Ok(renderer) = CanvasRenderer::new((*canvas).clone().unchecked_into()) {
            // ガター幅を超えているか確認
            if x > renderer.gutter_width() {
                let text_x = x - renderer.gutter_width() - 15.0;
                let clicked_line = ((y + tab.scroll_top) / LINE_HEIGHT).floor() as usize;

                // 行範囲内に制限
                let line = clicked_line.min(tab.buffer.len_lines().saturating_sub(1));

                // 行のテキストを取得
                let line_text = tab.buffer.line(line)
                    .map(|s| s.trim_end_matches('\n').to_string())
                    .unwrap_or_default();

                let line_len = line_text.chars().count();

                // 列位置を計算（measureText()を使って正確に）
                let col = find_column_from_x_position(&renderer, &line_text, text_x);

                tab.cursor_line = line;
                tab.cursor_col = col.min(line_len);

                // 単語選択を実行
                tab.select_word_at_cursor();

                leptos::logging::log!("🖱️ Double click: line={}, col={}, selected word", line, col);

                current_tab.set(Some(tab));
                render_trigger.update(|v| *v += 1);
            }
        }

        // ドラッグ状態をリセット（ダブルクリック後にドラッグさせない）
        is_dragging.set(false);

        // IME inputに再フォーカス
        if let Some(input) = ime_input_ref.get() {
            let _ = input.focus();
        }
    };

    // ホイールでスクロール
    let on_wheel = move |ev: leptos::ev::WheelEvent| {
        ev.prevent_default();

        let Some(mut tab) = current_tab.get() else {
            return;
        };

        let Some(canvas) = canvas_ref.get() else {
            return;
        };

        // スクロール量（1行 = LINE_HEIGHT）
        let delta = ev.delta_y();
        let scroll_lines = (delta / LINE_HEIGHT).round();

        // ✅ FIX: 新しいスクロール位置を計算
        let new_scroll = tab.scroll_top + scroll_lines * LINE_HEIGHT;

        // ✅ FIX: Canvasの実際のクライアント高さを取得
        let canvas_height = canvas.client_height() as f64;
        let total_lines = tab.buffer.len_lines();
        let content_height = total_lines as f64 * LINE_HEIGHT;

        // 🎨 IntelliJ-style: スクロール範囲を拡大して最終行を画面中央に配置可能にする
        // "Scroll past end" 機能 - 最終行を読みやすい位置に配置できる
        // コンテンツがビューポートより小さい場合は0
        let max_scroll = (content_height - canvas_height / 2.0).max(0.0);

        // ✅ FIX: スクロール位置を0～max_scrollの範囲内に制限
        tab.scroll_top = new_scroll.max(0.0).min(max_scroll);

        current_tab.set(Some(tab));
        render_trigger.update(|v| *v += 1);
    };

    // Canvasのリサイズとレンダリング
    Effect::new(move |_| {
        // render_triggerに依存して、変更時に再描画
        let _ = render_trigger.get();

        let Some(canvas) = canvas_ref.get() else {
            leptos::logging::log!("❌ Canvas ref not available");
            return;
        };

        // Canvas の親要素(.berry-editor-pane)のサイズを取得
        let Some(parent) = canvas.parent_element() else {
            leptos::logging::log!("❌ Canvas parent not available");
            return;
        };

        let rect = parent.get_bounding_client_rect();
        let mut width = rect.width();
        let mut height = rect.height();

        // ✅ 高さが0の場合は、フォールバックとして親要素から取得を試みる
        if height <= 0.0 {
            leptos::logging::log!("⚠️ Parent height is 0, trying grandparent...");

            if let Some(grandparent) = parent.parent_element() {
                let gp_rect = grandparent.get_bounding_client_rect();
                leptos::logging::log!(
                    "📏 Grandparent (.berry-editor-main): {}x{}, class: {}",
                    gp_rect.width(),
                    gp_rect.height(),
                    grandparent.class_name()
                );

                if gp_rect.height() > 0.0 {
                    height = gp_rect.height();
                    leptos::logging::log!("✅ Using grandparent height: {}", height);
                }
            }

            // それでも0なら、最低限の高さを確保
            if height <= 0.0 {
                height = 500.0; // フォールバック高さ
                leptos::logging::log!("⚠️ Using fallback height: {}", height);
            }
        }

        leptos::logging::log!(
            "📏 Final canvas size: {}x{}",
            width,
            height
        );

        // サイズチェック（フォールバック後もまだ無効なら）
        if width <= 0.0 || height <= 0.0 {
            leptos::logging::log!("❌ Invalid canvas size after fallback: {}x{}", width, height);
            return;
        }

        // ✅ 「黄金の組み合わせ」ステップ2: 内部バッファサイズを DPR 倍に設定
        // Retinaディスプレイ対応: devicePixelRatioを取得（Macなら 2.0）
        let window = web_sys::window().expect("no global window");
        let dpr = window.device_pixel_ratio();

        leptos::logging::log!(
            "✅ Canvas resize: CSS={}x{}, DPR={}, Physical={}x{}",
            width,
            height,
            dpr,
            (width * dpr) as u32,
            (height * dpr) as u32
        );

        // ✅ Canvas要素をHtmlCanvasElementにキャスト
        let canvas_el: HtmlCanvasElement = (*canvas).clone().unchecked_into();

        // 📝 Note: Tailwind の w-full h-full が既にCSSサイズを設定しているが、
        // ResizeObserver のタイミング問題で明示的に再設定（念のため）
        use wasm_bindgen::JsCast;
        let html_el: &web_sys::HtmlElement = canvas_el.as_ref();
        let _ = html_el
            .style()
            .set_property("width", &format!("{}px", width));
        let _ = html_el
            .style()
            .set_property("height", &format!("{}px", height));

        // ✅ 重要: 内部バッファを物理ピクセルサイズに設定（これが「にじみ」を消す鍵）
        // 例: 表示サイズ 1000×600px、DPR=2.0 → 内部バッファ 2000×1200px
        canvas_el.set_width((width * dpr) as u32);
        canvas_el.set_height((height * dpr) as u32);

        // 📝 仕組み:
        // 1. Tailwind: 「箱のサイズ」を制御 (w-full h-full) → 1000×600px
        // 2. ここ: 「中身の密度」を制御 (set_width/height) → 2000×1200px
        // 3. CanvasRenderer: 描画座標系を調整 (set_transform) → 論理座標で描画
        // → 結果: Retinaディスプレイで文字が「クッキリ」表示される！

        // レンダリング
        let tab_data = current_tab.get();
        if tab_data.is_none() {
            leptos::logging::log!("⚠️ No tab data available for rendering");
            return;
        }

        if let Some(mut tab) = tab_data {
            leptos::logging::log!(
                "🎨 Rendering tab: {} lines, cursor at ({}, {})",
                tab.buffer.len_lines(),
                tab.cursor_line,
                tab.cursor_col
            );

            if let Ok(renderer) = CanvasRenderer::new(canvas_el) {
                // Canvas全体をクリア
                renderer.clear(width as f64, height as f64);

                // 可視範囲の行を計算
                let start_line = (tab.scroll_top / LINE_HEIGHT).floor() as usize;
                let visible_lines = (height as f64 / LINE_HEIGHT).ceil() as usize + 1;
                let end_line = (start_line + visible_lines).min(tab.buffer.len_lines());

                // 🎨 ABSOLUTE BEAUTY: 洗練されたガター（グラデーション、影、階層化された行番号）
                renderer.draw_refined_gutter(start_line, end_line, tab.cursor_line, height as f64);

                // Git差分インジケータを描画（IntelliJ風の色付きバー）
                // TODO: 実際のGit統合時に、ここで本物のGit statusを渡す
                renderer.draw_git_diff_indicators(
                    |_line_num| crate::core::canvas_renderer::GitLineStatus::Unmodified,
                    start_line,
                    end_line,
                    tab.scroll_top
                );

                // 選択範囲を描画（テキストの背景として）
                if tab.has_selection() {
                    if let (Some((start_line, start_col)), Some((end_line, end_col))) =
                        (tab.selection_start, tab.selection_end) {
                        leptos::logging::log!("🎨 Drawing selection: ({}, {}) to ({}, {})", start_line, start_col, end_line, end_col);

                        // 行番号から行のテキストを取得するクロージャを作成
                        let buffer = &tab.buffer;
                        let get_line_text = |line_num: usize| -> String {
                            buffer
                                .line(line_num)
                                .map(|s| s.trim_end_matches('\n').to_string())
                                .unwrap_or_default()
                        };

                        // 選択範囲の正規化（逆方向選択に対応）
                        let (norm_start_line, norm_start_col, norm_end_line, norm_end_col) =
                            if start_line > end_line || (start_line == end_line && start_col > end_col) {
                                // 逆方向選択 - 座標を入れ替える
                                (end_line, end_col, start_line, start_col)
                            } else {
                                // 順方向選択 - そのまま
                                (start_line, start_col, end_line, end_col)
                            };

                        renderer.draw_selection(
                            norm_start_line,
                            norm_start_col,
                            norm_end_line,
                            norm_end_col,
                            tab.scroll_top,
                            get_line_text,
                        );
                    }
                }

                // 🎨 アクティブ行のハイライト（IntelliJ風の微妙な背景色）
                renderer.draw_active_line_highlight(tab.cursor_line, tab.scroll_top, width as f64);

                // テキスト行を描画（シンタックスハイライト付き）
                let theme = EditorTheme::current();
                let language = tab.language.as_deref(); // Option<String> -> Option<&str>
                for line_num in start_line..end_line {
                    // Ropeから行のテキストを取得（改行を除く）
                    let line_text = tab
                        .buffer
                        .line(line_num)
                        .map(|s| s.trim_end_matches('\n').to_string())
                        .unwrap_or_default();

                    let y_offset = (line_num - start_line) as f64 * LINE_HEIGHT;
                    // 🚀 PERFORMANCE: Pass buffer for token caching
                    renderer.draw_line_highlighted(&mut tab.buffer, line_num, y_offset, &line_text, theme, language);
                }

                // カーソルを描画（現在行のテキストを渡す）
                // ✅ FIX: 改行を除いたテキストを渡す（改行があると文字数計算がずれる）
                let cursor_line_text = tab.buffer.line(tab.cursor_line)
                    .map(|s| s.trim_end_matches('\n').to_string())
                    .unwrap_or_default();

                // IME未確定文字列を取得
                let composing = composing_text.get();

                // IME組成中は、仮想的なテキスト（確定文字+未確定文字）を作成してカーソル位置を計算
                let (virtual_line_text, cursor_col_display) = if !composing.is_empty() {
                    // 未確定文字列がある場合、カーソル位置に挿入した仮想テキストを作る
                    let before: String = cursor_line_text.chars().take(tab.cursor_col).collect();
                    let after: String = cursor_line_text.chars().skip(tab.cursor_col).collect();
                    let virtual_text = format!("{}{}{}", before, composing, after);
                    let virtual_col = tab.cursor_col + composing.chars().count();
                    (virtual_text, virtual_col)
                } else {
                    (cursor_line_text.clone(), tab.cursor_col)
                };

                leptos::logging::log!(
                    "🎯 Drawing cursor: line={}, col={} (display_col={}), composing='{}', line_text='{}' (len={})",
                    tab.cursor_line,
                    tab.cursor_col,
                    cursor_col_display,
                    &composing,
                    &cursor_line_text,
                    cursor_line_text.chars().count()
                );

                // アニメーション用の前回行のテキストを取得
                let prev_line_text = tab.buffer.line(tab.prev_cursor_line)
                    .map(|s| s.trim_end_matches('\n').to_string())
                    .unwrap_or_default();

                // カーソルを描画（composing中は未確定文字列の後ろに表示）
                // 🎨 IntelliJ-style smooth animation with 100ms easing
                renderer.draw_cursor(
                    tab.cursor_line,
                    cursor_col_display,
                    tab.scroll_top,
                    &virtual_line_text,
                    tab.prev_cursor_line,
                    tab.prev_cursor_col,
                    tab.cursor_move_timestamp,
                    &prev_line_text
                );

                // ✅ LSP: Cmd+Hover underline
                if let Some((line, start_col, end_col)) = hover_symbol_underline.get() {
                    if let Some(line_text) = tab.buffer.line(line) {
                        renderer.draw_symbol_underline(
                            line,
                            start_col,
                            end_col,
                            tab.scroll_top,
                            &line_text.trim_end_matches('\n')
                        );
                    }
                }

                // IME未確定文字列を描画（あれば）
                if !composing.is_empty() {
                    // 全角文字を考慮してカーソル位置までの実際の幅を測定
                    let text_before_cursor: String = cursor_line_text
                        .chars()
                        .take(tab.cursor_col)
                        .collect();
                    let x = renderer.gutter_width() + 15.0
                        + renderer.measure_text(&text_before_cursor);
                    let y = tab.cursor_line as f64 * LINE_HEIGHT - tab.scroll_top + 15.0;

                    // 未確定文字列をカーソル位置から描画（灰色）
                    renderer.draw_text_at(x, y, &composing, "#808080");

                    leptos::logging::log!("Drew composing text '{}' at ({}, {})", composing, x, y);
                }

                // カーソル位置を計算（IME用）- 全角文字対応
                // composing中は未確定文字列の後ろに配置
                let text_before_cursor_display: String = virtual_line_text
                    .chars()
                    .take(cursor_col_display)
                    .collect();
                let cursor_pixel_x = renderer.gutter_width() + 15.0
                    + renderer.measure_text(&text_before_cursor_display);
                let cursor_pixel_y = tab.cursor_line as f64 * LINE_HEIGHT - tab.scroll_top;

                cursor_x.set(cursor_pixel_x);
                cursor_y.set(cursor_pixel_y);

                leptos::logging::log!(
                    "Rendered {} lines ({}..{}), cursor at ({}, {})",
                    end_line - start_line,
                    start_line,
                    end_line,
                    cursor_pixel_x,
                    cursor_pixel_y
                );

                // 🚀 MEMORY OPTIMIZATION: Trim token cache after rendering
                // Keep visible lines + 20 line margin to balance cache hits vs memory usage
                // This prevents unbounded cache growth during long editing sessions
                const CACHE_MARGIN: usize = 20;
                tab.buffer.trim_token_cache(start_line, end_line, CACHE_MARGIN);

                #[cfg(debug_assertions)]
                leptos::logging::log!(
                    "Token cache size: {} lines (visible: {}-{}, margin: {})",
                    tab.buffer.token_cache_size(),
                    start_line,
                    end_line,
                    CACHE_MARGIN
                );

                // 🎨 Animation continuation: TODO - Re-enable when Performance API is available
                // Currently disabled due to web_sys API compatibility
                // if tab.cursor_move_timestamp > 0.0 {
                //     requestAnimationFrame for smooth animation
                // }
            }
        }
    });

    view! {
        <div
            node_ref=container_ref
            class="berry-editor-main"
            style="display: flex; flex-direction: column; flex: 1; min-width: 0; min-height: 0;"
        >
            // タブバー
            <div class="berry-editor-tabs" style="display: flex; background: var(--bg-tab-bar); border-bottom: 1px solid var(--bg-main); min-height: 35px; overflow-x: auto; scrollbar-width: thin; scrollbar-color: var(--icon-muted) var(--bg-sidebar);">
                {move || {
                    let tabs_vec = current_tab.tabs.get();
                    let active_index = current_tab.active_index.get();

                    if tabs_vec.is_empty() {
                        view! {
                            <div style="padding: 8px 16px; color: var(--tree-text-muted); font-size: 13px;">
                                "No file open"
                            </div>
                        }.into_any()
                    } else {
                        // 全てのタブを表示
                        tabs_vec.into_iter().enumerate().map(|(index, tab)| {
                            let is_active = Some(index) == active_index;
                            let file_name = tab.file_path
                                .split('/')
                                .last()
                                .unwrap_or(&tab.file_path)
                                .to_string();

                            let tab_class = if is_active { "berry-tab active" } else { "berry-tab" };
                            let bg_color = if is_active { "#1E1E1E" } else { "#2B2B2B" };

                            // file_pathをクローンしてクロージャーで使う（indexは古くなる可能性があるため）
                            let tab_path = tab.file_path.clone();
                            let tab_path_for_close = tab_path.clone();

                            view! {
                                <div
                                    class=tab_class
                                    on:click=move |_| {
                                        // クリック時に最新のindexを検索
                                        let tabs_vec = current_tab.tabs.get();
                                        if let Some(idx) = tabs_vec.iter().position(|t| t.file_path == tab_path) {
                                            current_tab.active_index.set(Some(idx));
                                        }
                                    }
                                    style=format!("
                                        display: flex;
                                        align-items: center;
                                        padding: 8px 12px 8px 16px;
                                        background: {};
                                        border-right: 1px solid #323232;
                                        color: #A9B7C6;
                                        font-size: 13px;
                                        font-family: 'JetBrains Mono', monospace;
                                        gap: 8px;
                                        cursor: pointer;
                                        flex-shrink: 0;
                                        white-space: nowrap;
                                    ", bg_color)
                                >
                                    <span>{file_name}</span>
                                    <button
                                        on:click=move |ev| {
                                            ev.stop_propagation();
                                            // タブを閉じる（file_pathで検索して削除）
                                            let mut tabs_vec = current_tab.tabs.get();
                                            if let Some(close_index) = tabs_vec.iter().position(|t| t.file_path == tab_path_for_close) {
                                                tabs_vec.remove(close_index);
                                                current_tab.tabs.set(tabs_vec.clone());

                                                // アクティブタブのインデックスを調整
                                                if tabs_vec.is_empty() {
                                                    // 全てのタブが閉じられた場合
                                                    current_tab.active_index.set(None);
                                                } else if Some(close_index) == current_tab.active_index.get() {
                                                    // 閉じたタブがアクティブだった場合、前のタブか次のタブをアクティブにする
                                                    let new_index = if close_index > 0 {
                                                        close_index - 1 // 前のタブ
                                                    } else {
                                                        0 // 最初のタブが閉じられた場合は新しい最初のタブ
                                                    };
                                                    // tabs_vec.len() は少なくとも 1 なので、安全に -1 できる
                                                    current_tab.active_index.set(Some(new_index.min(tabs_vec.len() - 1)));
                                                } else if let Some(active_idx) = current_tab.active_index.get() {
                                                    // 閉じたタブがアクティブタブより前にあった場合、インデックスを調整
                                                    if close_index < active_idx {
                                                        current_tab.active_index.set(Some(active_idx - 1));
                                                    }
                                                    // 閉じたタブがアクティブタブより後ろにある場合は調整不要
                                                }
                                            }
                                        }
                                        style="
                                            background: transparent;
                                            border: none;
                                            color: #606366;
                                            cursor: pointer;
                                            padding: 2px 4px;
                                            font-size: 16px;
                                            line-height: 1;
                                            display: flex;
                                            align-items: center;
                                            justify-content: center;
                                            border-radius: 2px;
                                        "
                                        onmouseover="this.style.background='#4E5157'; this.style.color='#A9B7C6';"
                                        onmouseout="this.style.background='transparent'; this.style.color='#606366';"
                                    >
                                        "×"
                                    </button>
                                </div>
                            }
                        }).collect_view().into_any()
                    }
                }}
            </div>

            // ✅ Tailwind: 親要素でレイアウトを固定、Canvas用の「箱」を作る
            <div class="berry-editor-pane flex-1 min-h-0 flex bg-berry-bg-main relative overflow-hidden">
                // ✅ Tailwind + Rust の黄金の組み合わせ:
                // - Tailwind (w-full h-full): 表示サイズを親に合わせる
                // - Rust (set_width/set_height): 内部解像度をDPRに合わせる
                // - image-rendering: ブラウザの補完をシャープにする
                <canvas
                    node_ref=canvas_ref
                    class="w-full h-full block touch-none outline-none cursor-text"
                    style="image-rendering: crisp-edges;"
                    on:mousedown=on_mousedown
                    on:mousemove=on_mousemove
                    on:mouseup=on_mouseup
                    on:dblclick=on_dblclick
                    on:wheel=on_wheel
                />

                // 隠しinput要素（IME候補ウィンドウの位置制御用）
                <input
                    node_ref=ime_input_ref
                    type="text"
                    on:compositionstart=on_composition_start
                    on:compositionupdate=on_composition_update
                    on:compositionend=on_composition_end
                    on:keydown=on_keydown
                    on:focus=move |_| {
                        leptos::logging::log!("✅ IME input FOCUSED");
                    }
                    on:blur=move |ev: leptos::ev::FocusEvent| {
                        leptos::logging::log!("❌ IME input BLURRED");
                        // 即座に再フォーカス（ただしIME composing中、ドラッグ中、またはエディタが非アクティブの場合は除く）
                        if !is_composing.get() && !is_dragging.get() && is_active.get() {
                            leptos::logging::log!("🔄 Editor is active, re-focusing...");
                            if let Some(input) = ime_input_ref.get() {
                                // Use requestAnimationFrame to avoid immediate blur loop
                                use wasm_bindgen::JsCast;
                                let input_clone = input.clone();
                                let callback = wasm_bindgen::closure::Closure::once(move || {
                                    let _ = input_clone.focus();
                                    leptos::logging::log!("✅ Re-focused IME input after blur");
                                });
                                let window = web_sys::window().unwrap();
                                let _ = window.request_animation_frame(callback.as_ref().unchecked_ref());
                                callback.forget();
                            }
                        } else {
                            leptos::logging::log!("⏸️  Not re-focusing (editor inactive or composing/dragging)");
                        }
                    }
                    style=move || {
                        // ✅ During IME composition, position at cursor for IME candidate window
                        // Otherwise, position off-screen to avoid capturing mouse events
                        if is_composing.get() {
                            format!(
                                "position: absolute; \
                                 left: {}px; \
                                 top: {}px; \
                                 width: 2px; \
                                 height: {}px; \
                                 opacity: 0; \
                                 z-index: 999; \
                                 color: transparent; \
                                 background: transparent; \
                                 border: none; \
                                 outline: none; \
                                 padding: 0; \
                                 margin: 0; \
                                 caret-color: transparent;",
                                cursor_x.get(),
                                cursor_y.get(),
                                LINE_HEIGHT
                            )
                        } else {
                            // Position off-screen so mouse clicks go to Canvas
                            format!(
                                "position: absolute; \
                                 left: -9999px; \
                                 top: -9999px; \
                                 width: 1px; \
                                 height: 1px; \
                                 opacity: 0; \
                                 z-index: -1; \
                                 color: transparent; \
                                 background: transparent; \
                                 border: none; \
                                 outline: none; \
                                 padding: 0; \
                                 margin: 0; \
                                 caret-color: transparent;"
                            )
                        }
                    }
                />

                // ✅ LSP: Completion Widget Overlay
                {move || {
                    if show_completion.get() {
                        // Get current tab for cursor position
                        if let Some(tab) = current_tab.get() {
                            // Get renderer for coordinate conversion
                            if let Some(canvas_el) = canvas_ref.get() {
                                if let Ok(renderer) = CanvasRenderer::new((*canvas_el).clone().unchecked_into()) {
                                    // Convert LSP position to pixel coordinates
                                    let position = Position::new(tab.cursor_line, tab.cursor_col);
                                    let (pixel_x, pixel_y) = lsp_position_to_canvas_pixel(
                                        &renderer,
                                        position,
                                        tab.scroll_top,
                                        &tab.buffer,
                                    );

                                    return view! {
                                        <CompletionWidget
                                            items=completion_items
                                            position=Position::new(pixel_x as usize, (pixel_y + 20.0) as usize)
                                            on_select=move |item: CompletionItem| {
                                                // Insert completion into buffer
                                                tabs.update(|tabs_vec| {
                                                    if let Some(active_idx) = active_tab_index.get_untracked() {
                                                        if let Some(tab) = tabs_vec.get_mut(active_idx) {
                                                            let char_idx = tab.buffer.line_to_char(tab.cursor_line) + tab.cursor_col;
                                                            tab.buffer.insert(char_idx, &item.label);
                                                            tab.cursor_col += item.label.len();
                                                        }
                                                    }
                                                });
                                                show_completion.set(false);
                                                render_trigger.update(|v| *v += 1);
                                            }
                                        />
                                    }.into_any();
                                }
                            }
                        }
                    }

                    view! { <></> }.into_any()
                }}

                // ✅ LSP: Hover Tooltip Overlay
                {move || {
                    if let Some(info) = hover_info.get() {
                        if let Some((pixel_x, pixel_y)) = hover_pixel_position.get() {
                            return view! {
                                <HoverTooltip
                                    hover_info=hover_info
                                    position=hover_pixel_position
                                />
                            }.into_any();
                        }
                    }
                    view! { <></> }.into_any()
                }}

                // ✅ LSP: Diagnostics Panel (below editor, only show when there are diagnostics)
                {move || {
                    if !diagnostics.get().is_empty() {
                        view! {
                            <DiagnosticsPanel
                                diagnostics=diagnostics
                                on_click=move |line: u32, character: u32| {
                                    // Jump to diagnostic location
                                    tabs.update(|tabs_vec| {
                                        if let Some(active_idx) = active_tab_index.get_untracked() {
                                            if let Some(tab) = tabs_vec.get_mut(active_idx) {
                                                tab.cursor_line = line as usize;
                                                tab.cursor_col = character as usize;
                                            }
                                        }
                                    });
                                    render_trigger.update(|v| *v += 1);
                                }
                            />
                        }.into_any()
                    } else {
                        view! { <></> }.into_any()
                    }
                }}
            </div>
        </div>
    }
}

#[cfg(test)]
mod lsp_tests {
    use super::*;

    #[test]
    fn test_position_creation() {
        let pos = Position::new(0, 0);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.column, 0);

        let pos2 = Position::new(100, 50);
        assert_eq!(pos2.line, 100);
        assert_eq!(pos2.column, 50);
    }

    #[test]
    fn test_position_from_cursor() {
        // Verify Position can be created from cursor coordinates
        let line = 5;
        let col = 10;
        let position = Position::new(line, col);

        assert_eq!(position.line, line);
        assert_eq!(position.column, col);
    }
}

#[cfg(test)]
mod action_tests {
    use super::*;
    use crate::core::actions::{Direction, EditorAction};

    #[test]
    fn test_execute_action_insert_char() {
        let mut tab = EditorTab::new("test.txt".to_string(), "Hello".to_string());
        tab.cursor_col = 5; // After "Hello"

        let changed = tab.execute_action(&EditorAction::InsertChar(' '));

        assert!(changed);
        assert_eq!(tab.buffer.to_string(), "Hello ");
        assert_eq!(tab.cursor_col, 6);
    }

    #[test]
    fn test_execute_action_backspace() {
        let mut tab = EditorTab::new("test.txt".to_string(), "Hello".to_string());
        tab.cursor_col = 5; // After "Hello"

        let changed = tab.execute_action(&EditorAction::Backspace);

        assert!(changed);
        assert_eq!(tab.buffer.to_string(), "Hell");
        assert_eq!(tab.cursor_col, 4);
    }

    #[test]
    fn test_execute_action_backspace_at_start() {
        let mut tab = EditorTab::new("test.txt".to_string(), "Hello".to_string());
        tab.cursor_col = 0; // At start

        let changed = tab.execute_action(&EditorAction::Backspace);

        assert!(!changed); // No change when at start
        assert_eq!(tab.buffer.to_string(), "Hello");
        assert_eq!(tab.cursor_col, 0);
    }

    #[test]
    fn test_execute_action_newline() {
        let mut tab = EditorTab::new("test.txt".to_string(), "Hello".to_string());
        tab.cursor_col = 5; // After "Hello"

        let changed = tab.execute_action(&EditorAction::NewLine);

        assert!(changed);
        assert_eq!(tab.buffer.to_string(), "Hello\n");
        assert_eq!(tab.cursor_line, 1);
        assert_eq!(tab.cursor_col, 0);
    }

    #[test]
    fn test_execute_action_move_cursor_left() {
        let mut tab = EditorTab::new("test.txt".to_string(), "Hello".to_string());
        tab.cursor_col = 5;

        let changed = tab.execute_action(&EditorAction::MoveCursor(Direction::Left));

        assert!(!changed); // Cursor movement doesn't modify buffer
        assert_eq!(tab.cursor_col, 4);
    }

    #[test]
    fn test_execute_action_move_cursor_right() {
        let mut tab = EditorTab::new("test.txt".to_string(), "Hello".to_string());
        tab.cursor_col = 0;

        let changed = tab.execute_action(&EditorAction::MoveCursor(Direction::Right));

        assert!(!changed);
        assert_eq!(tab.cursor_col, 1);
    }

    #[test]
    fn test_execute_action_move_to_line_start() {
        let mut tab = EditorTab::new("test.txt".to_string(), "Hello".to_string());
        tab.cursor_col = 5;

        let changed = tab.execute_action(&EditorAction::MoveToLineStart);

        assert!(!changed);
        assert_eq!(tab.cursor_col, 0);
    }

    #[test]
    fn test_execute_action_move_to_line_end() {
        let mut tab = EditorTab::new("test.txt".to_string(), "Hello".to_string());
        tab.cursor_col = 0;

        let changed = tab.execute_action(&EditorAction::MoveToLineEnd);

        assert!(!changed);
        assert_eq!(tab.cursor_col, 5);
    }

    #[test]
    fn test_execute_action_delete() {
        let mut tab = EditorTab::new("test.txt".to_string(), "Hello".to_string());
        tab.cursor_col = 0; // At start

        let changed = tab.execute_action(&EditorAction::Delete);

        assert!(changed);
        assert_eq!(tab.buffer.to_string(), "ello");
        assert_eq!(tab.cursor_col, 0);
    }

    #[test]
    fn test_execute_action_undo_redo() {
        let mut tab = EditorTab::new("test.txt".to_string(), "Hello".to_string());
        tab.cursor_col = 5;

        // Insert a character
        tab.execute_action(&EditorAction::InsertChar('!'));
        assert_eq!(tab.buffer.to_string(), "Hello!");

        // Undo
        let changed = tab.execute_action(&EditorAction::Undo);
        assert!(changed);
        assert_eq!(tab.buffer.to_string(), "Hello");

        // Redo
        let changed = tab.execute_action(&EditorAction::Redo);
        assert!(changed);
        assert_eq!(tab.buffer.to_string(), "Hello!");
    }

    #[test]
    fn test_execute_action_select_all() {
        let mut tab = EditorTab::new("test.txt".to_string(), "Hello\nWorld".to_string());

        let changed = tab.execute_action(&EditorAction::SelectAll);

        assert!(!changed); // Selection doesn't modify buffer
        assert!(tab.selection_start.is_some());
        assert!(tab.selection_end.is_some());
        assert_eq!(tab.selection_start.unwrap(), (0, 0));
        assert_eq!(tab.selection_end.unwrap(), (1, 5)); // End of "World"
    }

    #[test]
    fn test_execute_action_insert_text() {
        let mut tab = EditorTab::new("test.txt".to_string(), "Hello".to_string());
        tab.cursor_col = 5;

        let changed = tab.execute_action(&EditorAction::InsertText(" World".to_string()));

        assert!(changed);
        assert_eq!(tab.buffer.to_string(), "Hello World");
        assert_eq!(tab.cursor_col, 11);
    }

    #[test]
    fn test_execute_action_insert_char_with_selection() {
        let mut tab = EditorTab::new("test.txt".to_string(), "Hello".to_string());
        tab.selection_start = Some((0, 0));
        tab.selection_end = Some((0, 5));
        tab.cursor_col = 5;

        let changed = tab.execute_action(&EditorAction::InsertChar('H'));

        assert!(changed);
        assert_eq!(tab.buffer.to_string(), "H");
        assert_eq!(tab.cursor_col, 1);
        assert!(tab.selection_start.is_none());
        assert!(tab.selection_end.is_none());
    }

    #[test]
    fn test_action_modifies_buffer() {
        assert!(EditorAction::InsertChar('a').modifies_buffer());
        assert!(EditorAction::Backspace.modifies_buffer());
        assert!(EditorAction::Delete.modifies_buffer());
        assert!(EditorAction::NewLine.modifies_buffer());
        assert!(EditorAction::InsertText("test".to_string()).modifies_buffer());

        assert!(!EditorAction::MoveCursor(Direction::Left).modifies_buffer());
        assert!(!EditorAction::Copy.modifies_buffer());
        assert!(!EditorAction::SelectAll.modifies_buffer());
    }

    #[test]
    fn test_action_requires_undo_save() {
        assert!(EditorAction::InsertChar('a').requires_undo_save());
        assert!(EditorAction::Backspace.requires_undo_save());
        assert!(EditorAction::Delete.requires_undo_save());

        assert!(!EditorAction::Undo.requires_undo_save());
        assert!(!EditorAction::Redo.requires_undo_save());
        assert!(!EditorAction::MoveCursor(Direction::Left).requires_undo_save());
    }
}
