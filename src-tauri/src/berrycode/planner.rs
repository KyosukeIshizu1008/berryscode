//! Planner - Task Planning and Execution Manager
//!
//! Inspired by OpenHands (OpenDevin) architecture.
//! Separates planning from execution, allowing AI to:
//! - Break down complex tasks into steps
//! - Track progress
//! - Adapt plans based on results
//! - Maintain long-term context
//!
//! Philosophy:
//! - Planner (DeepSeek-R1): Strategic thinking, decomposition
//! - Actor (DeepSeek-V3): Tactical execution, tool use
//! - Observer: Validates results, triggers replanning

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Task plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    /// Plan ID
    pub id: String,

    /// Plan title
    pub title: String,

    /// Plan description
    pub description: String,

    /// Steps in the plan
    pub steps: Vec<Step>,

    /// Current step index
    pub current_step: usize,

    /// Plan status
    pub status: PlanStatus,

    /// Created timestamp
    pub created_at: DateTime<Utc>,

    /// Updated timestamp
    pub updated_at: DateTime<Utc>,

    /// Metadata (reasoning, context, etc.)
    pub metadata: HashMap<String, String>,
}

/// Plan status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PlanStatus {
    /// Plan is being created
    Planning,

    /// Plan is approved and ready
    Ready,

    /// Plan is being executed
    InProgress,

    /// Plan completed successfully
    Completed,

    /// Plan failed
    Failed,

    /// Plan was cancelled
    Cancelled,

    /// Plan needs revision
    NeedsRevision,
}

/// Step in a plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    /// Step ID
    pub id: String,

    /// Step index (order in plan)
    pub index: usize,

    /// Step title/description
    pub title: String,

    /// Detailed instructions
    pub instructions: String,

    /// Expected outcome
    pub expected_outcome: String,

    /// Step status
    pub status: StepStatus,

    /// Step type (investigation, implementation, verification, etc.)
    pub step_type: StepType,

    /// Dependencies (other step IDs)
    pub dependencies: Vec<String>,

    /// Execution result
    pub result: Option<StepResult>,

    /// Started timestamp
    pub started_at: Option<DateTime<Utc>>,

    /// Completed timestamp
    pub completed_at: Option<DateTime<Utc>>,
}

/// Step status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    /// Not started
    Pending,

    /// Currently executing
    InProgress,

    /// Completed successfully
    Completed,

    /// Failed
    Failed,

    /// Skipped (dependencies not met or obsolete)
    Skipped,

    /// Blocked (waiting for dependencies)
    Blocked,
}

/// Step type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StepType {
    /// Investigation/exploration step
    Investigation,

    /// Environment setup
    Setup,

    /// Implementation/coding
    Implementation,

    /// Testing/verification
    Verification,

    /// Bug fixing
    BugFix,

    /// Deployment
    Deployment,

    /// Documentation
    Documentation,
}

/// Step execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    /// Success status
    pub success: bool,

    /// Output/observations
    pub output: String,

    /// Actions taken
    pub actions: Vec<String>,

    /// Files modified
    pub files_modified: Vec<String>,

    /// Errors encountered
    pub errors: Vec<String>,

    /// Lessons learned (for replanning)
    pub lessons: Vec<String>,
}

/// Plan update request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanUpdate {
    /// Plan ID
    pub plan_id: String,

    /// Update type
    pub update_type: PlanUpdateType,

    /// Reason for update
    pub reason: String,

    /// New steps to add (if applicable)
    pub new_steps: Option<Vec<Step>>,

    /// Steps to modify (step_id -> new step)
    pub modified_steps: Option<HashMap<String, Step>>,

    /// Steps to remove
    pub removed_steps: Option<Vec<String>>,
}

/// Plan update type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PlanUpdateType {
    /// Add new steps
    AddSteps,

    /// Modify existing steps
    ModifySteps,

    /// Remove steps
    RemoveSteps,

    /// Reorder steps
    ReorderSteps,

    /// Complete plan revision
    Revise,
}

/// Planner - Manages task plans
pub struct Planner {
    /// Active plans (plan_id -> plan)
    plans: Arc<RwLock<HashMap<String, Plan>>>,
}

