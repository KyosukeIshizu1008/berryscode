//! Debug inspector overlay for Play Mode.
//!
//! Shows live transform values and physics velocities when play mode is active.

use crate::app::BerryCodeApp;

impl BerryCodeApp {
    /// Render debug info overlay in the inspector during play mode.
    pub(crate) fn render_debug_overlay(&mut self, ui: &mut egui::Ui) {
        if !self.play_mode.is_active() {
            return;
        }

        ui.separator();
        ui.colored_label(
            egui::Color32::from_rgb(255, 200, 80),
            "Live Debug Values",
        );

        let selected_id = match self.primary_selected_id {
            Some(id) => id,
            None => {
                ui.label("No entity selected");
                return;
            }
        };

        if let Some(_entity) = self.scene_model.entities.get(&selected_id) {
            // World transform
            let world = self.scene_model.compute_world_transform(selected_id);
            ui.label("World Position:");
            ui.monospace(format!(
                "  [{:.3}, {:.3}, {:.3}]",
                world.translation[0], world.translation[1], world.translation[2]
            ));

            ui.label("World Rotation:");
            ui.monospace(format!(
                "  [{:.3}, {:.3}, {:.3}]",
                world.rotation_euler[0], world.rotation_euler[1], world.rotation_euler[2]
            ));

            ui.label("World Scale:");
            ui.monospace(format!(
                "  [{:.3}, {:.3}, {:.3}]",
                world.scale[0], world.scale[1], world.scale[2]
            ));

            // Physics velocity
            if let Some(vel) = self.physics_state.velocities.get(&selected_id) {
                ui.separator();
                ui.label("Velocity:");
                ui.monospace(format!("  [{:.3}, {:.3}, {:.3}]", vel[0], vel[1], vel[2]));
                let speed = (vel[0] * vel[0] + vel[1] * vel[1] + vel[2] * vel[2]).sqrt();
                ui.monospace(format!("  speed: {:.3} m/s", speed));
            }
        }
    }
}
