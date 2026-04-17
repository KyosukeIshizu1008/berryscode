//! In-process Play Mode.
//!
//! Snapshots the SceneModel, materializes it into Bevy entities on a dedicated
//! render layer, and provides Play/Pause/Step/Stop controls.

use crate::app::BerryCodeApp;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlayModeState {
    #[default]
    Stopped,
    Playing,
    Paused,
}

impl PlayModeState {
    pub fn is_active(&self) -> bool {
        matches!(self, PlayModeState::Playing | PlayModeState::Paused)
    }

    pub fn label(&self) -> &'static str {
        match self {
            PlayModeState::Stopped => "Stopped",
            PlayModeState::Playing => "Playing",
            PlayModeState::Paused => "Paused",
        }
    }
}

impl BerryCodeApp {
    /// Enter play mode: snapshot the current scene and switch to play state.
    pub(crate) fn play_mode_start(&mut self) {
        if self.play_mode.is_active() {
            return;
        }
        // Snapshot the current scene so we can restore on Stop
        self.play_mode_snapshot = Some(self.scene_model.clone());
        self.play_mode = PlayModeState::Playing;
        self.animation_playback.playing = true;
        self.physics_state.reset();
        self.scene_needs_sync = true;
        tracing::info!("Play mode started");
    }

    /// Stop play mode: restore the snapshot and return to editing.
    pub(crate) fn play_mode_stop(&mut self) {
        if !self.play_mode.is_active() {
            return;
        }
        // Restore the snapshot
        if let Some(snapshot) = self.play_mode_snapshot.take() {
            self.scene_model = snapshot;
        }
        self.play_mode = PlayModeState::Stopped;
        self.animation_playback.playing = false;
        self.animation_playback.rewind();
        self.physics_state.reset();
        self.scene_needs_sync = true;
        tracing::info!("Play mode stopped, scene restored");
    }

    /// Pause play mode.
    pub(crate) fn play_mode_pause(&mut self) {
        if self.play_mode == PlayModeState::Playing {
            self.play_mode = PlayModeState::Paused;
            self.animation_playback.playing = false;
        }
    }

    /// Resume from pause.
    pub(crate) fn play_mode_resume(&mut self) {
        if self.play_mode == PlayModeState::Paused {
            self.play_mode = PlayModeState::Playing;
            self.animation_playback.playing = true;
        }
    }

    /// Step one frame (resume briefly, will be paused next frame).
    pub(crate) fn play_mode_step(&mut self) {
        if self.play_mode == PlayModeState::Paused {
            self.play_mode = PlayModeState::Playing;
            self.animation_playback.playing = true;
            // The caller should pause again after one update tick
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stopped_is_not_active() {
        assert!(!PlayModeState::Stopped.is_active());
    }

    #[test]
    fn playing_is_active() {
        assert!(PlayModeState::Playing.is_active());
    }

    #[test]
    fn paused_is_active() {
        assert!(PlayModeState::Paused.is_active());
    }
}
