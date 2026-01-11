//! Linting support for aider

use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use crate::berrycode::Result;

pub struct Linter {
    lint_commands: HashMap<String, String>,
}

impl Linter {
    pub fn new(lint_commands: HashMap<String, String>) -> Self {
        Self { lint_commands }
    }

    pub fn lint_file(&self, file: &Path, language: &str) -> Result<Vec<String>> {
        let cmd = if let Some(cmd) = self.lint_commands.get(language) {
            cmd
        } else {
            // Try default command based on language
            return self.lint_with_default(file, language);
        };

        self.run_lint_command(file, cmd)
    }

    fn lint_with_default(&self, file: &Path, language: &str) -> Result<Vec<String>> {
        let default_cmd = match language {
            "python" => "flake8",
            "rust" => "cargo clippy",
            "javascript" | "typescript" => "eslint",
            "go" => "golint",
            _ => return Ok(Vec::new()),
        };

        self.run_lint_command(file, default_cmd)
    }

    fn run_lint_command(&self, file: &Path, cmd: &str) -> Result<Vec<String>> {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(Vec::new());
        }

        let mut command = Command::new(parts[0]);
        for arg in &parts[1..] {
            command.arg(arg);
        }
        command.arg(file);

        let output = command.output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let mut errors = Vec::new();
        for line in stdout.lines().chain(stderr.lines()) {
            if !line.trim().is_empty() {
                errors.push(line.to_string());
            }
        }

        Ok(errors)
    }

    pub fn auto_fix(&self, file: &Path, language: &str) -> Result<bool> {
        let fix_cmd = match language {
            "python" => Some("black"),
            "rust" => Some("rustfmt"),
            "javascript" | "typescript" => Some("eslint --fix"),
            _ => None,
        };

        if let Some(cmd) = fix_cmd {
            let parts: Vec<&str> = cmd.split_whitespace().collect();
            let mut command = Command::new(parts[0]);
            for arg in &parts[1..] {
                command.arg(arg);
            }
            command.arg(file);

            let output = command.output()?;
            Ok(output.status.success())
        } else {
            Ok(false)
        }
    }
}
