use super::types::WorkflowPreset;

/// Get all available workflow presets
pub fn get_workflow_presets() -> Vec<WorkflowPreset> {
    vec![
        WorkflowPreset {
            id: "tdd-loop".to_string(),
            name: "TDD Loop".to_string(),
            description: "Test → Implementation → Fix → Repeat until all tests pass".to_string(),
            icon: "codicon-debug-alt".to_string(),
            nodes_count: 3,
        },
        WorkflowPreset {
            id: "full-dev".to_string(),
            name: "Full Development Pipeline".to_string(),
            description: "Design → Implement → Test → Fix → Refactor → Document → Commit".to_string(),
            icon: "codicon-rocket".to_string(),
            nodes_count: 7,
        },
        WorkflowPreset {
            id: "bug-fix".to_string(),
            name: "Bug Fix Workflow".to_string(),
            description: "Analyze → Reproduce → Fix → Test → Verify".to_string(),
            icon: "codicon-bug".to_string(),
            nodes_count: 5,
        },
        WorkflowPreset {
            id: "refactor".to_string(),
            name: "Code Refactoring".to_string(),
            description: "Analyze → Plan → Refactor → Test → Review".to_string(),
            icon: "codicon-symbol-method".to_string(),
            nodes_count: 5,
        },
        WorkflowPreset {
            id: "feature-dev".to_string(),
            name: "Feature Development".to_string(),
            description: "Requirements → Design → Implement → Test → Document".to_string(),
            icon: "codicon-sparkle".to_string(),
            nodes_count: 5,
        },
    ]
}
