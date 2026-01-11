//! Self-Healing Loop - Autonomous code repair system
//!
//! This module implements **"Mad Science Level 1"** - a system that automatically
//! fixes code until tests pass, without human intervention.
//!
//! ## How It Works
//!
//! 1. **Fix**: AI writes code changes
//! 2. **Verify**: Run tests automatically (cargo test, npm test, etc.)
//! 3. **Judge**:
//!    - ✅ Success: Report "Fixed!"
//!    - ❌ Failure: Parse error logs, generate new fix, go back to step 1
//! 4. **Repeat**: Loop up to max_iterations (default: 3) to prevent infinite loops
//!
//! ## Example
//!
//! ```text
//! User: "Fix this bug"
//!
//! [Iteration 1]
//! AI: *writes fix*
//! System: *runs cargo test*
//! Result: FAILED - "expected String, found &str"
//!
//! [Iteration 2]
//! AI: *analyzes error, writes better fix*
//! System: *runs cargo test*
//! Result: FAILED - "borrowed value does not live long enough"
//!
//! [Iteration 3]
//! AI: *adds lifetime annotations*
//! System: *runs cargo test*
//! Result: ✅ SUCCESS!
//!
//! System: "Fixed! All tests passing."
//! ```
//!
//! This is **true autonomous engineering** - you can go get coffee while BerryCode
//! fixes your code! ☕

use crate::berrycode::Result;
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

/// Self-healing configuration
#[derive(Debug, Clone)]
pub struct SelfHealingConfig {
    /// Maximum number of fix attempts
    pub max_iterations: usize,
    /// Timeout for each test run (seconds)
    pub test_timeout: Duration,
    /// Project type (rust, node, python, etc.)
    pub project_type: ProjectType,
}

impl Default for SelfHealingConfig {
    fn default() -> Self {
        Self {
            max_iterations: 3,
            test_timeout: Duration::from_secs(60),
            project_type: ProjectType::Rust,
        }
    }
}

/// Supported project types
#[derive(Debug, Clone, PartialEq)]
pub enum ProjectType {
    Rust,
    Node,
    Python,
    Go,
}

/// Result of a healing iteration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealingIteration {
    /// Iteration number (1-indexed)
    pub iteration: usize,
    /// What was attempted
    pub attempt_description: String,
    /// Test output
    pub test_output: String,
    /// Whether tests passed
    pub success: bool,
    /// Error message if failed
    pub error_message: Option<String>,
    /// Duration of this iteration
    pub duration_ms: u64,
}

/// Self-healing loop controller
pub struct SelfHealingLoop {
    /// Configuration
    config: SelfHealingConfig,
    /// Project root directory
    project_root: std::path::PathBuf,
    /// History of all iterations
    iterations: Vec<HealingIteration>,
}

impl SelfHealingLoop {
    /// Create a new self-healing loop
    pub fn new(project_root: &Path, config: SelfHealingConfig) -> Self {
        Self {
            config,
            project_root: project_root.to_path_buf(),
            iterations: Vec::new(),
        }
    }

    /// Detect project type from directory structure
    pub fn detect_project_type(project_root: &Path) -> ProjectType {
        if project_root.join("Cargo.toml").exists() {
            ProjectType::Rust
        } else if project_root.join("package.json").exists() {
            ProjectType::Node
        } else if project_root.join("setup.py").exists()
            || project_root.join("pyproject.toml").exists()
        {
            ProjectType::Python
        } else if project_root.join("go.mod").exists() {
            ProjectType::Go
        } else {
            ProjectType::Rust // Default
        }
    }

