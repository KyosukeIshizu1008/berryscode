//! iTerm2-class terminal emulator with PTY sessions, VT100 grid, and tab support
//!
//! Architecture:
//! - Each tab owns a persistent PTY shell session (zsh/bash)
//! - A background reader thread parses VT100 via `vte` crate and updates the grid
//! - The UI thread reads the grid (behind Arc<Mutex>) and paints it via egui
//! - Keyboard input is written directly to the PTY master

use egui::Color32;
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

// ═══════════════════════════════════════════════════════════════════
// Constants — iTerm2 Default Dark theme colors
// ═══════════════════════════════════════════════════════════════════

pub const TERM_BG: Color32 = Color32::from_rgb(29, 29, 29);
pub const TERM_FG: Color32 = Color32::from_rgb(204, 204, 204);
#[allow(dead_code)]
pub const CURSOR_COLOR: Color32 = Color32::from_rgb(192, 192, 192);
pub const SELECTION_BG: Color32 = Color32::from_rgb(63, 110, 176);
pub const TAB_BAR_BG: Color32 = Color32::from_rgb(22, 22, 22);
pub const TAB_ACTIVE_BG: Color32 = Color32::from_rgb(44, 44, 44);
pub const TAB_INACTIVE_BG: Color32 = Color32::from_rgb(30, 30, 30);
pub const TAB_BORDER: Color32 = Color32::from_rgb(60, 60, 60);
pub const SCROLLBAR_COLOR: Color32 = Color32::from_rgb(80, 80, 80);

const MAX_SCROLLBACK: usize = 10_000;
const DEFAULT_ROWS: usize = 24;
const DEFAULT_COLS: usize = 80;

