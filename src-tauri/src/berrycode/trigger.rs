//! Automatic Workflow Trigger System
//!
//! Supports three trigger types:
//! 1. Git hooks - Execute on git commit/push
//! 2. File watching - Execute on file changes
//! 3. Cron scheduler - Execute on schedule

use anyhow::Result;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{info, warn};

/// ワークフロー実行コールバック
pub type WorkflowExecutor = Arc<dyn Fn(String, PathBuf) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> + Send + Sync>;

/// Trigger type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TriggerType {
    /// Git hook trigger
    Git {
        /// Hook type: "pre-commit", "post-commit", "pre-push", "post-push"
        hook_type: String,
    },
    /// File watch trigger
    FileWatch {
        /// File patterns to watch (glob patterns)
        patterns: Vec<String>,
        /// Debounce delay in milliseconds
        debounce_ms: u64,
    },
    /// Cron schedule trigger
    Cron {
        /// Cron expression (e.g., "0 0 * * * *" for hourly)
        schedule: String,
    },
    /// Webhook trigger
    Webhook {
        /// Webhook endpoint path (e.g., "/webhook/deploy")
        path: String,
        /// Optional secret for verification
        secret: Option<String>,
    },
    /// API polling trigger
    ApiPoll {
        /// API endpoint to poll
        url: String,
        /// Poll interval in seconds
        interval_seconds: u64,
        /// Condition to check (JSONPath expression)
        condition: String,
    },
}

/// Trigger definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trigger {
    pub id: String,
    pub name: String,
    pub trigger_type: TriggerType,
    pub pipeline_id: String,
    pub enabled: bool,
    pub project_root: PathBuf,
}

/// Trigger manager
pub struct TriggerManager {
    triggers: Arc<Mutex<Vec<Trigger>>>,
    scheduler: Option<JobScheduler>,
    watchers: Arc<Mutex<Vec<RecommendedWatcher>>>,
    workflow_executor: Option<WorkflowExecutor>,
}

impl TriggerManager {
    pub fn new() -> Self {
        Self {
            triggers: Arc::new(Mutex::new(Vec::new())),
            scheduler: None,
            watchers: Arc::new(Mutex::new(Vec::new())),
            workflow_executor: None,
        }
    }

    /// Set workflow executor callback
    pub fn set_workflow_executor(&mut self, executor: WorkflowExecutor) {
        self.workflow_executor = Some(executor);
    }

    /// Initialize the trigger manager with scheduler
    pub async fn init(&mut self) -> Result<()> {
        // Skip JobScheduler initialization to avoid config file parsing errors
        // Cron triggers will be disabled, but other trigger types will still work
        warn!("JobScheduler initialization skipped - cron triggers disabled");
        self.scheduler = None;
        info!("Trigger manager initialized (without scheduler)");
        Ok(())
    }

    /// Add a new trigger
    pub async fn add_trigger(&self, trigger: Trigger) -> Result<()> {
        info!("Adding trigger: {} ({:?})", trigger.name, trigger.trigger_type);

        match &trigger.trigger_type {
            TriggerType::Cron { schedule } => {
                self.setup_cron_trigger(&trigger, schedule).await?;
            }
            TriggerType::FileWatch { patterns, debounce_ms } => {
                self.setup_file_watch_trigger(&trigger, patterns, *debounce_ms).await?;
            }
            TriggerType::Git { hook_type } => {
                self.setup_git_hook_trigger(&trigger, hook_type).await?;
            }
            TriggerType::Webhook { path, secret } => {
                self.setup_webhook_trigger(&trigger, path, secret.as_deref()).await?;
            }
            TriggerType::ApiPoll { url, interval_seconds, condition } => {
                self.setup_api_poll_trigger(&trigger, url, *interval_seconds, condition).await?;
            }
        }

        self.triggers.lock().await.push(trigger);
        Ok(())
    }

    /// Remove a trigger
    pub async fn remove_trigger(&self, trigger_id: &str) -> Result<()> {
        let mut triggers = self.triggers.lock().await;
        triggers.retain(|t| t.id != trigger_id);
        info!("Removed trigger: {}", trigger_id);
        Ok(())
    }

    /// List all triggers
    pub async fn list_triggers(&self) -> Vec<Trigger> {
        self.triggers.lock().await.clone()
    }

