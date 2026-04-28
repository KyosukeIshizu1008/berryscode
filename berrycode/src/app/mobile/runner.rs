//! Mobile-runtime deploy / launch / log dispatcher.
//!
//! Drives `xcrun simctl` (iOS Simulator), `xcrun devicectl` (iOS Device /
//! visionOS Device), and `adb` (Android / Quest). One process is spawned
//! per `start_run`; its merged stdout/stderr stream feeds a `LogStream`
//! that the panel renders.
//!
//! Phase B owns *only* the runner — the artifact path is supplied by the
//! caller (the user via file picker for now; Phase E will wire the
//! packagers in). The hot-reload TCP socket is Phase B carryover.

use super::log_relay::LogStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Receiver, Sender};

/// What we're launching against. `bundle_id` and `package_name` are
/// kept distinct so the runner can't silently substitute one for the
/// other (an Android package name is also a dotted string but the
/// `am start` command needs the activity suffix that iOS doesn't have).
#[derive(Debug, Clone)]
#[allow(dead_code)] // IosDevice variant is unused until devicectl device probing lands
pub enum MobileTarget {
    IosSim {
        udid: String,
        bundle_id: String,
    },
    IosDevice {
        udid: String,
        bundle_id: String,
    },
    Android {
        serial: String,
        package_name: String,
        /// Activity in the form `MainActivity` or
        /// `com.example.android.MainActivity`. `am start` accepts both
        /// forms; we pass through verbatim.
        activity: String,
    },
}

impl MobileTarget {
    #[allow(dead_code)] // surface for status-bar / window-title use
    pub fn label(&self) -> String {
        match self {
            Self::IosSim { udid, .. } => format!("iOS Sim {}", short(udid)),
            Self::IosDevice { udid, .. } => format!("iOS Device {}", short(udid)),
            Self::Android { serial, .. } => format!("Android {serial}"),
        }
    }

    #[allow(dead_code)] // surface for telemetry / user-visible target kind
    pub fn target_label(&self) -> &'static str {
        match self {
            Self::IosSim { .. } => "iOS Simulator",
            Self::IosDevice { .. } => "iOS Device",
            Self::Android { .. } => "Android",
        }
    }
}

#[allow(dead_code)]
fn short(s: &str) -> String {
    if s.len() > 8 {
        format!("{}…", &s[..8])
    } else {
        s.to_string()
    }
}

/// One launched mobile process — owns the child + the receiver feeding
/// the log stream. Drop kills the child to avoid orphaned simulators
/// after BerryCode quits mid-run.
pub struct MobileRunSession {
    target: MobileTarget,
    child: Option<Child>,
    /// Stays `Some` for the lifetime of the session; `poll` drains from
    /// it into the panel's `LogStream`.
    output_rx: Option<Receiver<String>>,
}

impl std::fmt::Debug for MobileRunSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MobileRunSession")
            .field("target", &self.target)
            .field("running", &self.child.is_some())
            .finish()
    }
}

impl MobileRunSession {
    #[allow(dead_code)] // surface for telemetry; not yet wired into UI
    pub fn target(&self) -> &MobileTarget {
        &self.target
    }

    /// Drain available log lines into `stream`. Non-blocking — call
    /// per-frame from the egui pass.
    pub fn poll_into(&mut self, stream: &mut LogStream) {
        if let Some(rx) = &self.output_rx {
            // Cap per-frame drain so a flood doesn't stall the UI; egui
            // requests a repaint until the channel is empty.
            for _ in 0..256 {
                match rx.try_recv() {
                    Ok(line) => stream.push_line(line),
                    Err(_) => break,
                }
            }
        }
        if let Some(child) = &mut self.child {
            if let Ok(Some(status)) = child.try_wait() {
                stream.push_line(format!("─── Exited with code {:?} ───", status.code()));
                self.child = None;
                self.output_rx = None;
            }
        }
    }

