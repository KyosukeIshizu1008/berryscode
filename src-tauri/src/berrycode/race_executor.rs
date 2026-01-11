//! Race Executor - Run multiple tasks in parallel and return the first successful result
//!
//! This module provides a "race" execution model where multiple tasks compete,
//! and the first one to produce a valid result wins. All other tasks are
//! immediately aborted to save resources.

use tokio::task::JoinHandle;
use std::future::Future;
use std::pin::Pin;

/// Race executor that runs multiple tasks in parallel and returns the first successful result
pub struct RaceExecutor;

impl RaceExecutor {
    /// Race multiple tasks against each other
    ///
    /// # Arguments
    /// * `tasks` - Vector of async tasks that return `Option<T>`
    ///
    /// # Returns
    /// * `Some(T)` - Result from the first task to complete with `Some(value)`
    /// * `None` - If all tasks complete with `None`
    ///
    /// # Behavior
    /// - All tasks start simultaneously (SHOTGUN START 🔫)
    /// - First task to return `Some(T)` wins
    /// - All other tasks are immediately aborted
    /// - If all tasks return `None`, returns `None`
    ///
    /// # Example
    /// ```no_run
    /// use berryscode::race_executor::RaceExecutor;
    ///
    /// async fn slow_task() -> Option<String> {
    ///     tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    ///     Some("slow".to_string())
    /// }
    ///
    /// async fn fast_task() -> Option<String> {
    ///     tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    ///     Some("fast".to_string())
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let result = RaceExecutor::race(vec![
    ///         Box::pin(slow_task()),
    ///         Box::pin(fast_task()),
    ///     ]).await;
    ///
    ///     assert_eq!(result, Some("fast".to_string()));
    /// }
    /// ```
    pub async fn race<T>(
        tasks: Vec<Pin<Box<dyn Future<Output = Option<T>> + Send>>>,
    ) -> Option<T>
    where
        T: Send + 'static,
    {
        if tasks.is_empty() {
            return None;
        }

        // Channel to receive the winner's result (capacity 1 = first come, first served)
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let mut handles: Vec<JoinHandle<()>> = Vec::new();

        tracing::debug!("🏁 Starting race with {} tasks", tasks.len());

        // Start all tasks simultaneously (SHOTGUN START)
        for (idx, task) in tasks.into_iter().enumerate() {
            let tx = tx.clone();
            let handle = tokio::spawn(async move {
                tracing::trace!("Task {} starting...", idx);
                let result = task.await;

                if let Some(value) = result {
                    tracing::debug!("✅ Task {} finished with result!", idx);
                    // Try to send result - only the first one will succeed
                    let _ = tx.send(value).await;
                } else {
                    tracing::trace!("Task {} returned None", idx);
                }
            });
            handles.push(handle);
        }

        // Drop the original sender so the channel closes when all tasks complete
        drop(tx);

        // Wait for the first winner
        let winner_result = rx.recv().await;

        if winner_result.is_some() {
            tracing::info!("🏆 Winner found! Aborting {} other tasks...", handles.len() - 1);
        } else {
            tracing::debug!("❌ All tasks returned None");
        }

        // Kill all tasks (including the winner - it's already done anyway)
        for handle in handles {
            handle.abort();
        }

        winner_result
    }

    /// Race multiple tasks with a timeout
    ///
    /// # Arguments
    /// * `tasks` - Vector of async tasks
    /// * `timeout` - Maximum duration to wait
    ///
    /// # Returns
    /// * `Some(T)` - Result from the first successful task
    /// * `None` - If timeout occurs or all tasks fail
    pub async fn race_with_timeout<T>(
        tasks: Vec<Pin<Box<dyn Future<Output = Option<T>> + Send>>>,
        timeout: std::time::Duration,
    ) -> Option<T>
    where
        T: Send + 'static,
    {
        tokio::time::timeout(timeout, Self::race(tasks))
            .await
            .ok()
            .flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_race_fastest_wins() {
        // Task A: Fast winner (10ms)
        let task_a = async {
            sleep(Duration::from_millis(10)).await;
            Some("A wins".to_string())
        };

        // Task B: Slow loser (1000ms)
        let task_b = async {
            sleep(Duration::from_secs(1)).await;
            Some("B wins".to_string())
        };

        // Task C: Medium loser (500ms)
        let task_c = async {
            sleep(Duration::from_millis(500)).await;
            Some("C wins".to_string())
        };

        let start = std::time::Instant::now();
        let result = RaceExecutor::race(vec![
            Box::pin(task_a),
            Box::pin(task_b),
            Box::pin(task_c),
        ])
        .await;
        let elapsed = start.elapsed();

        // Assertions
        assert_eq!(result, Some("A wins".to_string()));
        assert!(
            elapsed < Duration::from_millis(100),
            "Should complete quickly (~10ms), took {:?}",
            elapsed
        );

        println!("✅ Fastest task won in {:?}", elapsed);
    }

    #[tokio::test]
    async fn test_race_slow_tasks_are_aborted() {
        let completed_a = Arc::new(AtomicBool::new(false));
        let completed_b = Arc::new(AtomicBool::new(false));
        let completed_c = Arc::new(AtomicBool::new(false));

        let completed_a_clone = completed_a.clone();
        let completed_b_clone = completed_b.clone();
        let completed_c_clone = completed_c.clone();

        // Task A: Fast winner
        let task_a = async move {
            sleep(Duration::from_millis(10)).await;
            completed_a_clone.store(true, Ordering::SeqCst);
            Some("A".to_string())
        };

        // Task B: Should be aborted before completion
        let task_b = async move {
            sleep(Duration::from_millis(500)).await;
            completed_b_clone.store(true, Ordering::SeqCst);
            Some("B".to_string())
        };

        // Task C: Should be aborted before completion
        let task_c = async move {
            sleep(Duration::from_secs(1)).await;
            completed_c_clone.store(true, Ordering::SeqCst);
            Some("C".to_string())
        };

        let result = RaceExecutor::race(vec![
            Box::pin(task_a),
            Box::pin(task_b),
            Box::pin(task_c),
        ])
        .await;

        // Give a small delay to ensure aborted tasks don't complete
        sleep(Duration::from_millis(100)).await;

        // Assertions
        assert_eq!(result, Some("A".to_string()));
        assert!(completed_a.load(Ordering::SeqCst), "Winner A should complete");
        assert!(
            !completed_b.load(Ordering::SeqCst),
            "Loser B should be aborted before completion"
        );
        assert!(
            !completed_c.load(Ordering::SeqCst),
            "Loser C should be aborted before completion"
        );

        println!("✅ Slow tasks were successfully aborted");
    }

    #[tokio::test]
    async fn test_race_all_return_none() {
        let task_a = async { None };
        let task_b = async { None };
        let task_c = async { None };

        let result: Option<String> = RaceExecutor::race(vec![
            Box::pin(task_a),
            Box::pin(task_b),
            Box::pin(task_c),
        ])
        .await;

        assert_eq!(result, None);
        println!("✅ All tasks returned None correctly");
    }

    #[tokio::test]
    async fn test_race_mixed_some_and_none() {
        // Task A: Returns None
        let task_a = async {
            sleep(Duration::from_millis(10)).await;
            None
        };

        // Task B: Returns Some (winner)
        let task_b = async {
            sleep(Duration::from_millis(50)).await;
            Some("B found it".to_string())
        };

        // Task C: Would return Some but slower
        let task_c = async {
            sleep(Duration::from_millis(100)).await;
            Some("C found it".to_string())
        };

        let result = RaceExecutor::race(vec![
            Box::pin(task_a),
            Box::pin(task_b),
            Box::pin(task_c),
        ])
        .await;

        assert_eq!(result, Some("B found it".to_string()));
        println!("✅ First Some() wins even with None tasks");
    }

    #[tokio::test]
    async fn test_race_with_timeout_success() {
        let task_a = async {
            sleep(Duration::from_millis(10)).await;
            Some("Success".to_string())
        };

        let result = RaceExecutor::race_with_timeout(
            vec![Box::pin(task_a)],
            Duration::from_secs(1),
        )
        .await;

        assert_eq!(result, Some("Success".to_string()));
        println!("✅ Timeout race succeeded");
    }

    #[tokio::test]
    async fn test_race_with_timeout_expires() {
        let task_a = async {
            sleep(Duration::from_secs(5)).await;
            Some("Too slow".to_string())
        };

        let result: Option<String> = RaceExecutor::race_with_timeout(
            vec![Box::pin(task_a)],
            Duration::from_millis(100),
        )
        .await;

        assert_eq!(result, None);
        println!("✅ Timeout correctly expired");
    }

    #[tokio::test]
    async fn test_race_empty_tasks() {
        let result: Option<String> = RaceExecutor::race(vec![]).await;
        assert_eq!(result, None);
        println!("✅ Empty task list handled correctly");
    }

    #[tokio::test]
    async fn test_race_execution_count() {
        // Counter to track how many tasks actually complete their work
        let execution_count = Arc::new(AtomicU32::new(0));

        let count_clone_a = execution_count.clone();
        let count_clone_b = execution_count.clone();
        let count_clone_c = execution_count.clone();

        // Task A: Fast winner
        let task_a = async move {
            sleep(Duration::from_millis(10)).await;
            count_clone_a.fetch_add(1, Ordering::SeqCst);
            Some("A".to_string())
        };

        // Task B: Slow (should be aborted)
        let task_b = async move {
            sleep(Duration::from_secs(1)).await;
            count_clone_b.fetch_add(1, Ordering::SeqCst);
            Some("B".to_string())
        };

        // Task C: Slow (should be aborted)
        let task_c = async move {
            sleep(Duration::from_secs(2)).await;
            count_clone_c.fetch_add(1, Ordering::SeqCst);
            Some("C".to_string())
        };

        let result = RaceExecutor::race(vec![
            Box::pin(task_a),
            Box::pin(task_b),
            Box::pin(task_c),
        ])
        .await;

        // Give time to ensure aborted tasks don't increment
        sleep(Duration::from_millis(100)).await;

        assert_eq!(result, Some("A".to_string()));
        assert_eq!(
            execution_count.load(Ordering::SeqCst),
            1,
            "Only 1 task should complete (the winner)"
        );

        println!("✅ Only winner completed, losers were aborted");
    }
}
