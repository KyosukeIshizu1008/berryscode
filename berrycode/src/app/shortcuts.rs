//! Editor keyboard shortcuts and file operations

use super::types::ActivePanel;
use super::BerryCodeApp;
use crate::app::keymap::KeyAction;
use crate::focus_stack::FocusLayer;
use crate::native;

impl BerryCodeApp {
    /// Handle global keyboard shortcuts
    pub(crate) fn handle_editor_shortcuts(&mut self, ctx: &egui::Context) {
        // Panel switching: Ctrl+1..9 (Cmd+number is intercepted by macOS)
        ctx.input(|i| {
            if i.modifiers.ctrl {
                if i.key_pressed(egui::Key::Num1) {
                    self.active_panel = ActivePanel::Explorer;
                }
                if i.key_pressed(egui::Key::Num2) {
                    self.active_panel = ActivePanel::Search;
                }
                if i.key_pressed(egui::Key::Num3) {
                    self.active_panel = ActivePanel::Git;
                }
                if i.key_pressed(egui::Key::Num4) {
                    self.active_panel = ActivePanel::Terminal;
                }
                if i.key_pressed(egui::Key::Num5) {
                    self.active_panel = ActivePanel::EcsInspector;
                }
                if i.key_pressed(egui::Key::Num6) {
                    self.active_panel = ActivePanel::BevyTemplates;
                }
                if i.key_pressed(egui::Key::Num7) {
                    self.active_panel = ActivePanel::AssetBrowser;
                }
                if i.key_pressed(egui::Key::Num8) {
                    self.active_panel = ActivePanel::SceneEditor;
                }
            }
        });

        // Scene editor has its own Cmd+S binding; dispatch before falling
        // through to the regular editor shortcuts so we don't trample an
        // in-memory scene by "saving" an unrelated focused tab.
        if self.active_panel == ActivePanel::SceneEditor {
            self.handle_scene_editor_shortcuts(ctx);
            return;
        }

        // Only handle shortcuts when editor is focused
        if self.active_focus != FocusLayer::Editor {
            return;
        }

        // Skip if no tabs open
        if self.editor_tabs.is_empty() {
            return;
        }

        // Clone keymap so we can query it inside the closure while still
        // mutating other fields on `self`.
        let keymap = self.keymap.clone();

        ctx.input(|i| {
            // Ctrl+F / Cmd+F: Open search dialog
            if keymap.is_pressed(KeyAction::Find, i) {
                self.search_dialog_open = true;
                self.show_replace = false;
                self.search_results.clear();
            }

            // Ctrl+H / Cmd+H: Open replace dialog
            if keymap.is_pressed(KeyAction::Replace, i) {
                self.search_dialog_open = true;
                self.show_replace = true;
                self.search_results.clear();
            }

            // Ctrl+S / Cmd+S: Save file
            if keymap.is_pressed(KeyAction::Save, i) {
                self.save_current_file();
            }

            // Ctrl+Shift+F / Cmd+Shift+F: Format file
            if keymap.is_pressed(KeyAction::Format, i) {
                self.format_current_file();
            }

            // Ctrl+Z / Cmd+Z: Undo
            if keymap.is_pressed(KeyAction::Undo, i) {
                if let Some(_tab) = self.editor_tabs.get_mut(self.active_tab_idx) {
                    tracing::info!("⏪ Undo requested (full implementation in later phase)");
                }
            }

            // Ctrl+Shift+Z / Cmd+Shift+Z: Redo
            if keymap.is_pressed(KeyAction::Redo, i) {
                if let Some(_tab) = self.editor_tabs.get_mut(self.active_tab_idx) {
                    tracing::info!("⏩ Redo requested (full implementation in later phase)");
                }
            }

            // Ctrl+Shift+D: Duplicate current line
            if keymap.is_pressed(KeyAction::DuplicateLine, i) {
                if let Some(tab) = self.editor_tabs.get_mut(self.active_tab_idx) {
                    let text = tab.get_text().to_string();
                    let lines: Vec<&str> = text.lines().collect();
                    if tab.cursor_line < lines.len() {
                        let line_content = lines[tab.cursor_line].to_string();
                        let insert_pos = tab.buffer.line_to_char(tab.cursor_line + 1);
                        let new_line = format!("{}\n", line_content);
                        tab.buffer
                            .insert(insert_pos.min(tab.buffer.len_chars()), &new_line);
                        tab.text_cache_version = 0; // invalidate cache
                        tab.is_dirty = true;
                    }
                }
            }

            // Ctrl+D (without Shift): Add cursor at next occurrence of selected word
            if keymap.is_pressed(KeyAction::AddCursorNext, i) {
                self.add_cursor_at_next_occurrence();
            }

            // Escape: Clear multi-cursors and close peek definition
            if keymap.is_pressed(KeyAction::Escape, i) {
                if !self.multi_cursors.is_empty() {
                    self.multi_cursors.clear();
                }
                if self.peek_definition.is_some() {
                    self.close_peek_definition();
                }
            }

            // Alt+F12: Peek definition (instead of jumping)
            if keymap.is_pressed(KeyAction::PeekDefinition, i) {
                self.open_peek_definition();
            }

            // Ctrl+Shift+[: Fold current block
            if keymap.is_pressed(KeyAction::FoldBlock, i) {
                if let Some(tab) = self.editor_tabs.get(self.active_tab_idx) {
                    let line = tab.cursor_line;
                    // Check if this line has a foldable block
                    let text = tab.text_cache.clone();
                    let lines: Vec<&str> = text.lines().collect();
                    if line < lines.len() && lines[line].contains('{') {
                        self.toggle_fold_at_line(line);
                    }
                }
            }

            // Ctrl+Shift+]: Unfold current block
            if keymap.is_pressed(KeyAction::UnfoldBlock, i) {
                if let Some(tab) = self.editor_tabs.get(self.active_tab_idx) {
                    let line = tab.cursor_line;
                    // If this line is folded, unfold it
                    let is_folded = tab.folded_regions.iter().any(|(s, _)| *s == line);
                    if is_folded {
                        self.toggle_fold_at_line(line);
                    }
                }
            }

            // F2: Rename symbol
            if keymap.is_pressed(KeyAction::Rename, i) {
                self.open_rename_dialog();
            }

            // F5: Start/Continue debugging
            if keymap.is_pressed(KeyAction::StartDebug, i) {
                if self.debug_state.active {
                    self.debug_continue();
                } else {
                    self.start_debug();
                }
            }

            // F9: Toggle breakpoint
            if keymap.is_pressed(KeyAction::ToggleBreakpoint, i) {
                self.toggle_breakpoint();
            }

            // Cmd+R: Run Bevy project
            if keymap.is_pressed(KeyAction::RunProject, i) {
                if self.run_process.is_some() {
                    self.stop_run();
                } else {
                    self.start_run();
                }
            }

            // Note: Ctrl+C/V/X are handled by egui::TextEdit automatically
        });
    }