// ═══════════════════════════════════════════════════════════════════
// Cell — single character cell in the terminal grid
// ═══════════════════════════════════════════════════════════════════

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct Cell {
    pub ch: char,
    pub fg: Color32,
    pub bg: Color32,
    pub bold: bool,
    pub underline: bool,
    pub inverse: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            fg: TERM_FG,
            bg: TERM_BG,
            bold: false,
            underline: false,
            inverse: false,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// TerminalGrid — VT100/xterm character grid with scrollback
// ═══════════════════════════════════════════════════════════════════

pub struct TerminalGrid {
    pub cells: Vec<Vec<Cell>>,
    pub rows: usize,
    pub cols: usize,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub cursor_visible: bool,
    pub scrollback: Vec<Vec<Cell>>,
    pub scroll_offset: usize,
    pub title: String,

    // Current text attributes
    fg: Color32,
    bg: Color32,
    bold: bool,
    underline: bool,
    inverse: bool,

    // Saved cursor (DECSC / DECRC)
    saved_cursor: Option<(usize, usize, Color32, Color32, bool)>,

    // Scroll region (DECSTBM)
    scroll_top: usize,
    scroll_bottom: usize,

    // Alternate screen buffer (for vim, less, etc.)
    alt_screen: Option<AltScreen>,
    in_alt_screen: bool,

    // Auto-wrap mode
    wrap_next: bool,
    auto_wrap: bool,

    // Origin mode (DECOM)
    origin_mode: bool,

    // Bracketed paste
    pub bracketed_paste: bool,

    // Application cursor keys mode
    pub app_cursor_keys: bool,

    pub dirty: bool,
}

struct AltScreen {
    cells: Vec<Vec<Cell>>,
    cursor_row: usize,
    cursor_col: usize,
    scroll_top: usize,
    scroll_bottom: usize,
}

impl TerminalGrid {
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            cells: vec![vec![Cell::default(); cols]; rows],
            rows,
            cols,
            cursor_row: 0,
            cursor_col: 0,
            cursor_visible: true,
            scrollback: Vec::new(),
            scroll_offset: 0,
            title: String::new(),
            fg: TERM_FG,
            bg: TERM_BG,
            bold: false,
            underline: false,
            inverse: false,
            saved_cursor: None,
            scroll_top: 0,
            scroll_bottom: rows.saturating_sub(1),
            alt_screen: None,
            in_alt_screen: false,
            wrap_next: false,
            auto_wrap: true,
            origin_mode: false,
            bracketed_paste: false,
            app_cursor_keys: false,
            dirty: true,
        }
    }

    // ─── Scroll operations ───────────────────────────────────────

    fn scroll_up_region(&mut self) {
        if !self.in_alt_screen {
            self.scrollback.push(self.cells[self.scroll_top].clone());
            if self.scrollback.len() > MAX_SCROLLBACK {
                self.scrollback.remove(0);
            }
        }
        for i in self.scroll_top..self.scroll_bottom {
            self.cells[i] = self.cells[i + 1].clone();
        }
        self.cells[self.scroll_bottom] = vec![Cell::default(); self.cols];
    }

    fn scroll_down_region(&mut self) {
        for i in (self.scroll_top + 1..=self.scroll_bottom).rev() {
            self.cells[i] = self.cells[i - 1].clone();
        }
        self.cells[self.scroll_top] = vec![Cell::default(); self.cols];
    }

    // ─── Character output ────────────────────────────────────────

    fn newline(&mut self) {
        self.wrap_next = false;
        if self.cursor_row == self.scroll_bottom {
            self.scroll_up_region();
        } else if self.cursor_row < self.rows - 1 {
            self.cursor_row += 1;
        }
    }

    fn put_char(&mut self, ch: char) {
        if self.wrap_next && self.auto_wrap {
            self.cursor_col = 0;
            self.newline();
            self.wrap_next = false;
        }

        if self.cursor_row < self.rows && self.cursor_col < self.cols {
            let (fg, bg) = if self.inverse {
                (self.bg, self.fg)
            } else {
                (self.fg, self.bg)
            };
            self.cells[self.cursor_row][self.cursor_col] = Cell {
                ch,
                fg,
                bg,
                bold: self.bold,
                underline: self.underline,
                inverse: false,
            };
        }

        if self.cursor_col + 1 >= self.cols {
            self.wrap_next = true;
        } else {
            self.cursor_col += 1;
        }
    }

    // ─── Erase operations ────────────────────────────────────────

    fn erase_display(&mut self, mode: u16) {
        match mode {
            0 => {
                // Cursor to end
                for col in self.cursor_col..self.cols {
                    if self.cursor_row < self.rows {
                        self.cells[self.cursor_row][col] = Cell::default();
                    }
                }
                for row in (self.cursor_row + 1)..self.rows {
                    self.cells[row] = vec![Cell::default(); self.cols];
                }
            }
            1 => {
                // Start to cursor
                for row in 0..self.cursor_row {
                    self.cells[row] = vec![Cell::default(); self.cols];
                }
                for col in 0..=self.cursor_col.min(self.cols.saturating_sub(1)) {
                    self.cells[self.cursor_row][col] = Cell::default();
                }
            }
            2 => {
                for row in 0..self.rows {
                    self.cells[row] = vec![Cell::default(); self.cols];
                }
            }
            3 => {
                // Erase display + scrollback
                for row in 0..self.rows {
                    self.cells[row] = vec![Cell::default(); self.cols];
                }
                self.scrollback.clear();
                self.scroll_offset = 0;
            }
            _ => {}
        }
    }

    fn erase_line(&mut self, mode: u16) {
        if self.cursor_row >= self.rows {
            return;
        }
        match mode {
            0 => {
                for col in self.cursor_col..self.cols {
                    self.cells[self.cursor_row][col] = Cell::default();
                }
            }
            1 => {
                for col in 0..=self.cursor_col.min(self.cols.saturating_sub(1)) {
                    self.cells[self.cursor_row][col] = Cell::default();
                }
            }
            2 => {
                self.cells[self.cursor_row] = vec![Cell::default(); self.cols];
            }
            _ => {}
        }
    }

    // ─── SGR (Select Graphic Rendition) ──────────────────────────

    fn apply_sgr(&mut self, params: &[u16]) {
        if params.is_empty() {
            self.reset_attrs();
            return;
        }

        let mut i = 0;
        while i < params.len() {
            match params[i] {
                0 => self.reset_attrs(),
                1 => self.bold = true,
                2 => {} // dim — accept but don't render
                3 => {} // italic
                4 => self.underline = true,
                5 | 6 => {} // blink
                7 => self.inverse = true,
                8 => {} // hidden
                9 => {} // strikethrough
                22 => self.bold = false,
                23 => {} // not italic
                24 => self.underline = false,
                25 => {} // not blink
                27 => self.inverse = false,
                28 => {} // not hidden
                29 => {} // not strikethrough

                // Standard foreground (30-37)
                30 => self.fg = Color32::from_rgb(0, 0, 0),
                31 => self.fg = Color32::from_rgb(194, 54, 33),
                32 => self.fg = Color32::from_rgb(37, 188, 36),
                33 => self.fg = Color32::from_rgb(173, 173, 39),
                34 => self.fg = Color32::from_rgb(73, 46, 225),
                35 => self.fg = Color32::from_rgb(211, 56, 211),
                36 => self.fg = Color32::from_rgb(51, 187, 200),
                37 => self.fg = Color32::from_rgb(203, 204, 205),

                // Extended foreground: 38;5;N or 38;2;R;G;B
                38 => {
                    if i + 1 < params.len() {
                        match params[i + 1] {
                            5 if i + 2 < params.len() => {
                                self.fg = ansi_256_color(params[i + 2]);
                                i += 2;
                            }
                            2 if i + 4 < params.len() => {
                                self.fg = Color32::from_rgb(
                                    params[i + 2] as u8,
                                    params[i + 3] as u8,
                                    params[i + 4] as u8,
                                );
                                i += 4;
                            }
                            _ => {}
                        }
                    }
                }
                39 => self.fg = TERM_FG,

                // Standard background (40-47)
                40 => self.bg = Color32::from_rgb(0, 0, 0),
                41 => self.bg = Color32::from_rgb(194, 54, 33),
                42 => self.bg = Color32::from_rgb(37, 188, 36),
                43 => self.bg = Color32::from_rgb(173, 173, 39),
                44 => self.bg = Color32::from_rgb(73, 46, 225),
                45 => self.bg = Color32::from_rgb(211, 56, 211),
                46 => self.bg = Color32::from_rgb(51, 187, 200),
                47 => self.bg = Color32::from_rgb(203, 204, 205),

                // Extended background: 48;5;N or 48;2;R;G;B
                48 => {
                    if i + 1 < params.len() {
                        match params[i + 1] {
                            5 if i + 2 < params.len() => {
                                self.bg = ansi_256_color(params[i + 2]);
                                i += 2;
                            }
                            2 if i + 4 < params.len() => {
                                self.bg = Color32::from_rgb(
                                    params[i + 2] as u8,
                                    params[i + 3] as u8,
                                    params[i + 4] as u8,
                                );
                                i += 4;
                            }
                            _ => {}
                        }
                    }
                }
                49 => self.bg = TERM_BG,

                // Bright foreground (90-97)
                90 => self.fg = Color32::from_rgb(129, 131, 131),
                91 => self.fg = Color32::from_rgb(252, 57, 31),
                92 => self.fg = Color32::from_rgb(49, 231, 34),
                93 => self.fg = Color32::from_rgb(234, 236, 35),
                94 => self.fg = Color32::from_rgb(88, 51, 255),
                95 => self.fg = Color32::from_rgb(249, 53, 248),
                96 => self.fg = Color32::from_rgb(20, 240, 240),
                97 => self.fg = Color32::from_rgb(233, 235, 235),

                // Bright background (100-107)
                100 => self.bg = Color32::from_rgb(129, 131, 131),
                101 => self.bg = Color32::from_rgb(252, 57, 31),
                102 => self.bg = Color32::from_rgb(49, 231, 34),
                103 => self.bg = Color32::from_rgb(234, 236, 35),
                104 => self.bg = Color32::from_rgb(88, 51, 255),
                105 => self.bg = Color32::from_rgb(249, 53, 248),
                106 => self.bg = Color32::from_rgb(20, 240, 240),
                107 => self.bg = Color32::from_rgb(233, 235, 235),

                _ => {}
            }
            i += 1;
        }
    }

    fn reset_attrs(&mut self) {
        self.fg = TERM_FG;
        self.bg = TERM_BG;
        self.bold = false;
        self.underline = false;
        self.inverse = false;
    }

    // ─── Alternate screen buffer ─────────────────────────────────

    fn enter_alt_screen(&mut self) {
        if self.in_alt_screen {
            return;
        }
        self.alt_screen = Some(AltScreen {
            cells: self.cells.clone(),
            cursor_row: self.cursor_row,
            cursor_col: self.cursor_col,
            scroll_top: self.scroll_top,
            scroll_bottom: self.scroll_bottom,
        });
        self.cells = vec![vec![Cell::default(); self.cols]; self.rows];
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.scroll_top = 0;
        self.scroll_bottom = self.rows.saturating_sub(1);
        self.in_alt_screen = true;
    }

    fn exit_alt_screen(&mut self) {
        if !self.in_alt_screen {
            return;
        }
        if let Some(alt) = self.alt_screen.take() {
            self.cells = alt.cells;
            self.cursor_row = alt.cursor_row;
            self.cursor_col = alt.cursor_col;
            self.scroll_top = alt.scroll_top;
            self.scroll_bottom = alt.scroll_bottom;
        }
        self.in_alt_screen = false;
    }

    // ─── Resize ──────────────────────────────────────────────────

    pub fn resize(&mut self, new_rows: usize, new_cols: usize) {
        if new_rows == 0 || new_cols == 0 || (new_rows == self.rows && new_cols == self.cols) {
            return;
        }

        let mut new_cells = vec![vec![Cell::default(); new_cols]; new_rows];
        let copy_rows = new_rows.min(self.rows);
        let copy_cols = new_cols.min(self.cols);

        for row in 0..copy_rows {
            for col in 0..copy_cols {
                new_cells[row][col] = self.cells[row][col];
            }
        }

        self.cells = new_cells;
        self.rows = new_rows;
        self.cols = new_cols;
        self.cursor_row = self.cursor_row.min(new_rows.saturating_sub(1));
        self.cursor_col = self.cursor_col.min(new_cols.saturating_sub(1));
        self.scroll_bottom = new_rows.saturating_sub(1);
        if self.scroll_top >= new_rows {
            self.scroll_top = 0;
        }
        self.dirty = true;
    }

    // ─── DECSET / DECRST helpers ─────────────────────────────────

    fn handle_decset(&mut self, mode: u16) {
        match mode {
            1 => self.app_cursor_keys = true,
            6 => self.origin_mode = true,
            7 => self.auto_wrap = true,
            25 => self.cursor_visible = true,
            47 | 1047 => self.enter_alt_screen(),
            1049 => {
                self.save_cursor();
                self.enter_alt_screen();
            }
            2004 => self.bracketed_paste = true,
            _ => {}
        }
    }

    fn handle_decrst(&mut self, mode: u16) {
        match mode {
            1 => self.app_cursor_keys = false,
            6 => self.origin_mode = false,
            7 => self.auto_wrap = false,
            25 => self.cursor_visible = false,
            47 | 1047 => self.exit_alt_screen(),
            1049 => {
                self.exit_alt_screen();
                self.restore_cursor();
            }
            2004 => self.bracketed_paste = false,
            _ => {}
        }
    }

    fn save_cursor(&mut self) {
        self.saved_cursor = Some((
            self.cursor_row,
            self.cursor_col,
            self.fg,
            self.bg,
            self.bold,
        ));
    }

    fn restore_cursor(&mut self) {
        if let Some((row, col, fg, bg, bold)) = self.saved_cursor {
            self.cursor_row = row.min(self.rows.saturating_sub(1));
            self.cursor_col = col.min(self.cols.saturating_sub(1));
            self.fg = fg;
            self.bg = bg;
            self.bold = bold;
        }
    }

    /// Get all visible lines (scrollback + active) for the current scroll position
    ///
    /// scroll_offset = 0: show current grid (latest output)
    /// scroll_offset = N: show N lines into scrollback history
    /// scroll_offset = scrollback.len(): show the very beginning
    pub fn visible_lines(&self, viewport_rows: usize) -> Vec<&[Cell]> {
        let mut lines = Vec::with_capacity(viewport_rows);
        let sb_len = self.scrollback.len();

        if self.scroll_offset == 0 {
            // No scrollback — show current grid
            for i in 0..viewport_rows.min(self.rows).min(self.cells.len()) {
                lines.push(self.cells[i].as_slice());
            }
        } else {
            // Combined view: scrollback + current grid as one continuous buffer
            // Total virtual lines = sb_len + self.rows
            // We want to show viewport_rows lines ending at (total - scroll_offset)
            let total = sb_len + self.rows;
            let view_end = total.saturating_sub(self.scroll_offset);
            let view_start = view_end.saturating_sub(viewport_rows);

            for vline in view_start..view_end {
                if vline < sb_len {
                    lines.push(self.scrollback[vline].as_slice());
                } else {
                    let grid_row = vline - sb_len;
                    if grid_row < self.cells.len() {
                        lines.push(self.cells[grid_row].as_slice());
                    }
                }
            }
        }

        lines
    }
}

