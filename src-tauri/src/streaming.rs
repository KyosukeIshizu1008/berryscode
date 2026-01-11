//! Streaming File Reader for Large Files
//!
//! Uses Rust async streams to progressively load large files
//! Prevents UI freezing when opening multi-MB files

use futures::stream::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::pin::Pin;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};

/// File chunk with line information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChunk {
    pub start_line: usize,
    pub end_line: usize,
    pub lines: Vec<String>,
    pub is_final: bool,
    pub total_lines: Option<usize>,
}

/// Stream file contents line by line
pub fn stream_file_lines(
    path: impl AsRef<Path>,
) -> Pin<Box<dyn Stream<Item = Result<FileChunk, String>> + Send>> {
    let path = path.as_ref().to_path_buf();

    Box::pin(async_stream::stream! {
        let file = match File::open(&path).await {
            Ok(f) => f,
            Err(e) => {
                yield Err(format!("Failed to open file: {}", e));
                return;
            }
        };

        let reader = BufReader::new(file);
        let mut lines_iter = reader.lines();

        const CHUNK_SIZE: usize = 1000; // Send 1000 lines at a time
        let mut current_chunk = Vec::new();
        let mut line_number = 0;
        let mut chunk_start = 0;

        while let Some(line_result) = lines_iter.next_line().await.transpose() {
            match line_result {
                Ok(line) => {
                    current_chunk.push(line);
                    line_number += 1;

                    if current_chunk.len() >= CHUNK_SIZE {
                        yield Ok(FileChunk {
                            start_line: chunk_start,
                            end_line: line_number - 1,
                            lines: std::mem::take(&mut current_chunk),
                            is_final: false,
                            total_lines: None,
                        });
                        chunk_start = line_number;
                    }
                }
                Err(e) => {
                    yield Err(format!("Error reading line {}: {}", line_number, e));
                    return;
                }
            }
        }

        // Send remaining lines
        if !current_chunk.is_empty() {
            yield Ok(FileChunk {
                start_line: chunk_start,
                end_line: line_number - 1,
                lines: current_chunk,
                is_final: true,
                total_lines: Some(line_number),
            });
        } else if line_number > 0 {
            // Send final marker even if last chunk was exactly CHUNK_SIZE
            yield Ok(FileChunk {
                start_line: line_number,
                end_line: line_number,
                lines: vec![],
                is_final: true,
                total_lines: Some(line_number),
            });
        }
    })
}

/// Read file in streaming fashion (Tauri command wrapper)
///
/// Note: Tauri commands don't support returning streams directly,
/// so this function collects chunks and sends them via events
pub async fn read_file_streaming(path: String, window: tauri::Window) -> Result<(), String> {
    let mut stream = stream_file_lines(&path);

    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                // Emit chunk to frontend via Tauri event
                use tauri::Emitter;
                if let Err(e) = window.emit("file_chunk", &chunk) {
                    return Err(format!("Failed to emit chunk: {}", e));
                }

                // Small delay to prevent overwhelming the UI thread
                if !chunk.is_final {
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                }
            }
            Err(e) => {
                // Emit error event
                use tauri::Emitter;
                let _ = window.emit("file_error", e.clone());
                return Err(e);
            }
        }
    }

    Ok(())
}

/// Tauri command: Stream large file
#[tauri::command]
pub async fn stream_large_file(path: String, window: tauri::Window) -> Result<(), String> {
    read_file_streaming(path, window).await
}

/// Tauri command: Read file with automatic streaming detection
/// Uses streaming for files > 1MB
#[tauri::command]
pub async fn read_file_auto(path: String, window: tauri::Window) -> Result<String, String> {
    // Check file size
    let metadata = tokio::fs::metadata(&path)
        .await
        .map_err(|e| format!("Failed to get metadata: {}", e))?;

    const STREAMING_THRESHOLD: u64 = 1024 * 1024; // 1MB

    if metadata.len() > STREAMING_THRESHOLD {
        // Use streaming for large files
        stream_large_file(path, window).await?;
        Ok(String::new()) // Return empty string, chunks sent via events
    } else {
        // Read entire file for small files
        tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| format!("Failed to read file: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn test_stream_small_file() {
        // Create temp file with 100 lines
        let mut temp_file = NamedTempFile::new().unwrap();
        for i in 0..100 {
            temp_file
                .write_all(format!("Line {}\n", i).as_bytes())
                .await
                .unwrap();
        }
        temp_file.flush().await.unwrap();

        let path = temp_file.path().to_path_buf();
        let mut stream = stream_file_lines(&path);

        let mut total_lines = 0;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.unwrap();
            total_lines += chunk.lines.len();
            if chunk.is_final {
                assert_eq!(chunk.total_lines, Some(100));
                break;
            }
        }

        assert_eq!(total_lines, 100);
    }

    #[tokio::test]
    async fn test_stream_large_file() {
        // Create temp file with 5000 lines (5 chunks)
        let mut temp_file = NamedTempFile::new().unwrap();
        for i in 0..5000 {
            temp_file
                .write_all(format!("Line {}\n", i).as_bytes())
                .await
                .unwrap();
        }
        temp_file.flush().await.unwrap();

        let path = temp_file.path().to_path_buf();
        let mut stream = stream_file_lines(&path);

        let mut chunks_count = 0;
        let mut total_lines = 0;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.unwrap();
            chunks_count += 1;
            total_lines += chunk.lines.len();
            if chunk.is_final {
                assert_eq!(chunk.total_lines, Some(5000));
                break;
            }
        }

        assert_eq!(total_lines, 5000);
        assert!(chunks_count >= 5); // At least 5 chunks for 5000 lines
    }
}
