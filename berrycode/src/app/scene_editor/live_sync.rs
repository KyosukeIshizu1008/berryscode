//! Live sync: query component values from a running Bevy game via BRP.
//!
//! During Play Mode (external process started via `cargo run`), the editor
//! queries the game's ECS state through HTTP/BRP and displays live values in
//! the Inspector. This provides reflection-like behaviour without dynamic
//! library loading.
//!
//! The BRP endpoint defaults to `http://127.0.0.1:15702` (Bevy's default
//! remote port). The user can override this in the editor settings.

use std::collections::HashMap;

/// A snapshot of one entity's component values as returned by BRP.
#[derive(Debug, Clone)]
pub struct LiveComponentValue {
    /// Display name of the entity (from `Name` component, or entity id).
    pub entity_name: String,
    /// The component type name (e.g. `bevy_transform::components::Transform`).
    pub component_type: String,
    /// Flat map of `"component.field" -> stringified value`.
    pub fields: HashMap<String, String>,
}

/// Default BRP endpoint for a local Bevy game.
pub const DEFAULT_BRP_ENDPOINT: &str = "http://127.0.0.1:15702";

/// Query a running Bevy app's component values via BRP using `curl`.
///
/// Returns `None` if the app is not running, BRP is not available, or
/// the query fails for any reason. This function is intentionally
/// non-blocking-friendly: it spawns a short-lived `curl` process with a
/// 500ms timeout so the editor never hangs waiting for a dead game.
pub fn query_live_components(
    brp_endpoint: &str,
    entity_name: &str,
) -> Option<Vec<LiveComponentValue>> {
    let url = format!("{}/bevy/query", brp_endpoint);
    let body = format!(
        r#"{{"data":{{"components":["*"],"filter":{{"name":"{}"}}}}}}"#,
        entity_name
    );

    let output = std::process::Command::new("curl")
        .args([
            "-s",
            "--max-time",
            "1",
            "-X",
            "POST",
            &url,
            "-H",
            "Content-Type: application/json",
            "-d",
            &body,
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    let mut results = Vec::new();

    if let Some(entities) = json.as_array() {
        for entity in entities {
            let mut fields = HashMap::new();
            if let Some(components) = entity.get("components").and_then(|c| c.as_object()) {
                for (comp_name, comp_val) in components {
                    if let Some(obj) = comp_val.as_object() {
                        for (field_name, field_val) in obj {
                            fields.insert(
                                format!("{}.{}", comp_name, field_name),
                                field_val.to_string(),
                            );
                        }
                    } else {
                        // Scalar component value
                        fields.insert(comp_name.clone(), comp_val.to_string());
                    }
                }
            }
            results.push(LiveComponentValue {
                entity_name: entity_name.to_string(),
                component_type: "Mixed".to_string(),
                fields,
            });
        }
    }

    Some(results)
}

/// Check whether a BRP endpoint is reachable (game is running).
/// Returns `true` if the endpoint responds within 500ms.
pub fn is_game_running(brp_endpoint: &str) -> bool {
    let output = std::process::Command::new("curl")
        .args([
            "-s",
            "--max-time",
            "1",
            "-o",
            "/dev/null",
            "-w",
            "%{http_code}",
            brp_endpoint,
        ])
        .output();

    match output {
        Ok(o) => {
            let code = String::from_utf8_lossy(&o.stdout);
            code.trim() != "000" && o.status.success()
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_returns_none_when_no_server() {
        // Use an unreachable port so curl fails fast
        let result = query_live_components("http://127.0.0.1:1", "test");
        assert!(result.is_none());
    }

    #[test]
    fn is_game_running_returns_false_when_no_server() {
        assert!(!is_game_running("http://127.0.0.1:1"));
    }

    #[test]
    fn default_endpoint_is_valid() {
        assert!(DEFAULT_BRP_ENDPOINT.starts_with("http://"));
        assert!(DEFAULT_BRP_ENDPOINT.contains("15702"));
    }

    #[test]
    fn live_component_value_debug() {
        let val = LiveComponentValue {
            entity_name: "Player".into(),
            component_type: "Transform".into(),
            fields: HashMap::from([("translation.x".into(), "1.0".into())]),
        };
        let debug = format!("{:?}", val);
        assert!(debug.contains("Player"));
    }
}
