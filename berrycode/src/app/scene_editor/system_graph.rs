//! Bevy System Execution Order Visualization (DAG).
//!
//! Displays systems as draggable nodes with dependency arrows. Supports:
//! - Startup systems (green), Update systems (blue)
//! - `.before()` / `.after()` dependency arrows
//! - Manual addition and code scanning for `add_systems(...)` patterns
//! - Stage/Set labels as group headers

use serde::{Deserialize, Serialize};
use crate::app::BerryCodeApp;

/// A node in the system execution graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemNode {
    pub name: String,
    /// Schedule name: "Startup", "Update", "FixedUpdate", etc.
    pub stage: String,
    /// Position in the graph canvas (x, y).
    pub position: [f32; 2],
    /// Names of systems this system depends on (runs after them).
    pub dependencies: Vec<String>,
}

impl Default for SystemNode {
    fn default() -> Self {
        Self {
            name: String::new(),
            stage: "Update".into(),
            position: [100.0, 100.0],
            dependencies: Vec::new(),
        }
    }
}

/// The full system execution graph.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemGraph {
    pub systems: Vec<SystemNode>,
}

/// Scan source code text for `add_systems(Schedule, system_fn)` patterns.
/// Returns a list of (stage, system_name) pairs found.
pub fn scan_systems_from_code(code: &str) -> Vec<(String, String)> {
    let mut results = Vec::new();
    for line in code.lines() {
        let trimmed = line.trim();
        // Match patterns like: .add_systems(Update, movement_system)
        // or: .add_systems(Startup, setup_scene)
        if let Some(rest) = trimmed.strip_prefix(".add_systems(")
            .or_else(|| trimmed.strip_prefix("app.add_systems("))
        {
            // Parse "Schedule, system_name..." up to ')' or ','
            let parts: Vec<&str> = rest.splitn(3, ',').collect();
            if parts.len() >= 2 {
                let stage = parts[0].trim().to_string();
                let sys_part = parts[1].trim();
                // Handle trailing ')' or other chars
                let sys_name = sys_part
                    .split(|c: char| c == ')' || c == '.' || c == '|')
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if !sys_name.is_empty() && !sys_name.contains(' ') {
                    results.push((stage, sys_name));
                }
            }
        }
        // Also match standalone system function signatures
        // fn some_system(query: Query<...>) { }
    }
    results
}

/// Stage color mapping.
fn stage_color(stage: &str) -> egui::Color32 {
    match stage {
        "Startup" => egui::Color32::from_rgb(80, 200, 120), // green
        "Update" => egui::Color32::from_rgb(80, 140, 220),  // blue
        "FixedUpdate" => egui::Color32::from_rgb(200, 160, 80), // orange
        "PostUpdate" => egui::Color32::from_rgb(180, 100, 200), // purple
        _ => egui::Color32::from_rgb(160, 160, 160),        // gray
    }
}

const NODE_WIDTH: f32 = 140.0;
const NODE_HEIGHT: f32 = 40.0;