impl Planner {
    /// Create new planner
    pub fn new() -> Self {
        Self {
            plans: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new plan
    pub async fn create_plan(
        &self,
        title: String,
        description: String,
        steps: Vec<Step>,
    ) -> Result<String> {
        let plan_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let plan = Plan {
            id: plan_id.clone(),
            title,
            description,
            steps,
            current_step: 0,
            status: PlanStatus::Ready,
            created_at: now,
            updated_at: now,
            metadata: HashMap::new(),
        };

        self.plans.write().await.insert(plan_id.clone(), plan);

        tracing::info!("Created new plan: {}", plan_id);

        Ok(plan_id)
    }

    /// Get a plan by ID
    pub async fn get_plan(&self, plan_id: &str) -> Option<Plan> {
        self.plans.read().await.get(plan_id).cloned()
    }

    /// List all plans
    pub async fn list_plans(&self) -> Vec<Plan> {
        self.plans.read().await.values().cloned().collect()
    }

    /// Start executing a plan
    pub async fn start_plan(&self, plan_id: &str) -> Result<()> {
        let mut plans = self.plans.write().await;
        let plan = plans
            .get_mut(plan_id)
            .ok_or_else(|| anyhow!("Plan not found: {}", plan_id))?;

        plan.status = PlanStatus::InProgress;
        plan.updated_at = Utc::now();

        tracing::info!("Started plan: {}", plan_id);

        Ok(())
    }

    /// Mark current step as in progress
    pub async fn start_step(&self, plan_id: &str, step_id: &str) -> Result<()> {
        let mut plans = self.plans.write().await;
        let plan = plans
            .get_mut(plan_id)
            .ok_or_else(|| anyhow!("Plan not found: {}", plan_id))?;

        let step = plan
            .steps
            .iter_mut()
            .find(|s| s.id == step_id)
            .ok_or_else(|| anyhow!("Step not found: {}", step_id))?;

        step.status = StepStatus::InProgress;
        step.started_at = Some(Utc::now());

        tracing::info!("Started step: {} in plan: {}", step_id, plan_id);

        Ok(())
    }

    /// Complete a step with result
    pub async fn complete_step(
        &self,
        plan_id: &str,
        step_id: &str,
        result: StepResult,
    ) -> Result<()> {
        let mut plans = self.plans.write().await;
        let plan = plans
            .get_mut(plan_id)
            .ok_or_else(|| anyhow!("Plan not found: {}", plan_id))?;

        let step = plan
            .steps
            .iter_mut()
            .find(|s| s.id == step_id)
            .ok_or_else(|| anyhow!("Step not found: {}", step_id))?;

        step.status = if result.success {
            StepStatus::Completed
        } else {
            StepStatus::Failed
        };
        step.result = Some(result.clone());
        step.completed_at = Some(Utc::now());

        // Move to next step
        if result.success && plan.current_step < plan.steps.len() - 1 {
            plan.current_step += 1;
        }

        // Check if all steps are completed
        if plan.steps.iter().all(|s| s.status == StepStatus::Completed) {
            plan.status = PlanStatus::Completed;
        }

        plan.updated_at = Utc::now();

        tracing::info!("Completed step: {} in plan: {}", step_id, plan_id);

        Ok(())
    }

    /// Update a plan (AI can modify its own plan)
    pub async fn update_plan(&self, update: PlanUpdate) -> Result<()> {
        let mut plans = self.plans.write().await;
        let plan = plans
            .get_mut(&update.plan_id)
            .ok_or_else(|| anyhow!("Plan not found: {}", update.plan_id))?;

        match update.update_type {
            PlanUpdateType::AddSteps => {
                if let Some(new_steps) = update.new_steps {
                    plan.steps.extend(new_steps);
                }
            }
            PlanUpdateType::ModifySteps => {
                if let Some(modified_steps) = update.modified_steps {
                    for (step_id, new_step) in modified_steps {
                        if let Some(step) = plan.steps.iter_mut().find(|s| s.id == step_id) {
                            *step = new_step;
                        }
                    }
                }
            }
            PlanUpdateType::RemoveSteps => {
                if let Some(removed_steps) = update.removed_steps {
                    plan.steps.retain(|s| !removed_steps.contains(&s.id));
                }
            }
            PlanUpdateType::ReorderSteps => {
                // TODO: Implement reordering logic
            }
            PlanUpdateType::Revise => {
                // Full plan revision
                if let Some(new_steps) = update.new_steps {
                    plan.steps = new_steps;
                    plan.current_step = 0;
                    plan.status = PlanStatus::Ready;
                }
            }
        }

        plan.status = PlanStatus::NeedsRevision;
        plan.updated_at = Utc::now();

        tracing::info!(
            "Updated plan: {} (reason: {})",
            update.plan_id,
            update.reason
        );

        Ok(())
    }

    /// Get current step
    pub async fn get_current_step(&self, plan_id: &str) -> Option<Step> {
        let plans = self.plans.read().await;
        let plan = plans.get(plan_id)?;

        plan.steps.get(plan.current_step).cloned()
    }

    /// Get next actionable step
    pub async fn get_next_step(&self, plan_id: &str) -> Option<Step> {
        let plans = self.plans.read().await;
        let plan = plans.get(plan_id)?;

        // Find first pending or blocked step
        plan.steps
            .iter()
            .find(|s| matches!(s.status, StepStatus::Pending | StepStatus::Blocked))
            .cloned()
    }

    /// Cancel a plan
    pub async fn cancel_plan(&self, plan_id: &str) -> Result<()> {
        let mut plans = self.plans.write().await;
        let plan = plans
            .get_mut(plan_id)
            .ok_or_else(|| anyhow!("Plan not found: {}", plan_id))?;

        plan.status = PlanStatus::Cancelled;
        plan.updated_at = Utc::now();

        tracing::info!("Cancelled plan: {}", plan_id);

        Ok(())
    }
}

impl Default for Planner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_and_execute_plan() {
        let planner = Planner::new();

        let steps = vec![
            Step {
                id: Uuid::new_v4().to_string(),
                index: 0,
                title: "Setup environment".to_string(),
                instructions: "Install dependencies".to_string(),
                expected_outcome: "Dependencies installed".to_string(),
                status: StepStatus::Pending,
                step_type: StepType::Setup,
                dependencies: vec![],
                result: None,
                started_at: None,
                completed_at: None,
            },
            Step {
                id: Uuid::new_v4().to_string(),
                index: 1,
                title: "Implement feature".to_string(),
                instructions: "Write code".to_string(),
                expected_outcome: "Feature implemented".to_string(),
                status: StepStatus::Pending,
                step_type: StepType::Implementation,
                dependencies: vec![],
                result: None,
                started_at: None,
                completed_at: None,
            },
        ];

        let plan_id = planner
            .create_plan("Test Plan".to_string(), "A test plan".to_string(), steps)
            .await
            .unwrap();

        let plan = planner.get_plan(&plan_id).await.unwrap();
        assert_eq!(plan.steps.len(), 2);
        assert_eq!(plan.status, PlanStatus::Ready);

        planner.start_plan(&plan_id).await.unwrap();
        let plan = planner.get_plan(&plan_id).await.unwrap();
        assert_eq!(plan.status, PlanStatus::InProgress);
    }

    #[tokio::test]
    async fn test_step_execution() {
        let planner = Planner::new();
        let step_id = Uuid::new_v4().to_string();

        let steps = vec![Step {
            id: step_id.clone(),
            index: 0,
            title: "Test step".to_string(),
            instructions: "Do something".to_string(),
            expected_outcome: "Success".to_string(),
            status: StepStatus::Pending,
            step_type: StepType::Implementation,
            dependencies: vec![],
            result: None,
            started_at: None,
            completed_at: None,
        }];

        let plan_id = planner
            .create_plan("Test".to_string(), "Test".to_string(), steps)
            .await
            .unwrap();

        planner.start_step(&plan_id, &step_id).await.unwrap();

        let result = StepResult {
            success: true,
            output: "Done".to_string(),
            actions: vec!["action1".to_string()],
            files_modified: vec![],
            errors: vec![],
            lessons: vec![],
        };

        planner
            .complete_step(&plan_id, &step_id, result)
            .await
            .unwrap();

        let plan = planner.get_plan(&plan_id).await.unwrap();
        assert_eq!(plan.steps[0].status, StepStatus::Completed);
        assert_eq!(plan.status, PlanStatus::Completed);
    }
}
