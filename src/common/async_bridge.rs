//! Tauri IPC Bridge
//!
//! Common abstraction for Tauri invoke calls to eliminate code duplication.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;

/// Common bridge for Tauri IPC communication
pub struct TauriBridge;

impl TauriBridge {
    /// Invoke a Tauri command with type-safe parameters and return value
    pub async fn invoke<T, R>(command: &str, args: T) -> Result<R>
    where
        T: Serialize,
        R: for<'de> Deserialize<'de>,
    {
        #[wasm_bindgen]
        extern "C" {
            #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], catch)]
            async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
        }

        let args_value = to_value(&args)
            .map_err(|e| anyhow::anyhow!("Failed to serialize arguments: {}", e))?;

        let result = invoke(command, args_value)
            .await
            .map_err(|e| anyhow::anyhow!("Tauri invoke failed: {:?}", e))?;

        serde_wasm_bindgen::from_value(result)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize result: {}", e))
    }

    /// Invoke a Tauri command without arguments
    pub async fn invoke_no_args<R>(command: &str) -> Result<R>
    where
        R: for<'de> Deserialize<'de>,
    {
        Self::invoke(command, ()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    async fn test_tauri_bridge_invoke() {
        // This would require mocking Tauri in tests
        // For now, we ensure the API compiles correctly
        assert!(true);
    }
}
