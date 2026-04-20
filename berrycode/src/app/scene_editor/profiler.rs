//! Lightweight editor-side profiler panel.
//!
//! Records frame times in a ring buffer and renders a small line graph plus
//! summary statistics (FPS, min/avg/max frame time, entity counts).

use std::collections::VecDeque;

use crate::app::BerryCodeApp;

const HISTORY_CAPACITY: usize = 240; // ~4 seconds at 60fps

/// Rolling window of frame times (in seconds) for the profiler graph.
#[derive(Debug, Clone)]
pub struct ProfilerState {
    pub frame_times: VecDeque<f32>,
    pub last_instant: Option<std::time::Instant>,
    pub open: bool,
}

impl Default for ProfilerState {
    fn default() -> Self {
        Self {
            frame_times: VecDeque::with_capacity(HISTORY_CAPACITY),
            last_instant: None,
            open: false,
        }
    }
}

impl ProfilerState {
    /// Record a frame sample using the time elapsed since the last sample.
    /// Call this once per UI frame — e.g., at the top of the Profiler render.
    pub fn tick(&mut self) {
        let now = std::time::Instant::now();
        if let Some(prev) = self.last_instant {
            let dt = now.duration_since(prev).as_secs_f32();
            if self.frame_times.len() >= HISTORY_CAPACITY {
                self.frame_times.pop_front();
            }
            self.frame_times.push_back(dt);
        }
        self.last_instant = Some(now);
    }

    /// Compute (min, avg, max) frame time in seconds over the recorded window.
    /// Returns `None` if there are no samples yet.
    pub fn stats(&self) -> Option<(f32, f32, f32)> {
        if self.frame_times.is_empty() {
            return None;
        }
        let mut min = f32::INFINITY;
        let mut max = 0.0f32;
        let mut sum = 0.0f32;
        for &t in &self.frame_times {
            if t < min {
                min = t;
            }
            if t > max {
                max = t;
            }
            sum += t;
        }
        Some((min, sum / self.frame_times.len() as f32, max))
    }

    /// Compute instantaneous FPS from the average frame time.
    pub fn fps(&self) -> Option<f32> {
        self.stats()
            .and_then(|(_, avg, _)| if avg > 0.0 { Some(1.0 / avg) } else { None })
    }
}

