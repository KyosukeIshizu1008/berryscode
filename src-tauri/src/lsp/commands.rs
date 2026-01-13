//! Tauri Commands for LSP
//! Exposes LSP functionality to the WASM frontend

use super::LspManager;
use crate::lsp_core::*;
use std::sync::Arc;
use tauri::State;
use crate::grpc_client::BerryApiClient;

/// Register all LSP commands
pub fn register_lsp_commands<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder.invoke_handler(tauri::generate_handler![
        lsp_initialize,
        lsp_get_completions,
        lsp_get_hover,
        lsp_goto_definition,
        lsp_get_diagnostics,
        lsp_find_references,
        lsp_shutdown,
        lsp_add_file_to_context,
    ])
}

/// Initialize LSP for a language
#[tauri::command]
pub async fn lsp_initialize(
    language: String,
    root_uri: String,
    manager: State<'_, Arc<LspManager>>,
) -> Result<bool, String> {
    eprintln!("[LSP COMMAND] ========================================");
    eprintln!("[LSP COMMAND] lsp_initialize CALLED");
    eprintln!("[LSP COMMAND] language={}, root_uri={}", language, root_uri);
    eprintln!("[LSP COMMAND] ========================================");

    // ✅ PERFORMANCE FIX: Don't fail the command if LSP init fails
    // Allow editor to work even without LSP features
    match manager.initialize_client(language.clone(), root_uri.clone()).await {
        Ok(_) => {
            eprintln!("[LSP COMMAND] ✅ Initialization completed successfully");
            Ok(true)
        }
        Err(e) => {
            eprintln!("[LSP COMMAND] ⚠️  Initialization failed: {}", e);
            eprintln!("[LSP COMMAND] ⚠️  Editor will work but LSP features unavailable");
            // Return success even if LSP init failed - don't block UI
            Ok(false)
        }
    }
}

/// Get completions at position
#[tauri::command]
pub async fn lsp_get_completions(
    language: String,
    file_path: String,
    line: u32,
    character: u32,
    manager: State<'_, Arc<LspManager>>,
) -> Result<Vec<CompletionItem>, String> {
    let client_arc = manager
        .get_client(&language).await
        .ok_or_else(|| format!("LSP not initialized for {}", language))?;

    let client = client_arc.lock().await;

    // Convert file path to URI
    let file_uri = if file_path.starts_with("file://") {
        file_path
    } else {
        format!("file://{}", file_path)
    };

    client.get_completions(&file_uri, line, character).await
}

/// Get hover information at position
#[tauri::command]
pub async fn lsp_get_hover(
    language: String,
    file_path: String,
    line: u32,
    character: u32,
    manager: State<'_, Arc<LspManager>>,
) -> Result<Option<Hover>, String> {
    let client_arc = manager
        .get_client(&language).await
        .ok_or_else(|| format!("LSP not initialized for {}", language))?;

    let client = client_arc.lock().await;

    // Convert file path to URI
    let file_uri = if file_path.starts_with("file://") {
        file_path
    } else {
        format!("file://{}", file_path)
    };

    client.get_hover(&file_uri, line, character).await
}

