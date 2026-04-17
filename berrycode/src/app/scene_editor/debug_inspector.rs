//! Debug inspector overlay for Play Mode.
//!
//! Shows live, editable transform values and physics velocities when play mode
//! is active. Editing values during play mode modifies the scene in-place
//! (the snapshot taken at play-start is restored on Stop).

use crate::app::BerryCodeApp;

impl BerryCodeApp {
    /// Render debug info overlay in the inspector during play mode.
    ///
    /// Transform fields are editable via drag-value widgets. Changes set
    /// `scene_needs_sync` so the preview updates in real time.
    pub(crate) fn render_debug_overlay(&mut self, ui: &mut egui::Ui) {
        if !self.play_mode.is_active() {
            return;
        }

        ui.separator();
        ui.colored_label(
            egui::Color32::from_rgb(255, 200, 80),
            "Live Debug",
        );
        ui.label(
            egui::RichText::new("Transform is editable. Custom component values are not reflected at runtime — stop and re-run to apply changes.")
                .size(10.0)
                .color(egui::Color32::from_rgb(180, 160, 120)),
        );

        let selected_id = match self.primary_selected_id {
            Some(id) => id,
            None => {
                ui.label("No entity selected");
                return;
            }
        };

        let mut changed = false;

        if let Some(entity) = self.scene_model.entities.get_mut(&selected_id) {
            // Editable transform during play mode
            ui.label("Position (live):");
            ui.horizontal(|ui| {
                let prefixes = ["x:", "y:", "z:"];
                for i in 0..3 {
                    if ui
                        .add(
                            egui::DragValue::new(&mut entity.transform.translation[i])
                                .speed(0.1)
                                .prefix(prefixes[i]),
                        )
                        .changed()
                    {
                        changed = true;
                    }
                }
            });

            ui.label("Rotation (live):");
            ui.horizontal(|ui| {
                let prefixes = ["x:", "y:", "z:"];
                for i in 0..3 {
                    if ui
                        .add(
                            egui::DragValue::new(&mut entity.transform.rotation_euler[i])
                                .speed(0.01)
                                .prefix(prefixes[i]),
                        )
                        .changed()
                    {
                        changed = true;
                    }
                }
            });

            ui.label("Scale (live):");
            ui.horizontal(|ui| {
                let prefixes = ["x:", "y:", "z:"];
                for i in 0..3 {
                    if ui
                        .add(
                            egui::DragValue::new(&mut entity.transform.scale[i])
                                .speed(0.01)
                                .prefix(prefixes[i]),
                        )
                        .changed()
                    {
                        changed = true;
                    }
                }
            });
        }

        // Show velocity if physics is running
        if let Some(vel) = self.physics_state.velocities.get(&selected_id) {
            ui.separator();
            ui.label("Velocity:");
            ui.monospace(format!(
                "[{:.3}, {:.3}, {:.3}]",
                vel[0], vel[1], vel[2]
            ));
            let speed = (vel[0] * vel[0] + vel[1] * vel[1] + vel[2] * vel[2]).sqrt();
            ui.monospace(format!("speed: {:.3} m/s", speed));
        }

        if changed {
            self.scene_needs_sync = true;
        }
    }
}
