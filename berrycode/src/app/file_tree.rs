//! File tree rendering and operations

use super::BerryCodeApp;
use super::types::FileTreeEvent;
use super::ui_colors;
use super::file_icon_colors;
use crate::native;
use crate::native::fs::DirEntry;

/// Returns true if the given file name is a 3D asset that can be dragged from
/// the file tree onto the Scene View to spawn an entity (Phase H).
fn is_droppable_asset(filename: &str) -> bool {
    let lower = filename.to_lowercase();
    matches!(
        lower.rsplit('.').next().unwrap_or(""),
        "glb" | "gltf" | "obj" | "stl" | "ply" | "bprefab"
    )
}

impl BerryCodeApp {
    /// Render File Tree panel (Phase 2: full implementation)
    pub(crate) fn render_file_tree(&mut self, ui: &mut egui::Ui) {
        // Project name dropdown
        let project_name = self.root_path
            .split('/')
            .last()
            .unwrap_or("oracleberry");

        ui.horizontal(|ui| {
            // Folder icon
            ui.label(
                egui::RichText::new("\u{ea83}") // codicon-folder
                    .size(16.0)
                    .color(ui_colors::TEXT_DEFAULT)
                    .family(egui::FontFamily::Name("codicon".into()))
            );

            ui.add_space(4.0);

            // Project name with dropdown
            let response = ui.button(
                egui::RichText::new(format!("{} ▼", project_name.to_uppercase()))
                    .size(11.0)
                    .strong()
            );

            // TODO: Show dropdown menu when clicked
            if response.clicked() {
                // Future: Show project switcher menu
            }
        });

        ui.separator();

        // New File / New Folder buttons
        ui.horizontal(|ui| {
            if ui.button(
                egui::RichText::new("\u{ea7f}") // codicon: new-file
                    .family(egui::FontFamily::Name("codicon".into()))
            ).on_hover_text("New File").clicked() {
                self.new_file_dialog_open = true;
            }
            if ui.button(
                egui::RichText::new("\u{ea83}") // codicon: new-folder
                    .family(egui::FontFamily::Name("codicon".into()))
            ).on_hover_text("New Folder").clicked() {
                self.new_folder_dialog_open = true;
            }
        });

        ui.add_space(4.0);

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
            // Set font style ONCE for the whole tree (not per node)
            ui.style_mut().text_styles.insert(
                egui::TextStyle::Body,
                egui::FontId::proportional(14.0),
            );
            ui.style_mut().text_styles.insert(
                egui::TextStyle::Button,
                egui::FontId::proportional(15.0),
            );

            // Load file tree on first render
            if self.file_tree_cache.is_empty() && self.file_tree_load_pending {
                ui.label("読み込み中...");

                match native::fs::read_dir(&self.root_path, Some(1)) {
                    Ok(entries) => {
                        tracing::info!("✅ Loaded {} entries from {}", entries.len(), self.root_path);
                        self.file_tree_cache = entries;
                        self.file_tree_load_pending = false;
                        // Auto-expand root folder on first load
                        self.expanded_dirs.insert(self.root_path.clone());
                    }
                    Err(e) => {
                        ui.colored_label(egui::Color32::RED, format!("エラー: {}", e));
                        self.file_tree_load_pending = false;
                    }
                }
            }

            // Root folder row
            let root_name = self.root_path.split('/').last().unwrap_or(&self.root_path);

            let is_root_expanded = self.expanded_dirs.contains(&self.root_path);
            let root_icon = if is_root_expanded { "\u{ea7c}" } else { "\u{ea83}" };

            let response = ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(root_icon)
                        .family(egui::FontFamily::Name("codicon".into()))
                );
                ui.add(egui::Label::new(root_name).sense(egui::Sense::click()))
            }).inner;

            if response.clicked() {
                if is_root_expanded {
                    self.expanded_dirs.remove(&self.root_path);
                } else {
                    self.expanded_dirs.insert(self.root_path.clone());
                }
            }

            // Render children without cloning: take cache, render read-only,
            // restore cache, then apply any event.
            if is_root_expanded {
                let cache = std::mem::take(&mut self.file_tree_cache);
                let selected = self.editor_tabs.get(self.active_tab_idx).map(|t| t.file_path.as_str());
                let mut event: Option<FileTreeEvent> = None;
                for entry in &cache {
                    if event.is_none() {
                        event = Self::render_tree_node(ui, entry, 1, &self.expanded_dirs, selected);
                    } else {
                        Self::render_tree_node(ui, entry, 1, &self.expanded_dirs, selected);
                    }
                }
                self.file_tree_cache = cache;

                // Apply the single event (if any) after rendering
                match event {
                    Some(FileTreeEvent::ExpandDir(path, needs_load)) => {
                        tracing::info!("📂 Expanded: {}", path);
                        self.expanded_dirs.insert(path.clone());
                        if needs_load {
                            self.load_directory_children(&path);
                        }
                    }
                    Some(FileTreeEvent::CollapseDir(path)) => {
                        tracing::info!("📁 Collapsed: {}", path);
                        self.expanded_dirs.remove(&path);
                    }
                    Some(FileTreeEvent::OpenFile(path)) => {
                        self.open_file_from_path(&path);
                    }
                    Some(FileTreeEvent::ContextMenu(path, is_dir)) => {
                        self.context_menu_path = Some(path);
                        self.context_menu_is_dir = is_dir;
                        self.context_menu_pos = ui.ctx().input(|i| {
                            i.pointer.hover_pos().unwrap_or(egui::Pos2::ZERO)
                        });
                    }
                    Some(FileTreeEvent::StartAssetDrag(path)) => {
                        self.dragged_asset_path = Some(path);
                        tracing::info!("Drag started for asset: {:?}", self.dragged_asset_path);
                    }
                    None => {}
                }
            }
        });
    }

    /// Render a single tree node (file or directory) recursively.
    /// Read-only: does not mutate self. Returns at most one FileTreeEvent per frame.
    pub(crate) fn render_tree_node(
        ui: &mut egui::Ui,
        node: &DirEntry,
        depth: usize,
        expanded_dirs: &std::collections::HashSet<String>,
        selected_file: Option<&str>,
    ) -> Option<FileTreeEvent> {
        let indent = depth as f32 * 20.0;
        let mut event: Option<FileTreeEvent> = None;

        if node.is_dir {
            let is_expanded = expanded_dirs.contains(&node.path);
            let icon = if is_expanded { "\u{ea7c}" } else { "\u{ea83}" };

            ui.add_space(1.0);
            let row_response = ui.horizontal(|ui| {
                ui.add_space(indent);
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.label(
                    egui::RichText::new(icon)
                        .family(egui::FontFamily::Name("codicon".into()))
                );
                ui.add(egui::Label::new(
                    egui::RichText::new(&node.name).strong()
                ).sense(egui::Sense::click()))
            });

            let label_response = row_response.inner;
            let full_response = row_response.response.interact(egui::Sense::hover());

            if label_response.clicked() {
                event = Some(if is_expanded {
                    FileTreeEvent::CollapseDir(node.path.clone())
                } else {
                    FileTreeEvent::ExpandDir(node.path.clone(), node.children.is_none())
                });
            }

            if label_response.secondary_clicked() {
                event = Some(FileTreeEvent::ContextMenu(node.path.clone(), true));
            }

            // Hover cursor
            if full_response.hovered() || label_response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Default);
            }

            if is_expanded {
                if let Some(children) = &node.children {
                    for child in children {
                        if event.is_none() {
                            event = Self::render_tree_node(ui, child, depth + 1, expanded_dirs, selected_file);
                        } else {
                            Self::render_tree_node(ui, child, depth + 1, expanded_dirs, selected_file);
                        }
                    }
                }
            }
        } else {
            let (icon, color) = Self::get_file_icon_with_color(&node.name);
            let is_selected = selected_file == Some(node.path.as_str());
            let droppable = is_droppable_asset(&node.name);

            ui.add_space(1.0);

            // Draw selection background before the row
            let row_response = ui.horizontal(|ui| {
                ui.add_space(indent);
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.label(
                    egui::RichText::new(icon)
                        .color(color)
                        .family(egui::FontFamily::Name("codicon".into()))
                );
                let text = if is_selected {
                    egui::RichText::new(&node.name).color(egui::Color32::WHITE)
                } else {
                    egui::RichText::new(&node.name)
                };
                // Droppable assets need click_and_drag so we can detect a drag
                // start and forward the path to the Scene View.
                let sense = if droppable {
                    egui::Sense::click_and_drag()
                } else {
                    egui::Sense::click()
                };
                ui.add(egui::Label::new(text).sense(sense))
            });

            let label_response = row_response.inner;

            // Selection/hover highlight - use label rect (not full row)
            let highlight_rect = label_response.rect.expand2(egui::vec2(4.0, 1.0));

            if is_selected {
                ui.painter().rect_filled(
                    highlight_rect,
                    3.0,
                    egui::Color32::from_rgba_premultiplied(60, 100, 180, 80), // subtle blue selection
                );
            } else if label_response.hovered() {
                ui.painter().rect_filled(
                    highlight_rect,
                    3.0,
                    egui::Color32::from_rgba_premultiplied(255, 255, 255, 12),
                );
            }

            if label_response.clicked() {
                event = Some(FileTreeEvent::OpenFile(node.path.clone()));
            }

            if label_response.secondary_clicked() {
                event = Some(FileTreeEvent::ContextMenu(node.path.clone(), false));
            }

            // Detect drag-start on droppable assets (3D models, etc).
            if droppable && label_response.drag_started() {
                event = Some(FileTreeEvent::StartAssetDrag(node.path.clone()));
            }
        }

        event
    }

    /// Check if a file extension indicates an image file
    fn is_image_extension(ext: &str) -> bool {
        matches!(
            ext,
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "ico" | "svg"
        )
    }

    /// Open a file from a given path (used by file tree and search results)
    pub(crate) fn open_file_from_path(&mut self, file_path: &str) {
        tracing::info!("📄 Opening file: {}", file_path);

        // Prefab files (.bprefab) are instantiated into the current scene at
        // the origin and the Scene Editor is brought to focus (Phase I).
        if file_path.ends_with(".bprefab") {
            match crate::app::scene_editor::prefab::load_prefab(file_path) {
                Ok(prefab) => {
                    self.scene_snapshot();
                    let new_root = crate::app::scene_editor::prefab::instantiate_prefab_from_path(
                        &mut self.scene_model,
                        &prefab,
                        file_path,
                    );
                    self.scene_model.select_only(new_root);
                    self.primary_selected_id = Some(new_root);
                    self.scene_needs_sync = true;
                    self.active_panel = crate::app::types::ActivePanel::SceneEditor;
                    self.status_message =
                        format!("Instantiated prefab at origin: {}", file_path);
                    self.status_message_timestamp = Some(std::time::Instant::now());
                }
                Err(e) => {
                    self.status_message = format!("Failed to load prefab: {}", e);
                    self.status_message_timestamp = Some(std::time::Instant::now());
                }
            }
            return;
        }

        // Scene files (.bscene) are not opened as text tabs — they replace the
        // current `SceneModel` and switch to the Scene Editor panel.
        if file_path.ends_with(".bscene") {
            use crate::app::scene_editor::scene_tabs::SceneTab;

            // Save current model back to its tab if tabs exist.
            if !self.scene_tabs.is_empty() {
                self.scene_tabs[self.active_scene_tab].model = self.scene_model.clone();
            }

            // Check if this file is already open in a tab.
            if let Some(idx) = self.scene_tabs.iter().position(|t| {
                t.model.file_path.as_deref() == Some(file_path)
            }) {
                self.active_scene_tab = idx;
                self.scene_model = self.scene_tabs[idx].model.clone();
                self.scene_needs_sync = true;
                self.active_panel = crate::app::types::ActivePanel::SceneEditor;
            } else {
                self.load_scene(file_path);
                let label = std::path::Path::new(file_path)
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "Scene".to_string());
                let tab = SceneTab::new(self.scene_model.clone(), label);
                self.scene_tabs.push(tab);
                self.active_scene_tab = self.scene_tabs.len() - 1;
            }
            return;
        }

        // Check if file is already open
        if let Some(idx) = self.editor_tabs.iter().position(|tab| tab.file_path == file_path) {
            self.active_tab_idx = idx;
            tracing::info!("✅ Switched to existing tab: {}", file_path);
            return;
        }

        // Check if it's an image file
        let ext = file_path.rsplit('.').next().unwrap_or("").to_lowercase();
        if Self::is_image_extension(&ext) {
            let mut tab = super::types::EditorTab::new(file_path.to_string(), String::new());
            tab.is_image = true;
            tab.is_readonly = true;
            self.editor_tabs.push(tab);
            self.active_tab_idx = self.editor_tabs.len() - 1;
            self.selected_file = Some((file_path.to_string(), String::new()));
            tracing::info!("🖼 Opened image file: {}", file_path);
            return;
        }

        // Check if it's a 3D model file
        if matches!(ext.as_str(), "gltf" | "glb" | "obj" | "stl" | "ply") {
            let mut tab = super::types::EditorTab::new(file_path.to_string(), String::new());
            tab.is_model = true;
            tab.is_readonly = true;
            self.editor_tabs.push(tab);
            self.active_tab_idx = self.editor_tabs.len() - 1;
            self.selected_file = Some((file_path.to_string(), String::new()));
            tracing::info!("Opened 3D model file: {}", file_path);
            return;
        }

        match native::fs::read_file(file_path) {
            Ok(content) => {
                // Create new editor tab
                let tab = super::types::EditorTab::new(file_path.to_string(), content.clone());
                self.editor_tabs.push(tab);
                self.active_tab_idx = self.editor_tabs.len() - 1;

                // Load git line changes for gutter markers
                if let Ok(changes) = crate::native::git::get_line_changes(&self.root_path, file_path) {
                    if let Some(tab) = self.editor_tabs.last_mut() {
                        tab.git_line_changes = changes;
                        tab.git_changes_loaded = true;
                    }
                }

                tracing::info!("File loaded in new tab: {} ({} bytes)", file_path, content.len());

                // Notify LSP about opened file
                if let Some(lang) = crate::native::lsp_native::detect_server_language(file_path) {
                    if let Some(client) = &self.lsp_native_client {
                        let client = client.clone();
                        let path = file_path.to_string();
                        let content_for_lsp = content.clone();
                        let language = lang.to_string();
                        self.lsp_runtime.spawn(async move {
                            let _ = client.open_file(&language, &path, &content_for_lsp).await;
                        });
                    }
                }

                self.selected_file = Some((file_path.to_string(), content));
            }
            Err(e) => {
                tracing::error!("❌ Failed to read file {}: {}", file_path, e);
            }
        }
    }

    /// Load children for a specific directory
    pub(crate) fn load_directory_children(&mut self, dir_path: &str) {
        match native::fs::read_dir(dir_path, Some(1)) {
            Ok(children) => {
                tracing::info!("✅ Loaded {} children for {}", children.len(), dir_path);

                // Update the cache by finding the directory and updating its children
                Self::update_dir_entry_children(&mut self.file_tree_cache, dir_path, children);
            }
            Err(e) => {
                tracing::error!("❌ Failed to load directory {}: {}", dir_path, e);
            }
        }
    }

    /// Recursively update a directory entry's children in the cache
    pub(crate) fn update_dir_entry_children(entries: &mut Vec<DirEntry>, target_path: &str, new_children: Vec<DirEntry>) {
        for entry in entries.iter_mut() {
            if entry.path == target_path {
                entry.children = Some(new_children);
                return;
            }

            if let Some(children) = &mut entry.children {
                Self::update_dir_entry_children(children, target_path, new_children.clone());
            }
        }
    }

    /// Get file icon based on file extension (static version for use in closures)
    #[allow(dead_code)]
    pub(crate) fn get_file_icon_static(filename: &str) -> &'static str {
        // Codicon icons (using Unicode code points)
        if filename.ends_with(".rs") {
            "\u{eb8b}" // codicon-file-code (Rust)
        } else if filename.ends_with(".toml") {
            "\u{ea7e}" // codicon-settings-gear (Config)
        } else if filename.ends_with(".md") {
            "\u{ea82}" // codicon-markdown (Markdown)
        } else if filename.ends_with(".json") {
            "\u{ead1}" // codicon-json (JSON)
        } else if filename.ends_with(".yaml") || filename.ends_with(".yml") {
            "\u{ea7e}" // codicon-settings-gear (YAML)
        } else if filename.ends_with(".js") {
            "\u{ea7a}" // codicon-symbol-method (JavaScript)
        } else if filename.ends_with(".ts") {
            "\u{ea7a}" // codicon-symbol-method (TypeScript)
        } else if filename.ends_with(".html") {
            "\u{eb7e}" // codicon-code (HTML)
        } else if filename.ends_with(".css") {
            "\u{eb7e}" // codicon-code (CSS)
        } else if filename.ends_with(".py") {
            "\u{eb8b}" // codicon-file-code (Python)
        } else if filename.ends_with(".sh") {
            "\u{ea85}" // codicon-terminal (Shell script)
        } else if filename.ends_with(".txt") {
            "\u{ea7b}" // codicon-file (Text)
        } else if filename.ends_with(".lock") {
            "\u{ea7f}" // codicon-lock (Lock file)
        } else if filename.ends_with(".proto") {
            "\u{eb8b}" // codicon-file-code (Protocol buffers)
        } else if filename.ends_with(".xml") {
            "\u{eb7e}" // codicon-code (XML)
        } else if filename.ends_with(".svg") {
            "\u{eaf0}" // codicon-file-media (SVG)
        } else if filename.ends_with(".png") || filename.ends_with(".jpg") || filename.ends_with(".jpeg") {
            "\u{eaf0}" // codicon-file-media (Images)
        } else if filename.ends_with(".gitignore") || filename.ends_with(".gitattributes") {
            "\u{ea84}" // codicon-git-branch (Git)
        } else if filename == "Cargo.toml" || filename == "Cargo.lock" {
            "\u{ea7e}" // codicon-settings-gear (Cargo)
        } else if filename == "package.json" {
            "\u{ead1}" // codicon-json (npm)
        } else if filename == "README.md" {
            "\u{ea82}" // codicon-markdown (README)
        } else {
            "\u{ea7b}" // codicon-file (Default)
        }
    }

    /// Render the "New File" dialog window
    pub(crate) fn render_new_file_dialog(&mut self, ctx: &egui::Context) {
        if self.new_file_dialog_open {
            let mut open = true;
            egui::Window::new("New File")
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("File name:");
                        ui.text_edit_singleline(&mut self.new_file_name);
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Create").clicked() && !self.new_file_name.is_empty() {
                            let path = format!("{}/{}", self.root_path, self.new_file_name);
                            match crate::native::fs::create_file(&path) {
                                Ok(_) => {
                                    self.file_tree_load_pending = true;
                                    self.file_tree_cache.clear();
                                    self.open_file_from_path(&path);
                                    self.status_message = format!("Created: {}", self.new_file_name);
                                    self.status_message_timestamp = Some(std::time::Instant::now());
                                }
                                Err(e) => {
                                    self.status_message = format!("Error: {}", e);
                                    self.status_message_timestamp = Some(std::time::Instant::now());
                                }
                            }
                            self.new_file_name.clear();
                            self.new_file_dialog_open = false;
                        }
                        if ui.button("Cancel").clicked() {
                            self.new_file_name.clear();
                            self.new_file_dialog_open = false;
                        }
                    });
                });
            if !open {
                self.new_file_dialog_open = false;
                self.new_file_name.clear();
            }
        }
    }

    /// Render the "New Folder" dialog window
    pub(crate) fn render_new_folder_dialog(&mut self, ctx: &egui::Context) {
        if self.new_folder_dialog_open {
            let mut open = true;
            egui::Window::new("New Folder")
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Folder name:");
                        ui.text_edit_singleline(&mut self.new_folder_name);
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Create").clicked() && !self.new_folder_name.is_empty() {
                            let path = format!("{}/{}", self.root_path, self.new_folder_name);
                            match std::fs::create_dir_all(&path) {
                                Ok(_) => {
                                    self.file_tree_load_pending = true;
                                    self.file_tree_cache.clear();
                                    self.status_message = format!("Created folder: {}", self.new_folder_name);
                                    self.status_message_timestamp = Some(std::time::Instant::now());
                                }
                                Err(e) => {
                                    self.status_message = format!("Error: {}", e);
                                    self.status_message_timestamp = Some(std::time::Instant::now());
                                }
                            }
                            self.new_folder_name.clear();
                            self.new_folder_dialog_open = false;
                        }
                        if ui.button("Cancel").clicked() {
                            self.new_folder_name.clear();
                            self.new_folder_dialog_open = false;
                        }
                    });
                });
            if !open {
                self.new_folder_dialog_open = false;
                self.new_folder_name.clear();
            }
        }
    }

    /// Render the right-click context menu for file tree items
    pub(crate) fn render_file_context_menu(&mut self, ctx: &egui::Context) {
        if let Some(path) = self.context_menu_path.clone() {
            let menu_id = egui::Id::new("file_tree_context_menu");
            let mut close_menu = false;

            egui::Area::new(menu_id)
                .order(egui::Order::Foreground)
                .fixed_pos(self.context_menu_pos)
                .show(ctx, |ui| {
                    egui::Frame::popup(ui.style())
                        .show(ui, |ui| {
                            ui.set_min_width(160.0);

                            if self.context_menu_is_dir {
                                if ui.button("New File Here...").clicked() {
                                    // Set parent dir for new file creation
                                    self.new_file_name = String::new();
                                    self.new_file_dialog_open = true;
                                    close_menu = true;
                                }
                                if ui.button("New Folder Here...").clicked() {
                                    self.new_folder_name = String::new();
                                    self.new_folder_dialog_open = true;
                                    close_menu = true;
                                }
                                ui.separator();
                            }

                            if ui.button("Rename...").clicked() {
                                let name = path.rsplit('/').next().unwrap_or(&path).to_string();
                                self.rename_file_old_path = path.clone();
                                self.rename_file_new_name = name;
                                self.rename_file_dialog_open = true;
                                close_menu = true;
                            }

                            if ui.button("Delete").clicked() {
                                let is_dir = std::path::Path::new(&path).is_dir();
                                let result = if is_dir {
                                    std::fs::remove_dir_all(&path)
                                } else {
                                    std::fs::remove_file(&path)
                                };
                                match result {
                                    Ok(_) => {
                                        self.status_message = format!(
                                            "Deleted: {}",
                                            path.rsplit('/').next().unwrap_or(&path)
                                        );
                                        self.status_message_timestamp =
                                            Some(std::time::Instant::now());
                                        self.file_tree_cache.clear();
                                        self.file_tree_load_pending = true;
                                        // Close tab if the deleted file was open
                                        if let Some(idx) =
                                            self.editor_tabs.iter().position(|t| t.file_path == path)
                                        {
                                            self.editor_tabs.remove(idx);
                                            if self.active_tab_idx >= self.editor_tabs.len()
                                                && !self.editor_tabs.is_empty()
                                            {
                                                self.active_tab_idx = self.editor_tabs.len() - 1;
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        self.status_message = format!("Delete failed: {}", e);
                                        self.status_message_timestamp =
                                            Some(std::time::Instant::now());
                                    }
                                }
                                close_menu = true;
                            }

                            ui.separator();

                            if ui.button("Copy Path").clicked() {
                                ui.ctx().copy_text(path.clone());
                                self.status_message = "Path copied".to_string();
                                self.status_message_timestamp = Some(std::time::Instant::now());
                                close_menu = true;
                            }

                            #[cfg(target_os = "macos")]
                            if ui.button("Reveal in Finder").clicked() {
                                let _ = std::process::Command::new("open")
                                    .arg("-R")
                                    .arg(&path)
                                    .spawn();
                                close_menu = true;
                            }
                        });
                });

            // Close menu if an item was clicked or user clicks elsewhere
            if close_menu {
                self.context_menu_path = None;
            } else {
                // Close on click outside the menu
                let pointer_pressed = ctx.input(|i| i.pointer.any_pressed());
                if pointer_pressed {
                    // Check if the click is outside the menu area
                    let pointer_pos = ctx.input(|i| i.pointer.hover_pos());
                    if let Some(pos) = pointer_pos {
                        // Use the area response to check if click is inside
                        let layer_id = egui::LayerId::new(egui::Order::Foreground, menu_id);
                        if !ctx.layer_id_at(pos).is_some_and(|id| id == layer_id) {
                            self.context_menu_path = None;
                        }
                    }
                }
            }
        }
    }

    /// Render the rename file/folder dialog
    pub(crate) fn render_rename_file_dialog(&mut self, ctx: &egui::Context) {
        if !self.rename_file_dialog_open {
            return;
        }

        let mut should_rename = false;
        let mut should_close = false;

        egui::Window::new("Rename")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("New name:");
                    let response = ui.text_edit_singleline(&mut self.rename_file_new_name);
                    if response.lost_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter))
                    {
                        should_rename = true;
                    }
                    response.request_focus();
                });
                ui.horizontal(|ui| {
                    if ui.button("Rename").clicked() && !self.rename_file_new_name.is_empty() {
                        should_rename = true;
                    }
                    if ui.button("Cancel").clicked() {
                        should_close = true;
                    }
                });
            });

        if should_rename && !self.rename_file_new_name.is_empty() {
            let old = self.rename_file_old_path.clone();
            let parent = std::path::Path::new(&old)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            let new_path = format!("{}/{}", parent, self.rename_file_new_name);
            match std::fs::rename(&old, &new_path) {
                Ok(_) => {
                    self.status_message =
                        format!("Renamed to {}", self.rename_file_new_name);
                    self.status_message_timestamp = Some(std::time::Instant::now());
                    self.file_tree_cache.clear();
                    self.file_tree_load_pending = true;
                    // Update open tab path if the renamed file was open
                    if let Some(tab) =
                        self.editor_tabs.iter_mut().find(|t| t.file_path == old)
                    {
                        tab.file_path = new_path;
                    }
                }
                Err(e) => {
                    self.status_message = format!("Rename failed: {}", e);
                    self.status_message_timestamp = Some(std::time::Instant::now());
                }
            }
            self.rename_file_dialog_open = false;
        } else if should_close {
            self.rename_file_dialog_open = false;
        }
    }

    /// Get file icon with color based on file extension
    pub(crate) fn get_file_icon_with_color(filename: &str) -> (&'static str, egui::Color32) {
        // Use named color constants from file_icon_colors module
        if filename.ends_with(".rs") {
            ("\u{eb8b}", file_icon_colors::RUST_ORANGE)
        } else if filename.ends_with(".toml") {
            ("\u{ea7e}", file_icon_colors::CONFIG_GRAY)
        } else if filename.ends_with(".md") {
            ("\u{ea82}", file_icon_colors::MARKDOWN_BLUE)
        } else if filename.ends_with(".json") {
            ("\u{ead1}", file_icon_colors::JSON_YELLOW)
        } else if filename.ends_with(".yaml") || filename.ends_with(".yml") {
            ("\u{ea7e}", file_icon_colors::CONFIG_GRAY)
        } else if filename.ends_with(".js") {
            ("\u{ea7a}", file_icon_colors::JAVASCRIPT_YELLOW)
        } else if filename.ends_with(".ts") {
            ("\u{ea7a}", file_icon_colors::TYPESCRIPT_BLUE)
        } else if filename.ends_with(".html") {
            ("\u{eb7e}", file_icon_colors::HTML_ORANGE)
        } else if filename.ends_with(".css") {
            ("\u{eb7e}", file_icon_colors::CSS_BLUE)
        } else if filename.ends_with(".py") {
            ("\u{eb8b}", file_icon_colors::PYTHON_GREEN)
        } else if filename.ends_with(".sh") {
            ("\u{ea85}", file_icon_colors::SHELL_GREEN)
        } else if filename.ends_with(".txt") {
            ("\u{ea7b}", ui_colors::TEXT_DEFAULT)
        } else if filename.ends_with(".lock") {
            ("\u{ea7f}", file_icon_colors::CONFIG_GRAY)
        } else if filename.ends_with(".proto") {
            ("\u{eb8b}", file_icon_colors::PROTO_PURPLE)
        } else if filename.ends_with(".xml") {
            ("\u{eb7e}", file_icon_colors::HTML_ORANGE)
        } else if filename.ends_with(".svg") {
            ("\u{eaf0}", file_icon_colors::SVG_AMBER)
        } else if filename.ends_with(".png") || filename.ends_with(".jpg") || filename.ends_with(".jpeg") {
            ("\u{eaf0}", file_icon_colors::IMAGE_PURPLE)
        } else if filename.ends_with(".gitignore") || filename.ends_with(".gitattributes") {
            ("\u{ea84}", file_icon_colors::GIT_ORANGE)
        } else if filename == "Cargo.toml" || filename == "Cargo.lock" {
            ("\u{ea7e}", file_icon_colors::RUST_ORANGE)
        } else if filename == "package.json" {
            ("\u{ead1}", file_icon_colors::JSON_YELLOW)
        } else if filename == "README.md" {
            ("\u{ea82}", file_icon_colors::MARKDOWN_BLUE)
        } else {
            ("\u{ea7b}", ui_colors::TEXT_DEFAULT)
        }
    }
}
