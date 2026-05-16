//! One-click mobile run — v0.7.8.
//!
//! Wraps `cargo-mobile2` so the user can press a single button and end
//! up with the project running on the iOS Simulator / Android Emulator,
//! without ever leaving BerryCode. The wrapper is intentionally
//! best-effort: missing toolchains return a structured `Err(stage, msg)`
//! so the UI can surface a "click here to install Xcode" /
//! "run `rustup target add …`" hint inline.
//!
//! Stages:
//! 1. **Probe** — `cargo mobile --version`. If missing, kick off
//!    `cargo install cargo-mobile2` (lengthy — UI streams logs).
//! 2. **Init** — first run only: `cargo mobile init --non-interactive`
//!    with the project name / bundle id from `BuildSettings`. Skipped
//!    when `<root>/mobile.toml` exists.
//! 3. **Run** — `cargo apple run --release` (Simulator) or
//!    `cargo android run --release` (emulator / device).
//!
//! The runner returns a `(Child, Receiver<String>)` pair the same
//! shape as `mobile::packager`, so the existing log dock reuses its
//! polling loop.

#![allow(dead_code)]

use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{channel, Receiver};

use crate::common::shell::shell_quote;

#[derive(Debug)]
pub struct OneClickConfig<'a> {
    pub project_root: &'a Path,
    pub project_name: &'a str,
    pub ios_bundle_id: &'a str,
    pub android_package_name: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub enum OneClickStage {
    /// `cargo mobile` binary lookup.
    ProbeToolchain,
    /// `cargo install cargo-mobile2` if missing.
    InstallToolchain,
    /// `cargo mobile init` first-run scaffolding.
    InitProject,
    /// `cargo apple run` / `cargo android run` final launch.
    Launch,
    /// `rustup target add <triples>`.
    InstallRustTargets,
    /// `xcodebuild -downloadPlatform iOS`.
    DownloadIosSimRuntime,
}

impl OneClickStage {
    pub fn label(self) -> &'static str {
        match self {
            OneClickStage::ProbeToolchain => "probe `cargo mobile`",
            OneClickStage::InstallToolchain => "install cargo-mobile2",
            OneClickStage::InitProject => "init mobile project",
            OneClickStage::Launch => "launch on device",
            OneClickStage::InstallRustTargets => "install rustup targets",
            OneClickStage::DownloadIosSimRuntime => "download iOS Simulator runtime",
        }
    }
}

#[derive(Debug)]
pub struct OneClickError {
    pub stage: OneClickStage,
    pub message: String,
}

impl OneClickError {
    fn new(stage: OneClickStage, message: impl Into<String>) -> Self {
        Self {
            stage,
            message: message.into(),
        }
    }
}

