//! Bevy System Execution Order Visualization (DAG).
//!
//! Displays systems as draggable nodes with dependency arrows. Supports:
//! - Startup systems (green), Update systems (blue)
//! - `.before()` / `.after()` dependency arrows
//! - Manual addition and code scanning for `add_systems(...)` patterns
//! - Stage/Set labels as group headers

use crate::app::BerryCodeApp;
use serde::{Deserialize, Serialize};

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
    /// Manual ordering index within the system's `stage`. Lower runs
    /// first. Mirrored into `.before()` chains when codegen exports
    /// the schedule. v0.5 / drag-to-reorder.
    #[serde(default)]
    pub order: u32,
}

impl Default for SystemNode {
    fn default() -> Self {
        Self {
            name: String::new(),
            stage: "Update".into(),
            position: [100.0, 100.0],
            dependencies: Vec::new(),
            order: 0,
        }
    }
}

/// The full system execution graph.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemGraph {
    pub systems: Vec<SystemNode>,
}

/// Which view of the system graph the window currently shows. Persisted
/// implicitly through `BerryCodeApp::system_graph_view`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemGraphView {
    /// Free-form DAG canvas with `.before()` / `.after()` arrows.
    Dag,
    /// Per-stage ordered list with drag-to-reorder rows. The list
    /// translates the explicit `order` field on each node into
    /// vertical position, so this view is the simplest answer to
    /// "which system runs first in Update?". v0.5.
    List,
}

impl Default for SystemGraphView {
    fn default() -> Self {
        Self::Dag
    }
}

/// Scan source code text for `add_systems(Schedule, system_fn)` patterns.
/// Returns a list of (stage, system_name) pairs found.
///
/// Handles, depth-aware across the whole source (not line-by-line):
/// - `.add_systems(Update, movement_system)`
/// - `app.add_systems(Startup, setup_scene)`
/// - `add_systems(Update, (a, b, c))`            tuple form, multi-line OK
/// - `add_systems(OnEnter(AppScene::Test), ...)` state-bound schedules
/// - `add_systems(Update, sys.before(other))`    method chains stripped
///
/// The previous implementation was line-oriented and split on `,` without
/// tracking parens, so `(` ended up as a system name and `OnEnter(AppScene::Test)`
/// confused the schedule extraction. The current parser walks the source
/// once, finds each `add_systems(` call, brackets-balances to its closing
/// `)`, and decomposes the argument list at top-level commas only.
pub fn scan_systems_from_code(code: &str) -> Vec<(String, String)> {
    let mut results = Vec::new();
    let chars: Vec<char> = code.chars().collect();
    let needle: Vec<char> = "add_systems(".chars().collect();
    let mut i = 0;
    while i + needle.len() <= chars.len() {
        if chars[i..i + needle.len()] != needle[..] {
            i += 1;
            continue;
        }
        // The match must be preceded by `.` or whitespace / start of file
        // so we don't pick up identifiers like `my_add_systems(`.
        let preceded_ok = i == 0
            || matches!(
                chars[i - 1],
                '.' | ' ' | '\t' | '\n' | '\r' | '(' | ',' | ';' | '}' | '{'
            );
        if !preceded_ok {
            i += 1;
            continue;
        }
        let args_start = i + needle.len();
        let Some(args_end) = find_matching_close_paren(&chars, args_start) else {
            i = args_start;
            continue;
        };
        let args: String = chars[args_start..args_end].iter().collect();
        let parts = split_top_level_commas(&args);
        if parts.len() >= 2 {
            let stage = parts[0].trim().to_string();
            let systems = extract_systems(parts[1].trim());
            for sys in systems {
                results.push((stage.clone(), sys));
            }
        }
        i = args_end + 1;
    }
    results
}

