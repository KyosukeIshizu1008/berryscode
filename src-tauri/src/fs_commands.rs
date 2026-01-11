use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use walkdir::WalkDir;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct FileNode {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub children: Option<Vec<FileNode>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileMetadata {
    pub size: u64,
    pub modified: Option<u64>,
    pub is_readonly: bool,
}

/// Get current working directory (returns project root, not src-tauri)
#[tauri::command]
pub async fn get_current_dir() -> Result<String, String> {
    let current =
        std::env::current_dir().map_err(|e| format!("Failed to get current directory: {}", e))?;

    // If we're in src-tauri, return parent directory (project root)
    let path = if current.ends_with("src-tauri") {
        current
            .parent()
            .ok_or_else(|| "Cannot get parent directory".to_string())?
            .to_path_buf()
    } else {
        current
    };

    path.to_str()
        .ok_or_else(|| "Invalid UTF-8 in path".to_string())
        .map(|s| s.to_string())
}

/// ⚠️ DEPRECATED: Use read_file_partial instead to avoid memory issues
/// This command has a 10MB safety limit. For larger files, use read_file_partial.
#[tauri::command]
pub async fn read_file(path: String) -> Result<String, String> {
    // ✅ Safety check: Prevent reading files larger than 10MB at once
    const MAX_SAFE_SIZE: u64 = 10_000_000; // 10MB

    let metadata = fs::metadata(&path).map_err(|e| format!("Failed to get file size: {}", e))?;

    if metadata.len() > MAX_SAFE_SIZE {
        return Err(format!(
            "File too large ({} bytes). Use read_file_partial for files > 10MB",
            metadata.len()
        ));
    }

    fs::read_to_string(&path).map_err(|e| {
        let error_msg = e.to_string();
        if error_msg.contains("stream did not contain valid UTF-8") {
            format!("Cannot read binary file as text: {}", path)
        } else {
            format!("Failed to read file: {}", e)
        }
    })
}

/// ✅ IntelliJ Pro: Read file with partial loading (first N bytes only)
/// Returns: (content, is_partial, total_size)
#[tauri::command]
pub async fn read_file_partial(
    path: String,
    max_bytes: Option<usize>,
) -> Result<(String, bool, u64), String> {
    use std::io::Read;

    let metadata = fs::metadata(&path).map_err(|e| format!("Failed to get file size: {}", e))?;
    let total_size = metadata.len();
    let max_bytes = max_bytes.unwrap_or(10_000_000); // Default 10MB

    if total_size <= max_bytes as u64 {
        // File is small, read entirely
        let content =
            fs::read_to_string(&path).map_err(|e| {
        let error_msg = e.to_string();
        if error_msg.contains("stream did not contain valid UTF-8") {
            format!("Cannot read binary file as text: {}", path)
        } else {
            format!("Failed to read file: {}", e)
        }
    })?;
        Ok((content, false, total_size))
    } else {
        // File is large, read only first N bytes
        let mut file = fs::File::open(&path).map_err(|e| format!("Failed to open file: {}", e))?;
        let mut buffer = vec![0; max_bytes];
        let bytes_read = file
            .read(&mut buffer)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        buffer.truncate(bytes_read);
        let content = String::from_utf8_lossy(&buffer).to_string();
        Ok((content, true, total_size))
    }
}

/// ✅ IntelliJ Pro: Read file chunk (for streaming large files)
/// offset: byte offset, length: bytes to read
#[tauri::command]
pub async fn read_file_chunk(path: String, offset: u64, length: usize) -> Result<String, String> {
    use std::io::{Read, Seek, SeekFrom};

    let mut file = fs::File::open(&path).map_err(|e| format!("Failed to open file: {}", e))?;
    file.seek(SeekFrom::Start(offset))
        .map_err(|e| format!("Failed to seek: {}", e))?;

    let mut buffer = vec![0; length];
    let bytes_read = file
        .read(&mut buffer)
        .map_err(|e| format!("Failed to read chunk: {}", e))?;

    buffer.truncate(bytes_read);
    let content = String::from_utf8_lossy(&buffer).to_string();
    Ok(content)
}

/// Write file contents
#[tauri::command]
pub async fn write_file(path: String, contents: String) -> Result<(), String> {
    fs::write(&path, contents).map_err(|e| format!("Failed to write file: {}", e))
}

/// Read directory contents recursively
#[tauri::command]
pub async fn read_dir(path: String, max_depth: Option<usize>) -> Result<Vec<FileNode>, String> {
    let max_depth = max_depth.unwrap_or(3);
    let root_path = PathBuf::from(&path);

    if !root_path.exists() {
        return Err(format!("Path does not exist: {}", path));
    }

    let mut nodes = Vec::new();

    for entry in WalkDir::new(&root_path)
        .max_depth(1)
        .min_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let entry_path = entry.path();
        let is_dir = entry_path.is_dir();

        let name = entry_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        // Skip hidden files/directories (starting with .)
        if name.starts_with('.') && name != "." && name != ".." {
            continue;
        }

        let path_str = entry_path.to_string_lossy().to_string();

        let children = if is_dir && max_depth > 1 {
            read_dir_recursive(entry_path, max_depth - 1).ok()
        } else {
            None
        };

        nodes.push(FileNode {
            name,
            path: path_str,
            is_dir,
            children,
        });
    }

    // Sort: directories first, then files, alphabetically
    nodes.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    Ok(nodes)
}

