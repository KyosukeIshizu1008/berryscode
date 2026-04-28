//! Event-driven audio data model + editor.
//!
//! v0.6 Phase C. First-class "audio event" objects (one-shot,
//! loop, ducking, parameter-driven layer mix) so gameplay code
//! calls `events.fire("Footstep")` instead of spawning raw
//! `AudioSource`s. Persistence target is a `<root>/audio/<name>.baudio`
//! file alongside the existing scene RON — separate so audio
//! designers can iterate without touching scene files.
//!
//! This module ships the data model + the editor UI in v0.6.0;
//! the runtime dispatcher (`AudioEventRegistry::fire(name)`,
//! ducking application, parameter-driven layer weights) lands in
//! v0.6.1 alongside the rest of the runtime hooks.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum EventKind {
    OneShot,
    Loop,
}

impl Default for EventKind {
    fn default() -> Self {
        Self::OneShot
    }
}

/// One source layer within an event. Multiple layers play in
/// parallel; the runtime sums their gain after applying the
/// parameter-driven weight curve below.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventLayer {
    pub asset_path: String,
    /// Constant layer gain on top of the curve. Keeps "always at
    /// 50% volume" simple to express without having to author a
    /// flat curve.
    #[serde(default = "default_gain")]
    pub gain: f32,
    /// Optional parameter that drives the layer's effective weight
    /// in [0, 1]. `None` = always full weight.
    #[serde(default)]
    pub parameter: Option<String>,
    /// Pairs of (parameter value, weight). Linearly interpolated
    /// between adjacent points. Empty = full weight.
    #[serde(default)]
    pub curve: Vec<(f32, f32)>,
}

fn default_gain() -> f32 {
    1.0
}

impl Default for EventLayer {
    fn default() -> Self {
        Self {
            asset_path: String::new(),
            gain: 1.0,
            parameter: None,
            curve: Vec::new(),
        }
    }
}

/// Side-chain ducking — when `target_event` is firing, attenuate
/// our event by `attenuation_db` for `release_ms` after the trigger
/// stops. Applied multiplicatively across overlapping rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuckingRule {
    pub target_event: String,
    /// Attenuation in dB (negative = quieter). `-6.0` cuts the
    /// volume in half perceptually.
    #[serde(default = "default_duck_db")]
    pub attenuation_db: f32,
    /// Release in milliseconds after the target event ends. Long
    /// releases produce smoother ducking; short ones snap back.
    #[serde(default = "default_release_ms")]
    pub release_ms: f32,
}

fn default_duck_db() -> f32 {
    -6.0
}
fn default_release_ms() -> f32 {
    200.0
}

