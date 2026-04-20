//! Bevy Query Visualizer: show which entities match which ECS queries.
//!
//! Parses user code for `Query<(...)>` patterns and compares query requirements
//! against entities in the scene model to determine matches.

use super::model::*;
use crate::app::BerryCodeApp;
use serde::{Deserialize, Serialize};

/// A parsed Bevy query definition.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QueryDef {
    /// Display name (typically the function name containing the query).
    pub name: String,
    /// Required component type names (e.g. ["Transform", "MeshCube"]).
    pub components: Vec<String>,
    /// `With<T>` filters.
    pub with_filters: Vec<String>,
    /// `Without<T>` filters.
    pub without_filters: Vec<String>,
}

/// Scan source code text for `Query<(...)>` patterns.
/// Returns query definitions with extracted component types.
pub fn scan_queries_from_code(code: &str) -> Vec<QueryDef> {
    let mut results = Vec::new();
    let lines: Vec<&str> = code.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Look for function signatures containing Query<
        if !trimmed.contains("Query<") {
            continue;
        }

        // Extract function name from the surrounding `fn name(...)` if present
        let fn_name = find_enclosing_fn_name(&lines, i);

        // Extract the Query type parameter
        if let Some(query_content) = extract_query_content(trimmed) {
            let (components, with_f, without_f) = parse_query_params(&query_content);
            if !components.is_empty() || !with_f.is_empty() {
                results.push(QueryDef {
                    name: fn_name.unwrap_or_else(|| format!("query_{}", results.len())),
                    components,
                    with_filters: with_f,
                    without_filters: without_f,
                });
            }
        }
    }

    results
}

/// Find the nearest `fn` name above or on the given line.
fn find_enclosing_fn_name(lines: &[&str], line_idx: usize) -> Option<String> {
    for i in (0..=line_idx).rev() {
        let trimmed = lines[i].trim();
        if let Some(rest) = trimmed
            .strip_prefix("fn ")
            .or_else(|| trimmed.strip_prefix("pub fn "))
            .or_else(|| trimmed.strip_prefix("pub(crate) fn "))
        {
            let name = rest
                .split(|c: char| c == '(' || c == '<' || c.is_whitespace())
                .next()
                .unwrap_or("")
                .to_string();
            if !name.is_empty() {
                return Some(name);
            }
        }
        // Do not look past more than 5 lines back
        if line_idx - i > 5 {
            break;
        }
    }
    None
}

/// Extract the content inside `Query<...>`, handling basic nesting.
fn extract_query_content(line: &str) -> Option<String> {
    let start = line.find("Query<")?;
    let after = &line[start + 6..];
    let mut depth = 1i32;
    let mut end_idx = 0;
    for (i, ch) in after.char_indices() {
        match ch {
            '<' => depth += 1,
            '>' => {
                depth -= 1;
                if depth == 0 {
                    end_idx = i;
                    break;
                }
            }
            _ => {}
        }
    }
    if depth == 0 {
        Some(after[..end_idx].to_string())
    } else {
        None
    }
}

/// Parse query params like `(&Transform, &MeshCube), With<Player>, Without<Enemy>`
fn parse_query_params(content: &str) -> (Vec<String>, Vec<String>, Vec<String>) {
    let mut components = Vec::new();
    let mut with_filters = Vec::new();
    let mut without_filters = Vec::new();

    // Split by top-level commas (not nested in <>)
    let parts = split_top_level(content, ',');

    for part in &parts {
        let trimmed = part.trim();

        if let Some(inner) = trimmed
            .strip_prefix("With<")
            .and_then(|s| s.strip_suffix('>'))
        {
            with_filters.push(inner.trim().to_string());
        } else if let Some(inner) = trimmed
            .strip_prefix("Without<")
            .and_then(|s| s.strip_suffix('>'))
        {
            without_filters.push(inner.trim().to_string());
        } else if trimmed.starts_with('(') || trimmed.ends_with(')') {
            // Tuple of component references: (&Transform, &Velocity)
            let inner = trimmed.trim_start_matches('(').trim_end_matches(')');
            for comp in inner.split(',') {
                let c = comp
                    .trim()
                    .trim_start_matches('&')
                    .trim_start_matches("mut ");
                let c = c.trim();
                if !c.is_empty() {
                    components.push(c.to_string());
                }
            }
        } else {
            // Single component reference: &Transform or Transform
            let c = trimmed.trim_start_matches('&').trim_start_matches("mut ");
            let c = c.trim();
            if !c.is_empty() && c != "Entity" {
                components.push(c.to_string());
            }
        }
    }

    (components, with_filters, without_filters)
}