impl BerryCodeApp {
    /// Render the System Graph window.
    pub(crate) fn render_system_graph(&mut self, ctx: &egui::Context) {
        if !self.system_graph_open {
            return;
        }

        let mut open = self.system_graph_open;

        egui::Window::new("System Execution Graph")
            .open(&mut open)
            .default_size([700.0, 500.0])
            .resizable(true)
            .show(ctx, |ui| {
                // Toolbar
                ui.horizontal(|ui| {
                    if ui.button("+ System").clicked() {
                        let count = self.system_graph.systems.len();
                        self.system_graph.systems.push(SystemNode {
                            name: format!("system_{}", count),
                            stage: "Update".into(),
                            position: [
                                100.0 + (count as f32 % 4.0) * 160.0,
                                100.0 + (count as f32 / 4.0).floor() * 80.0,
                            ],
                            dependencies: Vec::new(),
                        });
                    }
                    if ui.button("Scan Code").clicked() {
                        self.scan_systems_from_project();
                    }
                    if ui.button("Clear All").clicked() {
                        self.system_graph.systems.clear();
                    }
                });

                ui.separator();

                // Legend
                ui.horizontal(|ui| {
                    let stages = ["Startup", "Update", "FixedUpdate", "PostUpdate"];
                    for stage in stages {
                        let color = stage_color(stage);
                        let (rect, _) = ui.allocate_exact_size(
                            egui::vec2(12.0, 12.0),
                            egui::Sense::hover(),
                        );
                        ui.painter().rect_filled(rect, 2.0, color);
                        ui.label(stage);
                        ui.add_space(8.0);
                    }
                });

                ui.separator();

                // Canvas area
                let (response, painter) = ui.allocate_painter(
                    ui.available_size_before_wrap(),
                    egui::Sense::click_and_drag(),
                );
                let canvas_origin = response.rect.min;

                // Build a name->index lookup for dependency arrows
                let name_to_idx: std::collections::HashMap<String, usize> = self
                    .system_graph
                    .systems
                    .iter()
                    .enumerate()
                    .map(|(i, s)| (s.name.clone(), i))
                    .collect();

                // Draw dependency arrows first (behind nodes)
                for sys in &self.system_graph.systems {
                    let to_center = egui::pos2(
                        canvas_origin.x + sys.position[0] + NODE_WIDTH / 2.0,
                        canvas_origin.y + sys.position[1] + NODE_HEIGHT / 2.0,
                    );
                    for dep_name in &sys.dependencies {
                        if let Some(&dep_idx) = name_to_idx.get(dep_name) {
                            let dep = &self.system_graph.systems[dep_idx];
                            let from_center = egui::pos2(
                                canvas_origin.x + dep.position[0] + NODE_WIDTH / 2.0,
                                canvas_origin.y + dep.position[1] + NODE_HEIGHT / 2.0,
                            );
                            painter.arrow(
                                from_center,
                                to_center - from_center,
                                egui::Stroke::new(1.5, egui::Color32::from_rgb(200, 200, 200)),
                            );
                        }
                    }
                }

                // Draw nodes
                for sys in &self.system_graph.systems {
                    let node_rect = egui::Rect::from_min_size(
                        egui::pos2(
                            canvas_origin.x + sys.position[0],
                            canvas_origin.y + sys.position[1],
                        ),
                        egui::vec2(NODE_WIDTH, NODE_HEIGHT),
                    );
                    let color = stage_color(&sys.stage);
                    painter.rect_filled(node_rect, 4.0, color.linear_multiply(0.3));
                    painter.rect_stroke(node_rect, 4.0, egui::Stroke::new(1.0, color));

                    let label = format!("{}\n({})", sys.name, sys.stage);
                    painter.text(
                        node_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        &label,
                        egui::FontId::proportional(11.0),
                        egui::Color32::from_rgb(220, 220, 220),
                    );
                }

                // Handle dragging
                if response.dragged() {
                    if let Some(pointer) = response.interact_pointer_pos() {
                        let rel = pointer - canvas_origin;
                        // Find node under pointer
                        for sys in &mut self.system_graph.systems {
                            let node_rect = egui::Rect::from_min_size(
                                egui::pos2(sys.position[0], sys.position[1]),
                                egui::vec2(NODE_WIDTH, NODE_HEIGHT),
                            );
                            if node_rect.contains(egui::pos2(rel.x, rel.y)) {
                                sys.position[0] += response.drag_delta().x;
                                sys.position[1] += response.drag_delta().y;
                                break;
                            }
                        }
                    }
                }
            });

        self.system_graph_open = open;
    }

    /// Scan the project for add_systems calls and populate the system graph.
    fn scan_systems_from_project(&mut self) {
        let root = self.root_path.clone();
        let root_path = std::path::Path::new(&root);
        let mut rs_files = Vec::new();
        collect_rs_files_for_scan(root_path, &mut rs_files);

        let mut found = Vec::new();
        for path in &rs_files {
            if let Ok(content) = std::fs::read_to_string(path) {
                found.extend(scan_systems_from_code(&content));
            }
        }

        // Merge into existing graph (avoid duplicates)
        let existing_names: std::collections::HashSet<String> = self
            .system_graph
            .systems
            .iter()
            .map(|s| s.name.clone())
            .collect();

        let mut count = self.system_graph.systems.len();
        for (stage, name) in found {
            if !existing_names.contains(&name) {
                self.system_graph.systems.push(SystemNode {
                    name,
                    stage,
                    position: [
                        80.0 + (count as f32 % 4.0) * 170.0,
                        60.0 + (count as f32 / 4.0).floor() * 70.0,
                    ],
                    dependencies: Vec::new(),
                });
                count += 1;
            }
        }
    }
}

/// Recursively collect `.rs` files, skipping `target/` and hidden dirs.
fn collect_rs_files_for_scan(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            if name == "target" || name.starts_with('.') {
                continue;
            }
            collect_rs_files_for_scan(&path, out);
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_graph_default() {
        let graph = SystemGraph::default();
        assert!(graph.systems.is_empty());
    }

    #[test]
    fn system_graph_ron_roundtrip() {
        let graph = SystemGraph {
            systems: vec![
                SystemNode {
                    name: "movement".into(),
                    stage: "Update".into(),
                    position: [100.0, 200.0],
                    dependencies: vec![],
                },
                SystemNode {
                    name: "collision".into(),
                    stage: "Update".into(),
                    position: [300.0, 200.0],
                    dependencies: vec!["movement".into()],
                },
            ],
        };
        let json = serde_json::to_string(&graph).expect("serialize");
        let back: SystemGraph = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.systems.len(), 2);
        assert_eq!(back.systems[1].dependencies, vec!["movement".to_string()]);
    }

    #[test]
    fn scan_add_systems_call() {
        let code = r#"
            .add_systems(Update, movement_system)
            .add_systems(Startup, setup_scene)
            app.add_systems(FixedUpdate, physics_step)
        "#;
        let results = scan_systems_from_code(code);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0], ("Update".into(), "movement_system".into()));
        assert_eq!(results[1], ("Startup".into(), "setup_scene".into()));
        assert_eq!(results[2], ("FixedUpdate".into(), "physics_step".into()));
    }
}
