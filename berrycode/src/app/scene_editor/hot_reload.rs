//! Hot reload: watch src/**/*.rs for changes, debounce, and trigger cargo build.

use std::time::Instant;

/// Hot reload state stored on `BerryCodeApp`.
#[derive(Debug)]
pub struct HotReloadState {
    pub watching: bool,
    pub last_build_status: Option<String>,
    pub build_process: Option<std::process::Child>,
    pub build_output_rx: Option<std::sync::mpsc::Receiver<String>>,
    pub last_change_time: Option<Instant>,
    pub debounce_ms: u64,
}

impl Default for HotReloadState {
    fn default() -> Self {
        Self {
            watching: false,
            last_build_status: None,
            build_process: None,
            build_output_rx: None,
            last_change_time: None,
            debounce_ms: 500,
        }
    }
}

impl HotReloadState {
    /// Check if a debounced build should be triggered and spawn it.
    pub fn poll(&mut self, root_path: &str) -> Option<String> {
        // If a build is running, check if it finished.
        if let Some(ref mut child) = self.build_process {
            match child.try_wait() {
                Ok(Some(status)) => {
                    let msg = if status.success() {
                        "Hot reload: build succeeded".to_string()
                    } else {
                        format!(
                            "Hot reload: build failed (exit {})",
                            status.code().unwrap_or(-1)
                        )
                    };
                    self.last_build_status = Some(msg.clone());
                    self.build_process = None;

                    // Drain output
                    if let Some(ref rx) = self.build_output_rx {
                        while let Ok(line) = rx.try_recv() {
                            let _ = line;
                        }
                    }
                    self.build_output_rx = None;
                    return Some(msg);
                }
                Ok(None) => { /* still running */ }
                Err(_) => {
                    self.build_process = None;
                    self.build_output_rx = None;
                }
            }
            return None;
        }

        // If watching and a change was detected, check debounce.
        if let Some(change_time) = self.last_change_time {
            if change_time.elapsed().as_millis() as u64 >= self.debounce_ms {
                self.last_change_time = None;
                self.start_build(root_path);
            }
        }

        None
    }

    /// Record that a .rs file was modified (sets debounce timer).
    pub fn notify_change(&mut self) {
        if self.watching && self.build_process.is_none() {
            self.last_change_time = Some(Instant::now());
        }
    }

    /// Spawn `cargo build` in the project root.
    fn start_build(&mut self, root_path: &str) {
        let (tx, rx) = std::sync::mpsc::channel();
        let root = root_path.to_string();

        match std::process::Command::new("cargo")
            .arg("build")
            .current_dir(&root)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
        {
            Ok(mut child) => {
                // Capture stderr in a thread
                if let Some(stderr) = child.stderr.take() {
                    let tx_clone = tx.clone();
                    std::thread::spawn(move || {
                        use std::io::BufRead;
                        let reader = std::io::BufReader::new(stderr);
                        for line in reader.lines() {
                            if let Ok(line) = line {
                                let _ = tx_clone.send(line);
                            }
                        }
                    });
                }
                self.build_process = Some(child);
                self.build_output_rx = Some(rx);
                self.last_build_status = Some("Building...".to_string());
            }
            Err(e) => {
                self.last_build_status = Some(format!("Failed to start build: {}", e));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_not_watching() {
        let state = HotReloadState::default();
        assert!(!state.watching);
        assert!(state.last_build_status.is_none());
    }

    #[test]
    fn notify_sets_timer_when_watching() {
        let mut state = HotReloadState::default();
        state.watching = true;
        state.notify_change();
        assert!(state.last_change_time.is_some());
    }

    #[test]
    fn notify_ignored_when_not_watching() {
        let mut state = HotReloadState::default();
        state.notify_change();
        assert!(state.last_change_time.is_none());
    }
}
