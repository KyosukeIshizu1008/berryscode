use super::ecs_state::EntityInfo;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;

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
    pub async fn send_request(
        &mut self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value> {
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

        brp_response
            .result
            .ok_or_else(|| anyhow::anyhow!("No result in BRP response"))
    }

    /// List named entities (user-created, not internal Bevy observers).
    ///
    /// Single BRP call. Component lists are fetched lazily on selection.
    pub async fn list_entities(&mut self) -> Result<Vec<EntityInfo>> {
        let result = self
            .send_request(
                "bevy/query",
                Some(json!({
                    "data": {
                        "components": ["bevy_core::name::Name"]
                    }
                })),
            )
            .await?;

        let entities = Self::parse_query_response(&result);
        Ok(entities)
    }

    /// Get component values for an entity
    pub async fn get_entity_components(
        &mut self,
        entity_id: u64,
        component_names: &[String],
    ) -> Result<Vec<(String, serde_json::Value)>> {
        // If no component names provided, first discover them via bevy/list
        let names: Vec<String> = if component_names.is_empty() {
            let list = self
                .send_request("bevy/list", Some(json!({"entity": entity_id})))
                .await?;
            list.as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default()
        } else {
            component_names.to_vec()
        };

        if names.is_empty() {
            return Ok(Vec::new());
        }

        // Filter out internal Bevy types that don't support reflect/serialization
        let filtered: Vec<&String> = names
            .iter()
            .filter(|n| {
                !n.contains("observer::runner")
                    && !n.contains("ObserverState")
                    && !n.contains("On<")
                    && !n.contains("EventMeta")
            })
            .collect();

        if filtered.is_empty() {
            return Ok(Vec::new());
        }

        let result = self
            .send_request(
                "bevy/get",
                Some(json!({
                    "entity": entity_id,
                    "components": filtered
                })),
            )
            .await?;

        let mut components = Vec::new();
        if let Some(obj) = result.as_object() {
            // BRP response format: { "components": {...}, "errors": {...} }
            if let Some(comps) = obj.get("components").and_then(|c| c.as_object()) {
                for (name, value) in comps {
                    components.push((name.clone(), value.clone()));
                }
            } else {
                // Fallback: treat top-level keys as components (skip errors)
                for (name, value) in obj {
                    if name != "errors" {
                        components.push((name.clone(), value.clone()));
                    }
                }
            }
        }

        Ok(components)
    }

    /// Check if connection is alive
    pub async fn ping(&mut self) -> bool {
        self.send_request("bevy/list", None).await.is_ok()
    }

    /// Parse query response into EntityInfo list (public for testing)
    pub fn parse_query_response(result: &serde_json::Value) -> Vec<EntityInfo> {
        let mut entities = Vec::new();
        if let Some(rows) = result.as_array() {
            for row in rows {
                let id = row.get("entity").and_then(|e| e.as_u64()).unwrap_or(0);
                let name = row
                    .get("components")
                    .and_then(|c| c.get("bevy_core::name::Name"))
                    .and_then(|n| n.get("name"))
                    .and_then(|n| n.as_str())
                    .map(String::from);
                entities.push(EntityInfo {
                    id,
                    components: Vec::new(),
                    name,
                });
            }
        }
        entities
    }

    /// Parse get response into component list (public for testing)
    pub fn parse_get_response(result: &serde_json::Value) -> Vec<(String, serde_json::Value)> {
        let mut components = Vec::new();
        if let Some(obj) = result.as_object() {
            if let Some(comps) = obj.get("components").and_then(|c| c.as_object()) {
                for (name, value) in comps {
                    components.push((name.clone(), value.clone()));
                }
            }
        }
        components
    }

    /// Parse list response into component names (public for testing)
    pub fn parse_list_response(result: &serde_json::Value) -> Vec<String> {
        result
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Filter out internal Bevy types that don't support reflect
    pub fn filter_internal_components(names: &[String]) -> Vec<String> {
        names
            .iter()
            .filter(|n| {
                !n.contains("observer::runner")
                    && !n.contains("ObserverState")
                    && !n.contains("On<")
                    && !n.contains("EventMeta")
            })
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_query_response_with_names() {
        let response = json!([
            {
                "components": {
                    "bevy_core::name::Name": {"hash": 123, "name": "Box 1"}
                },
                "entity": 100
            },
            {
                "components": {
                    "bevy_core::name::Name": {"hash": 456, "name": "Player"}
                },
                "entity": 200
            }
        ]);

        let entities = BrpClient::parse_query_response(&response);
        assert_eq!(entities.len(), 2);
        assert_eq!(entities[0].id, 100);
        assert_eq!(entities[0].name, Some("Box 1".to_string()));
        assert_eq!(entities[1].id, 200);
        assert_eq!(entities[1].name, Some("Player".to_string()));
    }

    #[test]
    fn test_parse_query_response_empty() {
        let response = json!([]);
        let entities = BrpClient::parse_query_response(&response);
        assert_eq!(entities.len(), 0);
    }

    #[test]
    fn test_parse_get_response() {
        let response = json!({
            "components": {
                "bevy_core::name::Name": {"hash": 123, "name": "Box 1"},
                "bevy_transform::components::transform::Transform": {
                    "translation": [4.0, 0.75, 0.0],
                    "rotation": [0.0, 0.0, 0.0, 1.0],
                    "scale": [1.0, 1.0, 1.0]
                }
            },
            "errors": {}
        });

        let components = BrpClient::parse_get_response(&response);
        assert_eq!(components.len(), 2);
        let names: Vec<&str> = components.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"bevy_core::name::Name"));
        assert!(names.contains(&"bevy_transform::components::transform::Transform"));
    }

    #[test]
    fn test_parse_get_response_with_errors() {
        let response = json!({
            "components": {
                "bevy_core::name::Name": {"hash": 123, "name": "Test"}
            },
            "errors": {
                "bevy_ecs::observer::runner::Observer": {
                    "code": -23002,
                    "message": "Unknown component type"
                }
            }
        });

        let components = BrpClient::parse_get_response(&response);
        assert_eq!(components.len(), 1);
        assert_eq!(components[0].0, "bevy_core::name::Name");
    }

    #[test]
    fn test_parse_get_response_empty() {
        let response = json!({
            "components": {},
            "errors": {}
        });

        let components = BrpClient::parse_get_response(&response);
        assert_eq!(components.len(), 0);
    }

    #[test]
    fn test_parse_list_response() {
        let response = json!([
            "bevy_core::name::Name",
            "bevy_transform::components::transform::Transform",
            "bevy_render::mesh::components::Mesh3d"
        ]);

        let names = BrpClient::parse_list_response(&response);
        assert_eq!(names.len(), 3);
        assert_eq!(names[0], "bevy_core::name::Name");
    }

    #[test]
    fn test_parse_list_response_empty() {
        let response = json!([]);
        let names = BrpClient::parse_list_response(&response);
        assert_eq!(names.len(), 0);
    }

    #[test]
    fn test_filter_internal_components() {
        let names = vec![
            "bevy_core::name::Name".to_string(),
            "bevy_transform::components::transform::Transform".to_string(),
            "bevy_ecs::observer::runner::Observer".to_string(),
            "bevy_ecs::observer::runner::ObserverState".to_string(),
            "bevy_render::view::visibility::Visibility".to_string(),
        ];

        let filtered = BrpClient::filter_internal_components(&names);
        assert_eq!(filtered.len(), 3);
        assert!(!filtered.iter().any(|n| n.contains("observer::runner")));
        assert!(!filtered.iter().any(|n| n.contains("ObserverState")));
    }

    #[test]
    fn test_filter_keeps_all_normal_components() {
        let names = vec![
            "bevy_core::name::Name".to_string(),
            "bevy_transform::components::transform::Transform".to_string(),
        ];

        let filtered = BrpClient::filter_internal_components(&names);
        assert_eq!(filtered.len(), 2);
    }
}
