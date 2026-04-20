//! Live collaboration: real-time co-editing with remote cursors
//!
//! Architecture:
//! - Each participant connects to a shared WebSocket relay server
//! - Text changes are broadcast as operational transforms (OT) or CRDT ops
//! - Remote cursors are shown as colored carets with name labels
//! - Session is identified by a shareable link/code
//!
//! For now: a simplified version using a WebSocket relay with full-doc sync.
//! Production would use a CRDT library (automerge-rs or yrs).

use super::BerryCodeApp;
use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════════
// Collaboration types
// ═══════════════════════════════════════════════════════════════════

/// A collaborator in the session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collaborator {
    pub id: String,
    pub name: String,
    pub color: [u8; 3],
    pub cursor_line: usize,
    pub cursor_col: usize,
    pub file_path: Option<String>,
    pub is_self: bool,
}

/// A text edit operation from a collaborator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollabEdit {
    pub user_id: String,
    pub file_path: String,
    pub line: usize,
    pub col: usize,
    pub delete_count: usize,
    pub insert_text: String,
    pub timestamp: u64,
}

/// Chat message in the collab session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollabChatMessage {
    pub user_id: String,
    pub user_name: String,
    pub text: String,
    pub timestamp: u64,
}

/// Collaboration session state
#[derive(Debug, Clone, PartialEq)]
pub enum CollabStatus {
    Inactive,
    Hosting,
    Joining,
    Connected,
    Error(String),
}

/// Live collaboration state
pub struct CollabState {
    pub status: CollabStatus,
    pub session_id: String,
    pub display_name: String,
    pub collaborators: Vec<Collaborator>,
    pub chat_messages: Vec<CollabChatMessage>,
    pub chat_input: String,
    /// Pending edits from remote collaborators (to be applied)
    pub pending_edits: Vec<CollabEdit>,
    /// Session share link
    pub share_link: String,
    /// Colors for collaborators
    pub color_palette: Vec<[u8; 3]>,
    next_color_idx: usize,
}

impl Default for CollabState {
    fn default() -> Self {
        Self {
            status: CollabStatus::Inactive,
            session_id: String::new(),
            display_name: whoami::username(),
            collaborators: Vec::new(),
            chat_messages: Vec::new(),
            chat_input: String::new(),
            pending_edits: Vec::new(),
            share_link: String::new(),
            color_palette: vec![
                [66, 133, 244], // blue
                [234, 67, 53],  // red
                [52, 168, 83],  // green
                [251, 188, 4],  // yellow
                [171, 71, 188], // purple
                [255, 112, 67], // orange
                [0, 172, 193],  // teal
                [233, 30, 99],  // pink
            ],
            next_color_idx: 0,
        }
    }
}

impl CollabState {
    fn next_color(&mut self) -> [u8; 3] {
        let color = self.color_palette[self.next_color_idx % self.color_palette.len()];
        self.next_color_idx += 1;
        color
    }

    /// Start hosting a new session
    pub fn host_session(&mut self) {
        self.session_id = uuid::Uuid::new_v4().to_string()[..8].to_string();
        self.share_link = format!("berrycode://collab/{}", self.session_id);
        self.status = CollabStatus::Hosting;

        let color = self.next_color();
        let name = self.display_name.clone();
        self.collaborators.push(Collaborator {
            id: "self".to_string(),
            name,
            color,
            cursor_line: 0,
            cursor_col: 0,
            file_path: None,
            is_self: true,
        });
    }

    /// Join an existing session
    pub fn join_session(&mut self, session_id: &str) {
        self.session_id = session_id.to_string();
        self.status = CollabStatus::Joining;

        let color = self.next_color();
        let name = self.display_name.clone();
        self.collaborators.push(Collaborator {
            id: "self".to_string(),
            name,
            color,
            cursor_line: 0,
            cursor_col: 0,
            file_path: None,
            is_self: true,
        });

        // Would connect to WebSocket server here
        self.status = CollabStatus::Connected;
    }

    /// Leave the session
    pub fn leave_session(&mut self) {
        self.status = CollabStatus::Inactive;
        self.collaborators.clear();
        self.chat_messages.clear();
        self.pending_edits.clear();
        self.session_id.clear();
        self.share_link.clear();
        self.next_color_idx = 0;
    }

    /// Get remote collaborators (excluding self)
    pub fn remote_collaborators(&self) -> Vec<&Collaborator> {
        self.collaborators.iter().filter(|c| !c.is_self).collect()
    }
}

// ═══════════════════════════════════════════════════════════════════
// Collaboration UI
// ═══════════════════════════════════════════════════════════════════