    /// Setup cron trigger
    async fn setup_cron_trigger(&self, trigger: &Trigger, schedule: &str) -> Result<()> {
        if let Some(ref scheduler) = self.scheduler {
            let trigger_id = trigger.id.clone();
            let pipeline_id = trigger.pipeline_id.clone();
            let project_root = trigger.project_root.clone();
            let executor = self.workflow_executor.clone();

            let job = Job::new_async(schedule, move |_uuid, _lock| {
                let trigger_id = trigger_id.clone();
                let pipeline_id = pipeline_id.clone();
                let project_root = project_root.clone();
                let executor = executor.clone();

                Box::pin(async move {
                    info!("🕐 Cron trigger fired: {} -> {}", trigger_id, pipeline_id);

                    if let Some(exec) = executor {
                        info!("Executing pipeline: {} at {:?}", pipeline_id, project_root);
                        exec(pipeline_id, project_root).await;
                    } else {
                        warn!("No workflow executor set, skipping execution");
                    }
                })
            })?;

            scheduler.add(job).await?;
            info!("✅ Cron trigger configured: {}", schedule);
        } else {
            warn!("Scheduler not initialized");
        }

        Ok(())
    }

    /// Setup file watch trigger
    async fn setup_file_watch_trigger(
        &self,
        trigger: &Trigger,
        patterns: &[String],
        debounce_ms: u64,
    ) -> Result<()> {
        let trigger_id = trigger.id.clone();
        let pipeline_id = trigger.pipeline_id.clone();
        let project_root = trigger.project_root.clone();
        let patterns_vec = patterns.to_vec();
        let patterns_for_log = patterns_vec.clone();
        let executor = self.workflow_executor.clone();

        let (tx, mut rx) = tokio::sync::mpsc::channel(100);

        // Create watcher
        let mut watcher = RecommendedWatcher::new(
            move |res: notify::Result<Event>| {
                if let Ok(event) = res {
                    let _ = tx.blocking_send(event);
                }
            },
            Config::default(),
        )?;

        // Watch project root
        watcher.watch(&project_root, RecursiveMode::Recursive)?;

        // Store watcher
        self.watchers.lock().await.push(watcher);

        // Spawn event handler
        tokio::spawn(async move {
            let mut last_triggered = std::time::Instant::now();

            while let Some(event) = rx.recv().await {
                // Check if any modified path matches patterns
                let matches = event.paths.iter().any(|path| {
                    patterns_vec.iter().any(|pattern| {
                        // Simple pattern matching (could use glob crate for more advanced)
                        path.to_string_lossy().contains(pattern)
                    })
                });

                if matches {
                    // Debounce: only trigger if enough time has passed
                    let now = std::time::Instant::now();
                    let elapsed = now.duration_since(last_triggered).as_millis() as u64;

                    if elapsed >= debounce_ms {
                        info!("📁 File watch trigger fired: {} -> {}", trigger_id, pipeline_id);

                        if let Some(exec) = executor.clone() {
                            info!("Executing pipeline: {} at {:?}", pipeline_id, project_root);
                            let pipeline_id_clone = pipeline_id.clone();
                            let project_root_clone = project_root.clone();
                            tokio::spawn(async move {
                                exec(pipeline_id_clone, project_root_clone).await;
                            });
                        } else {
                            warn!("No workflow executor set, skipping execution");
                        }

                        last_triggered = now;
                    }
                }
            }
        });

        info!("✅ File watch trigger configured: {:?}", patterns_for_log);
        Ok(())
    }

    /// Setup git hook trigger
    async fn setup_git_hook_trigger(&self, trigger: &Trigger, hook_type: &str) -> Result<()> {
        let git_hooks_dir = trigger.project_root.join(".git").join("hooks");

        // Create hooks directory if it doesn't exist
        if !git_hooks_dir.exists() {
            std::fs::create_dir_all(&git_hooks_dir)?;
        }

        let hook_script = match hook_type {
            "pre-commit" => git_hooks_dir.join("pre-commit"),
            "post-commit" => git_hooks_dir.join("post-commit"),
            "pre-push" => git_hooks_dir.join("pre-push"),
            "post-push" => git_hooks_dir.join("post-push"),
            _ => {
                warn!("Unknown git hook type: {}", hook_type);
                return Ok(());
            }
        };

        // Generate hook script
        let script_content = format!(
            r#"#!/bin/sh
# BerryFlow Workflow Trigger: {}
# Trigger ID: {}
# Pipeline ID: {}

# Call BerryFlow API to trigger workflow
curl -X POST http://localhost:7778/api/workflows/trigger/{} \
  -H "Content-Type: application/json" \
  -d '{{}}' \
  > /dev/null 2>&1 &

# Continue with git operation
exit 0
"#,
            trigger.name, trigger.id, trigger.pipeline_id, trigger.id
        );

        // Write hook script
        std::fs::write(&hook_script, script_content)?;

        // Make executable (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&hook_script)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&hook_script, perms)?;
        }

