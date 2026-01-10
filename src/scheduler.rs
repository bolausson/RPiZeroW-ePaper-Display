//! Refresh scheduler for periodic display updates.
//!
//! Manages automatic refresh of the display at configurable intervals.

use crate::config::Config;
use crate::image_proc::ImageProcessor;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, RwLock};

/// Scheduler for periodic display refresh
pub struct Scheduler {
    config: Arc<RwLock<Config>>,
    processor: Arc<ImageProcessor>,
}

impl Scheduler {
    /// Create a new scheduler
    pub fn new(config: Arc<RwLock<Config>>, processor: Arc<ImageProcessor>) -> Self {
        Self { config, processor }
    }

    /// Run the scheduler loop
    ///
    /// Periodically refreshes the display based on the configured interval.
    /// Listens for shutdown signal to gracefully stop.
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
            // Get current interval from config
            let interval = {
                let config = self.config.read().await;
                Duration::from_secs(config.refresh_interval_min as u64 * 60)
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

    /// Perform a display refresh
    async fn refresh_display(&self) {
        let config = self.config.read().await;

        if !config.has_image_url() {
            tracing::debug!("No image URL configured, skipping refresh");
            return;
        }

        tracing::info!("Scheduled refresh starting...");

        match self.processor.process_and_display(&config).await {
            Ok(_) => tracing::info!("Scheduled refresh completed successfully"),
            Err(e) => tracing::error!("Scheduled refresh failed: {}", e),
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
            let interval = {
                let config = self.inner.config.read().await;
                Duration::from_secs(config.refresh_interval_min as u64 * 60)
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

