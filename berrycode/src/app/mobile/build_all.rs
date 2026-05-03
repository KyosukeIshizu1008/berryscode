//! Cross-platform "build all" — Phase E follow-up.
//!
//! Runs the user's project through the desktop pipeline + every mobile
//! packager that's enabled in `BuildSettings`, in parallel, so a single
//! click ships to macOS / Windows / Linux / iOS / Android in one go.
//!
//! v0.7.2 ships the orchestrator scaffold: it sequences the platform
//! list, returns a per-platform `Result`, and lets the UI render a
//! status grid. Real parallelism (rayon / std::thread::scope) lands
//! when the per-platform packagers stop being thin shell-outs and grow
//! true Rust-side asset pipelines (v0.7.4+).

#![allow(dead_code)]

use crate::app::mobile::packager;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildAllPlatform {
    MacOs,
    Windows,
    Linux,
    Web,
    IosSimulator,
    IosDevice,
    Android,
}

impl BuildAllPlatform {
    pub fn label(self) -> &'static str {
        match self {
            BuildAllPlatform::MacOs => "macOS",
            BuildAllPlatform::Windows => "Windows",
            BuildAllPlatform::Linux => "Linux",
            BuildAllPlatform::Web => "Web (WASM)",
            BuildAllPlatform::IosSimulator => "iOS Simulator",
            BuildAllPlatform::IosDevice => "iOS Device",
            BuildAllPlatform::Android => "Android",
        }
    }
    pub fn target_triple(self) -> &'static str {
        match self {
            BuildAllPlatform::MacOs => "aarch64-apple-darwin",
            BuildAllPlatform::Windows => "x86_64-pc-windows-msvc",
            BuildAllPlatform::Linux => "x86_64-unknown-linux-gnu",
            BuildAllPlatform::Web => "wasm32-unknown-unknown",
            BuildAllPlatform::IosSimulator => "aarch64-apple-ios-sim",
            BuildAllPlatform::IosDevice => "aarch64-apple-ios",
            BuildAllPlatform::Android => "aarch64-linux-android",
        }
    }
}

#[derive(Debug)]
pub struct PlatformBuildResult {
    pub platform: BuildAllPlatform,
    pub status: BuildStatus,
}

#[derive(Debug)]
pub enum BuildStatus {
    Ok,
    Skipped(String),
    Failed(String),
}

/// Sequentially kick off a build for each requested platform. Each
/// returns the spawned `Child` + its log channel; the caller drives
/// them to completion in whatever async / threading model the UI uses.
/// Failures from one platform don't block the others.
pub fn build_all(
    root: &str,
    platforms: &[BuildAllPlatform],
) -> Vec<(
    BuildAllPlatform,
    Result<(std::process::Child, std::sync::mpsc::Receiver<String>), String>,
)> {
    platforms
        .iter()
        .map(|&p| {
            let res = match p {
                BuildAllPlatform::IosSimulator => packager::build_ios_simulator(root),
                BuildAllPlatform::IosDevice => packager::build_ios_device(root),
                BuildAllPlatform::Android => packager::build_android(root),
                BuildAllPlatform::MacOs
                | BuildAllPlatform::Windows
                | BuildAllPlatform::Linux
                | BuildAllPlatform::Web => Err(format!(
                    "{} desktop / web build dispatch not yet wired into build_all \
                     (use the Build Settings panel for now). Tracked for v0.7.4.",
                    p.label()
                )),
            };
            (p, res)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_triples_match_rustup_names() {
        assert_eq!(
            BuildAllPlatform::IosSimulator.target_triple(),
            "aarch64-apple-ios-sim"
        );
        assert_eq!(
            BuildAllPlatform::Android.target_triple(),
            "aarch64-linux-android"
        );
    }

    #[test]
    fn desktop_targets_emit_skipped_in_v0_7_2() {
        let results = build_all("/tmp", &[BuildAllPlatform::MacOs]);
        assert_eq!(results.len(), 1);
        match &results[0].1 {
            Err(msg) => assert!(msg.contains("v0.7.4"), "{msg}"),
            Ok(_) => panic!("desktop dispatch should be a placeholder until v0.7.4"),
        }
    }
}
