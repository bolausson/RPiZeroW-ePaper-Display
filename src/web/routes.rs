//! HTTP route handlers for the web interface.

use super::templates;
use crate::config::{Config, SchedulePeriod};
use crate::image_proc::ImageProcessor;
use axum::{
    extract::{Form, Path, State},
    http::StatusCode,
    response::{Html, IntoResponse},
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<RwLock<Config>>,
    pub processor: Arc<ImageProcessor>,
    pub config_path: String,
}

/// Form data is captured as a HashMap to handle dynamic schedule fields
type FormData = HashMap<String, String>;

fn default_display_width() -> u32 {
    800
}

fn default_display_height() -> u32 {
    480
}

/// Parse schedule periods from form data
fn parse_schedule_from_form(form: &FormData) -> Result<Vec<SchedulePeriod>, String> {
    let mut periods = Vec::new();

    // Find all schedule indices by looking for schedule_start_* fields
    let mut indices: Vec<usize> = form
        .keys()
        .filter_map(|k| {
            k.strip_prefix("schedule_start_")
                .and_then(|s| s.parse::<usize>().ok())
        })
        .collect();
    indices.sort();

    for i in &indices {
        let start = form
            .get(&format!("schedule_start_{}", i))
            .ok_or_else(|| format!("Missing start time for period {}", i))?;
        let end = form
            .get(&format!("schedule_end_{}", i))
            .ok_or_else(|| format!("Missing end time for period {}", i))?;
        let interval_str = form
            .get(&format!("schedule_interval_{}", i))
            .ok_or_else(|| format!("Missing interval for period {}", i))?;

        let interval: u32 = interval_str
            .parse()
            .map_err(|_| format!("Invalid interval '{}' for period {}", interval_str, i))?;

        periods.push(SchedulePeriod::new(start, end, interval));
    }

    if periods.is_empty() {
        return Err("At least one schedule period is required".to_string());
    }

    Ok(periods)
}

/// GET / - Main configuration page
pub async fn index(State(state): State<AppState>) -> Html<String> {
    let config = state.config.read().await;
    Html(templates::render_config_page(&config, None))
}

/// POST /save - Save configuration
pub async fn save_config(
    State(state): State<AppState>,
    Form(form): Form<FormData>,
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
    Form(form): Form<FormData>,
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

/// Helper to get a form field with a default value
fn get_form_field<'a>(form: &'a FormData, key: &str, default: &'a str) -> &'a str {
    form.get(key).map(|s| s.as_str()).unwrap_or(default)
}

/// Helper to parse a form field as a number
fn parse_form_field<T: std::str::FromStr>(form: &FormData, key: &str, default: T) -> T {
    form.get(key)
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

/// Update configuration from form data
async fn update_config(state: &AppState, form: &FormData) -> Result<(), String> {
    let mut config = state.config.write().await;

    // Parse basic fields
    config.image_url = get_form_field(form, "image_url", "").to_string();
    config.display_width = parse_form_field(form, "display_width", default_display_width());
    config.display_height = parse_form_field(form, "display_height", default_display_height());
    config.rotation = parse_form_field(form, "rotation", 0);

    // rotate_first: "1" = true, "0" = false
    config.rotate_first = get_form_field(form, "rotate_first", "1") == "1";

    // Checkboxes: present = checked
    config.mirror_h = form.contains_key("mirror_h");
    config.mirror_v = form.contains_key("mirror_v");
    config.scale_to_fit = form.contains_key("scale_to_fit");

    // Parse schedule periods
    config.schedule = parse_schedule_from_form(form)?;

    // Validate
    config.validate().map_err(|e| e.to_string())?;

    // Save to file
    config.save(&state.config_path).map_err(|e| e.to_string())?;

    tracing::info!("Configuration saved to {}", state.config_path);
    Ok(())
}
