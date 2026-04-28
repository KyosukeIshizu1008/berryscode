//! Toolchain detection.
//!
//! Each routine is best-effort and returns `None` on missing tooling so a
//! Linux box without Xcode and an SDK-less Mac both produce a sensible
//! snapshot rather than panicking. Probes are *blocking* on purpose —
//! they're driven from the explicit "Refresh" button in the panel, not the
//! per-frame egui pass.

use super::types::*;
use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, SystemTime};

/// Hard cap on every shelled-out probe. Keeps a wedged `xcrun` / `adb` from
/// freezing the panel on Refresh; the timeout is enforced by the OS via
/// `kill_on_drop`-style semantics on the spawned child below.
const PROBE_TIMEOUT: Duration = Duration::from_secs(8);

pub fn probe_all() -> MobileToolchain {
    MobileToolchain {
        xcode: probe_xcode(),
        android: probe_android(),
        rust_targets: probe_rust_targets(),
        last_probed: Some(SystemTime::now()),
    }
}

// ─── Xcode ───────────────────────────────────────────────────────────────

pub fn probe_xcode() -> Option<XcodeInstall> {
    if !cfg!(target_os = "macos") {
        return None;
    }

    let developer_dir = run_capture("xcode-select", &["-p"])?.trim().to_string();
    if developer_dir.is_empty() {
        return None;
    }

    let version = run_capture("xcodebuild", &["-version"])
        .and_then(|s| s.lines().next().map(|l| l.trim().to_string()))
        .unwrap_or_else(|| "Xcode (unknown version)".into());

    let sdks = run_capture("xcodebuild", &["-showsdks"])
        .map(parse_xcodebuild_sdks)
        .unwrap_or_default();

    let simulators = run_capture("xcrun", &["simctl", "list", "-j", "devices"])
        .and_then(|json| parse_simctl_devices(&json).ok())
        .unwrap_or_default();

    Some(XcodeInstall {
        developer_dir: PathBuf::from(developer_dir),
        version,
        sdks,
        simulators,
        codesign_identities: Vec::new(),
    })
}

fn parse_xcodebuild_sdks(stdout: String) -> Vec<String> {
    // Lines look like:
    //   iOS 17.5                          -sdk iphoneos17.5
    //   iOS 17.5 Simulator                -sdk iphonesimulator17.5
    // We keep the human-readable left half (everything before "-sdk") so the
    // panel can show "iOS 17.5 Simulator" rather than the raw flag.
    let mut out = Vec::new();
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(idx) = line.find("-sdk ") {
            let name = line[..idx].trim();
            if !name.is_empty() {
                out.push(name.to_string());
            }
        }
    }
    out
}

fn parse_simctl_devices(json: &str) -> Result<Vec<Simulator>, String> {
    let v: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("simctl json: {e}"))?;
    let devices = v
        .get("devices")
        .and_then(|d| d.as_object())
        .ok_or_else(|| "simctl: no `devices` map".to_string())?;

    let mut out = Vec::new();
    for (runtime_key, list) in devices {
        let Some(arr) = list.as_array() else { continue };
        let runtime = humanise_runtime(runtime_key);
        let family = classify_runtime(runtime_key);
        for d in arr {
            let Some(udid) = d.get("udid").and_then(|s| s.as_str()) else {
                continue;
            };
            let name = d
                .get("name")
                .and_then(|s| s.as_str())
                .unwrap_or("Unknown")
                .to_string();
            let state = match d.get("state").and_then(|s| s.as_str()) {
                Some("Booted") => SimState::Booted,
                Some("Shutdown") => SimState::Shutdown,
                _ => SimState::Unknown,
            };
            // Skip unavailable simulators (their runtime image was deleted).
            // simctl reports them with `isAvailable: false`.
            if d.get("isAvailable")
                .and_then(|v| v.as_bool())
                .map(|b| !b)
                .unwrap_or(false)
            {
                continue;
            }
            out.push(Simulator {
                udid: udid.to_string(),
                name,
                runtime: runtime.clone(),
                state,
                family,
            });
        }
    }
    Ok(out)
}

