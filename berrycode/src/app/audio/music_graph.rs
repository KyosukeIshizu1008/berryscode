//! Music graph — node-graph state machine for BGM transitions.
//!
//! v0.6 Phase D. Authors crossfade rules between music clips with
//! optional "wait for bar" stinger triggers and vertical re-mixing
//! through parameter-weighted parallel layers. The data model
//! mirrors the System Graph / Shader Graph shape so the editor
//! widget can converge on a single shared node-graph helper in a
//! later release.
//!
//! Runtime side (state machine on top of `bevy_audio::AudioSink`,
//! crossfade scheduling, cue-point alignment) ships in v0.6.1
//! alongside the Phase C event dispatcher.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MusicNodeKind {
    /// Plain music clip — runtime spawns a looping AudioSink.
    Clip,
    /// One-shot stinger that overlays without taking over.
    Stinger,
    /// Cue point reachable from multiple Clips, useful for
    /// "return to verse" loops.
    Cue,
}

impl Default for MusicNodeKind {
    fn default() -> Self {
        Self::Clip
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicNode {
    pub id: u64,
    pub kind: MusicNodeKind,
    /// Asset path the clip / stinger plays. Cue nodes ignore this.
    pub asset_path: String,
    /// Where the node draws on the canvas (drag-to-move handled by
    /// the editor). Mirrors `SystemNode::position`.
    pub position: [f32; 2],
    /// Display name. Cues use it as a label, clips fall back to the
    /// asset filename when empty.
    pub label: String,
    /// Default looping flag for clips. Stingers ignore this.
    #[serde(default = "default_loop")]
    pub looped: bool,
    /// Constant gain applied on top of any layer weight.
    #[serde(default = "default_gain")]
    pub gain: f32,
}

fn default_loop() -> bool {
    true
}
fn default_gain() -> f32 {
    1.0
}

impl Default for MusicNode {
    fn default() -> Self {
        Self {
            id: 0,
            kind: MusicNodeKind::Clip,
            asset_path: String::new(),
            position: [80.0, 80.0],
            label: String::new(),
            looped: true,
            gain: 1.0,
        }
    }
}

/// Transition rule between two nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransitionTrigger {
    /// Fire as soon as the source node finishes playing (or at
    /// the next bar boundary if `wait_for_bar` is set).
    Immediate,
    /// Trigger when the named parameter crosses `value`. Direction
    /// is "rising past" — the runtime can mirror with a second rule
    /// for "falling below" use cases.
    OnParameterAbove { parameter: String, value: f32 },
    /// Trigger when the named event fires (Phase C events).
    OnEvent { event_name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicEdge {
    pub from_node: u64,
    pub to_node: u64,
    pub trigger: TransitionTrigger,
    /// Crossfade duration in seconds. `0.0` = hard cut.
    #[serde(default = "default_crossfade")]
    pub crossfade_secs: f32,
    /// If true, defer the transition to the next bar boundary so
    /// the cut feels musical even when the trigger fires off-beat.
    #[serde(default)]
    pub wait_for_bar: bool,
}

fn default_crossfade() -> f32 {
    1.5
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MusicGraph {
    pub name: String,
    pub nodes: Vec<MusicNode>,
    pub edges: Vec<MusicEdge>,
    /// Tempo in BPM. Used by the runtime to compute bar boundaries
    /// when `wait_for_bar` is set on an edge.
    #[serde(default = "default_bpm")]
    pub bpm: f32,
    /// Beats per bar (e.g. 4 for 4/4 time).
    #[serde(default = "default_beats_per_bar")]
    pub beats_per_bar: u32,
}

fn default_bpm() -> f32 {
    120.0
}
fn default_beats_per_bar() -> u32 {
    4
}

pub fn save_music_graph(graph: &MusicGraph, path: &str) -> Result<(), String> {
    let s = ron::ser::to_string_pretty(graph, ron::ser::PrettyConfig::default())
        .map_err(|e| e.to_string())?;
    std::fs::write(path, s).map_err(|e| e.to_string())
}

pub fn load_music_graph(path: &str) -> Result<MusicGraph, String> {
    let s = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    ron::from_str(&s).map_err(|e| e.to_string())
}

const NODE_W: f32 = 140.0;
const NODE_H: f32 = 50.0;

fn node_color(kind: MusicNodeKind) -> egui::Color32 {
    match kind {
        MusicNodeKind::Clip => egui::Color32::from_rgb(80, 140, 220),
        MusicNodeKind::Stinger => egui::Color32::from_rgb(200, 130, 80),
        MusicNodeKind::Cue => egui::Color32::from_rgb(140, 100, 200),
    }
}

impl crate::app::BerryCodeApp {
    /// Render the music graph editor as a free-floating window.
    /// Reuses the System Graph canvas idiom (drag nodes around,
    /// arrows for transitions) but doesn't share code yet — once a
    /// third graph editor lands we should extract a shared helper.
    pub(crate) fn render_music_graph_editor(&mut self, ctx: &egui::Context) {
        if !self.music_graph_window_open {
            return;
        }
        let mut open = self.music_graph_window_open;

        egui::Window::new("Music Graph")
            .open(&mut open)
            .default_size([720.0, 480.0])
            .resizable(true)
            .show(ctx, |ui| {
                let graph = &mut self.music_graph;

                // Toolbar.
                ui.horizontal(|ui| {
                    if ui.button("+ Clip").clicked() {
                        let id = next_id(graph);
                        graph.nodes.push(MusicNode {
                            id,
                            kind: MusicNodeKind::Clip,
                            label: format!("Clip_{}", graph.nodes.len()),
                            position: [
                                80.0 + (graph.nodes.len() as f32 % 4.0) * 160.0,
                                80.0 + (graph.nodes.len() as f32 / 4.0).floor() * 80.0,
                            ],
                            ..Default::default()
                        });
                    }
                    if ui.button("+ Stinger").clicked() {
                        let id = next_id(graph);
                        graph.nodes.push(MusicNode {
                            id,
                            kind: MusicNodeKind::Stinger,
                            label: format!("Stinger_{}", graph.nodes.len()),
                            position: [80.0 + graph.nodes.len() as f32 * 30.0, 240.0],
                            ..Default::default()
                        });
                    }
                    if ui.button("+ Cue").clicked() {
                        let id = next_id(graph);
                        graph.nodes.push(MusicNode {
                            id,
                            kind: MusicNodeKind::Cue,
                            label: format!("Cue_{}", graph.nodes.len()),
                            position: [200.0, 320.0],
                            ..Default::default()
                        });
                    }
                    ui.separator();
                    ui.label("BPM:");
                    ui.add(
                        egui::DragValue::new(&mut graph.bpm)
                            .speed(0.5)
                            .range(20.0..=400.0),
                    );
                    ui.label("Beats/bar:");
                    ui.add(
                        egui::DragValue::new(&mut graph.beats_per_bar)
                            .speed(1.0)
                            .range(1..=16),
                    );
                    if ui.button("Save").clicked() {
                        let path = format!("{}/music_graph.bmusic", self.root_path);
                        match save_music_graph(graph, &path) {
                            Ok(_) => {
                                self.status_message = format!("Music graph saved: {path}");
                                self.status_message_timestamp =
                                    Some(std::time::Instant::now());
                            }
                            Err(e) => {
                                self.status_message = format!("Save failed: {e}");
                                self.status_message_timestamp =
                                    Some(std::time::Instant::now());
                            }
                        }
                    }
                });
                ui.separator();

                // Canvas.
                let (response, painter) = ui.allocate_painter(
                    ui.available_size_before_wrap(),
                    egui::Sense::click_and_drag(),
                );
                let origin = response.rect.min;

                // Edges first (behind nodes).
                for edge in &graph.edges {
                    let from = graph.nodes.iter().find(|n| n.id == edge.from_node);
                    let to = graph.nodes.iter().find(|n| n.id == edge.to_node);
                    let (Some(from), Some(to)) = (from, to) else {
                        continue;
                    };
                    let p1 = egui::pos2(
                        origin.x + from.position[0] + NODE_W / 2.0,
                        origin.y + from.position[1] + NODE_H / 2.0,
                    );
                    let p2 = egui::pos2(
                        origin.x + to.position[0] + NODE_W / 2.0,
                        origin.y + to.position[1] + NODE_H / 2.0,
                    );
                    painter.arrow(
                        p1,
                        p2 - p1,
                        egui::Stroke::new(1.5, egui::Color32::from_rgb(180, 180, 180)),
                    );
                }

                // Nodes.
                for node in &graph.nodes {
                    let rect = egui::Rect::from_min_size(
                        egui::pos2(
                            origin.x + node.position[0],
                            origin.y + node.position[1],
                        ),
                        egui::vec2(NODE_W, NODE_H),
                    );
                    let color = node_color(node.kind);
                    painter.rect_filled(rect, 4.0, color.linear_multiply(0.3));
                    painter.rect_stroke(
                        rect,
                        4.0,
                        egui::Stroke::new(1.0, color),
                        egui::StrokeKind::Middle,
                    );
                    let label_text = if node.label.is_empty() {
                        node.asset_path
                            .rsplit('/')
                            .next()
                            .unwrap_or(&node.asset_path)
                            .to_string()
                    } else {
                        node.label.clone()
                    };
                    painter.text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        format!(
                            "{}\n{}",
                            label_text,
                            match node.kind {
                                MusicNodeKind::Clip => "Clip",
                                MusicNodeKind::Stinger => "Stinger",
                                MusicNodeKind::Cue => "Cue",
                            }
                        ),
                        egui::FontId::proportional(11.0),
                        egui::Color32::from_rgb(220, 220, 220),
                    );
                }

                // Drag a node by clicking its rect and dragging.
                if response.dragged() {
                    if let Some(pointer) = response.interact_pointer_pos() {
                        let rel = pointer - origin;
                        for node in &mut graph.nodes {
                            let rect = egui::Rect::from_min_size(
                                egui::pos2(node.position[0], node.position[1]),
                                egui::vec2(NODE_W, NODE_H),
                            );
                            if rect.contains(egui::pos2(rel.x, rel.y)) {
                                node.position[0] += response.drag_delta().x;
                                node.position[1] += response.drag_delta().y;
                                break;
                            }
                        }
                    }
                }

                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(
                        "Edge editing + crossfade preview ship in v0.6.1; data model handles all triggers already.",
                    )
                    .size(10.5)
                    .color(egui::Color32::from_gray(120))
                    .italics(),
                );
            });

        self.music_graph_window_open = open;
    }
}

fn next_id(graph: &MusicGraph) -> u64 {
    graph.nodes.iter().map(|n| n.id).max().unwrap_or(0) + 1
}