    /// Run tests and return the result
    pub fn run_tests(&self) -> Result<TestResult> {
        let start = Instant::now();

        let (command, args) = match self.config.project_type {
            ProjectType::Rust => ("cargo", vec!["test", "--color=never"]),
            ProjectType::Node => ("npm", vec!["test"]),
            ProjectType::Python => ("pytest", vec!["-v"]),
            ProjectType::Go => ("go", vec!["test", "./..."]),
        };

        tracing::info!("Running tests: {} {}", command, args.join(" "));

        let output = Command::new(command)
            .args(&args)
            .current_dir(&self.project_root)
            .output()
            .map_err(|e| anyhow!("Failed to run tests: {}", e))?;

        let duration = start.elapsed();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let success = output.status.success();

        Ok(TestResult {
            success,
            stdout,
            stderr,
            duration,
            exit_code: output.status.code(),
        })
    }

    /// Parse test errors to extract actionable information
    pub fn parse_test_errors(&self, test_result: &TestResult) -> Vec<String> {
        let mut errors = Vec::new();

        let combined_output = format!("{}\n{}", test_result.stdout, test_result.stderr);

        // Extract error lines
        for line in combined_output.lines() {
            if line.contains("error:") || line.contains("Error:") || line.contains("FAILED") {
                errors.push(line.trim().to_string());
            }
        }

        // For Rust, extract compiler errors with context
        if self.config.project_type == ProjectType::Rust {
            let lines: Vec<&str> = combined_output.lines().collect();
            for (i, line) in lines.iter().enumerate() {
                if line.contains("error[E") {
                    // Collect error with next few lines for context
                    let context_lines = 5;
                    let end = (i + context_lines).min(lines.len());
                    let error_context = lines[i..end].join("\n");
                    errors.push(error_context);
                }
            }
        }

        errors
    }

    /// Generate a fix prompt based on test errors
    pub fn generate_fix_prompt(&self, errors: &[String], iteration: usize) -> String {
        let mut prompt = format!(
            "## Self-Healing Iteration {}/{}\n\n",
            iteration, self.config.max_iterations
        );

        prompt.push_str("The tests failed with the following errors:\n\n");
        prompt.push_str("```\n");
        for error in errors {
            prompt.push_str(error);
            prompt.push_str("\n");
        }
        prompt.push_str("```\n\n");

        if iteration == 1 {
            prompt.push_str(
                "Please analyze these errors and provide a fix. Focus on:\n\
                 1. Understanding the root cause\n\
                 2. Making minimal, targeted changes\n\
                 3. Ensuring the fix addresses the specific error messages\n\n",
            );
        } else {
            prompt.push_str(&format!(
                "This is attempt {}. The previous fix didn't work. Please:\n\
                 1. Consider what went wrong with the previous approach\n\
                 2. Try a different strategy\n\
                 3. Make sure to address ALL error messages\n\n",
                iteration
            ));
        }

        prompt.push_str("Provide the complete fixed code.");

        prompt
    }

    /// Execute one iteration of the healing loop
    pub fn execute_iteration(
        &mut self,
        iteration: usize,
        attempt_description: String,
    ) -> Result<HealingIteration> {
        let start = Instant::now();

        tracing::info!("🔄 Self-healing iteration {}/{}", iteration, self.config.max_iterations);

        // Run tests
        let test_result = self.run_tests()?;

        let duration = start.elapsed();

        let iteration_result = HealingIteration {
            iteration,
            attempt_description,
            test_output: format!("{}\n{}", test_result.stdout, test_result.stderr),
            success: test_result.success,
            error_message: if !test_result.success {
                Some(self.parse_test_errors(&test_result).join("\n"))
            } else {
                None
            },
            duration_ms: duration.as_millis() as u64,
        };

        self.iterations.push(iteration_result.clone());

        Ok(iteration_result)
    }

