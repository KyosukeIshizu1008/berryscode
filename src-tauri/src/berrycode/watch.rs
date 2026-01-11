//! File watching for aider

use std::path::PathBuf;
use std::sync::mpsc::channel;
use notify::{Watcher, RecursiveMode, recommended_watcher, Event, EventKind};
use crate::berrycode::Result;

pub struct FileWatcher {
    paths: Vec<PathBuf>,
}

impl FileWatcher {
    pub fn new(paths: Vec<PathBuf>) -> Result<Self> {
        Ok(Self { paths })
    }

    pub fn watch(&mut self) -> Result<()> {
        let (tx, rx) = channel();

        let mut watcher = recommended_watcher(move |res: std::result::Result<Event, notify::Error>| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        })?;

        for path in &self.paths {
            watcher.watch(path, RecursiveMode::NonRecursive)?;
        }

        println!("Watching for file changes...");
        println!("Press Ctrl+C to stop");

        loop {
            match rx.recv() {
                Ok(event) => {
                    println!("File changed: {:?}", event);
                }
                Err(e) => {
                    eprintln!("Watch error: {:?}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    pub fn watch_with_callback<F>(&mut self, mut callback: F) -> Result<()>
    where
        F: FnMut(PathBuf) + Send + 'static,
    {
        let (tx, rx) = channel();

        let mut watcher = recommended_watcher(move |res: std::result::Result<Event, notify::Error>| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        })?;

        for path in &self.paths {
            watcher.watch(path, RecursiveMode::NonRecursive)?;
        }

        loop {
            match rx.recv() {
                Ok(event) => {
                    if matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                        for path in event.paths {
                            callback(path);
                        }
                    }
                }
                Err(_) => break,
            }
        }

        Ok(())
    }
}
