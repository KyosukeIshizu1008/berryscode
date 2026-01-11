//! Utility functions for aider

use std::path::Path;
use anyhow::Result;

/// Check if a file is an image file based on extension
pub fn is_image_file(path: &Path) -> bool {
    if let Some(ext) = path.extension() {
        let ext = ext.to_string_lossy().to_lowercase();
        matches!(
            ext.as_str(),
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "svg"
        )
    } else {
        false
    }
}

/// Get the relative path from a base directory
pub fn get_rel_fname(base: &Path, fname: &Path) -> Result<String> {
    let rel_path = fname.strip_prefix(base)?;
    Ok(rel_path.to_string_lossy().to_string())
}

/// Safe absolute path resolution
pub fn safe_abs_path(path: &Path) -> Result<std::path::PathBuf> {
    Ok(path.canonicalize()?)
}

/// Check if a path is within a directory
pub fn is_within_directory(directory: &Path, target: &Path) -> bool {
    let abs_directory = match directory.canonicalize() {
        Ok(p) => p,
        Err(_) => return false,
    };

    let abs_target = match target.canonicalize() {
        Ok(p) => p,
        Err(_) => return false,
    };

    abs_target.starts_with(abs_directory)
}

/// Format file size in human-readable format
pub fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;

    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    if unit_idx == 0 {
        format!("{} {}", size as u64, UNITS[unit_idx])
    } else {
        format!("{:.2} {}", size, UNITS[unit_idx])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_image_file() {
        assert!(is_image_file(Path::new("test.png")));
        assert!(is_image_file(Path::new("test.jpg")));
        assert!(is_image_file(Path::new("test.jpeg")));
        assert!(!is_image_file(Path::new("test.txt")));
        assert!(!is_image_file(Path::new("test.rs")));
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(100), "100 B");
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(1024 * 1024), "1.00 MB");
        assert_eq!(format_size(1536), "1.50 KB");
    }
}