    /// Run the complete self-healing loop
    pub fn run_loop(&mut self) -> Result<SelfHealingResult> {
        tracing::info!("🚀 Starting self-healing loop (max {} iterations)", self.config.max_iterations);

        for i in 1..=self.config.max_iterations {
            let iteration = self.execute_iteration(
                i,
                format!("Attempt {} to fix the code", i),
            )?;

            if iteration.success {
                tracing::info!("✅ Self-healing successful after {} iteration(s)!", i);
                return Ok(SelfHealingResult {
                    success: true,
                    iterations: self.iterations.clone(),
                    final_message: format!(
                        "Fixed! All tests passing after {} iteration(s).",
                        i
                    ),
                });
            }

            if i < self.config.max_iterations {
                tracing::warn!(
                    "❌ Iteration {} failed. Error: {}",
                    i,
                    iteration.error_message.as_deref().unwrap_or("Unknown")
                );
                tracing::info!("Preparing next iteration...");
            }
        }

        // All iterations exhausted
        tracing::error!("❌ Self-healing failed after {} iterations", self.config.max_iterations);

        Ok(SelfHealingResult {
            success: false,
            iterations: self.iterations.clone(),
            final_message: format!(
                "Failed to fix after {} iterations. Manual intervention required.",
                self.config.max_iterations
            ),
        })
    }

    /// Get the history of all iterations
    pub fn get_iterations(&self) -> &[HealingIteration] {
        &self.iterations
    }

    /// Get a summary of the healing process
    pub fn get_summary(&self) -> String {
        let mut summary = String::new();
        summary.push_str("# Self-Healing Summary\n\n");

        for iteration in &self.iterations {
            summary.push_str(&format!(
                "## Iteration {}\n",
                iteration.iteration
            ));
            summary.push_str(&format!(
                "- Status: {}\n",
                if iteration.success { "✅ SUCCESS" } else { "❌ FAILED" }
            ));
            summary.push_str(&format!("- Duration: {}ms\n", iteration.duration_ms));

            if let Some(error) = &iteration.error_message {
                summary.push_str(&format!("- Error:\n```\n{}\n```\n", error));
            }

            summary.push_str("\n");
        }

        summary
    }
}

/// Result of running tests
#[derive(Debug, Clone)]
pub struct TestResult {
    /// Whether tests passed
    pub success: bool,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Test duration
    pub duration: Duration,
    /// Exit code
    pub exit_code: Option<i32>,
}

/// Final result of the self-healing loop
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfHealingResult {
    /// Whether healing was successful
    pub success: bool,
    /// All iterations
    pub iterations: Vec<HealingIteration>,
    /// Final message
    pub final_message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_detect_project_type() {
        let temp_dir = TempDir::new().unwrap();

        // Rust project
        fs::write(temp_dir.path().join("Cargo.toml"), "[package]").unwrap();
        assert_eq!(
            SelfHealingLoop::detect_project_type(temp_dir.path()),
            ProjectType::Rust
        );
    }

    #[test]
    fn test_self_healing_loop_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = SelfHealingConfig::default();
        let loop_controller = SelfHealingLoop::new(temp_dir.path(), config);

        assert_eq!(loop_controller.iterations.len(), 0);
    }

    #[test]
    fn test_parse_test_errors() {
        let temp_dir = TempDir::new().unwrap();
        let config = SelfHealingConfig::default();
        let loop_controller = SelfHealingLoop::new(temp_dir.path(), config);

        let test_result = TestResult {
            success: false,
            stdout: "error: expected String, found &str".to_string(),
            stderr: "FAILED: test_function".to_string(),
            duration: Duration::from_secs(1),
            exit_code: Some(1),
        };

        let errors = loop_controller.parse_test_errors(&test_result);
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.contains("expected String")));
    }

    #[test]
    fn test_generate_fix_prompt() {
        let temp_dir = TempDir::new().unwrap();
        let config = SelfHealingConfig::default();
        let loop_controller = SelfHealingLoop::new(temp_dir.path(), config);

        let errors = vec!["error: undefined reference to `main`".to_string()];
        let prompt = loop_controller.generate_fix_prompt(&errors, 1);

        assert!(prompt.contains("Iteration 1"));
        assert!(prompt.contains("undefined reference"));
    }
}
