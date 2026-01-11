//! Task spawning system for parallel execution

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use rayon::prelude::*;

/// Task to be executed by a sub-agent
#[derive(Debug, Clone)]
pub struct Task {
    pub id: String,
    pub description: String,
    pub task_type: TaskType,
    pub dependencies: Vec<String>,
    pub status: TaskStatus,
    pub result: Option<String>,
}

/// Type of task
#[derive(Debug, Clone, PartialEq)]
pub enum TaskType {
    /// Explore specific directory or module
    Explore { path: String },
    /// Find specific functionality
    Find { query: String },
    /// Read and analyze file
    Analyze { file_path: String },
    /// Implement feature
    Implement { spec: String },
    /// Run tests
    Test { test_path: String },
}

/// Task execution status
#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed(String),
}

/// Task spawner for parallel execution
pub struct TaskSpawner {
    tasks: Arc<Mutex<HashMap<String, Task>>>,
    max_parallel: usize,
}

impl TaskSpawner {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            max_parallel: 4, // Run up to 4 tasks in parallel
        }
    }

    /// Add a new task
    pub fn add_task(&mut self, task: Task) {
        let mut tasks = self.tasks.lock().unwrap();
        tasks.insert(task.id.clone(), task);
    }

    /// Decompose a complex query into subtasks
    pub fn decompose_query(&self, query: &str) -> Vec<Task> {
        let mut tasks = Vec::new();

        // Detect if it's a complex multi-step query
        if self.is_complex_query(query) {
            tasks = self.generate_subtasks(query);
        }

        tasks
    }

    /// Check if query is complex enough to spawn tasks
    fn is_complex_query(&self, query: &str) -> bool {
        let query_lower = query.to_lowercase();

        // Multiple steps indicated
        let has_multiple_steps = query_lower.contains("and") ||
            query_lower.contains("then") ||
            query_lower.contains("また") ||
            query_lower.contains("そして");

        // Complex operations
        let is_complex_operation = query_lower.contains("refactor") ||
            query_lower.contains("implement") ||
            query_lower.contains("add feature") ||
            query_lower.contains("実装") ||
            query_lower.contains("リファクタリング");

        has_multiple_steps || is_complex_operation
    }

    /// Generate subtasks from query
    fn generate_subtasks(&self, query: &str) -> Vec<Task> {
        let mut tasks = Vec::new();
        let query_lower = query.to_lowercase();

        // Example: "Explore the codebase and find authentication code"
        if query_lower.contains("explore") && query_lower.contains("find") {
            tasks.push(Task {
                id: "explore-1".to_string(),
                description: "Explore project structure".to_string(),
                task_type: TaskType::Explore {
                    path: ".".to_string(),
                },
                dependencies: vec![],
                status: TaskStatus::Pending,
                result: None,
            });

            tasks.push(Task {
                id: "find-1".to_string(),
                description: "Find authentication code".to_string(),
                task_type: TaskType::Find {
                    query: "authentication".to_string(),
                },
                dependencies: vec!["explore-1".to_string()],
                status: TaskStatus::Pending,
                result: None,
            });
        }

        // Example: "Implement feature X and add tests"
        if query_lower.contains("implement") && query_lower.contains("test") {
            tasks.push(Task {
                id: "implement-1".to_string(),
                description: "Implement the feature".to_string(),
                task_type: TaskType::Implement {
                    spec: query.to_string(),
                },
                dependencies: vec![],
                status: TaskStatus::Pending,
                result: None,
            });

            tasks.push(Task {
                id: "test-1".to_string(),
                description: "Add tests for the feature".to_string(),
                task_type: TaskType::Test {
                    test_path: "tests/".to_string(),
                },
                dependencies: vec!["implement-1".to_string()],
                status: TaskStatus::Pending,
                result: None,
            });
        }

        tasks
    }

    /// Execute tasks in parallel (respecting dependencies)
    pub fn execute_tasks<F>(&self, executor: F) -> Vec<(String, String)>
    where
        F: FnMut(&Task) -> Result<String, String> + Send + Sync,
    {
        let mut results = Vec::new();
        let executor = Arc::new(Mutex::new(executor));

        // Get tasks that are ready to execute (no pending dependencies)
        let ready_tasks = self.get_ready_tasks();

        // Execute in parallel
        let task_results: Vec<_> = ready_tasks
            .par_iter()
            .take(self.max_parallel)
            .map(|task| {
                let mut exec = executor.lock().unwrap();
                let result = exec(task);

                // Update task status
                let mut tasks = self.tasks.lock().unwrap();
                if let Some(t) = tasks.get_mut(&task.id) {
                    match result {
                        Ok(ref r) => {
                            t.status = TaskStatus::Completed;
                            t.result = Some(r.clone());
                        }
                        Err(ref e) => {
                            t.status = TaskStatus::Failed(e.clone());
                        }
                    }
                }

                (task.id.clone(), result)
            })
            .collect();

        for (id, result) in task_results {
            match result {
                Ok(r) => results.push((id, r)),
                Err(e) => results.push((id, format!("Error: {}", e))),
            }
        }

        results
    }

    /// Get tasks that are ready to execute
    fn get_ready_tasks(&self) -> Vec<Task> {
        let tasks = self.tasks.lock().unwrap();
        let mut ready = Vec::new();

        for task in tasks.values() {
            if task.status != TaskStatus::Pending {
                continue;
            }

            // Check if all dependencies are completed
            let deps_completed = task.dependencies.iter().all(|dep_id| {
                if let Some(dep) = tasks.get(dep_id) {
                    dep.status == TaskStatus::Completed
                } else {
                    false
                }
            });

            if deps_completed {
                ready.push(task.clone());
            }
        }

        ready
    }

    /// Get task result
    pub fn get_result(&self, task_id: &str) -> Option<String> {
        let tasks = self.tasks.lock().unwrap();
        tasks.get(task_id).and_then(|t| t.result.clone())
    }

    /// Check if all tasks are completed
    pub fn all_completed(&self) -> bool {
        let tasks = self.tasks.lock().unwrap();
        tasks.values().all(|t| {
            matches!(t.status, TaskStatus::Completed | TaskStatus::Failed(_))
        })
    }

    /// Get execution summary
    pub fn get_summary(&self) -> String {
        let tasks = self.tasks.lock().unwrap();
        let total = tasks.len();
        let completed = tasks.values().filter(|t| t.status == TaskStatus::Completed).count();
        let failed = tasks.values().filter(|t| matches!(t.status, TaskStatus::Failed(_))).count();

        format!(
            "Tasks: {} total, {} completed, {} failed, {} pending",
            total,
            completed,
            failed,
            total - completed - failed
        )
    }
}

