//! SfxRandomiser — pick one of N variations on play, with pitch /
//! volume jitter so repeated triggers don't sound identical.
//!
//! v0.6 Phase B. Ships the data model + an inspector helper that
//! the Scene Editor (and future event editor in Phase C) can call
//! to render the editing UI. Runtime hooks (`fire(name)` →
//! `AudioSource` spawn with jitter) live alongside `bevy_plugin.rs`
//! and arrive in v0.6.1.

use serde::{Deserialize, Serialize};

/// One audible variation in a randomiser. `weight` is relative;
/// the runtime normalises across siblings so the absolute scale
/// doesn't matter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SfxVariation {
    /// Asset path relative to `assets/`, e.g. `"sfx/footstep_01.wav"`.
    pub path: String,
    /// Relative selection weight. Must be > 0; a row with weight
    /// `0.0` is equivalent to muted/disabled.
    pub weight: f32,
}

impl Default for SfxVariation {
    fn default() -> Self {
        Self {
            path: String::new(),
            weight: 1.0,
        }
    }
}

/// Inclusive-inclusive range used for the pitch and volume jitter.
/// Stored as a tuple of (min, max) to avoid an `serde` rename
/// ceremony — `(f32, f32)` is the natural choice and `serde_json`
/// already round-trips it as a 2-element array.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct JitterRange {
    pub min: f32,
    pub max: f32,
}

impl JitterRange {
    pub const PITCH_DEFAULT: Self = Self {
        min: 0.95,
        max: 1.05,
    };
    pub const VOLUME_DEFAULT: Self = Self {
        min: 0.85,
        max: 1.0,
    };
}

/// Component / asset payload. Lives next to other Bevy components
/// in the scene's `.bscene`; the runtime plugin reads it on `Play`
/// events to pick a variation and apply jitter to the spawned
/// `AudioSource`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SfxRandomiser {
    pub variations: Vec<SfxVariation>,
    /// Pitch multiplier range — `1.0` = original speed.
    #[serde(default = "default_pitch_range")]
    pub pitch_range: JitterRange,
    /// Volume multiplier range — `1.0` = original gain.
    #[serde(default = "default_volume_range")]
    pub volume_range: JitterRange,
    /// Optional fixed seed for deterministic playback (testing,
    /// replays). `None` uses a fresh `rand::thread_rng()` per fire.
    #[serde(default)]
    pub seed: Option<u64>,
}

fn default_pitch_range() -> JitterRange {
    JitterRange::PITCH_DEFAULT
}
fn default_volume_range() -> JitterRange {
    JitterRange::VOLUME_DEFAULT
}

impl Default for SfxRandomiser {
    fn default() -> Self {
        Self {
            variations: Vec::new(),
            pitch_range: JitterRange::PITCH_DEFAULT,
            volume_range: JitterRange::VOLUME_DEFAULT,
            seed: None,
        }
    }
}

