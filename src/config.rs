//! Configuration management for the ePaper Display Server.
//!
//! Handles loading, saving, and validating configuration from JSON files.

use chrono::{Datelike, Timelike};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

/// Default configuration file path
#[allow(dead_code)]
pub const DEFAULT_CONFIG_PATH: &str = "/opt/epaper-display/config.json";

/// Type alias for day-of-week to schedule plan name mapping
pub type DayAssignments = HashMap<Weekday, String>;

/// Days of the week
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl Weekday {
    /// Get all weekdays in order
    pub fn all() -> &'static [Weekday] {
        &[
            Weekday::Monday,
            Weekday::Tuesday,
            Weekday::Wednesday,
            Weekday::Thursday,
            Weekday::Friday,
            Weekday::Saturday,
            Weekday::Sunday,
        ]
    }

    /// Get display name for the weekday
    pub fn display_name(&self) -> &'static str {
        match self {
            Weekday::Monday => "Monday",
            Weekday::Tuesday => "Tuesday",
            Weekday::Wednesday => "Wednesday",
            Weekday::Thursday => "Thursday",
            Weekday::Friday => "Friday",
            Weekday::Saturday => "Saturday",
            Weekday::Sunday => "Sunday",
        }
    }

    /// Get short name for the weekday
    pub fn short_name(&self) -> &'static str {
        match self {
            Weekday::Monday => "Mon",
            Weekday::Tuesday => "Tue",
            Weekday::Wednesday => "Wed",
            Weekday::Thursday => "Thu",
            Weekday::Friday => "Fri",
            Weekday::Saturday => "Sat",
            Weekday::Sunday => "Sun",
        }
    }

    /// Convert from chrono::Weekday
    pub fn from_chrono(wd: chrono::Weekday) -> Self {
        match wd {
            chrono::Weekday::Mon => Weekday::Monday,
            chrono::Weekday::Tue => Weekday::Tuesday,
            chrono::Weekday::Wed => Weekday::Wednesday,
            chrono::Weekday::Thu => Weekday::Thursday,
            chrono::Weekday::Fri => Weekday::Friday,
            chrono::Weekday::Sat => Weekday::Saturday,
            chrono::Weekday::Sun => Weekday::Sunday,
        }
    }
}

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

/// A named schedule plan containing multiple time periods
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SchedulePlan {
    /// Name of the schedule plan (e.g., "Weekday", "Weekend")
    pub name: String,
    /// Time periods within this plan
    pub periods: Vec<SchedulePeriod>,
}

impl SchedulePlan {
    /// Create a new schedule plan
    pub fn new(name: &str, periods: Vec<SchedulePeriod>) -> Self {
        Self {
            name: name.to_string(),
            periods,
        }
    }

    /// Create a default schedule plan
    pub fn default_plan() -> Self {
        Self {
            name: "Default".to_string(),
            periods: vec![SchedulePeriod::new("00:00", "00:00", 60)],
        }
    }

    /// Validate this schedule plan
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.name.trim().is_empty() {
            return Err(ConfigError::ValidationError(
                "Schedule plan name cannot be empty".to_string(),
            ));
        }

        if self.periods.is_empty() {
            return Err(ConfigError::ValidationError(format!(
                "Schedule plan '{}' must have at least one period",
                self.name
            )));
        }

        for (i, period) in self.periods.iter().enumerate() {
            period.validate().map_err(|e| {
                ConfigError::ValidationError(format!(
                    "Plan '{}' period {}: {}",
                    self.name,
                    i + 1,
                    e
                ))
            })?;
        }

        // Validate coverage for this plan
        self.validate_coverage()?;

