use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DbType {
    PostgreSQL,
    MySQL,
    SQLite,
    MongoDB,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbConnection {
    pub id: String,
    pub name: String,
    pub db_type: DbType,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub database: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub ssl: bool,
    pub created_at: i64,
    pub last_used: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionTestResult {
    pub success: bool,
    pub message: String,
    pub latency_ms: Option<u64>,
    pub server_version: Option<String>,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = berry_invoke, catch)]
    async fn tauri_invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

pub async fn db_list_connections() -> Result<Vec<DbConnection>, String> {
    #[cfg(target_arch = "wasm32")]
    {
        let result = tauri_invoke("db_list_connections", JsValue::NULL)
            .await
            .map_err(|e| format!("Failed to list DB connections: {:?}", e))?;
        serde_wasm_bindgen::from_value(result)
            .map_err(|e| format!("Failed to deserialize: {}", e))
    }
    #[cfg(not(target_arch = "wasm32"))]
    Ok(Vec::new())
}

pub async fn db_add_connection(connection: DbConnection) -> Result<(), String> {
    #[cfg(target_arch = "wasm32")]
    {
        let args = serde_wasm_bindgen::to_value(&connection)
            .map_err(|e| format!("Failed to serialize: {}", e))?;
        tauri_invoke("db_add_connection", args)
            .await
            .map_err(|e| format!("Failed to add DB connection: {:?}", e))?;
        Ok(())
    }
    #[cfg(not(target_arch = "wasm32"))]
    Ok(())
}

pub async fn db_update_connection(connection: DbConnection) -> Result<(), String> {
    #[cfg(target_arch = "wasm32")]
    {
        let args = serde_wasm_bindgen::to_value(&connection)
            .map_err(|e| format!("Failed to serialize: {}", e))?;
        tauri_invoke("db_update_connection", args)
            .await
            .map_err(|e| format!("Failed to update DB connection: {:?}", e))?;
        Ok(())
    }
    #[cfg(not(target_arch = "wasm32"))]
    Ok(())
}

pub async fn db_delete_connection(connection_id: String) -> Result<(), String> {
    #[cfg(target_arch = "wasm32")]
    {
        #[derive(Serialize)]
        struct Args {
            connection_id: String,
        }
        let args = serde_wasm_bindgen::to_value(&Args { connection_id })
            .map_err(|e| format!("Failed to serialize: {}", e))?;
        tauri_invoke("db_delete_connection", args)
            .await
            .map_err(|e| format!("Failed to delete DB connection: {:?}", e))?;
        Ok(())
    }
    #[cfg(not(target_arch = "wasm32"))]
    Ok(())
}

pub async fn db_test_connection(connection: DbConnection) -> Result<ConnectionTestResult, String> {
    #[cfg(target_arch = "wasm32")]
    {
        let args = serde_wasm_bindgen::to_value(&connection)
            .map_err(|e| format!("Failed to serialize: {}", e))?;
        let result = tauri_invoke("db_test_connection", args)
            .await
            .map_err(|e| format!("Failed to test DB connection: {:?}", e))?;
        serde_wasm_bindgen::from_value(result)
            .map_err(|e| format!("Failed to deserialize: {}", e))
    }
    #[cfg(not(target_arch = "wasm32"))]
    Ok(ConnectionTestResult {
        success: false,
        message: "Not in WASM environment".to_string(),
        latency_ms: None,
        server_version: None,
    })
}
