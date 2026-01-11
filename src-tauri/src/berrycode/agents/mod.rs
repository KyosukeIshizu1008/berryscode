//! Agent system for BerryFlow
//!
//! Each agent represents a specialized role (Architect, Programmer, etc.)
//! with specific system prompts and capabilities.

pub mod architect;
pub mod programmer;
pub mod qa_engineer;
pub mod bug_fixer;
pub mod refactorer;
pub mod doc_writer;

#[cfg(test)]
mod tests;

use serde::{Serialize, Deserialize};
use crate::berrycode::Result;
use crate::berrycode::llm::LLMClient;
use crate::berrycode::repomap::RepoMap;
use std::collections::HashMap;
use std::sync::Arc;

/// Agent trait that all agents must implement
#[async_trait::async_trait]
pub trait Agent: Send + Sync {
    /// Get agent name
    fn name(&self) -> &str;
    
    /// Get agent role
    fn role(&self) -> AgentRole;
    
    /// Get system prompt for this agent
    fn system_prompt(&self) -> String;
    
    /// Execute agent task
    async fn execute(&self, context: &AgentContext) -> Result<AgentOutput>;
    
    /// Validate output
    fn validate_output(&self, output: &AgentOutput) -> Result<()>;
}

/// Agent role enum
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AgentRole {
    Architect,
    UXDesigner,
    UIDesigner,
    Programmer,
    QAEngineer,
    BugFixer,
    Refactorer,
    DocWriter,
}

/// Agent execution context
pub struct AgentContext {
    pub project_root: std::path::PathBuf,
    pub inputs: HashMap<String, String>,
    pub config: AgentConfig,
    pub llm_client: Arc<LLMClient>,
    pub repo_map: Option<Arc<RepoMap>>,
}

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub model: String,
    pub temperature: f32,
    pub max_tokens: usize,
    pub parameters: HashMap<String, serde_json::Value>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            model: "claude-sonnet-4-5@20250929".to_string(),
            temperature: 0.7,
            max_tokens: 4096,
            parameters: HashMap::new(),
        }
    }
}

/// Agent output
#[derive(Debug, Clone)]
pub struct AgentOutput {
    pub files: HashMap<std::path::PathBuf, String>,
    pub metadata: HashMap<String, String>,
    pub success: bool,
    pub message: String,
}

/// Create agent by role
pub fn create_agent(role: AgentRole) -> Box<dyn Agent> {
    match role {
        AgentRole::Architect => Box::new(architect::ArchitectAgent),
        AgentRole::Programmer => Box::new(programmer::ProgrammerAgent),
        AgentRole::QAEngineer => Box::new(qa_engineer::QAEngineerAgent),
        AgentRole::BugFixer => Box::new(bug_fixer::BugFixerAgent),
        AgentRole::Refactorer => Box::new(refactorer::RefactorerAgent),
        AgentRole::DocWriter => Box::new(doc_writer::DocWriterAgent),
        _ => Box::new(architect::ArchitectAgent), // Default fallback
    }
}
