//! Retry logic with exponential backoff

use std::time::Duration;
use anyhow::Result;

/// Retry configuration
pub struct RetryConfig {
    pub max_attempts: usize,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub multiplier: f32,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            multiplier: 2.0,
        }
    }
}

/// Retry a function with exponential backoff
pub fn retry_with_backoff<F, T>(config: &RetryConfig, mut f: F) -> Result<T>
where
    F: FnMut() -> Result<T>,
{
    let mut attempt = 0;
    let mut delay = config.initial_delay;

    loop {
        attempt += 1;

        match f() {
            Ok(result) => return Ok(result),
            Err(e) if attempt >= config.max_attempts => {
                return Err(e);
            }
            Err(_) => {
                // Wait before retrying
                std::thread::sleep(delay);

                // Calculate next delay with exponential backoff
                delay = Duration::from_secs_f32(
                    (delay.as_secs_f32() * config.multiplier).min(config.max_delay.as_secs_f32())
                );
            }
        }
    }
}

/// Retry an async function with exponential backoff
pub async fn retry_with_backoff_async<F, Fut, T>(config: &RetryConfig, mut f: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut attempt = 0;
    let mut delay = config.initial_delay;

    loop {
        attempt += 1;

        match f().await {
            Ok(result) => return Ok(result),
            Err(e) if attempt >= config.max_attempts => {
                return Err(e);
            }
            Err(_) => {
                // Wait before retrying
                tokio::time::sleep(delay).await;

                // Calculate next delay with exponential backoff
                delay = Duration::from_secs_f32(
                    (delay.as_secs_f32() * config.multiplier).min(config.max_delay.as_secs_f32())
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_retry_succeeds_on_first_attempt() {
        let config = RetryConfig::default();
        let result = retry_with_backoff(&config, || Ok(42));
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_retry_succeeds_after_failures() {
        let config = RetryConfig::default();
        let counter = Arc::new(Mutex::new(0));
        let counter_clone = Arc::clone(&counter);

        let result = retry_with_backoff(&config, || {
            let mut count = counter_clone.lock().unwrap();
            *count += 1;

            if *count < 3 {
                Err(anyhow::anyhow!("Failed"))
            } else {
                Ok(42)
            }
        });

        assert_eq!(result.unwrap(), 42);
        assert_eq!(*counter.lock().unwrap(), 3);
    }

    #[test]
    fn test_retry_fails_after_max_attempts() {
        let config = RetryConfig {
            max_attempts: 2,
            ..Default::default()
        };

        let result = retry_with_backoff(&config, || {
            Err::<i32, _>(anyhow::anyhow!("Always fails"))
        });

        assert!(result.is_err());
    }
}
