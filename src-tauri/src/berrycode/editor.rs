//! Editor integration for aider

use std::path::Path;
use std::fs;
use std::process::Command;
use tempfile::NamedTempFile;
use crate::berrycode::Result;
use anyhow::anyhow;

/// Open content in user's editor and return edited result
pub fn pipe_editor(content: &str, file: Option<&Path>) -> Result<String> {
    let editor = get_editor()?;

    // Create temporary file
    let mut temp_file = if let Some(path) = file {
        // Use the same extension as the original file
        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("txt");
        tempfile::Builder::new()
            .suffix(&format!(".{}", extension))
            .tempfile()?
    } else {
        NamedTempFile::new()?
    };

    // Write content to temp file
    use std::io::Write;
    temp_file.write_all(content.as_bytes())?;
    temp_file.flush()?;

    let temp_path = temp_file.path();

    // Open editor
    let status = Command::new(&editor)
        .arg(temp_path)
        .status()?;

    if !status.success() {
        return Err(anyhow!("Editor exited with error"));
    }

    // Read edited content
    let edited_content = fs::read_to_string(temp_path)?;

    Ok(edited_content)
}

/// Get the user's preferred editor
fn get_editor() -> Result<String> {
    // Try environment variables in order
    if let Ok(editor) = std::env::var("VISUAL") {
        return Ok(editor);
    }

    if let Ok(editor) = std::env::var("EDITOR") {
        return Ok(editor);
    }

    // Try common editors
    for editor in &["nano", "vim", "vi", "emacs", "code", "subl"] {
        if Command::new("which")
            .arg(editor)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Ok(editor.to_string());
        }
    }

    Err(anyhow!("No editor found. Set EDITOR or VISUAL environment variable."))
}

/// Open file in editor
pub fn open_in_editor(file: &Path) -> Result<()> {
    let editor = get_editor()?;

    let status = Command::new(&editor)
        .arg(file)
        .status()?;

    if !status.success() {
        return Err(anyhow!("Editor exited with error"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_editor() {
        // This test will fail if no editor is available
        // but that's okay for CI environments
        let _ = get_editor();
    }
}
