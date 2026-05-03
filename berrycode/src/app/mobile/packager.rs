//! Mobile artifact packagers — Phase E.
//!
//! Wraps `cargo build` for iOS / Android targets and returns the path to
//! the produced binary. Full Xcode-project / IPA codesign and Gradle / AAB
//! assembly are tracked as v0.7.x / v0.8.x follow-ups; this module covers
//! the "compile for the right triple, surface the binary path" baseline so
//! the user can hand the binary off to Xcode / Android Studio for signing
//! while the editor-side automation is being built out.
//!
//! The functions are intentionally thin shell-outs — heavy lifting lives
//! in `xcrun` / `cargo-apk` / `cargo-xcodebuild`, which the user installs
//! separately. We surface failures as `Err(String)` so the UI can render
//! the toolchain-missing message inline.
//!
//! Toolchain readiness is best checked via `app::mobile::probe` *before*
//! calling these — they assume the appropriate `rustup target` and
//! `xcrun` / NDK are already on PATH.

#![allow(dead_code)]

use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{channel, Receiver};

/// Spawn a `cargo build --release --target <triple>` and return the child
/// + a stdout/stderr-merged log channel. The caller polls the channel and
/// reaps the child the same way `app::scene_editor::build_settings::execute_build`
/// does for the desktop path.
fn spawn_cargo_build(root: &str, triple: &str) -> Result<(Child, Receiver<String>), String> {
    let mut cmd = Command::new("cargo");
    cmd.arg("build")
        .arg("--release")
        .arg("--target")
        .arg(triple)
        .current_dir(root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to start `cargo build` for {triple}: {e}"))?;
    let (tx, rx) = channel();
    if let Some(stderr) = child.stderr.take() {
        let tx_clone = tx.clone();
        std::thread::spawn(move || {
            use std::io::BufRead;
            let reader = std::io::BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                let _ = tx_clone.send(line);
            }
        });
    }
    if let Some(stdout) = child.stdout.take() {
        std::thread::spawn(move || {
            use std::io::BufRead;
            let reader = std::io::BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                let _ = tx.send(line);
            }
        });
    }
    Ok((child, rx))
}

/// Build for the iOS Simulator — `aarch64-apple-ios-sim`. Returns the
/// build process + log channel. The caller should hand the resulting
/// `.app` bundle to `xcrun simctl install` (already covered by
/// `app::mobile::runner::start_run` for a `MobileTarget::IosSim`).
pub fn build_ios_simulator(root: &str) -> Result<(Child, Receiver<String>), String> {
    spawn_cargo_build(root, "aarch64-apple-ios-sim")
}

/// Build for an iOS Device — `aarch64-apple-ios`. Codesigning + IPA
/// assembly is intentionally not handled here yet (v0.7.1 will wrap
/// `xcrun altool`); the produced binary needs to be packaged in
/// Xcode / `xcodebuild` until then.
pub fn build_ios_device(root: &str) -> Result<(Child, Receiver<String>), String> {
    spawn_cargo_build(root, "aarch64-apple-ios")
}

/// Build for Android — `aarch64-linux-android`. AAB / APK packaging
/// belongs in v0.8.x; for now this just produces the shared-library
/// binary that `cargo apk` / `xbuild` can pick up downstream.
pub fn build_android(root: &str) -> Result<(Child, Receiver<String>), String> {
    spawn_cargo_build(root, "aarch64-linux-android")
}

/// Required `rustup` target triples per platform. Mirrors
/// `cargo --print target-list` exactly so a missing target produces a
/// "run `rustup target add <triple>`" hint in the UI.
pub fn required_rustup_target(platform_label: &str) -> Option<&'static str> {
    match platform_label {
        "iOS Simulator" => Some("aarch64-apple-ios-sim"),
        "iOS Device" => Some("aarch64-apple-ios"),
        "Android" => Some("aarch64-linux-android"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_target_for_ios_sim() {
        assert_eq!(
            required_rustup_target("iOS Simulator"),
            Some("aarch64-apple-ios-sim")
        );
    }

    #[test]
    fn required_target_for_unknown_platform_is_none() {
        assert!(required_rustup_target("PlayStation").is_none());
    }
}