        Ok(())
    }

    /// Validate that this plan's periods cover all 24 hours
    fn validate_coverage(&self) -> Result<(), ConfigError> {
        // Special case: single period from 00:00 to 00:00 covers full day
        if self.periods.len() == 1
            && self.periods[0].start_time == "00:00"
            && self.periods[0].end_time == "00:00"
        {
            return Ok(());
        }

        // Check each minute of the day is covered by exactly one period
        let mut coverage = vec![false; 1440];

        for period in &self.periods {
            let start = period.start_minutes()?;
            let end = period.end_minutes()?;

            if period.spans_midnight()? {
                for minute in start..1440 {
                    if coverage[minute as usize] {
                        return Err(ConfigError::ValidationError(format!(
                            "Plan '{}': Overlapping schedule at {:02}:{:02}",
                            self.name,
                            minute / 60,
                            minute % 60
                        )));
                    }
                    coverage[minute as usize] = true;
                }
                for minute in 0..end {
                    if coverage[minute as usize] {
                        return Err(ConfigError::ValidationError(format!(
                            "Plan '{}': Overlapping schedule at {:02}:{:02}",
                            self.name,
                            minute / 60,
                            minute % 60
                        )));
                    }
                    coverage[minute as usize] = true;
                }
            } else {
                for minute in start..end {
                    if coverage[minute as usize] {
                        return Err(ConfigError::ValidationError(format!(
                            "Plan '{}': Overlapping schedule at {:02}:{:02}",
                            self.name,
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
                    "Plan '{}': Schedule gap at {:02}:{:02}",
                    self.name,
                    minute / 60,
                    minute % 60
                )));
            }
        }

        Ok(())
    }

    /// Get the interval for a specific time (minutes since midnight)
    pub fn get_interval_for_time(&self, time_minutes: u32) -> u32 {
        for period in &self.periods {
            if let Ok(true) = period.contains_time(time_minutes) {
                return period.interval_min;
            }
        }
        self.periods.first().map(|p| p.interval_min).unwrap_or(60)
    }

    /// Get the active period for a specific time
    pub fn get_period_for_time(&self, time_minutes: u32) -> Option<&SchedulePeriod> {
        for period in &self.periods {
            if let Ok(true) = period.contains_time(time_minutes) {
                return Some(period);
            }
        }
        self.periods.first()
    }
}

/// Default schedule plans
fn default_schedule_plans() -> Vec<SchedulePlan> {
    vec![SchedulePlan::default_plan()]
}

/// Default day assignments (all days use "Default" plan)
fn default_day_assignments() -> DayAssignments {
    let mut map = DayAssignments::new();
    for day in Weekday::all() {
        map.insert(*day, "Default".to_string());
    }
    map
}

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// URL of the image to display
    #[serde(default)]
    pub image_url: String,

    /// Legacy: Refresh interval in minutes (for backward compatibility)
    /// Will be migrated to schedule_plans on load
    #[serde(default, skip_serializing)]
    pub refresh_interval_min: Option<u32>,

    /// Legacy: Single schedule array (for backward compatibility)
    /// Will be migrated to schedule_plans on load
    #[serde(default, skip_serializing)]
    pub schedule: Option<Vec<SchedulePeriod>>,

    /// Named schedule plans
    #[serde(default = "default_schedule_plans")]
    pub schedule_plans: Vec<SchedulePlan>,

    /// Day-of-week to schedule plan assignments
    #[serde(default = "default_day_assignments")]
    pub day_assignments: DayAssignments,

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
            schedule: None,
            schedule_plans: default_schedule_plans(),
            day_assignments: default_day_assignments(),
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

        // Migrate legacy configurations to new format
        config.migrate_legacy_config();

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

    /// Migrate legacy configurations to new format
    fn migrate_legacy_config(&mut self) {
        let mut migrated = false;

        // Check if we need to migrate from old single-schedule format
        if let Some(schedule) = self.schedule.take() {
            if !schedule.is_empty() {
                // Check if schedule_plans is default (single Default plan)
                let is_default_plans = self.schedule_plans.len() == 1
                    && self.schedule_plans[0].name == "Default"
                    && self.schedule_plans[0].periods.len() == 1
                    && self.schedule_plans[0].periods[0].start_time == "00:00"
                    && self.schedule_plans[0].periods[0].end_time == "00:00"
                    && self.schedule_plans[0].periods[0].interval_min == 60;

                if is_default_plans {
                    tracing::info!("Migrating legacy schedule array to schedule_plans");
                    self.schedule_plans = vec![SchedulePlan::new("Default", schedule)];
                    migrated = true;
                }
            }
        }

        // Migrate legacy refresh_interval_min
        if let Some(interval) = self.refresh_interval_min.take() {
            let is_default_plans = self.schedule_plans.len() == 1
                && self.schedule_plans[0].name == "Default"
                && self.schedule_plans[0].periods.len() == 1
                && self.schedule_plans[0].periods[0].interval_min == 60;

            if is_default_plans {
                tracing::info!(
                    "Migrating legacy refresh_interval_min ({}) to schedule_plans",
                    interval
                );
                self.schedule_plans = vec![SchedulePlan::new(
                    "Default",
                    vec![SchedulePeriod::new("00:00", "00:00", interval)],
                )];
                migrated = true;
            }
        }

        if migrated {
            // Ensure all days are assigned to Default plan
            self.day_assignments = default_day_assignments();
        }
    }

    /// Validate configuration values
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate schedule plans
        if self.schedule_plans.is_empty() {
            return Err(ConfigError::ValidationError(
                "At least one schedule plan is required".to_string(),
            ));
        }

        // Check for duplicate plan names
        let mut plan_names: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for plan in &self.schedule_plans {
            if !plan_names.insert(&plan.name) {
                return Err(ConfigError::ValidationError(format!(
                    "Duplicate schedule plan name: '{}'",
                    plan.name
                )));
            }
            plan.validate()?;
        }

        // Validate day assignments
        for day in Weekday::all() {
            let plan_name = self.day_assignments.get(day).ok_or_else(|| {
                ConfigError::ValidationError(format!(
                    "Missing day assignment for {}",
                    day.display_name()
                ))
            })?;

            if !self.schedule_plans.iter().any(|p| p.name == *plan_name) {
                return Err(ConfigError::ValidationError(format!(
                    "{} is assigned to non-existent plan '{}'",
                    day.display_name(),
                    plan_name
                )));
            }
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

    /// Get schedule plan by name
    pub fn get_plan(&self, name: &str) -> Option<&SchedulePlan> {
        self.schedule_plans.iter().find(|p| p.name == name)
    }

    /// Get the schedule plan for a specific weekday
    pub fn get_plan_for_day(&self, day: Weekday) -> Option<&SchedulePlan> {
        self.day_assignments
            .get(&day)
            .and_then(|name| self.get_plan(name))
    }

    /// Get the current active schedule plan based on today's day of week
    pub fn get_current_plan(&self) -> Option<&SchedulePlan> {
        let now = chrono::Local::now();
        let weekday = Weekday::from_chrono(now.weekday());
        self.get_plan_for_day(weekday)
    }

    /// Get the current weekday
    pub fn get_current_weekday() -> Weekday {
        let now = chrono::Local::now();
        Weekday::from_chrono(now.weekday())
    }

    /// Get the current refresh interval based on day and time
    pub fn get_current_interval(&self) -> u32 {
        let now = chrono::Local::now();
        let current_minutes = now.hour() * 60 + now.minute();

        if let Some(plan) = self.get_current_plan() {
            plan.get_interval_for_time(current_minutes)
        } else {
            60 // Fallback
        }
    }

    /// Get the currently active schedule period
    pub fn get_current_period(&self) -> Option<&SchedulePeriod> {
        let now = chrono::Local::now();
        let current_minutes = now.hour() * 60 + now.minute();

        self.get_current_plan()
            .and_then(|plan| plan.get_period_for_time(current_minutes))
    }

    /// Check if an image URL is configured
    pub fn has_image_url(&self) -> bool {
        !self.image_url.trim().is_empty()
    }
}
