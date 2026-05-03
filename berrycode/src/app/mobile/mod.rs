//! Mobile / XR toolchain integration — Phase A of the v0.8 plan.
//!
//! Owns the data model for installed Xcode / Android SDK / NDK / `rustup`
//! targets, and the probe routines that fill it in. The Phase B run /
//! deploy code, the Phase C asset compressor, and the Phase E packagers
//! all read from the same `MobileToolchain` snapshot.

pub mod build_all;
pub mod log_relay;
pub mod one_click;
pub mod packager;
pub mod play_console;
pub mod probe;
pub mod project_emit;
pub mod runner;
pub mod types;

pub use log_relay::{LogSeverity, LogStream};
pub use probe::{probe_adb_devices, probe_all, probe_codesign_identities};
pub use runner::{start_run, MobileRunSession, MobileTarget};
pub use types::*;