fn humanise_runtime(key: &str) -> String {
    // Keys look like `com.apple.CoreSimulator.SimRuntime.iOS-17-5`; we want
    // "iOS 17.5".
    let tail = key
        .rsplit('.')
        .next()
        .unwrap_or(key)
        .trim_start_matches("SimRuntime.");
    let tail = tail.replace('-', " ");
    // Convert "iOS 17 5" → "iOS 17.5" by re-joining the trailing version
    // chunks with a dot.
    let parts: Vec<&str> = tail.split_whitespace().collect();
    if parts.len() >= 3 {
        let head = parts[0];
        let version = parts[1..].join(".");
        format!("{head} {version}")
    } else {
        tail
    }
}

fn classify_runtime(key: &str) -> SimFamily {
    let lower = key.to_lowercase();
    if lower.contains("iphone") || lower.contains(".ios-") {
        SimFamily::Ios
    } else if lower.contains("xros") || lower.contains("visionos") {
        SimFamily::VisionOs
    } else {
        SimFamily::Other
    }
}

// ─── Android ─────────────────────────────────────────────────────────────

pub fn probe_android() -> Option<AndroidInstall> {
    let sdk_root = locate_android_sdk()?;
    if !sdk_root.exists() {
        return None;
    }

    let platforms = list_dir_versions(&sdk_root.join("platforms"), "android-");
    let build_tools = list_dir_names(&sdk_root.join("build-tools"));
    let ndk = locate_ndk(&sdk_root);

    let adb_path = sdk_root.join("platform-tools").join(adb_filename());
    let adb = if adb_path.exists() {
        Some(adb_path)
    } else {
        None
    };

    Some(AndroidInstall {
        sdk_root,
        ndk,
        platforms,
        build_tools,
        adb,
        devices: Vec::new(),
    })
}

fn locate_android_sdk() -> Option<PathBuf> {
    for var in ["ANDROID_HOME", "ANDROID_SDK_ROOT"] {
        if let Ok(p) = std::env::var(var) {
            if !p.is_empty() {
                return Some(PathBuf::from(p));
            }
        }
    }

    // Fall back to the platform-conventional install paths.
    let home = std::env::var("HOME").ok().map(PathBuf::from);
    let candidates = [
        home.as_ref().map(|h| h.join("Library/Android/sdk")), // macOS
        home.as_ref().map(|h| h.join("Android/Sdk")),         // Linux
        std::env::var("LOCALAPPDATA")
            .ok()
            .map(|p| PathBuf::from(p).join("Android/Sdk")), // Windows
    ];
    candidates.into_iter().flatten().find(|p| p.exists())
}

fn locate_ndk(sdk_root: &std::path::Path) -> Option<NdkInstall> {
    // NDK can live under `ndk/<version>` (cmdline-tools layout) or `ndk-bundle`
    // (legacy). Prefer the newest entry in the versioned directory.
    let versioned = sdk_root.join("ndk");
    if versioned.is_dir() {
        let mut dirs: Vec<PathBuf> = std::fs::read_dir(&versioned)
            .ok()?
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        dirs.sort();
        if let Some(latest) = dirs.into_iter().last() {
            let version = read_ndk_version(&latest).unwrap_or_else(|| "unknown".into());
            return Some(NdkInstall {
                root: latest,
                version,
            });
        }
    }
    let legacy = sdk_root.join("ndk-bundle");
    if legacy.is_dir() {
        let version = read_ndk_version(&legacy).unwrap_or_else(|| "unknown".into());
        return Some(NdkInstall {
            root: legacy,
            version,
        });
    }
    None
}

fn read_ndk_version(root: &std::path::Path) -> Option<String> {
    let props = std::fs::read_to_string(root.join("source.properties")).ok()?;
    for line in props.lines() {
        if let Some(rest) = line.strip_prefix("Pkg.Revision") {
            return Some(rest.trim_start_matches([' ', '=']).trim().to_string());
        }
    }
    None
}

fn list_dir_versions(dir: &std::path::Path, prefix: &str) -> Vec<u32> {
    let mut out: Vec<u32> = std::fs::read_dir(dir)
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.strip_prefix(prefix)?.parse().ok()
        })
        .collect();
    out.sort();
    out
}

fn list_dir_names(dir: &std::path::Path) -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(dir)
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    out.sort();
    out
}

fn adb_filename() -> &'static str {
    if cfg!(target_os = "windows") {
        "adb.exe"
    } else {
        "adb"
    }
}

// ─── Rust targets ─────────────────────────────────────────────────────────