/// Walk forward from `from` (the char *inside* an opening paren) and
/// return the index of the matching `)`. Tracks paren / bracket / brace
/// depth and skips characters inside double-quoted strings.
fn find_matching_close_paren(chars: &[char], from: usize) -> Option<usize> {
    let mut paren = 1i32;
    let mut bracket = 0i32;
    let mut brace = 0i32;
    let mut in_string = false;
    let mut prev = ' ';
    let mut i = from;
    while i < chars.len() {
        let c = chars[i];
        if in_string {
            if c == '"' && prev != '\\' {
                in_string = false;
            }
        } else {
            match c {
                '"' => in_string = true,
                '(' => paren += 1,
                ')' => {
                    paren -= 1;
                    if paren == 0 && bracket == 0 && brace == 0 {
                        return Some(i);
                    }
                }
                '[' => bracket += 1,
                ']' => bracket -= 1,
                '{' => brace += 1,
                '}' => brace -= 1,
                _ => {}
            }
        }
        prev = c;
        i += 1;
    }
    None
}

/// Split `s` at commas that sit at paren/bracket/brace depth 0 and
/// outside of double-quoted strings. Generic angle brackets (`<>`) are
/// intentionally *not* tracked — Rust uses `<` for both generics and
/// comparisons, and the second arg of `add_systems` rarely contains
/// either, so tracking them would create more false positives than it
/// would solve.
fn split_top_level_commas(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut paren = 0i32;
    let mut bracket = 0i32;
    let mut brace = 0i32;
    let mut in_string = false;
    let mut prev = ' ';
    for c in s.chars() {
        if in_string {
            current.push(c);
            if c == '"' && prev != '\\' {
                in_string = false;
            }
        } else {
            match c {
                '"' => {
                    in_string = true;
                    current.push(c);
                }
                '(' => {
                    paren += 1;
                    current.push(c);
                }
                ')' => {
                    paren -= 1;
                    current.push(c);
                }
                '[' => {
                    bracket += 1;
                    current.push(c);
                }
                ']' => {
                    bracket -= 1;
                    current.push(c);
                }
                '{' => {
                    brace += 1;
                    current.push(c);
                }
                '}' => {
                    brace -= 1;
                    current.push(c);
                }
                ',' if paren == 0 && bracket == 0 && brace == 0 => {
                    parts.push(current.clone());
                    current.clear();
                }
                _ => current.push(c),
            }
        }
        prev = c;
    }
    if !current.trim().is_empty() {
        parts.push(current);
    }
    parts
}

/// Decompose the second argument of `add_systems` into a list of system
/// function names. Handles the tuple form `(a, b, c.before(d))` by
/// recursing into the tuple body, and a single-system form by stripping
/// any trailing method chain (`.before(...)` / `.after(...)` etc.).
fn extract_systems(arg: &str) -> Vec<String> {
    let trimmed = arg.trim();
    // Tuple form: take whatever is inside the *first* matching pair of
    // parens, then split that at top-level commas. Anything *after* the
    // closing paren (e.g. `.chain()`) is metadata about the tuple, not
    // a system, so we discard it.
    if trimmed.starts_with('(') {
        let inner_chars: Vec<char> = trimmed.chars().skip(1).collect();
        if let Some(end) = find_matching_close_paren(&inner_chars, 0) {
            let inner: String = inner_chars[..end].iter().collect();
            return split_top_level_commas(&inner)
                .iter()
                .map(|item| extract_system_name(item.trim()))
                .filter(|s| is_valid_system_name(s))
                .collect();
        }
    }
    let name = extract_system_name(trimmed);
    if is_valid_system_name(&name) {
        vec![name]
    } else {
        vec![]
    }
}

/// Extract the leading identifier (allowing `::` path separators) from
/// `s`, stopping at whitespace, `.` (method chain), or any non-ident
/// punctuation. Leading whitespace is skipped so callers can pass
/// untrimmed slices.
fn extract_system_name(s: &str) -> String {
    let mut name = String::new();
    let mut started = false;
    for c in s.chars() {
        if !started && c.is_whitespace() {
            continue;
        }
        if c.is_alphanumeric() || c == '_' || c == ':' {
            name.push(c);
            started = true;
        } else {
            break;
        }
    }
    name
}

