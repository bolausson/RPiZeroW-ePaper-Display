//! Refresh scheduler for periodic display updates.
//!
//! Manages automatic refresh of the display at configurable intervals.
//! Includes failure tracking and exponential backoff for resilience.

use crate::config::Config;
use crate::image_proc::ImageProcessor;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, RwLock};

/// Scheduler for periodic display refresh
///
/// Tracks consecutive failures and applies exponential backoff
/// to avoid hammering a failing resource.
pub struct Scheduler {
    config: Arc<RwLock<Config>>,
    processor: Arc<ImageProcessor>,
    /// Counter for consecutive failures
    consecutive_failures: AtomicU32,
}

impl Scheduler {
    /// Maximum consecutive failures before applying backoff
    const MAX_CONSECUTIVE_FAILURES: u32 = 5;

    /// Backoff multiplier (interval doubled for each failure beyond threshold)
    const FAILURE_BACKOFF_MULTIPLIER: u64 = 2;

    /// Maximum backoff duration (1 hour)
    const MAX_BACKOFF_SECS: u64 = 3600;

    /// Create a new scheduler
    pub fn new(config: Arc<RwLock<Config>>, processor: Arc<ImageProcessor>) -> Self {
        Self {
            config,
            processor,
            consecutive_failures: AtomicU32::new(0),
        }
    }

    /// Run the scheduler loop
    ///
    /// Periodically refreshes the display based on the configured interval.
    /// Listens for shutdown signal to gracefully stop.
    /// Applies exponential backoff after repeated failures.
    pub async fn run(&self, mut shutdown: broadcast::Receiver<()>) {
        tracing::info!("Scheduler started");

        // Initial delay before first refresh (wait for system to stabilize)
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(10)) => {}
            _ = shutdown.recv() => {
                tracing::info!("Scheduler shutdown before initial refresh");
                return;
            }
        }

        // Initial refresh
        self.refresh_display().await;

        loop {
            // Get current interval from config based on day and time, with backoff applied
            let interval = {
                let config = self.config.read().await;
                let current_interval = config.get_current_interval();
                let base_interval = Duration::from_secs(current_interval as u64 * 60);

                if let Some(plan) = config.get_current_plan() {
                    if let Some(period) = config.get_current_period() {
                        tracing::debug!(
                            "Active plan: '{}' ({}) - period {} - {} (every {} min)",
                            plan.name,
                            crate::config::Config::get_current_weekday().display_name(),
                            period.start_time,
                            period.end_time,
                            period.interval_min
                        );
                    }
                }

                self.get_effective_interval(base_interval)
            };

            tracing::debug!("Next refresh in {:?}", interval);

            tokio::select! {
                _ = tokio::time::sleep(interval) => {
                    self.refresh_display().await;
                }
                _ = shutdown.recv() => {
                    tracing::info!("Scheduler shutting down");
                    break;
                }
            }
        }
    }

    /// Calculate effective interval with backoff applied
    fn get_effective_interval(&self, base_interval: Duration) -> Duration {
        let failures = self.consecutive_failures.load(Ordering::Relaxed);

        if failures >= Self::MAX_CONSECUTIVE_FAILURES {
            // Apply exponential backoff: interval * 2^(failures - threshold + 1)
            // Cap the exponent to avoid overflow
            let exponent = (failures - Self::MAX_CONSECUTIVE_FAILURES + 1).min(6);
            let multiplier = Self::FAILURE_BACKOFF_MULTIPLIER.pow(exponent);

            let backoff_secs = base_interval
                .as_secs()
                .saturating_mul(multiplier)
                .min(Self::MAX_BACKOFF_SECS);

            let backoff = Duration::from_secs(backoff_secs);

            tracing::warn!(
                "Applying backoff due to {} consecutive failures: {:?} -> {:?}",
                failures,
                base_interval,
                backoff
            );

            backoff
        } else {
            base_interval
        }
    }

    /// Perform a display refresh with failure tracking
    async fn refresh_display(&self) {
        let config = self.config.read().await;

        if !config.has_image_url() {
            tracing::debug!("No image URL configured, skipping refresh");
            return;
        }

        tracing::info!("Scheduled refresh starting...");

        match self.processor.process_and_display(&config).await {
            Ok(_) => {
                let prev_failures = self.consecutive_failures.swap(0, Ordering::Relaxed);
                if prev_failures > 0 {
                    tracing::info!(
                        "Scheduled refresh succeeded after {} previous failures",
                        prev_failures
                    );
                } else {
                    tracing::info!("Scheduled refresh completed successfully");
                }
            }
            Err(e) => {
                let failures = self.consecutive_failures.fetch_add(1, Ordering::Relaxed) + 1;
                tracing::error!(
                    "Scheduled refresh failed ({}/{} before backoff): {}",
                    failures,
                    Self::MAX_CONSECUTIVE_FAILURES,
                    e
                );
            }
        }
    }
}

/// Scheduler with manual trigger support
#[allow(dead_code)]
pub struct SchedulerWithTrigger {
    inner: Scheduler,
    trigger_rx: tokio::sync::mpsc::Receiver<()>,
}

#[allow(dead_code)]
impl SchedulerWithTrigger {
    /// Create a new scheduler with manual trigger
    pub fn new(
        config: Arc<RwLock<Config>>,
        processor: Arc<ImageProcessor>,
    ) -> (Self, tokio::sync::mpsc::Sender<()>) {
        let (trigger_tx, trigger_rx) = tokio::sync::mpsc::channel(1);
        let inner = Scheduler::new(config, processor);
        (Self { inner, trigger_rx }, trigger_tx)
    }

    /// Run the scheduler with manual trigger support
    ///
    /// Includes exponential backoff from the inner scheduler.
    pub async fn run(mut self, mut shutdown: broadcast::Receiver<()>) {
        tracing::info!("Scheduler with trigger started");

        // Initial delay
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(10)) => {}
            _ = shutdown.recv() => return,
        }

        // Initial refresh
        self.inner.refresh_display().await;

        loop {
            // Get effective interval based on day and time (with backoff applied)
            let interval = {
                let config = self.inner.config.read().await;
                let current_interval = config.get_current_interval();
                let base_interval = Duration::from_secs(current_interval as u64 * 60);
                self.inner.get_effective_interval(base_interval)
            };

            tokio::select! {
                _ = tokio::time::sleep(interval) => {
                    self.inner.refresh_display().await;
                }
                Some(_) = self.trigger_rx.recv() => {
                    tracing::info!("Manual refresh triggered");
                    self.inner.refresh_display().await;
                }
                _ = shutdown.recv() => {
                    tracing::info!("Scheduler shutting down");
                    break;
                }
            }
        }
    }
}

