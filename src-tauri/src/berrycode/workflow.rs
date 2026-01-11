//! Workflow engine for BerryFlow autonomous development pipelines
//!
//! This module defines the workflow structure, node types (agents), and
//! execution logic for automated development workflows.

use serde::{Serialize, Deserialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use crate::berrycode::Result;

/// Workflow definition with nodes and connections
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub nodes: Vec<WorkflowNode>,
    pub connections: Vec<Connection>,
    #[serde(default)]
    pub variables: HashMap<String, serde_json::Value>,
}

/// Individual workflow node (agent task)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNode {
    pub id: usize,
    pub node_type: NodeType,
    pub name: String,
    pub icon: String,
    pub config: NodeConfig,
    #[serde(default)]
    pub position: Position,
}

/// Node position on canvas
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

/// Connection between nodes
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Connection {
    #[serde(default)]
    pub from: usize,
    #[serde(default)]
    pub to: usize,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub width: Option<f64>,
    // Bezier control points
    #[serde(default)]
    pub cp1x: Option<f64>,
    #[serde(default)]
    pub cp1y: Option<f64>,
    #[serde(default)]
    pub cp2x: Option<f64>,
    #[serde(default)]
    pub cp2y: Option<f64>,
}

/// Node type (agent role)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    // Design Phase
    Architect,
    UxDesigner,
    UiDesigner,

    // Development Phase
    Programmer,

    // QA Phase
    TestGenerator,
    TestRunner,
    BugFixer,
    Refactorer,

    // Ops Phase
    DocWriter,
    GitCommit,

    // Utility
    Custom,
}

impl NodeType {
    /// Convert NodeType to AgentRole
    pub fn to_agent_role(&self) -> crate::berrycode::agents::AgentRole {
        use crate::berrycode::agents::AgentRole;
        match self {
            NodeType::Architect => AgentRole::Architect,
            NodeType::UxDesigner => AgentRole::UXDesigner,
            NodeType::UiDesigner => AgentRole::UIDesigner,
            NodeType::Programmer => AgentRole::Programmer,
            NodeType::TestGenerator => AgentRole::QAEngineer,
            NodeType::TestRunner => AgentRole::QAEngineer,
            NodeType::BugFixer => AgentRole::BugFixer,
            NodeType::Refactorer => AgentRole::Refactorer,
            NodeType::DocWriter => AgentRole::DocWriter,
            NodeType::GitCommit | NodeType::Custom => AgentRole::Programmer, // Fallback
        }
    }
}

/// Node configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    /// Prompt template for this node
    pub prompt_template: Option<String>,
    
    /// Output file pattern (e.g., "DESIGN.md", "src/**/*.rs")
    pub output_pattern: Option<String>,

    /// Required input from previous nodes
    pub required_inputs: Vec<String>,

    /// Custom parameters
    #[serde(default)]
    pub parameters: HashMap<String, serde_json::Value>,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            prompt_template: None,
            output_pattern: None,
            required_inputs: Vec::new(),
            parameters: HashMap::new(),
        }
    }
}