// ─── vte::Perform implementation ─────────────────────────────────

impl vte::Perform for TerminalGrid {
    fn print(&mut self, ch: char) {
        self.put_char(ch);
        self.dirty = true;
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            0x07 => {} // BEL
            0x08 => {
                // BS (backspace)
                self.wrap_next = false;
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                }
            }
            0x09 => {
                // TAB
                self.cursor_col = ((self.cursor_col / 8) + 1) * 8;
                if self.cursor_col >= self.cols {
                    self.cursor_col = self.cols - 1;
                }
                self.wrap_next = false;
            }
            0x0A | 0x0B | 0x0C => {
                // LF, VT, FF
                self.newline();
            }
            0x0D => {
                // CR
                self.cursor_col = 0;
                self.wrap_next = false;
            }
            0x0E => {} // SO (shift out)
            0x0F => {} // SI (shift in)
            _ => {}
        }
        self.dirty = true;
    }

    fn csi_dispatch(
        &mut self,
        params: &vte::Params,
        intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        let params_vec: Vec<u16> = params.iter().map(|p| p[0]).collect();
        let p = |idx: usize, default: u16| -> u16 {
            params_vec
                .get(idx)
                .copied()
                .filter(|&v| v != 0)
                .unwrap_or(default)
        };
        let is_private = intermediates.first() == Some(&b'?');

        self.wrap_next = false;

        match action {
            'A' => {
                // CUU — cursor up
                let n = p(0, 1) as usize;
                self.cursor_row = self.cursor_row.saturating_sub(n);
            }
            'B' | 'e' => {
                // CUD — cursor down
                let n = p(0, 1) as usize;
                self.cursor_row = (self.cursor_row + n).min(self.rows.saturating_sub(1));
            }
            'C' | 'a' => {
                // CUF — cursor forward
                let n = p(0, 1) as usize;
                self.cursor_col = (self.cursor_col + n).min(self.cols.saturating_sub(1));
            }
            'D' => {
                // CUB — cursor backward
                let n = p(0, 1) as usize;
                self.cursor_col = self.cursor_col.saturating_sub(n);
            }
            'E' => {
                // CNL — cursor next line
                let n = p(0, 1) as usize;
                self.cursor_col = 0;
                self.cursor_row = (self.cursor_row + n).min(self.rows.saturating_sub(1));
            }
            'F' => {
                // CPL — cursor previous line
                let n = p(0, 1) as usize;
                self.cursor_col = 0;
                self.cursor_row = self.cursor_row.saturating_sub(n);
            }
            'G' | '`' => {
                // CHA — cursor horizontal absolute
                let col = (p(0, 1) as usize).saturating_sub(1);
                self.cursor_col = col.min(self.cols.saturating_sub(1));
            }
            'H' | 'f' => {
                // CUP — cursor position
                let row = (p(0, 1) as usize).saturating_sub(1);
                let col = (p(1, 1) as usize).saturating_sub(1);
                self.cursor_row = row.min(self.rows.saturating_sub(1));
                self.cursor_col = col.min(self.cols.saturating_sub(1));
            }
            'J' => {
                // ED — erase display
                self.erase_display(p(0, 0));
            }
            'K' => {
                // EL — erase line
                self.erase_line(p(0, 0));
            }
            'L' => {
                // IL — insert lines
                let n = p(0, 1) as usize;
                for _ in 0..n {
                    if self.cursor_row <= self.scroll_bottom {
                        for i in (self.cursor_row + 1..=self.scroll_bottom).rev() {
                            self.cells[i] = self.cells[i - 1].clone();
                        }
                        self.cells[self.cursor_row] = vec![Cell::default(); self.cols];
                    }
                }
            }
            'M' => {
                // DL — delete lines
                let n = p(0, 1) as usize;
                for _ in 0..n {
                    if self.cursor_row <= self.scroll_bottom {
                        for i in self.cursor_row..self.scroll_bottom {
                            self.cells[i] = self.cells[i + 1].clone();
                        }
                        self.cells[self.scroll_bottom] = vec![Cell::default(); self.cols];
                    }
                }
            }
            'P' => {
                // DCH — delete characters
                let n = p(0, 1) as usize;
                let row = self.cursor_row;
                let col = self.cursor_col;
                if row < self.rows {
                    let end = self.cols;
                    for i in col..end {
                        self.cells[row][i] = if i + n < end {
                            self.cells[row][i + n]
                        } else {
                            Cell::default()
                        };
                    }
                }
            }
            '@' => {
                // ICH — insert characters
                let n = p(0, 1) as usize;
                let row = self.cursor_row;
                let col = self.cursor_col;
                if row < self.rows {
                    for i in (col + n..self.cols).rev() {
                        self.cells[row][i] = self.cells[row][i - n];
                    }
                    for i in col..(col + n).min(self.cols) {
                        self.cells[row][i] = Cell::default();
                    }
                }
            }
            'S' => {
                // SU — scroll up
                let n = p(0, 1) as usize;
                for _ in 0..n {
                    self.scroll_up_region();
                }
            }
            'T' => {
                // SD — scroll down
                if !is_private {
                    let n = p(0, 1) as usize;
                    for _ in 0..n {
                        self.scroll_down_region();
                    }
                }
            }
            'X' => {
                // ECH — erase characters
                let n = p(0, 1) as usize;
                let row = self.cursor_row;
                for i in self.cursor_col..(self.cursor_col + n).min(self.cols) {
                    if row < self.rows {
                        self.cells[row][i] = Cell::default();
                    }
                }
            }
            'd' => {
                // VPA — vertical position absolute
                let row = (p(0, 1) as usize).saturating_sub(1);
                self.cursor_row = row.min(self.rows.saturating_sub(1));
            }
            'h' => {
                if is_private {
                    for &mode in &params_vec {
                        self.handle_decset(mode);
                    }
                }
            }
            'l' => {
                if is_private {
                    for &mode in &params_vec {
                        self.handle_decrst(mode);
                    }
                }
            }
            'm' => {
                // SGR
                if params_vec.is_empty() {
                    self.reset_attrs();
                } else {
                    self.apply_sgr(&params_vec);
                }
            }
            'n' => {
                // DSR — device status report (we ignore; would need write-back)
            }
            'r' => {
                // DECSTBM — set scrolling region
                let top = (p(0, 1) as usize).saturating_sub(1);
                let bottom = if params_vec.len() > 1 && params_vec[1] != 0 {
                    (params_vec[1] as usize).saturating_sub(1)
                } else {
                    self.rows.saturating_sub(1)
                };
                self.scroll_top = top.min(self.rows.saturating_sub(1));
                self.scroll_bottom = bottom.min(self.rows.saturating_sub(1));
                self.cursor_row = if self.origin_mode { self.scroll_top } else { 0 };
                self.cursor_col = 0;
            }
            's' => {
                // SCP — save cursor position
                self.save_cursor();
            }
            'u' => {
                // RCP — restore cursor position
                self.restore_cursor();
            }
            't' => {
                // Window manipulation (ignored)
            }
            _ => {}
        }
        self.dirty = true;
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        if params.len() >= 2 {
            match params[0] {
                b"0" | b"1" | b"2" => {
                    self.title = String::from_utf8_lossy(params[1]).to_string();
                }
                _ => {}
            }
        }
        self.dirty = true;
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], _ignore: bool, byte: u8) {
        self.wrap_next = false;
        match byte {
            b'7' => self.save_cursor(),
            b'8' => {
                if intermediates.first() == Some(&b'#') {
                    // DECALN — fill screen with 'E'
                    for row in 0..self.rows {
                        for col in 0..self.cols {
                            self.cells[row][col] = Cell {
                                ch: 'E',
                                ..Cell::default()
                            };
                        }
                    }
                } else {
                    self.restore_cursor();
                }
            }
            b'D' => {
                // IND — index (scroll up)
                if self.cursor_row == self.scroll_bottom {
                    self.scroll_up_region();
                } else if self.cursor_row < self.rows - 1 {
                    self.cursor_row += 1;
                }
            }
            b'E' => {
                // NEL — next line
                self.cursor_col = 0;
                self.newline();
            }
            b'M' => {
                // RI — reverse index (scroll down)
                if self.cursor_row == self.scroll_top {
                    self.scroll_down_region();
                } else if self.cursor_row > 0 {
                    self.cursor_row -= 1;
                }
            }
            b'c' => {
                // RIS — full reset
                *self = TerminalGrid::new(self.rows, self.cols);
            }
            _ => {}
        }
        self.dirty = true;
    }

    fn hook(&mut self, _params: &vte::Params, _intermediates: &[u8], _ignore: bool, _action: char) {
    }
    fn unhook(&mut self) {}
    fn put(&mut self, _byte: u8) {}
}

