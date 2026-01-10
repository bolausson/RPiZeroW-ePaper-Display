//! HTTP route handlers for the web interface.

use super::templates;
use crate::config::Config;
use crate::image_proc::ImageProcessor;
use axum::{
    extract::{Form, Path, State},
    http::StatusCode,
    response::{Html, IntoResponse},
};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<RwLock<Config>>,
    pub processor: Arc<ImageProcessor>,
    pub config_path: String,
}

/// Form data for configuration update
#[derive(Debug, Deserialize)]
pub struct ConfigForm {
    pub image_url: String,
    pub refresh_interval_min: u32,
    #[serde(default = "default_display_width")]
    pub display_width: u32,
    #[serde(default = "default_display_height")]
    pub display_height: u32,
    pub rotation: u16,
    #[serde(default)]
    pub rotate_first: Option<String>,
    #[serde(default)]
    pub mirror_h: Option<String>,
    #[serde(default)]
    pub mirror_v: Option<String>,
    #[serde(default)]
    pub scale_to_fit: Option<String>,
}

fn default_display_width() -> u32 {
    800
}

fn default_display_height() -> u32 {
    480
}

/// GET / - Main configuration page
pub async fn index(State(state): State<AppState>) -> Html<String> {
    let config = state.config.read().await;
    Html(templates::render_config_page(&config, None))
}

/// POST /save - Save configuration
pub async fn save_config(
    State(state): State<AppState>,
    Form(form): Form<ConfigForm>,
) -> impl IntoResponse {
    match update_config(&state, &form).await {
        Ok(_) => {
            let config = state.config.read().await;
            Html(templates::render_config_page(&config, Some("Configuration saved!")))
        }
        Err(e) => {
            let config = state.config.read().await;
            Html(templates::render_config_page(
                &config,
                Some(&format!("Error: {}", e)),
            ))
        }
    }
}

/// POST /apply - Save configuration and refresh display
pub async fn save_and_apply(
    State(state): State<AppState>,
    Form(form): Form<ConfigForm>,
) -> impl IntoResponse {
    // Save config first
    if let Err(e) = update_config(&state, &form).await {
        let config = state.config.read().await;
        return Html(templates::render_config_page(
            &config,
            Some(&format!("Error saving: {}", e)),
        ));
    }

    // Apply to display
    let config = state.config.read().await;
    match state.processor.process_and_display(&config).await {
        Ok(_) => Html(templates::render_config_page(
            &config,
            Some("Configuration saved and applied!"),
        )),
        Err(e) => Html(templates::render_config_page(
            &config,
            Some(&format!("Saved, but display error: {}", e)),
        )),
    }
}

/// GET /action/:action - Display actions
pub async fn display_action(
    State(state): State<AppState>,
    Path(action): Path<String>,
) -> impl IntoResponse {
    let result = match action.as_str() {
        "show" => {
            let config = state.config.read().await;
            state.processor.process_and_display(&config).await
        }
        "test" => state.processor.show_test_pattern().await,
        "clear" => state.processor.clear_display().await,
        _ => {
            return (
                StatusCode::NOT_FOUND,
                Html(templates::render_message_page("Not Found", "Unknown action", true)),
            );
        }
    };

    match result {
        Ok(_) => (
            StatusCode::OK,
            Html(templates::render_message_page(
                "Success",
                &format!("Action '{}' completed successfully!", action),
                true,
            )),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(templates::render_message_page(
                "Error",
                &format!("Action failed: {}", e),
                true,
            )),
        ),
    }
}

/// Health check endpoint
pub async fn health() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

/// Update configuration from form data
async fn update_config(state: &AppState, form: &ConfigForm) -> Result<(), String> {
    let mut config = state.config.write().await;

    config.image_url = form.image_url.clone();
    config.refresh_interval_min = form.refresh_interval_min;
    config.display_width = form.display_width;
    config.display_height = form.display_height;
    config.rotation = form.rotation;
    // rotate_first: "1" = true, "0" = false, parse the value
    config.rotate_first = form
        .rotate_first
        .as_ref()
        .map(|v| v == "1")
        .unwrap_or(true);
    config.mirror_h = form.mirror_h.is_some();
    config.mirror_v = form.mirror_v.is_some();
    config.scale_to_fit = form.scale_to_fit.is_some();

    // Validate
    config.validate().map_err(|e| e.to_string())?;

    // Save to file
    config.save(&state.config_path).map_err(|e| e.to_string())?;

    tracing::info!("Configuration saved to {}", state.config_path);
    Ok(())
}