/// Go to definition
#[tauri::command]
pub async fn lsp_goto_definition(
    language: String,
    file_path: String,
    line: u32,
    character: u32,
    manager: State<'_, Arc<LspManager>>,
) -> Result<Location, String> {
    eprintln!("[LSP COMMAND] ========================================");
    eprintln!("[LSP COMMAND] lsp_goto_definition CALLED");
    eprintln!("[LSP COMMAND] language={}, file={}, line={}, char={}", language, file_path, line, character);
    eprintln!("[LSP COMMAND] ========================================");

    // Convert file path to URI
    let file_uri = if file_path.starts_with("file://") {
        file_path.clone()
    } else {
        format!("file://{}", file_path)
    };

    // Try berry_api gRPC service first (has fallback search for stdlib & project symbols)
    eprintln!("[LSP COMMAND] 🌐 Trying berry_api gRPC service (with fallback search)...");

    // Add timeout for berry_api call (10 seconds)
    let berry_api_future = try_berry_api_goto_definition(&file_uri, line, character);
    let timeout_result = tokio::time::timeout(
        tokio::time::Duration::from_secs(10),
        berry_api_future
    ).await;

    match timeout_result {
        Ok(Ok(Some(location))) => {
            eprintln!("[LSP COMMAND] ✅ berry_api found definition: uri={}, line={}, char={}",
                location.uri, location.range.start.line, location.range.start.character);
            eprintln!("[LSP COMMAND] 🔙 Returning location to frontend...");
            let result = Ok(location);
            eprintln!("[LSP COMMAND] 🔙 About to return: {:?}", result);
            return result;
        }
        Ok(Ok(None)) => {
            eprintln!("[LSP COMMAND] ⚠️  berry_api returned None (definition not found)");
        }
        Ok(Err(e)) => {
            eprintln!("[LSP COMMAND] ❌ berry_api error: {}", e);
        }
        Err(_timeout) => {
            eprintln!("[LSP COMMAND] ⏱️  berry_api TIMEOUT (10s) - falling back to local rust-analyzer");
        }
    }

    // Fallback to local rust-analyzer
    eprintln!("[LSP COMMAND] 🔄 berry_api unavailable, falling back to local rust-analyzer...");

    let client_arc = manager
        .get_client(&language).await
        .ok_or_else(|| {
            let err = format!("LSP not initialized for {}", language);
            eprintln!("[LSP COMMAND] ❌ ERROR: {}", err);
            err
        })?;

    let client = client_arc.lock().await;

    eprintln!("[LSP COMMAND] Calling local rust-analyzer with URI: {}", file_uri);

    // Call LSP client's goto_definition method with timeout
    let goto_future = client.goto_definition(&file_uri, line, character);

    eprintln!("[LSP COMMAND] Waiting for rust-analyzer response (60s timeout)...");
    let result = tokio::time::timeout(
        tokio::time::Duration::from_secs(60),
        goto_future
    ).await;

    eprintln!("[LSP COMMAND] rust-analyzer responded (or timed out)");

    match result {
        Ok(Ok(Some(location))) => {
            eprintln!("[LSP COMMAND] ✅ Definition found: uri={}, line={}, char={}",
                location.uri, location.range.start.line, location.range.start.character);
            Ok(location)
        }
        Ok(Ok(None)) => {
            eprintln!("[LSP COMMAND] ⚠️  No definition found, returning current position");
            // No definition found, return current position
            Ok(Location {
                uri: file_uri,
                range: Range {
                    start: Position { line, character },
                    end: Position { line, character },
                },
            })
        }
        Ok(Err(e)) => {
            eprintln!("[LSP COMMAND] ❌ LSP error: {}", e);
            Err(format!("LSP error: {}", e))
        }
        Err(_timeout) => {
            eprintln!("[LSP COMMAND] ⏱️  TIMEOUT: rust-analyzer did not respond within 60s");
            eprintln!("[LSP COMMAND] ⚠️  This may happen if rust-analyzer is still indexing the project");
            Err("Timeout: rust-analyzer did not respond (60s). It may still be indexing the project.".to_string())
        }
    }
}

/// Try goto_definition via berry_api gRPC service
async fn try_berry_api_goto_definition(
    file_uri: &str,
    line: u32,
    character: u32,
) -> anyhow::Result<Option<Location>> {
    eprintln!("[LSP COMMAND] 🔌 Connecting to berry_api at localhost:50051...");
    // Connect to berry_api server
    let client = BerryApiClient::connect("http://localhost:50051").await?;
    eprintln!("[LSP COMMAND] ✅ Connected to berry_api");

    // Get project root from file_uri
    let project_root = extract_project_root(file_uri)?;
    eprintln!("[LSP COMMAND] 📁 Project root: {}", project_root);

    // Initialize session
    let session_id = client.init_session(&project_root).await?;
    eprintln!("[LSP COMMAND] 🎫 Session initialized: {}", session_id);

    // Initialize LSP for rust (pass empty string for root_uri to use project_root from session)
    client.initialize_lsp("rust", "").await?;
    eprintln!("[LSP COMMAND] 🦀 LSP initialized for Rust");

    // Goto definition
    eprintln!("[LSP COMMAND] 🔍 Calling berry_api goto_definition...");
    let grpc_location = client.goto_definition(file_uri, line, character).await?;

    if let Some(ref loc) = grpc_location {
        eprintln!("[LSP COMMAND] ✅ berry_api returned: uri={}, line={}, char={}",
            loc.uri,
            loc.range.as_ref().and_then(|r| r.start.as_ref()).map(|p| p.line).unwrap_or(0),
            loc.range.as_ref().and_then(|r| r.start.as_ref()).map(|p| p.character).unwrap_or(0)
        );
    } else {
        eprintln!("[LSP COMMAND] ⚠️  berry_api returned None");
    }

    // Convert from berry_api_proto::Location to lsp_core::types::Location
    Ok(grpc_location.map(convert_grpc_location_to_lsp))
}