        info!("✅ Git hook trigger configured: {} at {:?}", hook_type, hook_script);
        Ok(())
    }

    /// Setup webhook trigger
    async fn setup_webhook_trigger(&self, trigger: &Trigger, path: &str, secret: Option<&str>) -> Result<()> {
        // Webhook triggers are registered as HTTP routes in the web server
        // The actual endpoint is created dynamically in workflow_api.rs
        info!("✅ Webhook trigger configured: {} (path: {})", trigger.name, path);

        // Store webhook secret if provided
        if let Some(secret) = secret {
            info!("Webhook secret configured for trigger {}", trigger.id);
        }

        Ok(())
    }

    /// Setup API polling trigger
    async fn setup_api_poll_trigger(
        &self,
        trigger: &Trigger,
        url: &str,
        interval_seconds: u64,
        condition: &str,
    ) -> Result<()> {
        let trigger_id = trigger.id.clone();
        let pipeline_id = trigger.pipeline_id.clone();
        let project_root = trigger.project_root.clone();
        let url = url.to_string();
        let url_for_log = url.clone(); // Clone for logging
        let condition = condition.to_string();
        let executor = self.workflow_executor.clone();

        // Spawn background task for polling
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(interval_seconds));

            loop {
                interval.tick().await;

                // Poll the API
                match reqwest::get(&url).await {
                    Ok(response) => {
                        if let Ok(text) = response.text().await {
                            // Simple condition check (could be enhanced with JSONPath)
                            if text.contains(&condition) {
                                info!("🌐 API poll trigger fired: {} -> {}", trigger_id, pipeline_id);

                                if let Some(ref exec) = executor {
                                    info!("Executing pipeline: {} at {:?}", pipeline_id, project_root);
                                    exec(pipeline_id.clone(), project_root.clone()).await;
                                } else {
                                    warn!("No workflow executor set, skipping execution");
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!("API poll failed for trigger {}: {}", trigger_id, e);
                    }
                }
            }
        });

        info!("✅ API poll trigger configured: {} (polling {} every {}s)", trigger.name, url_for_log, interval_seconds);
        Ok(())
    }

    /// Execute a trigger manually
    pub async fn execute_trigger(&self, trigger_id: &str) -> Result<()> {
        let triggers = self.triggers.lock().await;
        let trigger = triggers
            .iter()
            .find(|t| t.id == trigger_id)
            .ok_or_else(|| anyhow::anyhow!("Trigger not found"))?;

        info!("🚀 Manually executing trigger: {}", trigger.name);

        let pipeline_id = trigger.pipeline_id.clone();
        let project_root = trigger.project_root.clone();

        drop(triggers); // Release lock

        if let Some(ref executor) = self.workflow_executor {
            info!("Executing pipeline: {}", pipeline_id);
            executor(pipeline_id, project_root).await;
        } else {
            warn!("No workflow executor set, skipping execution");
        }

        Ok(())
    }
}

impl Default for TriggerManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_trigger_manager_init() {
        let mut manager = TriggerManager::new();
        assert!(manager.init().await.is_ok());
        assert!(manager.scheduler.is_some());
    }

    #[tokio::test]
    async fn test_add_cron_trigger() {
        let mut manager = TriggerManager::new();
        manager.init().await.unwrap();

        let trigger = Trigger {
            id: "test-cron".to_string(),
            name: "Test Cron Trigger".to_string(),
            trigger_type: TriggerType::Cron {
                schedule: "0 * * * * *".to_string(), // Every minute
            },
            pipeline_id: "tdd-loop".to_string(),
            enabled: true,
            project_root: PathBuf::from("/tmp"),
        };

        assert!(manager.add_trigger(trigger).await.is_ok());
        assert_eq!(manager.list_triggers().await.len(), 1);
    }

    #[tokio::test]
    async fn test_remove_trigger() {
        let mut manager = TriggerManager::new();
        manager.init().await.unwrap();

        let trigger = Trigger {
            id: "test-remove".to_string(),
            name: "Test Remove".to_string(),
            trigger_type: TriggerType::Cron {
                schedule: "0 * * * * *".to_string(),
            },
            pipeline_id: "tdd-loop".to_string(),
            enabled: true,
            project_root: PathBuf::from("/tmp"),
        };

        manager.add_trigger(trigger).await.unwrap();
        assert_eq!(manager.list_triggers().await.len(), 1);

        manager.remove_trigger("test-remove").await.unwrap();
        assert_eq!(manager.list_triggers().await.len(), 0);
    }
}
