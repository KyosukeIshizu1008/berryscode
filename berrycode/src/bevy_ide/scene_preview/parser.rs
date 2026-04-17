use serde::{Deserialize, Serialize};

/// Parsed scene entity for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneEntity {
    pub entity_id: u64,
    pub components: Vec<SceneComponent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneComponent {
    pub type_name: String,
    pub properties: Vec<(String, String)>,
}

/// Scene preview state
pub struct ScenePreviewState {
    pub entities: Vec<SceneEntity>,
    pub parse_error: Option<String>,
    pub last_content_hash: u64,
    pub selected_entity: Option<usize>,
}

impl Default for ScenePreviewState {
    fn default() -> Self {
        Self {
            entities: Vec::new(),
            parse_error: None,
            last_content_hash: 0,
            selected_entity: None,
        }
    }
}

/// Parse a RON scene file content and extract entity/component structure
pub fn parse_scene_ron(content: &str) -> Result<Vec<SceneEntity>, String> {
    // Parse RON as a generic Value first
    let value: ron::Value =
        ron::from_str(content).map_err(|e| format!("RON parse error: {}", e))?;

    let mut entities = Vec::new();

    // Bevy scene format: ( entities: [ ( entity: 0, components: [ ... ] ), ... ] )
    // or newer format with named structs
    match &value {
        ron::Value::Map(map) => {
            // Look for "entities" key
            for (key, val) in map.iter() {
                if let ron::Value::String(k) = key {
                    if k == "entities" {
                        if let ron::Value::Seq(ents) = val {
                            for (idx, ent) in ents.iter().enumerate() {
                                entities.push(parse_entity(ent, idx as u64));
                            }
                        }
                    }
                }
            }
        }
        ron::Value::Seq(seq) => {
            // Could be a list of entities directly
            for (idx, ent) in seq.iter().enumerate() {
                entities.push(parse_entity(ent, idx as u64));
            }
        }
        _ => {
            // Try to extract something meaningful
            entities.push(SceneEntity {
                entity_id: 0,
                components: vec![SceneComponent {
                    type_name: "Root".to_string(),
                    properties: vec![("value".to_string(), format!("{:?}", value))],
                }],
            });
        }
    }

    Ok(entities)
}

fn parse_entity(value: &ron::Value, default_id: u64) -> SceneEntity {
    let mut entity_id = default_id;
    let mut components = Vec::new();

    match value {
        ron::Value::Map(map) => {
            for (key, val) in map.iter() {
                let key_str = match key {
                    ron::Value::String(s) => s.clone(),
                    _ => format!("{:?}", key),
                };

                if key_str == "entity" {
                    if let ron::Value::Number(n) = val {
                        entity_id = n.into_f64() as u64;
                    }
                } else if key_str == "components" {
                    if let ron::Value::Seq(comps) = val {
                        for comp in comps {
                            components.push(parse_component(comp));
                        }
                    } else if let ron::Value::Map(comp_map) = val {
                        for (comp_name, comp_val) in comp_map.iter() {
                            let name = match comp_name {
                                ron::Value::String(s) => s.clone(),
                                _ => format!("{:?}", comp_name),
                            };
                            components.push(SceneComponent {
                                type_name: name,
                                properties: extract_properties(comp_val),
                            });
                        }
                    }
                } else {
                    // Treat as a component
                    components.push(SceneComponent {
                        type_name: key_str,
                        properties: extract_properties(val),
                    });
                }
            }
        }
        _ => {
            components.push(SceneComponent {
                type_name: "Unknown".to_string(),
                properties: vec![("value".to_string(), format!("{:?}", value))],
            });
        }
    }

    SceneEntity {
        entity_id,
        components,
    }
}

fn parse_component(value: &ron::Value) -> SceneComponent {
    match value {
        ron::Value::Map(map) => {
            let mut type_name = "Unknown".to_string();
            let mut properties = Vec::new();

            for (key, val) in map.iter() {
                let key_str = match key {
                    ron::Value::String(s) => s.clone(),
                    _ => format!("{:?}", key),
                };

                if key_str == "type" {
                    type_name = match val {
                        ron::Value::String(s) => s.clone(),
                        _ => format!("{:?}", val),
                    };
                } else {
                    properties.push((key_str, format_ron_value(val)));
                }
            }

            SceneComponent {
                type_name,
                properties,
            }
        }
        _ => SceneComponent {
            type_name: format!("{:?}", value),
            properties: Vec::new(),
        },
    }
}

fn extract_properties(value: &ron::Value) -> Vec<(String, String)> {
    match value {
        ron::Value::Map(map) => map
            .iter()
            .map(|(k, v)| {
                let key = match k {
                    ron::Value::String(s) => s.clone(),
                    _ => format!("{:?}", k),
                };
                (key, format_ron_value(v))
            })
            .collect(),
        _ => vec![("value".to_string(), format_ron_value(value))],
    }
}

fn format_ron_value(value: &ron::Value) -> String {
    match value {
        ron::Value::Number(n) => format!("{}", n.into_f64()),
        ron::Value::String(s) => format!("\"{}\"", s),
        ron::Value::Bool(b) => format!("{}", b),
        ron::Value::Seq(seq) => {
            let items: Vec<String> = seq.iter().map(format_ron_value).collect();
            format!("[{}]", items.join(", "))
        }
        ron::Value::Map(map) => {
            let items: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("{}: {}", format_ron_value(k), format_ron_value(v)))
                .collect();
            format!("{{{}}}", items.join(", "))
        }
        ron::Value::Unit => "()".to_string(),
        ron::Value::Option(opt) => match opt {
            Some(v) => format!("Some({})", format_ron_value(v)),
            None => "None".to_string(),
        },
        ron::Value::Char(c) => format!("'{}'", c),
    }
}
