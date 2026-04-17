//! Shader graph: node-based material parameter graph.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShaderGraph {
    pub name: String,
    pub nodes: Vec<ShaderNode>,
    pub edges: Vec<ShaderEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShaderNode {
    pub id: u64,
    pub node_type: ShaderNodeType,
    pub position: [f32; 2],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShaderNodeType {
    OutputPBR, // Final output: color, metallic, roughness, emissive, normal
    TextureSample { path: String },
    ColorConstant { value: [f32; 4] },
    FloatConstant { value: f32 },
    Multiply,
    Add,
    Lerp,
    UVCoord,
    Time,
    Fresnel { power: f32 },
}

impl ShaderNodeType {
    pub fn label(&self) -> &'static str {
        match self {
            ShaderNodeType::OutputPBR => "PBR Output",
            ShaderNodeType::TextureSample { .. } => "Texture",
            ShaderNodeType::ColorConstant { .. } => "Color",
            ShaderNodeType::FloatConstant { .. } => "Float",
            ShaderNodeType::Multiply => "Multiply",
            ShaderNodeType::Add => "Add",
            ShaderNodeType::Lerp => "Lerp",
            ShaderNodeType::UVCoord => "UV",
            ShaderNodeType::Time => "Time",
            ShaderNodeType::Fresnel { .. } => "Fresnel",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShaderEdge {
    pub from_node: u64,
    pub from_pin: usize,
    pub to_node: u64,
    pub to_pin: usize,
}

impl Default for ShaderGraph {
    fn default() -> Self {
        Self {
            name: "New Shader".into(),
            nodes: vec![
                ShaderNode {
                    id: 1,
                    node_type: ShaderNodeType::ColorConstant {
                        value: [0.5, 0.5, 1.0, 1.0],
                    },
                    position: [100.0, 100.0],
                },
                ShaderNode {
                    id: 2,
                    node_type: ShaderNodeType::OutputPBR,
                    position: [400.0, 100.0],
                },
            ],
            edges: vec![ShaderEdge {
                from_node: 1,
                from_pin: 0,
                to_node: 2,
                to_pin: 0,
            }],
        }
    }
}

/// Evaluate the shader graph to produce PBR material parameters.
pub fn evaluate_graph(graph: &ShaderGraph) -> PbrParams {
    let mut params = PbrParams::default();
    // Simple topological evaluation: find OutputPBR node, trace inputs
    let output = graph
        .nodes
        .iter()
        .find(|n| matches!(n.node_type, ShaderNodeType::OutputPBR));
    let output_id = match output {
        Some(n) => n.id,
        None => return params,
    };

    // Pin 0 = base_color, Pin 1 = metallic, Pin 2 = roughness, Pin 3 = emissive
    for pin in 0..4 {
        if let Some(edge) = graph
            .edges
            .iter()
            .find(|e| e.to_node == output_id && e.to_pin == pin)
        {
            if let Some(source) = graph.nodes.iter().find(|n| n.id == edge.from_node) {
                match &source.node_type {
                    ShaderNodeType::ColorConstant { value } => match pin {
                        0 => params.base_color = [value[0], value[1], value[2]],
                        3 => params.emissive = [value[0], value[1], value[2]],
                        _ => {}
                    },
                    ShaderNodeType::FloatConstant { value } => match pin {
                        1 => params.metallic = *value,
                        2 => params.roughness = *value,
                        _ => {}
                    },
                    _ => {} // Complex nodes not evaluated in MVP
                }
            }
        }
    }
    params
}

#[derive(Debug, Clone)]
pub struct PbrParams {
    pub base_color: [f32; 3],
    pub metallic: f32,
    pub roughness: f32,
    pub emissive: [f32; 3],
}

impl Default for PbrParams {
    fn default() -> Self {
        Self {
            base_color: [1.0, 1.0, 1.0],
            metallic: 0.0,
            roughness: 0.5,
            emissive: [0.0, 0.0, 0.0],
        }
    }
}

pub fn save_shader_graph(graph: &ShaderGraph, path: &str) -> Result<(), String> {
    let s = ron::ser::to_string_pretty(graph, ron::ser::PrettyConfig::default())
        .map_err(|e| e.to_string())?;
    std::fs::write(path, s).map_err(|e| e.to_string())
}

pub fn load_shader_graph(path: &str) -> Result<ShaderGraph, String> {
    let s = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    ron::from_str(&s).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_graph_has_output() {
        let g = ShaderGraph::default();
        assert!(g
            .nodes
            .iter()
            .any(|n| matches!(n.node_type, ShaderNodeType::OutputPBR)));
    }
    #[test]
    fn evaluate_default_gets_color() {
        let g = ShaderGraph::default();
        let p = evaluate_graph(&g);
        assert!((p.base_color[2] - 1.0).abs() < 0.01); // blue from default ColorConstant
    }
    #[test]
    fn evaluate_empty_returns_defaults() {
        let g = ShaderGraph {
            name: "empty".into(),
            nodes: vec![],
            edges: vec![],
        };
        let p = evaluate_graph(&g);
        assert!((p.metallic - 0.0).abs() < 0.01);
    }
    #[test]
    fn ron_roundtrip() {
        let g = ShaderGraph::default();
        let s = ron::ser::to_string(&g).unwrap();
        let loaded: ShaderGraph = ron::from_str(&s).unwrap();
        assert_eq!(loaded.nodes.len(), g.nodes.len());
    }
}
