//! Diff parsing and application

use crate::berrycode::Result;
use crate::berrycode::exceptions::AiderError;
use similar::{ChangeTag, TextDiff};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct DiffHunk {
    pub old_start: usize,
    pub old_count: usize,
    pub new_start: usize,
    pub new_count: usize,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone)]
pub struct DiffLine {
    pub tag: DiffTag,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DiffTag {
    Equal,
    Insert,
    Delete,
}

pub struct DiffApplier {
    pub dry_run: bool,
}

impl DiffApplier {
    pub fn new(dry_run: bool) -> Self {
        Self { dry_run }
    }

    /// Apply a unified diff to a file
    pub fn apply_diff(&self, file_path: &Path, diff_text: &str) -> Result<String> {
        let original = fs::read_to_string(file_path).unwrap_or_default();
        let hunks = self.parse_diff(diff_text)?;

        let mut result = original.clone();
        for hunk in hunks.iter().rev() {
            result = self.apply_hunk(&result, hunk)?;
        }

        if !self.dry_run {
            fs::write(file_path, &result)?;
        }

        Ok(result)
    }

    /// Parse unified diff format
    fn parse_diff(&self, diff_text: &str) -> Result<Vec<DiffHunk>> {
        let mut hunks = Vec::new();
        let lines: Vec<&str> = diff_text.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            if lines[i].starts_with("@@") {
                let hunk = self.parse_hunk(&lines, &mut i)?;
                hunks.push(hunk);
            } else {
                i += 1;
            }
        }

        Ok(hunks)
    }

    fn parse_hunk(&self, lines: &[&str], index: &mut usize) -> Result<DiffHunk> {
        let hunk_header = lines[*index];
        let (old_start, old_count, new_start, new_count) = self.parse_hunk_header(hunk_header)?;

        *index += 1;
        let mut hunk_lines = Vec::new();

        while *index < lines.len() && !lines[*index].starts_with("@@") {
            let line = lines[*index];
            let (tag, content) = if line.starts_with('+') {
                (DiffTag::Insert, &line[1..])
            } else if line.starts_with('-') {
                (DiffTag::Delete, &line[1..])
            } else if line.starts_with(' ') {
                (DiffTag::Equal, &line[1..])
            } else {
                break;
            };

            hunk_lines.push(DiffLine {
                tag,
                content: content.to_string(),
            });
            *index += 1;
        }

        Ok(DiffHunk {
            old_start,
            old_count,
            new_start,
            new_count,
            lines: hunk_lines,
        })
    }

    fn parse_hunk_header(&self, header: &str) -> Result<(usize, usize, usize, usize)> {
        // Format: @@ -old_start,old_count +new_start,new_count @@
        let parts: Vec<&str> = header
            .trim_start_matches("@@")
            .trim_end_matches("@@")
            .trim()
            .split_whitespace()
            .collect();

        if parts.len() < 2 {
            return Err(AiderError::ParseError("Invalid hunk header".to_string()).into());
        }

        let old_part = parts[0].trim_start_matches('-');
        let new_part = parts[1].trim_start_matches('+');

        let (old_start, old_count) = self.parse_range(old_part)?;
        let (new_start, new_count) = self.parse_range(new_part)?;

        Ok((old_start, old_count, new_start, new_count))
    }

    fn parse_range(&self, range_str: &str) -> Result<(usize, usize)> {
        let parts: Vec<&str> = range_str.split(',').collect();
        let start = parts[0].parse::<usize>()?;
        let count = if parts.len() > 1 {
            parts[1].parse::<usize>()?
        } else {
            1
        };
        Ok((start, count))
    }

    fn apply_hunk(&self, content: &str, hunk: &DiffHunk) -> Result<String> {
        let lines: Vec<&str> = content.lines().collect();
        let mut result = Vec::new();

        // Add lines before the hunk
        for line in &lines[..hunk.old_start.saturating_sub(1)] {
            result.push(line.to_string());
        }

        // Apply the hunk
        for diff_line in &hunk.lines {
            match diff_line.tag {
                DiffTag::Insert | DiffTag::Equal => {
                    result.push(diff_line.content.clone());
                }
                DiffTag::Delete => {
                    // Skip deleted lines
                }
            }
        }

        // Add lines after the hunk
        let skip_count = hunk.old_start.saturating_sub(1) + hunk.old_count;
        for line in &lines[skip_count..] {
            result.push(line.to_string());
        }

        Ok(result.join("\n"))
    }

    /// Generate a unified diff between two strings
    pub fn generate_diff(old_text: &str, new_text: &str, filename: &str) -> String {
        let diff = TextDiff::from_lines(old_text, new_text);
        let mut result = format!("--- {}\n+++ {}\n", filename, filename);

        let mut hunk_lines = Vec::new();
        let mut old_line = 1;
        let mut new_line = 1;
        let hunk_old_start = 1;
        let hunk_new_start = 1;

        for change in diff.iter_all_changes() {
            match change.tag() {
                ChangeTag::Equal => {
                    hunk_lines.push(format!(" {}", change));
                    old_line += 1;
                    new_line += 1;
                }
                ChangeTag::Delete => {
                    hunk_lines.push(format!("-{}", change));
                    old_line += 1;
                }
                ChangeTag::Insert => {
                    hunk_lines.push(format!("+{}", change));
                    new_line += 1;
                }
            }
        }

        if !hunk_lines.is_empty() {
            let hunk_old_count = old_line - hunk_old_start;
            let hunk_new_count = new_line - hunk_new_start;
            result.push_str(&format!(
                "@@ -{},{} +{},{} @@\n",
                hunk_old_start, hunk_old_count, hunk_new_start, hunk_new_count
            ));
            result.push_str(&hunk_lines.join(""));
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_diff() {
        let old = "line 1\nline 2\nline 3\n";
        let new = "line 1\nmodified line 2\nline 3\n";
        let diff = DiffApplier::generate_diff(old, new, "test.txt");
        assert!(diff.contains("modified line 2"));
    }

    #[test]
    fn test_parse_range() {
        let applier = DiffApplier::new(true);
        assert_eq!(applier.parse_range("10,5").unwrap(), (10, 5));
        assert_eq!(applier.parse_range("10").unwrap(), (10, 1));
    }
}
