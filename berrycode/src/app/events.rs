//! Event polling for file watcher and LSP responses

use super::BerryCodeApp;
use super::types::LspResponse;
use crate::native;

impl BerryCodeApp {
    pub(crate) fn poll_file_watcher_events(&mut self) {
        // Deferred scene re-import path: collected inside the watcher loop,
        // processed after the borrow on self.file_watcher is released.
        let mut pending_scene_reimport: Option<(String, String)> = None;

        if let Some(watcher) = &mut self.file_watcher {
            while let Some(event) = watcher.try_recv() {
                match event {
                    native::watcher::FileEvent::Created(path) => {
                        tracing::debug!("📄 File created: {}", path.display());
                        self.file_tree_load_pending = true;
                    }
                    native::watcher::FileEvent::Modified(path) => {
                        tracing::debug!("File modified: {}", path.display());

                        // Bevy Asset Hot Reload: detect changes to asset files
                        // (.png, .jpg, .glb, .gltf, .wav, .ogg) in the project's
                        // assets/ directory and trigger a scene re-sync.
                        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                            let is_asset_ext = matches!(
                                ext,
                                "png" | "jpg" | "jpeg" | "glb" | "gltf" | "wav" | "ogg" | "mp3"
                            );
                            let in_assets_dir = path
                                .to_string_lossy()
                                .contains("/assets/");
                            if is_asset_ext && in_assets_dir {
                                self.scene_needs_sync = true;
                                let filename = path
                                    .file_name()
                                    .map(|s| s.to_string_lossy().to_string())
                                    .unwrap_or_default();
                                self.status_message = format!("Asset reloaded: {}", filename);
                                self.status_message_timestamp = Some(std::time::Instant::now());
                                tracing::info!("Asset hot reload: {}", filename);
                            }
                        }

                        // Phase 75: hot reload - notify on .rs file changes
                        if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                            self.hot_reload.notify_change();
                            // Rescan user component definitions for bidirectional sync
                            self.scanned_user_components =
                                crate::app::scene_editor::script_scan::scan_components_with_fields(
                                    &self.root_path,
                                );

                            // If a _scene.rs file was modified, defer re-import
                            let path_str = path.to_string_lossy().to_string();
                            if path_str.ends_with("_scene.rs") {
                                let bscene_path = path_str.replace("_scene.rs", ".bscene");
                                let should_reimport = self
                                    .scene_model
                                    .file_path
                                    .as_ref()
                                    .map(|p| *p == bscene_path)
                                    .unwrap_or(false);
                                if should_reimport {
                                    pending_scene_reimport = Some((path_str, bscene_path));
                                }
                            }
                        }
                    }
                    native::watcher::FileEvent::Removed(path) => {
                        tracing::debug!("🗑️  File removed: {}", path.display());
                        self.file_tree_load_pending = true;

                        let path_str = path.to_string_lossy().to_string();
                        if let Some(tab_idx) = self.editor_tabs.iter().position(|tab| tab.file_path == path_str) {
                            self.editor_tabs.remove(tab_idx);
                            if self.active_tab_idx >= self.editor_tabs.len() && !self.editor_tabs.is_empty() {
                                self.active_tab_idx = self.editor_tabs.len() - 1;
                            }
                            tracing::info!("🗑️  Closed tab for deleted file: {}", path_str);
                        }
                    }
                    native::watcher::FileEvent::Renamed { from, to } => {
                        tracing::debug!("📝 File renamed: {} -> {}", from.display(), to.display());
                        self.file_tree_load_pending = true;

                        let from_str = from.to_string_lossy().to_string();
                        let to_str = to.to_string_lossy().to_string();
                        if let Some(tab) = self.editor_tabs.iter_mut().find(|tab| tab.file_path == from_str) {
                            tab.file_path = to_str.clone();
                            tracing::info!("📝 Updated tab path: {} -> {}", from_str, to_str);
                        }
                    }
                }
            }
        }

        // Process deferred scene re-import (outside the watcher borrow)
        if let Some((scene_rs_path, bscene_path)) = pending_scene_reimport {
            if let Ok(code) = std::fs::read_to_string(&scene_rs_path) {
                let imported =
                    crate::app::scene_editor::code_import::import_scene_from_code(&code);
                if !imported.entities.is_empty() {
                    self.scene_snapshot();
                    self.scene_model = imported;
                    self.scene_model.file_path = Some(bscene_path);
                    self.scene_needs_sync = true;
                    self.status_message =
                        format!("Scene re-imported from {}", scene_rs_path);
                    self.status_message_timestamp = Some(std::time::Instant::now());
                }
            }
        }
    }

    pub(crate) fn poll_lsp_responses(&mut self) {
        // Deferred actions to perform after releasing rx borrow
        enum DeferredAction {
            NavigateToLocation(super::types::LspLocation),
            ShowPicker(Vec<super::types::LspLocation>),
        }

        let mut deferred_actions: Vec<DeferredAction> = Vec::new();

        if let Some(rx) = &mut self.lsp_response_rx {
            while let Ok(response) = rx.try_recv() {
                match response {
                    LspResponse::Connected => {
                        tracing::info!("🟢 LSP connection established");
                        self.lsp_connected = true;
                        self.status_message = "✅ LSP connected".to_string();
                        self.status_message_timestamp = Some(std::time::Instant::now());
                    }
                    LspResponse::Diagnostics(diagnostics) => {
                        tracing::info!("📋 Received {} diagnostics", diagnostics.len());
                        self.lsp_diagnostics = diagnostics;
                    }
                    LspResponse::Hover(hover_info) => {
                        tracing::info!("💡 Received hover info");
                        let has_hover = hover_info.is_some();
                        self.lsp_hover_info = hover_info;
                        self.lsp_show_hover = has_hover;
                    }
                    LspResponse::Completions(completions) => {
                        tracing::info!("💡 Received {} completions", completions.len());
                        self.lsp_completions = completions;
                        self.lsp_show_completions = !self.lsp_completions.is_empty();
                    }
                    LspResponse::Definition(locations) => {
                        tracing::info!("🔍 Received {} definition locations", locations.len());

                        if locations.is_empty() {
                            self.pending_goto_definition.take();
                            self.status_message = "❌ Definition not found (LSP)".to_string();
                            self.status_message_timestamp = Some(std::time::Instant::now());
                        } else if locations.len() == 1 {
                            deferred_actions.push(DeferredAction::NavigateToLocation(locations[0].clone()));
                            self.pending_goto_definition = None;
                        } else {
                            tracing::info!("📋 Multiple definitions found, showing picker");
                            deferred_actions.push(DeferredAction::ShowPicker(locations));
                            self.pending_goto_definition = None;
                        }
                    }
                    LspResponse::References(locations) => {
                        tracing::info!("🔍 Received {} references", locations.len());

                        if locations.is_empty() {
                            self.status_message = "No references found".to_string();
                            self.status_message_timestamp = Some(std::time::Instant::now());
                        } else {
                            self.lsp_references = locations;
                            self.show_references_panel = true;
                            self.status_message = format!("Found {} references", self.lsp_references.len());
                            self.status_message_timestamp = Some(std::time::Instant::now());
                        }
                    }
                }
            }
        }

        // Process deferred actions after releasing the borrow
        for action in deferred_actions {
            match action {
                DeferredAction::NavigateToLocation(location) => {
                    self.navigate_to_location(&location);
                }
                DeferredAction::ShowPicker(locations) => {
                    self.definition_picker_locations = locations;
                    self.show_definition_picker = true;
                }
            }
        }

        // Poll diagnostics from LSP server notifications (publishDiagnostics)
        self.poll_lsp_diagnostics();
    }

    /// Poll the diagnostics channel for publishDiagnostics notifications
    /// from the LSP server and convert them into our LspDiagnostic format.
    pub(crate) fn poll_lsp_diagnostics(&mut self) {
        if let Some(rx) = &mut self.lsp_diagnostics_rx {
            while let Ok(published) = rx.try_recv() {
                tracing::info!(
                    "Received {} diagnostics for {}",
                    published.diagnostics.len(),
                    published.uri
                );

                // Remove old diagnostics for this URI, then add new ones
                // (URI is a file:// URL; we match by checking if the diagnostic source URI matches)
                // For simplicity we replace the entire diagnostics list per URI.
                // First, extract file path from URI for display purposes.
                let file_path = if let Ok(url) = lsp_types::Url::parse(&published.uri) {
                    url.to_file_path()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|_| published.uri.clone())
                } else {
                    published.uri.clone()
                };

                // Remove existing diagnostics for this file
                self.lsp_diagnostics.retain(|d| {
                    d.source.as_deref() != Some(&file_path)
                });

                // Convert lsp_types::Diagnostic to our LspDiagnostic
                for diag in &published.diagnostics {
                    let severity = match diag.severity {
                        Some(lsp_types::DiagnosticSeverity::ERROR) => {
                            super::types::DiagnosticSeverity::Error
                        }
                        Some(lsp_types::DiagnosticSeverity::WARNING) => {
                            super::types::DiagnosticSeverity::Warning
                        }
                        Some(lsp_types::DiagnosticSeverity::INFORMATION) => {
                            super::types::DiagnosticSeverity::Information
                        }
                        Some(lsp_types::DiagnosticSeverity::HINT) => {
                            super::types::DiagnosticSeverity::Hint
                        }
                        _ => super::types::DiagnosticSeverity::Warning,
                    };

                    self.lsp_diagnostics.push(super::types::LspDiagnostic {
                        line: diag.range.start.line as usize,
                        column: diag.range.start.character as usize,
                        message: diag.message.clone(),
                        severity,
                        source: Some(file_path.clone()),
                    });
                }
            }
        }
    }
}
