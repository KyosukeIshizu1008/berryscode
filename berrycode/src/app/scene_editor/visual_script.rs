//! Visual scripting: node-based logic graph data model.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualScript {
    pub name: String,
    pub nodes: Vec<ScriptNode>,
    pub edges: Vec<ScriptEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptNode {
    pub id: u64,
    pub node_type: NodeType,
    pub position: [f32; 2],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeType {
    OnStart,
    OnUpdate,
    Branch,
    Print { message: String },
    SetTransform,
    GetTransform,
    FloatAdd,
    FloatCompare { threshold: f32 },
    Delay { seconds: f32 },
    SpawnEntity { entity_name: String },
}

impl NodeType {
    pub fn label(&self) -> &'static str {
        match self {
            NodeType::OnStart => "On Start",
            NodeType::OnUpdate => "On Update",
            NodeType::Branch => "Branch",
            NodeType::Print { .. } => "Print",
            NodeType::SetTransform => "Set Transform",
            NodeType::GetTransform => "Get Transform",
            NodeType::FloatAdd => "Float Add",
            NodeType::FloatCompare { .. } => "Float Compare",
            NodeType::Delay { .. } => "Delay",
            NodeType::SpawnEntity { .. } => "Spawn Entity",
        }
    }

    pub fn input_count(&self) -> usize {
        match self {
            NodeType::OnStart | NodeType::OnUpdate => 0,
            NodeType::Branch
            | NodeType::Print { .. }
            | NodeType::SetTransform
            | NodeType::Delay { .. }
            | NodeType::SpawnEntity { .. } => 1,
            NodeType::FloatAdd | NodeType::FloatCompare { .. } => 2,
            NodeType::GetTransform => 1,
        }
    }

    pub fn output_count(&self) -> usize {
        match self {
            NodeType::OnStart | NodeType::OnUpdate => 1,
            NodeType::Branch => 2, // true/false
            NodeType::Print { .. }
            | NodeType::SetTransform
            | NodeType::Delay { .. }
            | NodeType::SpawnEntity { .. } => 1,
            NodeType::FloatAdd => 1,
            NodeType::FloatCompare { .. } => 1,
            NodeType::GetTransform => 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptEdge {
    pub from_node: u64,
    pub from_pin: usize,
    pub to_node: u64,
    pub to_pin: usize,
}

impl Default for VisualScript {
    fn default() -> Self {
        Self {
            name: "New Script".into(),
            nodes: vec![ScriptNode {
                id: 1,
                node_type: NodeType::OnStart,
                position: [100.0, 100.0],
            }],
            edges: vec![],
        }
    }
}

pub fn save_visual_script(script: &VisualScript, path: &str) -> Result<(), String> {
    let s = ron::ser::to_string_pretty(script, ron::ser::PrettyConfig::default())
        .map_err(|e| e.to_string())?;
    std::fs::write(path, s).map_err(|e| e.to_string())
}

pub fn load_visual_script(path: &str) -> Result<VisualScript, String> {
    let s = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    ron::from_str(&s).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_has_on_start() {
        let s = VisualScript::default();
        assert_eq!(s.nodes.len(), 1);
        assert!(matches!(s.nodes[0].node_type, NodeType::OnStart));
    }
    #[test]
    fn ron_roundtrip() {
        let mut s = VisualScript::default();
        s.nodes.push(ScriptNode {
            id: 2,
            node_type: NodeType::Print {
                message: "Hello".into(),
            },
            position: [300.0, 100.0],
        });
        s.edges.push(ScriptEdge {
            from_node: 1,
            from_pin: 0,
            to_node: 2,
            to_pin: 0,
        });
        let ron_str =
            ron::ser::to_string_pretty(&s, ron::ser::PrettyConfig::default()).unwrap();
        let loaded: VisualScript = ron::from_str(&ron_str).unwrap();
        assert_eq!(loaded.nodes.len(), 2);
        assert_eq!(loaded.edges.len(), 1);
    }
    #[test]
    fn node_type_labels() {
        assert_eq!(NodeType::OnStart.label(), "On Start");
        assert_eq!(NodeType::FloatAdd.label(), "Float Add");
    }
}