impl Workflow {
    /// Create a new empty workflow
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            description: None,
            nodes: Vec::new(),
            connections: Vec::new(),
            variables: HashMap::new(),
        }
    }

    /// Add a node to the workflow
    pub fn add_node(&mut self, node: WorkflowNode) {
        self.nodes.push(node);
    }

    /// Add a connection between nodes
    pub fn add_connection(&mut self, connection: Connection) {
        self.connections.push(connection);
    }

    /// Get execution order (topological sort)
    pub fn get_execution_order(&self) -> Result<Vec<usize>> {
        use std::collections::VecDeque;

        // Build adjacency list
        let mut adj: HashMap<usize, Vec<usize>> = HashMap::new();
        let mut in_degree: HashMap<usize, usize> = HashMap::new();

        // Initialize
        for node in &self.nodes {
            adj.insert(node.id, Vec::new());
            in_degree.insert(node.id, 0);
        }

        // Build graph
        for conn in &self.connections {
            adj.get_mut(&conn.from).unwrap().push(conn.to);
            *in_degree.get_mut(&conn.to).unwrap() += 1;
        }

        // Kahn's algorithm
        let mut queue: VecDeque<usize> = VecDeque::new();
        for (&node_id, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(node_id);
            }
        }

        let mut order = Vec::new();
        while let Some(node_id) = queue.pop_front() {
            order.push(node_id);

            if let Some(neighbors) = adj.get(&node_id) {
                for &neighbor in neighbors {
                    let deg = in_degree.get_mut(&neighbor).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(neighbor);
                    }
                }
            }
        }

        // Check for cycles
        if order.len() != self.nodes.len() {
            return Err(anyhow::anyhow!("Workflow contains cycles"));
        }

        Ok(order)
    }

    /// Validate workflow structure
    pub fn validate(&self) -> Result<()> {
        // Check for empty workflow
        if self.nodes.is_empty() {
            return Err(anyhow::anyhow!("Workflow has no nodes"));
        }

        // Check for duplicate node IDs
        let mut seen_ids = HashSet::new();
        for node in &self.nodes {
            if !seen_ids.insert(node.id) {
                return Err(anyhow::anyhow!("Duplicate node ID: {}", node.id));
            }
        }

        // Check connections reference valid nodes
        let node_ids: HashSet<usize> = self.nodes.iter().map(|n| n.id).collect();
        for conn in &self.connections {
            if !node_ids.contains(&conn.from) {
                return Err(anyhow::anyhow!("Connection references non-existent node: {}", conn.from));
            }
            if !node_ids.contains(&conn.to) {
                return Err(anyhow::anyhow!("Connection references non-existent node: {}", conn.to));
            }
        }

        // Note: Cycles are allowed in workflows (e.g., for retry loops)
        // The get_execution_order() method handles cycles appropriately

        Ok(())
    }

    /// Save workflow to file
    pub fn save(&self, path: &PathBuf) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        tracing::info!("✅ Workflow saved to {:?}", path);
        Ok(())
    }

    /// Load workflow from file
    pub fn load(path: &PathBuf) -> Result<Self> {
        let json = std::fs::read_to_string(path)?;
        let workflow = serde_json::from_str(&json)?;
        tracing::info!("✅ Workflow loaded from {:?}", path);
        Ok(workflow)
    }
}

/// Preset workflows for common tasks
pub mod presets {
    use super::*;

    /// Full development workflow: Design → Dev → QA → Docs → Commit
    pub fn full_dev_pipeline() -> Workflow {
        let mut workflow = Workflow::new(
            "full_dev".to_string(),
            "Full Development Pipeline".to_string(),
        );

        workflow.description = Some("Complete development workflow from requirements to commit".to_string());

        // Design Phase
        workflow.add_node(WorkflowNode {
            id: 1,
            node_type: NodeType::Architect,
            name: "Architecture Design".to_string(),
            icon: "🏗️".to_string(),
            config: NodeConfig {
                prompt_template: Some("Design the architecture for: {requirement}".to_string()),
                output_pattern: Some("DESIGN.md".to_string()),
                ..Default::default()
            },
            position: Position { x: 50.0, y: 150.0 },
        });

        workflow.add_node(WorkflowNode {
            id: 2,
            node_type: NodeType::UiDesigner,
            name: "UI Design".to_string(),
            icon: "🎨".to_string(),
            config: NodeConfig {
                prompt_template: Some("Design UI based on architecture".to_string()),
                output_pattern: Some("ui_design.html".to_string()),
                required_inputs: vec!["DESIGN.md".to_string()],
                ..Default::default()
            },
            position: Position { x: 250.0, y: 150.0 },
        });

        // Development Phase
        workflow.add_node(WorkflowNode {
            id: 3,
            node_type: NodeType::Programmer,
            name: "Implementation".to_string(),
            icon: "💻".to_string(),
            config: NodeConfig {
                prompt_template: Some("Implement the design".to_string()),
                output_pattern: Some("src/**/*.rs".to_string()),
                required_inputs: vec!["DESIGN.md".to_string(), "ui_design.html".to_string()],
                ..Default::default()
            },
            position: Position { x: 450.0, y: 150.0 },
        });

        // QA Phase
        workflow.add_node(WorkflowNode {
            id: 4,
            node_type: NodeType::TestGenerator,
            name: "Generate Tests".to_string(),
            icon: "🧪".to_string(),
            config: NodeConfig {
                prompt_template: Some("Generate comprehensive tests".to_string()),
                output_pattern: Some("tests/**/*.rs".to_string()),
                ..Default::default()
            },
            position: Position { x: 650.0, y: 50.0 },
        });

        workflow.add_node(WorkflowNode {
            id: 5,
            node_type: NodeType::TestRunner,
            name: "Run Tests".to_string(),
            icon: "▶️".to_string(),
            config: NodeConfig::default(),
            position: Position { x: 850.0, y: 50.0 },
        });

        workflow.add_node(WorkflowNode {
            id: 6,
            node_type: NodeType::BugFixer,
            name: "Fix Bugs".to_string(),
            icon: "🐛".to_string(),
            config: NodeConfig::default(),
            position: Position { x: 850.0, y: 250.0 },
        });

        workflow.add_node(WorkflowNode {
            id: 7,
            node_type: NodeType::Refactorer,
            name: "Refactor Code".to_string(),
            icon: "🧹".to_string(),
            config: NodeConfig::default(),
            position: Position { x: 1050.0, y: 150.0 },
        });

        // Ops Phase
        workflow.add_node(WorkflowNode {
            id: 8,
            node_type: NodeType::DocWriter,
            name: "Write Documentation".to_string(),
            icon: "📝".to_string(),
            config: NodeConfig {
                output_pattern: Some("README.md".to_string()),
                ..Default::default()
            },
            position: Position { x: 1250.0, y: 150.0 },
        });

        workflow.add_node(WorkflowNode {
            id: 9,
            node_type: NodeType::GitCommit,
            name: "Git Commit".to_string(),
            icon: "📦".to_string(),
            config: NodeConfig::default(),
            position: Position { x: 1450.0, y: 150.0 },
        });

        // Connections
        workflow.add_connection(Connection { from: 1, to: 2, ..Default::default() });
        workflow.add_connection(Connection { from: 2, to: 3, ..Default::default() });
        workflow.add_connection(Connection { from: 3, to: 4, ..Default::default() });
        workflow.add_connection(Connection { from: 4, to: 5, ..Default::default() });
        workflow.add_connection(Connection { from: 5, to: 6, label: Some("Failed".to_string()), color: Some("#ef4444".to_string()), ..Default::default() });
        workflow.add_connection(Connection { from: 6, to: 5, label: Some("Retry".to_string()), ..Default::default() });
        workflow.add_connection(Connection { from: 5, to: 7, label: Some("Passed".to_string()), color: Some("#4ade80".to_string()), ..Default::default() });
        workflow.add_connection(Connection { from: 7, to: 8, ..Default::default() });
        workflow.add_connection(Connection { from: 8, to: 9, ..Default::default() });

        workflow
    }

