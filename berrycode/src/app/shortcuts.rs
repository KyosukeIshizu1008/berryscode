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

                // Patch main.rs alongside any legacy save (codegen
                // dispatch itself is in `run_codegen_for_save` so the
                // "exactly one .rs file per scene" contract is unit-
                // testable without standing up `BerryCodeApp`).
                let use_modular =
                    crate::app::scene_editor::codegen::has_modular_structure(&self.root_path);
                if !use_modular {
                    let main_rs_path = format!("{}/src/main.rs", self.root_path);
                    if std::path::Path::new(&main_rs_path).exists() {
                        if let Ok(main_code) = std::fs::read_to_string(&main_rs_path) {
                            let updated = crate::app::scene_editor::codegen::patch_main_rs_setup(
                                &main_code,
                                &self.scene_model,
                            );
                            if updated != main_code {
                                let _ = std::fs::write(&main_rs_path, &updated);
                                tracing::info!("Updated main.rs with scene entities (legacy)");
                            }
                        }
                    }
                }
                let codegen_result =
                    run_codegen_for_save(&self.scene_model, &path, &self.root_path);

                match codegen_result {
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

    /// Reload the active scene in-place from disk. Used by the asset
    /// watcher's hot-reload path; the active tab's model + label are
    /// overwritten with the freshly-parsed file.
    pub(crate) fn load_scene(&mut self, path: &str) {
        match crate::app::scene_editor::serialization::load_scene_from_ron(path) {
            Ok(scene) => {
                self.scene_model = scene.clone();
                self.scene_needs_sync = true;
                self.active_panel = ActivePanel::SceneEditor;
                self.current_scene_path = Some(path.to_string());
                let label = scene_label_from_path(path);
                if let Some(tab) = self.scene_tabs.get_mut(self.active_scene_tab) {
                    tab.model = scene;
                    tab.label = label;
                }
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

    /// Open a scene in a new tab — or, if the same file is already
    /// open, just switch to that tab. Snapshots the active tab first
    /// so switching back doesn't lose unsaved edits.
    /// Currently unused on the UI side (the Open Scene dropdown was
    /// retired in favour of file-tree double-click), but kept for
    /// future "Open in new tab" wiring.
    #[allow(dead_code)]
    pub(crate) fn open_scene_in_new_tab(&mut self, path: &str) {
        // Dedupe: if any tab already points at this file, just
        // activate it instead of pushing a duplicate. Two tabs labelled
        // the same with one of them empty was the giveaway.
        if let Some(existing) = self
            .scene_tabs
            .iter()
            .position(|t| t.model.file_path.as_deref() == Some(path))
        {
            if let Some(tab) = self.scene_tabs.get_mut(self.active_scene_tab) {
                tab.model = self.scene_model.clone();
            }
            self.active_scene_tab = existing;
            self.scene_model = self.scene_tabs[existing].model.clone();
            self.scene_needs_sync = true;
            self.active_panel = ActivePanel::SceneEditor;
            self.current_scene_path = Some(path.to_string());
            self.primary_selected_id = None;
            return;
        }

        match crate::app::scene_editor::serialization::load_scene_from_ron(path) {
            Ok(scene) => {
                if let Some(tab) = self.scene_tabs.get_mut(self.active_scene_tab) {
                    tab.model = self.scene_model.clone();
                }
                let label = scene_label_from_path(path);
                self.scene_tabs
                    .push(crate::app::scene_editor::scene_tabs::SceneTab::new(
                        scene.clone(),
                        label,
                    ));
                self.active_scene_tab = self.scene_tabs.len() - 1;
                self.scene_model = scene;
                self.scene_needs_sync = true;
                self.active_panel = ActivePanel::SceneEditor;
                self.current_scene_path = Some(path.to_string());
                self.primary_selected_id = None;
                self.status_message = format!("Scene loaded: {}", path);
                self.status_message_timestamp = Some(std::time::Instant::now());
                tracing::info!("📂 Scene loaded into new tab: {}", path);
            }
            Err(e) => {
                self.status_message = format!("Load failed: {}", e);
                self.status_message_timestamp = Some(std::time::Instant::now());
                tracing::error!("❌ Scene load failed: {:#}", e);
            }
        }
    }

    /// Drain pending events from the asset watcher and react. Live-
    /// reload the active scene when its `.bscene` file changes on
    /// disk; surface a status message for shader changes (Bevy's own
    /// asset hot-reload picks them up — we just confirm we noticed).
    /// Called once per frame from `berry_ui_system`.
    pub(crate) fn poll_asset_watcher(&mut self) {
        // Make sure the watcher is rooted at the current project. This
        // is idempotent and handles the "user opened a different
        // project" case for free.
        let root = std::path::PathBuf::from(self.root_path.clone());
        if root.is_dir() {
            self.asset_watcher.ensure_watching(&root);
        }

        let events = self.asset_watcher.drain();
        for event in events {
            match event {
                crate::app::asset_watcher::AssetEvent::SceneChanged(p) => {
                    let path_str = p.to_string_lossy().to_string();
                    // Only auto-reload if it's the scene the user is
                    // currently viewing — silently re-parsing every
                    // .bscene in the project would be surprising.
                    if self.current_scene_path.as_deref() == Some(path_str.as_str()) {
                        tracing::info!("Hot-reload: re-parsing active scene {}", path_str);
                        // Re-use `load_scene` so the status message
                        // and panel-switch behaviour stay consistent
                        // with manual loads.
                        self.load_scene(&path_str);
                    } else {
                        self.status_message = format!("Scene file changed: {}", path_str);
                        self.status_message_timestamp = Some(std::time::Instant::now());
                    }
                }
                crate::app::asset_watcher::AssetEvent::SceneRemoved(p) => {
                    // The .bscene was deleted on disk — drop any open
                    // tabs pointing at it. Care needed: only refresh
                    // `self.scene_model` from a tab if the *active*
                    // tab was the one closed; otherwise we'd clobber
                    // the user's in-flight edits with a stale tab
                    // snapshot.
                    let path_str = p.to_string_lossy().to_string();
                    let active_was_removed = self
                        .scene_tabs
                        .get(self.active_scene_tab)
                        .map(|t| t.model.file_path.as_deref() == Some(path_str.as_str()))
                        .unwrap_or(false);
                    let active_path_before = self
                        .scene_tabs
                        .get(self.active_scene_tab)
                        .and_then(|t| t.model.file_path.clone());
                    let mut closed_any = false;
                    self.scene_tabs.retain(|t| {
                        let keep = t.model.file_path.as_deref() != Some(path_str.as_str());
                        if !keep {
                            closed_any = true;
                        }
                        keep
                    });
                    if closed_any {
                        if self.scene_tabs.is_empty() {
                            self.scene_tabs.push(
                                crate::app::scene_editor::scene_tabs::SceneTab::new(
                                    crate::app::scene_editor::model::SceneModel::new(),
                                    "Untitled".to_string(),
                                ),
                            );
                            self.active_scene_tab = 0;
                        } else if active_was_removed {
                            // Active tab vanished — retain wasn't told to
                            // preserve our index, so re-anchor to the
                            // first remaining tab.
                            self.active_scene_tab =
                                self.active_scene_tab.min(self.scene_tabs.len() - 1);
                        } else {
                            // A non-active tab was closed; preserve the
                            // active by re-locating it via its path.
                            if let Some(orig) = active_path_before.as_deref() {
                                if let Some(idx) = self
                                    .scene_tabs
                                    .iter()
                                    .position(|t| t.model.file_path.as_deref() == Some(orig))
                                {
                                    self.active_scene_tab = idx;
                                }
                            }
                        }
                        if active_was_removed {
                            // Only swap the in-memory scene when the
                            // *active* tab was the one removed. Otherwise
                            // unrelated edits on the active tab would be
                            // overwritten by a stale tab snapshot.
                            self.scene_model = self.scene_tabs[self.active_scene_tab].model.clone();
                            if self.current_scene_path.as_deref() == Some(path_str.as_str()) {
                                self.current_scene_path = None;
                            }
                            self.scene_needs_sync = true;
                            self.primary_selected_id = None;
                        }
                        // Also delete the orphan generated `.rs`
                        // plugin (e.g. `scenes/main.bscene` removed →
                        // delete `src/scenes/main.rs`). Without this
                        // step the codegen file lingers, mod.rs still
                        // references it, and `cargo check` errors on
                        // the missing module — surfacing as the
                        // "warnings won't go away" pain.
                        let removed_orphan = cleanup_orphan_scene_rs(&self.root_path, &path_str);
                        if removed_orphan {
                            // Refresh `mod.rs` to drop the removed
                            // module declaration.
                            let scenes_dir = format!("{}/src/scenes", self.root_path);
                            if std::path::Path::new(&scenes_dir).is_dir() {
                                let mod_rs =
                                    crate::app::scene_editor::codegen::generate_scenes_mod_rs(
                                        &scenes_dir,
                                    );
                                let _ = std::fs::write(format!("{}/mod.rs", scenes_dir), mod_rs);
                            }
                        }
                        self.status_message = format!("Scene removed: {}", path_str);
                        self.status_message_timestamp = Some(std::time::Instant::now());
                        tracing::info!("🗑 Scene tab closed (file removed): {}", path_str);
                    }
                }
                crate::app::asset_watcher::AssetEvent::ShaderChanged(p) => {
                    self.status_message = format!(
                        "Shader changed: {} (Bevy will auto-reload)",
                        p.file_name()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_default()
                    );
                    self.status_message_timestamp = Some(std::time::Instant::now());
                }
                crate::app::asset_watcher::AssetEvent::AudioChanged(p) => {
                    // If the active preview points at the changed
                    // file, re-decode the peak buffer so the
                    // waveform reflects the new content next frame.
                    // Phase F / v0.6.
                    let path_str = p.to_string_lossy().to_string();
                    if self
                        .audio_preview
                        .loaded_path
                        .as_ref()
                        .map(|q| q.to_string_lossy() == path_str)
                        .unwrap_or(false)
                    {
                        self.audio_preview.open(p.clone());
                    }
                    self.status_message = format!(
                        "Audio changed: {} (Bevy will auto-reload)",
                        p.file_name()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_default()
                    );
                    self.status_message_timestamp = Some(std::time::Instant::now());
                }
            }
        }
    }
}

/// Derive a human-readable tab label from a `.bscene` path. Falls
/// back to "scene" if the path has no useful stem (Windows roots,
/// stripped extensions, etc.). The label is also the rendered tab
/// text in the Hierarchy panel — the test suite locks this to the
/// `.bscene` stem so users always see the same name in the file
/// tree, the tab strip, and the generated `.rs` filename.
pub(crate) fn scene_label_from_path(path: &str) -> String {
    std::path::Path::new(path)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "scene".to_string())
}

/// Codegen dispatch for `save_current_scene` — pure function so we
/// can test the "modular vs legacy, never both" contract without
/// spinning up `BerryCodeApp`. Returns the path it wrote to.
///
/// Modular projects (`src/scenes/mod.rs` exists) write a single
/// plugin file at `src/scenes/<name>.rs` and aggressively delete any
/// leftover legacy alongside file at `scenes/<name>_scene.rs`.
/// Legacy projects keep the alongside form. Doing both was the bug
/// where Test7 ended up with `scenes/scene_scene.rs` AND
/// `src/scenes/scene.rs` for the same `.bscene`.
pub(crate) fn run_codegen_for_save(
    scene: &crate::app::scene_editor::model::SceneModel,
    bscene_path: &str,
    project_root: &str,
) -> Result<String, String> {
    let use_modular = crate::app::scene_editor::codegen::has_modular_structure(project_root);
    if use_modular {
        if let Some(legacy) = bscene_path
            .strip_suffix(".bscene")
            .map(|s| format!("{}_scene.rs", s))
        {
            if std::path::Path::new(&legacy).is_file() {
                let _ = std::fs::remove_file(&legacy);
            }
        }
        crate::app::scene_editor::codegen::save_scene_code_modular(scene, bscene_path, project_root)
    } else {
        crate::app::scene_editor::codegen::save_scene_code(scene, bscene_path)
    }
}

/// When a `.bscene` is deleted on disk, the generated scene plugin
/// `<root>/src/scenes/<name>.rs` becomes an orphan that still gets
/// registered by `mod.rs`. This helper removes both the new-style
/// `<name>.rs` and the legacy-style `<name>_scene.rs` if either
/// exists. Returns true if anything was deleted (the caller uses
/// that signal to refresh `mod.rs`).
pub(crate) fn cleanup_orphan_scene_rs(project_root: &str, bscene_path: &str) -> bool {
    let stem = match std::path::Path::new(bscene_path).file_stem() {
        Some(s) => s.to_string_lossy().into_owned(),
        None => return false,
    };
    let module = crate::app::scene_editor::codegen::scene_name_to_module(&stem);
    let scenes_dir = format!("{}/src/scenes", project_root);
    let mut deleted = false;
    for candidate in [
        format!("{}/{}.rs", scenes_dir, module),
        format!("{}/{}_scene.rs", scenes_dir, module),
    ] {
        let p = std::path::Path::new(&candidate);
        if p.exists() && p.is_file() {
            if std::fs::remove_file(p).is_ok() {
                deleted = true;
            }
        }
    }
    deleted
}

#[cfg(test)]
mod scene_naming_consistency_tests {
    //! Lock the contract that the file-tree filename, the Hierarchy
    //! tab label, and the generated `.rs` plugin filename all agree.
    //! Regression tests for the cluster of bugs the user hit when the
    //! tab label said `scene` but the codegen wrote `scene_scene.rs`,
    //! and again when deleting `.bscene` left an orphan `.rs`.

    use super::*;
    use crate::app::scene_editor::codegen::{save_scene_code_modular, scene_name_to_module};
    use crate::app::scene_editor::model::SceneModel;

    /// `.bscene` stem and `.rs` stem must match the same module name
    /// for every reasonable scene name a user might type.
    #[test]
    fn bscene_stem_matches_rs_stem_for_modular_save() {
        let cases = [
            "main",
            "test",
            "level_01",
            "Untitled 1",  // typed name with a space
            "MyCoolScene", // CamelCase
            "玩家",        // unicode
        ];
        for raw in &cases {
            let tmp = tempfile::tempdir().unwrap();
            let root = tmp.path().to_string_lossy().to_string();
            std::fs::create_dir_all(format!("{}/src/scenes", root)).unwrap();

            let bscene_rel = format!("scenes/{}.bscene", raw);
            let scene = SceneModel::new();
            let rs_path = save_scene_code_modular(&scene, &bscene_rel, &root)
                .unwrap_or_else(|e| panic!("save failed for {:?}: {}", raw, e));

            let bscene_stem = std::path::Path::new(&bscene_rel)
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .to_string();
            let rs_stem = std::path::Path::new(&rs_path)
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .to_string();
            // Both sides go through the same sanitiser so spaces /
            // unicode collapse identically. The contract is "the same
            // module name on both sides", not "byte-for-byte equal
            // strings".
            assert_eq!(
                scene_name_to_module(&bscene_stem),
                rs_stem,
                "case {:?}: rs stem '{}' must match module of bscene stem",
                raw,
                rs_stem
            );
        }
    }

    /// Specific guard for the "scene_scene.rs / test_scene.rs"
    /// regression: codegen used to append `_scene` to the module name
    /// independent of the user's input. Make sure that path is dead.
    #[test]
    fn codegen_does_not_append_scene_suffix() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_string_lossy().to_string();
        std::fs::create_dir_all(format!("{}/src/scenes", root)).unwrap();
        let rs_path =
            save_scene_code_modular(&SceneModel::new(), "scenes/test.bscene", &root).unwrap();
        assert!(
            rs_path.ends_with("/test.rs"),
            "expected /test.rs, got {}",
            rs_path
        );
        // mod.rs must reference `test` and `TestPlugin`, not the
        // legacy `test_scene` / `TestScenePlugin`.
        let mod_rs = std::fs::read_to_string(format!("{}/src/scenes/mod.rs", root)).unwrap();
        assert!(mod_rs.contains("pub mod test;"));
        assert!(mod_rs.contains("test::TestPlugin"));
        assert!(!mod_rs.contains("test_scene"));
        assert!(!mod_rs.contains("TestScenePlugin"));
    }

    /// Tab label and `.rs` file stem must agree on the same module.
    /// `scene_label_from_path` drives both the Hierarchy tab text
    /// (raw stem, e.g. "Untitled 1") and `save_current_scene` →
    /// codegen (sanitised stem, e.g. "untitled_1"); both paths must
    /// resolve to the same module name when sanitised.
    #[test]
    fn tab_label_and_rs_filename_resolve_to_same_module() {
        let cases = [
            ("scenes/main.bscene", "main"),
            ("scenes/Untitled 1.bscene", "Untitled 1"),
            ("scenes/Level 01.bscene", "Level 01"),
        ];
        for (path, expected_label) in &cases {
            let label = scene_label_from_path(path);
            assert_eq!(&label, expected_label, "tab label for {}", path);
            let label_module = scene_name_to_module(&label);

            let tmp = tempfile::tempdir().unwrap();
            let root = tmp.path().to_string_lossy().to_string();
            std::fs::create_dir_all(format!("{}/src/scenes", root)).unwrap();
            let rs_path = save_scene_code_modular(&SceneModel::new(), path, &root).unwrap();
            let rs_stem = std::path::Path::new(&rs_path)
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .to_string();
            assert_eq!(
                label_module, rs_stem,
                "{}: tab label module must match rs filename",
                path
            );
        }
    }

    /// Deleting a `.bscene` should also delete the matching `.rs`
    /// (both the new-style filename and any leftover legacy
    /// `<name>_scene.rs`). And `mod.rs` should drop the module.
    #[test]
    fn cleanup_orphan_scene_rs_removes_both_old_and_new() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_string_lossy().to_string();
        let scenes_dir = format!("{}/src/scenes", root);
        std::fs::create_dir_all(&scenes_dir).unwrap();
        // Pretend both the new and legacy plugins exist (a project
        // mid-migration).
        std::fs::write(format!("{}/main.rs", scenes_dir), "// new").unwrap();
        std::fs::write(format!("{}/main_scene.rs", scenes_dir), "// legacy").unwrap();
        std::fs::write(format!("{}/unrelated.rs", scenes_dir), "// keep me").unwrap();

        let removed = cleanup_orphan_scene_rs(&root, "/anywhere/scenes/main.bscene");
        assert!(removed, "should report deletion when files were present");
        assert!(
            !std::path::Path::new(&format!("{}/main.rs", scenes_dir)).exists(),
            "main.rs should be deleted"
        );
        assert!(
            !std::path::Path::new(&format!("{}/main_scene.rs", scenes_dir)).exists(),
            "main_scene.rs should be deleted"
        );
        assert!(
            std::path::Path::new(&format!("{}/unrelated.rs", scenes_dir)).exists(),
            "unrelated.rs must not be touched"
        );
    }

    /// When the `.bscene` has no matching `.rs` to delete, the helper
    /// must return false (so the caller knows not to rewrite mod.rs).
    #[test]
    fn cleanup_orphan_scene_rs_returns_false_when_nothing_to_delete() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_string_lossy().to_string();
        std::fs::create_dir_all(format!("{}/src/scenes", root)).unwrap();
        let removed = cleanup_orphan_scene_rs(&root, "/anywhere/scenes/never_existed.bscene");
        assert!(!removed);
    }

    /// Modular projects must produce exactly one scene plugin file —
    /// `src/scenes/<name>.rs` and nothing else. Legacy alongside copies
    /// (`scenes/<name>_scene.rs`) must not appear, even if a stale
    /// version was left behind from an earlier release.
    #[test]
    fn modular_save_writes_one_rs_no_legacy_alongside() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_string_lossy().to_string();
        // Modular structure marker.
        std::fs::create_dir_all(format!("{}/src/scenes", root)).unwrap();
        std::fs::write(format!("{}/src/scenes/mod.rs", root), "").unwrap();
        // The .bscene + a stale legacy alongside file.
        std::fs::create_dir_all(format!("{}/scenes", root)).unwrap();
        let bscene = format!("{}/scenes/main.bscene", root);
        std::fs::write(&bscene, "(entities: [])").unwrap();
        let stale_alongside = format!("{}/scenes/main_scene.rs", root);
        std::fs::write(&stale_alongside, "// stale").unwrap();

        let scene = crate::app::scene_editor::model::SceneModel::new();
        let result = run_codegen_for_save(&scene, &bscene, &root);
        assert!(result.is_ok(), "{:?}", result);
        let written = result.unwrap();
        assert!(
            written.ends_with("/src/scenes/main.rs"),
            "wrote to {}",
            written
        );
        // Old alongside copy must be gone.
        assert!(
            !std::path::Path::new(&stale_alongside).exists(),
            "stale {} should have been removed",
            stale_alongside
        );
        // And no fresh legacy file created either.
        assert!(
            !std::path::Path::new(&format!("{}/scenes/main_scene.rs", root)).exists(),
            "modular save must not write legacy alongside"
        );
        // Sanity: the modular file is there.
        assert!(std::path::Path::new(&written).exists());
    }

    /// Legacy (non-modular) projects keep the old
    /// `scenes/<name>_scene.rs` convention.
    #[test]
    fn legacy_save_writes_alongside_rs() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_string_lossy().to_string();
        std::fs::create_dir_all(format!("{}/scenes", root)).unwrap();
        let bscene = format!("{}/scenes/main.bscene", root);
        std::fs::write(&bscene, "(entities: [])").unwrap();
        // No `src/scenes/` ⇒ legacy mode.

        let scene = crate::app::scene_editor::model::SceneModel::new();
        let result = run_codegen_for_save(&scene, &bscene, &root);
        assert!(result.is_ok(), "{:?}", result);
        let written = result.unwrap();
        assert!(
            written.ends_with("scenes/main_scene.rs"),
            "legacy mode wrote to {}",
            written
        );
        assert!(std::path::Path::new(&written).exists());
    }

    #[test]
    fn scene_label_from_path_strips_extension_and_dirs() {
        assert_eq!(scene_label_from_path("/abs/scenes/main.bscene"), "main");
        assert_eq!(scene_label_from_path("scenes/test.bscene"), "test");
        assert_eq!(scene_label_from_path("Untitled 1.bscene"), "Untitled 1");
        // Empty / pathological → falls back to "scene".
        assert_eq!(scene_label_from_path(""), "scene");
        assert_eq!(scene_label_from_path("/"), "scene");
    }
}
