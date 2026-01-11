//! Whole file replacement coder
//! Note: All edit format handling is implemented in base_coder.rs
//! This module exists for API compatibility but delegates to base_coder

use crate::berrycode::Result;
use super::base_coder::Coder;
use std::fs;
use std::path::Path;

pub struct WholefileCoder {
    base: Coder,
}

impl WholefileCoder {
    pub fn new(base: Coder) -> Self {
        Self { base }
    }

    /// Apply whole file replacement
    pub fn apply_replacement(&mut self, file: &str, content: &str) -> Result<()> {
        let path = Path::new(file);

        if !self.base.dry_run {
            fs::write(path, content)?;
        }

        self.base.io.tool_output(&format!("Replaced entire file: {}", file));

        // Auto-commit if enabled
        if self.base.auto_commits && !self.base.dry_run {
            if let Some(ref repo) = self.base.git_repo {
                repo.add(&[path])?;
                repo.commit(&format!("aider: Updated {}", file))?;
            }
        }

        Ok(())
    }

    /// Run the coder
    pub fn run(&mut self) -> Result<()> {
        self.base.run()
    }
}