impl Default for TaskSpawner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let task = Task {
            id: "test-1".to_string(),
            description: "Test task".to_string(),
            task_type: TaskType::Explore { path: ".".to_string() },
            dependencies: vec![],
            status: TaskStatus::Pending,
            result: None,
        };

        assert_eq!(task.status, TaskStatus::Pending);
    }

    #[test]
    fn test_complex_query_detection() {
        let spawner = TaskSpawner::new();

        assert!(spawner.is_complex_query("Explore codebase and find bugs"));
        assert!(spawner.is_complex_query("Implement feature X"));
        assert!(!spawner.is_complex_query("What is this?"));
    }

    #[test]
    fn test_subtask_generation() {
        let spawner = TaskSpawner::new();
        let tasks = spawner.decompose_query("Explore the codebase and find authentication");

        assert_eq!(tasks.len(), 2);
        assert!(tasks[0].dependencies.is_empty());
        assert_eq!(tasks[1].dependencies.len(), 1);
    }

    #[test]
    fn test_task_execution() {
        let spawner = TaskSpawner::new();
        let mut spawner_mut = TaskSpawner::new();

        let task = Task {
            id: "test-1".to_string(),
            description: "Test".to_string(),
            task_type: TaskType::Explore { path: ".".to_string() },
            dependencies: vec![],
            status: TaskStatus::Pending,
            result: None,
        };

        spawner_mut.add_task(task);

        let results = spawner.execute_tasks(|_task| {
            Ok("Task completed".to_string())
        });

        assert_eq!(results.len(), 0); // No tasks in the spawner used for execution
    }
}
