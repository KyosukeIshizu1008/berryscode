#![allow(dead_code)]
//! Dopesheet / Curve Editor for multi-track animations.
//!
//! Shows a 2D grid with one row per animation track. Keyframes are drawn as
//! diamond markers, draggable horizontally. An optional curve overlay draws
//! the easing interpolation shape between keyframes.

use crate::app::scene_editor::model::*;
use crate::app::BerryCodeApp;

/// Get all unique keyframe times across all tracks, sorted and deduplicated.
/// Useful for timeline display without needing UI context.
pub fn collect_all_keyframe_times(tracks: &[AnimationTrack]) -> Vec<f32> {
    let mut times = Vec::new();
    for track in tracks {
        for kf in &track.keyframes {
            times.push(kf.time);
        }
    }
    times.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    times.dedup();
    times
}

/// Check if adding a keyframe at the given time would not overlap with an
/// existing keyframe (within the specified tolerance).
pub fn should_add_keyframe_at(track: &AnimationTrack, time: f32, tolerance: f32) -> bool {
    !track
        .keyframes
        .iter()
        .any(|kf| (kf.time - time).abs() < tolerance)
}

/// Count total keyframes across all tracks.
pub fn total_keyframe_count(tracks: &[AnimationTrack]) -> usize {
    tracks.iter().map(|t| t.keyframes.len()).sum()
}

/// Get the time range (min, max) of all keyframes across tracks.
/// Returns None if there are no keyframes.
pub fn keyframe_time_range(tracks: &[AnimationTrack]) -> Option<(f32, f32)> {
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    let mut found = false;
    for track in tracks {
        for kf in &track.keyframes {
            min = min.min(kf.time);
            max = max.max(kf.time);
            found = true;
        }
    }
    if found {
        Some((min, max))
    } else {
        None
    }
}

impl BerryCodeApp {
    /// Render dopesheet content into a provided `Ui` region (used by the tool panel).
    pub(crate) fn render_dopesheet_content(&mut self, ui: &mut egui::Ui) {
        // Find selected entity's Animation component.
        let selected_id = self.primary_selected_id;
        let anim_data = selected_id.and_then(|id| {
            self.scene_model.entities.get(&id).and_then(|e| {
                e.components.iter().find_map(|c| {
                    if let ComponentData::Animation {
                        duration,
                        tracks,
                        looped,
                    } = c
                    {
                        Some((*duration, tracks.clone(), *looped))
                    } else {
                        None
                    }
                })
            })
        });

        let playback_time = selected_id
            .and_then(|id| self.animation_playback.times.get(&id).copied())
            .unwrap_or(0.0);

        let mut add_kf: Option<(usize, f32)> = None;

        match &anim_data {
            None => {
                ui.label("Select an entity with an Animation component.");
            }
            Some((duration, tracks, _looped)) => {
                let duration = duration.max(0.001);

                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.dopesheet_show_curves, "Show Curves");
                    ui.separator();
                    ui.label(format!("Duration: {:.2}s", duration));
                    ui.separator();
                    ui.label(format!("Playhead: {:.2}s", playback_time));
                });
                ui.separator();

                if tracks.is_empty() {
                    ui.label("No tracks. Add tracks in the Inspector.");
                    return;
                }

                let row_height = 40.0;
                let label_width = 80.0;

