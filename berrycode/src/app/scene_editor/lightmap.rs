//! Lightmap bake settings, reflection probes, and GPU profiler table.

use serde::{Deserialize, Serialize};

use crate::app::BerryCodeApp;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightmapSettings {
    pub resolution: u32,
    pub samples: u32,
    pub bounce_count: u32,
    pub ambient_color: [f32; 3],
    pub ambient_intensity: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionProbe {
    pub position: [f32; 3],
    pub radius: f32,
    pub resolution: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuProfileEntry {
    pub name: String,
    pub duration_ms: f64,
    pub memory_mb: f64,
}

pub struct LightingProfilerState {
    pub open: bool,
    pub lightmap_settings: LightmapSettings,
    pub probes: Vec<ReflectionProbe>,
    pub gpu_entries: Vec<GpuProfileEntry>,
    pub memory_usage_mb: f64,
}

impl Default for LightingProfilerState {
    fn default() -> Self {
        Self {
            open: false,
            lightmap_settings: LightmapSettings {
                resolution: 1024,
                samples: 128,
                bounce_count: 2,
                ambient_color: [0.1, 0.1, 0.15],
                ambient_intensity: 0.5,
            },
            probes: Vec::new(),
            gpu_entries: Vec::new(),
            memory_usage_mb: 0.0,
        }
    }
}

impl LightingProfilerState {
    pub fn add_probe(&mut self, position: [f32; 3], radius: f32, resolution: u32) {
        self.probes.push(ReflectionProbe {
            position,
            radius,
            resolution,
        });
    }

    pub fn remove_probe(&mut self, idx: usize) {
        if idx < self.probes.len() {
            self.probes.remove(idx);
        }
    }

    pub fn total_gpu_time_ms(&self) -> f64 {
        self.gpu_entries.iter().map(|e| e.duration_ms).sum()
    }

    pub fn total_gpu_memory_mb(&self) -> f64 {
        self.gpu_entries.iter().map(|e| e.memory_mb).sum()
    }
}

impl BerryCodeApp {
    pub(crate) fn render_lighting_profiler(&mut self, ctx: &egui::Context) {
        if !self.lighting_profiler.open {
            return;
        }
        let mut open = self.lighting_profiler.open;
        egui::Window::new("Lighting & GPU Profiler")
            .open(&mut open)
            .default_size([500.0, 400.0])
            .show(ctx, |ui| {
                ui.heading("Lightmap Bake Settings");
                let ls = &mut self.lighting_profiler.lightmap_settings;
                ui.horizontal(|ui| {
                    ui.label("Resolution:");
                    ui.add(egui::DragValue::new(&mut ls.resolution).range(64..=4096));
                });
                ui.horizontal(|ui| {
                    ui.label("Samples:");
                    ui.add(egui::DragValue::new(&mut ls.samples).range(1..=4096));
                });
                ui.horizontal(|ui| {
                    ui.label("Bounces:");
                    ui.add(egui::DragValue::new(&mut ls.bounce_count).range(0..=16));
                });
                ui.add(egui::Slider::new(&mut ls.ambient_intensity, 0.0..=2.0).text("Ambient"));
                if ui.button("Bake (placeholder)").clicked() {
                    self.status_message = "Lightmap bake not yet implemented".into();
                    self.status_message_timestamp = Some(std::time::Instant::now());
                }
                ui.separator();
                ui.heading("Reflection Probes");
                ui.label(format!("{} probes", self.lighting_profiler.probes.len()));
                if ui.small_button("+ Probe").clicked() {
                    self.lighting_profiler.add_probe([0.0, 1.0, 0.0], 10.0, 256);
                }
                let mut remove_idx = None;
                for (i, p) in self.lighting_profiler.probes.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(format!(
                            "Probe {} [{:.1},{:.1},{:.1}] r={:.1}",
                            i, p.position[0], p.position[1], p.position[2], p.radius
                        ));
                        if ui.small_button("x").clicked() {
                            remove_idx = Some(i);
                        }
                    });
                }
                if let Some(idx) = remove_idx {
                    self.lighting_profiler.remove_probe(idx);
                }
                ui.separator();
                ui.heading("GPU Profiler");
                ui.label(format!(
                    "Total: {:.2}ms | Memory: {:.1}MB",
                    self.lighting_profiler.total_gpu_time_ms(),
                    self.lighting_profiler.total_gpu_memory_mb()
                ));
                egui::Grid::new("gpu_profiler_grid")
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("Pass");
                        ui.label("Time (ms)");
                        ui.label("Mem (MB)");
                        ui.end_row();
                        for entry in &self.lighting_profiler.gpu_entries {
                            ui.label(&entry.name);
                            ui.label(format!("{:.2}", entry.duration_ms));
                            ui.label(format!("{:.1}", entry.memory_mb));
                            ui.end_row();
                        }
                    });
            });
        self.lighting_profiler.open = open;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings() {
        let s = LightingProfilerState::default();
        assert_eq!(s.lightmap_settings.resolution, 1024);
        assert_eq!(s.lightmap_settings.bounce_count, 2);
        assert!(s.probes.is_empty());
    }

    #[test]
    fn add_and_remove_probe() {
        let mut s = LightingProfilerState::default();
        s.add_probe([1.0, 2.0, 3.0], 5.0, 512);
        assert_eq!(s.probes.len(), 1);
        assert_eq!(s.probes[0].radius, 5.0);
        s.remove_probe(0);
        assert!(s.probes.is_empty());
    }

    #[test]
    fn gpu_totals() {
        let mut s = LightingProfilerState::default();
        s.gpu_entries.push(GpuProfileEntry {
            name: "Shadow".into(),
            duration_ms: 2.5,
            memory_mb: 64.0,
        });
        s.gpu_entries.push(GpuProfileEntry {
            name: "GBuffer".into(),
            duration_ms: 1.5,
            memory_mb: 32.0,
        });
        assert!((s.total_gpu_time_ms() - 4.0).abs() < f64::EPSILON);
        assert!((s.total_gpu_memory_mb() - 96.0).abs() < f64::EPSILON);
    }

    #[test]
    fn remove_probe_out_of_bounds() {
        let mut s = LightingProfilerState::default();
        s.remove_probe(5); // should be a no-op
        assert!(s.probes.is_empty());
    }

    #[test]
    fn lightmap_serialization() {
        let ls = LightmapSettings {
            resolution: 512,
            samples: 64,
            bounce_count: 1,
            ambient_color: [0.1, 0.2, 0.3],
            ambient_intensity: 0.8,
        };
        let json = serde_json::to_string(&ls).unwrap();
        let parsed: LightmapSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.resolution, 512);
        assert_eq!(parsed.bounce_count, 1);
    }
}