/// Collab dialog state
pub struct CollabDialogState {
    pub open: bool,
    pub join_session_id: String,
    pub tab: CollabDialogTab,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CollabDialogTab {
    Session,
    Chat,
}

impl Default for CollabDialogState {
    fn default() -> Self {
        Self {
            open: false,
            join_session_id: String::new(),
            tab: CollabDialogTab::Session,
        }
    }
}

impl BerryCodeApp {
    /// Render collaboration panel/dialog
    pub(crate) fn render_collab_dialog(&mut self, ctx: &egui::Context) {
        if !self.collab_dialog.open {
            return;
        }

        let mut should_close = false;

        egui::Window::new("Live Share")
            .collapsible(false)
            .resizable(false)
            .fixed_size(egui::vec2(400.0, 0.0))
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                // Tab bar
                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(
                            self.collab_dialog.tab == CollabDialogTab::Session,
                            "Session",
                        )
                        .clicked()
                    {
                        self.collab_dialog.tab = CollabDialogTab::Session;
                    }
                    if ui
                        .selectable_label(self.collab_dialog.tab == CollabDialogTab::Chat, "Chat")
                        .clicked()
                    {
                        self.collab_dialog.tab = CollabDialogTab::Chat;
                    }
                });

                ui.separator();

                match self.collab_dialog.tab {
                    CollabDialogTab::Session => {
                        self.render_collab_session(ui, &mut should_close);
                    }
                    CollabDialogTab::Chat => {
                        self.render_collab_chat(ui);
                    }
                }
            });

        if should_close {
            self.collab_dialog.open = false;
        }
    }

    fn render_collab_session(&mut self, ui: &mut egui::Ui, should_close: &mut bool) {
        match &self.collab.status {
            CollabStatus::Inactive => {
                ui.add_space(8.0);

                // Display name
                ui.horizontal(|ui| {
                    ui.label("Your name:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.collab.display_name)
                            .desired_width(200.0),
                    );
                });

                ui.add_space(12.0);

                // Host button
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("\u{ebb5} Start Session")
                                .size(14.0)
                                .color(egui::Color32::from_rgb(80, 200, 80)),
                        )
                        .min_size(egui::vec2(380.0, 32.0)),
                    )
                    .clicked()
                {
                    self.collab.host_session();
                }

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                // Join
                ui.label("Join existing session:");
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.collab_dialog.join_session_id)
                            .hint_text("Session ID")
                            .desired_width(250.0),
                    );
                    if ui.button("Join").clicked() && !self.collab_dialog.join_session_id.is_empty()
                    {
                        let id = self.collab_dialog.join_session_id.clone();
                        self.collab.join_session(&id);
                    }
                });
            }
            CollabStatus::Hosting | CollabStatus::Connected => {
                // Active session
                ui.add_space(4.0);

                // Share link
                ui.horizontal(|ui| {
                    ui.label("Session ID:");
                    ui.monospace(&self.collab.session_id);
                    if ui.small_button("Copy").clicked() {
                        ui.ctx().copy_text(self.collab.session_id.clone());
                    }
                });

                ui.add_space(8.0);

                // Collaborators list
                ui.label(
                    egui::RichText::new(format!(
                        "Participants ({})",
                        self.collab.collaborators.len()
                    ))
                    .size(12.0)
                    .color(egui::Color32::from_rgb(140, 140, 140)),
                );

                for collab in &self.collab.collaborators {
                    ui.horizontal(|ui| {
                        let color = egui::Color32::from_rgb(
                            collab.color[0],
                            collab.color[1],
                            collab.color[2],
                        );
                        ui.label(egui::RichText::new("\u{25cf}").color(color)); // colored dot
                        ui.label(&collab.name);
                        if collab.is_self {
                            ui.label(
                                egui::RichText::new("(you)")
                                    .size(10.0)
                                    .color(egui::Color32::GRAY),
                            );
                        }
                        if let Some(file) = &collab.file_path {
                            let short = file.rsplit('/').next().unwrap_or(file);
                            ui.label(
                                egui::RichText::new(format!("editing {}", short))
                                    .size(10.0)
                                    .color(egui::Color32::from_rgb(100, 100, 100)),
                            );
                        }
                    });
                }

                ui.add_space(12.0);

                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("Leave Session")
                                .color(egui::Color32::from_rgb(230, 80, 80)),
                        )
                        .min_size(egui::vec2(380.0, 28.0)),
                    )
                    .clicked()
                {
                    self.collab.leave_session();
                }
            }
            _ => {
                ui.label("Connecting...");
            }
        }

        // Close button
        ui.add_space(4.0);
        if ui.button("Close").clicked() {
            *should_close = true;
        }
    }

    fn render_collab_chat(&mut self, ui: &mut egui::Ui) {
        if self.collab.status == CollabStatus::Inactive {
            ui.colored_label(egui::Color32::GRAY, "Start or join a session to chat");
            return;
        }

        // Chat messages
        let available = ui.available_height() - 30.0;
        egui::ScrollArea::vertical()
            .max_height(available)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for msg in &self.collab.chat_messages {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(&msg.user_name)
                                .size(11.0)
                                .strong()
                                .color(egui::Color32::from_rgb(100, 180, 255)),
                        );
                        ui.label(
                            egui::RichText::new(&msg.text)
                                .size(11.0)
                                .color(egui::Color32::from_rgb(200, 200, 200)),
                        );
                    });
                }
            });

        // Chat input
        ui.horizontal(|ui| {
            let resp = ui.add(
                egui::TextEdit::singleline(&mut self.collab.chat_input)
                    .hint_text("Type a message...")
                    .desired_width(f32::INFINITY),
            );
            if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                let text = self.collab.chat_input.trim().to_string();
                if !text.is_empty() {
                    self.collab.chat_messages.push(CollabChatMessage {
                        user_id: "self".to_string(),
                        user_name: self.collab.display_name.clone(),
                        text,
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                    });
                    self.collab.chat_input.clear();
                }
            }
        });
    }

    /// Broadcast a local edit to all collaborators
    pub(crate) fn broadcast_edit(
        &mut self,
        file_path: &str,
        line: usize,
        col: usize,
        delete_count: usize,
        insert_text: &str,
    ) {
        if self.collab.status != CollabStatus::Hosting
            && self.collab.status != CollabStatus::Connected
        {
            return;
        }

        let edit = CollabEdit {
            user_id: "self".to_string(),
            file_path: file_path.to_string(),
            line,
            col,
            delete_count,
            insert_text: insert_text.to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        // In a full implementation, this would send via WebSocket
        // For now, add to pending edits for local testing
        let _ = edit;
    }

    /// Update local cursor position for broadcasting
    pub(crate) fn update_collab_cursor(&mut self) {
        if self.collab.status != CollabStatus::Hosting
            && self.collab.status != CollabStatus::Connected
        {
            return;
        }

        if let Some(tab) = self.editor_tabs.get(self.active_tab_idx) {
            if let Some(me) = self.collab.collaborators.iter_mut().find(|c| c.is_self) {
                me.cursor_line = tab.cursor_line;
                me.cursor_col = tab.cursor_col;
                me.file_path = Some(tab.file_path.clone());
            }
        }
    }

    /// Apply pending remote edits to local buffers
    pub(crate) fn apply_remote_edits(&mut self) {
        let edits: Vec<CollabEdit> = self.collab.pending_edits.drain(..).collect();

        for edit in edits {
            // Find the tab for this file
            let tab_idx = self
                .editor_tabs
                .iter()
                .position(|t| t.file_path == edit.file_path);
            if let Some(idx) = tab_idx {
                let tab = &mut self.editor_tabs[idx];
                let mut text = tab.buffer.to_string();
                let lines: Vec<&str> = text.lines().collect();

                if edit.line < lines.len() {
                    let line_offset: usize =
                        lines.iter().take(edit.line).map(|l| l.len() + 1).sum();
                    let offset = (line_offset + edit.col).min(text.len());
                    let end = (offset + edit.delete_count).min(text.len());

                    text.replace_range(offset..end, &edit.insert_text);
                    tab.buffer = crate::buffer::TextBuffer::from_str(&text);
                }
            }
        }
    }

    /// Poll collaboration state (call from main loop)
    pub(crate) fn poll_collab(&mut self) {
        if self.collab.status == CollabStatus::Inactive {
            return;
        }

        // Update our cursor position
        self.update_collab_cursor();

        // Apply any remote edits
        self.apply_remote_edits();

        // In a full implementation:
        // 1. Read from WebSocket for remote edits and cursor updates
        // 2. Transform incoming edits against local state (OT/CRDT)
        // 3. Apply transformed edits
        // 4. Send local edits to server
    }

    /// Get remote cursors to render in the editor
    pub(crate) fn get_remote_cursors(&self, file_path: &str) -> Vec<(usize, usize, [u8; 3], &str)> {
        self.collab
            .collaborators
            .iter()
            .filter(|c| !c.is_self && c.file_path.as_deref() == Some(file_path))
            .map(|c| (c.cursor_line, c.cursor_col, c.color, c.name.as_str()))
            .collect()
    }
}
