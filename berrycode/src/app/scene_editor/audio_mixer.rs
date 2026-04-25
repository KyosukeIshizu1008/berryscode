//! Audio mixer console with channel strips and effects.

use serde::{Deserialize, Serialize};

use crate::app::BerryCodeApp;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioMixerState {
    pub open: bool,
    pub channels: Vec<AudioChannel>,
    pub master_volume: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioChannel {
    pub name: String,
    pub volume: f32,
    pub pan: f32,
    pub mute: bool,
    pub solo: bool,
    pub effects: Vec<AudioEffect>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AudioEffect {
    LowPass { cutoff: f32 },
    HighPass { cutoff: f32 },
    Reverb { wet: f32, room_size: f32 },
    Delay { time_ms: f32, feedback: f32 },
}

impl Default for AudioMixerState {
    fn default() -> Self {
        Self {
            open: false,
            channels: vec![
                AudioChannel::new("Music"),
                AudioChannel::new("SFX"),
                AudioChannel::new("Voice"),
            ],
            master_volume: 1.0,
        }
    }
}

impl AudioChannel {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            volume: 1.0,
            pan: 0.0,
            mute: false,
            solo: false,
            effects: Vec::new(),
        }
    }

    pub fn effective_volume(&self) -> f32 {
        if self.mute {
            0.0
        } else {
            self.volume
        }
    }
}

impl AudioMixerState {
    pub fn add_channel(&mut self, name: &str) {
        self.channels.push(AudioChannel::new(name));
    }

    pub fn remove_channel(&mut self, idx: usize) {
        if idx < self.channels.len() {
            self.channels.remove(idx);
        }
    }

    /// Returns indices of channels that are currently solo'd.
    pub fn solo_channels(&self) -> Vec<usize> {
        self.channels
            .iter()
            .enumerate()
            .filter(|(_, ch)| ch.solo)
            .map(|(i, _)| i)
            .collect()
    }
}

impl BerryCodeApp {
    pub(crate) fn render_audio_mixer(&mut self, ctx: &egui::Context) {
        if !self.audio_mixer.open {
            return;
        }
        let mut open = self.audio_mixer.open;
        egui::Window::new("Audio Mixer")
            .open(&mut open)
            .default_size([500.0, 320.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Master:");
                    ui.add(egui::Slider::new(
                        &mut self.audio_mixer.master_volume,
                        0.0..=2.0,
                    ));
                    if ui.small_button("+ Channel").clicked() {
                        let n = self.audio_mixer.channels.len() + 1;
                        self.audio_mixer.add_channel(&format!("Ch{}", n));
                    }
                });
                ui.separator();
                let mut remove_idx = None;
                let chan_count = self.audio_mixer.channels.len();
                for i in 0..chan_count {
                    ui.horizontal(|ui| {
                        let ch = &mut self.audio_mixer.channels[i];
                        ui.label(&ch.name);
                        ui.add(egui::Slider::new(&mut ch.volume, 0.0..=2.0).text("Vol"));
                        ui.add(egui::Slider::new(&mut ch.pan, -1.0..=1.0).text("Pan"));
                        ui.checkbox(&mut ch.mute, "M");
                        ui.checkbox(&mut ch.solo, "S");
                        ui.label(format!("FX: {}", ch.effects.len()));
                        if ui.small_button("x").clicked() {
                            remove_idx = Some(i);
                        }
                    });
                }
                if let Some(idx) = remove_idx {
                    self.audio_mixer.remove_channel(idx);
                }
            });
        self.audio_mixer.open = open;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_three_channels() {
        let m = AudioMixerState::default();
        assert_eq!(m.channels.len(), 3);
        assert_eq!(m.master_volume, 1.0);
    }

    #[test]
    fn add_and_remove_channel() {
        let mut m = AudioMixerState::default();
        m.add_channel("Ambient");
        assert_eq!(m.channels.len(), 4);
        m.remove_channel(0);
        assert_eq!(m.channels.len(), 3);
    }

    #[test]
    fn effective_volume_muted() {
        let mut ch = AudioChannel::new("test");
        ch.volume = 0.8;
        assert!((ch.effective_volume() - 0.8).abs() < f32::EPSILON);
        ch.mute = true;
        assert_eq!(ch.effective_volume(), 0.0);
    }

    #[test]
    fn solo_channels_filter() {
        let mut m = AudioMixerState::default();
        m.channels[1].solo = true;
        let solos = m.solo_channels();
        assert_eq!(solos, vec![1]);
    }

    #[test]
    fn remove_out_of_bounds_is_noop() {
        let mut m = AudioMixerState::default();
        let before = m.channels.len();
        m.remove_channel(99);
        assert_eq!(m.channels.len(), before);
    }
}
