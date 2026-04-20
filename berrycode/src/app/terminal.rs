//! iTerm2-style terminal UI rendering
//!
//! Features:
//! - Tab bar with close buttons and "+" to add tabs
//! - GPU-painted character grid (batched by style runs)
//! - Persistent PTY shell sessions (zsh/bash)
//! - Full keyboard → PTY passthrough (Ctrl+C, Ctrl+D, arrows, etc.)
//! - Mouse selection + clipboard copy
//! - Scrollback with mouse wheel
//! - Cursor blink
//! - Search bar (Cmd+F)
//! - Right-click context menu (Copy/Paste/Clear/Close)

use super::terminal_emulator::*;
use super::BerryCodeApp;

impl BerryCodeApp {
    // ═══════════════════════════════════════════════════════════════
    // Sidebar compact terminal
    // ═══════════════════════════════════════════════════════════════

    pub(crate) fn render_terminal(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Terminal")
                    .color(egui::Color32::from_rgb(200, 200, 200))
                    .size(13.0)
                    .strong(),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("\u{eab0}") // codicon: terminal
                                .size(14.0)
                                .color(egui::Color32::from_rgb(160, 160, 160)),
                        )
                        .frame(false),
                    )
                    .on_hover_text("Open fullscreen terminal")
                    .clicked()
                {
                    self.active_panel = super::types::ActivePanel::Terminal;
                }
            });
        });

        ui.add_space(4.0);

        // Show a mini preview of the terminal grid
        self.terminal.ensure_tab();
        if let Some(tab) = self.terminal.active_tab() {
            if let Ok(grid) = tab.grid.lock() {
                let available = ui.available_size();
                let font_size = 10.0_f32;
                let cell_h = font_size * 1.35;
                let visible_rows = ((available.y - 30.0) / cell_h).max(4.0) as usize;
                let lines = grid.visible_lines(visible_rows.min(grid.rows));

                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .stick_to_bottom(true)
                    .max_height(available.y - 30.0)
                    .show(ui, |ui| {
                        for line in &lines {
                            ui.horizontal(|ui| {
                                ui.add_space(4.0);
                                ui.spacing_mut().item_spacing.x = 0.0;
                                let mut run_text = String::new();
                                let mut run_fg = TERM_FG;
                                let mut run_bold = false;

                                for (col_idx, cell) in line.iter().enumerate() {
                                    let same = cell.fg == run_fg && cell.bold == run_bold;
                                    if !same && !run_text.is_empty() {
                                        let mut rt = egui::RichText::new(&run_text)
                                            .color(run_fg)
                                            .font(egui::FontId::monospace(font_size));
                                        if run_bold {
                                            rt = rt.strong();
                                        }
                                        ui.label(rt);
                                        run_text.clear();
                                    }
                                    run_fg = cell.fg;
                                    run_bold = cell.bold;
                                    run_text.push(cell.ch);

                                    if col_idx == line.len() - 1 && !run_text.is_empty() {
                                        let trimmed = run_text.trim_end();
                                        if !trimmed.is_empty() {
                                            let mut rt = egui::RichText::new(trimmed)
                                                .color(run_fg)
                                                .font(egui::FontId::monospace(font_size));
                                            if run_bold {
                                                rt = rt.strong();
                                            }
                                            ui.label(rt);
                                        }
                                        run_text.clear();
                                    }
                                }
                            });
                        }
                    });
            }
        }

        // Click to open fullscreen
        ui.add_space(4.0);
        if ui
            .add(
                egui::Button::new(
                    egui::RichText::new("Click to expand terminal")
                        .color(egui::Color32::from_rgb(100, 100, 100))
                        .size(11.0),
                )
                .frame(false),
            )
            .clicked()
        {
            self.active_panel = super::types::ActivePanel::Terminal;
        }
    }

    // ═══════════════════════════════════════════════════════════════
    // Fullscreen iTerm2-style terminal
    // ═══════════════════════════════════════════════════════════════

    pub(crate) fn render_terminal_fullscreen(&mut self, ctx: &egui::Context) {
        // Ensure at least one tab
        self.terminal.ensure_tab();

        // Update cursor blink
        let dt = ctx.input(|i| i.unstable_dt as f64);
        self.terminal.update_blink(dt);

        // Request continuous repaint for cursor blink + PTY output
        ctx.request_repaint();

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(TERM_BG).inner_margin(0.0))
            .show(ctx, |ui| {
                // ─── Tab bar ────────────────────────────────────
                self.render_tab_bar(ui);

                // ─── Search bar (if active) ─────────────────────
                self.render_search_bar(ui);

                // ─── Terminal grid ──────────────────────────────
                self.render_terminal_grid(ui);
            });
    }

    // ─── Tab bar ─────────────────────────────────────────────────

    fn render_tab_bar(&mut self, ui: &mut egui::Ui) {
        let tab_bar_rect = ui.available_rect_before_wrap();
        let bar_height = 32.0;
        let bar_rect = egui::Rect::from_min_size(
            tab_bar_rect.left_top(),
            egui::vec2(tab_bar_rect.width(), bar_height),
        );

        // Background
        ui.painter().rect_filled(bar_rect, 0.0, TAB_BAR_BG);

        // Bottom border
        ui.painter().line_segment(
            [bar_rect.left_bottom(), bar_rect.right_bottom()],
            egui::Stroke::new(1.0, TAB_BORDER),
        );

        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(bar_rect), |ui| {
            ui.horizontal_centered(|ui| {
                ui.add_space(8.0);
                ui.spacing_mut().item_spacing.x = 0.0;

                let mut tab_to_close: Option<usize> = None;
                let mut tab_to_activate: Option<usize> = None;

                for (idx, tab) in self.terminal.tabs.iter().enumerate() {
                    let is_active = idx == self.terminal.active_tab;
                    let title = tab.title();
                    let alive = tab.is_alive();

                    let tab_bg = if is_active {
                        TAB_ACTIVE_BG
                    } else {
                        TAB_INACTIVE_BG
                    };

                    let tab_width = 160.0_f32;
                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(tab_width, bar_height - 2.0),
                        egui::Sense::click(),
                    );

                    // Tab background
                    let rounding = egui::Rounding {
                        nw: 6.0,
                        ne: 6.0,
                        sw: 0.0,
                        se: 0.0,
                    };
                    ui.painter().rect_filled(rect, rounding, tab_bg);

                    if is_active {
                        // Active indicator line at bottom
                        ui.painter().line_segment(
                            [rect.left_bottom(), rect.right_bottom()],
                            egui::Stroke::new(2.0, egui::Color32::from_rgb(70, 130, 220)),
                        );
                    }

                    // Tab title
                    let display_title = if title.len() > 18 {
                        format!("{}...", &title[..15])
                    } else {
                        title.clone()
                    };

                    let title_color = if !alive {
                        egui::Color32::from_rgb(120, 60, 60)
                    } else if is_active {
                        egui::Color32::from_rgb(220, 220, 220)
                    } else {
                        egui::Color32::from_rgb(140, 140, 140)
                    };

                    // Shell icon
                    let icon_pos = rect.left_center() + egui::vec2(10.0, 0.0);
                    ui.painter().text(
                        icon_pos,
                        egui::Align2::LEFT_CENTER,
                        "\u{ea85}", // codicon: terminal
                        egui::FontId::proportional(12.0),
                        title_color,
                    );

                    // Title text
                    ui.painter().text(
                        rect.left_center() + egui::vec2(28.0, 0.0),
                        egui::Align2::LEFT_CENTER,
                        &display_title,
                        egui::FontId::proportional(12.0),
                        title_color,
                    );

                    // Close button
                    let close_rect = egui::Rect::from_center_size(
                        rect.right_center() - egui::vec2(14.0, 0.0),
                        egui::vec2(16.0, 16.0),
                    );
                    let close_response = ui.interact(
                        close_rect,
                        egui::Id::new(("tab_close", idx)),
                        egui::Sense::click(),
                    );

                    let close_color = if close_response.hovered() {
                        egui::Color32::from_rgb(220, 80, 80)
                    } else if is_active {
                        egui::Color32::from_rgb(140, 140, 140)
                    } else {
                        egui::Color32::from_rgb(80, 80, 80)
                    };

                    ui.painter().text(
                        close_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "\u{00d7}", // multiplication sign ×
                        egui::FontId::proportional(14.0),
                        close_color,
                    );

                    if close_response.clicked() {
                        tab_to_close = Some(idx);
                    } else if response.clicked() {
                        tab_to_activate = Some(idx);
                    }

                    // Separator between tabs
                    if !is_active
                        && idx + 1 < self.terminal.tabs.len()
                        && idx + 1 != self.terminal.active_tab
                    {
                        ui.painter().line_segment(
                            [
                                rect.right_top() + egui::vec2(0.0, 6.0),
                                rect.right_bottom() - egui::vec2(0.0, 6.0),
                            ],
                            egui::Stroke::new(1.0, egui::Color32::from_rgb(50, 50, 50)),
                        );
                    }
                }

                // "+" button
                ui.add_space(4.0);
                let plus_btn = ui.add(
                    egui::Button::new(
                        egui::RichText::new("+")
                            .color(egui::Color32::from_rgb(140, 140, 140))
                            .size(16.0),
                    )
                    .frame(false)
                    .min_size(egui::vec2(28.0, 28.0)),
                );
                if plus_btn.clicked() {
                    self.terminal.add_tab();
                }

                // Process deferred actions
                if let Some(idx) = tab_to_close {
                    self.terminal.close_tab(idx);
                    if self.terminal.tabs.is_empty() {
                        self.terminal.add_tab();
                    }
                } else if let Some(idx) = tab_to_activate {
                    self.terminal.active_tab = idx;
                }
            });
        });

        ui.add_space(bar_height);
    }

    // ─── Search bar ──────────────────────────────────────────────

    fn render_search_bar(&mut self, ui: &mut egui::Ui) {
        let search_active = self
            .terminal
            .active_tab()
            .map_or(false, |t| t.search_active);

        if !search_active {
            return;
        }

        let bar_rect = ui.available_rect_before_wrap();
        let bar_h = 32.0;
        let rect =
            egui::Rect::from_min_size(bar_rect.left_top(), egui::vec2(bar_rect.width(), bar_h));
        ui.painter()
            .rect_filled(rect, 0.0, egui::Color32::from_rgb(38, 38, 38));

        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(rect), |ui| {
            ui.horizontal_centered(|ui| {
                ui.add_space(12.0);
                ui.label(
                    egui::RichText::new("\u{eb1c}") // codicon: search
                        .size(14.0)
                        .color(egui::Color32::from_rgb(140, 140, 140)),
                );
                ui.add_space(4.0);

                if let Some(tab) = self.terminal.active_tab_mut() {
                    let text_edit = egui::TextEdit::singleline(&mut tab.search_query)
                        .font(egui::FontId::monospace(12.0))
                        .text_color(egui::Color32::from_rgb(220, 220, 220))
                        .desired_width(300.0)
                        .frame(false);
                    let resp = ui.add(text_edit);
                    if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        tab.search_active = false;
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(12.0);
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("\u{00d7}")
                                    .size(14.0)
                                    .color(egui::Color32::from_rgb(140, 140, 140)),
                            )
                            .frame(false),
                        )
                        .clicked()
                    {
                        if let Some(tab) = self.terminal.active_tab_mut() {
                            tab.search_active = false;
                        }
                    }
                });
            });
        });

        ui.add_space(bar_h);
    }

    // ─── Terminal grid rendering ─────────────────────────────────

    fn render_terminal_grid(&mut self, ui: &mut egui::Ui) {
        let available = ui.available_rect_before_wrap();

        // Calculate cell dimensions from monospace font
        let font_size = 13.5_f32;
        let cell_w = font_size * 0.6; // approximate monospace char width
        let cell_h = font_size * 1.4;
        let padding_x = 6.0;
        let padding_y = 4.0;

        let grid_cols = ((available.width() - padding_x * 2.0) / cell_w).max(10.0) as usize;
        let grid_rows = ((available.height() - padding_y * 2.0) / cell_h).max(4.0) as usize;

        // Resize PTY if dimensions changed
        if let Some(tab) = self.terminal.active_tab_mut() {
            tab.resize(grid_rows, grid_cols);
        }

        // Allocate the full area and handle input
        let (response, painter) =
            ui.allocate_painter(available.size(), egui::Sense::click_and_drag());
        let rect = response.rect;

        // Fill background
        painter.rect_filled(rect, 0.0, TERM_BG);

        // ─── Keyboard input → PTY ───────────────────────────────
        self.handle_terminal_keyboard(ui, &response);

        // ─── Mouse input (selection + scroll) ───────────────────
        self.handle_terminal_mouse(ui, &response, rect, cell_w, cell_h, padding_x, padding_y);

        // ─── Paint the grid ─────────────────────────────────────
        if let Some(tab) = self.terminal.active_tab() {
            if let Ok(grid) = tab.grid.lock() {
                let origin = rect.left_top() + egui::vec2(padding_x, padding_y);
                let font_id = egui::FontId::monospace(font_size);

                for row in 0..grid.rows.min(grid_rows) {
                    if row >= grid.cells.len() {
                        break;
                    }

                    let y = origin.y + row as f32 * cell_h;
                    let line = &grid.cells[row];

                    // Paint by style runs (batched for performance)
                    let mut run_start_col = 0;
                    let mut run_text = String::new();
                    let mut run_fg = TERM_FG;
                    let mut run_bg = TERM_BG;
                    let mut run_bold = false;
                    let mut run_underline = false;

                    for col in 0..=grid.cols.min(line.len()) {
                        let flush = if col < grid.cols.min(line.len()) {
                            let cell = &line[col];

                            // Check if selection overrides colors
                            let in_selection = tab
                                .selection
                                .as_ref()
                                .filter(|s| s.active)
                                .map_or(false, |s| s.contains(row, col));

                            let (cell_fg, cell_bg) = if in_selection {
                                (egui::Color32::WHITE, SELECTION_BG)
                            } else {
                                (cell.fg, cell.bg)
                            };

                            let same = cell_fg == run_fg
                                && cell_bg == run_bg
                                && cell.bold == run_bold
                                && cell.underline == run_underline;

                            if same {
                                run_text.push(cell.ch);
                                false
                            } else {
                                true
                            }
                        } else {
                            true // flush remaining
                        };

                        if flush && !run_text.is_empty() {
                            let x = origin.x + run_start_col as f32 * cell_w;

                            // Background rect
                            if run_bg != TERM_BG {
                                let bg_rect = egui::Rect::from_min_size(
                                    egui::pos2(x, y),
                                    egui::vec2(run_text.len() as f32 * cell_w, cell_h),
                                );
                                painter.rect_filled(bg_rect, 0.0, run_bg);
                            }

                            // Text
                            let text_pos = egui::pos2(x, y + 1.0);
                            let fid = if run_bold {
                                egui::FontId::monospace(font_size) // egui doesn't have bold monospace; we use color intensity
                            } else {
                                font_id.clone()
                            };
                            painter.text(text_pos, egui::Align2::LEFT_TOP, &run_text, fid, run_fg);

                            // Underline
                            if run_underline {
                                let ul_y = y + cell_h - 2.0;
                                painter.line_segment(
                                    [
                                        egui::pos2(x, ul_y),
                                        egui::pos2(x + run_text.len() as f32 * cell_w, ul_y),
                                    ],
                                    egui::Stroke::new(1.0, run_fg),
                                );
                            }

                            run_text.clear();
                            run_start_col = col;
                        }

                        if col < grid.cols.min(line.len()) {
                            let cell = &line[col];
                            let in_selection = tab
                                .selection
                                .as_ref()
                                .filter(|s| s.active)
                                .map_or(false, |s| s.contains(row, col));

                            if run_text.is_empty() {
                                run_start_col = col;
                                if in_selection {
                                    run_fg = egui::Color32::WHITE;
                                    run_bg = SELECTION_BG;
                                } else {
                                    run_fg = cell.fg;
                                    run_bg = cell.bg;
                                }
                                run_bold = cell.bold;
                                run_underline = cell.underline;
                            }
                            run_text.push(cell.ch);
                        }
                    }
                }

                // ─── Cursor ─────────────────────────────────────
                if grid.cursor_visible && self.terminal.cursor_visible_blink {
                    let cx = origin.x + grid.cursor_col as f32 * cell_w;
                    let cy = origin.y + grid.cursor_row as f32 * cell_h;
                    let cursor_rect =
                        egui::Rect::from_min_size(egui::pos2(cx, cy), egui::vec2(cell_w, cell_h));

                    // Block cursor with slight transparency
                    painter.rect_filled(
                        cursor_rect,
                        0.0,
                        egui::Color32::from_rgba_unmultiplied(192, 192, 192, 180),
                    );

                    // Draw the character under cursor in inverted color
                    if grid.cursor_row < grid.cells.len()
                        && grid.cursor_col < grid.cells[grid.cursor_row].len()
                    {
                        let ch = grid.cells[grid.cursor_row][grid.cursor_col].ch;
                        if ch != ' ' {
                            painter.text(
                                egui::pos2(cx, cy + 1.0),
                                egui::Align2::LEFT_TOP,
                                ch.to_string(),
                                font_id.clone(),
                                TERM_BG,
                            );
                        }
                    }
                }

                // ─── Scrollbar ──────────────────────────────────
                let total_lines = grid.scrollback.len() + grid.rows;
                if total_lines > grid.rows {
                    let scrollbar_x = rect.right() - 8.0;
                    let scrollbar_h = rect.height();
                    let thumb_h = (grid.rows as f32 / total_lines as f32 * scrollbar_h).max(20.0);
                    let scroll_frac =
                        1.0 - (grid.scroll_offset as f32 / grid.scrollback.len().max(1) as f32);
                    let thumb_y = rect.top() + scroll_frac * (scrollbar_h - thumb_h);

                    let thumb_rect = egui::Rect::from_min_size(
                        egui::pos2(scrollbar_x, thumb_y),
                        egui::vec2(6.0, thumb_h),
                    );
                    painter.rect_filled(thumb_rect, 3.0, SCROLLBAR_COLOR);
                }
            }
        }

        // ─── Context menu ───────────────────────────────────────
        response.context_menu(|ui| {
            if ui.button("Copy").clicked() {
                self.terminal_copy_selection();
                ui.close_menu();
            }
            if ui.button("Paste").clicked() {
                self.terminal_paste();
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Clear").clicked() {
                if let Some(tab) = self.terminal.active_tab() {
                    // Send Ctrl+L to clear
                    tab.write_to_pty(b"\x0c");
                }
                ui.close_menu();
            }
            if ui.button("Search").clicked() {
                if let Some(tab) = self.terminal.active_tab_mut() {
                    tab.search_active = true;
                }
                ui.close_menu();
            }
            ui.separator();
            if ui.button("New Tab").clicked() {
                self.terminal.add_tab();
                ui.close_menu();
            }
            if self.terminal.tabs.len() > 1 {
                if ui.button("Close Tab").clicked() {
                    let idx = self.terminal.active_tab;
                    self.terminal.close_tab(idx);
                    ui.close_menu();
                }
            }
        });
    }

    // ─── Keyboard handling ───────────────────────────────────────

    fn handle_terminal_keyboard(&mut self, ui: &mut egui::Ui, _response: &egui::Response) {
        let events: Vec<egui::Event> = ui.input(|i| i.events.clone());

        let tab = match self.terminal.active_tab() {
            Some(t) => t,
            None => return,
        };

        // Reset cursor blink on any input
        let mut had_input = false;

        for event in &events {
            match event {
                egui::Event::Text(text) => {
                    // Normal text input (not consumed by modifiers)
                    tab.write_to_pty(text.as_bytes());
                    had_input = true;
                }
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => {
                    had_input = true;
                    let app_cursor = if let Ok(g) = tab.grid.lock() {
                        g.app_cursor_keys
                    } else {
                        false
                    };

                    // Cmd+T → new tab
                    if modifiers.command && *key == egui::Key::T {
                        self.terminal.add_tab();
                        return;
                    }
                    // Cmd+W → close tab
                    if modifiers.command && *key == egui::Key::W {
                        let idx = self.terminal.active_tab;
                        self.terminal.close_tab(idx);
                        if self.terminal.tabs.is_empty() {
                            self.terminal.add_tab();
                        }
                        return;
                    }
                    // Cmd+F → search
                    if modifiers.command && *key == egui::Key::F {
                        if let Some(t) = self.terminal.active_tab_mut() {
                            t.search_active = !t.search_active;
                        }
                        return;
                    }
                    // Cmd+Shift+] → next tab
                    if modifiers.command && modifiers.shift && *key == egui::Key::CloseBracket {
                        if !self.terminal.tabs.is_empty() {
                            self.terminal.active_tab =
                                (self.terminal.active_tab + 1) % self.terminal.tabs.len();
                        }
                        return;
                    }
                    // Cmd+Shift+[ → prev tab
                    if modifiers.command && modifiers.shift && *key == egui::Key::OpenBracket {
                        if !self.terminal.tabs.is_empty() {
                            self.terminal.active_tab = if self.terminal.active_tab == 0 {
                                self.terminal.tabs.len() - 1
                            } else {
                                self.terminal.active_tab - 1
                            };
                        }
                        return;
                    }
                    // Cmd+C with selection → copy
                    if modifiers.command && *key == egui::Key::C {
                        if let Some(t) = self.terminal.active_tab() {
                            if t.selection.as_ref().map_or(false, |s| s.active) {
                                self.terminal_copy_selection();
                                return;
                            }
                        }
                        // No selection → send Ctrl+C (SIGINT)
                        tab.write_to_pty(&[0x03]);
                        return;
                    }
                    // Cmd+V → paste
                    if modifiers.command && *key == egui::Key::V {
                        self.terminal_paste();
                        return;
                    }

                    // Ctrl+key combinations
                    if modifiers.ctrl {
                        let ctrl_byte = match key {
                            egui::Key::A => Some(0x01),
                            egui::Key::B => Some(0x02),
                            egui::Key::C => Some(0x03),
                            egui::Key::D => Some(0x04),
                            egui::Key::E => Some(0x05),
                            egui::Key::F => Some(0x06),
                            egui::Key::G => Some(0x07),
                            egui::Key::H => Some(0x08),
                            egui::Key::I => Some(0x09),
                            egui::Key::J => Some(0x0A),
                            egui::Key::K => Some(0x0B),
                            egui::Key::L => Some(0x0C),
                            egui::Key::M => Some(0x0D),
                            egui::Key::N => Some(0x0E),
                            egui::Key::O => Some(0x0F),
                            egui::Key::P => Some(0x10),
                            egui::Key::Q => Some(0x11),
                            egui::Key::R => Some(0x12),
                            egui::Key::S => Some(0x13),
                            egui::Key::T => Some(0x14),
                            egui::Key::U => Some(0x15),
                            egui::Key::V => Some(0x16),
                            egui::Key::W => Some(0x17),
                            egui::Key::X => Some(0x18),
                            egui::Key::Y => Some(0x19),
                            egui::Key::Z => Some(0x1A),
                            _ => None,
                        };
                        if let Some(byte) = ctrl_byte {
                            tab.write_to_pty(&[byte]);
                            continue;
                        }
                    }

                    // Special keys
                    let seq: Option<&[u8]> = match key {
                        egui::Key::Enter => Some(b"\r"),
                        egui::Key::Backspace => Some(b"\x7f"),
                        egui::Key::Tab => Some(b"\t"),
                        egui::Key::Escape => Some(b"\x1b"),
                        egui::Key::ArrowUp => {
                            if app_cursor {
                                Some(b"\x1bOA")
                            } else {
                                Some(b"\x1b[A")
                            }
                        }
                        egui::Key::ArrowDown => {
                            if app_cursor {
                                Some(b"\x1bOB")
                            } else {
                                Some(b"\x1b[B")
                            }
                        }
                        egui::Key::ArrowRight => {
                            if app_cursor {
                                Some(b"\x1bOC")
                            } else {
                                Some(b"\x1b[C")
                            }
                        }
                        egui::Key::ArrowLeft => {
                            if app_cursor {
                                Some(b"\x1bOD")
                            } else {
                                Some(b"\x1b[D")
                            }
                        }
                        egui::Key::Home => Some(b"\x1b[H"),
                        egui::Key::End => Some(b"\x1b[F"),
                        egui::Key::PageUp => Some(b"\x1b[5~"),
                        egui::Key::PageDown => Some(b"\x1b[6~"),
                        egui::Key::Insert => Some(b"\x1b[2~"),
                        egui::Key::Delete => Some(b"\x1b[3~"),
                        egui::Key::F1 => Some(b"\x1bOP"),
                        egui::Key::F2 => Some(b"\x1bOQ"),
                        egui::Key::F3 => Some(b"\x1bOR"),
                        egui::Key::F4 => Some(b"\x1bOS"),
                        egui::Key::F5 => Some(b"\x1b[15~"),
                        egui::Key::F6 => Some(b"\x1b[17~"),
                        egui::Key::F7 => Some(b"\x1b[18~"),
                        egui::Key::F8 => Some(b"\x1b[19~"),
                        egui::Key::F9 => Some(b"\x1b[20~"),
                        egui::Key::F10 => Some(b"\x1b[21~"),
                        egui::Key::F11 => Some(b"\x1b[23~"),
                        egui::Key::F12 => Some(b"\x1b[24~"),
                        _ => None,
                    };

                    if let Some(seq) = seq {
                        tab.write_to_pty(seq);
                    }
                }
                _ => {}
            }
        }

        if had_input {
            self.terminal.cursor_blink_timer = 0.0;
            self.terminal.cursor_visible_blink = true;
            // Reset scroll to bottom on input
            if let Some(tab) = self.terminal.active_tab() {
                if let Ok(mut grid) = tab.grid.lock() {
                    grid.scroll_offset = 0;
                }
            }
        }
    }

    // ─── Mouse handling (selection + scroll) ─────────────────────

    fn handle_terminal_mouse(
        &mut self,
        ui: &mut egui::Ui,
        response: &egui::Response,
        rect: egui::Rect,
        cell_w: f32,
        cell_h: f32,
        padding_x: f32,
        padding_y: f32,
    ) {
        let origin = rect.left_top() + egui::vec2(padding_x, padding_y);

        // Mouse scroll → scrollback
        let scroll_delta = ui.input(|i| i.raw_scroll_delta.y);
        if scroll_delta != 0.0 {
            if let Some(tab) = self.terminal.active_tab() {
                if let Ok(mut grid) = tab.grid.lock() {
                    let lines = (scroll_delta / cell_h).round() as i32;
                    let max_scroll = grid.scrollback.len();
                    if lines > 0 {
                        // Scroll up (into scrollback)
                        grid.scroll_offset = (grid.scroll_offset + lines as usize).min(max_scroll);
                    } else {
                        // Scroll down (toward current)
                        grid.scroll_offset = grid.scroll_offset.saturating_sub((-lines) as usize);
                    }
                }
            }
        }

        // Mouse selection
        if let Some(pos) = response.interact_pointer_pos() {
            let col = ((pos.x - origin.x) / cell_w).max(0.0) as usize;
            let row = ((pos.y - origin.y) / cell_h).max(0.0) as usize;

            if response.drag_started() {
                if let Some(tab) = self.terminal.active_tab_mut() {
                    tab.selection = Some(Selection {
                        start_row: row,
                        start_col: col,
                        end_row: row,
                        end_col: col,
                        active: false,
                    });
                }
            } else if response.dragged() {
                if let Some(tab) = self.terminal.active_tab_mut() {
                    if let Some(sel) = &mut tab.selection {
                        sel.end_row = row;
                        sel.end_col = col;
                        sel.active = true;
                    }
                }
            }
        }

        // Clear selection on simple click (no drag)
        if response.clicked() && !response.dragged() {
            if let Some(tab) = self.terminal.active_tab_mut() {
                tab.selection = None;
            }
        }
    }

    // ─── Clipboard operations ────────────────────────────────────

    fn terminal_copy_selection(&mut self) {
        if let Some(tab) = self.terminal.active_tab() {
            if let Some(text) = tab.get_selected_text() {
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    let _ = clipboard.set_text(&text);
                }
            }
        }
    }

    fn terminal_paste(&mut self) {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            if let Ok(text) = clipboard.get_text() {
                if let Some(tab) = self.terminal.active_tab() {
                    // Bracketed paste if supported
                    let bracketed = if let Ok(g) = tab.grid.lock() {
                        g.bracketed_paste
                    } else {
                        false
                    };

                    if bracketed {
                        tab.write_to_pty(b"\x1b[200~");
                        tab.write_to_pty(text.as_bytes());
                        tab.write_to_pty(b"\x1b[201~");
                    } else {
                        tab.write_to_pty(text.as_bytes());
                    }
                }
            }
        }
    }
}