/// Whether `name` looks like a valid Rust path identifier — starts with
/// a letter or underscore, and contains only word chars / `:` after.
/// Used both to filter `extract_systems` output and to garbage-collect
/// stale graph nodes that the previous (broken) parser produced
/// (e.g. `(` or `(setup_world`).
fn is_valid_system_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == ':')
}

/// Stage color mapping. Uses prefix matching so `OnEnter(AppScene::Test)`
/// and `OnExit(AppScene::Test)` resolve to the OnEnter / OnExit colors
/// — state-bound schedules carry the scene name as part of the stage,
/// and we want all of them to share a colour family.
fn stage_color(stage: &str) -> egui::Color32 {
    let s = stage.trim();
    if s.starts_with("Startup") {
        egui::Color32::from_rgb(80, 200, 120) // green
    } else if s.starts_with("OnEnter") {
        egui::Color32::from_rgb(120, 220, 160) // mint — scene init
    } else if s.starts_with("OnExit") {
        egui::Color32::from_rgb(220, 130, 110) // coral — scene teardown
    } else if s.starts_with("FixedUpdate") {
        egui::Color32::from_rgb(200, 160, 80) // orange
    } else if s.starts_with("PostUpdate") {
        egui::Color32::from_rgb(180, 100, 200) // purple
    } else if s.starts_with("Update") {
        egui::Color32::from_rgb(80, 140, 220) // blue
    } else {
        egui::Color32::from_rgb(160, 160, 160) // gray (unknown)
    }
}

/// Minimum node width. Real width is the max of this and the text
/// extent — without the floor short labels would render as cramped
/// boxes, with it long labels like
/// `cleanup_scene2 (OnExit(AppScene::Scene2))` still expand to fit.
const NODE_MIN_WIDTH: f32 = 140.0;
const NODE_HEIGHT: f32 = 40.0;
const NODE_HORIZONTAL_PADDING: f32 = 12.0;
const NODE_LABEL_FONT_SIZE: f32 = 11.0;

/// Measure the rendered width of a node label so the box can grow to
/// fit. Multi-line labels (`name\n(stage)`) take the widest line.
/// Uses `Painter::layout_no_wrap` rather than `ui.fonts(|f| ...)`
/// because the latter takes `&Fonts` (immutable) but `layout_no_wrap`
/// needs `&mut self` access in this egui version.
fn measure_node_width(ui: &egui::Ui, name: &str, stage: &str) -> f32 {
    let font_id = egui::FontId::proportional(NODE_LABEL_FONT_SIZE);
    let painter = ui.painter();
    let mut max_line_width = 0.0_f32;
    for line in [name.to_string(), format!("({})", stage)] {
        let galley = painter.layout_no_wrap(line, font_id.clone(), egui::Color32::WHITE);
        max_line_width = max_line_width.max(galley.size().x);
    }
    (max_line_width + NODE_HORIZONTAL_PADDING * 2.0).max(NODE_MIN_WIDTH)
}

