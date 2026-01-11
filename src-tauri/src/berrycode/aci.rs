//! ACI (Agent-Computer Interface)
//!
//! Inspired by SWE-agent from Princeton University.
//! Formats terminal output, compiler errors, test results, etc.
//! to be optimally consumed by AI agents (DeepSeek-R1/V3).
//!
//! Philosophy:
//! - Humans need full context, colors, and pretty formatting
//! - AI needs structured, minimal, ID-referenced information
//! - Remove noise (progress bars, ANSI codes, timestamps)
//! - Highlight actionable items (file:line, error codes)

// use anyhow::Result; // Currently unused
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Formatted output for AI consumption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ACIOutput {
    /// Output type
    pub output_type: OutputType,

    /// Structured data
    pub data: OutputData,

    /// Original raw output (for debugging)
    pub raw: String,
}

/// Type of output
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OutputType {
    /// Compiler error (Rust, TypeScript, etc.)
    CompilerError,

    /// Test results
    TestResult,

    /// Git operation
    GitOperation,

    /// File operation
    FileOperation,

    /// Shell command
    ShellCommand,

    /// Generic output
    Generic,
}

/// Structured output data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OutputData {
    CompilerErrors(Vec<CompilerError>),
    TestResults(TestResults),
    GitStatus(GitStatus),
    FileOperation(FileOperationResult),
    ShellOutput(ShellOutput),
    Generic(String),
}

/// Compiler error entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilerError {
    /// File path
    pub file: String,

    /// Line number
    pub line: usize,

    /// Column number (optional)
    pub column: Option<usize>,

    /// Error code (e.g., "E0308" for Rust)
    pub code: Option<String>,

    /// Error message
    pub message: String,

    /// Severity (error, warning, info)
    pub severity: Severity,

    /// Suggestion (how to fix)
    pub suggestion: Option<String>,
}

/// Error severity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
    Hint,
}

/// Test results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResults {
    /// Total tests
    pub total: usize,

    /// Passed tests
    pub passed: usize,

    /// Failed tests
    pub failed: usize,

    /// Failed test details
    pub failures: Vec<TestFailure>,
}

/// Test failure detail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestFailure {
    /// Test name
    pub test_name: String,

    /// File path
    pub file: Option<String>,

    /// Line number
    pub line: Option<usize>,

    /// Failure message
    pub message: String,

    /// Expected vs actual
    pub diff: Option<String>,
}

/// Git status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatus {
    /// Current branch
    pub branch: String,

    /// Modified files
    pub modified: Vec<String>,

    /// Untracked files
    pub untracked: Vec<String>,

    /// Staged files
    pub staged: Vec<String>,
}

/// File operation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileOperationResult {
    /// Operation type (read, write, delete, etc.)
    pub operation: String,

    /// File path
    pub file: String,

    /// Success status
    pub success: bool,

    /// Error message if failed
    pub error: Option<String>,
}

/// Shell command output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellOutput {
    /// Command executed
    pub command: String,

    /// Exit code
    pub exit_code: i32,

    /// Standard output (cleaned)
    pub stdout: String,

    /// Standard error (cleaned)
    pub stderr: String,
}

/// ACI Formatter
///
/// Converts raw terminal output into AI-friendly structured format.
pub struct ACIFormatter {
    /// Rust compiler error patterns
    rust_error_regex: Regex,

    /// TypeScript compiler error patterns
    ts_error_regex: Regex,

    /// Test framework patterns
    test_regex: HashMap<String, Regex>,
}

impl ACIFormatter {
    /// Create new ACI formatter
    pub fn new() -> Self {
        // Rust: error[E0308]: mismatched types --> src/main.rs:10:5
        let rust_error_regex = Regex::new(
            r"(?m)^(error|warning)\[([^\]]+)\]:\s+(.+)\n\s+-->\s+([^:]+):(\d+):(\d+)"
        ).unwrap();

        // TypeScript: src/app.ts(10,5): error TS2322: Type 'string' is not assignable
        let ts_error_regex = Regex::new(
            r"(?m)^([^(]+)\((\d+),(\d+)\):\s+(error|warning)\s+TS(\d+):\s+(.+)"
        ).unwrap();

        let mut test_regex = HashMap::new();
        // Rust test:   test result: FAILED. 2 passed; 1 failed; 0 ignored
        test_regex.insert(
            "rust".to_string(),
            Regex::new(r"test result: (\w+)\.\s+(\d+) passed;\s+(\d+) failed").unwrap(),
        );

        Self {
            rust_error_regex,
            ts_error_regex,
            test_regex,
        }
    }