fn read_dir_recursive(path: &std::path::Path, max_depth: usize) -> Result<Vec<FileNode>, String> {
    if max_depth == 0 {
        return Ok(Vec::new());
    }

    let mut nodes = Vec::new();

    for entry in WalkDir::new(path)
        .max_depth(1)
        .min_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let entry_path = entry.path();
        let is_dir = entry_path.is_dir();

        let name = entry_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        // Skip hidden files
        if name.starts_with('.') {
            continue;
        }

        let path_str = entry_path.to_string_lossy().to_string();

        let children = if is_dir && max_depth > 1 {
            read_dir_recursive(entry_path, max_depth - 1).ok()
        } else {
            None
        };

        nodes.push(FileNode {
            name,
            path: path_str,
            is_dir,
            children,
        });
    }

    nodes.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    Ok(nodes)
}

/// Create a new file
#[tauri::command]
pub async fn create_file(path: String, contents: Option<String>) -> Result<(), String> {
    let content = contents.unwrap_or_default();
    fs::write(&path, content).map_err(|e| format!("Failed to create file: {}", e))
}

/// Delete a file or directory
#[tauri::command]
pub async fn delete_file(path: String) -> Result<(), String> {
    let path_buf = PathBuf::from(&path);

    if path_buf.is_dir() {
        fs::remove_dir_all(&path).map_err(|e| format!("Failed to delete directory: {}", e))
    } else {
        fs::remove_file(&path).map_err(|e| format!("Failed to delete file: {}", e))
    }
}

/// Rename/move a file or directory
#[tauri::command]
pub async fn rename_file(old_path: String, new_path: String) -> Result<(), String> {
    fs::rename(&old_path, &new_path).map_err(|e| format!("Failed to rename file: {}", e))
}

/// Get file metadata
#[tauri::command]
pub async fn get_file_metadata(path: String) -> Result<FileMetadata, String> {
    let metadata = fs::metadata(&path).map_err(|e| format!("Failed to get metadata: {}", e))?;

    let modified = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs());

    Ok(FileMetadata {
        size: metadata.len(),
        modified,
        is_readonly: metadata.permissions().readonly(),
    })
}

