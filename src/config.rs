//! Configuration management for the ePaper Display Server.
//!
//! Handles loading, saving, and validating configuration from JSON files.

use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

/// Default configuration file path
#[allow(dead_code)]
pub const DEFAULT_CONFIG_PATH: &str = "/opt/epaper-display/config.json";

/// Configuration errors
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    ReadError(#[from] std::io::Error),

    #[error("Failed to parse config JSON: {0}")]
    ParseError(#[from] serde_json::Error),

    #[error("Invalid configuration: {0}")]
    ValidationError(String),
}

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// URL of the image to display
    #[serde(default)]
    pub image_url: String,

    /// Refresh interval in minutes (1-1440)
    #[serde(default = "default_refresh_interval")]
    pub refresh_interval_min: u32,

    /// Display rotation in degrees (0, 90, 180, 270)
    #[serde(default)]
    pub rotation: u16,

    /// Horizontal mirror
    #[serde(default)]
    pub mirror_h: bool,

    /// Vertical mirror
    #[serde(default)]
    pub mirror_v: bool,

    /// Scale image to fit display
    #[serde(default = "default_true")]
    pub scale_to_fit: bool,

    /// Apply rotation before mirroring (true) or mirror before rotating (false)
    #[serde(default = "default_true")]
    pub rotate_first: bool,

    /// Display width in pixels
    #[serde(default = "default_display_width")]
    pub display_width: u32,

    /// Display height in pixels
    #[serde(default = "default_display_height")]
    pub display_height: u32,

    /// Web server port
    #[serde(default = "default_web_port")]
    pub web_port: u16,

    /// Enable verbose logging
    #[serde(default)]
    pub verbose: bool,
}

fn default_refresh_interval() -> u32 {
    60
}

fn default_web_port() -> u16 {
    8888
}

fn default_true() -> bool {
    true
}

fn default_display_width() -> u32 {
    800
}

fn default_display_height() -> u32 {
    480
}

impl Default for Config {
    fn default() -> Self {
        Self {
            image_url: String::new(),
            refresh_interval_min: default_refresh_interval(),
            rotation: 0,
            mirror_h: false,
            mirror_v: false,
            scale_to_fit: true,
            rotate_first: true,
            display_width: default_display_width(),
            display_height: default_display_height(),
            web_port: default_web_port(),
            verbose: false,
        }
    }
}

impl Config {
    /// Load configuration from a JSON file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = serde_json::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    /// Load configuration from default path, or return default config if not found
    #[allow(dead_code)]
    pub fn load_or_default() -> Self {
        Self::load(DEFAULT_CONFIG_PATH).unwrap_or_else(|e| {
            tracing::warn!("Failed to load config: {}, using defaults", e);
            Self::default()
        })
    }

    /// Save configuration to a JSON file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), ConfigError> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Save configuration to default path
    #[allow(dead_code)]
    pub fn save_default(&self) -> Result<(), ConfigError> {
        self.save(DEFAULT_CONFIG_PATH)
    }

    /// Validate configuration values
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.refresh_interval_min < 1 || self.refresh_interval_min > 1440 {
            return Err(ConfigError::ValidationError(
                "refresh_interval_min must be between 1 and 1440".to_string(),
            ));
        }

        if !matches!(self.rotation, 0 | 90 | 180 | 270) {
            return Err(ConfigError::ValidationError(
                "rotation must be 0, 90, 180, or 270".to_string(),
            ));
        }

        if self.web_port == 0 {
            return Err(ConfigError::ValidationError(
                "web_port must be greater than 0".to_string(),
            ));
        }

        if self.display_width < 100 || self.display_width > 2000 {
            return Err(ConfigError::ValidationError(
                "display_width must be between 100 and 2000".to_string(),
            ));
        }

        if self.display_height < 100 || self.display_height > 2000 {
            return Err(ConfigError::ValidationError(
                "display_height must be between 100 and 2000".to_string(),
            ));
        }

        Ok(())
    }

    /// Check if an image URL is configured
    pub fn has_image_url(&self) -> bool {
        !self.image_url.trim().is_empty()
    }
}

