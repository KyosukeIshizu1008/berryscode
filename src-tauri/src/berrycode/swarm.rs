//! Hive Mind (Multi-Agent Swarm) - Team of specialized AI agents
//!
//! This module implements **"Singularity Evolution Level 1"** - a system where
//! multiple specialized AI agents collaborate like a real development team.
//!
//! ## How It Works
//!
//! 1. **Project Manager**: Breaks down tasks, coordinates the team
//! 2. **Architect**: Designs solutions, uses knowledge graph for impact analysis
//! 3. **Coder**: Implements features, writes code
//! 4. **Security**: Reviews code for vulnerabilities, suggests fixes
//! 5. **QA**: Tests code using self-healing loop, ensures quality
//!
//! ## Example
//!
//! ```text
//! User: "Add user authentication"
//!
//! [Project Manager]
//! "I'll break this into 5 tasks:
//!  1. Design auth schema
//!  2. Implement JWT tokens
//!  3. Add login endpoint
//!  4. Add middleware
//!  5. Write tests"
//!
//! [Architect]
//! "Using knowledge graph... 12 files will be affected.
//!  Recommend JWT over sessions for scalability."
//!
//! [Security]
//! "⚠️ CRITICAL: Must hash passwords with bcrypt!
//!  ⚠️ Add rate limiting to prevent brute force!"
//!
//! [Coder]
//! "Implementing... (3 files created, 2 modified)"
//!
//! [QA]
//! "Running self-healing loop... Found 2 bugs, fixed both.
//!  All tests passing! ✅"
//!
//! [Team Consensus]
//! "✅ Authentication system complete and secure!"
//! ```
//!
//! This is TRUE team collaboration - multiple perspectives, better decisions!

use crate::berrycode::Result;
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

/// Role of an agent in the swarm
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentRole {
    /// Project manager - task breakdown and coordination
    ProjectManager,
    /// Architect - design and impact analysis
    Architect,
    /// Coder - implementation
    Coder,
    /// Security expert - vulnerability detection
    Security,
    /// QA engineer - testing and quality
    QA,
}

impl AgentRole {
    /// Get the system prompt for this role
    pub fn system_prompt(&self) -> &str {
        match self {
            AgentRole::ProjectManager => {
                "You are a PROJECT MANAGER AI agent in a development team.\n\n\
                 Your responsibilities:\n\
                 - Break down user requests into clear, actionable tasks\n\
                 - Coordinate between team members\n\
                 - Ensure all requirements are understood\n\
                 - Create implementation roadmaps\n\n\
                 Be concise, organized, and detail-oriented. Use numbered lists."
            }
            AgentRole::Architect => {
                "You are an ARCHITECT AI agent in a development team.\n\n\
                 Your responsibilities:\n\
                 - Design system architecture\n\
                 - Analyze impact using knowledge graph\n\
                 - Make technology choices (libraries, patterns)\n\
                 - Consider scalability and maintainability\n\n\
                 Be technical, strategic, and forward-thinking. Reference existing code structure."
            }
            AgentRole::Coder => {
                "You are a CODER AI agent in a development team.\n\n\
                 Your responsibilities:\n\
                 - Implement features according to architect's design\n\
                 - Write clean, efficient code\n\
                 - Follow team's coding standards\n\
                 - Document your code\n\n\
                 Be practical, efficient, and thorough. Focus on implementation details."
            }
            AgentRole::Security => {
                "You are a SECURITY EXPERT AI agent in a development team.\n\n\
                 Your responsibilities:\n\
                 - Review code for security vulnerabilities\n\
                 - Check for OWASP Top 10 issues\n\
                 - Suggest security best practices\n\
                 - Flag potential attack vectors\n\n\
                 Be paranoid, critical, and protective. Use ⚠️ for warnings."
            }
            AgentRole::QA => {
                "You are a QA ENGINEER AI agent in a development team.\n\n\
                 Your responsibilities:\n\
                 - Test implementations thoroughly\n\
                 - Use self-healing loop to find and fix bugs\n\
                 - Verify edge cases\n\
                 - Ensure quality standards\n\n\
                 Be meticulous, skeptical, and quality-focused. Report test results clearly."
            }
        }
    }

    /// Get emoji icon for this role
    pub fn icon(&self) -> &str {
        match self {
            AgentRole::ProjectManager => "📋",
            AgentRole::Architect => "🏗️",
            AgentRole::Coder => "💻",
            AgentRole::Security => "🛡️",
            AgentRole::QA => "🔍",
        }
    }

