//! Audio editing surfaces for BerryCode.
//!
//! v0.6 ships in phases:
//!
//! - **Phase A** ([`preview`]) — waveform display + click-to-scrub
//!   for WAV files. Decoding lives in [`decode`].
//! - **Phase B** ([`sfx`]) — `SfxRandomiser` data model + inspector
//!   helper for picking one of N variations with pitch / volume
//!   jitter on play.
//!
//! Bevy-side runtime hooks (actual `AudioSource` spawning, hot
//! reload re-issue, spatial sinks) live alongside `bevy_plugin.rs`
//! and arrive in v0.6 phases C–F.

pub mod decode;
pub mod events;
pub mod music_graph;
pub mod preview;
pub mod sfx;
pub mod spatial;
