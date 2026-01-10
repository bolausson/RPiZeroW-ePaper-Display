//! Display module for e-paper display control.
//!
//! This module provides the interface to the Waveshare 7.3" E Ink Spectra 6
//! (EPD7IN3E) display connected via SPI.

pub mod epd7in3e;
pub mod gpio;
pub mod spi;

// Re-export main types
pub use epd7in3e::{Color, DisplayError, Epd7in3e};

use std::sync::Arc;
use tokio::sync::Mutex;

/// Thread-safe display controller wrapper
pub struct DisplayController {
    display: Arc<Mutex<Option<Epd7in3e>>>,
}

impl DisplayController {
    /// Create a new display controller (uninitialized)
    pub fn new() -> Self {
        Self {
            display: Arc::new(Mutex::new(None)),
        }
    }

    /// Initialize the display hardware
    pub async fn init(&self) -> Result<(), DisplayError> {
        let mut display_guard = self.display.lock().await;

        if display_guard.is_some() {
            tracing::debug!("Display already initialized");
            return Ok(());
        }

        // Create and initialize display
        let mut epd = Epd7in3e::new()?;
        epd.init()?;

        *display_guard = Some(epd);
        Ok(())
    }

    /// Display image buffer
    pub async fn display(&self, buffer: &[u8]) -> Result<(), DisplayError> {
        let mut display_guard = self.display.lock().await;

        let display = display_guard
            .as_mut()
            .ok_or(DisplayError::NotInitialized)?;

        display.display(buffer)
    }

    /// Clear display to white
    pub async fn clear(&self) -> Result<(), DisplayError> {
        let mut display_guard = self.display.lock().await;

        let display = display_guard
            .as_mut()
            .ok_or(DisplayError::NotInitialized)?;

        display.clear(Color::White)
    }

    /// Show test pattern
    pub async fn test_pattern(&self) -> Result<(), DisplayError> {
        let mut display_guard = self.display.lock().await;

        // Initialize if needed
        if display_guard.is_none() {
            drop(display_guard);
            self.init().await?;
            display_guard = self.display.lock().await;
        }

        let display = display_guard
            .as_mut()
            .ok_or(DisplayError::NotInitialized)?;

        display.test_pattern()
    }

    /// Put display to sleep
    pub async fn sleep(&self) -> Result<(), DisplayError> {
        let mut display_guard = self.display.lock().await;

        if let Some(display) = display_guard.as_mut() {
            display.sleep()?;
            *display_guard = None;
        }

        Ok(())
    }

    /// Check if display is initialized
    #[allow(dead_code)]
    pub async fn is_initialized(&self) -> bool {
        self.display.lock().await.is_some()
    }
}

impl Default for DisplayController {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for DisplayController {
    fn clone(&self) -> Self {
        Self {
            display: Arc::clone(&self.display),
        }
    }
}

