//! Web server module for the configuration UI.
//!
//! Provides an HTTP server using Axum for the configuration web interface.

pub mod routes;
pub mod templates;

use crate::config::Config;
use crate::display::DisplayController;
use crate::image_proc::ImageProcessor;
use axum::{routing::get, Router};
use routes::AppState;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use thiserror::Error;

/// Web server errors
#[derive(Error, Debug)]
pub enum WebError {
    #[error("Failed to bind to address: {0}")]
    BindError(#[from] std::io::Error),

    #[error("Server error: {0}")]
    ServerError(String),
}

/// Web server configuration
pub struct WebServer {
    config: Arc<RwLock<Config>>,
    processor: Arc<ImageProcessor>,
    config_path: String,
}

impl WebServer {
    /// Create a new web server
    pub fn new(config: Config, display: DisplayController, config_path: String) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            processor: Arc::new(ImageProcessor::new(display)),
            config_path,
        }
    }

    /// Get shared config reference for scheduler
    pub fn config(&self) -> Arc<RwLock<Config>> {
        Arc::clone(&self.config)
    }

    /// Get shared processor reference for scheduler
    pub fn processor(&self) -> Arc<ImageProcessor> {
        Arc::clone(&self.processor)
    }

    /// Build the router with all routes
    fn build_router(&self) -> Router {
        let state = AppState {
            config: Arc::clone(&self.config),
            processor: Arc::clone(&self.processor),
            config_path: self.config_path.clone(),
        };

        Router::new()
            .route("/", get(routes::index))
            .route("/save", axum::routing::post(routes::save_config))
            .route("/apply", axum::routing::post(routes::save_and_apply))
            .route("/action/{action}", get(routes::display_action))
            .route("/health", get(routes::health))
            .with_state(state)
    }

    /// Run the web server
    #[allow(dead_code)]
    pub async fn run(&self, port: u16) -> Result<(), WebError> {
        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        let listener = TcpListener::bind(addr).await?;

        tracing::info!("Web server listening on http://{}", addr);

        axum::serve(listener, self.build_router())
            .await
            .map_err(|e| WebError::ServerError(e.to_string()))
    }

    /// Run the web server with graceful shutdown
    pub async fn run_with_shutdown(
        &self,
        port: u16,
        shutdown: tokio::sync::broadcast::Receiver<()>,
    ) -> Result<(), WebError> {
        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        let listener = TcpListener::bind(addr).await?;

        tracing::info!("Web server listening on http://{}", addr);

        let mut shutdown = shutdown;
        axum::serve(listener, self.build_router())
            .with_graceful_shutdown(async move {
                let _ = shutdown.recv().await;
                tracing::info!("Web server shutting down gracefully");
            })
            .await
            .map_err(|e| WebError::ServerError(e.to_string()))
    }
}