    /// Get name for this role
    pub fn name(&self) -> &str {
        match self {
            AgentRole::ProjectManager => "Project Manager",
            AgentRole::Architect => "Architect",
            AgentRole::Coder => "Coder",
            AgentRole::Security => "Security",
            AgentRole::QA => "QA",
        }
    }
}

/// A message in the shared memory (team chat)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmMessage {
    /// Who sent this message
    pub from: AgentRole,
    /// Message content
    pub content: String,
    /// Optional: addressed to specific agent
    pub to: Option<AgentRole>,
    /// Timestamp
    pub timestamp: std::time::SystemTime,
    /// Message type (discussion, proposal, approval, etc.)
    pub msg_type: MessageType,
}

/// Type of message
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MessageType {
    /// General discussion
    Discussion,
    /// Proposal for action
    Proposal,
    /// Approval of proposal
    Approval,
    /// Concern or objection
    Concern,
    /// Final decision
    Decision,
    /// Status update
    Status,
}

/// Shared memory for team collaboration
#[derive(Debug, Clone)]
pub struct SharedMemory {
    /// All messages in chronological order
    messages: Arc<Mutex<Vec<SwarmMessage>>>,
    /// Current task being worked on
    current_task: Arc<Mutex<Option<String>>>,
    /// Decisions made by the team
    decisions: Arc<Mutex<HashMap<String, String>>>,
}

