//! Godot scene viewer panel (v0.8.x Migration & interop).
//!
//! Auto-opens when the active editor tab is a `.tscn` file and shows
//! the scene's node hierarchy plus selected-node properties. Read-only
//! by design — BerryCode is the bridge that lets users browse their
//! Godot project while migrating, not an editor that writes back.
//!
//! The parse result is cached against `(path, mtime)` so the tree
//! doesn't get re-parsed every frame. We invalidate when the file
//! changes on disk or the user switches tabs.

use std::path::PathBuf;
use std::time::SystemTime;

use crate::godot_import::{GodotScene, NodeId};

use super::BerryCodeApp;

/// Frame-by-frame state for the Godot scene panel. Lives on
/// `BerryCodeApp` so cached parses survive across egui repaints.
pub struct GodotScenePanelState {
    /// `(path, mtime, scene)` cache. Re-parsed when the path changes
    /// or the file's mtime advances on disk.
    cached: Option<(PathBuf, SystemTime, GodotScene)>,
    /// Index into `scene.nodes` of the user-selected node, if any.
    selected: Option<NodeId>,
    /// Last error string from a failed load attempt, surfaced inline
    /// so the user understands why the panel went dark instead of
    /// silently rendering an empty tree.
    last_error: Option<String>,
}

impl Default for GodotScenePanelState {
    fn default() -> Self {
        Self {
            cached: None,
            selected: None,
            last_error: None,
        }
    }
}

impl GodotScenePanelState {
    /// Drop the cache + selection. Called when the active tab changes
    /// away from the cached file so the next time we land on a `.tscn`
    /// we re-parse cleanly.
    fn clear(&mut self) {
        self.cached = None;
        self.selected = None;
        self.last_error = None;
    }
}

impl BerryCodeApp {
    /// Auto-show a side window with the scene tree whenever the active
    /// tab is a `.tscn` file. No-ops for any other tab.
    pub(crate) fn render_godot_scene_panel(&mut self, ctx: &egui::Context) {
        let Some(active_path) = self.active_tscn_path() else {
            // Active tab isn't a .tscn — release any cached parse so
            // we don't hold onto an unrelated file's data.
            if self.godot_scene_panel.cached.is_some() {
                self.godot_scene_panel.clear();
            }
            return;
        };

        // Refresh the cache if needed: path changed or mtime advanced.
        let needs_reload = match &self.godot_scene_panel.cached {
            Some((cached_path, cached_mtime, _)) => {
                if cached_path != &active_path {
                    true
                } else {
                    std::fs::metadata(&active_path)
                        .and_then(|m| m.modified())
                        .map(|m| m > *cached_mtime)
                        .unwrap_or(false)
                }
            }
            None => true,
        };

        if needs_reload {
            match GodotScene::load(&active_path) {
                Ok(scene) => {
                    let mtime = std::fs::metadata(&active_path)
                        .and_then(|m| m.modified())
                        .unwrap_or_else(|_| SystemTime::now());
                    self.godot_scene_panel.cached = Some((active_path.clone(), mtime, scene));
                    self.godot_scene_panel.selected = None;
                    self.godot_scene_panel.last_error = None;
                }
                Err(e) => {
                    self.godot_scene_panel.cached = None;
                    self.godot_scene_panel.last_error = Some(format!("{e}"));
                }
            }
        }

        let mut open = true;
        egui::Window::new("Godot Scene")
            .id(egui::Id::new("godot_scene_panel_v1"))
            .open(&mut open)
            .default_size([320.0, 480.0])
            .resizable(true)
            .show(ctx, |ui| {
                self.render_godot_scene_panel_body(ui);
            });
        // We deliberately ignore `open == false`: the panel is tied
        // to the active tab being a `.tscn`, so closing it via the
        // window X is meaningless. Switching tabs is the way to
        // dismiss it.
    }

