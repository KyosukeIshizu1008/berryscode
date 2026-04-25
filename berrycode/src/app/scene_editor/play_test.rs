//! Play-mode inline test runner panel.

use crate::app::BerryCodeApp;

pub struct PlayTestState {
    pub open: bool,
    pub test_results: Vec<TestResult>,
    pub running: bool,
    pub auto_run_on_play: bool,
}

pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub duration_ms: f64,
    pub output: String,
}

impl Default for PlayTestState {
    fn default() -> Self {
        Self {
            open: false,
            test_results: Vec::new(),
            running: false,
            auto_run_on_play: false,
        }
    }
}

impl PlayTestState {
    pub fn add_result(&mut self, name: String, passed: bool, duration_ms: f64, output: String) {
        self.test_results.push(TestResult {
            name,
            passed,
            duration_ms,
            output,
        });
    }

    pub fn clear_results(&mut self) {
        self.test_results.clear();
    }

    pub fn passed_count(&self) -> usize {
        self.test_results.iter().filter(|r| r.passed).count()
    }

    pub fn failed_count(&self) -> usize {
        self.test_results.iter().filter(|r| !r.passed).count()
    }

    pub fn total_duration_ms(&self) -> f64 {
        self.test_results.iter().map(|r| r.duration_ms).sum()
    }
}

impl BerryCodeApp {
    pub(crate) fn render_play_test_panel(&mut self, ui: &mut egui::Ui) {
        if !self.play_test.open {
            return;
        }
        ui.separator();
        ui.horizontal(|ui| {
            ui.heading("Play Tests");
            ui.checkbox(&mut self.play_test.auto_run_on_play, "Auto-run on play");
            if ui.small_button("Run").clicked() {
                self.play_test.running = true;
                // Actual test execution would be async; placeholder
                self.play_test.running = false;
            }
            if ui.small_button("Clear").clicked() {
                self.play_test.clear_results();
            }
        });
        if self.play_test.running {
            ui.spinner();
            ui.label("Running tests...");
        }
        let passed = self.play_test.passed_count();
        let failed = self.play_test.failed_count();
        let total = self.play_test.test_results.len();
        if total > 0 {
            ui.label(format!(
                "{} tests: {} passed, {} failed ({:.1}ms)",
                total,
                passed,
                failed,
                self.play_test.total_duration_ms()
            ));
        }
        egui::ScrollArea::vertical()
            .max_height(200.0)
            .show(ui, |ui| {
                for result in &self.play_test.test_results {
                    let icon = if result.passed { "ok" } else { "FAIL" };
                    let color = if result.passed {
                        egui::Color32::from_rgb(80, 200, 80)
                    } else {
                        egui::Color32::from_rgb(255, 80, 80)
                    };
                    ui.horizontal(|ui| {
                        ui.colored_label(color, icon);
                        ui.label(&result.name);
                        ui.label(format!("{:.1}ms", result.duration_ms));
                    });
                    if !result.output.is_empty() {
                        ui.indent(result.name.as_str(), |ui| {
                            ui.label(
                                egui::RichText::new(&result.output)
                                    .monospace()
                                    .size(11.0)
                                    .color(egui::Color32::from_gray(160)),
                            );
                        });
                    }
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state() {
        let s = PlayTestState::default();
        assert!(!s.running);
        assert!(!s.auto_run_on_play);
        assert!(s.test_results.is_empty());
    }

    #[test]
    fn add_and_count_results() {
        let mut s = PlayTestState::default();
        s.add_result("test_a".into(), true, 10.0, String::new());
        s.add_result("test_b".into(), false, 5.0, "assertion failed".into());
        s.add_result("test_c".into(), true, 3.0, String::new());
        assert_eq!(s.passed_count(), 2);
        assert_eq!(s.failed_count(), 1);
        assert!((s.total_duration_ms() - 18.0).abs() < f64::EPSILON);
    }

    #[test]
    fn clear_results() {
        let mut s = PlayTestState::default();
        s.add_result("t".into(), true, 1.0, String::new());
        s.clear_results();
        assert!(s.test_results.is_empty());
        assert_eq!(s.passed_count(), 0);
    }
}
