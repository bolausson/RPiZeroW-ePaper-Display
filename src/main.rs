//! E-Paper Display Server for Raspberry Pi Zero W
//!
//! A Rust-based server that:
//! - Fetches images from a configurable URL
//! - Processes and dithers them for 7-color e-paper display
//! - Provides a web interface for configuration
//! - Runs as a systemd service with graceful shutdown

mod config;
mod display;
mod image_proc;
mod scheduler;
mod web;

use clap::Parser;
use config::Config;
use display::DisplayController;
use scheduler::Scheduler;
use tokio::sync::broadcast;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Command line arguments
#[derive(Parser, Debug)]
#[command(name = "epaper-display")]
#[command(about = "E-Paper Display Server for Raspberry Pi Zero W")]
#[command(version)]
struct Args {
    /// Configuration file path
    #[arg(short, long, default_value = "/opt/epaper-display/config.json")]
    config: String,

    /// Web server port (overrides config, default: 8888)
    #[arg(long = "http-port")]
    http_port: Option<u16>,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Show test pattern and exit
    #[arg(long)]
    test: bool,

    /// Clear display and exit
    #[arg(long)]
    clear: bool,
}

/// Using current_thread runtime for single-core Pi Zero W
/// This reduces memory overhead and avoids thread synchronization costs
#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Initialize logging
    init_logging(args.verbose);

    tracing::info!("Starting E-Paper Display Server");

    // Load configuration
    let config = Config::load(&args.config).unwrap_or_else(|e| {
        tracing::warn!("Failed to load config from {}: {}", args.config, e);
        tracing::info!("Using default configuration");
        Config::default()
    });

    // Initialize display controller
    let display = DisplayController::new();

    // Handle one-shot commands
    if args.test {
        tracing::info!("Running test pattern...");
        display.test_pattern().await?;
        tracing::info!("Test pattern complete");
        return Ok(());
    }

    if args.clear {
        tracing::info!("Clearing display...");
        display.init().await?;
        display.clear().await?;
        display.sleep().await?;
        tracing::info!("Display cleared");
        return Ok(());
    }

    // Setup shutdown signal handling
    let (shutdown_tx, _) = broadcast::channel::<()>(1);

    // Create web server
    let port = args.http_port.unwrap_or(config.web_port);
    let web_server = web::WebServer::new(config, display, args.config.clone());

    // Create scheduler
    let scheduler = Scheduler::new(web_server.config(), web_server.processor());

    // Spawn scheduler task
    let scheduler_shutdown = shutdown_tx.subscribe();
    let scheduler_handle = tokio::spawn(async move {
        scheduler.run(scheduler_shutdown).await;
    });

    // Spawn web server task
    let web_shutdown = shutdown_tx.subscribe();
    let web_handle = tokio::spawn(async move {
        if let Err(e) = web_server.run_with_shutdown(port, web_shutdown).await {
            tracing::error!("Web server error: {}", e);
        }
    });

    // Wait for shutdown signal
    wait_for_shutdown().await;
    tracing::info!("Shutdown signal received");

    // Send shutdown to all tasks
    let _ = shutdown_tx.send(());

    // Wait for tasks to complete with timeout
    tokio::select! {
        _ = scheduler_handle => {},
        _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
            tracing::warn!("Scheduler shutdown timeout");
        }
    }

    tokio::select! {
        _ = web_handle => {},
        _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
            tracing::warn!("Web server shutdown timeout");
        }
    }

    tracing::info!("Shutdown complete");
    Ok(())
}

/// Initialize tracing/logging
///
/// Default level is "warn" to minimize SD card wear from log writes.
/// Use --verbose flag for "debug" level during development/troubleshooting.
fn init_logging(verbose: bool) {
    let level = if verbose { "debug" } else { "warn" };

    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| format!("rpizerow_epaper_display={}", level).into());

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();
}

/// Wait for shutdown signals (SIGTERM, SIGINT)
async fn wait_for_shutdown() {
    use tokio::signal::unix::{signal, SignalKind};

    let mut sigterm = signal(SignalKind::terminate()).expect("Failed to setup SIGTERM handler");
    let mut sigint = signal(SignalKind::interrupt()).expect("Failed to setup SIGINT handler");

    tokio::select! {
        _ = sigterm.recv() => {
            tracing::info!("Received SIGTERM");
        }
        _ = sigint.recv() => {
            tracing::info!("Received SIGINT");
        }
    }
}