/// Open folder selection dialog
#[tauri::command]
pub async fn select_folder() -> Result<Option<String>, String> {
    // Use rfd (Rust File Dialog) for cross-platform folder selection
    let folder = rfd::FileDialog::new()
        .set_title("Select Folder")
        .pick_folder();

    Ok(folder.and_then(|path| path.to_str().map(String::from)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    fn setup_test_dir() -> tempfile::TempDir {
        tempfile::tempdir().expect("Failed to create temp dir")
    }

    #[tokio::test]
    async fn test_read_write_file() {
        let temp_dir = setup_test_dir();
        let file_path = temp_dir.path().join("test.txt");
        let file_path_str = file_path.to_str().unwrap().to_string();

        // Write file
        let content = "Hello, BerryEditor!";
        let write_result = write_file(file_path_str.clone(), content.to_string()).await;
        assert!(write_result.is_ok(), "Failed to write file");

        // Read file
        let read_result = read_file(file_path_str).await;
        assert!(read_result.is_ok(), "Failed to read file");
        assert_eq!(read_result.unwrap(), content);
    }

    #[tokio::test]
    async fn test_create_file() {
        let temp_dir = setup_test_dir();
        let file_path = temp_dir.path().join("new_file.txt");
        let file_path_str = file_path.to_str().unwrap().to_string();

        // Create file with content
        let result = create_file(file_path_str.clone(), Some("Test content".to_string())).await;
        assert!(result.is_ok(), "Failed to create file");

        // Verify file exists and has content
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "Test content");
    }

    #[tokio::test]
    async fn test_create_empty_file() {
        let temp_dir = setup_test_dir();
        let file_path = temp_dir.path().join("empty.txt");
        let file_path_str = file_path.to_str().unwrap().to_string();

        // Create empty file
        let result = create_file(file_path_str, None).await;
        assert!(result.is_ok(), "Failed to create empty file");

        // Verify file is empty
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "");
    }

    #[tokio::test]
    async fn test_delete_file() {
        let temp_dir = setup_test_dir();
        let file_path = temp_dir.path().join("to_delete.txt");
        fs::write(&file_path, "Delete me").unwrap();

        let file_path_str = file_path.to_str().unwrap().to_string();

        // Delete file
        let result = delete_file(file_path_str).await;
        assert!(result.is_ok(), "Failed to delete file");

        // Verify file is deleted
        assert!(!file_path.exists());
    }

    #[tokio::test]
    async fn test_delete_directory() {
        let temp_dir = setup_test_dir();
        let dir_path = temp_dir.path().join("to_delete_dir");
        fs::create_dir(&dir_path).unwrap();
        fs::write(dir_path.join("file.txt"), "content").unwrap();

        let dir_path_str = dir_path.to_str().unwrap().to_string();

        // Delete directory
        let result = delete_file(dir_path_str).await;
        assert!(result.is_ok(), "Failed to delete directory");

        // Verify directory is deleted
        assert!(!dir_path.exists());
    }

    #[tokio::test]
    async fn test_rename_file() {
        let temp_dir = setup_test_dir();
        let old_path = temp_dir.path().join("old_name.txt");
        let new_path = temp_dir.path().join("new_name.txt");

        fs::write(&old_path, "Content").unwrap();

        let old_path_str = old_path.to_str().unwrap().to_string();
        let new_path_str = new_path.to_str().unwrap().to_string();

        // Rename file
        let result = rename_file(old_path_str, new_path_str).await;
        assert!(result.is_ok(), "Failed to rename file");

        // Verify old path doesn't exist and new path exists
        assert!(!old_path.exists());
        assert!(new_path.exists());
        assert_eq!(fs::read_to_string(&new_path).unwrap(), "Content");
    }

    #[tokio::test]
    async fn test_get_file_metadata() {
        let temp_dir = setup_test_dir();
        let file_path = temp_dir.path().join("metadata_test.txt");
        let content = "Hello, World!";
        fs::write(&file_path, content).unwrap();

        let file_path_str = file_path.to_str().unwrap().to_string();

        // Get metadata
        let result = get_file_metadata(file_path_str).await;
        assert!(result.is_ok(), "Failed to get metadata");

        let metadata = result.unwrap();
        assert_eq!(metadata.size, content.len() as u64);
        assert!(metadata.modified.is_some());
    }

    #[tokio::test]
    async fn test_read_dir_basic() {
        let temp_dir = setup_test_dir();

        // Create test structure
        fs::write(temp_dir.path().join("file1.txt"), "content1").unwrap();
        fs::write(temp_dir.path().join("file2.txt"), "content2").unwrap();
        fs::create_dir(temp_dir.path().join("subdir")).unwrap();

        let dir_path_str = temp_dir.path().to_str().unwrap().to_string();

        // Read directory
        let result = read_dir(dir_path_str, Some(1)).await;
        assert!(result.is_ok(), "Failed to read directory");

        let nodes = result.unwrap();
        assert_eq!(nodes.len(), 3, "Should have 3 items");

        // Verify directory comes first (sorted)
        assert!(nodes[0].is_dir);
        assert_eq!(nodes[0].name, "subdir");

        // Verify files come after
        assert!(!nodes[1].is_dir);
        assert!(!nodes[2].is_dir);
    }

    #[tokio::test]
    async fn test_read_dir_recursive() {
        let temp_dir = setup_test_dir();

        // Create nested structure
        let subdir = temp_dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();
        fs::write(subdir.join("nested_file.txt"), "nested content").unwrap();

        let dir_path_str = temp_dir.path().to_str().unwrap().to_string();

        // Read directory recursively
        let result = read_dir(dir_path_str, Some(2)).await;
        assert!(result.is_ok(), "Failed to read directory recursively");

        let nodes = result.unwrap();
        assert_eq!(nodes.len(), 1);
        assert!(nodes[0].is_dir);
        assert!(nodes[0].children.is_some());

        let children = nodes[0].children.as_ref().unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].name, "nested_file.txt");
    }

    #[tokio::test]
    async fn test_read_dir_nonexistent() {
        let result = read_dir("/nonexistent/path".to_string(), Some(1)).await;
        assert!(result.is_err(), "Should fail for nonexistent path");
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[tokio::test]
    async fn test_read_file_nonexistent() {
        let result = read_file("/nonexistent/file.txt".to_string()).await;
        assert!(result.is_err(), "Should fail for nonexistent file");
    }

    #[tokio::test]
    async fn test_file_node_sorting() {
        let temp_dir = setup_test_dir();

        // Create files and directories in random order
        fs::write(temp_dir.path().join("z_file.txt"), "").unwrap();
        fs::create_dir(temp_dir.path().join("a_dir")).unwrap();
        fs::write(temp_dir.path().join("b_file.txt"), "").unwrap();
        fs::create_dir(temp_dir.path().join("c_dir")).unwrap();

        let dir_path_str = temp_dir.path().to_str().unwrap().to_string();
        let result = read_dir(dir_path_str, Some(1)).await.unwrap();

        // Directories should come first, then files, alphabetically
        assert_eq!(result[0].name, "a_dir");
        assert!(result[0].is_dir);
        assert_eq!(result[1].name, "c_dir");
        assert!(result[1].is_dir);
        assert_eq!(result[2].name, "b_file.txt");
        assert!(!result[2].is_dir);
        assert_eq!(result[3].name, "z_file.txt");
        assert!(!result[3].is_dir);
    }

    #[tokio::test]
    async fn test_hidden_files_skipped() {
        let temp_dir = setup_test_dir();

        // Create hidden file
        fs::write(temp_dir.path().join(".hidden"), "").unwrap();
        fs::write(temp_dir.path().join("visible.txt"), "").unwrap();

        let dir_path_str = temp_dir.path().to_str().unwrap().to_string();
        let result = read_dir(dir_path_str, Some(1)).await.unwrap();

        // Should only have visible file
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "visible.txt");
    }
}