// ═══════════════════════════════════════════════════════════════════
// Selection — text selection state
// ═══════════════════════════════════════════════════════════════════

#[derive(Clone, Copy, Debug)]
pub struct Selection {
    pub start_row: usize,
    pub start_col: usize,
    pub end_row: usize,
    pub end_col: usize,
    pub active: bool,
}

impl Selection {
    pub fn normalized(&self) -> (usize, usize, usize, usize) {
        if self.start_row < self.end_row
            || (self.start_row == self.end_row && self.start_col <= self.end_col)
        {
            (self.start_row, self.start_col, self.end_row, self.end_col)
        } else {
            (self.end_row, self.end_col, self.start_row, self.start_col)
        }
    }

    pub fn contains(&self, row: usize, col: usize) -> bool {
        let (sr, sc, er, ec) = self.normalized();
        if row < sr || row > er {
            return false;
        }
        if row == sr && row == er {
            return col >= sc && col <= ec;
        }
        if row == sr {
            return col >= sc;
        }
        if row == er {
            return col <= ec;
        }
        true
    }
}

// ═══════════════════════════════════════════════════════════════════
// TerminalTab — single PTY session
// ═══════════════════════════════════════════════════════════════════

pub struct TerminalTab {
    pub id: usize,
    pub grid: Arc<Mutex<TerminalGrid>>,
    pub writer: Arc<Mutex<Box<dyn Write + Send>>>,
    pub master: Arc<Mutex<Box<dyn portable_pty::MasterPty + Send>>>,
    pub alive: Arc<AtomicBool>,
    pub selection: Option<Selection>,
    pub search_query: String,
    pub search_active: bool,
    pub last_cols: usize,
    pub last_rows: usize,
}

