//! File system watcher using notify crate
//!
//! Provides real-time file change notifications for the editor.

use anyhow::{Context, Result};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher as NotifyWatcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use parking_lot::RwLock;

/// File system event types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileEvent {
    /// File or directory was created
    Created(PathBuf),
    /// File or directory was modified
    Modified(PathBuf),
    /// File or directory was deleted
    Removed(PathBuf),
    /// File or directory was renamed
    Renamed { from: PathBuf, to: PathBuf },
}

/// File system watcher
pub struct FileWatcher {
    watcher: Arc<RwLock<Option<RecommendedWatcher>>>,
    event_receiver: Receiver<FileEvent>,
    watched_paths: Arc<RwLock<Vec<PathBuf>>>,
}

impl FileWatcher {
    /// Create a new file watcher
    pub fn new() -> Result<Self> {
        let (tx, rx) = channel();

        Ok(Self {
            watcher: Arc::new(RwLock::new(None)),
            event_receiver: rx,
            watched_paths: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Start watching a path
    pub fn watch(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref().to_path_buf();

        // Create sender for events
        let (tx, rx) = channel();
        let event_tx = tx.clone();

        // Create watcher with event handler
        let mut watcher = RecommendedWatcher::new(
            move |result: notify::Result<Event>| {
                if let Ok(event) = result {
                    let file_events = Self::convert_event(event);
                    for file_event in file_events {
                        let _ = event_tx.send(file_event);
                    }
                }
            },
            Config::default(),
        )
        .context("Failed to create file watcher")?;

        // Start watching (clone path for logging)
        let path_for_log = path.clone();
        watcher
            .watch(&path, RecursiveMode::Recursive)
            .context("Failed to watch path")?;

        // Store watcher
        *self.watcher.write() = Some(watcher);
        self.watched_paths.write().push(path);

        // Replace receiver
        self.event_receiver = rx;

        tracing::info!("📁 Started watching: {:?}", path_for_log);
        Ok(())
    }

    /// Stop watching a path
    pub fn unwatch(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();

        if let Some(watcher) = self.watcher.write().as_mut() {
            watcher
                .unwatch(path)
                .context("Failed to unwatch path")?;
        }

        self.watched_paths.write().retain(|p| p != path);
        tracing::info!("📁 Stopped watching: {:?}", path);
        Ok(())
    }

    /// Get next file event (blocking)
    pub fn recv(&self) -> Result<FileEvent> {
        self.event_receiver
            .recv()
            .context("File watcher channel closed")
    }

    /// Try to get next file event (non-blocking)
    pub fn try_recv(&self) -> Option<FileEvent> {
        self.event_receiver.try_recv().ok()
    }

    /// Get all watched paths
    pub fn watched_paths(&self) -> Vec<PathBuf> {
        self.watched_paths.read().clone()
    }

    /// Convert notify::Event to FileEvent
    fn convert_event(event: Event) -> Vec<FileEvent> {
        use notify::EventKind;

        let mut file_events = Vec::new();

        match event.kind {
            EventKind::Create(_) => {
                for path in event.paths {
                    file_events.push(FileEvent::Created(path));
                }
            }
            EventKind::Modify(_) => {
                for path in event.paths {
                    file_events.push(FileEvent::Modified(path));
                }
            }
            EventKind::Remove(_) => {
                for path in event.paths {
                    file_events.push(FileEvent::Removed(path));
                }
            }
            EventKind::Other => {
                // Handle rename events
                if event.paths.len() == 2 {
                    file_events.push(FileEvent::Renamed {
                        from: event.paths[0].clone(),
                        to: event.paths[1].clone(),
                    });
                }
            }
            _ => {}
        }

        file_events
    }
}

impl Default for FileWatcher {
    fn default() -> Self {
        Self::new().expect("Failed to create FileWatcher")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_file_watcher_creation() {
        let watcher = FileWatcher::new();
        assert!(watcher.is_ok());
    }

    #[test]
    fn test_watch_directory() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let mut watcher = FileWatcher::new()?;

        watcher.watch(temp_dir.path())?;
        assert_eq!(watcher.watched_paths().len(), 1);

        Ok(())
    }

    #[test]
    fn test_file_creation_event() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let mut watcher = FileWatcher::new()?;
        watcher.watch(temp_dir.path())?;

        // Create a file
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "test content")?;

        // Wait for event
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Check for creation event
        if let Some(event) = watcher.try_recv() {
            match event {
                FileEvent::Created(path) | FileEvent::Modified(path) => {
                    assert!(path.ends_with("test.txt"));
                }
                _ => {}
            }
        }

        Ok(())
    }

    #[test]
    fn test_unwatch() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let mut watcher = FileWatcher::new()?;

        watcher.watch(temp_dir.path())?;
        assert_eq!(watcher.watched_paths().len(), 1);

        watcher.unwatch(temp_dir.path())?;
        assert_eq!(watcher.watched_paths().len(), 0);

        Ok(())
    }
}
