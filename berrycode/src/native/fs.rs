//! Native file system operations
//! Replaces tauri_bindings fs commands

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DirEntry {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
    pub children: Option<Vec<DirEntry>>,
}

/// Get project root directory.
/// Walks up from CWD to find the nearest `.git` directory.
/// Falls back to CWD if no git root is found.
pub fn get_current_dir() -> Result<String> {
    let cwd = std::env::current_dir().context("Failed to get current directory")?;

    // Walk up from CWD to find git root
    let mut dir = cwd.clone();
    loop {
        if dir.join(".git").exists() {
            return Ok(dir.to_str().context("Invalid UTF-8 in path")?.to_string());
        }
        if !dir.pop() {
            break;
        }
    }

    // No git root found — fall back to CWD
    Ok(cwd.to_str().context("Invalid UTF-8 in path")?.to_string())
}

/// Read file contents (with size limit for safety)
pub fn read_file(path: impl AsRef<Path>) -> Result<String> {
    let path = path.as_ref();
    let metadata = fs::metadata(path).context("Failed to read file metadata")?;

    // Safety limit: 10MB
    const MAX_SIZE: u64 = 10 * 1024 * 1024;
    if metadata.len() > MAX_SIZE {
        anyhow::bail!("File too large: {} bytes (max: {} bytes)", metadata.len(), MAX_SIZE);
    }

    fs::read_to_string(path).context("Failed to read file")
}

/// Read file contents partially (for large files)
pub fn read_file_partial(path: impl AsRef<Path>, max_bytes: Option<usize>) -> Result<(String, bool, u64)> {
    let path = path.as_ref();
    let metadata = fs::metadata(path).context("Failed to read file metadata")?;
    let total_size = metadata.len();

    let max_bytes = max_bytes.unwrap_or(5 * 1024 * 1024); // Default 5MB

    if total_size <= max_bytes as u64 {
        // File is small enough, read completely
        let content = fs::read_to_string(path).context("Failed to read file")?;
        Ok((content, false, total_size))
    } else {
        // Read only first max_bytes
        use std::io::Read;
        let mut file = fs::File::open(path).context("Failed to open file")?;
        let mut buffer = vec![0u8; max_bytes];
        let bytes_read = file.read(&mut buffer).context("Failed to read file")?;
        buffer.truncate(bytes_read);

        let content = String::from_utf8_lossy(&buffer).to_string();
        Ok((content, true, total_size))
    }
}

/// Write file contents
pub fn write_file(path: impl AsRef<Path>, content: &str) -> Result<()> {
    let path = path.as_ref();

    // Create parent directories if they don't exist
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("Failed to create parent directories")?;
    }

    fs::write(path, content).context("Failed to write file")
}

/// Create a new file
pub fn create_file(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::File::create(path).context("Failed to create file")?;
    Ok(())
}

/// Delete a file or directory
pub fn delete_file(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();

    if path.is_dir() {
        fs::remove_dir_all(path).context("Failed to remove directory")?;
    } else {
        fs::remove_file(path).context("Failed to remove file")?;
    }

    Ok(())
}

/// Rename/move a file or directory
pub fn rename_file(old_path: impl AsRef<Path>, new_path: impl AsRef<Path>) -> Result<()> {
    fs::rename(old_path.as_ref(), new_path.as_ref()).context("Failed to rename file")
}

/// Get file metadata
pub fn get_file_info(path: impl AsRef<Path>) -> Result<FileInfo> {
    let path = path.as_ref();
    let metadata = fs::metadata(path).context("Failed to get file metadata")?;

    let modified = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs());

    Ok(FileInfo {
        path: path.to_string_lossy().to_string(),
        name: path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string(),
        is_dir: metadata.is_dir(),
        size: metadata.len(),
        modified,
    })
}

/// Read directory contents with depth limit
pub fn read_dir(path: impl AsRef<Path>, max_depth: Option<usize>) -> Result<Vec<DirEntry>> {
    let path = path.as_ref();
    read_dir_internal(path, max_depth.unwrap_or(usize::MAX), 0)
}

/// Internal helper for read_dir with current depth tracking
fn read_dir_internal(path: &Path, max_depth: usize, current_depth: usize) -> Result<Vec<DirEntry>> {
    let mut entries = Vec::new();

    for entry in fs::read_dir(path).context("Failed to read directory")? {
        let entry = entry.context("Failed to read directory entry")?;
        let entry_path = entry.path();
        let name = entry
            .file_name()
            .to_string_lossy()
            .to_string();

        let is_dir = entry
            .file_type()
            .context("Failed to get file type")?
            .is_dir();

        let children = if is_dir && current_depth < max_depth {
            read_dir_internal(&entry_path, max_depth, current_depth + 1).ok()
        } else if is_dir {
            // Don't load children yet, but indicate it's a directory
            None
        } else {
            None
        };

        entries.push(DirEntry {
            path: entry_path.to_string_lossy().to_string(),
            name,
            is_dir,
            children,
        });
    }

    entries.sort_by(|a, b| {
        // Directories first, then alphabetical
        match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        }
    });

    Ok(entries)
}

/// Read directory contents recursively
pub fn read_dir_recursive(path: impl AsRef<Path>) -> Result<Vec<DirEntry>> {
    read_dir(path, None)
}

/// Search for files by name pattern
pub fn search_files(root: impl AsRef<Path>, pattern: &str) -> Result<Vec<String>> {
    let root = root.as_ref();
    let pattern_lower = pattern.to_lowercase();

    let mut results = Vec::new();

    for entry in WalkDir::new(root)
        .max_depth(10)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            if let Some(name) = entry.file_name().to_str() {
                if name.to_lowercase().contains(&pattern_lower) {
                    results.push(entry.path().to_string_lossy().to_string());
                }
            }
        }
    }

    Ok(results)
}

/// Select a folder using native file dialog
/// TODO: Implement using native file picker (rfd crate or platform-specific)
pub async fn select_folder() -> Result<Option<String>> {
    // Stub implementation - needs native file picker
    // For now, return None to indicate cancellation
    #[cfg(debug_assertions)]
    tracing::warn!("select_folder() is not yet implemented - returning None");
    Ok(None)
}

/// Listen for file system changes
/// TODO: Implement using notify crate for file system watching
pub async fn listen_file_changed<F>(callback: F) -> Result<()>
where
    F: Fn(String) + Send + 'static,
{
    // Stub implementation - needs file system watcher
    #[cfg(debug_assertions)]
    tracing::warn!("listen_file_changed() is not yet implemented - doing nothing");

    // Prevent unused variable warning
    let _ = callback;
    Ok(())
}

