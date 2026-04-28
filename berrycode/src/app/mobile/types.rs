//! Toolchain data model.
//!
//! Everything here is plain data — populated by `probe`, displayed by
//! `app::mobile_toolchain`, consumed by the Phase B / E pipelines.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::SystemTime;

/// Top-level snapshot of every mobile / XR toolchain BerryCode knows about.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MobileToolchain {
    #[serde(default)]
    pub xcode: Option<XcodeInstall>,
    #[serde(default)]
    pub android: Option<AndroidInstall>,
    /// Rust target triples currently installed via `rustup`. We keep the raw
    /// strings so the panel can match against the new mobile triples without
    /// hard-coding the enum here.
    #[serde(default)]
    pub rust_targets: HashSet<String>,
    #[serde(default)]
    pub last_probed: Option<SystemTime>,
}

impl MobileToolchain {
    /// Whether `rustup target list --installed` reported the given triple.
    pub fn has_rust_target(&self, triple: &str) -> bool {
        self.rust_targets.contains(triple)
    }

    /// Path to the on-disk toolchain cache. Returns `None` only when
    /// `dirs::config_dir()` itself fails (sandboxed env without HOME) — in
    /// that case persistence is silently skipped, the rest of the panel
    /// still works against an in-memory snapshot.
    pub fn cache_path() -> Option<PathBuf> {
        let dir = dirs::config_dir()?.join("berrycode");
        std::fs::create_dir_all(&dir).ok()?;
        Some(dir.join("toolchain.ron"))
    }

    /// Load a cached snapshot from disk. Treats every read / parse failure
    /// the same way: return `None` so the caller falls back to a live probe.
    pub fn load_from_disk() -> Option<Self> {
        let path = Self::cache_path()?;
        let s = std::fs::read_to_string(&path).ok()?;
        ron::from_str(&s).ok()
    }

