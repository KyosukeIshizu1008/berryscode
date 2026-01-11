use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// Terminal command response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalCommandResponse {
    pub output: String,
    pub success: bool,
    pub process_id: Option<String>,
}

/// Background process info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundProcessInfo {
    pub id: String,
    pub command: String,
    pub pid: u32,
    pub status: String,
    pub output_lines: Vec<String>,
}

// Tauri invoke bridge (defined in index.html)
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = berry_invoke, catch)]
    async fn tauri_invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

// Check if running in Tauri context
#[cfg(target_arch = "wasm32")]
fn is_tauri_context() -> bool {
    use wasm_bindgen::JsCast;
    if let Some(window) = web_sys::window() {
        let js_val = js_sys::Reflect::get(&window, &"berry_invoke".into()).ok();
        if js_val.is_some() && !js_val.unwrap().is_undefined() {
            return true;
        }
        let js_val = js_sys::Reflect::get(&window, &"__TAURI_INTERNALS__".into()).ok();
        return js_val.is_some() && !js_val.unwrap().is_undefined();
    }
    false
}

/// Execute command in persistent terminal
#[cfg(target_arch = "wasm32")]
pub async fn terminal_execute_command(
    project_path: String,
    command: String,
    background: Option<bool>,
) -> Result<TerminalCommandResponse, String> {
    if !is_tauri_context() {
        return Err("Terminal not available in web mode".to_string());
    }

    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "projectPath": project_path,
        "command": command,
        "background": background,
    }))
    .map_err(|e| format!("Failed to serialize args: {}", e))?;

    let result = tauri_invoke("terminal_execute_command", args)
        .await
        .map_err(|e| format!("Failed to execute command: {:?}", e))?;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Failed to deserialize result: {}", e))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn terminal_execute_command(
    _project_path: String,
    _command: String,
    _background: Option<bool>,
) -> Result<TerminalCommandResponse, String> {
    Err("Terminal only available in WASM context".to_string())
}

/// Get command history
#[cfg(target_arch = "wasm32")]
pub async fn terminal_get_history(project_path: String) -> Result<Vec<String>, String> {
    if !is_tauri_context() {
        return Ok(vec![]);
    }

    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "projectPath": project_path,
    }))
    .map_err(|e| format!("Failed to serialize args: {}", e))?;

    let result = tauri_invoke("terminal_get_history", args)
        .await
        .map_err(|e| format!("Failed to get history: {:?}", e))?;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Failed to deserialize result: {}", e))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn terminal_get_history(_project_path: String) -> Result<Vec<String>, String> {
    Err("Terminal only available in WASM context".to_string())
}

/// List background processes
#[cfg(target_arch = "wasm32")]
pub async fn terminal_list_background_processes(
    project_path: String,
) -> Result<Vec<BackgroundProcessInfo>, String> {
    if !is_tauri_context() {
        return Ok(vec![]);
    }

    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "projectPath": project_path,
    }))
    .map_err(|e| format!("Failed to serialize args: {}", e))?;

    let result = tauri_invoke("terminal_list_background_processes", args)
        .await
        .map_err(|e| format!("Failed to list processes: {:?}", e))?;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Failed to deserialize result: {}", e))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn terminal_list_background_processes(
    _project_path: String,
) -> Result<Vec<BackgroundProcessInfo>, String> {
    Err("Terminal only available in WASM context".to_string())
}

/// Kill a background process
#[cfg(target_arch = "wasm32")]
pub async fn terminal_kill_process(project_path: String, process_id: String) -> Result<(), String> {
    if !is_tauri_context() {
        return Err("Terminal not available in web mode".to_string());
    }

    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "projectPath": project_path,
        "processId": process_id,
    }))
    .map_err(|e| format!("Failed to serialize args: {}", e))?;

    tauri_invoke("terminal_kill_process", args)
        .await
        .map_err(|e| format!("Failed to kill process: {:?}", e))?;

    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn terminal_kill_process(_project_path: String, _process_id: String) -> Result<(), String> {
    Err("Terminal only available in WASM context".to_string())
}

/// Change working directory
#[cfg(target_arch = "wasm32")]
pub async fn terminal_change_directory(project_path: String, path: String) -> Result<(), String> {
    if !is_tauri_context() {
        return Err("Terminal not available in web mode".to_string());
    }

    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "projectPath": project_path,
        "path": path,
    }))
    .map_err(|e| format!("Failed to serialize args: {}", e))?;

    tauri_invoke("terminal_change_directory", args)
        .await
        .map_err(|e| format!("Failed to change directory: {:?}", e))?;

    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn terminal_change_directory(_project_path: String, _path: String) -> Result<(), String> {
    Err("Terminal only available in WASM context".to_string())
}

/// Get current working directory
#[cfg(target_arch = "wasm32")]
pub async fn terminal_get_current_directory(project_path: String) -> Result<String, String> {
    if !is_tauri_context() {
        return Ok("~".to_string());
    }

    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "projectPath": project_path,
    }))
    .map_err(|e| format!("Failed to serialize args: {}", e))?;

    let result = tauri_invoke("terminal_get_current_directory", args)
        .await
        .map_err(|e| format!("Failed to get current directory: {:?}", e))?;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Failed to deserialize result: {}", e))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn terminal_get_current_directory(_project_path: String) -> Result<String, String> {
    Err("Terminal only available in WASM context".to_string())
}
