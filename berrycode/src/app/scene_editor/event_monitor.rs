//! Bevy Event Monitor: live log of events during Play Mode.
//!
//! Shows a scrollable list of events with timestamp, type, and data columns.
//! During Play Mode, animation, physics, and other subsystems push entries
//! into the log. The user can filter by event type.

use crate::app::BerryCodeApp;
use serde::{Deserialize, Serialize};

/// A single event entry in the monitor log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEntry {
    /// Time in seconds since play mode started.
    pub timestamp: f64,
    /// Type/category of the event (e.g. "Animation", "Physics", "Collision").
    pub event_type: String,
    /// Human-readable data payload.
    pub data: String,
}

impl EventEntry {
    pub fn new(timestamp: f64, event_type: impl Into<String>, data: impl Into<String>) -> Self {
        Self {
            timestamp,
            event_type: event_type.into(),
            data: data.into(),
        }
    }
}

/// Known event type categories for filtering.
pub const EVENT_TYPES: &[&str] = &[
    "Animation",
    "Physics",
    "Collision",
    "Spawn",
    "Despawn",
    "State",
    "Custom",
];

impl BerryCodeApp {
    /// Render the Event Monitor window.
    pub(crate) fn render_event_monitor(&mut self, ctx: &egui::Context) {
        if !self.event_monitor_open {
            return;
        }

        let mut open = self.event_monitor_open;

        egui::Window::new("Event Monitor")
            .open(&mut open)
            .default_size([600.0, 350.0])
            .resizable(true)
            .show(ctx, |ui| {
                // Toolbar
                ui.horizontal(|ui| {
                    if ui.button("Clear").clicked() {
                        self.event_log.clear();
                    }
                    ui.separator();
                    ui.label("Filter:");
                    ui.text_edit_singleline(&mut self.event_filter_text);
                    ui.separator();
                    ui.label(format!("{} events", self.event_log.len()));
                });

                ui.separator();

                // Filter checkboxes
                ui.horizontal_wrapped(|ui| {
                    for etype in EVENT_TYPES {
                        let key = etype.to_string();
                        let enabled = self.event_filter_types.contains(&key);
                        let mut checked = enabled;
                        if ui.checkbox(&mut checked, *etype).changed() {
                            if checked {
                                self.event_filter_types.insert(key);
                            } else {
                                self.event_filter_types.remove(&key.to_string());
                            }
                        }
                    }
                });

                ui.separator();

                // Column headers
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Time").strong().monospace());
                    ui.add_space(40.0);
                    ui.label(egui::RichText::new("Type").strong().monospace());
                    ui.add_space(60.0);
                    ui.label(egui::RichText::new("Data").strong().monospace());
                });
                ui.separator();

                // Scrollable event list
                let filter_text = self.event_filter_text.to_lowercase();
                let filter_types = &self.event_filter_types;

                egui::ScrollArea::vertical()
                    .id_salt("event_log")
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for entry in &self.event_log {
                            // Apply type filter
                            if !filter_types.is_empty() && !filter_types.contains(&entry.event_type)
                            {
                                continue;
                            }
                            // Apply text filter
                            if !filter_text.is_empty()
                                && !entry.event_type.to_lowercase().contains(&filter_text)
                                && !entry.data.to_lowercase().contains(&filter_text)
                            {
                                continue;
                            }

                            let type_color = match entry.event_type.as_str() {
                                "Animation" => egui::Color32::from_rgb(120, 200, 255),
                                "Physics" => egui::Color32::from_rgb(255, 200, 80),
                                "Collision" => egui::Color32::from_rgb(255, 120, 120),
                                "Spawn" => egui::Color32::from_rgb(120, 255, 120),
                                "Despawn" => egui::Color32::from_rgb(255, 100, 100),
                                "State" => egui::Color32::from_rgb(200, 160, 255),
                                _ => egui::Color32::from_rgb(180, 180, 180),
                            };

                            ui.horizontal(|ui| {
                                ui.monospace(format!("{:8.3}", entry.timestamp));
                                ui.add_space(8.0);
                                ui.label(
                                    egui::RichText::new(&entry.event_type)
                                        .color(type_color)
                                        .monospace(),
                                );
                                ui.add_space(8.0);
                                ui.label(
                                    egui::RichText::new(&entry.data)
                                        .monospace()
                                        .color(egui::Color32::from_rgb(200, 200, 200)),
                                );
                            });
                        }
                    });
            });

        self.event_monitor_open = open;
    }

    /// Push an event into the monitor log (called by subsystems during play mode).
    pub(crate) fn log_event(&mut self, event_type: impl Into<String>, data: impl Into<String>) {
        // Use a monotonic counter based on event log length as a simple timestamp.
        let timestamp = self.event_log.len() as f64 * 0.016;
        self.event_log
            .push(EventEntry::new(timestamp, event_type, data));

        // Cap the log to prevent unbounded growth.
        const MAX_EVENTS: usize = 10_000;
        if self.event_log.len() > MAX_EVENTS {
            let drain_count = self.event_log.len() - MAX_EVENTS;
            self.event_log.drain(..drain_count);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_entry_creation() {
        let entry = EventEntry::new(1.234, "Physics", "RigidBody woke up: entity 42");
        assert!((entry.timestamp - 1.234).abs() < f64::EPSILON);
        assert_eq!(entry.event_type, "Physics");
        assert!(entry.data.contains("42"));
    }
}