    /// Simple TDD loop: Write test → Implement → Run → Fix
    pub fn tdd_loop() -> Workflow {
        let mut workflow = Workflow::new(
            "tdd_loop".to_string(),
            "TDD Development Loop".to_string(),
        );

        workflow.add_node(WorkflowNode {
            id: 1,
            node_type: NodeType::TestGenerator,
            name: "Write Test".to_string(),
            icon: "🧪".to_string(),
            config: NodeConfig::default(),
            position: Position { x: 100.0, y: 150.0 },
        });

        workflow.add_node(WorkflowNode {
            id: 2,
            node_type: NodeType::Programmer,
            name: "Implement".to_string(),
            icon: "💻".to_string(),
            config: NodeConfig::default(),
            position: Position { x: 300.0, y: 150.0 },
        });

        workflow.add_node(WorkflowNode {
            id: 3,
            node_type: NodeType::TestRunner,
            name: "Run Tests".to_string(),
            icon: "▶️".to_string(),
            config: NodeConfig::default(),
            position: Position { x: 500.0, y: 150.0 },
        });

        workflow.add_node(WorkflowNode {
            id: 4,
            node_type: NodeType::BugFixer,
            name: "Fix Bugs".to_string(),
            icon: "🐛".to_string(),
            config: NodeConfig::default(),
            position: Position { x: 500.0, y: 300.0 },
        });

        workflow.add_connection(Connection { from: 1, to: 2, ..Default::default() });
        workflow.add_connection(Connection { from: 2, to: 3, ..Default::default() });
        workflow.add_connection(Connection { from: 3, to: 4, label: Some("Failed".to_string()), ..Default::default() });
        workflow.add_connection(Connection { from: 4, to: 3, label: Some("Retry".to_string()), ..Default::default() });

        workflow
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_execution_order() {
        let workflow = presets::tdd_loop();

        // TDD loop contains a cycle (test -> fix -> test), so execution order cannot be determined
        assert!(workflow.get_execution_order().is_err());

        // Validate should still pass (cycles are allowed)
        assert!(workflow.validate().is_ok());
    }

    #[test]
    fn test_workflow_validation() {
        let workflow = presets::full_dev_pipeline();
        assert!(workflow.validate().is_ok());
    }

    #[test]
    fn test_cycle_detection() {
        let mut workflow = Workflow::new("test".to_string(), "Cycle Test".to_string());

        workflow.add_node(WorkflowNode {
            id: 1,
            node_type: NodeType::Programmer,
            name: "Node 1".to_string(),
            icon: "💻".to_string(),
            config: NodeConfig::default(),
            position: Position::default(),
        });

        workflow.add_node(WorkflowNode {
            id: 2,
            node_type: NodeType::TestRunner,
            name: "Node 2".to_string(),
            icon: "▶️".to_string(),
            config: NodeConfig::default(),
            position: Position::default(),
        });

        // Create a cycle: 1 -> 2 -> 1
        workflow.add_connection(Connection { from: 1, to: 2, ..Default::default() });
        workflow.add_connection(Connection { from: 2, to: 1, ..Default::default() });

        // Should detect cycle and return error
        let result = workflow.get_execution_order();
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_node_reference() {
        let mut workflow = Workflow::new("test".to_string(), "Invalid Ref Test".to_string());

        workflow.add_node(WorkflowNode {
            id: 1,
            node_type: NodeType::Programmer,
            name: "Node 1".to_string(),
            icon: "💻".to_string(),
            config: NodeConfig::default(),
            position: Position::default(),
        });

        // Connection references non-existent node
        workflow.add_connection(Connection { from: 1, to: 999, ..Default::default() });

        let result = workflow.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_workflow() {
        let workflow = Workflow::new("empty".to_string(), "Empty".to_string());

        // Empty workflow should return error on validation
        assert!(workflow.validate().is_err());

        // But execution order should be empty
        let order = workflow.get_execution_order().unwrap();
        assert_eq!(order.len(), 0);
    }

    #[test]
    fn test_linear_workflow() {
        let mut workflow = Workflow::new("linear".to_string(), "Linear Test".to_string());

        for i in 1..=5 {
            workflow.add_node(WorkflowNode {
                id: i,
                node_type: NodeType::Programmer,
                name: format!("Node {}", i),
                icon: "💻".to_string(),
                config: NodeConfig::default(),
                position: Position::default(),
            });
        }

        // Create linear chain: 1 -> 2 -> 3 -> 4 -> 5
        for i in 1..=4 {
            workflow.add_connection(Connection { from: i, to: i + 1, ..Default::default() });
        }

        let order = workflow.get_execution_order().unwrap();
        assert_eq!(order, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_bezier_connection() {
        let mut workflow = Workflow::new("bezier".to_string(), "Bezier Test".to_string());

        workflow.add_node(WorkflowNode {
            id: 1,
            node_type: NodeType::Programmer,
            name: "Start".to_string(),
            icon: "💻".to_string(),
            config: NodeConfig::default(),
            position: Position { x: 0.0, y: 0.0 },
        });

        workflow.add_node(WorkflowNode {
            id: 2,
            node_type: NodeType::TestRunner,
            name: "End".to_string(),
            icon: "▶️".to_string(),
            config: NodeConfig::default(),
            position: Position { x: 100.0, y: 100.0 },
        });

        workflow.add_connection(Connection {
            from: 1,
            to: 2,
            cp1x: Some(25.0),
            cp1y: Some(50.0),
            cp2x: Some(75.0),
            cp2y: Some(50.0),
            ..Default::default()
        });

        assert_eq!(workflow.connections.len(), 1);
        assert_eq!(workflow.connections[0].cp1x, Some(25.0));
    }

    #[test]
    fn test_node_config() {
        let mut parameters = HashMap::new();
        parameters.insert("max_tokens".to_string(), serde_json::json!(8192));
        parameters.insert("temperature".to_string(), serde_json::json!(0.8));

        let config = NodeConfig {
            prompt_template: Some("Test prompt".to_string()),
            output_pattern: Some("*.rs".to_string()),
            required_inputs: vec!["input1".to_string()],
            parameters,
        };

        assert_eq!(config.prompt_template, Some("Test prompt".to_string()));
        assert_eq!(config.output_pattern, Some("*.rs".to_string()));
        assert_eq!(config.required_inputs.len(), 1);
        assert_eq!(config.parameters.get("max_tokens"), Some(&serde_json::json!(8192)));
    }

    #[test]
    fn test_full_dev_pipeline_structure() {
        let workflow = presets::full_dev_pipeline();

        // Should have all nodes
        assert!(workflow.nodes.len() >= 7);

        // Should have connections
        assert!(workflow.connections.len() > 0);

        // Should be valid (cycles are allowed)
        assert!(workflow.validate().is_ok());

        // Has a cycle (TestRunner -> BugFixer -> TestRunner), so execution order cannot be determined
        assert!(workflow.get_execution_order().is_err());
    }
}
