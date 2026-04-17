//! Editor-side animation playback.
//!
//! Keeps per-entity playback time so the Scene View can show animations live
//! while the user edits keyframes. This state is NOT serialized into .bscene —
//! it only exists while the editor is running.

use super::model::*;
use crate::app::BerryCodeApp;
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct AnimationPlayback {
    /// Per-entity current playback time in seconds.
    pub times: HashMap<u64, f32>,
    /// Whether playback is active. When false, times are frozen.
    pub playing: bool,
    /// Last tick timestamp.
    pub last_instant: Option<std::time::Instant>,
}

impl AnimationPlayback {
    /// Advance playback by real elapsed time since the last tick.
    pub fn tick(&mut self, scene: &SceneModel) {
        let now = std::time::Instant::now();
        let dt = match self.last_instant {
            Some(prev) => now.duration_since(prev).as_secs_f32().min(0.1),
            None => 0.0,
        };
        self.last_instant = Some(now);

        if !self.playing {
            return;
        }

        // Drop entries whose entity no longer exists.
        self.times.retain(|id, _| scene.entities.contains_key(id));

        // Advance each entity that has an Animation component.
        for (id, entity) in &scene.entities {
            for component in &entity.components {
                if let ComponentData::Animation { duration, looped, tracks, .. } = component {
                    let entry = self.times.entry(*id).or_insert(0.0);
                    let prev_time = *entry;
                    *entry += dt;
                    if *duration > 0.0 {
                        if *looped {
                            if *entry >= *duration {
                                *entry %= *duration;
                            }
                        } else {
                            *entry = entry.min(*duration);
                        }
                    }
                    let new_time = *entry;
                    // Check animation events that were crossed during this tick.
                    for track in tracks {
                        for evt in &track.events {
                            if prev_time <= evt.time && new_time > evt.time {
                                tracing::info!(
                                    "Animation event: {} at t={:.2}",
                                    evt.callback_name,
                                    evt.time
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    /// Reset all playback times to 0.
    pub fn rewind(&mut self) {
        for t in self.times.values_mut() {
            *t = 0.0;
        }
    }
}

// ---------------------------------------------------------------------------
// Easing functions (Phase 8 v2)
// ---------------------------------------------------------------------------

/// Evaluate an easing curve at parameter `t` in `[0, 1]`.
pub fn ease(easing: EasingType, t: f32) -> f32 {
    match easing {
        EasingType::Linear => t,
        EasingType::EaseInQuad => t * t,
        EasingType::EaseOutQuad => t * (2.0 - t),
        EasingType::EaseInOutQuad => {
            if t < 0.5 {
                2.0 * t * t
            } else {
                -1.0 + (4.0 - 2.0 * t) * t
            }
        }
        EasingType::EaseInCubic => t * t * t,
        EasingType::EaseOutCubic => {
            let t1 = t - 1.0;
            t1 * t1 * t1 + 1.0
        }
        EasingType::EaseInOutCubic => {
            if t < 0.5 {
                4.0 * t * t * t
            } else {
                let t1 = 2.0 * t - 2.0;
                0.5 * t1 * t1 * t1 + 1.0
            }
        }
        EasingType::EaseInOutSine => 0.5 * (1.0 - (std::f32::consts::PI * t).cos()),
    }
}

/// Sample a single animation track at time `t`, returning the interpolated
/// `[f32; 3]` value, or `None` if the track has no keyframes.
pub fn sample_track(track: &AnimationTrack, t: f32) -> Option<[f32; 3]> {
    if track.keyframes.is_empty() {
        return None;
    }
    if track.keyframes.len() == 1 || t <= track.keyframes[0].time {
        return Some(track.keyframes[0].value);
    }
    let last = &track.keyframes[track.keyframes.len() - 1];
    if t >= last.time {
        return Some(last.value);
    }
    for pair in track.keyframes.windows(2) {
        let a = &pair[0];
        let b = &pair[1];
        if t >= a.time && t <= b.time {
            let span = (b.time - a.time).max(1e-6);
            let u = ((t - a.time) / span).clamp(0.0, 1.0);
            let eased = ease(a.easing, u);
            return Some([
                a.value[0] + (b.value[0] - a.value[0]) * eased,
                a.value[1] + (b.value[1] - a.value[1]) * eased,
                a.value[2] + (b.value[2] - a.value[2]) * eased,
            ]);
        }
    }
    Some(last.value)
}

/// Sample all tracks at time `t` and compose the result onto `base` (the
/// entity's static local transform). Each track overrides the property it
/// drives; properties without a track keep their `base` value.
pub fn sample_animation_tracks(
    tracks: &[AnimationTrack],
    t: f32,
    base: &TransformData,
) -> TransformData {
    let mut result = base.clone();
    for track in tracks {
        if let Some(val) = sample_track(track, t) {
            match track.property {
                AnimProperty::Position => result.translation = val,
                AnimProperty::Rotation => result.rotation_euler = val,
                AnimProperty::Scale => result.scale = val,
            }
        }
    }
    result
}

impl BerryCodeApp {
    /// Render timeline content into a provided `Ui` region (used by the tool panel).
    pub(crate) fn render_timeline_content(&mut self, ui: &mut egui::Ui) {
        // Tick playback every frame.
        self.animation_playback.tick(&self.scene_model);

        // Find which selected entity has an Animation component.
        let selected_id = self.primary_selected_id;
        let (entity_id, duration, tracks_snapshot): (
            Option<u64>,
            f32,
            Vec<AnimationTrack>,
        ) = match selected_id.and_then(|id| self.scene_model.entities.get(&id).map(|e| (id, e))) {
            Some((id, e)) => {
                let anim = e.components.iter().find_map(|c| match c {
                    ComponentData::Animation { duration, tracks, .. } => {
                        Some((*duration, tracks.clone()))
                    }
                    _ => None,
                });
                match anim {
                    Some((d, ts)) => (Some(id), d, ts),
                    None => (None, 0.0, vec![]),
                }
            }
            None => (None, 0.0, vec![]),
        };

        let playback_time = entity_id
            .and_then(|id| self.animation_playback.times.get(&id).copied())
            .unwrap_or(0.0);

        if entity_id.is_none() {
            ui.label("Select an entity with an Animation component.");
            return;
        }

        let mut add_keyframe = false;
        let mut toggle_play = false;
        let mut rewind_requested = false;
        let mut scrub_request: Option<f32> = None;
        let mut delete_idx: Option<usize> = None;

        ui.horizontal(|ui| {
            if ui
                .button(if self.animation_playback.playing {
                    "Pause"
                } else {
                    "Play"
                })
                .clicked()
            {
                toggle_play = true;
            }
            if ui.button("Rewind").clicked() {
                rewind_requested = true;
            }
            if ui.button("+ Keyframe @ current time").clicked() {
                add_keyframe = true;
            }
            ui.separator();
            ui.label(format!("t = {:.2}s / {:.2}s", playback_time, duration));
        });

        ui.separator();

        // Scrubber slider.
        let mut t = playback_time;
        if ui
            .add(egui::Slider::new(&mut t, 0.0..=duration.max(0.001)).text("scrub"))
            .changed()
        {
            scrub_request = Some(t);
        }

        ui.separator();

        // Timeline bar: draw keyframe markers.
        let (rect, _resp) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), 32.0),
            egui::Sense::hover(),
        );
        ui.painter().rect_filled(
            rect,
            2.0,
            egui::Color32::from_rgb(25, 27, 31),
        );
        let d = duration.max(0.001);
        let track_colors = [
            egui::Color32::from_rgb(120, 200, 255),
            egui::Color32::from_rgb(200, 255, 120),
            egui::Color32::from_rgb(255, 160, 120),
        ];
        for track in &tracks_snapshot {
            let color = match track.property {
                AnimProperty::Position => track_colors[0],
                AnimProperty::Rotation => track_colors[1],
                AnimProperty::Scale => track_colors[2],
            };
            for kf in &track.keyframes {
                let x = rect.left() + (kf.time / d) * rect.width();
                let tip = egui::pos2(x, rect.top() + 4.0);
                let base_l = egui::pos2(x - 5.0, rect.bottom() - 4.0);
                let base_r = egui::pos2(x + 5.0, rect.bottom() - 4.0);
                ui.painter().add(egui::Shape::convex_polygon(
                    vec![tip, base_l, base_r],
                    color,
                    egui::Stroke::NONE,
                ));
            }
        }
        // Play head.
        let ph_x = rect.left() + (playback_time / d).clamp(0.0, 1.0) * rect.width();
        ui.painter().line_segment(
            [
                egui::pos2(ph_x, rect.top()),
                egui::pos2(ph_x, rect.bottom()),
            ],
            egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 180, 80)),
        );

        ui.separator();

        // Per-track keyframe list.
        egui::ScrollArea::vertical().max_height(160.0).show(ui, |ui| {
            for (t_idx, track) in tracks_snapshot.iter().enumerate() {
                ui.label(
                    egui::RichText::new(format!(
                        "{} ({} kf)",
                        track.property.label(),
                        track.keyframes.len()
                    ))
                    .strong(),
                );
                for (k_idx, kf) in track.keyframes.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.monospace(format!(
                            "  #{:>2}  t={:>6.2}s  [{:+.2},{:+.2},{:+.2}]  {}",
                            k_idx,
                            kf.time,
                            kf.value[0],
                            kf.value[1],
                            kf.value[2],
                            kf.easing.label(),
                        ));
                        if ui.small_button("Delete").clicked() {
                            delete_idx = Some(t_idx * 10000 + k_idx);
                        }
                    });
                }
            }
        });

        // Apply requests.
        if toggle_play {
            self.animation_playback.playing = !self.animation_playback.playing;
        }
        if rewind_requested {
            self.animation_playback.rewind();
        }
        if let Some(t) = scrub_request {
            if let Some(id) = entity_id {
                self.animation_playback.times.insert(id, t);
                self.animation_playback.playing = false;
            }
        }
        if let (Some(id), true) = (entity_id, add_keyframe) {
            self.scene_snapshot();
            if let Some(entity) = self.scene_model.entities.get_mut(&id) {
                for component in &mut entity.components {
                    if let ComponentData::Animation { tracks, .. } = component {
                        for track in tracks.iter_mut() {
                            let value = match track.property {
                                AnimProperty::Position => entity.transform.translation,
                                AnimProperty::Rotation => entity.transform.rotation_euler,
                                AnimProperty::Scale => entity.transform.scale,
                            };
                            let tkf = TrackKeyframe {
                                time: playback_time,
                                value,
                                easing: EasingType::default(),
                            };
                            track.keyframes.push(tkf);
                            track.keyframes.sort_by(|a, b| {
                                a.time
                                    .partial_cmp(&b.time)
                                    .unwrap_or(std::cmp::Ordering::Equal)
                            });
                        }
                        break;
                    }
                }
                self.scene_model.modified = true;
                self.scene_needs_sync = true;
            }
        }
        if let (Some(id), Some(packed)) = (entity_id, delete_idx) {
            let track_idx = packed / 10000;
            let kf_idx = packed % 10000;
            self.scene_snapshot();
            if let Some(entity) = self.scene_model.entities.get_mut(&id) {
                for component in &mut entity.components {
                    if let ComponentData::Animation { tracks, .. } = component {
                        if let Some(track) = tracks.get_mut(track_idx) {
                            if kf_idx < track.keyframes.len() {
                                track.keyframes.remove(kf_idx);
                            }
                        }
                        break;
                    }
                }
                self.scene_model.modified = true;
                self.scene_needs_sync = true;
            }
        }
    }

