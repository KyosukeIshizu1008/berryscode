use std::path::{Path, PathBuf};
use std::sync::mpsc;
use notify::{RecommendedWatcher, RecursiveMode, Watcher, Event, EventKind};

/// File system event types
#[derive(Debug, Clone)]
pub enum FileEvent {
    Created(PathBuf),
    Modified(PathBuf),
    Removed(PathBuf),
    Renamed { from: PathBuf, to: PathBuf },
}

/// Watches a directory for file system changes using the `notify` crate
pub struct FileWatcher {
    _watcher: RecommendedWatcher,
    rx: mpsc::Receiver<FileEvent>,
}

impl FileWatcher {
    /// Create a new FileWatcher. Events can be polled via `try_recv()`.
    pub fn new() -> anyhow::Result<Self> {
        let (tx, rx) = mpsc::channel();

        let watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            match res {
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
            }
        })?;

        Ok(Self {
            _watcher: watcher,
            rx,
        })
    }

    /// Start watching a directory recursively
    pub fn watch(&mut self, path: &str) -> anyhow::Result<()> {
        self._watcher.watch(Path::new(path), RecursiveMode::Recursive)?;
        Ok(())
    }

    /// Try to receive a file event without blocking. Returns `None` if no events are pending.
    pub fn try_recv(&mut self) -> Option<FileEvent> {
        self.rx.try_recv().ok()
    }
}
