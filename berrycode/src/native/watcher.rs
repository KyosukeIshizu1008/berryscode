use notify::{Config, Event, EventKind, PollWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

/// File system event types
#[derive(Debug, Clone)]
pub enum FileEvent {
    Created(PathBuf),
    Modified(PathBuf),
    Removed(PathBuf),
    Renamed { from: PathBuf, to: PathBuf },
}

/// Watches a directory for file system changes.
///
/// Uses `PollWatcher` (mtime-based polling at 500 ms) instead of
/// `RecommendedWatcher` because the OS-native backend on Windows
/// (`ReadDirectoryChangesW`) misses edits made by editors that save
/// via "write to temp + rename" — Notepad in particular doesn't fire
/// the Modify events we need (issue #17). Polling is slightly heavier
/// but consistent across macOS / Linux / Windows.
pub struct FileWatcher {
    _watcher: PollWatcher,
    rx: mpsc::Receiver<FileEvent>,
}

impl FileWatcher {
    /// Create a new FileWatcher. Events can be polled via `try_recv()`.
    pub fn new() -> anyhow::Result<Self> {
        let (tx, rx) = mpsc::channel();

        let config = Config::default()
            .with_poll_interval(Duration::from_millis(500))
            .with_compare_contents(false);

        let watcher = PollWatcher::new(
            move |res: Result<Event, notify::Error>| match res {
                Ok(event) => {
                    let paths = event.paths;
                    match event.kind {
                        EventKind::Create(_) => {
                            for p in paths {
                                let _ = tx.send(FileEvent::Created(p));
                            }
                        }
                        EventKind::Modify(_) => {
                            for p in paths {
                                let _ = tx.send(FileEvent::Modified(p));
                            }
                        }
                        EventKind::Remove(_) => {
                            for p in paths {
                                let _ = tx.send(FileEvent::Removed(p));
                            }
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    tracing::warn!("File watcher error: {}", e);
                }
            },
            config,
        )?;

        Ok(Self {
            _watcher: watcher,
            rx,
        })
    }

    /// Start watching a directory recursively
    pub fn watch(&mut self, path: &str) -> anyhow::Result<()> {
        self._watcher
            .watch(Path::new(path), RecursiveMode::Recursive)?;
        Ok(())
    }

    /// Try to receive a file event without blocking. Returns `None` if no events are pending.
    pub fn try_recv(&mut self) -> Option<FileEvent> {
        self.rx.try_recv().ok()
    }
}