impl TerminalTab {
    /// Spawn a new PTY shell session
    pub fn new(id: usize, working_dir: &str, rows: usize, cols: usize) -> Option<Self> {
        let pty_system = NativePtySystem::default();
        let pair = pty_system
            .openpty(PtySize {
                rows: rows as u16,
                cols: cols as u16,
                pixel_width: 0,
                pixel_height: 0,
            })
            .ok()?;

        let (shell, login_flag) = default_shell();
        let mut cmd = CommandBuilder::new(&shell);
        if let Some(flag) = login_flag {
            cmd.arg(flag);
        }
        cmd.cwd(working_dir);
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        cmd.env(
            "LANG",
            std::env::var("LANG").unwrap_or_else(|_| "en_US.UTF-8".to_string()),
        );

        let _child = pair.slave.spawn_command(cmd).ok()?;
        drop(pair.slave);

        let reader = pair.master.try_clone_reader().ok()?;
        let writer = pair.master.take_writer().ok()?;

        let grid = Arc::new(Mutex::new(TerminalGrid::new(rows, cols)));
        let alive = Arc::new(AtomicBool::new(true));
        let master: Box<dyn portable_pty::MasterPty + Send> = pair.master;

        // Background reader thread: reads PTY output → parses VT → updates grid
        let grid_clone = Arc::clone(&grid);
        let alive_clone = Arc::clone(&alive);
        std::thread::Builder::new()
            .name(format!("pty-reader-{}", id))
            .spawn(move || {
                pty_reader_loop(reader, grid_clone, alive_clone);
            })
            .ok()?;

        Some(Self {
            id,
            grid,
            writer: Arc::new(Mutex::new(writer)),
            master: Arc::new(Mutex::new(master)),
            alive,
            selection: None,
            search_query: String::new(),
            search_active: false,
            last_cols: cols,
            last_rows: rows,
        })
    }