/// Convert berry_api_proto::Location to lsp_core::types::Location
fn convert_grpc_location_to_lsp(grpc_loc: crate::grpc_client::Location) -> Location {
    use crate::lsp_core::types::{Position, Range};

    let range = if let Some(grpc_range) = grpc_loc.range {
        let start = if let Some(grpc_start) = grpc_range.start {
            Position {
                line: grpc_start.line,
                character: grpc_start.character,
            }
        } else {
            Position { line: 0, character: 0 }
        };

        let end = if let Some(grpc_end) = grpc_range.end {
            Position {
                line: grpc_end.line,
                character: grpc_end.character,
            }
        } else {
            Position { line: 0, character: 0 }
        };

        Range { start, end }
    } else {
        Range {
            start: Position { line: 0, character: 0 },
            end: Position { line: 0, character: 0 },
        }
    };

    Location {
        uri: grpc_loc.uri,
        range,
    }
}

/// Extract project root from file URI
fn extract_project_root(file_uri: &str) -> anyhow::Result<String> {
    use std::path::PathBuf;

    let path = file_uri.strip_prefix("file://").unwrap_or(file_uri);
    let path_buf = PathBuf::from(path);

    // Find Cargo.toml by walking up the directory tree
    let mut current = path_buf.parent();
    while let Some(dir) = current {
        if dir.join("Cargo.toml").exists() {
            return Ok(dir.to_string_lossy().to_string());
        }
        current = dir.parent();
    }

    anyhow::bail!("Could not find Cargo.toml in parent directories")
}

/// Get diagnostics for a file
#[tauri::command]
pub async fn lsp_get_diagnostics(
    _language: String,
    _file_path: String,
    _manager: State<'_, Arc<LspManager>>,
) -> Result<Vec<Diagnostic>, String> {
    // Simplified implementation for now
    // Diagnostics are typically pushed from server, not pulled
    Ok(Vec::new())
}

/// Find all references
#[tauri::command]
pub async fn lsp_find_references(
    _language: String,
    _file_path: String,
    _line: u32,
    _character: u32,
    _manager: State<'_, Arc<LspManager>>,
) -> Result<Vec<Location>, String> {
    // Simplified implementation for now
    // Returning empty vector as placeholder
    Ok(Vec::new())
}

/// Shutdown LSP for a language
#[tauri::command]
pub async fn lsp_shutdown(
    language: String,
    manager: State<'_, Arc<LspManager>>,
) -> Result<bool, String> {
    manager.shutdown_client(&language).await?;
    Ok(true)
}

/// Add file to LSP context
#[tauri::command]
pub async fn lsp_add_file_to_context(
    language: String,
    file_path: String,
    manager: State<'_, Arc<LspManager>>,
) -> Result<bool, String> {
    eprintln!("[LSP COMMAND] ========================================");
    eprintln!("[LSP COMMAND] lsp_add_file_to_context CALLED");
    eprintln!("[LSP COMMAND] language={}, file={}", language, file_path);
    eprintln!("[LSP COMMAND] ========================================");

    let client_arc = manager
        .get_client(&language).await
        .ok_or_else(|| {
            let err = format!("LSP not initialized for {}", language);
            eprintln!("[LSP COMMAND] ❌ ERROR: {}", err);
            err
        })?;

    let client = client_arc.lock().await;

    // Convert file path to URI if needed
    let file_uri = if file_path.starts_with("file://") {
        file_path
    } else {
        format!("file://{}", file_path)
    };

    eprintln!("[LSP COMMAND] Adding file to context: {}", file_uri);

    match client.add_file_to_context(&file_uri).await {
        Ok(_) => {
            eprintln!("[LSP COMMAND] ✅ File added to context successfully");
            Ok(true)
        }
        Err(e) => {
            eprintln!("[LSP COMMAND] ⚠️  Failed to add file to context: {}", e);
            // Don't fail the command - LSP will still work without explicit context
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_uri_conversion() {
        let path = "/path/to/file.rs";
        let uri = format!("file://{}", path);
        assert_eq!(uri, "file:///path/to/file.rs");
    }

    #[test]
    fn test_file_uri_already_formatted() {
        let path = "file:///path/to/file.rs";
        let uri = if path.starts_with("file://") {
            path.to_string()
        } else {
            format!("file://{}", path)
        };
        assert_eq!(uri, "file:///path/to/file.rs");
    }
}
