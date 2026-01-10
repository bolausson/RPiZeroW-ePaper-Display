//! Configuration management for the ePaper Display Server.
//!
//! Handles loading, saving, and validating configuration from JSON files.

use chrono::Timelike;
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

/// A time-based refresh schedule period
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SchedulePeriod {
    /// Start time in HH:MM format (24-hour)
    pub start_time: String,
    /// End time in HH:MM format (24-hour)
    pub end_time: String,
    /// Refresh interval in minutes for this period
    pub interval_min: u32,
}

impl SchedulePeriod {
    /// Create a new schedule period
    pub fn new(start_time: &str, end_time: &str, interval_min: u32) -> Self {
        Self {
            start_time: start_time.to_string(),
            end_time: end_time.to_string(),
            interval_min,
        }
    }

    /// Parse time string to minutes since midnight
    pub fn parse_time(time_str: &str) -> Result<u32, ConfigError> {
        let parts: Vec<&str> = time_str.split(':').collect();
        if parts.len() != 2 {
            return Err(ConfigError::ValidationError(format!(
                "Invalid time format '{}', expected HH:MM",
                time_str
            )));
        }

        let hours: u32 = parts[0].parse().map_err(|_| {
            ConfigError::ValidationError(format!("Invalid hour in time '{}'", time_str))
        })?;
        let minutes: u32 = parts[1].parse().map_err(|_| {
            ConfigError::ValidationError(format!("Invalid minutes in time '{}'", time_str))
        })?;

        if hours >= 24 || minutes >= 60 {
            return Err(ConfigError::ValidationError(format!(
                "Time '{}' out of range (00:00-23:59)",
                time_str
            )));
        }

        Ok(hours * 60 + minutes)
    }

    /// Get start time as minutes since midnight
    pub fn start_minutes(&self) -> Result<u32, ConfigError> {
        Self::parse_time(&self.start_time)
    }

    /// Get end time as minutes since midnight
    pub fn end_minutes(&self) -> Result<u32, ConfigError> {
        Self::parse_time(&self.end_time)
    }

    /// Check if this period spans midnight
    pub fn spans_midnight(&self) -> Result<bool, ConfigError> {
        let start = self.start_minutes()?;
        let end = self.end_minutes()?;
        Ok(end <= start)
    }

    /// Check if a given time (minutes since midnight) falls within this period
    pub fn contains_time(&self, time_minutes: u32) -> Result<bool, ConfigError> {
        let start = self.start_minutes()?;
        let end = self.end_minutes()?;

        if self.spans_midnight()? {
            // Period spans midnight: e.g., 23:00 to 06:00
            Ok(time_minutes >= start || time_minutes < end)
        } else {
            // Normal period: e.g., 06:00 to 18:00
            Ok(time_minutes >= start && time_minutes < end)
        }
    }

    /// Validate this schedule period
    pub fn validate(&self) -> Result<(), ConfigError> {
        self.start_minutes()?;
        self.end_minutes()?;

        if self.interval_min < 1 || self.interval_min > 1440 {
            return Err(ConfigError::ValidationError(format!(
                "Interval {} must be between 1 and 1440 minutes",
                self.interval_min
            )));
        }

        Ok(())
    }
}