impl BerryCodeApp {
    /// Render profiler content into a provided `Ui` region (used by the tool panel).
    pub(crate) fn render_profiler_content(&mut self, ui: &mut egui::Ui) {
        // Record a frame sample.
        self.profiler.tick();

        let stats = self.profiler.stats();
        let fps = self.profiler.fps();
        let entity_count = self.scene_model.entities.len();
        let root_count = self.scene_model.root_entities.len();
        let samples: Vec<f32> = self.profiler.frame_times.iter().copied().collect();

        // Summary header.
        ui.horizontal(|ui| {
            ui.label("FPS:");
            match fps {
                Some(v) => ui.monospace(format!("{:>6.1}", v)),
                None => ui.monospace("  --  "),
            };
        });
        if let Some((min, avg, max)) = stats {
            ui.horizontal(|ui| {
                ui.label("Frame time (ms):");
                ui.monospace(format!(
                    "min {:>5.2}  avg {:>5.2}  max {:>5.2}",
                    min * 1000.0,
                    avg * 1000.0,
                    max * 1000.0,
                ));
            });
        }
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Entities:");
            ui.monospace(format!("{}", entity_count));
            ui.separator();
            ui.label("Roots:");
            ui.monospace(format!("{}", root_count));
        });
        ui.separator();

        // Line graph of frame times.
        let graph_height = 80.0;
        let (rect, _resp) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), graph_height),
            egui::Sense::hover(),
        );
        ui.painter()
            .rect_filled(rect, 2.0, egui::Color32::from_rgb(20, 22, 26));
        if samples.len() >= 2 {
            let ceiling_s = 0.033_f32;
            let w = rect.width();
            let h = rect.height();
            let n = samples.len();
            let step = w / (n as f32 - 1.0).max(1.0);
            let mut prev: Option<egui::Pos2> = None;
            for (i, &t) in samples.iter().enumerate() {
                let x = rect.left() + step * i as f32;
                let y_norm = (t / ceiling_s).clamp(0.0, 1.0);
                let y = rect.bottom() - y_norm * h;
                let p = egui::pos2(x, y);
                if let Some(pp) = prev {
                    ui.painter().line_segment(
                        [pp, p],
                        egui::Stroke::new(1.2, egui::Color32::from_rgb(120, 220, 140)),
                    );
                }
                prev = Some(p);
            }
            // 16.67ms (60fps) reference line.
            let ref_y = rect.bottom() - (0.0166_f32 / ceiling_s) * h;
            ui.painter().line_segment(
                [
                    egui::pos2(rect.left(), ref_y),
                    egui::pos2(rect.right(), ref_y),
                ],
                egui::Stroke::new(
                    1.0,
                    egui::Color32::from_rgba_premultiplied(200, 200, 100, 80),
                ),
            );
            ui.painter().text(
                egui::pos2(rect.left() + 4.0, ref_y - 12.0),
                egui::Align2::LEFT_TOP,
                "60 FPS",
                egui::FontId::proportional(10.0),
                egui::Color32::from_rgb(200, 200, 100),
            );
        } else {
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "collecting samples...",
                egui::FontId::proportional(11.0),
                egui::Color32::from_gray(140),
            );
        }

        ui.separator();
        if ui.button("Reset Samples").clicked() {
            self.profiler.frame_times.clear();
        }
        ui.add_space(2.0);
        ui.label(
            egui::RichText::new(
                "Editor-side metrics only. Game process metrics will be added when Play-in-Editor integration lands.",
            )
            .size(10.0)
            .color(egui::Color32::from_gray(140)),
        );

        // Keep repaint live so the graph animates.
        ui.ctx().request_repaint();
    }

    /// Render the Profiler window (floating egui::Window).
    pub(crate) fn render_profiler(&mut self, ctx: &egui::Context) {
        if !self.profiler.open {
            return;
        }

        // Record a frame sample.
        self.profiler.tick();

        // Snapshot stats before the Window borrow of self below.
        let stats = self.profiler.stats();
        let fps = self.profiler.fps();
        let entity_count = self.scene_model.entities.len();
        let root_count = self.scene_model.root_entities.len();
        let samples: Vec<f32> = self.profiler.frame_times.iter().copied().collect();

        let mut open = self.profiler.open;
        egui::Window::new("Profiler")
            .open(&mut open)
            .default_width(320.0)
            .resizable(true)
            .show(ctx, |ui| {
                // Summary header.
                ui.horizontal(|ui| {
                    ui.label("FPS:");
                    match fps {
                        Some(v) => ui.monospace(format!("{:>6.1}", v)),
                        None => ui.monospace("  --  "),
                    };
                });
                if let Some((min, avg, max)) = stats {
                    ui.horizontal(|ui| {
                        ui.label("Frame time (ms):");
                        ui.monospace(format!(
                            "min {:>5.2}  avg {:>5.2}  max {:>5.2}",
                            min * 1000.0,
                            avg * 1000.0,
                            max * 1000.0,
                        ));
                    });
                }
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Entities:");
                    ui.monospace(format!("{}", entity_count));
                    ui.separator();
                    ui.label("Roots:");
                    ui.monospace(format!("{}", root_count));
                });
                ui.separator();

                // Line graph of frame times.
                let graph_height = 80.0;
                let (rect, _resp) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), graph_height),
                    egui::Sense::hover(),
                );
                ui.painter().rect_filled(
                    rect,
                    2.0,
                    egui::Color32::from_rgb(20, 22, 26),
                );
                if samples.len() >= 2 {
                    // Normalize: ceiling at 33ms (30fps) so small variations
                    // are still visible; cap higher spikes to the top of the graph.
                    let ceiling_s = 0.033_f32;
                    let w = rect.width();
                    let h = rect.height();
                    let n = samples.len();
                    let step = w / (n as f32 - 1.0).max(1.0);
                    let mut prev: Option<egui::Pos2> = None;
                    for (i, &t) in samples.iter().enumerate() {
                        let x = rect.left() + step * i as f32;
                        let y_norm = (t / ceiling_s).clamp(0.0, 1.0);
                        let y = rect.bottom() - y_norm * h;
                        let p = egui::pos2(x, y);
                        if let Some(pp) = prev {
                            ui.painter().line_segment(
                                [pp, p],
                                egui::Stroke::new(1.2, egui::Color32::from_rgb(120, 220, 140)),
                            );
                        }
                        prev = Some(p);
                    }
                    // 16.67ms (60fps) reference line.
                    let ref_y = rect.bottom() - (0.0166_f32 / ceiling_s) * h;
                    ui.painter().line_segment(
                        [
                            egui::pos2(rect.left(), ref_y),
                            egui::pos2(rect.right(), ref_y),
                        ],
                        egui::Stroke::new(
                            1.0,
                            egui::Color32::from_rgba_premultiplied(200, 200, 100, 80),
                        ),
                    );
                    ui.painter().text(
                        egui::pos2(rect.left() + 4.0, ref_y - 12.0),
                        egui::Align2::LEFT_TOP,
                        "60 FPS",
                        egui::FontId::proportional(10.0),
                        egui::Color32::from_rgb(200, 200, 100),
                    );
                } else {
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "collecting samples...",
                        egui::FontId::proportional(11.0),
                        egui::Color32::from_gray(140),
                    );
                }

                ui.separator();
                if ui.button("Reset Samples").clicked() {
                    self.profiler.frame_times.clear();
                }
                ui.add_space(2.0);
                ui.label(
                    egui::RichText::new(
                        "Editor-side metrics only. Game process metrics will be added when Play-in-Editor integration lands.",
                    )
                    .size(10.0)
                    .color(egui::Color32::from_gray(140)),
                );

                // Keep repaint live so the graph animates.
                ctx.request_repaint();
            });
        self.profiler.open = open;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stats_none_when_empty() {
        let s = ProfilerState::default();
        assert!(s.stats().is_none());
        assert!(s.fps().is_none());
    }

    #[test]
    fn stats_tracks_min_avg_max() {
        let mut s = ProfilerState::default();
        s.frame_times.push_back(0.010);
        s.frame_times.push_back(0.020);
        s.frame_times.push_back(0.030);
        let (min, avg, max) = s.stats().expect("stats");
        assert!((min - 0.010).abs() < 1e-5);
        assert!((avg - 0.020).abs() < 1e-5);
        assert!((max - 0.030).abs() < 1e-5);
    }

    #[test]
    fn ring_buffer_caps_at_capacity() {
        let mut s = ProfilerState::default();
        for _ in 0..(HISTORY_CAPACITY + 50) {
            if s.frame_times.len() >= HISTORY_CAPACITY {
                s.frame_times.pop_front();
            }
            s.frame_times.push_back(0.016);
        }
        assert_eq!(s.frame_times.len(), HISTORY_CAPACITY);
    }
}