    /// Persist to `<config>/berrycode/toolchain.ron`. Errors here are
    /// non-fatal — the cache is just a startup-speed cache, not durable
    /// state — so we log and move on.
    pub fn save_to_disk(&self) {
        let Some(path) = Self::cache_path() else {
            return;
        };
        let cfg = ron::ser::PrettyConfig::default();
        let Ok(s) = ron::ser::to_string_pretty(self, cfg) else {
            return;
        };
        if let Err(e) = std::fs::write(&path, s) {
            tracing::warn!(
                "failed to save mobile toolchain cache to {}: {}",
                path.display(),
                e
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// Test seam for non-default cache paths.
    fn save_toolchain_to(toolchain: &MobileToolchain, path: &Path) -> std::io::Result<()> {
        let cfg = ron::ser::PrettyConfig::default();
        let s = ron::ser::to_string_pretty(toolchain, cfg)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        std::fs::write(path, s)
    }

    fn load_toolchain_from(path: &Path) -> Option<MobileToolchain> {
        let s = std::fs::read_to_string(path).ok()?;
        ron::from_str(&s).ok()
    }

    #[test]
    fn roundtrip_minimal() {
        let t = MobileToolchain::default();
        let dir = std::env::temp_dir();
        let path = dir.join(format!("berrycode-test-{}.ron", std::process::id()));
        save_toolchain_to(&t, &path).unwrap();
        let loaded = load_toolchain_from(&path).expect("load");
        assert!(loaded.xcode.is_none());
        assert!(loaded.android.is_none());
        assert!(loaded.rust_targets.is_empty());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn roundtrip_with_data() {
        let mut t = MobileToolchain::default();
        t.rust_targets.insert("aarch64-apple-ios".into());
        t.rust_targets.insert("aarch64-linux-android".into());
        t.android = Some(AndroidInstall {
            sdk_root: PathBuf::from("/opt/android-sdk"),
            ndk: Some(NdkInstall {
                root: PathBuf::from("/opt/android-sdk/ndk/27.0"),
                version: "27.0.12077973".into(),
            }),
            platforms: vec![34, 35],
            build_tools: vec!["34.0.0".into()],
            adb: Some(PathBuf::from("/opt/android-sdk/platform-tools/adb")),
            devices: vec![AdbDevice {
                serial: "R5CTC0ABC123".into(),
                model: "Pixel_2".into(),
                authorised: true,
            }],
        });
        let path =
            std::env::temp_dir().join(format!("berrycode-test-{}-data.ron", std::process::id()));
        save_toolchain_to(&t, &path).unwrap();
        let loaded = load_toolchain_from(&path).unwrap();
        assert!(loaded.has_rust_target("aarch64-apple-ios"));
        let a = loaded.android.unwrap();
        assert_eq!(a.platforms, vec![34, 35]);
        assert_eq!(a.devices.len(), 1);
        assert_eq!(a.devices[0].serial, "R5CTC0ABC123");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_returns_none_for_missing_file() {
        let path = std::env::temp_dir().join("berrycode-nonexistent-xxx.ron");
        let _ = std::fs::remove_file(&path);
        assert!(load_toolchain_from(&path).is_none());
    }

    #[test]
    fn load_returns_none_for_corrupt_file() {
        let path =
            std::env::temp_dir().join(format!("berrycode-corrupt-{}.ron", std::process::id()));
        std::fs::write(&path, "this is not valid RON").unwrap();
        assert!(load_toolchain_from(&path).is_none());
        let _ = std::fs::remove_file(&path);
    }
}

// ─── Xcode ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XcodeInstall {
    /// `DEVELOPER_DIR` from `xcode-select -p` — the active Xcode root.
    pub developer_dir: PathBuf,
    /// `xcodebuild -version` first line, e.g. "Xcode 16.4".
    pub version: String,
    /// SDKs reported by `xcodebuild -showsdks` (raw display names).
    pub sdks: Vec<String>,
    pub simulators: Vec<Simulator>,
    /// Output of `security find-identity -v -p codesigning` — populated when
    /// the user opens the signing panel; left empty on initial probe so we
    /// don't surface the keychain prompt unprompted.
    pub codesign_identities: Vec<CodesignIdentity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Simulator {
    pub udid: String,
    pub name: String,
    /// "iOS 17.5" / "visionOS 1.2" — copied verbatim from `simctl list`.
    pub runtime: String,
    pub state: SimState,
    /// Whether this is an iOS / visionOS / tvOS simulator. Lets the panel
    /// filter to just the runtimes BerryCode targets in v0.8.
    pub family: SimFamily,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SimState {
    Booted,
    Shutdown,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SimFamily {
    Ios,
    VisionOs,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodesignIdentity {
    /// SHA-1 fingerprint reported by `security find-identity`.
    pub id: String,
    /// "Apple Development: Foo Bar (XXXXXXXXXX)".
    pub common_name: String,
}

// ─── Android ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AndroidInstall {
    pub sdk_root: PathBuf,
    pub ndk: Option<NdkInstall>,
    /// Installed `platforms;android-XX` entries (the XX as integers).
    pub platforms: Vec<u32>,
    /// Installed `build-tools;X.Y.Z` entries.
    pub build_tools: Vec<String>,
    /// Resolved `adb` binary path — `None` if SDK root has no platform-tools.
    pub adb: Option<PathBuf>,
    /// Devices currently visible to `adb devices -l`. Populated lazily by
    /// the Phase B run panel; left empty on initial probe so we don't ARP
    /// the user's network without consent.
    pub devices: Vec<AdbDevice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NdkInstall {
    pub root: PathBuf,
    /// "27.0.12077973" or similar — read from `source.properties`.
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdbDevice {
    pub serial: String,
    /// Best-effort model string from `adb devices -l` (`product:` field).
    pub model: String,
    /// Whether the device line ended in `device` (vs `unauthorized`,
    /// `offline`, `recovery`).
    pub authorised: bool,
}
