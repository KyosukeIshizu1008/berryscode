//! `AudioSource3D` — placeable 3D audio source for the Scene Editor.
//!
//! v0.6 Phase E. Ships the data model + an inspector helper. The
//! Scene Editor renders a translucent sphere at the source's
//! position with radius = `max_distance` so spatial coverage is
//! visible without spawning the runtime sink. Bevy-side conversion
//! to `SpatialAudioSink` arrives in v0.6.1 alongside the event
//! editor's runtime hooks (Phase C).

use serde::{Deserialize, Serialize};

/// Falloff curve from "fully audible" (at the source) to silent (at
/// `max_distance`). Matches the names Bevy uses for its built-in
/// attenuation; the runtime swap is a one-line conversion.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AttenuationCurve {
    /// Volume drops with `1 / distance` past `min_distance`.
    Linear,
    /// Inverse-square falloff (closer to physical sound propagation).
    InverseSquare,
    /// Logarithmic curve — gentle near the source, steep near max.
    Logarithmic,
    /// No falloff; the source is heard at the same volume regardless
    /// of distance, useful for narration or UI sounds attached to
    /// world entities.
    Constant,
}

impl Default for AttenuationCurve {
    fn default() -> Self {
        Self::InverseSquare
    }
}

/// Component placed on a Bevy entity to mark it as a 3D audio
/// source. Drives the editor visualisation today; consumed by a
/// `convert_to_spatial_sink` runtime system in v0.6.1.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioSource3D {
    /// Asset path the source plays. Relative to `assets/`. The
    /// runtime hands this to `asset_server.load(path)`.
    pub asset_path: String,
    /// Distance below which the source plays at full volume — the
    /// "centre sphere" inside which there's no falloff yet.
    pub min_distance: f32,
    /// Distance at which the source becomes inaudible. The Scene
    /// Editor's translucent sphere uses this radius.
    pub max_distance: f32,
    pub attenuation: AttenuationCurve,
    /// Doppler shift multiplier — `0.0` disables the effect, `1.0`
    /// is physically realistic.
    #[serde(default = "default_doppler")]
    pub doppler_factor: f32,
    /// Loop playback. `false` = play once on enable.
    #[serde(default)]
    pub looped: bool,
    /// Per-source gain. Composes with whatever volume the runtime
    /// computes from the attenuation curve.
    #[serde(default = "default_volume")]
    pub volume: f32,
}

fn default_doppler() -> f32 {
    1.0
}
fn default_volume() -> f32 {
    1.0
}

impl Default for AudioSource3D {
    fn default() -> Self {
        Self {
            asset_path: String::new(),
            min_distance: 1.0,
            max_distance: 25.0,
            attenuation: AttenuationCurve::default(),
            doppler_factor: 1.0,
            looped: false,
            volume: 1.0,
        }
    }
}

