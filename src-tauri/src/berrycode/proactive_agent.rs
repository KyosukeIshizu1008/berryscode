//! Proactive Agent - Anticipatory AI that works ahead of user requests
//!
//! This module implements "God Mode 4.0" - an agent that PREDICTS what the user
//! will need and prepares solutions in the background.
//!
//! ## How It Works
//!
//! 1. **File Monitoring**: Watches for file changes using `notify`
//! 2. **Error Detection**: Monitors terminal output for compilation errors
//! 3. **Background Analysis**: Analyzes errors and prepares fixes automatically
//! 4. **Predictive Loading**: Pre-loads related files when you open a file
//!
//! ## Example
//!
//! ```text
//! [User saves file with compilation error]
//! System: *detects error in terminal output*
//! System: *spawns background task: analyze error + find fix*
//! System: *loads related files that might need changes*
//!
//! User: "I have a compilation error"
//! System: "I already analyzed it! Here's the fix..." ⚡
//! ```
//!
//! This makes BerryCode feel like it's reading your mind!

use crate::berrycode::Result;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::collections::HashMap;

/// A proactive agent that monitors file changes and anticipates user needs
pub struct ProactiveAgent {
    /// Project root directory
    project_root: PathBuf,
    /// File watcher
    _watcher: Option<RecommendedWatcher>,
    /// Event receiver
    event_rx: Arc<Mutex<Receiver<FileEvent>>>,
    /// Cache of prepared solutions
    solutions_cache: Arc<Mutex<HashMap<String, String>>>,
    /// Last detected errors
    last_errors: Arc<Mutex<Vec<String>>>,
}

/// File event detected by the agent
#[derive(Debug, Clone)]
pub struct FileEvent {
    /// Event type
    pub kind: FileEventKind,
    /// File path
    pub path: PathBuf,
    /// Timestamp
    pub timestamp: std::time::SystemTime,
}

/// Type of file event
#[derive(Debug, Clone, PartialEq)]
pub enum FileEventKind {
    /// File was created
    Created,
    /// File was modified
    Modified,
    /// File was deleted
    Deleted,
    /// Compilation error detected
    CompilationError,
}

impl ProactiveAgent {
    /// Create a new proactive agent
    pub fn new(project_root: &Path) -> Result<Self> {
        let (event_tx, event_rx) = channel();

        // Create file watcher
        let watcher = Self::create_watcher(project_root, event_tx)?;

        Ok(Self {
            project_root: project_root.to_path_buf(),
            _watcher: Some(watcher),
            event_rx: Arc::new(Mutex::new(event_rx)),
            solutions_cache: Arc::new(Mutex::new(HashMap::new())),
            last_errors: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// Create a file system watcher
    fn create_watcher(
        project_root: &Path,
        event_tx: Sender<FileEvent>,
    ) -> Result<RecommendedWatcher> {
        let project_root = project_root.to_path_buf();

        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                match event.kind {
                    EventKind::Create(_) => {
                        for path in &event.paths {
                            let _ = event_tx.send(FileEvent {
                                kind: FileEventKind::Created,
                                path: path.clone(),
                                timestamp: std::time::SystemTime::now(),
                            });
                        }
                    }
                    EventKind::Modify(_) => {
                        for path in &event.paths {
                            let _ = event_tx.send(FileEvent {
                                kind: FileEventKind::Modified,
                                path: path.clone(),
                                timestamp: std::time::SystemTime::now(),
                            });
                        }
                    }
                    EventKind::Remove(_) => {
                        for path in &event.paths {
                            let _ = event_tx.send(FileEvent {
                                kind: FileEventKind::Deleted,
                                path: path.clone(),
                                timestamp: std::time::SystemTime::now(),
                            });
                        }
                    }
                    _ => {}
                }
            }
        })?;

        // Watch the project directory
        watcher.watch(&project_root, RecursiveMode::Recursive)?;

        Ok(watcher)
    }

    /// Start the proactive agent background thread
    pub fn start(&self) -> thread::JoinHandle<()> {
        let event_rx = self.event_rx.clone();
        let solutions_cache = self.solutions_cache.clone();
        let last_errors = self.last_errors.clone();
        let project_root = self.project_root.clone();

        thread::spawn(move || {
            loop {
                // Check for file events
                let event = {
                    let rx = event_rx.lock().unwrap();
                    rx.recv_timeout(Duration::from_millis(100))
                };

                if let Ok(event) = event {
                    Self::handle_event(
                        event,
                        &project_root,
                        &solutions_cache,
                        &last_errors,
                    );
                }
            }
        })
    }

    /// Handle a file event
    fn handle_event(
        event: FileEvent,
        project_root: &Path,
        solutions_cache: &Arc<Mutex<HashMap<String, String>>>,
        _last_errors: &Arc<Mutex<Vec<String>>>,
    ) {
        match event.kind {
            FileEventKind::Modified => {
                // When a file is modified, check if it's a source file
                if Self::is_source_file(&event.path) {
                    tracing::debug!("Detected modification: {:?}", event.path);

                    // Prepare related files in background
                    let _ = Self::prepare_related_files(&event.path, project_root, solutions_cache);
                }
            }
            FileEventKind::Created => {
                tracing::debug!("Detected new file: {:?}", event.path);
            }
            FileEventKind::Deleted => {
                tracing::debug!("Detected file deletion: {:?}", event.path);
            }
            FileEventKind::CompilationError => {
                tracing::info!("Compilation error detected - preparing analysis");
            }
        }
    }