    pub fn is_running(&self) -> bool {
        self.child.is_some()
    }

    pub fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.output_rx = None;
    }
}

impl Drop for MobileRunSession {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Build the argv for a target + artifact pair. Pure function so the
/// dispatch matrix has unit tests; the actual spawn happens in
/// `start_run` below.
pub fn build_command(
    target: &MobileTarget,
    artifact: &Path,
) -> Result<(String, Vec<String>), String> {
    match target {
        MobileTarget::IosSim { udid, bundle_id } => {
            // `simctl launch --console booted` would technically work
            // after a `boot` + `install` sequence, but composing those
            // three subprocesses sequentially makes lifecycle fragile.
            // We use a small shell wrapper so the user gets one Child
            // to track and one log stream regardless of platform.
            let script = format!(
                "set -e; \
                 xcrun simctl boot {udid} >/dev/null 2>&1 || true; \
                 xcrun simctl install {udid} '{app}'; \
                 exec xcrun simctl launch --console-pty {udid} {bundle}",
                udid = udid,
                app = artifact.display(),
                bundle = bundle_id,
            );
            Ok(("sh".into(), vec!["-c".into(), script]))
        }
        MobileTarget::IosDevice { udid, bundle_id } => {
            // `devicectl` ships with Xcode 15+. Older toolchains
            // shipped `ios-deploy`; we deliberately don't fall back —
            // a clear error is better than a silently different path.
            let script = format!(
                "set -e; \
                 xcrun devicectl device install app --device {udid} '{app}'; \
                 exec xcrun devicectl device process launch \
                     --device {udid} --console {bundle}",
                udid = udid,
                app = artifact.display(),
                bundle = bundle_id,
            );
            Ok(("sh".into(), vec!["-c".into(), script]))
        }
        MobileTarget::Android {
            serial,
            package_name,
            activity,
        } => {
            // adb pidof returns empty until the launched activity has
            // attached its primary process; we sleep a beat then attach
            // logcat filtered to that pid so we don't drown in system
            // chatter.
            let component = if activity.contains('.') {
                activity.clone()
            } else {
                format!("{package_name}/.{activity}")
            };
            let component = if component.contains('/') {
                component
            } else {
                format!("{package_name}/{component}")
            };
            let script = format!(
                "set -e; \
                 adb -s {serial} install -r '{apk}'; \
                 adb -s {serial} shell am start -W -n {component}; \
                 sleep 1; \
                 PID=$(adb -s {serial} shell pidof -s {pkg} | tr -d '\\r'); \
                 if [ -z \"$PID\" ]; then \
                   exec adb -s {serial} logcat -v threadtime; \
                 else \
                   exec adb -s {serial} logcat -v threadtime --pid=$PID; \
                 fi",
                serial = serial,
                apk = artifact.display(),
                component = component,
                pkg = package_name,
            );
            Ok(("sh".into(), vec!["-c".into(), script]))
        }
    }
}

/// Spawn the runner subprocess. Returns the live session immediately;
/// the caller should poll it per-frame via `poll_into`. The artifact
/// must already exist on disk (Phase E will produce it; for now the
/// panel's file picker accepts any pre-built `.app` / `.apk`).
pub fn start_run(target: MobileTarget, artifact: PathBuf) -> Result<MobileRunSession, String> {
    if !artifact.exists() {
        return Err(format!("artifact not found: {}", artifact.display()));
    }
    // Linux/Windows hosts can't drive simctl/devicectl. We still allow
    // adb runs there — Android Studio installs adb on every host.
    if matches!(
        target,
        MobileTarget::IosSim { .. } | MobileTarget::IosDevice { .. }
    ) && !cfg!(target_os = "macos")
    {
        return Err("iOS / visionOS targets require macOS".into());
    }

    let (program, args) = build_command(&target, &artifact)?;
    let mut cmd = Command::new(&program);
    cmd.args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("spawn {program} failed: {e}"))?;