impl BerryCodeApp {
    /// Render the System Graph window.
    pub(crate) fn render_system_graph(&mut self, ctx: &egui::Context) {
        if !self.system_graph_open {
            return;
        }

        let mut open = self.system_graph_open;

        egui::Window::new("System Execution Graph")
            .open(&mut open)
            // Default wider than before — state-bound stage names like
            // `OnEnter(AppScene::Scene2)` are long, and 700px clipped
            // the rightmost column on first open. Users can still
            // shrink with the resize handle.
            .default_size([1100.0, 600.0])
            .resizable(true)
            .show(ctx, |ui| {
                // View toggle (v0.5: DAG canvas vs ordered list)
                ui.horizontal(|ui| {
                    let dag = self.system_graph_view == SystemGraphView::Dag;
                    let list = self.system_graph_view == SystemGraphView::List;
                    if ui.selectable_label(dag, "Graph").clicked() {
                        self.system_graph_view = SystemGraphView::Dag;
                    }
                    if ui.selectable_label(list, "List").clicked() {
                        self.system_graph_view = SystemGraphView::List;
                    }
                });
                ui.separator();

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
                            order: count as u32,
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
                    let stages = [
                        "Startup",
                        "OnEnter",
                        "OnExit",
                        "Update",
                        "FixedUpdate",
                        "PostUpdate",
                    ];
                    for stage in stages {
                        let color = stage_color(stage);
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, 2.0, color);
                        ui.label(stage);
                        ui.add_space(8.0);
                    }
                });

                ui.separator();

                // ── List view: per-stage drag-to-reorder rows ──
                // v0.5 — answers "which system runs first inside
                // Update?" without making the user trace arrows on
                // the canvas. The drag uses an explicit "↑ / ↓"
                // pair instead of an HTML5-style drag handle so the
                // intent is unambiguous in egui.
                if self.system_graph_view == SystemGraphView::List {
                    self.render_system_list(ui);
                    return;
                }

                // Pre-measure each node's width so long labels (e.g.
                // `OnEnter(AppScene::Scene2)`) don't clip and so the
                // canvas can be sized to fit all nodes plus a margin.
                let node_widths: Vec<f32> = self
                    .system_graph
                    .systems
                    .iter()
                    .map(|s| measure_node_width(ui, &s.name, &s.stage))
                    .collect();

                // Compute the canvas extent as the bounding box of all
                // nodes plus padding. ScrollArea will then expose any
                // overflow as a horizontal/vertical scrollbar instead
                // of clipping at the window edge.
                let mut canvas_extent = egui::vec2(800.0, 400.0);
                for (sys, &w) in self.system_graph.systems.iter().zip(node_widths.iter()) {
                    canvas_extent.x = canvas_extent.x.max(sys.position[0] + w + 40.0);
                    canvas_extent.y = canvas_extent.y.max(sys.position[1] + NODE_HEIGHT + 40.0);
                }

                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let (response, painter) =
                            ui.allocate_painter(canvas_extent, egui::Sense::click_and_drag());
                        let canvas_origin = response.rect.min;

                        // Build a name->index lookup for dependency arrows.
                        let name_to_idx: std::collections::HashMap<String, usize> = self
                            .system_graph
                            .systems
                            .iter()
                            .enumerate()
                            .map(|(i, s)| (s.name.clone(), i))
                            .collect();

                        // Draw dependency arrows first (behind nodes).
                        // Use each endpoint's measured width so the
                        // arrow lands at the visual centre, not the
                        // centre of the legacy 140px box.
                        for (i, sys) in self.system_graph.systems.iter().enumerate() {
                            let w_to = node_widths[i];
                            let to_center = egui::pos2(
                                canvas_origin.x + sys.position[0] + w_to / 2.0,
                                canvas_origin.y + sys.position[1] + NODE_HEIGHT / 2.0,
                            );
                            for dep_name in &sys.dependencies {
                                if let Some(&dep_idx) = name_to_idx.get(dep_name) {
                                    let dep = &self.system_graph.systems[dep_idx];
                                    let w_from = node_widths[dep_idx];
                                    let from_center = egui::pos2(
                                        canvas_origin.x + dep.position[0] + w_from / 2.0,
                                        canvas_origin.y + dep.position[1] + NODE_HEIGHT / 2.0,
                                    );
                                    painter.arrow(
                                        from_center,
                                        to_center - from_center,
                                        egui::Stroke::new(
                                            1.5,
                                            egui::Color32::from_rgb(200, 200, 200),
                                        ),
                                    );
                                }
                            }
                        }