/// Pure-function inspector helper. Called from the Scene Editor's
/// component inspector with a `&mut SfxRandomiser`; mutates in
/// place. Kept on its own struct (no `BerryCodeApp`) so the same
/// editor can run from a unit test or a future inspector
/// surface without dragging the whole app along.
pub fn render_sfx_inspector(ui: &mut egui::Ui, sfx: &mut SfxRandomiser) {
    ui.label(egui::RichText::new("SFX Randomiser").strong().size(13.0));
    ui.add_space(4.0);

    // Variations table.
    let mut remove: Option<usize> = None;
    egui::Grid::new("sfx_variations")
        .striped(true)
        .num_columns(3)
        .show(ui, |ui| {
            ui.label(egui::RichText::new("Asset path").strong());
            ui.label(egui::RichText::new("Weight").strong());
            ui.label("");
            ui.end_row();
            for (i, v) in sfx.variations.iter_mut().enumerate() {
                ui.add(
                    egui::TextEdit::singleline(&mut v.path)
                        .desired_width(220.0)
                        .hint_text("sfx/footstep_01.wav"),
                );
                ui.add(
                    egui::DragValue::new(&mut v.weight)
                        .speed(0.1)
                        .range(0.0..=100.0),
                );
                if ui.button("✕").clicked() {
                    remove = Some(i);
                }
                ui.end_row();
            }
        });
    if let Some(i) = remove {
        sfx.variations.remove(i);
    }
    if ui.button("+ Variation").clicked() {
        sfx.variations.push(SfxVariation::default());
    }

    ui.add_space(6.0);
    ui.separator();
    ui.add_space(4.0);

    // Jitter ranges. DragValue speed kept low so the user can
    // dial in subtle multipliers like 0.97/1.03 without flicking
    // past the target.
    ui.horizontal(|ui| {
        ui.label("Pitch:");
        ui.add(
            egui::DragValue::new(&mut sfx.pitch_range.min)
                .speed(0.01)
                .range(0.1..=4.0)
                .prefix("min "),
        );
        ui.add(
            egui::DragValue::new(&mut sfx.pitch_range.max)
                .speed(0.01)
                .range(0.1..=4.0)
                .prefix("max "),
        );
    });
    ui.horizontal(|ui| {
        ui.label("Volume:");
        ui.add(
            egui::DragValue::new(&mut sfx.volume_range.min)
                .speed(0.01)
                .range(0.0..=2.0)
                .prefix("min "),
        );
        ui.add(
            egui::DragValue::new(&mut sfx.volume_range.max)
                .speed(0.01)
                .range(0.0..=2.0)
                .prefix("max "),
        );
    });
    // Clamp invariants so min ≤ max even after manual edits.
    if sfx.pitch_range.min > sfx.pitch_range.max {
        sfx.pitch_range.max = sfx.pitch_range.min;
    }
    if sfx.volume_range.min > sfx.volume_range.max {
        sfx.volume_range.max = sfx.volume_range.min;
    }

    ui.add_space(4.0);
    ui.label(
        egui::RichText::new("Runtime hooks (fire on event, jittered playback) ship in v0.6.1.")
            .size(10.5)
            .color(egui::Color32::from_gray(120))
            .italics(),
    );
}

/// Pick one variation index based on the randomiser's weights. Used
/// by the runtime path once it lands; surfaced here so the inspector
/// can drive a "Test pick" button or unit tests can validate the
/// distribution.
pub fn pick_variation_index(sfx: &SfxRandomiser, rng_value: f32) -> Option<usize> {
    let total: f32 = sfx.variations.iter().map(|v| v.weight.max(0.0)).sum();
    if total <= 0.0 || sfx.variations.is_empty() {
        return None;
    }
    let target = rng_value.clamp(0.0, 1.0) * total;
    let mut acc = 0.0;
    for (i, v) in sfx.variations.iter().enumerate() {
        acc += v.weight.max(0.0);
        if target <= acc {
            return Some(i);
        }
    }
    Some(sfx.variations.len() - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pick_respects_weights() {
        let sfx = SfxRandomiser {
            variations: vec![
                SfxVariation {
                    path: "a".into(),
                    weight: 1.0,
                },
                SfxVariation {
                    path: "b".into(),
                    weight: 3.0,
                },
            ],
            ..Default::default()
        };
        // total = 4; target=0.5 → cum 1.0, falls into first
        assert_eq!(pick_variation_index(&sfx, 0.1), Some(0));
        // target=0.5 → 2.0, into second
        assert_eq!(pick_variation_index(&sfx, 0.5), Some(1));
        // 1.0 → end
        assert_eq!(pick_variation_index(&sfx, 1.0), Some(1));
    }

    #[test]
    fn pick_empty_returns_none() {
        let sfx = SfxRandomiser::default();
        assert_eq!(pick_variation_index(&sfx, 0.5), None);
    }

    #[test]
    fn pick_all_zero_returns_none() {
        let sfx = SfxRandomiser {
            variations: vec![SfxVariation {
                path: "x".into(),
                weight: 0.0,
            }],
            ..Default::default()
        };
        assert_eq!(pick_variation_index(&sfx, 0.5), None);
    }
}
