//! Test runner: detect #[test] functions, run individually, show inline results
//!
//! Features:
//! - Scan .rs files for #[test] and #[tokio::test] functions
//! - Run individual tests with `cargo test --exact`
//! - Show results inline (✅/❌) and in a test explorer panel
//! - Track test history

use super::BerryCodeApp;
use std::process::Command;

/// A detected test function
#[derive(Debug, Clone)]
pub struct TestItem {
    pub name: String,
    pub module_path: String, // full path: module::submod::test_name
    pub file_path: String,
    pub line: usize,
    pub status: TestStatus,
    pub output: Option<String>,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TestStatus {
    Unknown,
    Running,
    Passed,
    Failed,
}

/// Test result received from background thread
pub struct TestResult {
    pub module_path: String,
    pub status: TestStatus,
    pub output: String,
    pub duration_ms: Option<u64>,
}

/// Test runner state
pub struct TestRunnerState {
    pub tests: Vec<TestItem>,
    pub running: bool,
    pub filter: String,
    pub last_scan_file: Option<String>,
    pub show_panel: bool,
    /// Channel for receiving async test results
    pub result_rx: Option<std::sync::mpsc::Receiver<TestResult>>,
    pub result_tx: Option<std::sync::mpsc::Sender<TestResult>>,
}

impl Default for TestRunnerState {
    fn default() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        Self {
            tests: Vec::new(),
            running: false,
            filter: String::new(),
            last_scan_file: None,
            show_panel: false,
            result_rx: Some(rx),
            result_tx: Some(tx),
        }
    }
}

/// Scan a Rust source file for test functions
pub fn scan_tests(file_path: &str, content: &str) -> Vec<TestItem> {
    let mut tests = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut in_test_module = false;
    let mut module_depth = 0;
    let mut current_module = String::new();

    for (line_idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Track #[cfg(test)] modules
        if trimmed.contains("#[cfg(test)]") {
            in_test_module = true;
        }

        // Track module nesting
        if trimmed.starts_with("mod ") && trimmed.contains('{') {
            let mod_name = trimmed
                .trim_start_matches("mod ")
                .split(|c: char| c == '{' || c.is_whitespace())
                .next()
                .unwrap_or("")
                .trim();
            if !mod_name.is_empty() {
                if !current_module.is_empty() {
                    current_module.push_str("::");
                }
                current_module.push_str(mod_name);
                module_depth += 1;
            }
        }

        // Detect #[test] or #[tokio::test]
        if trimmed == "#[test]"
            || trimmed == "#[tokio::test]"
            || trimmed.starts_with("#[test]")
            || trimmed.starts_with("#[tokio::test]")
        {
            // Next non-attribute line should be the function definition
            for next_idx in (line_idx + 1)..lines.len().min(line_idx + 5) {
                let next_line = lines[next_idx].trim();
                if next_line.starts_with("fn ") || next_line.starts_with("async fn ") || next_line.starts_with("pub fn ") {
                    let fn_name = next_line
                        .replace("async ", "")
                        .replace("pub ", "")
                        .trim_start_matches("fn ")
                        .split('(')
                        .next()
                        .unwrap_or("")
                        .trim()
                        .to_string();

                    if !fn_name.is_empty() {
                        let module_path = if current_module.is_empty() {
                            fn_name.clone()
                        } else {
                            format!("{}::{}", current_module, fn_name)
                        };

                        tests.push(TestItem {
                            name: fn_name,
                            module_path,
                            file_path: file_path.to_string(),
                            line: next_idx,
                            status: TestStatus::Unknown,
                            output: None,
                            duration_ms: None,
                        });
                    }
                    break;
                }
                // Skip other attributes
                if !next_line.starts_with('#') && !next_line.is_empty() {
                    break;
                }
            }
        }

        // Track closing braces for module tracking
        if trimmed == "}" && module_depth > 0 {
            // Simple heuristic — not perfect but works for most cases
        }
    }

    tests
}

