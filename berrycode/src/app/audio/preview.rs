//! Audio Preview panel — waveform display + click-to-scrub.
//!
//! v0.6 Phase A. Wires the `decode_wav_peaks` output into an egui
//! widget that shows the waveform, plus a horizontal play-head the
//! user can click or drag to set a position. Playback itself isn't
//! triggered from here yet (Phase A scope is *visual* preview);
//! a `Play` button + Bevy-side `AudioSource` spawn arrive in a
//! follow-up so the runtime hook can be designed alongside the
//! event editor in Phase C.

use std::path::PathBuf;

use super::decode::{decode_wav_peaks, DecodedPeaks};

/// Per-app state for the audio preview panel. Stored on
/// `BerryCodeApp`; cleared when the user closes the panel or
/// switches to a non-audio file.
#[derive(Debug, Default)]
pub struct AudioPreviewState {
    /// Path of the audio file the panel is currently rendering.
    /// `None` when no audio is loaded.
    pub loaded_path: Option<PathBuf>,
    /// Decode result. `None` while a decode is in flight or after
    /// a decode error (the `error` field carries the message).
    pub peaks: Option<DecodedPeaks>,
    /// Last decode error, if any. Surfaces in the preview area.
    pub error: Option<String>,
    /// Play-head position in seconds. Driven by user clicks; the
    /// runtime playback path will read this when Phase A grows the
    /// Play button.
    pub playhead_secs: f32,
    /// How many bars the waveform is rendered with. Default 1200 —
    /// enough for a typical panel width without wasting decode time.
    pub bucket_count: usize,
}

impl AudioPreviewState {
    pub fn new() -> Self {
        Self {
            bucket_count: 1200,
            ..Default::default()
        }
    }

    /// Decode `path` synchronously and replace the previous peaks
    /// with the result. Synchronous is fine for the WAV-only MVP —
    /// even a 5-minute clip decodes in well under a frame budget at
    /// 44.1 kHz on modern hardware. Async dispatch becomes worth it
    /// once symphonia (mp3 / ogg / flac) lands in v0.6.1.
    pub fn open(&mut self, path: PathBuf) {
        self.error = None;
        self.peaks = None;
        self.playhead_secs = 0.0;
        match decode_wav_peaks(&path, self.bucket_count) {
            Ok(decoded) => {
                self.peaks = Some(decoded);
                self.loaded_path = Some(path);
            }
            Err(e) => {
                self.error = Some(e);
                self.loaded_path = Some(path);
            }
        }
    }

    pub fn close(&mut self) {
        *self = Self::new();
    }
}

impl crate::app::BerryCodeApp {
    /// Render the audio preview as a Central-panel-style view. The
    /// caller owns the surrounding `ScrollArea` / panel chrome — we
    /// just claim a vertical strip and draw the waveform + scrubber.
    pub(crate) fn render_audio_preview(&mut self, ui: &mut egui::Ui) {
        ui.heading("Audio Preview");
        ui.add_space(4.0);

        // Snapshot read-only fields up front so we can mutate
        // `self.audio_preview.playhead_secs` later in the same method
        // without tripping the borrow checker. The peak vec gets
        // cloned because the waveform render reads it after the
        // mutation point — cheap (a few thousand `f32` pairs at most)
        // and the alternative is restructuring into two passes that's
        // harder to read.
        let path_label = self.audio_preview.loaded_path.as_ref().map(|p| {
            (
                p.file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default(),
                self.audio_preview
                    .peaks
                    .as_ref()
                    .map(|d| (d.sample_rate, d.channels, d.duration_secs)),
            )
        });
        let error_msg = self.audio_preview.error.clone();
        let decoded_clone = self.audio_preview.peaks.clone();

        match path_label {
            Some((fname, meta)) => {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(fname).size(13.0).strong());
                    if let Some((sr, ch, dur)) = meta {
                        ui.label(
                            egui::RichText::new(format!("{} Hz · {} ch · {:.2}s", sr, ch, dur))
                                .size(11.0)
                                .color(egui::Color32::from_gray(150)),
                        );
                    }
                });
            }
            None => {
                ui.label(
                    egui::RichText::new("No audio loaded — open a .wav from the file tree.")
                        .color(egui::Color32::from_gray(140)),
                );
                return;
            }
        }

        if let Some(err) = error_msg {
            ui.colored_label(
                egui::Color32::from_rgb(220, 120, 120),
                format!("Decode failed: {err}"),
            );
            return;
        }

        let Some(decoded) = decoded_clone else {
            ui.label("Decoding…");
            return;
        };
        let decoded = &decoded;

        // Reserve a strip of about 160px tall for the waveform.
        let available_w = ui.available_width();
        let height = 160.0_f32;
        let (response, painter) = ui.allocate_painter(
            egui::vec2(available_w, height),
            egui::Sense::click_and_drag(),
        );
        let rect = response.rect;
        let mid_y = rect.center().y;

        // Background.
        painter.rect_filled(
            rect,
            egui::CornerRadius::same(2),
            egui::Color32::from_rgb(28, 30, 36),
        );

        // Centre line for visual reference (zero amplitude).
        painter.line_segment(
            [
                egui::pos2(rect.left(), mid_y),
                egui::pos2(rect.right(), mid_y),
            ],
            egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 64, 72)),
        );

        // Waveform bars. We always render the configured bucket count
        // even if the panel is wider than that — egui handles the
        // squish gracefully and the horizontal gap stays visually
        // consistent across panel sizes.
        let bar_color = egui::Color32::from_rgb(120, 180, 220);
        let n = decoded.peaks.len().max(1);
        let bar_w = rect.width() / n as f32;
        let half_h = (rect.height() / 2.0) - 4.0;
        for (i, p) in decoded.peaks.iter().enumerate() {
            let x = rect.left() + (i as f32 + 0.5) * bar_w;
            let y_top = mid_y - p.max * half_h;
            let y_bot = mid_y - p.min * half_h;
            painter.line_segment(
                [egui::pos2(x, y_top), egui::pos2(x, y_bot)],
                egui::Stroke::new(bar_w.max(1.0), bar_color),
            );
        }

        // Click / drag → set play head. Convert pointer X within the
        // strip to a fraction of duration_secs.
        if response.dragged() || response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let frac = ((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
                self.audio_preview.playhead_secs = frac * decoded.duration_secs;
            }
        }

        // Play-head line.
        let frac = if decoded.duration_secs > 0.0 {
            (self.audio_preview.playhead_secs / decoded.duration_secs).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let head_x = rect.left() + frac * rect.width();
        painter.line_segment(
            [
                egui::pos2(head_x, rect.top() + 2.0),
                egui::pos2(head_x, rect.bottom() - 2.0),
            ],
            egui::Stroke::new(1.5, egui::Color32::from_rgb(255, 200, 80)),
        );

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!(
                    "{:.2}s / {:.2}s",
                    self.audio_preview.playhead_secs, decoded.duration_secs
                ))
                .size(11.0)
                .color(egui::Color32::from_gray(150)),
            );
            if ui.button("Reset to start").clicked() {
                self.audio_preview.playhead_secs = 0.0;
            }
        });

        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(
                "Playback button arrives in a follow-up — Phase A scope here is visual scrub.",
            )
            .size(10.5)
            .color(egui::Color32::from_gray(120))
            .italics(),
        );
    }
}