    /// Write bytes to the PTY (e.g. typed characters, control sequences)
    pub fn write_to_pty(&self, data: &[u8]) {
        if !self.alive.load(Ordering::Relaxed) {
            return;
        }
        if let Ok(mut writer) = self.writer.lock() {
            let _ = writer.write_all(data);
            let _ = writer.flush();
        }
    }

    /// Resize the PTY and grid
    pub fn resize(&mut self, rows: usize, cols: usize) {
        if rows == self.last_rows && cols == self.last_cols {
            return;
        }
        self.last_rows = rows;
        self.last_cols = cols;

        if let Ok(master) = self.master.lock() {
            let _ = master.resize(PtySize {
                rows: rows as u16,
                cols: cols as u16,
                pixel_width: 0,
                pixel_height: 0,
            });
        }
        if let Ok(mut grid) = self.grid.lock() {
            grid.resize(rows, cols);
        }
    }

    /// Get the current title (from shell or OSC)
    pub fn title(&self) -> String {
        if let Ok(grid) = self.grid.lock() {
            if !grid.title.is_empty() {
                return grid.title.clone();
            }
        }
        format!("Terminal {}", self.id + 1)
    }

    /// Is this session still alive?
    pub fn is_alive(&self) -> bool {
        self.alive.load(Ordering::Relaxed)
    }

