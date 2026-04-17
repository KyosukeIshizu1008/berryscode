use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use super::ecs_state::EntityInfo;

/// JSON-RPC request for BRP
#[derive(Serialize)]
struct BrpRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<serde_json::Value>,
}

/// JSON-RPC response from BRP
#[derive(Deserialize)]
struct BrpResponse {
    #[allow(dead_code)]
    jsonrpc: String,
    #[allow(dead_code)]
    id: u64,
    #[serde(default)]
    result: Option<serde_json::Value>,
    #[serde(default)]
    error: Option<BrpError>,
}

#[derive(Deserialize)]
struct BrpError {
    code: i64,
    message: String,
}

/// BRP Client - communicates with a running Bevy application
pub struct BrpClient {
    endpoint: String,
    request_id: u64,
}

impl BrpClient {
    pub fn new(endpoint: &str) -> Self {
        Self {
            endpoint: endpoint.to_string(),
            request_id: 0,
        }
    }

    fn next_id(&mut self) -> u64 {
        self.request_id += 1;
        self.request_id
    }

    /// Send a BRP request (blocking - should be called from async context)
    pub async fn send_request(&mut self, method: &str, params: Option<serde_json::Value>) -> Result<serde_json::Value> {
        let id = self.next_id();
        let request = BrpRequest {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.to_string(),
            params,
        };

        let client = reqwest::Client::new();
        let response = client
            .post(&self.endpoint)
            .json(&request)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await?;

        let brp_response: BrpResponse = response.json().await?;

        if let Some(error) = brp_response.error {
            anyhow::bail!("BRP error {}: {}", error.code, error.message);
        }

        brp_response.result.ok_or_else(|| anyhow::anyhow!("No result in BRP response"))
    }

    /// List all entities with their components
    pub async fn list_entities(&mut self) -> Result<Vec<EntityInfo>> {
        // Use bevy/query to get all entities
        let result = self.send_request("bevy/query", Some(json!({
            "data": {
                "components": [],
                "option": ["bevy_core::name::Name"]
            }
        }))).await?;

        // Parse response into EntityInfo
        let mut entities = Vec::new();
        if let Some(rows) = result.as_array() {
            for row in rows {
                let id = row.get("entity")
                    .and_then(|e| e.as_u64())
                    .unwrap_or(0);
                let components = row.get("components")
                    .and_then(|c| c.as_object())
                    .map(|obj| obj.keys().cloned().collect())
                    .unwrap_or_default();
                let name = row.get("components")
                    .and_then(|c| c.get("bevy_core::name::Name"))
                    .and_then(|n| n.as_str())
                    .map(String::from);

                entities.push(EntityInfo { id, components, name });
            }
        }

        Ok(entities)
    }

    /// Get component values for an entity
    pub async fn get_entity_components(&mut self, entity_id: u64) -> Result<Vec<(String, serde_json::Value)>> {
        let result = self.send_request("bevy/get", Some(json!({
            "entity": entity_id
        }))).await?;

        let mut components = Vec::new();
        if let Some(obj) = result.as_object() {
            if let Some(comps) = obj.get("components").and_then(|c| c.as_object()) {
                for (name, value) in comps {
                    components.push((name.clone(), value.clone()));
                }
            }
        }

        Ok(components)
    }

    /// Check if connection is alive
    pub async fn ping(&mut self) -> bool {
        self.send_request("bevy/list", None).await.is_ok()
    }
}