/// Split a string by a delimiter, respecting `<>` nesting depth.
fn split_top_level(s: &str, delim: char) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut depth = 0i32;

    for ch in s.chars() {
        if ch == '<' || ch == '(' {
            depth += 1;
            current.push(ch);
        } else if ch == '>' || ch == ')' {
            depth -= 1;
            current.push(ch);
        } else if ch == delim && depth == 0 {
            parts.push(current.clone());
            current.clear();
        } else {
            current.push(ch);
        }
    }
    if !current.is_empty() {
        parts.push(current);
    }
    parts
}

/// Check if a scene entity matches a query definition.
/// An entity matches if it has ALL required component types and none of the
/// Without filter types, based on component labels.
pub fn entity_matches_query(entity: &SceneEntity, query: &QueryDef) -> bool {
    let entity_types: std::collections::HashSet<&str> =
        entity.components.iter().map(|c| c.label()).collect();

    // Check all required components are present
    for required in &query.components {
        if !entity_types.contains(required.as_str()) {
            return false;
        }
    }

    // Check With filters
    for with in &query.with_filters {
        if !entity_types.contains(with.as_str()) {
            return false;
        }
    }

    // Check Without filters (entity must NOT have these)
    for without in &query.without_filters {
        if entity_types.contains(without.as_str()) {
            return false;
        }
    }

    true
}

impl BerryCodeApp {
    /// Render the Query Visualizer window.
    pub(crate) fn render_query_viz(&mut self, ctx: &egui::Context) {
        if !self.query_viz_open {
            return;
        }

        let mut open = self.query_viz_open;

        egui::Window::new("Query Visualizer")
            .open(&mut open)
            .default_size([500.0, 400.0])
            .resizable(true)
            .show(ctx, |ui| {
                // Toolbar
                ui.horizontal(|ui| {
                    if ui.button("Scan Code").clicked() {
                        self.scan_queries_from_project();
                    }
                    if ui.button("+ Manual Query").clicked() {
                        self.queries.push(QueryDef {
                            name: format!("query_{}", self.queries.len()),
                            components: vec!["Transform".into()],
                            with_filters: Vec::new(),
                            without_filters: Vec::new(),
                        });
                    }
                    if ui.button("Clear").clicked() {
                        self.queries.clear();
                    }
                    ui.separator();
                    ui.label(format!("{} queries", self.queries.len()));
                });

                ui.separator();

                // List queries and their matching entities
                let entities_snapshot: Vec<(u64, String, Vec<String>)> = self
                    .scene_model
                    .entities
                    .values()
                    .map(|e| {
                        (
                            e.id,
                            e.name.clone(),
                            e.components.iter().map(|c| c.label().to_string()).collect(),
                        )
                    })
                    .collect();

                egui::ScrollArea::vertical()
                    .id_salt("query_list")
                    .show(ui, |ui| {
                        let mut remove_idx: Option<usize> = None;

                        for (qi, query) in self.queries.iter().enumerate() {
                            let id = ui.make_persistent_id(format!("query_{}", qi));
                            egui::collapsing_header::CollapsingState::load_with_default_open(
                                ui.ctx(),
                                id,
                                true,
                            )
                            .show_header(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(&query.name)
                                        .strong()
                                        .color(egui::Color32::from_rgb(120, 200, 255)),
                                );
                                let comp_str = query.components.join(", ");
                                ui.label(format!("Query<({})>", comp_str));
                                if ui.small_button("X").clicked() {
                                    remove_idx = Some(qi);
                                }
                            })
                            .body(|ui| {
                                // Show filters
                                if !query.with_filters.is_empty() {
                                    ui.label(format!("  With: {}", query.with_filters.join(", ")));
                                }
                                if !query.without_filters.is_empty() {
                                    ui.label(format!(
                                        "  Without: {}",
                                        query.without_filters.join(", ")
                                    ));
                                }

                                // Show matching entities
                                ui.label(
                                    egui::RichText::new("Matching entities:")
                                        .color(egui::Color32::from_rgb(180, 180, 180)),
                                );
                                let mut match_count = 0;
                                for (eid, ename, ecomps) in &entities_snapshot {
                                    // Check if entity matches
                                    let has_all = query
                                        .components
                                        .iter()
                                        .all(|c| ecomps.contains(c))
                                        && query.with_filters.iter().all(|c| ecomps.contains(c))
                                        && query
                                            .without_filters
                                            .iter()
                                            .all(|c| !ecomps.contains(c));

                                    if has_all {
                                        match_count += 1;
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new(format!("  [{}]", eid))
                                                    .monospace()
                                                    .color(egui::Color32::from_rgb(120, 255, 120)),
                                            );
                                            ui.label(ename);
                                        });
                                    }
                                }
                                if match_count == 0 {
                                    ui.label(
                                        egui::RichText::new("  (no matches)")
                                            .italics()
                                            .color(egui::Color32::from_rgb(160, 160, 160)),
                                    );
                                }
                            });
                        }