    /// Extract selected text
    pub fn get_selected_text(&self) -> Option<String> {
        let sel = self.selection.as_ref().filter(|s| s.active)?;
        let (sr, sc, er, ec) = sel.normalized();
        let grid = self.grid.lock().ok()?;
        let mut text = String::new();

        for row in sr..=er {
            if row >= grid.rows {
                break;
            }
            let col_start = if row == sr { sc } else { 0 };
            let col_end = if row == er {
                ec.min(grid.cols.saturating_sub(1))
            } else {
                grid.cols.saturating_sub(1)
            };

            for col in col_start..=col_end {
                text.push(grid.cells[row][col].ch);
            }
            // Trim trailing spaces on each line
            let trimmed = text.trim_end().len();
            text.truncate(trimmed);
            if row < er {
                text.push('\n');
            }
        }

        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// TerminalEmulator — manages multiple tabs
// ═══════════════════════════════════════════════════════════════════

pub struct TerminalEmulator {
    pub tabs: Vec<TerminalTab>,
    pub active_tab: usize,
    next_id: usize,
    pub working_dir: String,
    pub cursor_blink_timer: f64,
    pub cursor_visible_blink: bool,
}

impl TerminalEmulator {
    pub fn new(working_dir: &str) -> Self {
        Self {
            tabs: Vec::new(),
            active_tab: 0,
            next_id: 0,
            working_dir: working_dir.to_string(),
            cursor_blink_timer: 0.0,
            cursor_visible_blink: true,
        }
    }

    /// Ensure at least one tab exists (lazy init)
    pub fn ensure_tab(&mut self) {
        if self.tabs.is_empty() {
            self.add_tab();
        }
    }

    /// Add a new terminal tab
    pub fn add_tab(&mut self) {
        let id = self.next_id;
        self.next_id += 1;
        if let Some(tab) = TerminalTab::new(id, &self.working_dir, DEFAULT_ROWS, DEFAULT_COLS) {
            self.tabs.push(tab);
            self.active_tab = self.tabs.len() - 1;
        }
    }

    /// Close a tab by index
    pub fn close_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.tabs[index].alive.store(false, Ordering::Relaxed);
            self.tabs.remove(index);
            if self.active_tab >= self.tabs.len() && !self.tabs.is_empty() {
                self.active_tab = self.tabs.len() - 1;
            }
        }
    }