                        // Draw nodes at their measured widths.
                        for (i, sys) in self.system_graph.systems.iter().enumerate() {
                            let w = node_widths[i];
                            let node_rect = egui::Rect::from_min_size(
                                egui::pos2(
                                    canvas_origin.x + sys.position[0],
                                    canvas_origin.y + sys.position[1],
                                ),
                                egui::vec2(w, NODE_HEIGHT),
                            );
                            let color = stage_color(&sys.stage);
                            painter.rect_filled(node_rect, 4.0, color.linear_multiply(0.3));
                            painter.rect_stroke(
                                node_rect,
                                4.0,
                                egui::Stroke::new(1.0, color),
                                egui::StrokeKind::Middle,
                            );

                            let label = format!("{}\n({})", sys.name, sys.stage);
                            painter.text(
                                node_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                &label,
                                egui::FontId::proportional(NODE_LABEL_FONT_SIZE),
                                egui::Color32::from_rgb(220, 220, 220),
                            );
                        }

                        // Handle dragging (hit-test against measured
                        // width so the right edge of long nodes stays
                        // grabbable).
                        if response.dragged() {
                            if let Some(pointer) = response.interact_pointer_pos() {
                                let rel = pointer - canvas_origin;
                                for (i, sys) in self.system_graph.systems.iter_mut().enumerate() {
                                    let w = node_widths[i];
                                    let node_rect = egui::Rect::from_min_size(
                                        egui::pos2(sys.position[0], sys.position[1]),
                                        egui::vec2(w, NODE_HEIGHT),
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
            });

        self.system_graph_open = open;
    }

    /// Render the per-stage ordered list view. Each stage gets its own
    /// header; rows under the stage are drawn in execution order
    /// (sorted by `SystemNode::order`) with ↑ / ↓ buttons to swap with
    /// the previous / next sibling. Sibling-swap is what produces the
    /// drag-to-reorder behaviour for v0.5; a true mouse-drag handle
    /// can layer on later without changing the data model.
    fn render_system_list(&mut self, ui: &mut egui::Ui) {
        // Group system *indices* (not clones) per stage so we can
        // mutate `self.system_graph.systems[i].order` directly when
        // the user clicks ↑ / ↓.
        let mut by_stage: std::collections::BTreeMap<String, Vec<usize>> =
            std::collections::BTreeMap::new();
        for (i, sys) in self.system_graph.systems.iter().enumerate() {
            by_stage.entry(sys.stage.clone()).or_default().push(i);
        }
        for indices in by_stage.values_mut() {
            indices.sort_by_key(|&i| self.system_graph.systems[i].order);
        }

        // Pending swap so we don't mutate `self.system_graph.systems`
        // mid-iteration (rust borrow checker).
        let mut swap: Option<(usize, usize)> = None;

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (stage, indices) in &by_stage {
                ui.add_space(6.0);
                let color = stage_color(stage);
                ui.horizontal(|ui| {
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 2.0, color);
                    ui.label(egui::RichText::new(stage).strong().size(13.0));
                    ui.label(
                        egui::RichText::new(format!("({} systems)", indices.len()))
                            .size(11.0)
                            .color(egui::Color32::from_gray(150)),
                    );
                });

                for (row_idx, &sys_idx) in indices.iter().enumerate() {
                    let sys = &self.system_graph.systems[sys_idx];
                    egui::Frame::NONE
                        .fill(egui::Color32::from_rgb(34, 36, 42))
                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 64, 72)))
                        .corner_radius(egui::CornerRadius::same(4))
                        .inner_margin(egui::Margin::same(6))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(format!("{}.", row_idx + 1))
                                        .color(egui::Color32::from_gray(120))
                                        .monospace(),
                                );
                                ui.label(egui::RichText::new(&sys.name).monospace().strong());
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        let last = row_idx + 1 == indices.len();
                                        let first = row_idx == 0;
                                        // Buttons appear in screen
                                        // order ↑ then ↓ thanks to
                                        // right-to-left placement
                                        // (last placed = leftmost).
                                        if ui.add_enabled(!last, egui::Button::new("↓")).clicked()
                                        {
                                            swap = Some((sys_idx, indices[row_idx + 1]));
                                        }
                                        if ui.add_enabled(!first, egui::Button::new("↑")).clicked()
                                        {
                                            swap = Some((sys_idx, indices[row_idx - 1]));
                                        }
                                    },
                                );
                            });
                        });
                }
            }
            if self.system_graph.systems.is_empty() {
                ui.add_space(40.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new(
                            "No systems yet — click \"Scan Code\" or \"+ System\" above.",
                        )
                        .color(egui::Color32::from_gray(150)),
                    );
                });
            }
        });

        if let Some((a, b)) = swap {
            // Swap the `order` field — doing it via the explicit
            // index keeps `Vec<SystemNode>` insertion order untouched
            // (which the codegen output relies on for diff stability).
            let order_a = self.system_graph.systems[a].order;
            let order_b = self.system_graph.systems[b].order;
            self.system_graph.systems[a].order = order_b;
            self.system_graph.systems[b].order = order_a;
        }
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

        // Map system name → stage. Multiple files registering the same
        // function to different schedules collapses to "last write wins"
        // — this is rare in practice and matches the previous behaviour
        // of taking whichever was scanned latest.
        let scan_map: std::collections::HashMap<String, String> = found
            .into_iter()
            .map(|(stage, name)| (name, stage))
            .collect();

        // 1. Update stages of existing nodes that still appear in scan
        //    output. This is the fix for stale stages — e.g. a scene
        //    `setup_test` that used to be `Startup` becoming
        //    `OnEnter(AppScene::Test)` after the codegen swap.
        for sys in &mut self.system_graph.systems {
            if let Some(stage) = scan_map.get(&sys.name) {
                sys.stage = stage.clone();
            }
        }

        // 2. Garbage-collect entries whose name isn't a valid Rust
        //    identifier. The pre-fix parser produced stray nodes named
        //    `(` or `(setup_world` from tuple-form `add_systems` calls;
        //    those are invalid as system names and have to be dropped
        //    before we re-add the correctly-parsed versions.
        self.system_graph
            .systems
            .retain(|s| is_valid_system_name(&s.name));

        // 3. Add new nodes for names not yet in the graph. Manual
        //    `+ System` adds and untouched scanned nodes both survive
        //    here because the retain above only drops invalid names.
        let existing_names: std::collections::HashSet<String> = self
            .system_graph
            .systems
            .iter()
            .map(|s| s.name.clone())
            .collect();

        // Column step has to be wide enough for the longest label
        // we expect — `cleanup_scene2 (OnExit(AppScene::Scene2))` is
        // ~310px, so the previous 170px step had nodes overlapping
        // every time someone scanned a state-driven scene project.
        const SCAN_COL_STEP: f32 = 320.0;
        const SCAN_ROW_STEP: f32 = 70.0;
        const SCAN_COLS: f32 = 3.0;

        let mut count = self.system_graph.systems.len();
        for (name, stage) in scan_map {
            if !existing_names.contains(&name) {
                self.system_graph.systems.push(SystemNode {
                    name,
                    stage,
                    position: [
                        40.0 + (count as f32 % SCAN_COLS) * SCAN_COL_STEP,
                        40.0 + (count as f32 / SCAN_COLS).floor() * SCAN_ROW_STEP,
                    ],
                    dependencies: Vec::new(),
                    order: count as u32,
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
                    order: 0,
                },
                SystemNode {
                    name: "collision".into(),
                    stage: "Update".into(),
                    position: [300.0, 200.0],
                    dependencies: vec!["movement".into()],
                    order: 1,
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

    #[test]
    fn scan_tuple_form_extracts_each_system() {
        // Regression: the old line-oriented parser saw the bare `(` on
        // line 1 and produced a node named `(`. Now we span the whole
        // `add_systems(...)` call and split the inner tuple correctly.
        let code = r#"
            .add_systems(Update, (
                update_a,
                update_b,
                update_c,
            ))
        "#;
        let results = scan_systems_from_code(code);
        assert_eq!(results.len(), 3);
        for r in &results {
            assert_eq!(r.0, "Update");
        }
        let names: Vec<&str> = results.iter().map(|r| r.1.as_str()).collect();
        assert_eq!(names, vec!["update_a", "update_b", "update_c"]);
    }

    #[test]
    fn scan_state_bound_schedules() {
        // OnEnter / OnExit schedules carry the state value as part of
        // the schedule expression. The parser must keep the whole
        // `OnEnter(AppScene::Test)` together as a single stage string
        // and not let the inner `,`-free parens fool it.
        let code = r#"
            app.add_systems(OnEnter(AppScene::Test), setup_test)
                .add_systems(OnExit(AppScene::Test), cleanup_test);
        "#;
        let results = scan_systems_from_code(code);
        assert_eq!(results.len(), 2);
        assert_eq!(
            results[0],
            ("OnEnter(AppScene::Test)".into(), "setup_test".into())
        );
        assert_eq!(
            results[1],
            ("OnExit(AppScene::Test)".into(), "cleanup_test".into())
        );
    }

    #[test]
    fn scan_strips_method_chains() {
        // `.before` / `.after` / `.run_if` modify a system's ordering
        // but the system *name* is just the identifier in front. We
        // also accept them in tuple form so each entry's chain is
        // stripped independently.
        let code = r#"
            .add_systems(Update, movement.before(physics))
            .add_systems(Update, (
                input.before(movement),
                physics.after(movement),
            ))
        "#;
        let results = scan_systems_from_code(code);
        let names: Vec<&str> = results.iter().map(|r| r.1.as_str()).collect();
        assert_eq!(names, vec!["movement", "input", "physics"]);
    }

    #[test]
    fn scan_does_not_match_unrelated_identifiers() {
        // Don't pick up `my_add_systems(` or function definitions
        // that happen to contain `add_systems(` mid-identifier.
        let code = r#"
            fn my_add_systems(app: &mut App) { }
            .add_systems(Update, real_system)
        "#;
        let results = scan_systems_from_code(code);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].1, "real_system");
    }

    #[test]
    fn scan_handles_paths_with_double_colon() {
        // Systems referenced through a module path keep the path
        // intact, since they're still unique identifiers in the
        // graph (the user may have two `setup` functions in
        // different modules).
        let code = r#".add_systems(Startup, scenes::setup_world)"#;
        let results = scan_systems_from_code(code);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].1, "scenes::setup_world");
    }

    #[test]
    fn is_valid_system_name_filters_garbage() {
        assert!(is_valid_system_name("setup_test"));
        assert!(is_valid_system_name("scenes::setup_world"));
        assert!(is_valid_system_name("_internal"));
        // Reject the noise the old parser used to produce.
        assert!(!is_valid_system_name("("));
        assert!(!is_valid_system_name("(setup_world"));
        assert!(!is_valid_system_name(""));
        assert!(!is_valid_system_name("123abc"));
    }

    #[test]
    fn stage_color_resolves_state_bound_schedules() {
        // Different schedule literals for the same family must share
        // the same colour — otherwise every scene would render with
        // its own random hue in the graph legend.
        let test_enter = stage_color("OnEnter(AppScene::Test)");
        let other_enter = stage_color("OnEnter(AppScene::Other)");
        assert_eq!(test_enter, other_enter);
        let test_exit = stage_color("OnExit(AppScene::Test)");
        assert_ne!(test_enter, test_exit);
    }
}