    fn render_godot_scene_panel_body(&mut self, ui: &mut egui::Ui) {
        if let Some(err) = self.godot_scene_panel.last_error.clone() {
            ui.colored_label(
                egui::Color32::from_rgb(220, 80, 80),
                format!("Parse error: {err}"),
            );
            ui.label("BerryCode reads .tscn read-only — if your file is malformed, fix it in Godot first.");
            return;
        }

        let Some((path, _, scene)) = self.godot_scene_panel.cached.clone() else {
            ui.label("Loading scene…");
            return;
        };

        ui.horizontal(|ui| {
            ui.label("Scene:");
            let display = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("(unnamed)");
            ui.label(egui::RichText::new(display).strong());
            if let Some(ver) = scene.format_version {
                ui.weak(format!("· format={ver}"));
            }
            ui.weak(format!("· {} nodes", scene.nodes.len()));
        });
        ui.separator();

        // Two-pane layout: hierarchy on top, property panel below the
        // separator. We don't split horizontally because the panel
        // itself is narrow and the property list reads better as a
        // continuation of the tree.
        egui::ScrollArea::vertical()
            .id_salt("godot_scene_tree")
            .max_height(ui.available_height() * 0.6)
            .show(ui, |ui| {
                if let Some(root) = scene.root {
                    self.render_godot_node(ui, &scene, root);
                } else {
                    ui.weak("(no root node found in this scene)");
                }
            });

        ui.separator();
        ui.label(egui::RichText::new("Properties").strong());

        match self.godot_scene_panel.selected {
            Some(idx) if idx < scene.nodes.len() => {
                let node = &scene.nodes[idx];
                if let Some(ty) = &node.type_name {
                    ui.weak(format!("type: {ty}"));
                }
                if let Some(parent) = &node.parent_path {
                    ui.weak(format!("parent: {parent}"));
                }
                egui::ScrollArea::vertical()
                    .id_salt("godot_scene_props")
                    .show(ui, |ui| {
                        if node.properties.is_empty() {
                            ui.weak("(no properties on this node)");
                        }
                        for (k, v) in &node.properties {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(k).monospace().strong());
                                ui.label("=");
                                ui.label(egui::RichText::new(v).monospace());
                            });
                        }
                    });
            }
            _ => {
                ui.weak("Click a node above to see its properties.");
            }
        }
    }

    fn render_godot_node(&mut self, ui: &mut egui::Ui, scene: &GodotScene, id: NodeId) {
        let node = &scene.nodes[id];
        let label = match &node.type_name {
            Some(t) => format!("{} ({})", node.name, t),
            None => node.name.clone(),
        };
        let is_selected = self.godot_scene_panel.selected == Some(id);
        let header = egui::CollapsingHeader::new(label)
            .id_salt(("godot_scene_node", id))
            .default_open(true);
        let response = header.show(ui, |ui| {
            for &child in &node.children {
                self.render_godot_node(ui, scene, child);
            }
            // Click target: a transparent button below the children
            // doesn't conflict with the collapse triangle on the header.
            let select_label = if is_selected {
                "▶ selected"
            } else {
                "select"
            };
            if ui.small_button(select_label).clicked() {
                self.godot_scene_panel.selected = Some(id);
            }
        });
        // Clicking the header label itself also selects the node so
        // the user doesn't have to hunt for the small select button.
        if response.header_response.clicked() {
            self.godot_scene_panel.selected = Some(id);
        }
    }

    /// Returns the active editor tab's path iff it ends with `.tscn`.
    fn active_tscn_path(&self) -> Option<PathBuf> {
        let tab = self.editor_tabs.get(self.active_tab_idx)?;
        if !tab.file_path.ends_with(".tscn") {
            return None;
        }
        Some(PathBuf::from(&tab.file_path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_is_empty() {
        let s = GodotScenePanelState::default();
        assert!(s.cached.is_none());
        assert!(s.selected.is_none());
        assert!(s.last_error.is_none());
    }

    #[test]
    fn clear_resets_all_fields() {
        let mut s = GodotScenePanelState::default();
        s.last_error = Some("boom".to_string());
        s.clear();
        assert!(s.last_error.is_none());
    }
}