    /// Get the active tab (mutable)
    pub fn active_tab_mut(&mut self) -> Option<&mut TerminalTab> {
        self.tabs.get_mut(self.active_tab)
    }

    /// Get the active tab (immutable)
    pub fn active_tab(&self) -> Option<&TerminalTab> {
        self.tabs.get(self.active_tab)
    }

    /// Update cursor blink state
    pub fn update_blink(&mut self, dt: f64) {
        self.cursor_blink_timer += dt;
        if self.cursor_blink_timer > 0.53 {
            self.cursor_blink_timer = 0.0;
            self.cursor_visible_blink = !self.cursor_visible_blink;
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// Platform-specific shell detection
// ═══════════════════════════════════════════════════════════════════

/// Returns (shell_path, optional_login_flag) for the current platform.
fn default_shell() -> (String, Option<&'static str>) {
    #[cfg(target_os = "windows")]
    {
        // Prefer PowerShell if available, fall back to cmd.exe
        let ps = std::env::var("COMSPEC").unwrap_or_else(|_| {
            if std::path::Path::new(
                "C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe",
            )
            .exists()
            {
                "powershell.exe".to_string()
            } else {
                "cmd.exe".to_string()
            }
        });
        (ps, None) // no login flag for Windows shells
    }
    #[cfg(not(target_os = "windows"))]
    {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| {
            if std::path::Path::new("/bin/zsh").exists() {
                "/bin/zsh".to_string()
            } else {
                "/bin/sh".to_string()
            }
        });
        (shell, Some("-l")) // login shell for proper rc loading
    }
}

// ═══════════════════════════════════════════════════════════════════
// PTY reader loop (background thread)
// ═══════════════════════════════════════════════════════════════════

fn pty_reader_loop(
    mut reader: Box<dyn Read + Send>,
    grid: Arc<Mutex<TerminalGrid>>,
    alive: Arc<AtomicBool>,
) {
    let mut parser = vte::Parser::new();
    let mut buf = [0u8; 4096];

    while alive.load(Ordering::Relaxed) {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                if let Ok(mut g) = grid.lock() {
                    for &byte in &buf[..n] {
                        parser.advance(&mut *g, byte);
                    }
                }
            }
            Err(_) => break,
        }
    }
    alive.store(false, Ordering::Relaxed);
}

// ═══════════════════════════════════════════════════════════════════
// 256-color palette (xterm)
// ═══════════════════════════════════════════════════════════════════

fn ansi_256_color(n: u16) -> Color32 {
    match n {
        0 => Color32::from_rgb(0, 0, 0),
        1 => Color32::from_rgb(194, 54, 33),
        2 => Color32::from_rgb(37, 188, 36),
        3 => Color32::from_rgb(173, 173, 39),
        4 => Color32::from_rgb(73, 46, 225),
        5 => Color32::from_rgb(211, 56, 211),
        6 => Color32::from_rgb(51, 187, 200),
        7 => Color32::from_rgb(203, 204, 205),
        8 => Color32::from_rgb(129, 131, 131),
        9 => Color32::from_rgb(252, 57, 31),
        10 => Color32::from_rgb(49, 231, 34),
        11 => Color32::from_rgb(234, 236, 35),
        12 => Color32::from_rgb(88, 51, 255),
        13 => Color32::from_rgb(249, 53, 248),
        14 => Color32::from_rgb(20, 240, 240),
        15 => Color32::from_rgb(233, 235, 235),
        // 6x6x6 color cube (16–231)
        16..=231 => {
            let idx = n - 16;
            let r_idx = idx / 36;
            let g_idx = (idx % 36) / 6;
            let b_idx = idx % 6;
            let r = if r_idx == 0 {
                0
            } else {
                (r_idx * 40 + 55) as u8
            };
            let g = if g_idx == 0 {
                0
            } else {
                (g_idx * 40 + 55) as u8
            };
            let b = if b_idx == 0 {
                0
            } else {
                (b_idx * 40 + 55) as u8
            };
            Color32::from_rgb(r, g, b)
        }
        // Grayscale (232–255)
        232..=255 => {
            let gray = ((n - 232) * 10 + 8) as u8;
            Color32::from_rgb(gray, gray, gray)
        }
        _ => TERM_FG,
    }
}