                        if let Some(idx) = remove_idx {
                            self.queries.remove(idx);
                        }
                    });
            });

        self.query_viz_open = open;
    }

    /// Scan project source files for Query<...> patterns.
    fn scan_queries_from_project(&mut self) {
        let root = self.root_path.clone();
        let root_path = std::path::Path::new(&root);
        let mut rs_files = Vec::new();
        collect_rs_for_query_scan(root_path, &mut rs_files);

        let mut all_queries = Vec::new();
        for path in &rs_files {
            if let Ok(content) = std::fs::read_to_string(path) {
                all_queries.extend(scan_queries_from_code(&content));
            }
        }

        // Deduplicate by name
        let existing: std::collections::HashSet<String> =
            self.queries.iter().map(|q| q.name.clone()).collect();
        for q in all_queries {
            if !existing.contains(&q.name) {
                self.queries.push(q);
            }
        }
    }
}

fn collect_rs_for_query_scan(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
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
            collect_rs_for_query_scan(&path, out);
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_simple_query() {
        let code = r#"
fn movement_system(query: Query<(&Transform, &Velocity)>) {
}
"#;
        let queries = scan_queries_from_code(code);
        assert_eq!(queries.len(), 1);
        assert_eq!(queries[0].name, "movement_system");
        assert!(queries[0].components.contains(&"Transform".to_string()));
        assert!(queries[0].components.contains(&"Velocity".to_string()));
    }

    #[test]
    fn scan_query_with_filters() {
        let code = r#"
fn player_system(query: Query<&Transform, With<Player>, Without<Enemy>>) {
}
"#;
        let queries = scan_queries_from_code(code);
        assert_eq!(queries.len(), 1);
        assert!(queries[0].components.contains(&"Transform".to_string()));
        assert!(queries[0].with_filters.contains(&"Player".to_string()));
        assert!(queries[0].without_filters.contains(&"Enemy".to_string()));
    }

    #[test]
    fn entity_matches_basic_query() {
        let entity = SceneEntity::new(
            1,
            "Player".into(),
            vec![ComponentData::MeshCube {
                size: 1.0,
                color: [1.0, 1.0, 1.0],
                metallic: 0.0,
                roughness: 0.5,
                emissive: [0.0, 0.0, 0.0],
                texture_path: None,
                normal_map_path: None,
            }],
        );

        // ComponentData::MeshCube has label() == "Cube"
        let query_match = QueryDef {
            name: "test".into(),
            components: vec!["Cube".into()],
            with_filters: vec![],
            without_filters: vec![],
        };
        assert!(entity_matches_query(&entity, &query_match));

        let query_no_match = QueryDef {
            name: "test2".into(),
            components: vec!["Light".into()],
            with_filters: vec![],
            without_filters: vec![],
        };
        assert!(!entity_matches_query(&entity, &query_no_match));
    }
}
