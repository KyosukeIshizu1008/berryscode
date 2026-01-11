use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowPreset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub nodes_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStatus {
    pub execution_id: String,
    pub preset_id: String,
    pub status: ExecutionStatus,
    pub current_step: Option<String>,
    pub progress: f32,
    pub started_at: i64,
    pub completed_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionStatus {
    Running,
    Paused,
    Completed,
    Failed,
}

#[derive(Debug, Serialize)]
pub struct StartWorkflowRequest {
    pub preset_id: String,
    pub initial_prompt: String,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = berry_invoke, catch)]
    async fn tauri_invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

pub async fn workflow_list_presets() -> Result<Vec<WorkflowPreset>, String> {
    #[cfg(target_arch = "wasm32")]
    {
        let result = tauri_invoke("workflow_list_presets", JsValue::NULL)
            .await
            .map_err(|e| format!("Failed to list workflow presets: {:?}", e))?;
        serde_wasm_bindgen::from_value(result)
            .map_err(|e| format!("Failed to deserialize: {}", e))
    }
    #[cfg(not(target_arch = "wasm32"))]
    Ok(Vec::new())
}

pub async fn workflow_start(request: StartWorkflowRequest) -> Result<String, String> {
    #[cfg(target_arch = "wasm32")]
    {
        let args = serde_wasm_bindgen::to_value(&request)
            .map_err(|e| format!("Failed to serialize: {}", e))?;
        let result = tauri_invoke("workflow_start", args)
            .await
            .map_err(|e| format!("Failed to start workflow: {:?}", e))?;
        serde_wasm_bindgen::from_value(result)
            .map_err(|e| format!("Failed to deserialize: {}", e))
    }
    #[cfg(not(target_arch = "wasm32"))]
    Ok(String::new())
}

pub async fn workflow_get_status(execution_id: String) -> Result<Option<WorkflowStatus>, String> {
    #[cfg(target_arch = "wasm32")]
    {
        #[derive(Serialize)]
        struct Args {
            execution_id: String,
        }
        let args = serde_wasm_bindgen::to_value(&Args { execution_id })
            .map_err(|e| format!("Failed to serialize: {}", e))?;
        let result = tauri_invoke("workflow_get_status", args)
            .await
            .map_err(|e| format!("Failed to get workflow status: {:?}", e))?;
        serde_wasm_bindgen::from_value(result)
            .map_err(|e| format!("Failed to deserialize: {}", e))
    }
    #[cfg(not(target_arch = "wasm32"))]
    Ok(None)
}

pub async fn workflow_pause(execution_id: String) -> Result<(), String> {
    #[cfg(target_arch = "wasm32")]
    {
        #[derive(Serialize)]
        struct Args {
            execution_id: String,
        }
        let args = serde_wasm_bindgen::to_value(&Args { execution_id })
            .map_err(|e| format!("Failed to serialize: {}", e))?;
        tauri_invoke("workflow_pause", args)
            .await
            .map_err(|e| format!("Failed to pause workflow: {:?}", e))?;
        Ok(())
    }
    #[cfg(not(target_arch = "wasm32"))]
    Ok(())
}

pub async fn workflow_resume(execution_id: String) -> Result<(), String> {
    #[cfg(target_arch = "wasm32")]
    {
        #[derive(Serialize)]
        struct Args {
            execution_id: String,
        }
        let args = serde_wasm_bindgen::to_value(&Args { execution_id })
            .map_err(|e| format!("Failed to serialize: {}", e))?;
        tauri_invoke("workflow_resume", args)
            .await
            .map_err(|e| format!("Failed to resume workflow: {:?}", e))?;
        Ok(())
    }
    #[cfg(not(target_arch = "wasm32"))]
    Ok(())
}

pub async fn workflow_cancel(execution_id: String) -> Result<(), String> {
    #[cfg(target_arch = "wasm32")]
    {
        #[derive(Serialize)]
        struct Args {
            execution_id: String,
        }
        let args = serde_wasm_bindgen::to_value(&Args { execution_id })
            .map_err(|e| format!("Failed to serialize: {}", e))?;
        tauri_invoke("workflow_cancel", args)
            .await
            .map_err(|e| format!("Failed to cancel workflow: {:?}", e))?;
        Ok(())
    }
    #[cfg(not(target_arch = "wasm32"))]
    Ok(())
}