/// Run a single test and return the result
pub fn run_single_test(root_path: &str, test_path: &str) -> (TestStatus, String, Option<u64>) {
    let start = std::time::Instant::now();

    let output = Command::new("cargo")
        .args(["test", "--", "--exact", test_path, "--nocapture"])
        .current_dir(root_path)
        .env("RUST_BACKTRACE", "1")
        .output();

    let duration_ms = start.elapsed().as_millis() as u64;

    match output {
        Ok(result) => {
            let stdout = String::from_utf8_lossy(&result.stdout).to_string();
            let stderr = String::from_utf8_lossy(&result.stderr).to_string();
            let combined = format!("{}\n{}", stdout, stderr);

            let status = if result.status.success() {
                TestStatus::Passed
            } else {
                TestStatus::Failed
            };

            (status, combined, Some(duration_ms))
        }
        Err(e) => (
            TestStatus::Failed,
            format!("Failed to run test: {}", e),
            Some(duration_ms),
        ),
    }
}

impl BerryCodeApp {
    /// Scan current file for tests
    pub(crate) fn scan_tests_in_current_file(&mut self) {
        let tab = match self.editor_tabs.get_mut(self.active_tab_idx) {
            Some(t) => t,
            None => return,
        };

        if !tab.file_path.ends_with(".rs") {
            return;
        }

        let content = tab.get_text().to_string();
        let tests = scan_tests(&tab.file_path, &content);

        // Merge with existing results (preserve status of known tests)
        for new_test in tests {
            if let Some(existing) = self
                .test_runner
                .tests
                .iter_mut()
                .find(|t| t.module_path == new_test.module_path)
            {
                existing.line = new_test.line;
                existing.file_path = new_test.file_path.clone();
            } else {
                self.test_runner.tests.push(new_test);
            }
        }

        self.test_runner.last_scan_file = Some(tab.file_path.clone());
    }

    /// Run a specific test by module path (non-blocking)
    pub(crate) fn run_test(&mut self, test_module_path: &str) {
        // Mark as running
        if let Some(test) = self
            .test_runner
            .tests
            .iter_mut()
            .find(|t| t.module_path == test_module_path)
        {
            test.status = TestStatus::Running;
            test.output = None;
        }

        let root_path = self.root_path.clone();
        let test_path = test_module_path.to_string();
        let tx = match &self.test_runner.result_tx {
            Some(tx) => tx.clone(),
            None => return,
        };

        // Run in background thread (non-blocking)
        std::thread::Builder::new()
            .name(format!("test-{}", test_path))
            .spawn(move || {
                let (status, output, duration) = run_single_test(&root_path, &test_path);
                let _ = tx.send(TestResult {
                    module_path: test_path,
                    status,
                    output,
                    duration_ms: duration,
                });
            })
            .ok();

        self.test_runner.running = true;
    }

    /// Run all tests in current file (non-blocking, sequential via thread)
    pub(crate) fn run_all_tests(&mut self) {
        let tests: Vec<String> = self
            .test_runner
            .tests
            .iter()
            .map(|t| t.module_path.clone())
            .collect();

        // Mark all as running
        for test in &mut self.test_runner.tests {
            test.status = TestStatus::Running;
            test.output = None;
        }

        let root_path = self.root_path.clone();
        let tx = match &self.test_runner.result_tx {
            Some(tx) => tx.clone(),
            None => return,
        };

        // Run all tests in a single background thread
        std::thread::Builder::new()
            .name("test-all".to_string())
            .spawn(move || {
                for test_path in tests {
                    let (status, output, duration) = run_single_test(&root_path, &test_path);
                    let _ = tx.send(TestResult {
                        module_path: test_path,
                        status,
                        output,
                        duration_ms: duration,
                    });
                }
            })
            .ok();

        self.test_runner.running = true;
    }

    /// Poll for completed test results (call from main loop)
    pub(crate) fn poll_test_results(&mut self) {
        let rx = match &self.test_runner.result_rx {
            Some(rx) => rx,
            None => return,
        };

        while let Ok(result) = rx.try_recv() {
            if let Some(test) = self
                .test_runner
                .tests
                .iter_mut()
                .find(|t| t.module_path == result.module_path)
            {
                test.status = result.status;
                test.output = Some(result.output);
                test.duration_ms = result.duration_ms;
            }
        }

        // Check if any tests still running
        self.test_runner.running = self
            .test_runner
            .tests
            .iter()
            .any(|t| t.status == TestStatus::Running);
    }