pub fn probe_rust_targets() -> HashSet<String> {
    run_capture("rustup", &["target", "list", "--installed"])
        .map(|s| {
            s.lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

// ─── Lazy probes (driven by explicit panel buttons) ──────────────────────

/// Run `security find-identity -v -p codesigning` and parse the result. Lazy
/// because codesign identity enumeration is a keychain operation — we keep
/// it off the cold-start path even though it doesn't prompt today.
pub fn probe_codesign_identities() -> Vec<CodesignIdentity> {
    if !cfg!(target_os = "macos") {
        return Vec::new();
    }
    run_capture("security", &["find-identity", "-v", "-p", "codesigning"])
        .map(|s| parse_security_find_identity(&s))
        .unwrap_or_default()
}

fn parse_security_find_identity(stdout: &str) -> Vec<CodesignIdentity> {
    // Output:
    //   Policy: Code Signing
    //     Matching identities
    //     1) ABCD1234EF... "Apple Development: Foo Bar (XXXXXXXXXX)"
    //     2) 1234ABCDEF... "Developer ID Application: Acme (YYYYYYYYYY)"
    //        2 identities found
    //
    //     Valid identities only
    //     1) ABCD1234EF... "Apple Development: Foo Bar (XXXXXXXXXX)"
    //        1 valid identities found
    //
    // We accept any line shaped `\d+) <hex> "<name>"` and dedupe by id so
    // the "all identities" + "valid identities only" sections don't double
    // every row.
    let mut out: Vec<CodesignIdentity> = Vec::new();
    for line in stdout.lines() {
        let trimmed = line.trim_start();
        // Skip the "N identities found" trailers — they start with a digit
        // but have no closing `)` in the same shape.
        let Some(after_paren) = trimmed.find(')') else {
            continue;
        };
        if !trimmed[..after_paren].chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let rest = trimmed[after_paren + 1..].trim();
        let Some(id_end) = rest.find(char::is_whitespace) else {
            continue;
        };
        let id = rest[..id_end].to_string();
        let after_id = rest[id_end..].trim();
        let Some(name) = after_id.strip_prefix('"').and_then(|s| s.strip_suffix('"')) else {
            continue;
        };
        if !out.iter().any(|i| i.id == id) {
            out.push(CodesignIdentity {
                id,
                common_name: name.to_string(),
            });
        }
    }
    out
}

/// Run `adb devices -l` against the supplied adb binary. Lazy because
/// invoking adb auto-starts the daemon — the user opts in by clicking
/// "Refresh devices" rather than us spinning up a daemon at startup.
pub fn probe_adb_devices(adb: &std::path::Path) -> Vec<AdbDevice> {
    let path_str = adb.to_string_lossy().to_string();
    run_capture(&path_str, &["devices", "-l"])
        .map(|s| parse_adb_devices(&s))
        .unwrap_or_default()
}

fn parse_adb_devices(stdout: &str) -> Vec<AdbDevice> {
    // Output:
    //   List of devices attached
    //   R5CTC0ABC123  device usb:1-1 product:taimen model:Pixel_2 device:taimen
    //   emulator-5554 device product:sdk_gphone64_arm64 model:sdk_gphone64_arm64
    //   1234567890ABC unauthorized
    //
    // Anything before the first whitespace is the serial; the second field
    // is the state; the rest is `key:value` pairs.
    let mut out = Vec::new();
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("List of devices") {
            continue;
        }
        let mut parts = line.split_whitespace();
        let Some(serial) = parts.next() else { continue };
        let state = parts.next().unwrap_or("");
        let model = parts
            .find_map(|p| p.strip_prefix("model:"))
            .unwrap_or("")
            .to_string();
        out.push(AdbDevice {
            serial: serial.to_string(),
            model,
            authorised: state == "device",
        });
    }
    out
}

// ─── Helpers ──────────────────────────────────────────────────────────────

/// Spawn a command, wait up to `PROBE_TIMEOUT`, return stdout as a String on
/// success. Anything else (binary missing, non-zero exit, timeout) returns
/// `None`. We intentionally swallow stderr — every caller that needs the
/// failure detail is a Phase B / E concern, not Phase A.
fn run_capture(program: &str, args: &[&str]) -> Option<String> {
    use std::io::Read;
    use std::process::Stdio;

    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    let start = std::time::Instant::now();
    loop {
        match child.try_wait().ok()? {
            Some(status) if status.success() => break,
            Some(_) => return None,
            None => {
                if start.elapsed() >= PROBE_TIMEOUT {
                    let _ = child.kill();
                    return None;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }

    let mut buf = String::new();
    child.stdout.as_mut()?.read_to_string(&mut buf).ok()?;
    Some(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_xcodebuild_sdks_extracts_names() {
        let stdout = "iOS SDKs:\n\
             \tiOS 17.5                          \t-sdk iphoneos17.5\n\
             \tiOS 17.5 Simulator                \t-sdk iphonesimulator17.5\n\
             \n";
        let sdks = parse_xcodebuild_sdks(stdout.into());
        assert_eq!(sdks, vec!["iOS 17.5", "iOS 17.5 Simulator"]);
    }

    #[test]
    fn parse_simctl_devices_filters_unavailable() {
        let json = serde_json::json!({
            "devices": {
                "com.apple.CoreSimulator.SimRuntime.iOS-17-5": [
                    { "udid": "abc", "name": "iPhone 15", "state": "Booted", "isAvailable": true },
                    { "udid": "xyz", "name": "iPhone XS", "state": "Shutdown", "isAvailable": false },
                ]
            }
        })
        .to_string();
        let sims = parse_simctl_devices(&json).unwrap();
        assert_eq!(sims.len(), 1);
        assert_eq!(sims[0].udid, "abc");
        assert_eq!(sims[0].state, SimState::Booted);
        assert_eq!(sims[0].family, SimFamily::Ios);
        assert_eq!(sims[0].runtime, "iOS 17.5");
    }

    #[test]
    fn humanise_runtime_visionos() {
        let r = humanise_runtime("com.apple.CoreSimulator.SimRuntime.xrOS-1-2");
        // "xrOS" is the visionOS internal name; we keep it verbatim — the
        // panel groups by `SimFamily` rather than the string.
        assert!(r.starts_with("xrOS"));
    }

    #[test]
    fn classify_runtime_buckets() {
        assert_eq!(
            classify_runtime("com.apple.CoreSimulator.SimRuntime.iOS-17-5"),
            SimFamily::Ios
        );
        assert_eq!(
            classify_runtime("com.apple.CoreSimulator.SimRuntime.xrOS-1-2"),
            SimFamily::VisionOs
        );
        assert_eq!(
            classify_runtime("com.apple.CoreSimulator.SimRuntime.tvOS-17-5"),
            SimFamily::Other
        );
    }

    #[test]
    fn parse_security_find_identity_dedupes_sections() {
        let out = "Policy: Code Signing\n\
            \tMatching identities\n\
            \t1) ABCD1234EF \"Apple Development: Foo Bar (XXXXXXXXXX)\"\n\
            \t2) 1234ABCDEF \"Developer ID Application: Acme (YYYYYYYYYY)\"\n\
            \t2 identities found\n\
            \n\
            \tValid identities only\n\
            \t1) ABCD1234EF \"Apple Development: Foo Bar (XXXXXXXXXX)\"\n\
            \t1 valid identities found\n";
        let ids = parse_security_find_identity(out);
        assert_eq!(ids.len(), 2);
        assert_eq!(ids[0].id, "ABCD1234EF");
        assert!(ids[0].common_name.starts_with("Apple Development:"));
        assert_eq!(ids[1].id, "1234ABCDEF");
    }

    #[test]
    fn parse_security_find_identity_empty() {
        assert!(
            parse_security_find_identity("Policy: Code Signing\n\t0 identities found\n").is_empty()
        );
    }

    #[test]
    fn parse_adb_devices_extracts_state_and_model() {
        let out = "List of devices attached\n\
            R5CTC0ABC123\tdevice usb:1-1 product:taimen model:Pixel_2 device:taimen\n\
            emulator-5554\tdevice product:sdk_gphone64_arm64 model:sdk_gphone64_arm64\n\
            1234567890ABC\tunauthorized\n";
        let ds = parse_adb_devices(out);
        assert_eq!(ds.len(), 3);
        assert_eq!(ds[0].serial, "R5CTC0ABC123");
        assert_eq!(ds[0].model, "Pixel_2");
        assert!(ds[0].authorised);
        assert!(ds[1].serial.starts_with("emulator-"));
        assert!(ds[1].authorised);
        assert!(!ds[2].authorised);
    }

    #[test]
    fn parse_adb_devices_handles_empty_list() {
        let out = "List of devices attached\n\n";
        assert!(parse_adb_devices(out).is_empty());
    }
}
