//! Plan mode for complex task planning

use std::fmt;

/// Implementation plan step
#[derive(Debug, Clone)]
pub struct PlanStep {
    pub step_number: usize,
    pub description: String,
    pub files_to_modify: Vec<String>,
    pub estimated_complexity: Complexity,
    pub dependencies: Vec<usize>, // Step numbers this depends on
}

/// Task complexity
#[derive(Debug, Clone, PartialEq)]
pub enum Complexity {
    Simple,
    Moderate,
    Complex,
}

impl fmt::Display for Complexity {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Complexity::Simple => write!(f, "簡単"),
            Complexity::Moderate => write!(f, "中程度"),
            Complexity::Complex => write!(f, "複雑"),
        }
    }
}

/// Implementation plan
#[derive(Debug, Clone)]
pub struct Plan {
    pub title: String,
    pub description: String,
    pub steps: Vec<PlanStep>,
    pub risks: Vec<String>,
    pub estimated_time: String,
    pub approved: bool,
}

impl Plan {
    pub fn new(title: String, description: String) -> Self {
        Self {
            title,
            description,
            steps: Vec::new(),
            risks: Vec::new(),
            estimated_time: "Unknown".to_string(),
            approved: false,
        }
    }

    /// Add a step to the plan
    pub fn add_step(&mut self, step: PlanStep) {
        self.steps.push(step);
    }

    /// Add a risk
    pub fn add_risk(&mut self, risk: String) {
        self.risks.push(risk);
    }

    /// Get total complexity
    pub fn total_complexity(&self) -> Complexity {
        let complex_count = self.steps.iter().filter(|s| s.estimated_complexity == Complexity::Complex).count();
        let moderate_count = self.steps.iter().filter(|s| s.estimated_complexity == Complexity::Moderate).count();

        if complex_count > 0 || self.steps.len() > 5 {
            Complexity::Complex
        } else if moderate_count > 0 || self.steps.len() > 2 {
            Complexity::Moderate
        } else {
            Complexity::Simple
        }
    }

    /// Format plan for display
    pub fn format_plan(&self) -> String {
        let mut output = String::new();

        output.push_str("═══════════════════════════════════════════════════════\n");
        output.push_str(&format!("📋 実装計画: {}\n", self.title));
        output.push_str("═══════════════════════════════════════════════════════\n\n");

        output.push_str(&format!("**説明**: {}\n\n", self.description));

        output.push_str(&format!("**全体の複雑度**: {}\n", self.total_complexity()));
        output.push_str(&format!("**ステップ数**: {}\n", self.steps.len()));
        output.push_str(&format!("**推定時間**: {}\n\n", self.estimated_time));

        output.push_str("**実装ステップ**:\n");
        output.push_str("───────────────────────────────────────────────────────\n");

        for step in &self.steps {
            output.push_str(&format!("\n**{}. {}** [{}]\n", step.step_number, step.description, step.estimated_complexity));

            if !step.files_to_modify.is_empty() {
                output.push_str("   変更するファイル:\n");
                for file in &step.files_to_modify {
                    output.push_str(&format!("   - {}\n", file));
                }
            }

            if !step.dependencies.is_empty() {
                output.push_str(&format!("   依存関係: {:?}\n", step.dependencies));
            }
        }

        if !self.risks.is_empty() {
            output.push_str("\n**⚠️  リスク**:\n");
            output.push_str("───────────────────────────────────────────────────────\n");
            for (i, risk) in self.risks.iter().enumerate() {
                output.push_str(&format!("{}. {}\n", i + 1, risk));
            }
        }

        output.push_str("\n═══════════════════════════════════════════════════════\n");

        output
    }
}

/// Plan mode manager
pub struct PlanMode {
    current_plan: Option<Plan>,
    enabled: bool,
}

impl PlanMode {
    pub fn new() -> Self {
        Self {
            current_plan: None,
            enabled: true,
        }
    }

    /// Check if query requires planning
    pub fn requires_planning(&self, query: &str) -> bool {
        if !self.enabled {
            return false;
        }

        let query_lower = query.to_lowercase();

        // Keywords that indicate need for planning
        let planning_keywords = [
            "implement",
            "add feature",
            "refactor",
            "redesign",
            "migrate",
            "create",
            "build",
            "実装",
            "追加",
            "リファクタリング",
            "作成",
        ];

        // Complexity indicators
        let complexity_indicators = [
            "multiple",
            "complex",
            "large",
            "entire",
            "全体",
            "複数",
            "複雑",
        ];

        let has_planning_keyword = planning_keywords.iter().any(|kw| query_lower.contains(kw));
        let has_complexity = complexity_indicators.iter().any(|ind| query_lower.contains(ind));

        has_planning_keyword || has_complexity
    }

    /// Generate a plan from query
    pub fn generate_plan(&mut self, query: &str) -> Plan {
        let query_lower = query.to_lowercase();
        let mut plan = if query_lower.contains("implement") || query_lower.contains("実装") {
            Plan::new(
                "新機能実装".to_string(),
                query.to_string(),
            )
        } else if query_lower.contains("refactor") || query_lower.contains("リファクタ") {
            Plan::new(
                "コードリファクタリング".to_string(),
                query.to_string(),
            )
        } else if query_lower.contains("追加") || query_lower.contains("add") {
            Plan::new(
                "機能追加".to_string(),
                query.to_string(),
            )
        } else {
            Plan::new(
                "コード変更".to_string(),
                query.to_string(),
            )
        };

        // Generate steps based on query analysis
        self.analyze_and_add_steps(&mut plan, query);

        // Add common risks
        self.add_common_risks(&mut plan);

        // Estimate time
        plan.estimated_time = self.estimate_time(&plan);

        self.current_plan = Some(plan.clone());
        plan
    }