impl SharedMemory {
    /// Create new shared memory
    pub fn new() -> Self {
        Self {
            messages: Arc::new(Mutex::new(Vec::new())),
            current_task: Arc::new(Mutex::new(None)),
            decisions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Add a message to shared memory
    pub fn add_message(&self, message: SwarmMessage) {
        let mut messages = self.messages.lock().unwrap();
        messages.push(message);
    }

    /// Get all messages
    pub fn get_messages(&self) -> Vec<SwarmMessage> {
        let messages = self.messages.lock().unwrap();
        messages.clone()
    }

    /// Get recent messages (last N)
    pub fn get_recent_messages(&self, count: usize) -> Vec<SwarmMessage> {
        let messages = self.messages.lock().unwrap();
        let start = messages.len().saturating_sub(count);
        messages[start..].to_vec()
    }

    /// Set current task
    pub fn set_task(&self, task: String) {
        let mut current_task = self.current_task.lock().unwrap();
        *current_task = Some(task);
    }

    /// Get current task
    pub fn get_task(&self) -> Option<String> {
        let current_task = self.current_task.lock().unwrap();
        current_task.clone()
    }

    /// Record a decision
    pub fn record_decision(&self, key: String, value: String) {
        let mut decisions = self.decisions.lock().unwrap();
        decisions.insert(key, value);
    }

    /// Get all decisions
    pub fn get_decisions(&self) -> HashMap<String, String> {
        let decisions = self.decisions.lock().unwrap();
        decisions.clone()
    }

    /// Clear all memory (new task)
    pub fn clear(&self) {
        let mut messages = self.messages.lock().unwrap();
        messages.clear();

        let mut current_task = self.current_task.lock().unwrap();
        *current_task = None;

        let mut decisions = self.decisions.lock().unwrap();
        decisions.clear();
    }

    /// Generate context summary for agents
    pub fn get_context_summary(&self) -> String {
        let messages = self.get_messages();
        let task = self.get_task();
        let decisions = self.get_decisions();

        let mut summary = String::new();

        if let Some(task) = task {
            summary.push_str(&format!("## Current Task\n{}\n\n", task));
        }

        if !decisions.is_empty() {
            summary.push_str("## Team Decisions\n");
            for (key, value) in decisions {
                summary.push_str(&format!("- {}: {}\n", key, value));
            }
            summary.push_str("\n");
        }

        if !messages.is_empty() {
            summary.push_str("## Recent Discussion\n");
            for msg in messages.iter().rev().take(10).rev() {
                summary.push_str(&format!(
                    "{} [{}]: {}\n",
                    msg.from.icon(),
                    msg.from.name(),
                    msg.content
                ));
            }
        }

        summary
    }
}

impl Default for SharedMemory {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for swarm behavior
#[derive(Debug, Clone)]
pub struct SwarmConfig {
    /// Maximum number of discussion rounds
    pub max_rounds: usize,
    /// Minimum approvals needed for consensus
    pub min_approvals: usize,
    /// Enable verbose logging of agent discussions
    pub verbose: bool,
}

impl Default for SwarmConfig {
    fn default() -> Self {
        Self {
            max_rounds: 5,
            min_approvals: 3,
            verbose: true,
        }
    }
}

/// The swarm controller that orchestrates all agents
pub struct Swarm {
    /// Shared memory for team collaboration
    memory: SharedMemory,
    /// Configuration
    config: SwarmConfig,
    /// Project root directory
    project_root: std::path::PathBuf,
}

impl Swarm {
    /// Create a new swarm
    pub fn new(project_root: &Path, config: SwarmConfig) -> Self {
        Self {
            memory: SharedMemory::new(),
            config,
            project_root: project_root.to_path_buf(),
        }
    }

    /// Execute a task using the full team
    pub async fn execute_task(&mut self, task: &str) -> Result<SwarmResult> {
        use crate::berrycode::display::DisplayManager;

        let display = DisplayManager::new();
        display.log_action(&format!("🧠 Hive Mind: Assembling team for '{}'", task));

        // Clear previous context
        self.memory.clear();
        self.memory.set_task(task.to_string());

        let mut phases = Vec::new();

        // Phase 1: Project Manager breaks down the task
        display.print_sub_result("📋 Phase 1: Project Manager analyzing task...", true);
        let pm_phase = self.run_project_manager_phase(task).await?;
        phases.push(pm_phase.clone());

        if self.config.verbose {
            println!("\n{}", format!("  {} {}", AgentRole::ProjectManager.icon(), pm_phase.summary).as_str());
        }

        // Phase 2: Architect designs the solution
        display.print_sub_result("🏗️ Phase 2: Architect designing solution...", true);
        let arch_phase = self.run_architect_phase(task).await?;
        phases.push(arch_phase.clone());

        if self.config.verbose {
            println!("{}", format!("  {} {}", AgentRole::Architect.icon(), arch_phase.summary).as_str());
        }

        // Phase 3: Security reviews the plan
        display.print_sub_result("🛡️ Phase 3: Security reviewing design...", true);
        let sec_phase = self.run_security_phase(task).await?;
        phases.push(sec_phase.clone());

        if self.config.verbose {
            println!("{}", format!("  {} {}", AgentRole::Security.icon(), sec_phase.summary).as_str());
        }

        // Phase 4: Coder implements
        display.print_sub_result("💻 Phase 4: Coder implementing...", true);
        let coder_phase = self.run_coder_phase(task).await?;
        phases.push(coder_phase.clone());

        if self.config.verbose {
            println!("{}", format!("  {} {}", AgentRole::Coder.icon(), coder_phase.summary).as_str());
        }

        // Phase 5: QA tests
        display.print_sub_result("🔍 Phase 5: QA testing...", true);
        let qa_phase = self.run_qa_phase(task).await?;
        phases.push(qa_phase.clone());

        if self.config.verbose {
            println!("{}", format!("  {} {}", AgentRole::QA.icon(), qa_phase.summary).as_str());
        }

        // Build final result
        let success = qa_phase.success;

        let final_summary = format!(
            "Team completed task: {}\n\n\
             📋 PM: {}\n\
             🏗️ Architect: {}\n\
             🛡️ Security: {}\n\
             💻 Coder: {}\n\
             🔍 QA: {}",
            task,
            pm_phase.summary,
            arch_phase.summary,
            sec_phase.summary,
            coder_phase.summary,
            qa_phase.summary
        );

        display.print_sub_result(&format!(
            "{} Swarm {}",
            if success { "✅" } else { "⚠️" },
            if success { "succeeded!" } else { "completed with warnings" }
        ), success);

        Ok(SwarmResult {
            task: task.to_string(),
            success,
            phases,
            final_summary,
            discussion_log: self.memory.get_messages(),
        })
    }

    /// Phase 1: Project Manager analyzes and breaks down task
    async fn run_project_manager_phase(&self, task: &str) -> Result<PhaseResult> {
        let prompt = format!(
            "{}

Task from user: \"{}\"

Break this down into clear, numbered steps. Consider:
1. What needs to be built?
2. What are the key components?
3. What's the order of implementation?
4. Any dependencies or prerequisites?

Respond with a clear plan.",
            AgentRole::ProjectManager.system_prompt(),
            task
        );

        // Simulate LLM call (in production, this would call actual LLM)
        let response = self.simulate_agent_response(AgentRole::ProjectManager, &prompt);

        self.memory.add_message(SwarmMessage {
            from: AgentRole::ProjectManager,
            content: response.clone(),
            to: None,
            timestamp: std::time::SystemTime::now(),
            msg_type: MessageType::Proposal,
        });

        Ok(PhaseResult {
            agent: AgentRole::ProjectManager,
            summary: "Task breakdown complete".to_string(),
            output: response,
            success: true,
        })
    }

    /// Phase 2: Architect designs solution
    async fn run_architect_phase(&self, task: &str) -> Result<PhaseResult> {
        let context = self.memory.get_context_summary();

        let prompt = format!(
            "{}

{}

Design the architecture for this task. Consider:
1. What files need to be created/modified?
2. What's the impact on existing code (use knowledge graph)?
3. What libraries or patterns should be used?
4. How will this integrate with current architecture?

Provide architectural decisions.",
            AgentRole::Architect.system_prompt(),
            context
        );

        let response = self.simulate_agent_response(AgentRole::Architect, &prompt);

        self.memory.add_message(SwarmMessage {
            from: AgentRole::Architect,
            content: response.clone(),
            to: None,
            timestamp: std::time::SystemTime::now(),
            msg_type: MessageType::Proposal,
        });

        Ok(PhaseResult {
            agent: AgentRole::Architect,
            summary: "Design complete, impact analyzed".to_string(),
            output: response,
            success: true,
        })
    }

    /// Phase 3: Security reviews
    async fn run_security_phase(&self, _task: &str) -> Result<PhaseResult> {
        let context = self.memory.get_context_summary();

        let prompt = format!(
            "{}

{}

Review the proposed solution for security issues. Check for:
1. SQL injection vulnerabilities
2. XSS risks
3. Authentication/authorization flaws
4. Data exposure
5. OWASP Top 10 issues

Flag any concerns with ⚠️ prefix.",
            AgentRole::Security.system_prompt(),
            context
        );

        let response = self.simulate_agent_response(AgentRole::Security, &prompt);

        let has_critical_concerns = response.contains("⚠️ CRITICAL");

        self.memory.add_message(SwarmMessage {
            from: AgentRole::Security,
            content: response.clone(),
            to: None,
            timestamp: std::time::SystemTime::now(),
            msg_type: if has_critical_concerns {
                MessageType::Concern
            } else {
                MessageType::Approval
            },
        });

        Ok(PhaseResult {
            agent: AgentRole::Security,
            summary: if has_critical_concerns {
                "Critical security concerns raised"
            } else {
                "Security review passed"
            }
            .to_string(),
            output: response,
            success: !has_critical_concerns,
        })
    }

    /// Phase 4: Coder implements
    async fn run_coder_phase(&self, _task: &str) -> Result<PhaseResult> {
        let context = self.memory.get_context_summary();

        let prompt = format!(
            "{}

{}

Implement the solution following the architect's design and security requirements.
Provide implementation summary (files created/modified, key changes).",
            AgentRole::Coder.system_prompt(),
            context
        );

        let response = self.simulate_agent_response(AgentRole::Coder, &prompt);

        self.memory.add_message(SwarmMessage {
            from: AgentRole::Coder,
            content: response.clone(),
            to: None,
            timestamp: std::time::SystemTime::now(),
            msg_type: MessageType::Status,
        });

        Ok(PhaseResult {
            agent: AgentRole::Coder,
            summary: "Implementation complete".to_string(),
            output: response,
            success: true,
        })
    }

    /// Phase 5: QA tests
    async fn run_qa_phase(&self, _task: &str) -> Result<PhaseResult> {
        let context = self.memory.get_context_summary();

        let prompt = format!(
            "{}

{}

Test the implementation. Verify:
1. All requirements are met
2. Edge cases are handled
3. Tests pass
4. No regressions

Report test results.",
            AgentRole::QA.system_prompt(),
            context
        );

        let response = self.simulate_agent_response(AgentRole::QA, &prompt);

        let all_tests_pass = response.contains("✅") || response.contains("PASS");

        self.memory.add_message(SwarmMessage {
            from: AgentRole::QA,
            content: response.clone(),
            to: None,
            timestamp: std::time::SystemTime::now(),
            msg_type: MessageType::Decision,
        });

        Ok(PhaseResult {
            agent: AgentRole::QA,
            summary: if all_tests_pass {
                "All tests passing"
            } else {
                "Tests need fixes"
            }
            .to_string(),
            output: response,
            success: all_tests_pass,
        })
    }

    /// Simulate agent response (in production, this calls real LLM)
    fn simulate_agent_response(&self, role: AgentRole, _prompt: &str) -> String {
        match role {
            AgentRole::ProjectManager => {
                "Task breakdown:\n\
                 1. Design data structures\n\
                 2. Implement core logic\n\
                 3. Add API endpoints\n\
                 4. Write tests\n\
                 5. Documentation"
                    .to_string()
            }
            AgentRole::Architect => {
                "Architecture decision:\n\
                 - Will modify 3 existing files\n\
                 - Create 2 new modules\n\
                 - Impact: Low (isolated feature)\n\
                 - Use existing patterns from codebase"
                    .to_string()
            }
            AgentRole::Security => {
                "Security review:\n\
                 ✅ No SQL injection risks\n\
                 ✅ Input validation planned\n\
                 ✅ Authentication properly handled\n\
                 Approved for implementation."
                    .to_string()
            }
            AgentRole::Coder => {
                "Implementation summary:\n\
                 Created: src/feature.rs\n\
                 Modified: src/lib.rs, src/api.rs\n\
                 Lines added: 150\n\
                 Ready for testing."
                    .to_string()
            }
            AgentRole::QA => {
                "Test results:\n\
                 ✅ Unit tests: 12/12 passing\n\
                 ✅ Integration tests: 5/5 passing\n\
                 ✅ Edge cases verified\n\
                 All quality checks passed!"
                    .to_string()
            }
        }
    }
}

/// Result of a phase execution
#[derive(Debug, Clone)]
pub struct PhaseResult {
    /// Which agent executed this phase
    pub agent: AgentRole,
    /// Summary of what happened
    pub summary: String,
    /// Detailed output
    pub output: String,
    /// Whether this phase succeeded
    pub success: bool,
}

/// Final result of swarm execution
#[derive(Debug, Clone)]
pub struct SwarmResult {
    /// Original task
    pub task: String,
    /// Overall success
    pub success: bool,
    /// Results from each phase
    pub phases: Vec<PhaseResult>,
    /// Final summary
    pub final_summary: String,
    /// Full discussion log
    pub discussion_log: Vec<SwarmMessage>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_agent_roles() {
        assert_eq!(AgentRole::ProjectManager.name(), "Project Manager");
        assert_eq!(AgentRole::Architect.icon(), "🏗️");
        assert!(AgentRole::Security.system_prompt().contains("SECURITY"));
    }

    #[test]
    fn test_shared_memory() {
        let memory = SharedMemory::new();

        memory.set_task("Test task".to_string());
        assert_eq!(memory.get_task(), Some("Test task".to_string()));

        let msg = SwarmMessage {
            from: AgentRole::ProjectManager,
            content: "Hello team".to_string(),
            to: None,
            timestamp: std::time::SystemTime::now(),
            msg_type: MessageType::Discussion,
        };

        memory.add_message(msg);
        assert_eq!(memory.get_messages().len(), 1);

        memory.record_decision("auth_method".to_string(), "JWT".to_string());
        let decisions = memory.get_decisions();
        assert_eq!(decisions.get("auth_method"), Some(&"JWT".to_string()));
    }

    #[test]
    fn test_swarm_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = SwarmConfig::default();
        let swarm = Swarm::new(temp_dir.path(), config);

        assert_eq!(swarm.memory.get_messages().len(), 0);
    }

    #[tokio::test]
    async fn test_swarm_execution() {
        let temp_dir = TempDir::new().unwrap();
        let config = SwarmConfig {
            max_rounds: 3,
            min_approvals: 2,
            verbose: false,
        };
        let mut swarm = Swarm::new(temp_dir.path(), config);

        let result = swarm.execute_task("Add user authentication").await;
        assert!(result.is_ok());

        let swarm_result = result.unwrap();
        assert_eq!(swarm_result.phases.len(), 5); // All 5 agents participated
        assert!(swarm_result.final_summary.contains("Team completed task"));
    }
}