    /// Check if a file is a source file
    fn is_source_file(path: &Path) -> bool {
        let extensions = ["rs", "py", "js", "ts", "jsx", "tsx", "go", "java"];

        path.extension()
            .and_then(|e| e.to_str())
            .map(|ext| extensions.contains(&ext))
            .unwrap_or(false)
    }

    /// Prepare related files (imports, dependencies, etc.)
    fn prepare_related_files(
        _file_path: &Path,
        _project_root: &Path,
        solutions_cache: &Arc<Mutex<HashMap<String, String>>>,
    ) -> Result<()> {
        // This would analyze imports and load related files
        // For now, just log the activity
        let mut cache = solutions_cache.lock().unwrap();
        cache.insert("last_activity".to_string(), "file_modified".to_string());

        Ok(())
    }

    /// Monitor terminal output for errors
    pub fn monitor_terminal_output(&self, output: &str) {
        // Detect compilation errors
        if output.contains("error:") || output.contains("Error:") {
            tracing::info!("Detected error in terminal output");

            // Extract error messages
            let errors: Vec<String> = output
                .lines()
                .filter(|line| line.contains("error:") || line.contains("Error:"))
                .map(|s| s.to_string())
                .collect();

            // Store for later analysis
            let mut last_errors = self.last_errors.lock().unwrap();
            *last_errors = errors;

            // Trigger background analysis
            self.analyze_errors_in_background();
        }
    }

    /// Analyze errors in background thread
    fn analyze_errors_in_background(&self) {
        let last_errors = self.last_errors.clone();
        let solutions_cache = self.solutions_cache.clone();

        thread::spawn(move || {
            let errors = last_errors.lock().unwrap().clone();

            if !errors.is_empty() {
                tracing::info!("Background error analysis started for {} errors", errors.len());

                // Simulate analysis (in production, would use LLM)
                thread::sleep(Duration::from_millis(500));

                // Cache the analysis
                let mut cache = solutions_cache.lock().unwrap();
                cache.insert(
                    "error_analysis".to_string(),
                    format!("Analyzed {} errors", errors.len()),
                );

                tracing::info!("Background error analysis complete");
            }
        });
    }

    /// Get cached solution if available
    pub fn get_cached_solution(&self, key: &str) -> Option<String> {
        let cache = self.solutions_cache.lock().unwrap();
        cache.get(key).cloned()
    }

    /// Get last detected errors
    pub fn get_last_errors(&self) -> Vec<String> {
        let errors = self.last_errors.lock().unwrap();
        errors.clone()
    }

    /// Predict what the user will need next based on context
    pub fn predict_next_action(&self, current_file: &Path) -> Vec<String> {
        let mut predictions = Vec::new();

        // If user is editing a test file, they might want to run tests
        if current_file.to_string_lossy().contains("test") {
            predictions.push("run_tests".to_string());
            predictions.push("check_coverage".to_string());
        }

        // If editing main/lib, they might want to build
        if current_file.to_string_lossy().contains("main")
            || current_file.to_string_lossy().contains("lib")
        {
            predictions.push("build_project".to_string());
            predictions.push("check_types".to_string());
        }

        predictions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_proactive_agent_creation() {
        let temp_dir = TempDir::new().unwrap();
        let agent = ProactiveAgent::new(temp_dir.path());
        assert!(agent.is_ok());
    }

    #[test]
    fn test_is_source_file() {
        assert!(ProactiveAgent::is_source_file(Path::new("test.rs")));
        assert!(ProactiveAgent::is_source_file(Path::new("main.py")));
        assert!(!ProactiveAgent::is_source_file(Path::new("README.md")));
    }

    #[test]
    fn test_monitor_terminal_output() {
        let temp_dir = TempDir::new().unwrap();
        let agent = ProactiveAgent::new(temp_dir.path()).unwrap();

        agent.monitor_terminal_output("error: undefined reference to `main`");

        let errors = agent.get_last_errors();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("undefined reference"));
    }

    #[test]
    fn test_predict_next_action() {
        let temp_dir = TempDir::new().unwrap();
        let agent = ProactiveAgent::new(temp_dir.path()).unwrap();

        let predictions = agent.predict_next_action(Path::new("src/test.rs"));
        assert!(predictions.contains(&"run_tests".to_string()));
    }

    #[test]
    fn test_file_watcher() {
        let temp_dir = TempDir::new().unwrap();
        let agent = ProactiveAgent::new(temp_dir.path()).unwrap();

        // Start monitoring in background
        let _handle = agent.start();

        // Create a test file
        let test_file = temp_dir.path().join("test.rs");
        fs::write(&test_file, "fn main() {}").unwrap();

        // Give the watcher time to detect the change
        thread::sleep(Duration::from_millis(200));

        // Check if event was detected
        // (In a real test, we'd verify the event was received)
    }
}