    /// Render the floating Timeline window (Phase K). Advances per-entity
    /// playback time while visible, draws a scrubber + keyframe markers, and
    /// provides play/pause/rewind plus "add keyframe at current time" for the
    /// currently selected entity that carries an [`ComponentData::Animation`].
    pub(crate) fn render_timeline(&mut self, ctx: &egui::Context) {
        if !self.timeline_open {
            return;
        }

        // Tick playback every frame the window is open.
        self.animation_playback.tick(&self.scene_model);

        // Find which selected entity has an Animation component.
        let selected_id = self.primary_selected_id;
        let (entity_id, duration, tracks_snapshot): (
            Option<u64>,
            f32,
            Vec<AnimationTrack>,
        ) = match selected_id.and_then(|id| self.scene_model.entities.get(&id).map(|e| (id, e))) {
            Some((id, e)) => {
                let anim = e.components.iter().find_map(|c| match c {
                    ComponentData::Animation { duration, tracks, .. } => {
                        Some((*duration, tracks.clone()))
                    }
                    _ => None,
                });
                match anim {
                    Some((d, ts)) => (Some(id), d, ts),
                    None => (None, 0.0, vec![]),
                }
            }
            None => (None, 0.0, vec![]),
        };

        let mut open = self.timeline_open;
        let playback_time = entity_id
            .and_then(|id| self.animation_playback.times.get(&id).copied())
            .unwrap_or(0.0);

        let mut add_keyframe = false;
        let mut toggle_play = false;
        let mut rewind_requested = false;
        let mut scrub_request: Option<f32> = None;
        let mut delete_idx: Option<usize> = None;

        egui::Window::new("Timeline")
            .open(&mut open)
            .default_width(520.0)
            .resizable(true)
            .show(ctx, |ui| {
                if entity_id.is_none() {
                    ui.label("Select an entity with an Animation component.");
                    return;
                }
                ui.horizontal(|ui| {
                    if ui
                        .button(if self.animation_playback.playing {
                            "Pause"
                        } else {
                            "Play"
                        })
                        .clicked()
                    {
                        toggle_play = true;
                    }
                    if ui.button("Rewind").clicked() {
                        rewind_requested = true;
                    }
                    if ui.button("+ Keyframe @ current time").clicked() {
                        add_keyframe = true;
                    }
                    ui.separator();
                    ui.label(format!("t = {:.2}s / {:.2}s", playback_time, duration));
                });

                ui.separator();

                // Scrubber slider.
                let mut t = playback_time;
                if ui
                    .add(egui::Slider::new(&mut t, 0.0..=duration.max(0.001)).text("scrub"))
                    .changed()
                {
                    scrub_request = Some(t);
                }

                ui.separator();

                // Timeline bar: draw keyframe markers.
                let (rect, _resp) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), 32.0),
                    egui::Sense::hover(),
                );
                ui.painter().rect_filled(
                    rect,
                    2.0,
                    egui::Color32::from_rgb(25, 27, 31),
                );
                let d = duration.max(0.001);
                let track_colors = [
                    egui::Color32::from_rgb(120, 200, 255), // Position
                    egui::Color32::from_rgb(200, 255, 120), // Rotation
                    egui::Color32::from_rgb(255, 160, 120), // Scale
                ];
                for track in &tracks_snapshot {
                    let color = match track.property {
                        AnimProperty::Position => track_colors[0],
                        AnimProperty::Rotation => track_colors[1],
                        AnimProperty::Scale => track_colors[2],
                    };
                    for kf in &track.keyframes {
                        let x = rect.left() + (kf.time / d) * rect.width();
                        let tip = egui::pos2(x, rect.top() + 4.0);
                        let base_l = egui::pos2(x - 5.0, rect.bottom() - 4.0);
                        let base_r = egui::pos2(x + 5.0, rect.bottom() - 4.0);
                        ui.painter().add(egui::Shape::convex_polygon(
                            vec![tip, base_l, base_r],
                            color,
                            egui::Stroke::NONE,
                        ));
                    }
                }
                // Play head.
                let ph_x = rect.left() + (playback_time / d).clamp(0.0, 1.0) * rect.width();
                ui.painter().line_segment(
                    [
                        egui::pos2(ph_x, rect.top()),
                        egui::pos2(ph_x, rect.bottom()),
                    ],
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 180, 80)),
                );

                ui.separator();

                // Per-track keyframe list.
                egui::ScrollArea::vertical().max_height(160.0).show(ui, |ui| {
                    for (t_idx, track) in tracks_snapshot.iter().enumerate() {
                        ui.label(
                            egui::RichText::new(format!(
                                "{} ({} kf)",
                                track.property.label(),
                                track.keyframes.len()
                            ))
                            .strong(),
                        );
                        for (k_idx, kf) in track.keyframes.iter().enumerate() {
                            ui.horizontal(|ui| {
                                ui.monospace(format!(
                                    "  #{:>2}  t={:>6.2}s  [{:+.2},{:+.2},{:+.2}]  {}",
                                    k_idx,
                                    kf.time,
                                    kf.value[0],
                                    kf.value[1],
                                    kf.value[2],
                                    kf.easing.label(),
                                ));
                                if ui.small_button("Delete").clicked() {
                                    delete_idx = Some(t_idx * 10000 + k_idx);
                                }
                            });
                        }
                    }
                });
            });
        self.timeline_open = open;

        // Apply requests (outside the window closure to avoid borrow conflicts).
        if toggle_play {
            self.animation_playback.playing = !self.animation_playback.playing;
        }
        if rewind_requested {
            self.animation_playback.rewind();
        }
        if let Some(t) = scrub_request {
            if let Some(id) = entity_id {
                self.animation_playback.times.insert(id, t);
                self.animation_playback.playing = false;
            }
        }
        if let (Some(id), true) = (entity_id, add_keyframe) {
            self.scene_snapshot();
            if let Some(entity) = self.scene_model.entities.get_mut(&id) {
                // Add a keyframe at the current time to ALL existing tracks,
                // capturing the entity's current transform for each property.
                for component in &mut entity.components {
                    if let ComponentData::Animation { tracks, .. } = component {
                        for track in tracks.iter_mut() {
                            let value = match track.property {
                                AnimProperty::Position => entity.transform.translation,
                                AnimProperty::Rotation => entity.transform.rotation_euler,
                                AnimProperty::Scale => entity.transform.scale,
                            };
                            let tkf = TrackKeyframe {
                                time: playback_time,
                                value,
                                easing: EasingType::default(),
                            };
                            track.keyframes.push(tkf);
                            track.keyframes.sort_by(|a, b| {
                                a.time
                                    .partial_cmp(&b.time)
                                    .unwrap_or(std::cmp::Ordering::Equal)
                            });
                        }
                        break;
                    }
                }
                self.scene_model.modified = true;
                self.scene_needs_sync = true;
            }
        }
        if let (Some(id), Some(packed)) = (entity_id, delete_idx) {
            // packed = track_idx * 10000 + keyframe_idx
            let track_idx = packed / 10000;
            let kf_idx = packed % 10000;
            self.scene_snapshot();
            if let Some(entity) = self.scene_model.entities.get_mut(&id) {
                for component in &mut entity.components {
                    if let ComponentData::Animation { tracks, .. } = component {
                        if let Some(track) = tracks.get_mut(track_idx) {
                            if kf_idx < track.keyframes.len() {
                                track.keyframes.remove(kf_idx);
                            }
                        }
                        break;
                    }
                }
                self.scene_model.modified = true;
                self.scene_needs_sync = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pos_track(keyframes: Vec<TrackKeyframe>) -> AnimationTrack {
        AnimationTrack {
            property: AnimProperty::Position,
            keyframes,
            events: vec![],
        }
    }

    fn tkf(time: f32, x: f32) -> TrackKeyframe {
        TrackKeyframe {
            time,
            value: [x, 0.0, 0.0],
            easing: EasingType::Linear,
        }
    }

    #[test]
    fn empty_tracks_returns_base() {
        let base = TransformData::default();
        let result = sample_animation_tracks(&[], 0.5, &base);
        assert!((result.translation[0] - base.translation[0]).abs() < 1e-5);
    }

    #[test]
    fn single_keyframe_returns_that_value() {
        let base = TransformData::default();
        let tracks = [pos_track(vec![tkf(0.0, 3.0)])];
        let out = sample_animation_tracks(&tracks, 99.0, &base);
        assert!((out.translation[0] - 3.0).abs() < 1e-5);
    }

    #[test]
    fn linear_interpolation_mid() {
        let base = TransformData::default();
        let tracks = [pos_track(vec![tkf(0.0, 0.0), tkf(2.0, 10.0)])];
        let out = sample_animation_tracks(&tracks, 1.0, &base);
        assert!((out.translation[0] - 5.0).abs() < 1e-5);
    }

    #[test]
    fn clamps_before_first_and_after_last() {
        let base = TransformData::default();
        let tracks = [pos_track(vec![tkf(1.0, 5.0), tkf(2.0, 15.0)])];
        let before = sample_animation_tracks(&tracks, -1.0, &base);
        let after = sample_animation_tracks(&tracks, 10.0, &base);
        assert!((before.translation[0] - 5.0).abs() < 1e-5);
        assert!((after.translation[0] - 15.0).abs() < 1e-5);
    }

    #[test]
    fn ease_in_quad_at_midpoint() {
        let v = ease(EasingType::EaseInQuad, 0.5);
        assert!((v - 0.25).abs() < 1e-5);
    }

    #[test]
    fn ease_linear_identity() {
        for i in 0..=10 {
            let t = i as f32 / 10.0;
            assert!((ease(EasingType::Linear, t) - t).abs() < 1e-6);
        }
    }

    #[test]
    fn ease_endpoints() {
        // All easing functions should map 0->0 and 1->1.
        for &e in EasingType::ALL {
            assert!(ease(e, 0.0).abs() < 1e-5, "{:?} at 0", e);
            assert!((ease(e, 1.0) - 1.0).abs() < 1e-5, "{:?} at 1", e);
        }
    }

    #[test]
    fn easing_keyframe_interpolation() {
        let base = TransformData::default();
        let tracks = [AnimationTrack {
            property: AnimProperty::Position,
            keyframes: vec![
                TrackKeyframe {
                    time: 0.0,
                    value: [0.0, 0.0, 0.0],
                    easing: EasingType::EaseInQuad,
                },
                TrackKeyframe {
                    time: 2.0,
                    value: [10.0, 0.0, 0.0],
                    easing: EasingType::Linear,
                },
            ],
            events: vec![],
        }];
        // At t=1.0, u=0.5, EaseInQuad(0.5) = 0.25, so x = 0 + 10*0.25 = 2.5
        let out = sample_animation_tracks(&tracks, 1.0, &base);
        assert!((out.translation[0] - 2.5).abs() < 1e-5);
    }

    #[test]
    fn multi_track_overrides_independently() {
        let base = TransformData {
            translation: [1.0, 2.0, 3.0],
            rotation_euler: [0.1, 0.2, 0.3],
            scale: [1.0, 1.0, 1.0],
        };
        let tracks = [
            AnimationTrack {
                property: AnimProperty::Position,
                keyframes: vec![
                    TrackKeyframe { time: 0.0, value: [0.0, 0.0, 0.0], easing: EasingType::Linear },
                    TrackKeyframe { time: 1.0, value: [10.0, 0.0, 0.0], easing: EasingType::Linear },
                ],
                events: vec![],
            },
            AnimationTrack {
                property: AnimProperty::Scale,
                keyframes: vec![
                    TrackKeyframe { time: 0.0, value: [1.0, 1.0, 1.0], easing: EasingType::Linear },
                    TrackKeyframe { time: 1.0, value: [2.0, 2.0, 2.0], easing: EasingType::Linear },
                ],
                events: vec![],
            },
        ];
        let out = sample_animation_tracks(&tracks, 0.5, &base);
        // Position track overrides: x = 5.0
        assert!((out.translation[0] - 5.0).abs() < 1e-5);
        // Scale track overrides: [1.5, 1.5, 1.5]
        assert!((out.scale[0] - 1.5).abs() < 1e-5);
        // Rotation has no track, keeps base
        assert!((out.rotation_euler[0] - 0.1).abs() < 1e-5);
    }
}