/// Component inspector. Same shape as the SFX randomiser inspector
/// (a free function taking `&mut`) so callers can drop it next to
/// any other component editor without going through `BerryCodeApp`.
pub fn render_audio_source_3d_inspector(ui: &mut egui::Ui, source: &mut AudioSource3D) {
    ui.label(egui::RichText::new("Audio Source 3D").strong().size(13.0));
    ui.add_space(4.0);

    ui.horizontal(|ui| {
        ui.label("Asset:");
        ui.add(
            egui::TextEdit::singleline(&mut source.asset_path)
                .hint_text("audio/ambient/forest.wav")
                .desired_width(220.0),
        );
    });

    ui.horizontal(|ui| {
        ui.label("Min distance:");
        ui.add(
            egui::DragValue::new(&mut source.min_distance)
                .speed(0.1)
                .range(0.0..=10000.0)
                .suffix(" m"),
        );
        ui.label("Max:");
        ui.add(
            egui::DragValue::new(&mut source.max_distance)
                .speed(0.5)
                .range(0.0..=10000.0)
                .suffix(" m"),
        );
    });
    if source.min_distance > source.max_distance {
        // Keep the invariant the runtime depends on.
        source.max_distance = source.min_distance;
    }

    ui.horizontal(|ui| {
        ui.label("Attenuation:");
        egui::ComboBox::from_id_salt("audio_attenuation")
            .selected_text(match source.attenuation {
                AttenuationCurve::Linear => "Linear",
                AttenuationCurve::InverseSquare => "Inverse Square",
                AttenuationCurve::Logarithmic => "Logarithmic",
                AttenuationCurve::Constant => "Constant",
            })
            .show_ui(ui, |ui| {
                for (variant, label) in [
                    (AttenuationCurve::Linear, "Linear"),
                    (AttenuationCurve::InverseSquare, "Inverse Square"),
                    (AttenuationCurve::Logarithmic, "Logarithmic"),
                    (AttenuationCurve::Constant, "Constant"),
                ] {
                    ui.selectable_value(&mut source.attenuation, variant, label);
                }
            });
    });

    ui.horizontal(|ui| {
        ui.label("Doppler:");
        ui.add(
            egui::DragValue::new(&mut source.doppler_factor)
                .speed(0.05)
                .range(0.0..=4.0),
        );
        ui.checkbox(&mut source.looped, "Loop");
    });

    ui.horizontal(|ui| {
        ui.label("Volume:");
        ui.add(egui::Slider::new(&mut source.volume, 0.0..=2.0).text("× gain"));
    });

    ui.add_space(4.0);
    ui.label(
        egui::RichText::new(
            "Scene Editor renders a translucent sphere at max_distance; \
             SpatialAudioSink runtime conversion ships in v0.6.1.",
        )
        .size(10.5)
        .color(egui::Color32::from_gray(120))
        .italics(),
    );
}

/// Compute the attenuation factor for a given distance using the
/// curve. `0.0` = silent, `1.0` = full volume. Surfaced now so the
/// inspector preview ("at 5m: 0.42×") and the future runtime
/// system share a single source of truth.
pub fn attenuation_at(source: &AudioSource3D, distance_m: f32) -> f32 {
    if distance_m <= source.min_distance {
        return source.volume;
    }
    if distance_m >= source.max_distance {
        return 0.0;
    }
    let span = (source.max_distance - source.min_distance).max(0.0001);
    let t = (distance_m - source.min_distance) / span;
    let curve = match source.attenuation {
        AttenuationCurve::Constant => 1.0,
        AttenuationCurve::Linear => 1.0 - t,
        AttenuationCurve::InverseSquare => {
            // Map t ∈ [0,1] onto distance multiple ∈ [1,N] with N
            // chosen so the curve hits ~0 near max. N = 4 is a nice
            // game-feel default; tweakable later.
            let n = 4.0;
            let d = 1.0 + t * (n - 1.0);
            (1.0 / (d * d)).max(0.0)
        }
        AttenuationCurve::Logarithmic => {
            // Steeper near the end, gentle near the start.
            (1.0 - t * t).max(0.0)
        }
    };
    source.volume * curve
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attenuation_inside_min_is_full() {
        let s = AudioSource3D {
            min_distance: 2.0,
            max_distance: 10.0,
            volume: 0.8,
            ..Default::default()
        };
        assert!((attenuation_at(&s, 0.0) - 0.8).abs() < 1e-6);
        assert!((attenuation_at(&s, 1.0) - 0.8).abs() < 1e-6);
    }

    #[test]
    fn attenuation_outside_max_is_zero() {
        let s = AudioSource3D {
            min_distance: 2.0,
            max_distance: 10.0,
            volume: 1.0,
            ..Default::default()
        };
        assert_eq!(attenuation_at(&s, 10.0), 0.0);
        assert_eq!(attenuation_at(&s, 100.0), 0.0);
    }

    #[test]
    fn linear_decays_linearly() {
        let s = AudioSource3D {
            min_distance: 0.0,
            max_distance: 10.0,
            attenuation: AttenuationCurve::Linear,
            volume: 1.0,
            ..Default::default()
        };
        // Halfway should be 0.5, allowing for a tiny tolerance.
        assert!((attenuation_at(&s, 5.0) - 0.5).abs() < 1e-6);
    }
}