/// True if `cargo mobile --version` exits successfully.
pub fn probe_cargo_mobile() -> bool {
    Command::new("cargo")
        .args(["mobile", "--version"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Spawn `cargo install cargo-mobile2` and stream logs into the
/// returned channel. The caller is responsible for waiting on the
/// child + reflecting completion in the UI.
pub fn install_cargo_mobile() -> Result<(Child, Receiver<String>), OneClickError> {
    spawn_with_log_channel(
        Command::new("cargo")
            .args(["install", "cargo-mobile2", "--locked"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped()),
        OneClickStage::InstallToolchain,
    )
}

/// Spawn `rustup target add <triples...>` to fetch the missing Rust
/// mobile targets in one shot. Caller waits on the child and reflects
/// completion / failure in the UI.
pub fn install_rust_targets(triples: &[&str]) -> Result<(Child, Receiver<String>), OneClickError> {
    if triples.is_empty() {
        return Err(OneClickError::new(
            OneClickStage::InstallRustTargets,
            "no targets requested — nothing to install.",
        ));
    }
    let mut cmd = Command::new("rustup");
    cmd.arg("target").arg("add");
    for t in triples {
        cmd.arg(t);
    }
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    spawn_with_log_channel(&mut cmd, OneClickStage::InstallRustTargets)
}

/// Spawn `xcodebuild -downloadPlatform iOS` to fetch a Simulator
/// runtime. The download is several gigabytes and can take a while; the
/// caller polls the streamed log lines for progress.
///
/// macOS-only because `xcodebuild` ships with Xcode. We don't gate this
/// at compile time so the function stays callable from cross-platform
/// code; on non-macOS hosts the spawn fails and the error surfaces in
/// the UI just like a missing binary would.
pub fn download_ios_simulator_runtime() -> Result<(Child, Receiver<String>), OneClickError> {
    spawn_with_log_channel(
        Command::new("xcodebuild")
            .args(["-downloadPlatform", "iOS"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped()),
        OneClickStage::DownloadIosSimRuntime,
    )
}

/// Run `cargo mobile init` non-interactively. Most prompts answer
/// from `OneClickConfig`; the rest take the cargo-mobile2 default.
/// Skipped automatically when `<root>/mobile.toml` already exists.
pub fn init_mobile_project_if_needed(
    cfg: &OneClickConfig<'_>,
) -> Result<Option<(Child, Receiver<String>)>, OneClickError> {
    if cfg.project_root.join("mobile.toml").exists() {
        return Ok(None);
    }
    if cfg.ios_bundle_id.is_empty() {
        return Err(OneClickError::new(
            OneClickStage::InitProject,
            "iOS Bundle ID is empty — set it in Build Settings before \
             running `cargo mobile init`. cargo-mobile2 won't accept \
             an empty value.",
        ));
    }
    let pair = spawn_with_log_channel(
        Command::new("cargo")
            .args([
                "mobile",
                "init",
                "--non-interactive",
                "--name",
                cfg.project_name,
                "--bundle-id",
                cfg.ios_bundle_id,
            ])
            .current_dir(cfg.project_root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped()),
        OneClickStage::InitProject,
    )?;
    Ok(Some(pair))
}

/// Build the cargo-mobile2 Xcode project for the iOS Simulator SDK and
/// hand the resulting `.app` off to `simctl` so it boots a sim,
/// installs, and launches the game. Returns the child + log channel;
/// caller polls.
///
/// `cargo apple run` is hardcoded to "deploy IPA to connected device"
/// in cargo-mobile2 and has no Simulator path — when a real iPhone is
/// plugged in, it ends up calling xcodebuild against `iphoneos` and
/// dies on provisioning (exit 65). So we drive xcodebuild + simctl
/// ourselves here.
pub fn run_on_ios_simulator(
    cfg: &OneClickConfig<'_>,
) -> Result<(Child, Receiver<String>), OneClickError> {
    let apple_dir = cfg.project_root.join("gen").join("apple");
    if !apple_dir.exists() {
        return Err(OneClickError::new(
            OneClickStage::Launch,
            "`gen/apple` not found — click `Initialize for Mobile` first.",
        ));
    }
    let name_lower = cfg.project_name.to_lowercase();
    let xcodeproj = apple_dir.join(format!("{name_lower}.xcodeproj"));
    if !xcodeproj.exists() {
        return Err(OneClickError::new(
            OneClickStage::Launch,
            format!(
                "Xcode project not found at `{}`. Re-run `Initialize for Mobile`.",
                xcodeproj.display()
            ),
        ));
    }
    // Fall back to mobile.toml's `[app] identifier` when Build Settings
    // hasn't been filled — for an existing cargo-mobile2 project, the
    // bundle id already lives there and re-typing it would just drift.
    let bundle_id_owned;
    let bundle_id: &str = if cfg.ios_bundle_id.is_empty() {
        bundle_id_owned = read_bundle_id_from_mobile_toml(cfg.project_root).ok_or_else(|| {
            OneClickError::new(
                OneClickStage::Launch,
                "iOS Bundle ID is empty in Build Settings and `mobile.toml` \
                 has no `[app] identifier`. Set one and retry.",
            )
        })?;
        &bundle_id_owned
    } else {
        cfg.ios_bundle_id
    };
    let udid = pick_ios_simulator()?;
    let derived = apple_dir.join("build-sim");
    let products = derived.join("Build/Products/Release-iphonesimulator");

    let proj_q = shell_quote(&xcodeproj.display().to_string());
    let scheme_q = shell_quote(&format!("{name_lower}_iOS"));
    let derived_q = shell_quote(&derived.display().to_string());
    let products_q = shell_quote(&products.display().to_string());
    let udid_q = shell_quote(&udid);
    let bundle_q = shell_quote(bundle_id);

    // Single `sh -c` pipeline so the caller gets one Child to track and
    // one merged log stream.
    //
    // Three xcodebuild settings are non-obvious:
    //   - CODE_SIGNING_REQUIRED=NO / CODE_SIGN_IDENTITY='': sim builds
    //     don't need real codesigning; without these xcodebuild still
    //     consults the keychain.
    //   - GCC_PREPROCESSOR_DEFINITIONS='NDEBUG=1': cargo-mobile2's
    //     "Build Rust Code" script uses `${GCC_PREPROCESSOR_DEFINITIONS:?}`
    //     which exits if the setting is empty. The Release config it
    //     generates leaves the setting blank, so we inject NDEBUG=1.
    //   - BINDGEN_EXTRA_CLANG_ARGS_aarch64_apple_ios_sim: bevy's log
    //     stack pulls in `tracing-oslog`, whose build.rs runs bindgen
    //     against `os/log.h`. Without `-target …-simulator -isysroot
    //     <sim sdk>`, clang reports "version 'sim' in target triple is
    //     invalid" and "os/log.h not found". Setting it as an xcodebuild
    //     build setting propagates it to the script-phase env that
    //     `cargo apple xcode-script` then forwards to cargo.
    let script = format!(
        "set -e; \
         SDK=$(xcrun --sdk iphonesimulator --show-sdk-path); \
         echo '── Building for iOS Simulator (xcodebuild -sdk iphonesimulator) ──'; \
         xcodebuild \
             -project {proj_q} \
             -scheme {scheme_q} \
             -configuration release \
             -sdk iphonesimulator \
             -derivedDataPath {derived_q} \
             CODE_SIGNING_REQUIRED=NO \
             CODE_SIGN_IDENTITY='' \
             GCC_PREPROCESSOR_DEFINITIONS='NDEBUG=1' \
             \"BINDGEN_EXTRA_CLANG_ARGS_aarch64_apple_ios_sim=-target arm64-apple-ios14.0-simulator -isysroot $SDK\" \
             build; \
         APP=$(find {products_q} -maxdepth 1 -name '*.app' -print -quit); \
         if [ -z \"$APP\" ]; then \
             echo \"error: built .app not found under {products_q}\"; \
             exit 1; \
         fi; \
         echo \"── Booting simulator {udid} ──\"; \
         xcrun simctl boot {udid_q} >/dev/null 2>&1 || true; \
         open -a Simulator; \
         echo \"── Installing $APP ──\"; \
         xcrun simctl install {udid_q} \"$APP\"; \
         echo \"── Launching {bundle_q} ──\"; \
         exec xcrun simctl launch --console-pty {udid_q} {bundle_q}"
    );

    spawn_with_log_channel(
        Command::new("sh")
            .arg("-c")
            .arg(&script)
            .current_dir(cfg.project_root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped()),
        OneClickStage::Launch,
    )
}

/// Read `[app].identifier` from `<project_root>/mobile.toml`. Returns
/// `None` if the file is missing, unparseable, or has no identifier.
fn read_bundle_id_from_mobile_toml(project_root: &Path) -> Option<String> {
    let raw = std::fs::read_to_string(project_root.join("mobile.toml")).ok()?;
    let parsed: toml::Value = raw.parse().ok()?;
    parsed
        .get("app")
        .and_then(|v| v.get("identifier"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

/// Pick a sensible iOS Simulator UDID: prefer one that is already
/// `Booted`, otherwise the newest iPhone runtime available. Returns a
/// structured error pointing the user at Xcode → Settings → Platforms
/// when no sims are installed.
fn pick_ios_simulator() -> Result<String, OneClickError> {
    let out = Command::new("xcrun")
        .args(["simctl", "list", "-j", "devices"])
        .output()
        .map_err(|e| {
            OneClickError::new(
                OneClickStage::Launch,
                format!("`xcrun simctl` not found: {e}"),
            )
        })?;
    if !out.status.success() {
        return Err(OneClickError::new(
            OneClickStage::Launch,
            "`xcrun simctl list` failed",
        ));
    }
    let json: serde_json::Value = serde_json::from_slice(&out.stdout)
        .map_err(|e| OneClickError::new(OneClickStage::Launch, format!("simctl json: {e}")))?;
    let devices = json
        .get("devices")
        .and_then(|d| d.as_object())
        .ok_or_else(|| {
            OneClickError::new(OneClickStage::Launch, "simctl: missing `devices` map")
        })?;

    let mut booted: Option<String> = None;
    // (runtime_key, udid) — runtime keys sort lexicographically by version
    // because the suffix is "iOS-26-4" / "iOS-17-5", so a string compare
    // picks the newest installed runtime.
    let mut latest: Option<(String, String)> = None;
    for (runtime_key, list) in devices {
        if !runtime_key.contains(".iOS-") {
            continue;
        }
        let Some(arr) = list.as_array() else { continue };
        for d in arr {
            let Some(udid) = d.get("udid").and_then(|v| v.as_str()) else {
                continue;
            };
            let name = d.get("name").and_then(|v| v.as_str()).unwrap_or("");
            if !name.contains("iPhone") {
                continue;
            }
            let available = d
                .get("isAvailable")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if !available {
                continue;
            }
            let state = d.get("state").and_then(|v| v.as_str()).unwrap_or("");
            if state == "Booted" && booted.is_none() {
                booted = Some(udid.to_string());
            }
            if latest
                .as_ref()
                .map(|(rk, _)| runtime_key.as_str() > rk.as_str())
                .unwrap_or(true)
            {
                latest = Some((runtime_key.clone(), udid.to_string()));
            }
        }
    }
    booted.or_else(|| latest.map(|(_, u)| u)).ok_or_else(|| {
        OneClickError::new(
            OneClickStage::Launch,
            "No iOS Simulator available — install one via Xcode → Settings → Platforms.",
        )
    })
}

/// Same as `run_on_ios_simulator` for the Android emulator / device.
pub fn run_on_android(
    cfg: &OneClickConfig<'_>,
) -> Result<(Child, Receiver<String>), OneClickError> {
    if !probe_cargo_mobile() {
        return Err(OneClickError::new(
            OneClickStage::ProbeToolchain,
            "`cargo mobile` not found. Install with \
             `cargo install cargo-mobile2 --locked`.",
        ));
    }
    if cfg.android_package_name.is_empty() {
        return Err(OneClickError::new(
            OneClickStage::Launch,
            "Android package name is empty — set it in Build Settings.",
        ));
    }
    spawn_with_log_channel(
        Command::new("cargo")
            .args(["android", "run", "--release"])
            .current_dir(cfg.project_root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped()),
        OneClickStage::Launch,
    )
}

fn spawn_with_log_channel(
    cmd: &mut Command,
    stage: OneClickStage,
) -> Result<(Child, Receiver<String>), OneClickError> {
    let mut child = cmd
        .spawn()
        .map_err(|e| OneClickError::new(stage, format!("Failed to spawn `cargo`: {e}")))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_skips_when_mobile_toml_exists() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("mobile.toml"), "[mobile]\n").unwrap();
        let cfg = OneClickConfig {
            project_root: tmp.path(),
            project_name: "demo",
            ios_bundle_id: "com.example.demo",
            android_package_name: "com.example.demo",
        };
        let result = init_mobile_project_if_needed(&cfg).unwrap();
        assert!(
            result.is_none(),
            "expected init to skip when mobile.toml is already on disk"
        );
    }

    #[test]
    fn init_rejects_empty_ios_bundle_id() {
        let tmp = tempfile::tempdir().unwrap();
        let cfg = OneClickConfig {
            project_root: tmp.path(),
            project_name: "demo",
            ios_bundle_id: "",
            android_package_name: "com.example.demo",
        };
        let err = init_mobile_project_if_needed(&cfg).unwrap_err();
        assert!(matches!(err.stage, OneClickStage::InitProject));
        assert!(err.message.contains("Bundle ID"), "{}", err.message);
    }

    #[test]
    fn run_on_android_rejects_empty_package_name() {
        let tmp = tempfile::tempdir().unwrap();
        // mobile.toml exists so `init` doesn't try to run; the package-name
        // check fires before the toolchain probe path.
        std::fs::write(tmp.path().join("mobile.toml"), "[mobile]\n").unwrap();
        let cfg = OneClickConfig {
            project_root: tmp.path(),
            project_name: "demo",
            ios_bundle_id: "com.example.demo",
            android_package_name: "",
        };
        // Skip when toolchain isn't installed (CI has no cargo-mobile2);
        // we only care that the package_name validation fires.
        if probe_cargo_mobile() {
            let err = run_on_android(&cfg).unwrap_err();
            assert!(matches!(err.stage, OneClickStage::Launch));
            assert!(err.message.contains("package name"), "{}", err.message);
        }
    }

    #[test]
    fn stage_labels_are_distinct() {
        let labels = [
            OneClickStage::ProbeToolchain.label(),
            OneClickStage::InstallToolchain.label(),
            OneClickStage::InitProject.label(),
            OneClickStage::Launch.label(),
        ];
        let mut sorted: Vec<&str> = labels.to_vec();
        sorted.sort_unstable();
        let original = sorted.len();
        sorted.dedup();
        assert_eq!(sorted.len(), original, "duplicate OneClickStage label");
    }
}