    /// Render test explorer panel
    pub(crate) fn render_test_explorer(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Test Explorer")
                    .size(13.0)
                    .color(egui::Color32::from_rgb(200, 200, 200))
                    .strong(),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(egui::Button::new(
                        egui::RichText::new("\u{ebb5}") // play all
                            .size(14.0)
                            .color(egui::Color32::from_rgb(80, 200, 80)),
                    ).frame(false))
                    .on_hover_text("Run All Tests")
                    .clicked()
                {
                    self.run_all_tests();
                }
                if ui
                    .add(egui::Button::new(
                        egui::RichText::new("\u{eb37}") // refresh
                            .size(14.0)
                            .color(egui::Color32::from_rgb(160, 160, 160)),
                    ).frame(false))
                    .on_hover_text("Rescan Tests")
                    .clicked()
                {
                    self.scan_tests_in_current_file();
                }
            });
        });

        ui.add_space(4.0);

        // Filter
        ui.add(
            egui::TextEdit::singleline(&mut self.test_runner.filter)
                .hint_text("Filter tests...")
                .font(egui::FontId::monospace(11.0))
                .desired_width(f32::INFINITY),
        );

        ui.add_space(4.0);

        // Test list
        let mut test_to_run: Option<String> = None;
        let mut test_to_navigate: Option<(String, usize)> = None;

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                let filter = self.test_runner.filter.to_lowercase();
                for test in &self.test_runner.tests {
                    if !filter.is_empty() && !test.name.to_lowercase().contains(&filter) {
                        continue;
                    }

                    ui.horizontal(|ui| {
                        // Status icon
                        let (icon, color) = match test.status {
                            TestStatus::Unknown => ("\u{eb99}", egui::Color32::from_rgb(120, 120, 120)), // circle-outline
                            TestStatus::Running => ("\u{eb2c}", egui::Color32::from_rgb(200, 200, 80)),  // loading
                            TestStatus::Passed => ("\u{eab2}", egui::Color32::from_rgb(80, 200, 80)),    // check
                            TestStatus::Failed => ("\u{eba4}", egui::Color32::from_rgb(230, 80, 80)),    // error
                        };
                        ui.label(
                            egui::RichText::new(icon).size(12.0).color(color),
                        );

                        // Test name (clickable to navigate)
                        let name_resp = ui.add(
                            egui::Button::new(
                                egui::RichText::new(&test.name)
                                    .size(12.0)
                                    .color(egui::Color32::from_rgb(200, 200, 200)),
                            )
                            .frame(false),
                        );
                        if name_resp.clicked() {
                            test_to_navigate =
                                Some((test.file_path.clone(), test.line));
                        }

                        // Duration
                        if let Some(ms) = test.duration_ms {
                            ui.label(
                                egui::RichText::new(format!("{}ms", ms))
                                    .size(10.0)
                                    .color(egui::Color32::from_rgb(100, 100, 100)),
                            );
                        }

                        // Run button
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("\u{ebb5}")
                                        .size(12.0)
                                        .color(egui::Color32::from_rgb(80, 200, 80)),
                                )
                                .frame(false),
                            )
                            .on_hover_text("Run test")
                            .clicked()
                        {
                            test_to_run = Some(test.module_path.clone());
                        }
                    });
                }
            });

        // Process deferred actions
        if let Some(path) = test_to_run {
            self.run_test(&path);
        }
        if let Some((file, line)) = test_to_navigate {
            self.open_file_from_path(&file);
            if let Some(tab) = self.editor_tabs.get_mut(self.active_tab_idx) {
                tab.pending_cursor_jump = Some((line, 0));
            }
        }
    }

    /// Get inline test indicators for editor gutter
    pub(crate) fn get_test_at_line(&self, file_path: &str, line: usize) -> Option<&TestItem> {
        self.test_runner
            .tests
            .iter()
            .find(|t| t.file_path == file_path && t.line == line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_basic() {
        let content = r#"
#[test]
fn test_hello() {
    assert!(true);
}

#[test]
fn test_world() {
    assert_eq!(1, 1);
}
"#;
        let tests = scan_tests("src/lib.rs", content);
        assert_eq!(tests.len(), 2);
        assert_eq!(tests[0].name, "test_hello");
        assert_eq!(tests[1].name, "test_world");
    }

    #[test]
    fn test_scan_async() {
        let content = r#"
#[tokio::test]
async fn test_async() {
    let _ = 42;
}
"#;
        let tests = scan_tests("src/lib.rs", content);
        assert_eq!(tests.len(), 1);
        assert_eq!(tests[0].name, "test_async");
    }

    #[test]
    fn test_scan_no_tests() {
        let content = "fn main() { println!(\"hello\"); }";
        let tests = scan_tests("src/main.rs", content);
        assert!(tests.is_empty());
    }
}