    /// Analyze query and add steps
    fn analyze_and_add_steps(&self, plan: &mut Plan, query: &str) {
        // Step 1: Always understand current code
        plan.add_step(PlanStep {
            step_number: 1,
            description: "現在のコードベース構造を理解する".to_string(),
            files_to_modify: vec![],
            estimated_complexity: Complexity::Simple,
            dependencies: vec![],
        });

        // Step 2: Identify files to modify
        plan.add_step(PlanStep {
            step_number: 2,
            description: "変更が必要なファイルを特定する".to_string(),
            files_to_modify: vec![],
            estimated_complexity: Complexity::Simple,
            dependencies: vec![1],
        });

        // Step 3: Make changes
        plan.add_step(PlanStep {
            step_number: 3,
            description: "必要な変更を実装する".to_string(),
            files_to_modify: vec!["(後で決定)".to_string()],
            estimated_complexity: if query.len() > 100 {
                Complexity::Complex
            } else {
                Complexity::Moderate
            },
            dependencies: vec![2],
        });

        // Step 4: Test
        plan.add_step(PlanStep {
            step_number: 4,
            description: "テストを実行して変更を検証する".to_string(),
            files_to_modify: vec![],
            estimated_complexity: Complexity::Simple,
            dependencies: vec![3],
        });

        // Step 5: Commit if all good
        plan.add_step(PlanStep {
            step_number: 5,
            description: "テストが通ったら変更をコミットする".to_string(),
            files_to_modify: vec![],
            estimated_complexity: Complexity::Simple,
            dependencies: vec![4],
        });
    }

    /// Add common risks
    fn add_common_risks(&self, plan: &mut Plan) {
        plan.add_risk("既存機能を壊す可能性".to_string());
        plan.add_risk("エッジケースでバグが発生する可能性".to_string());

        if plan.total_complexity() == Complexity::Complex {
            plan.add_risk("複雑な変更のため複数回の反復が必要になる可能性".to_string());
            plan.add_risk("コードベースの他の部分に影響を与える可能性".to_string());
        }
    }

    /// Estimate time
    fn estimate_time(&self, plan: &Plan) -> String {
        match plan.total_complexity() {
            Complexity::Simple => "5〜10分".to_string(),
            Complexity::Moderate => "15〜30分".to_string(),
            Complexity::Complex => "30分以上".to_string(),
        }
    }

    /// Get current plan
    pub fn get_plan(&self) -> Option<&Plan> {
        self.current_plan.as_ref()
    }

    /// Approve current plan
    pub fn approve_plan(&mut self) {
        if let Some(ref mut plan) = self.current_plan {
            plan.approved = true;
        }
    }

    /// Clear current plan
    pub fn clear_plan(&mut self) {
        self.current_plan = None;
    }

    /// Check if there's an approved plan
    pub fn has_approved_plan(&self) -> bool {
        self.current_plan.as_ref().map_or(false, |p| p.approved)
    }
}

impl Default for PlanMode {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_creation() {
        let mut plan = Plan::new("Test".to_string(), "Description".to_string());

        plan.add_step(PlanStep {
            step_number: 1,
            description: "Step 1".to_string(),
            files_to_modify: vec![],
            estimated_complexity: Complexity::Simple,
            dependencies: vec![],
        });

        assert_eq!(plan.steps.len(), 1);
    }

    #[test]
    fn test_requires_planning() {
        let plan_mode = PlanMode::new();

        assert!(plan_mode.requires_planning("Implement new feature"));
        assert!(plan_mode.requires_planning("Refactor the code"));
        assert!(!plan_mode.requires_planning("What is this file?"));
    }

    #[test]
    fn test_complexity_calculation() {
        let mut plan = Plan::new("Test".to_string(), "Desc".to_string());

        plan.add_step(PlanStep {
            step_number: 1,
            description: "Simple step".to_string(),
            files_to_modify: vec![],
            estimated_complexity: Complexity::Simple,
            dependencies: vec![],
        });

        assert_eq!(plan.total_complexity(), Complexity::Simple);

        plan.add_step(PlanStep {
            step_number: 2,
            description: "Complex step".to_string(),
            files_to_modify: vec![],
            estimated_complexity: Complexity::Complex,
            dependencies: vec![],
        });

        assert_eq!(plan.total_complexity(), Complexity::Complex);
    }

    #[test]
    fn test_plan_formatting() {
        let mut plan = Plan::new("Test Plan".to_string(), "Test description".to_string());
        plan.add_step(PlanStep {
            step_number: 1,
            description: "Test step".to_string(),
            files_to_modify: vec!["file.rs".to_string()],
            estimated_complexity: Complexity::Moderate,
            dependencies: vec![],
        });

        let formatted = plan.format_plan();
        assert!(formatted.contains("Test Plan"));
        assert!(formatted.contains("Test step"));
    }
}