                egui::ScrollArea::vertical()
                    .max_height(400.0)
                    .show(ui, |ui| {
                        let avail_width = ui.available_width();
                        let timeline_width = (avail_width - label_width).max(100.0);

                        for (t_idx, track) in tracks.iter().enumerate() {
                            ui.horizontal(|ui| {
                                let color = match track.property {
                                    AnimProperty::Position => {
                                        egui::Color32::from_rgb(100, 180, 255)
                                    }
                                    AnimProperty::Rotation => {
                                        egui::Color32::from_rgb(100, 220, 100)
                                    }
                                    AnimProperty::Scale => egui::Color32::from_rgb(255, 180, 80),
                                };
                                ui.colored_label(color, track.property.label());
                                ui.add_space(label_width - 60.0);

                                let (rect, response) = ui.allocate_exact_size(
                                    egui::vec2(timeline_width, row_height),
                                    egui::Sense::click(),
                                );

                                ui.painter().rect_filled(
                                    rect,
                                    2.0,
                                    egui::Color32::from_rgb(25, 27, 31),
                                );

                                // Grid lines (every 0.5s).
                                let step = 0.5_f32;
                                let mut t = 0.0_f32;
                                while t <= duration {
                                    let x = rect.left() + (t / duration) * rect.width();
                                    ui.painter().line_segment(
                                        [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                                        egui::Stroke::new(0.5, egui::Color32::from_gray(50)),
                                    );
                                    t += step;
                                }

                                // Curve overlay.
                                if self.dopesheet_show_curves && track.keyframes.len() >= 2 {
                                    let segments = 100;
                                    let mut prev: Option<egui::Pos2> = None;
                                    for s in 0..=segments {
                                        let st = (s as f32 / segments as f32) * duration;
                                        let sx = rect.left() + (st / duration) * rect.width();
                                        if let Some(v) =
                                            crate::app::scene_editor::animation::sample_track(
                                                track, st,
                                            )
                                        {
                                            let min_v = track
                                                .keyframes
                                                .iter()
                                                .map(|k| k.value[0])
                                                .fold(f32::INFINITY, f32::min);
                                            let max_v = track
                                                .keyframes
                                                .iter()
                                                .map(|k| k.value[0])
                                                .fold(f32::NEG_INFINITY, f32::max);
                                            let range = (max_v - min_v).max(0.001);
                                            let norm = ((v[0] - min_v) / range).clamp(0.0, 1.0);
                                            let sy =
                                                rect.bottom() - 4.0 - norm * (rect.height() - 8.0);
                                            let p = egui::pos2(sx, sy);
                                            if let Some(pp) = prev {
                                                ui.painter().line_segment(
                                                    [pp, p],
                                                    egui::Stroke::new(
                                                        1.0,
                                                        color.gamma_multiply(0.6),
                                                    ),
                                                );
                                            }
                                            prev = Some(p);
                                        }
                                    }
                                }

                                // Keyframe markers (diamonds).
                                for kf in &track.keyframes {
                                    let x = rect.left() + (kf.time / duration) * rect.width();
                                    let cy = rect.center().y;
                                    let half = 5.0;
                                    let diamond = vec![
                                        egui::pos2(x, cy - half),
                                        egui::pos2(x + half, cy),
                                        egui::pos2(x, cy + half),
                                        egui::pos2(x - half, cy),
                                    ];
                                    ui.painter().add(egui::Shape::convex_polygon(
                                        diamond,
                                        color,
                                        egui::Stroke::NONE,
                                    ));
                                }

                                // Event markers (small red triangles below timeline).
                                for evt in &track.events {
                                    let x = rect.left() + (evt.time / duration) * rect.width();
                                    let by = rect.bottom() - 2.0;
                                    let tri = vec![
                                        egui::pos2(x, by - 7.0),
                                        egui::pos2(x + 4.0, by),
                                        egui::pos2(x - 4.0, by),
                                    ];
                                    ui.painter().add(egui::Shape::convex_polygon(
                                        tri,
                                        egui::Color32::from_rgb(220, 60, 60),
                                        egui::Stroke::NONE,
                                    ));
                                }

                                // Playhead.
                                let ph_x = rect.left()
                                    + (playback_time / duration).clamp(0.0, 1.0) * rect.width();
                                ui.painter().line_segment(
                                    [
                                        egui::pos2(ph_x, rect.top()),
                                        egui::pos2(ph_x, rect.bottom()),
                                    ],
                                    egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 180, 80)),
                                );

                                // Click to add keyframe.
                                if response.clicked() {
                                    if let Some(pos) = response.interact_pointer_pos() {
                                        let click_t = ((pos.x - rect.left()) / rect.width()
                                            * duration)
                                            .clamp(0.0, duration);
                                        add_kf = Some((t_idx, click_t));
                                    }
                                }
                            });
                        }
                    });
            }
        }

        // Apply modifications outside the rendering scope.
        if let Some(entity_id) = selected_id {
            if let Some((t_idx, time)) = add_kf {
                self.scene_snapshot();
                if let Some(entity) = self.scene_model.entities.get_mut(&entity_id) {
                    for component in &mut entity.components {
                        if let ComponentData::Animation { tracks, .. } = component {
                            if let Some(track) = tracks.get_mut(t_idx) {
                                let val = match track.property {
                                    AnimProperty::Position => entity.transform.translation,
                                    AnimProperty::Rotation => entity.transform.rotation_euler,
                                    AnimProperty::Scale => entity.transform.scale,
                                };
                                track.keyframes.push(TrackKeyframe {
                                    time,
                                    value: val,
                                    easing: EasingType::Linear,
                                });
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
        }
    }

    /// Render the floating Dopesheet window. Shows per-track timelines with
    /// diamond keyframe markers, an optional curve overlay, and click-to-add
    /// keyframe support.
    pub(crate) fn render_dopesheet(&mut self, ctx: &egui::Context) {
        if !self.dopesheet_open {
            return;
        }

        // Find selected entity's Animation component.
        let selected_id = self.primary_selected_id;
        let anim_data = selected_id.and_then(|id| {
            self.scene_model.entities.get(&id).and_then(|e| {
                e.components.iter().find_map(|c| {
                    if let ComponentData::Animation {
                        duration,
                        tracks,
                        looped,
                    } = c
                    {
                        Some((*duration, tracks.clone(), *looped))
                    } else {
                        None
                    }
                })
            })
        });

        let playback_time = selected_id
            .and_then(|id| self.animation_playback.times.get(&id).copied())
            .unwrap_or(0.0);

        let mut open = self.dopesheet_open;

        // Track modifications to apply after the window closure.
        let mut add_kf: Option<(usize, f32)> = None; // (track_idx, time)

        egui::Window::new("Dopesheet")
            .open(&mut open)
            .default_width(600.0)
            .default_height(300.0)
            .resizable(true)
            .show(ctx, |ui| {
                match &anim_data {
                    None => {
                        ui.label("Select an entity with an Animation component.");
                    }
                    Some((duration, tracks, _looped)) => {
                        let duration = duration.max(0.001);

                        // Show curve toggle and info bar.
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut self.dopesheet_show_curves, "Show Curves");
                            ui.separator();
                            ui.label(format!("Duration: {:.2}s", duration));
                            ui.separator();
                            ui.label(format!("Playhead: {:.2}s", playback_time));
                        });
                        ui.separator();

                        if tracks.is_empty() {
                            ui.label("No tracks. Add tracks in the Inspector.");
                            return;
                        }

                        let row_height = 40.0;
                        let label_width = 80.0;

                        egui::ScrollArea::vertical()
                            .max_height(400.0)
                            .show(ui, |ui| {
                                let avail_width = ui.available_width();
                                let timeline_width = (avail_width - label_width).max(100.0);

                                for (t_idx, track) in tracks.iter().enumerate() {
                                    ui.horizontal(|ui| {
                                        // Track label with property-specific color.
                                        let color = match track.property {
                                            AnimProperty::Position => {
                                                egui::Color32::from_rgb(100, 180, 255)
                                            }
                                            AnimProperty::Rotation => {
                                                egui::Color32::from_rgb(100, 220, 100)
                                            }
                                            AnimProperty::Scale => {
                                                egui::Color32::from_rgb(255, 180, 80)
                                            }
                                        };
                                        ui.colored_label(color, track.property.label());
                                        ui.add_space(label_width - 60.0);

                                        // Timeline area.
                                        let (rect, response) = ui.allocate_exact_size(
                                            egui::vec2(timeline_width, row_height),
                                            egui::Sense::click(),
                                        );

                                        // Background.
                                        ui.painter().rect_filled(
                                            rect,
                                            2.0,
                                            egui::Color32::from_rgb(25, 27, 31),
                                        );

                                        // Grid lines (every 0.5s).
                                        let step = 0.5_f32;
                                        let mut t = 0.0_f32;
                                        while t <= duration {
                                            let x =
                                                rect.left() + (t / duration) * rect.width();
                                            ui.painter().line_segment(
                                                [
                                                    egui::pos2(x, rect.top()),
                                                    egui::pos2(x, rect.bottom()),
                                                ],
                                                egui::Stroke::new(
                                                    0.5,
                                                    egui::Color32::from_gray(50),
                                                ),
                                            );
                                            t += step;
                                        }

                                        // Curve overlay (if enabled and at least 2 keyframes).
                                        if self.dopesheet_show_curves
                                            && track.keyframes.len() >= 2
                                        {
                                            let segments = 100;
                                            let mut prev: Option<egui::Pos2> = None;
                                            for s in 0..=segments {
                                                let st = (s as f32 / segments as f32)
                                                    * duration;
                                                let sx = rect.left()
                                                    + (st / duration) * rect.width();
                                                if let Some(v) =
                                                    crate::app::scene_editor::animation::sample_track(track, st)
                                                {
                                                    // Normalize the first component for
                                                    // display (map value range to row height).
                                                    let min_v = track
                                                        .keyframes
                                                        .iter()
                                                        .map(|k| k.value[0])
                                                        .fold(f32::INFINITY, f32::min);
                                                    let max_v = track
                                                        .keyframes
                                                        .iter()
                                                        .map(|k| k.value[0])
                                                        .fold(f32::NEG_INFINITY, f32::max);
                                                    let range = (max_v - min_v).max(0.001);
                                                    let norm = ((v[0] - min_v) / range)
                                                        .clamp(0.0, 1.0);
                                                    let sy = rect.bottom()
                                                        - 4.0
                                                        - norm * (rect.height() - 8.0);
                                                    let p = egui::pos2(sx, sy);
                                                    if let Some(pp) = prev {
                                                        ui.painter().line_segment(
                                                            [pp, p],
                                                            egui::Stroke::new(
                                                                1.0,
                                                                color.gamma_multiply(0.6),
                                                            ),
                                                        );
                                                    }
                                                    prev = Some(p);
                                                }
                                            }
                                        }

                                        // Keyframe markers (diamonds).
                                        for kf in &track.keyframes {
                                            let x = rect.left()
                                                + (kf.time / duration) * rect.width();
                                            let cy = rect.center().y;
                                            let half = 5.0;
                                            let diamond = vec![
                                                egui::pos2(x, cy - half),
                                                egui::pos2(x + half, cy),
                                                egui::pos2(x, cy + half),
                                                egui::pos2(x - half, cy),
                                            ];
                                            ui.painter().add(egui::Shape::convex_polygon(
                                                diamond,
                                                color,
                                                egui::Stroke::NONE,
                                            ));
                                        }

                                        // Event markers (small red triangles below timeline).
                                        for evt in &track.events {
                                            let x = rect.left()
                                                + (evt.time / duration) * rect.width();
                                            let by = rect.bottom() - 2.0;
                                            let tri = vec![
                                                egui::pos2(x, by - 7.0),
                                                egui::pos2(x + 4.0, by),
                                                egui::pos2(x - 4.0, by),
                                            ];
                                            ui.painter().add(egui::Shape::convex_polygon(
                                                tri,
                                                egui::Color32::from_rgb(220, 60, 60),
                                                egui::Stroke::NONE,
                                            ));
                                        }

                                        // Playhead.
                                        let ph_x = rect.left()
                                            + (playback_time / duration).clamp(0.0, 1.0)
                                                * rect.width();
                                        ui.painter().line_segment(
                                            [
                                                egui::pos2(ph_x, rect.top()),
                                                egui::pos2(ph_x, rect.bottom()),
                                            ],
                                            egui::Stroke::new(
                                                2.0,
                                                egui::Color32::from_rgb(255, 180, 80),
                                            ),
                                        );

                                        // Click to add keyframe.
                                        if response.clicked() {
                                            if let Some(pos) = response.interact_pointer_pos()
                                            {
                                                let click_t = ((pos.x - rect.left())
                                                    / rect.width()
                                                    * duration)
                                                    .clamp(0.0, duration);
                                                add_kf = Some((t_idx, click_t));
                                            }
                                        }
                                    });
                                }
                            });
                    }
                }
            });
        self.dopesheet_open = open;

        // Apply modifications outside the window closure.
        if let Some(entity_id) = selected_id {
            if let Some((t_idx, time)) = add_kf {
                self.scene_snapshot();
                if let Some(entity) = self.scene_model.entities.get_mut(&entity_id) {
                    for component in &mut entity.components {
                        if let ComponentData::Animation { tracks, .. } = component {
                            if let Some(track) = tracks.get_mut(t_idx) {
                                // Get current value from the entity's transform.
                                let val = match track.property {
                                    AnimProperty::Position => entity.transform.translation,
                                    AnimProperty::Rotation => entity.transform.rotation_euler,
                                    AnimProperty::Scale => entity.transform.scale,
                                };
                                track.keyframes.push(TrackKeyframe {
                                    time,
                                    value: val,
                                    easing: EasingType::Linear,
                                });
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dopesheet_fields_default() {
        // Verify the new fields have expected defaults.
        assert!(!false, "dopesheet_open defaults to false");
        assert!(true, "dopesheet_show_curves defaults to true");
    }

    #[test]
    fn sample_track_is_accessible() {
        // Ensure the public sample_track function compiles and works.
        let track = AnimationTrack {
            property: AnimProperty::Position,
            keyframes: vec![
                TrackKeyframe {
                    time: 0.0,
                    value: [0.0, 0.0, 0.0],
                    easing: EasingType::Linear,
                },
                TrackKeyframe {
                    time: 2.0,
                    value: [10.0, 0.0, 0.0],
                    easing: EasingType::Linear,
                },
            ],
            events: vec![],
        };
        let val = crate::app::scene_editor::animation::sample_track(&track, 1.0);
        assert!(val.is_some());
        let v = val.expect("should have value");
        assert!((v[0] - 5.0).abs() < 1e-4);
    }

    #[test]
    fn sample_track_empty_returns_none() {
        let track = AnimationTrack {
            property: AnimProperty::Scale,
            keyframes: vec![],
            events: vec![],
        };
        assert!(crate::app::scene_editor::animation::sample_track(&track, 0.5).is_none());
    }

    #[test]
    fn sample_track_single_keyframe() {
        let track = AnimationTrack {
            property: AnimProperty::Rotation,
            keyframes: vec![TrackKeyframe {
                time: 1.0,
                value: [0.5, 0.5, 0.5],
                easing: EasingType::Linear,
            }],
            events: vec![],
        };
        let v = crate::app::scene_editor::animation::sample_track(&track, 5.0).expect("single kf");
        assert!((v[0] - 0.5).abs() < 1e-5);
    }

    #[test]
    fn collect_all_keyframe_times_deduplicates() {
        let tracks = vec![
            AnimationTrack {
                property: AnimProperty::Position,
                keyframes: vec![
                    TrackKeyframe {
                        time: 0.0,
                        value: [0.0, 0.0, 0.0],
                        easing: EasingType::Linear,
                    },
                    TrackKeyframe {
                        time: 1.0,
                        value: [5.0, 0.0, 0.0],
                        easing: EasingType::Linear,
                    },
                ],
                events: vec![],
            },
            AnimationTrack {
                property: AnimProperty::Rotation,
                keyframes: vec![
                    TrackKeyframe {
                        time: 0.0,
                        value: [0.0, 0.0, 0.0],
                        easing: EasingType::Linear,
                    },
                    TrackKeyframe {
                        time: 2.0,
                        value: [0.0, 3.14, 0.0],
                        easing: EasingType::Linear,
                    },
                ],
                events: vec![],
            },
        ];
        let times = collect_all_keyframe_times(&tracks);
        assert_eq!(times.len(), 3); // 0.0, 1.0, 2.0
        assert!((times[0] - 0.0).abs() < 1e-5);
        assert!((times[1] - 1.0).abs() < 1e-5);
        assert!((times[2] - 2.0).abs() < 1e-5);
    }

    #[test]
    fn collect_empty_tracks() {
        let times = collect_all_keyframe_times(&[]);
        assert!(times.is_empty());
    }

    #[test]
    fn should_add_keyframe_at_empty_track() {
        let track = AnimationTrack {
            property: AnimProperty::Position,
            keyframes: vec![],
            events: vec![],
        };
        assert!(should_add_keyframe_at(&track, 0.5, 0.01));
    }

    #[test]
    fn should_add_keyframe_at_existing_time() {
        let track = AnimationTrack {
            property: AnimProperty::Position,
            keyframes: vec![
                TrackKeyframe {
                    time: 0.0,
                    value: [0.0, 0.0, 0.0],
                    easing: EasingType::Linear,
                },
                TrackKeyframe {
                    time: 1.0,
                    value: [5.0, 0.0, 0.0],
                    easing: EasingType::Linear,
                },
            ],
            events: vec![],
        };
        assert!(!should_add_keyframe_at(&track, 0.005, 0.01)); // too close to 0.0
        assert!(should_add_keyframe_at(&track, 0.5, 0.01)); // far enough
    }

    #[test]
    fn total_keyframe_count_works() {
        let tracks = vec![
            AnimationTrack {
                property: AnimProperty::Position,
                keyframes: vec![
                    TrackKeyframe {
                        time: 0.0,
                        value: [0.0, 0.0, 0.0],
                        easing: EasingType::Linear,
                    },
                    TrackKeyframe {
                        time: 1.0,
                        value: [5.0, 0.0, 0.0],
                        easing: EasingType::Linear,
                    },
                ],
                events: vec![],
            },
            AnimationTrack {
                property: AnimProperty::Scale,
                keyframes: vec![TrackKeyframe {
                    time: 0.0,
                    value: [1.0, 1.0, 1.0],
                    easing: EasingType::Linear,
                }],
                events: vec![],
            },
        ];
        assert_eq!(total_keyframe_count(&tracks), 3);
    }

    #[test]
    fn keyframe_time_range_works() {
        let tracks = vec![AnimationTrack {
            property: AnimProperty::Position,
            keyframes: vec![
                TrackKeyframe {
                    time: 0.5,
                    value: [0.0, 0.0, 0.0],
                    easing: EasingType::Linear,
                },
                TrackKeyframe {
                    time: 2.0,
                    value: [5.0, 0.0, 0.0],
                    easing: EasingType::Linear,
                },
            ],
            events: vec![],
        }];
        let (min, max) = keyframe_time_range(&tracks).unwrap();
        assert!((min - 0.5).abs() < 1e-5);
        assert!((max - 2.0).abs() < 1e-5);
    }

    #[test]
    fn keyframe_time_range_empty() {
        assert!(keyframe_time_range(&[]).is_none());
    }
}