    /// Save current file
    pub(crate) fn save_current_file(&mut self) {
        if let Some(tab) = self.editor_tabs.get(self.active_tab_idx) {
            let content = tab.buffer.to_string();
            let file_path = tab.file_path.clone();
            match native::fs::write_file(&file_path, &content) {
                Ok(_) => {
                    tracing::info!("💾 File saved: {} ({} bytes)", file_path, content.len());

                    // Notify LSP about the save (textDocument/didSave)
                    if let Some(lang) =
                        crate::native::lsp_native::detect_server_language(&file_path)
                    {
                        if let Some(client) = &self.lsp_native_client {
                            let client = client.clone();
                            let path = file_path.clone();
                            let language = lang.to_string();
                            self.lsp_runtime.spawn(async move {
                                let _ = client.save_file(&language, &path).await;
                            });
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("❌ Failed to save file {}: {}", file_path, e);
                }
            }
        }
    }

    /// Format current file using language-specific formatter
    pub(crate) fn format_current_file(&mut self) {
        if let Some(tab) = self.editor_tabs.get_mut(self.active_tab_idx) {
            tracing::info!("✨ Formatting file: {}", tab.file_path);

            // Save file first
            let content = tab.buffer.to_string();
            if let Err(e) = native::fs::write_file(&tab.file_path, &content) {
                tracing::error!("❌ Failed to save before formatting: {}", e);
                return;
            }

            // Run formatter based on file extension
            let formatter_result = if tab.file_path.ends_with(".rs") {
                std::process::Command::new("rustfmt")
                    .arg(&tab.file_path)
                    .output()
            } else {
                tracing::warn!("⚠️  No formatter configured for {}", tab.file_path);
                return;
            };

            match formatter_result {
                Ok(output) => {
                    if output.status.success() {
                        match native::fs::read_file(&tab.file_path) {
                            Ok(formatted_content) => {
                                tab.buffer =
                                    crate::buffer::TextBuffer::from_str(&formatted_content);
                                tracing::info!("✅ File formatted successfully");

                                // Logged via tracing above
                            }
                            Err(e) => {
                                tracing::error!("❌ Failed to reload formatted file: {}", e);
                            }
                        }
                    } else {
                        let error_msg = String::from_utf8_lossy(&output.stderr);
                        tracing::error!("❌ Formatter error: {}", error_msg);

                        // Logged via tracing above
                    }
                }
                Err(e) => {
                    tracing::error!("❌ Failed to run formatter: {}", e);
                    // Logged via tracing above
                }
            }
        }
    }

    /// Add a cursor at the next occurrence of the word under the primary cursor (Ctrl+D)
    pub(crate) fn add_cursor_at_next_occurrence(&mut self) {
        let tab = match self.editor_tabs.get(self.active_tab_idx) {
            Some(t) => t,
            None => return,
        };

        let text = tab.text_cache.clone();
        let chars: Vec<char> = text.chars().collect();

        // Determine the word under cursor
        // Use cursor_line and cursor_col to find char offset
        let mut cursor_char_offset = 0;
        for (idx, line) in text.lines().enumerate() {
            if idx == tab.cursor_line {
                cursor_char_offset += tab.cursor_col.min(line.len());
                break;
            }
            cursor_char_offset += line.len() + 1; // +1 for newline
        }

        // Find word boundaries at cursor
        let mut word_start = cursor_char_offset;
        while word_start > 0
            && (chars
                .get(word_start - 1)
                .map_or(false, |c| c.is_alphanumeric() || *c == '_'))
        {
            word_start -= 1;
        }
        let mut word_end = cursor_char_offset;
        while word_end < chars.len()
            && (chars[word_end].is_alphanumeric() || chars[word_end] == '_')
        {
            word_end += 1;
        }

        if word_start == word_end {
            return; // no word under cursor
        }

        let word: String = chars[word_start..word_end].iter().collect();

        // Search for next occurrence after the last known cursor position
        let search_start = self.multi_cursors.last().copied().unwrap_or(word_end);
        if let Some(pos) = text[search_start..].find(&word) {
            let abs_pos = search_start + pos;
            // Don't add duplicate
            if !self.multi_cursors.contains(&abs_pos) {
                self.multi_cursors.push(abs_pos);
            }
        } else {
            // Wrap around: search from the beginning
            if let Some(pos) = text.find(&word) {
                if !self.multi_cursors.contains(&pos) && pos != word_start {
                    self.multi_cursors.push(pos);
                }
            }
        }
    }

    /// Keyboard shortcuts that fire while the Scene Editor panel is active.
    fn handle_scene_editor_shortcuts(&mut self, ctx: &egui::Context) {
        // Suppress shortcuts while a text field (e.g. inline rename) has
        // keyboard focus, otherwise typing letters like "d" would delete the
        // selection.
        if ctx.wants_keyboard_input() {
            // Cmd+S still goes through even when typing in the rename buffer
            // would be surprising for "save scene", so just bail entirely.
            // Save is also exposed in the toolbar.
            return;
        }

        let keymap = self.keymap.clone();

        let mut save_requested = false;
        let mut duplicate_requested = false;
        let mut delete_requested = false;
        let mut rename_requested = false;
        let mut undo_requested = false;
        let mut redo_requested = false;

        ctx.input(|i| {
            if keymap.is_pressed(KeyAction::Save, i) {
                save_requested = true;
            }
            if keymap.is_pressed(KeyAction::DuplicateEntity, i) {
                duplicate_requested = true;
            }
            if keymap.is_pressed(KeyAction::DeleteEntity, i) || i.key_pressed(egui::Key::Backspace)
            {
                delete_requested = true;
            }
            if keymap.is_pressed(KeyAction::Rename, i) {
                rename_requested = true;
            }
            if keymap.is_pressed(KeyAction::Undo, i) {
                undo_requested = true;
            }
            if keymap.is_pressed(KeyAction::Redo, i) {
                redo_requested = true;
            }
        });

        if save_requested {
            self.save_current_scene();
        }

        if duplicate_requested {
            if !self.scene_model.selected_ids.is_empty() {
                self.scene_snapshot();
                let ids: Vec<u64> = self.scene_model.selected_ids.iter().copied().collect();
                self.scene_model.select_clear();
                let mut last_new = None;
                for sel in ids {
                    if let Some(new_id) = self.scene_model.duplicate_entity(sel) {
                        self.scene_model.select_add(new_id);
                        last_new = Some(new_id);
                    }
                }
                self.primary_selected_id = last_new;
                self.scene_needs_sync = true;
            }
        }

        if delete_requested {
            if !self.scene_model.selected_ids.is_empty() {
                self.scene_snapshot();
                let ids: Vec<u64> = self.scene_model.selected_ids.iter().copied().collect();
                for sel in ids {
                    self.scene_model.remove_entity(sel);
                }
                self.scene_model.select_clear();
                self.primary_selected_id = None;
                self.scene_needs_sync = true;
            }
        }

        if rename_requested {
            if let Some(sel) = self.primary_selected_id {
                if self.scene_model.is_selected(sel) {
                    if let Some(entity) = self.scene_model.entities.get(&sel) {
                        self.renaming_entity_id = Some(sel);
                        self.rename_buffer = entity.name.clone();
                    }
                }
            }
        }

        if undo_requested {
            if let Some(prev) = self.command_history.undo(&self.scene_model) {
                self.scene_model = prev;
                self.scene_needs_sync = true;
            }
        }

        if redo_requested {
            if let Some(next) = self.command_history.redo(&self.scene_model) {
                self.scene_model = next;
                self.scene_needs_sync = true;
            }
        }
    }

    /// Save the current scene to its `file_path`, falling back to
    /// `<root>/scenes/scene.bscene` if the scene has never been saved.
    pub(crate) fn save_current_scene(&mut self) {
        let path = match &self.scene_model.file_path {
            Some(p) => p.clone(),
            None => format!("{}/scenes/scene.bscene", self.root_path),
        };

        // Ensure the parent directory exists.
        if let Some(parent) = std::path::Path::new(&path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        match crate::app::scene_editor::serialization::save_scene_to_ron(&self.scene_model, &path) {
            Ok(_) => {
                self.scene_model.file_path = Some(path.clone());
                self.scene_model.modified = false;
                self.status_message = format!("Scene saved: {}", path);
                self.status_message_timestamp = Some(std::time::Instant::now());
                tracing::info!("Scene saved: {}", path);

                // Auto-generate Rust code alongside the scene
                match crate::app::scene_editor::codegen::save_scene_code(&self.scene_model, &path) {
                    Ok(rs_path) => {
                        tracing::info!("Code generated: {}", rs_path);

                        // Run cargo check in background after generating scene code
                        let (tx, rx) = std::sync::mpsc::channel();
                        self.cargo_check_rx = Some(rx);
                        let project_root = self.root_path.clone();
                        std::thread::spawn(move || {
                            let _ = tx.send("[cargo check] Running...".to_string());
                            let output = std::process::Command::new("cargo")
                                .arg("check")
                                .current_dir(&project_root)
                                .stderr(std::process::Stdio::piped())
                                .stdout(std::process::Stdio::piped())
                                .output();
                            match output {
                                Ok(out) => {
                                    if out.status.success() {
                                        let _ = tx.send("[cargo check] OK - no errors".to_string());
                                    } else {
                                        let stderr = String::from_utf8_lossy(&out.stderr);
                                        for line in stderr.lines() {
                                            let _ = tx.send(format!("[cargo check] {}", line));
                                        }
                                    }
                                }
                                Err(e) => {
                                    let _ = tx.send(format!("[cargo check] Failed: {}", e));
                                }
                            }
                        });
                    }
                    Err(e) => {
                        tracing::warn!("Code generation failed: {}", e);
                    }
                }
            }
            Err(e) => {
                self.status_message = format!("Save failed: {}", e);
                self.status_message_timestamp = Some(std::time::Instant::now());
                tracing::error!("❌ Scene save failed: {:#}", e);
            }
        }
    }

    /// Load a scene from disk, replacing the current `SceneModel` and switching
    /// the active panel to the Scene Editor.
    pub(crate) fn load_scene(&mut self, path: &str) {
        match crate::app::scene_editor::serialization::load_scene_from_ron(path) {
            Ok(scene) => {
                self.scene_model = scene;
                self.scene_needs_sync = true;
                self.active_panel = ActivePanel::SceneEditor;
                self.status_message = format!("Scene loaded: {}", path);
                self.status_message_timestamp = Some(std::time::Instant::now());
                tracing::info!("📂 Scene loaded: {}", path);
            }
            Err(e) => {
                self.status_message = format!("Load failed: {}", e);
                self.status_message_timestamp = Some(std::time::Instant::now());
                tracing::error!("❌ Scene load failed: {:#}", e);
            }
        }
    }
}
