use std::path::PathBuf;
use tauri::State;

use super::manager::TerminalManagerState;
use super::types::*;
use crate::persistent_terminal::ProcessStatus as CoreProcessStatus;

/// Execute command in persistent terminal
#[tauri::command]
pub async fn terminal_execute_command(
    project_path: String,
    command: String,
    background: Option<bool>,
    terminal_manager: State<'_, TerminalManagerState>,
) -> Result<TerminalCommandResponse, String> {
    let project_path_buf = PathBuf::from(project_path);

    // Get or create terminal
    let terminal = terminal_manager
        .get_or_create_terminal(project_path_buf)
        .await
        .map_err(|e| format!("Failed to get terminal: {}", e))?;

    // Execute command
    let result = if background.unwrap_or(false) {
        // Execute in background
        terminal
            .execute_background(&command)
            .await
            .map_err(|e| format!("Failed to execute background command: {}", e))?
    } else {
        // Execute normally
        terminal
            .execute(&command)
            .await
            .map_err(|e| format!("Failed to execute command: {}", e))?
    };

    let process_id = if background.unwrap_or(false) {
        Some(result.clone())
    } else {
        None
    };

    Ok(TerminalCommandResponse {
        output: result,
        success: true,
        process_id,
    })
}

/// Get command history
#[tauri::command]
pub async fn terminal_get_history(
    project_path: String,
    terminal_manager: State<'_, TerminalManagerState>,
) -> Result<Vec<String>, String> {
    let project_path_buf = PathBuf::from(project_path);

    let terminal = terminal_manager
        .get_or_create_terminal(project_path_buf)
        .await
        .map_err(|e| format!("Failed to get terminal: {}", e))?;

    let history = terminal.get_history().await;

    Ok(history)
}

/// List background processes
#[tauri::command]
pub async fn terminal_list_background_processes(
    project_path: String,
    terminal_manager: State<'_, TerminalManagerState>,
) -> Result<Vec<BackgroundProcessInfo>, String> {
    let project_path_buf = PathBuf::from(project_path);

    let terminal = terminal_manager
        .get_or_create_terminal(project_path_buf)
        .await
        .map_err(|e| format!("Failed to get terminal: {}", e))?;

    let processes = terminal.list_background_processes().await;

    let process_info: Vec<BackgroundProcessInfo> = processes
        .into_iter()
        .map(|p| BackgroundProcessInfo {
            id: p.id,
            command: p.command,
            pid: p.pid,
            status: match p.status {
                CoreProcessStatus::Running => "running".to_string(),
                CoreProcessStatus::Completed(code) => format!("completed ({})", code),
                CoreProcessStatus::Failed(msg) => format!("failed: {}", msg),
            },
            output_lines: p.output_buffer,
        })
        .collect();

    Ok(process_info)
}

/// Kill a background process
#[tauri::command]
pub async fn terminal_kill_process(
    project_path: String,
    process_id: String,
    terminal_manager: State<'_, TerminalManagerState>,
) -> Result<(), String> {
    let project_path_buf = PathBuf::from(project_path);

    let terminal = terminal_manager
        .get_or_create_terminal(project_path_buf)
        .await
        .map_err(|e| format!("Failed to get terminal: {}", e))?;

    terminal
        .kill_process(&process_id)
        .await
        .map_err(|e| format!("Failed to kill process: {}", e))?;

    Ok(())
}

/// Change working directory
#[tauri::command]
pub async fn terminal_change_directory(
    project_path: String,
    path: String,
    terminal_manager: State<'_, TerminalManagerState>,
) -> Result<(), String> {
    let project_path_buf = PathBuf::from(project_path);

    let terminal = terminal_manager
        .get_or_create_terminal(project_path_buf)
        .await
        .map_err(|e| format!("Failed to get terminal: {}", e))?;

    terminal
        .cd(&path)
        .await
        .map_err(|e| format!("Failed to change directory: {}", e))?;

    Ok(())
}

/// Get current working directory
#[tauri::command]
pub async fn terminal_get_current_directory(
    project_path: String,
    terminal_manager: State<'_, TerminalManagerState>,
) -> Result<String, String> {
    let project_path_buf = PathBuf::from(project_path);

    let terminal = terminal_manager
        .get_or_create_terminal(project_path_buf)
        .await
        .map_err(|e| format!("Failed to get terminal: {}", e))?;

    let cwd = terminal
        .get_cwd()
        .await
        .map_err(|e| format!("Failed to get current directory: {}", e))?;

    Ok(cwd.to_string_lossy().to_string())
}