/// Default schedule: single period covering 24 hours with 60-minute refresh
fn default_schedule() -> Vec<SchedulePeriod> {
    vec![SchedulePeriod::new("00:00", "00:00", 60)]
}

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// URL of the image to display
    #[serde(default)]
    pub image_url: String,

    /// Legacy: Refresh interval in minutes (for backward compatibility)
    /// Will be migrated to schedule on load
    #[serde(default, skip_serializing)]
    pub refresh_interval_min: Option<u32>,

    /// Time-based refresh schedule
    #[serde(default = "default_schedule")]
    pub schedule: Vec<SchedulePeriod>,

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
            refresh_interval_min: None,
            schedule: default_schedule(),
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
        let mut config: Config = serde_json::from_str(&content)?;

        // Migrate legacy refresh_interval_min to schedule if needed
        config.migrate_legacy_interval();

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

    /// Save configuration to a JSON file atomically
    ///
    /// Uses a write-to-temp-then-rename pattern to prevent corruption
    /// if power is lost during the write operation. This is critical
    /// for reliability on embedded devices without UPS.
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), ConfigError> {
        let path = path.as_ref();
        let content = serde_json::to_string_pretty(self)?;

        // Write to temporary file first
        let tmp_path = path.with_extension("json.tmp");
        std::fs::write(&tmp_path, &content)?;

        // Atomic rename - either fully succeeds or fails, never partial
        std::fs::rename(&tmp_path, path).map_err(|e| {
            // Clean up temp file on rename failure
            let _ = std::fs::remove_file(&tmp_path);
            ConfigError::ReadError(e)
        })?;

        Ok(())
    }

    /// Save configuration to default path
    #[allow(dead_code)]
    pub fn save_default(&self) -> Result<(), ConfigError> {
        self.save(DEFAULT_CONFIG_PATH)
    }

    /// Migrate legacy refresh_interval_min to new schedule format
    fn migrate_legacy_interval(&mut self) {
        if let Some(interval) = self.refresh_interval_min.take() {
            // Only migrate if schedule is empty or default
            if self.schedule.is_empty()
                || (self.schedule.len() == 1
                    && self.schedule[0].start_time == "00:00"
                    && self.schedule[0].end_time == "00:00"
                    && self.schedule[0].interval_min == 60)
            {
                tracing::info!(
                    "Migrating legacy refresh_interval_min ({}) to schedule",
                    interval
                );
                self.schedule = vec![SchedulePeriod::new("00:00", "00:00", interval)];
            }
        }
    }

    /// Validate configuration values
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate schedule
        if self.schedule.is_empty() {
            return Err(ConfigError::ValidationError(
                "Schedule must have at least one period".to_string(),
            ));
        }

        for (i, period) in self.schedule.iter().enumerate() {
            period.validate().map_err(|e| {
                ConfigError::ValidationError(format!("Schedule period {}: {}", i + 1, e))
            })?;
        }

        // Validate schedule coverage (all 24 hours covered)
        self.validate_schedule_coverage()?;

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

    /// Validate that the schedule covers all 24 hours without gaps
    fn validate_schedule_coverage(&self) -> Result<(), ConfigError> {
        // Special case: single period from 00:00 to 00:00 covers full day
        if self.schedule.len() == 1
            && self.schedule[0].start_time == "00:00"
            && self.schedule[0].end_time == "00:00"
        {
            return Ok(());
        }

        // Check each minute of the day is covered by exactly one period
        let mut coverage = vec![false; 1440];

        for period in &self.schedule {
            let start = period.start_minutes()?;
            let end = period.end_minutes()?;

            if period.spans_midnight()? {
                // Period spans midnight
                for minute in start..1440 {
                    if coverage[minute as usize] {
                        return Err(ConfigError::ValidationError(format!(
                            "Overlapping schedule at {:02}:{:02}",
                            minute / 60,
                            minute % 60
                        )));
                    }
                    coverage[minute as usize] = true;
                }
                for minute in 0..end {
                    if coverage[minute as usize] {
                        return Err(ConfigError::ValidationError(format!(
                            "Overlapping schedule at {:02}:{:02}",
                            minute / 60,
                            minute % 60
                        )));
                    }
                    coverage[minute as usize] = true;
                }
            } else {
                // Normal period
                for minute in start..end {
                    if coverage[minute as usize] {
                        return Err(ConfigError::ValidationError(format!(
                            "Overlapping schedule at {:02}:{:02}",
                            minute / 60,
                            minute % 60
                        )));
                    }
                    coverage[minute as usize] = true;
                }
            }
        }

        // Check for gaps
        for (minute, &covered) in coverage.iter().enumerate() {
            if !covered {
                return Err(ConfigError::ValidationError(format!(
                    "Schedule gap at {:02}:{:02}",
                    minute / 60,
                    minute % 60
                )));
            }
        }

        Ok(())
    }

    /// Get the current refresh interval based on time of day
    pub fn get_current_interval(&self) -> u32 {
        let now = chrono::Local::now();
        let current_minutes = now.hour() * 60 + now.minute();

        self.get_interval_for_time(current_minutes)
    }

    /// Get the refresh interval for a specific time (minutes since midnight)
    pub fn get_interval_for_time(&self, time_minutes: u32) -> u32 {
        for period in &self.schedule {
            if let Ok(true) = period.contains_time(time_minutes) {
                return period.interval_min;
            }
        }

        // Fallback to first period's interval (should never happen with valid config)
        self.schedule
            .first()
            .map(|p| p.interval_min)
            .unwrap_or(60)
    }

    /// Get the currently active schedule period
    pub fn get_current_period(&self) -> Option<&SchedulePeriod> {
        let now = chrono::Local::now();
        let current_minutes = now.hour() * 60 + now.minute();

        for period in &self.schedule {
            if let Ok(true) = period.contains_time(current_minutes) {
                return Some(period);
            }
        }

        self.schedule.first()
    }

    /// Check if an image URL is configured
    pub fn has_image_url(&self) -> bool {
        !self.image_url.trim().is_empty()
    }
}