impl Default for DuckingRule {
    fn default() -> Self {
        Self {
            target_event: String::new(),
            attenuation_db: -6.0,
            release_ms: 200.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ParameterKind {
    /// Continuous float in [min, max] range.
    Float,
    /// Boolean flag — exposed to layers as 0.0 / 1.0.
    Bool,
}

impl Default for ParameterKind {
    fn default() -> Self {
        Self::Float
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventParameter {
    pub name: String,
    pub kind: ParameterKind,
    /// Default value used when the runtime hasn't been given an
    /// explicit set yet.
    #[serde(default)]
    pub default_value: f32,
}

impl Default for EventParameter {
    fn default() -> Self {
        Self {
            name: String::new(),
            kind: ParameterKind::Float,
            default_value: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioEvent {
    pub name: String,
    pub kind: EventKind,
    pub layers: Vec<EventLayer>,
    pub ducking: Vec<DuckingRule>,
    pub parameters: Vec<EventParameter>,
}

impl Default for AudioEvent {
    fn default() -> Self {
        Self {
            name: "Untitled".into(),
            kind: EventKind::OneShot,
            layers: vec![EventLayer::default()],
            ducking: Vec::new(),
            parameters: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AudioEventRegistry {
    pub events: Vec<AudioEvent>,
    /// Index of the event currently selected in the editor. Skipped
    /// from disk so re-opens don't carry stale UI state.
    #[serde(skip)]
    pub selected: Option<usize>,
}

pub fn save_event_registry(reg: &AudioEventRegistry, path: &str) -> Result<(), String> {
    let s = ron::ser::to_string_pretty(reg, ron::ser::PrettyConfig::default())
        .map_err(|e| e.to_string())?;
    std::fs::write(path, s).map_err(|e| e.to_string())
}

pub fn load_event_registry(path: &str) -> Result<AudioEventRegistry, String> {
    let s = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    ron::from_str(&s).map_err(|e| e.to_string())
}

/// Render the event editor as a free-floating window. Toggled by
/// `BerryCodeApp::audio_events_window_open`; mutates the registry
/// stored on the app in place.
impl crate::app::BerryCodeApp {
    pub(crate) fn render_audio_events_editor(&mut self, ctx: &egui::Context) {
        if !self.audio_events_window_open {
            return;
        }
        let mut open = self.audio_events_window_open;

        egui::Window::new("Audio Events")
            .open(&mut open)
            .default_size([720.0, 480.0])
            .resizable(true)
            .show(ctx, |ui| {
                let registry = &mut self.audio_events;

                // Toolbar.
                ui.horizontal(|ui| {
                    if ui.button("+ Event").clicked() {
                        registry.events.push(AudioEvent {
                            name: format!("Event_{}", registry.events.len()),
                            ..Default::default()
                        });
                        registry.selected = Some(registry.events.len() - 1);
                    }
                    if ui.button("Save").clicked() {
                        let path = format!("{}/audio_events.baudio", self.root_path);
                        match save_event_registry(registry, &path) {
                            Ok(_) => {
                                self.status_message = format!("Audio events saved: {path}");
                                self.status_message_timestamp = Some(std::time::Instant::now());
                            }
                            Err(e) => {
                                self.status_message = format!("Save failed: {e}");
                                self.status_message_timestamp = Some(std::time::Instant::now());
                            }
                        }
                    }
                });
                ui.separator();

                ui.horizontal(|ui| {
                    // Left: event list.
                    ui.vertical(|ui| {
                        ui.set_width(180.0);
                        ui.label(
                            egui::RichText::new("EVENTS")
                                .size(11.0)
                                .color(egui::Color32::from_gray(150))
                                .strong(),
                        );
                        let mut remove: Option<usize> = None;
                        for (i, ev) in registry.events.iter().enumerate() {
                            let selected = registry.selected == Some(i);
                            let row = ui.horizontal(|ui| {
                                if ui.selectable_label(selected, &ev.name).clicked() {
                                    registry.selected = Some(i);
                                }
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui.small_button("✕").clicked() {
                                            remove = Some(i);
                                        }
                                    },
                                );
                            });
                            row.response.on_hover_text(match ev.kind {
                                EventKind::OneShot => "OneShot",
                                EventKind::Loop => "Loop",
                            });
                        }
                        if let Some(i) = remove {
                            registry.events.remove(i);
                            if registry.selected == Some(i) {
                                registry.selected = None;
                            }
                        }
                    });

                    ui.separator();

                    // Right: selected-event inspector.
                    ui.vertical(|ui| {
                        let Some(idx) = registry.selected else {
                            ui.label("Select an event on the left, or click \"+ Event\".");
                            return;
                        };
                        if idx >= registry.events.len() {
                            return;
                        }
                        render_audio_event_inspector(ui, &mut registry.events[idx]);
                    });
                });
            });

        self.audio_events_window_open = open;
    }
}

fn render_audio_event_inspector(ui: &mut egui::Ui, ev: &mut AudioEvent) {
    ui.horizontal(|ui| {
        ui.label("Name:");
        ui.add(egui::TextEdit::singleline(&mut ev.name).desired_width(220.0));
        ui.label("Kind:");
        egui::ComboBox::from_id_salt("audio_event_kind")
            .selected_text(match ev.kind {
                EventKind::OneShot => "OneShot",
                EventKind::Loop => "Loop",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut ev.kind, EventKind::OneShot, "OneShot");
                ui.selectable_value(&mut ev.kind, EventKind::Loop, "Loop");
            });
    });

    ui.add_space(4.0);
    ui.collapsing("Layers", |ui| {
        let mut remove: Option<usize> = None;
        for (i, layer) in ev.layers.iter_mut().enumerate() {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(format!("Layer {}", i + 1));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("✕").clicked() {
                            remove = Some(i);
                        }
                    });
                });
                ui.horizontal(|ui| {
                    ui.label("Asset:");
                    ui.add(
                        egui::TextEdit::singleline(&mut layer.asset_path)
                            .hint_text("audio/footstep_grass_01.wav")
                            .desired_width(220.0),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Gain:");
                    ui.add(egui::Slider::new(&mut layer.gain, 0.0..=2.0).suffix("×"));
                });
                ui.horizontal(|ui| {
                    ui.label("Parameter:");
                    let mut param = layer.parameter.clone().unwrap_or_default();
                    ui.add(
                        egui::TextEdit::singleline(&mut param)
                            .hint_text("(empty = full weight)")
                            .desired_width(150.0),
                    );
                    layer.parameter = if param.is_empty() { None } else { Some(param) };
                });
            });
        }
        if let Some(i) = remove {
            ev.layers.remove(i);
        }
        if ui.button("+ Layer").clicked() {
            ev.layers.push(EventLayer::default());
        }
    });

    ui.collapsing("Ducking", |ui| {
        let mut remove: Option<usize> = None;
        for (i, rule) in ev.ducking.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut rule.target_event)
                        .hint_text("target event name")
                        .desired_width(160.0),
                );
                ui.add(
                    egui::DragValue::new(&mut rule.attenuation_db)
                        .speed(0.5)
                        .range(-60.0..=0.0)
                        .suffix(" dB"),
                );
                ui.add(
                    egui::DragValue::new(&mut rule.release_ms)
                        .speed(10.0)
                        .range(0.0..=5000.0)
                        .suffix(" ms"),
                );
                if ui.small_button("✕").clicked() {
                    remove = Some(i);
                }
            });
        }
        if let Some(i) = remove {
            ev.ducking.remove(i);
        }
        if ui.button("+ Ducking rule").clicked() {
            ev.ducking.push(DuckingRule::default());
        }
    });

    ui.collapsing("Parameters", |ui| {
        let mut remove: Option<usize> = None;
        for (i, param) in ev.parameters.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut param.name)
                        .hint_text("intensity")
                        .desired_width(140.0),
                );
                egui::ComboBox::from_id_salt(("audio_param_kind", i))
                    .selected_text(match param.kind {
                        ParameterKind::Float => "Float",
                        ParameterKind::Bool => "Bool",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut param.kind, ParameterKind::Float, "Float");
                        ui.selectable_value(&mut param.kind, ParameterKind::Bool, "Bool");
                    });
                ui.add(egui::DragValue::new(&mut param.default_value).speed(0.05));
                if ui.small_button("✕").clicked() {
                    remove = Some(i);
                }
            });
        }
        if let Some(i) = remove {
            ev.parameters.remove(i);
        }
        if ui.button("+ Parameter").clicked() {
            ev.parameters.push(EventParameter::default());
        }
    });

    ui.add_space(8.0);
    ui.label(
        egui::RichText::new(
            "Runtime dispatcher (events.fire(name), ducking, parameter-driven mix) ships in v0.6.1.",
        )
        .size(10.5)
        .color(egui::Color32::from_gray(120))
        .italics(),
    );
}