    /// Format raw output into ACI format
    pub fn format(&self, raw_output: &str, context: OutputType) -> ACIOutput {
        match context {
            OutputType::CompilerError => self.format_compiler_errors(raw_output),
            OutputType::TestResult => self.format_test_results(raw_output),
            OutputType::GitOperation => self.format_git_output(raw_output),
            OutputType::ShellCommand => self.format_shell_output(raw_output),
            _ => ACIOutput {
                output_type: OutputType::Generic,
                data: OutputData::Generic(self.clean_output(raw_output)),
                raw: raw_output.to_string(),
            },
        }
    }

    /// Clean ANSI codes, progress bars, timestamps
    fn clean_output(&self, output: &str) -> String {
        let mut cleaned = output.to_string();

        // Remove ANSI escape codes
        let ansi_regex = Regex::new(r"\x1B\[[0-9;]*[a-zA-Z]").unwrap();
        cleaned = ansi_regex.replace_all(&cleaned, "").to_string();

        // Remove spinner/progress bar characters
        cleaned = cleaned.replace("⠋", "");
        cleaned = cleaned.replace("⠙", "");
        cleaned = cleaned.replace("⠹", "");
        cleaned = cleaned.replace("⠸", "");
        cleaned = cleaned.replace("⠼", "");
        cleaned = cleaned.replace("⠴", "");
        cleaned = cleaned.replace("⠦", "");
        cleaned = cleaned.replace("⠧", "");
        cleaned = cleaned.replace("⠇", "");
        cleaned = cleaned.replace("⠏", "");

        // Remove carriage returns (overwrite lines)
        // Note: regex crate doesn't support lookahead, so we use a simpler approach
        // First normalize CRLF to LF, then remove remaining CR characters
        cleaned = cleaned.replace("\r\n", "\n");
        cleaned = cleaned.replace('\r', "");

        // Collapse multiple blank lines
        let blank_regex = Regex::new(r"\n{3,}").unwrap();
        cleaned = blank_regex.replace_all(&cleaned, "\n\n").to_string();

        cleaned.trim().to_string()
    }

    /// Format compiler errors
    fn format_compiler_errors(&self, raw_output: &str) -> ACIOutput {
        let mut errors = Vec::new();

        // Try Rust format first
        for cap in self.rust_error_regex.captures_iter(raw_output) {
            errors.push(CompilerError {
                file: cap[4].to_string(),
                line: cap[5].parse().unwrap_or(0),
                column: Some(cap[6].parse().unwrap_or(0)),
                code: Some(cap[2].to_string()),
                message: cap[3].to_string(),
                severity: match &cap[1] {
                    "error" => Severity::Error,
                    "warning" => Severity::Warning,
                    _ => Severity::Info,
                },
                suggestion: None,
            });
        }

        // Try TypeScript format
        if errors.is_empty() {
            for cap in self.ts_error_regex.captures_iter(raw_output) {
                errors.push(CompilerError {
                    file: cap[1].to_string(),
                    line: cap[2].parse().unwrap_or(0),
                    column: Some(cap[3].parse().unwrap_or(0)),
                    code: Some(format!("TS{}", &cap[5])),
                    message: cap[6].to_string(),
                    severity: match &cap[4] {
                        "error" => Severity::Error,
                        "warning" => Severity::Warning,
                        _ => Severity::Info,
                    },
                    suggestion: None,
                });
            }
        }

        ACIOutput {
            output_type: OutputType::CompilerError,
            data: OutputData::CompilerErrors(errors),
            raw: raw_output.to_string(),
        }
    }