    let (tx, rx) = std::sync::mpsc::channel::<String>();
    pump_stream("stdout", child.stdout.take(), tx.clone());
    pump_stream("stderr", child.stderr.take(), tx);

    Ok(MobileRunSession {
        target,
        child: Some(child),
        output_rx: Some(rx),
    })
}

fn pump_stream<R: std::io::Read + Send + 'static>(
    label: &'static str,
    pipe: Option<R>,
    tx: Sender<String>,
) {
    let Some(pipe) = pipe else { return };
    std::thread::spawn(move || {
        use std::io::BufRead;
        let reader = std::io::BufReader::new(pipe);
        for line in reader.lines() {
            let Ok(line) = line else { break };
            // `simctl launch --console-pty` already merges device stdio.
            // `adb logcat` lines are already labelled by priority. We
            // only prefix lines that came in on stderr (e.g. devicectl
            // pre-launch warnings) so the panel can show them inline.
            let payload = if label == "stderr" {
                format!("[stderr] {line}")
            } else {
                line
            };
            if tx.send(payload).is_err() {
                break;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ios_sim_command_includes_boot_install_launch() {
        let target = MobileTarget::IosSim {
            udid: "ABC-DEF".into(),
            bundle_id: "com.example.Game".into(),
        };
        let (program, args) = build_command(&target, Path::new("/tmp/Game.app")).unwrap();
        assert_eq!(program, "sh");
        let script = &args[1];
        assert!(script.contains("simctl boot ABC-DEF"));
        assert!(script.contains("simctl install ABC-DEF '/tmp/Game.app'"));
        assert!(script.contains("simctl launch --console-pty ABC-DEF com.example.Game"));
    }

    #[test]
    fn ios_device_command_uses_devicectl() {
        let target = MobileTarget::IosDevice {
            udid: "00008110-001234".into(),
            bundle_id: "com.example.Game".into(),
        };
        let (_, args) = build_command(&target, Path::new("/tmp/Game.app")).unwrap();
        let script = &args[1];
        assert!(script.contains("devicectl device install"));
        assert!(script.contains("--device 00008110-001234"));
        assert!(script.contains("devicectl device process launch"));
        assert!(script.contains("com.example.Game"));
    }

    #[test]
    fn android_command_chains_install_start_logcat() {
        let target = MobileTarget::Android {
            serial: "R5CTC0ABC123".into(),
            package_name: "com.example.game".into(),
            activity: "MainActivity".into(),
        };
        let (_, args) = build_command(&target, Path::new("/tmp/game.apk")).unwrap();
        let script = &args[1];
        assert!(script.contains("adb -s R5CTC0ABC123 install -r '/tmp/game.apk'"));
        assert!(
            script.contains("am start -W -n com.example.game/.MainActivity"),
            "script: {script}"
        );
        assert!(script.contains("logcat"));
        assert!(script.contains("pidof -s com.example.game"));
    }

    #[test]
    fn android_activity_with_explicit_package_passes_through() {
        let target = MobileTarget::Android {
            serial: "S".into(),
            package_name: "com.example".into(),
            activity: "com.example.MainActivity".into(),
        };
        let (_, args) = build_command(&target, Path::new("/tmp/g.apk")).unwrap();
        let script = &args[1];
        // Already explicit — runner builds `pkg/com.example.MainActivity`
        // (the explicit path is preserved).
        assert!(
            script.contains("am start -W -n com.example/com.example.MainActivity"),
            "script: {script}"
        );
    }

    #[test]
    fn target_label_is_short() {
        let t = MobileTarget::IosSim {
            udid: "12345678-aaaa-bbbb-cccc-dddd".into(),
            bundle_id: "x".into(),
        };
        let l = t.label();
        // Truncates UDID with an ellipsis, doesn't show the full thing.
        assert!(l.contains("iOS Sim"));
        assert!(l.contains("…"));
    }
}
