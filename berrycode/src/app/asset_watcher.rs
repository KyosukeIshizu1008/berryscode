//! Filesystem watcher for project asset hot-reload.
//!
//! Watches `<project>/assets` and `<project>/scenes` recursively for
//! changes to scene (`.bscene`, `.scn.ron`) and shader (`.wgsl`,
//! `.glsl`, `.vert`, `.frag`) files. Events are coalesced into
//! [`AssetEvent`] values and delivered through an mpsc channel that
//! the main UI thread drains every frame.
//!
//! Distinct from [`super::scene_editor::hot_reload::HotReloadState`],
//! which spawns `cargo build` on `.rs` changes and is conceptually
//! "rebuild the binary". This module is "the binary is already
//! running, just refresh assets".
//!
//! v0.5 / Hot reload roadmap item.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::{Duration, Instant};

use notify::{RecursiveMode, Watcher};

/// One change classified by file kind so the UI layer doesn't have to
/// re-do the suffix check.
#[derive(Debug, Clone)]
pub enum AssetEvent {
    /// `.bscene` or `.scn.ron` file was modified.
    SceneChanged(PathBuf),
    /// Shader source (`.wgsl` / `.glsl` / `.vert` / `.frag`) was modified.
    ShaderChanged(PathBuf),
    /// Audio source (`.wav` / `.ogg` / `.mp3` / `.flac`) was modified.
    /// v0.6 / Phase F. The handler re-runs the waveform decode for the
    /// active preview and asks Bevy's asset server to reload any sink
    /// pointing at the same path.
    AudioChanged(PathBuf),
}

/// Background filesystem watcher. The recommended watcher impl is
/// platform-specific (FSEvents on macOS, inotify on Linux, ReadDirectoryChangesW
/// on Windows) and is created lazily when the user opens a project.
pub struct AssetWatcher {
    watcher: Option<notify::RecommendedWatcher>,
    rx: Option<Receiver<AssetEvent>>,
    /// Where we're currently watching (the project root). Used to
    /// detect "user switched projects" and re-start the watcher.
    watched_root: Option<PathBuf>,
    /// Per-path debounce — file-system writes often arrive as several
    /// events within a few ms (truncate, write chunks, close). We
    /// suppress repeats within `DEBOUNCE_MS` of the last accepted
    /// event for the same path.
    last_event_at: std::collections::HashMap<PathBuf, Instant>,
}

const DEBOUNCE_MS: u128 = 250;

impl Default for AssetWatcher {
    fn default() -> Self {
        Self {
            watcher: None,
            rx: None,
            watched_root: None,
            last_event_at: std::collections::HashMap::new(),
        }
    }
}

impl AssetWatcher {
    /// Make sure the watcher is running on `<root>/assets` and
    /// `<root>/scenes`. Idempotent: cheap to call on every frame
    /// while the project is open.
    pub fn ensure_watching(&mut self, root: &Path) {
        let target = root.to_path_buf();
        if self.watched_root.as_ref() == Some(&target) && self.watcher.is_some() {
            return;
        }
        self.start(target);
    }

    /// Stop watching. Called on project close or shutdown.
    pub fn stop(&mut self) {
        self.watcher = None;
        self.rx = None;
        self.watched_root = None;
        self.last_event_at.clear();
    }

    fn start(&mut self, root: PathBuf) {
        // Drop any previous watcher first so the OS handle is released
        // before we try to register new paths.
        self.stop();

        let (tx, rx) = std::sync::mpsc::channel::<AssetEvent>();

        // notify's callback runs on a worker thread; we map raw events
        // to our typed `AssetEvent` and drop everything else.
        let mut watcher = match notify::RecommendedWatcher::new(
            move |res: notify::Result<notify::Event>| {
                let Ok(event) = res else {
                    return;
                };
                if !matches!(
                    event.kind,
                    notify::EventKind::Modify(_) | notify::EventKind::Create(_)
                ) {
                    return;
                }
                for path in event.paths {
                    if let Some(typed) = classify(&path) {
                        let _ = tx.send(typed);
                    }
                }
            },
            notify::Config::default(),
        ) {
            Ok(w) => w,
            Err(e) => {
                tracing::warn!("AssetWatcher: failed to create watcher: {e}");
                return;
            }
        };

        // Watch each directory if it actually exists. Missing
        // directories silently skip — fresh projects don't ship a
        // `scenes/` until the user makes one, and that's fine.
        for sub in ["assets", "scenes"] {
            let p = root.join(sub);
            if p.is_dir() {
                if let Err(e) = watcher.watch(&p, RecursiveMode::Recursive) {
                    tracing::warn!("AssetWatcher: failed to watch {}: {}", p.display(), e);
                }
            }
        }

        self.watcher = Some(watcher);
        self.rx = Some(rx);
        self.watched_root = Some(root);
    }

    /// Drain pending events, applying debounce. Returns up to a small
    /// burst per call so the UI doesn't get hammered when a code
    /// generator writes 100 files in a row.
    pub fn drain(&mut self) -> Vec<AssetEvent> {
        let Some(rx) = self.rx.as_ref() else {
            return Vec::new();
        };
        let mut out = Vec::new();
        let now = Instant::now();
        // Cap to avoid pathological loops; UI gets the rest next frame.
        for _ in 0..32 {
            match rx.try_recv() {
                Ok(ev) => {
                    let path = match &ev {
                        AssetEvent::SceneChanged(p)
                        | AssetEvent::ShaderChanged(p)
                        | AssetEvent::AudioChanged(p) => p.clone(),
                    };
                    let recent = self
                        .last_event_at
                        .get(&path)
                        .map(|prev| {
                            now.duration_since(*prev) < Duration::from_millis(DEBOUNCE_MS as u64)
                        })
                        .unwrap_or(false);
                    if !recent {
                        self.last_event_at.insert(path, now);
                        out.push(ev);
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    // Watcher thread died — surface a single warning
                    // and let the caller restart on the next frame
                    // via `ensure_watching`.
                    tracing::warn!("AssetWatcher: channel disconnected; will be restarted");
                    self.stop();
                    break;
                }
            }
        }
        out
    }
}

fn classify(path: &Path) -> Option<AssetEvent> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "bscene" => Some(AssetEvent::SceneChanged(path.to_path_buf())),
        "ron" => {
            // Only `.scn.ron` (Bevy scene RON) is interesting here; the
            // project's general .ron files (config / save data) aren't
            // hot-reload material, and reacting to them would generate
            // noise for normal saves.
            let s = path.to_string_lossy();
            if s.ends_with(".scn.ron") {
                Some(AssetEvent::SceneChanged(path.to_path_buf()))
            } else {
                None
            }
        }
        "wgsl" | "glsl" | "vert" | "frag" => Some(AssetEvent::ShaderChanged(path.to_path_buf())),
        "wav" | "ogg" | "mp3" | "flac" => Some(AssetEvent::AudioChanged(path.to_path_buf())),
        _ => None,
    }
}
