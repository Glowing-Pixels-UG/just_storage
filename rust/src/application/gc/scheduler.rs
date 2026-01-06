use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time;
use tracing::{error, info};

/// Scheduler for periodic tasks with configurable intervals
pub struct TaskScheduler {
    interval: Duration,
    last_run: std::sync::Mutex<Instant>,
}

impl TaskScheduler {
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            last_run: std::sync::Mutex::new(Instant::now() - interval), // Allow immediate first run
        }
    }

    /// Check if the task should run based on the interval
    pub fn should_run(&self) -> bool {
        let now = Instant::now();
        let mut last_run = self.last_run.lock().unwrap();
        if now.duration_since(*last_run) >= self.interval {
            *last_run = now;
            true
        } else {
            false
        }
    }

    /// Reset the scheduler to allow immediate next run
    pub fn reset(&self) {
        let mut last_run = self.last_run.lock().unwrap();
        *last_run = Instant::now() - self.interval;
    }

    /// Get the time until next run
    pub fn time_until_next_run(&self) -> Duration {
        let last_run = self.last_run.lock().unwrap();
        let elapsed = last_run.elapsed();
        if elapsed >= self.interval {
            Duration::ZERO
        } else {
            self.interval - elapsed
        }
    }
}

/// Runner for periodic tasks with error handling and logging
pub struct PeriodicTaskRunner<T> {
    task: Arc<T>,
    scheduler: TaskScheduler,
    task_name: String,
}

impl<T> PeriodicTaskRunner<T>
where
    T: Send + Sync + 'static,
{
    pub fn new(task: Arc<T>, interval: Duration, task_name: impl Into<String>) -> Self {
        Self {
            task,
            scheduler: TaskScheduler::new(interval),
            task_name: task_name.into(),
        }
    }

    /// Run the periodic task
    pub async fn run<F, Fut>(&self, task_fn: F)
    where
        F: Fn(Arc<T>) -> Fut,
        Fut: std::future::Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>>,
    {
        info!(
            "Starting periodic task: {} with interval: {:?}",
            self.task_name, self.scheduler.interval
        );

        let mut interval_timer = time::interval(self.scheduler.interval);

        loop {
            interval_timer.tick().await;

            if let Err(e) = task_fn(Arc::clone(&self.task)).await {
                error!("Periodic task {} failed: {}", self.task_name, e);
            }
        }
    }

    /// Run the task once (for testing)
    pub async fn run_once<F, Fut>(
        &self,
        task_fn: F,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        F: Fn(Arc<T>) -> Fut,
        Fut: std::future::Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>>,
    {
        task_fn(Arc::clone(&self.task)).await
    }
}

/// Conditional runner that only runs when certain conditions are met
pub struct ConditionalTaskRunner<T, C> {
    task: Arc<T>,
    condition_checker: C,
}

impl<T, C> ConditionalTaskRunner<T, C>
where
    T: Send + Sync + 'static,
    C: Send + Sync + 'static,
{
    pub fn new(task: Arc<T>, condition_checker: C) -> Self {
        Self {
            task,
            condition_checker,
        }
    }

    /// Run the task if condition is met
    pub async fn run_if<F, Fut>(
        &self,
        condition_fn: impl Fn(&C) -> bool,
        task_fn: F,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        F: Fn(Arc<T>) -> Fut,
        Fut: std::future::Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>>,
    {
        if condition_fn(&self.condition_checker) {
            task_fn(Arc::clone(&self.task)).await
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::time::timeout;

    #[tokio::test]
    async fn test_task_scheduler_should_run() {
        let scheduler = TaskScheduler::new(Duration::from_millis(100));

        // Should run immediately on first check
        assert!(scheduler.should_run());

        // Should not run immediately after
        assert!(!scheduler.should_run());

        // Wait for interval to pass
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert!(scheduler.should_run());
    }

    #[tokio::test]
    async fn test_task_scheduler_reset() {
        let scheduler = TaskScheduler::new(Duration::from_millis(100));

        // Run once
        assert!(scheduler.should_run());
        assert!(!scheduler.should_run());

        // Reset and should run again
        scheduler.reset();
        assert!(scheduler.should_run());
    }

    #[tokio::test]
    async fn test_periodic_task_runner() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        let runner = PeriodicTaskRunner::new(Arc::new(()), Duration::from_millis(50), "test_task");

        let task_fn = move |_| {
            let counter = Arc::clone(&counter_clone);
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        };

        // Run with timeout to avoid infinite loop
        let _ = timeout(Duration::from_millis(200), runner.run(task_fn)).await;

        // Should have run multiple times
        assert!(counter.load(Ordering::SeqCst) >= 2);
    }

    #[tokio::test]
    async fn test_conditional_task_runner() {
        let task_data = Arc::new(AtomicUsize::new(0));
        let task_data_clone = Arc::clone(&task_data);

        let condition = true;
        let runner = ConditionalTaskRunner::new(Arc::new(()), condition);

        let task_fn = move |_| {
            let task_data = Arc::clone(&task_data_clone);
            async move {
                task_data.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        };

        runner.run_if(|&cond| cond, task_fn).await.unwrap();
        assert_eq!(task_data.load(Ordering::SeqCst), 1);

        // Test with false condition
        let false_condition = false;
        let runner_false = ConditionalTaskRunner::new(Arc::new(()), false_condition);

        runner_false
            .run_if(|&cond| cond, |_| async { Ok(()) })
            .await
            .unwrap();
        // Counter should still be 1
        assert_eq!(task_data.load(Ordering::SeqCst), 1);
    }
}