    /// Format test results
    fn format_test_results(&self, raw_output: &str) -> ACIOutput {
        // Simple test result parsing
        let mut passed = 0;
        let mut failed = 0;
        let failures = Vec::new();

        if let Some(rust_regex) = self.test_regex.get("rust") {
            if let Some(cap) = rust_regex.captures(raw_output) {
                passed = cap[2].parse().unwrap_or(0);
                failed = cap[3].parse().unwrap_or(0);
            }
        }

        ACIOutput {
            output_type: OutputType::TestResult,
            data: OutputData::TestResults(TestResults {
                total: passed + failed,
                passed,
                failed,
                failures,
            }),
            raw: raw_output.to_string(),
        }
    }

    /// Format git output
    fn format_git_output(&self, raw_output: &str) -> ACIOutput {
        // Simple git status parsing
        let mut modified = Vec::new();
        let mut untracked = Vec::new();
        let staged = Vec::new();

        for line in raw_output.lines() {
            if line.starts_with("modified:") {
                modified.push(line.trim_start_matches("modified:").trim().to_string());
            } else if line.starts_with("??") {
                untracked.push(line.trim_start_matches("??").trim().to_string());
            }
        }

        ACIOutput {
            output_type: OutputType::GitOperation,
            data: OutputData::GitStatus(GitStatus {
                branch: "main".to_string(), // TODO: Parse actual branch
                modified,
                untracked,
                staged,
            }),
            raw: raw_output.to_string(),
        }
    }

    /// Format shell command output
    fn format_shell_output(&self, raw_output: &str) -> ACIOutput {
        ACIOutput {
            output_type: OutputType::ShellCommand,
            data: OutputData::ShellOutput(ShellOutput {
                command: "".to_string(),
                exit_code: 0,
                stdout: self.clean_output(raw_output),
                stderr: String::new(),
            }),
            raw: raw_output.to_string(),
        }
    }

    /// Format output with automatic type detection
    pub fn format_auto(&self, raw_output: &str) -> ACIOutput {
        // Auto-detect output type
        if raw_output.contains("error[E") || raw_output.contains("warning[W") {
            self.format(raw_output, OutputType::CompilerError)
        } else if raw_output.contains("test result:") || raw_output.contains("PASSED") {
            self.format(raw_output, OutputType::TestResult)
        } else if raw_output.contains("modified:") || raw_output.contains("Untracked") {
            self.format(raw_output, OutputType::GitOperation)
        } else {
            self.format(raw_output, OutputType::ShellCommand)
        }
    }
}

impl Default for ACIFormatter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_compiler_error_parsing() {
        let formatter = ACIFormatter::new();
        let rust_error = r#"
error[E0308]: mismatched types
  --> src/main.rs:10:5
   |
10 |     "hello"
   |     ^^^^^^^ expected `i32`, found `&str`
"#;

        let output = formatter.format(rust_error, OutputType::CompilerError);

        if let OutputData::CompilerErrors(errors) = output.data {
            assert_eq!(errors.len(), 1);
            assert_eq!(errors[0].file, "src/main.rs");
            assert_eq!(errors[0].line, 10);
            assert_eq!(errors[0].severity, Severity::Error);
        } else {
            panic!("Expected CompilerErrors");
        }
    }

    #[test]
    fn test_ansi_cleaning() {
        let formatter = ACIFormatter::new();
        let dirty = "\x1B[32mSuccess\x1B[0m";
        let clean = formatter.clean_output(dirty);

        assert_eq!(clean, "Success");
    }

    #[test]
    fn test_auto_detection() {
        let formatter = ACIFormatter::new();

        // Should detect compiler error
        let rust_error = "error[E0308]: mismatched types\n  --> src/main.rs:10:5";
        let output = formatter.format_auto(rust_error);
        assert_eq!(output.output_type, OutputType::CompilerError);

        // Should detect test result
        let test_output = "test result: FAILED. 5 passed; 2 failed; 0 ignored";
        let output = formatter.format_auto(test_output);
        assert_eq!(output.output_type, OutputType::TestResult);
    }
}
