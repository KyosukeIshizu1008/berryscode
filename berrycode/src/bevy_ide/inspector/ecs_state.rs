use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// State of the ECS Inspector panel
pub struct EcsInspectorState {
    pub connected: bool,
    pub endpoint: String,
    pub entities: Vec<EntityInfo>,
    pub resources: Vec<ResourceInfo>,
    pub selected_entity: Option<u64>,
    pub selected_resource: Option<String>,
    pub component_values: HashMap<(u64, String), serde_json::Value>,
    pub resource_values: HashMap<String, serde_json::Value>,
    pub poll_interval_ms: u64,
    pub last_poll: Option<std::time::Instant>,
    pub filter_query: String,
    pub error_message: Option<String>,
    pub auto_refresh: bool,
    /// 3D view camera state
    pub view_yaw: f32,
    pub view_pitch: f32,
    pub view_zoom: f32,
    /// Pending async results
    pub pending_connect: Option<std::sync::mpsc::Receiver<bool>>,
    pub pending_entities: Option<std::sync::mpsc::Receiver<anyhow::Result<Vec<EntityInfo>>>>,
    pub pending_components:
        Option<std::sync::mpsc::Receiver<anyhow::Result<Vec<(String, serde_json::Value)>>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityInfo {
    pub id: u64,
    pub components: Vec<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceInfo {
    pub type_name: String,
}

impl Default for EcsInspectorState {
    fn default() -> Self {
        Self {
            connected: false,
            endpoint: "http://127.0.0.1:15702".to_string(),
            entities: Vec::new(),
            resources: Vec::new(),
            selected_entity: None,
            selected_resource: None,
            component_values: HashMap::new(),
            resource_values: HashMap::new(),
            poll_interval_ms: 500,
            last_poll: None,
            filter_query: String::new(),
            error_message: None,
            auto_refresh: true,
            view_yaw: std::f32::consts::PI * 0.25,
            view_pitch: std::f32::consts::PI * 0.15,
            view_zoom: 1.0,
            pending_connect: None,
            pending_entities: None,
            pending_components: None,
        }
    }
}
