//! GPIO controller for e-paper display.
//!
//! Manages the GPIO pins used for display control:
//! - RST (Reset): GPIO 17
//! - DC (Data/Command): GPIO 25
//! - BUSY: GPIO 24
//! - PWR (Power): GPIO 18

use rppal::gpio::{Gpio, InputPin, Level, OutputPin};
use std::thread;
use std::time::Duration;
use thiserror::Error;

/// GPIO pin assignments (BCM numbering)
pub mod pins {
    pub const RST: u8 = 17;   // Reset pin
    pub const DC: u8 = 25;    // Data/Command pin
    pub const BUSY: u8 = 24;  // Busy status pin
    pub const PWR: u8 = 18;   // Power control pin
}

/// GPIO-related errors
#[derive(Error, Debug)]
pub enum GpioError {
    #[error("GPIO initialization failed: {0}")]
    InitError(#[from] rppal::gpio::Error),

    #[error("Busy timeout: display did not respond within {0}ms")]
    BusyTimeout(u64),
}

/// GPIO controller for e-paper display
pub struct GpioController {
    rst: OutputPin,
    dc: OutputPin,
    pwr: OutputPin,
    busy: InputPin,
}

impl GpioController {
    /// Initialize GPIO pins for display control
    pub fn new() -> Result<Self, GpioError> {
        let gpio = Gpio::new()?;

        let mut rst = gpio.get(pins::RST)?.into_output();
        let mut dc = gpio.get(pins::DC)?.into_output();
        let mut pwr = gpio.get(pins::PWR)?.into_output();
        let busy = gpio.get(pins::BUSY)?.into_input_pulldown();

        // Initialize pins to known state
        rst.set_high();
        dc.set_low();
        pwr.set_low();

        tracing::debug!(
            "GPIO initialized: RST={}, DC={}, BUSY={}, PWR={}",
            pins::RST,
            pins::DC,
            pins::BUSY,
            pins::PWR
        );

        Ok(Self { rst, dc, pwr, busy })
    }

    /// Perform hardware reset sequence
    pub fn reset(&mut self) {
        tracing::debug!("Performing hardware reset");

        self.rst.set_high();
        thread::sleep(Duration::from_millis(20));

        self.rst.set_low();
        thread::sleep(Duration::from_millis(2));

        self.rst.set_high();
        thread::sleep(Duration::from_millis(20));
    }

    /// Wait for display to become ready (BUSY pin goes high)
    ///
    /// The display signals busy state by pulling the BUSY pin LOW.
    /// When ready, the pin goes HIGH.
    pub fn wait_busy(&self) -> Result<(), GpioError> {
        self.wait_busy_timeout(Duration::from_secs(30))
    }

    /// Wait for display with custom timeout
    pub fn wait_busy_timeout(&self, timeout: Duration) -> Result<(), GpioError> {
        let start = std::time::Instant::now();
        let poll_interval = Duration::from_millis(100);

        let initial_state = self.busy.read();
        tracing::debug!("BUSY pin initial state: {:?}", initial_state);

        // Wait while BUSY is LOW (display is busy)
        while self.busy.read() == Level::Low {
            if start.elapsed() > timeout {
                return Err(GpioError::BusyTimeout(timeout.as_millis() as u64));
            }
            thread::sleep(poll_interval);
        }

        let elapsed = start.elapsed();
        if elapsed.as_millis() > 100 {
            tracing::debug!("BUSY wait completed after {:?}", elapsed);
        }

        Ok(())
    }

    /// Check if display is currently busy
    #[allow(dead_code)]
    pub fn is_busy(&self) -> bool {
        self.busy.read() == Level::Low
    }

    /// Set DC pin low (command mode)
    #[inline]
    pub fn dc_low(&mut self) {
        self.dc.set_low();
    }

    /// Set DC pin high (data mode)
    #[inline]
    pub fn dc_high(&mut self) {
        self.dc.set_high();
    }

    /// Enable display power
    pub fn power_on(&mut self) {
        tracing::debug!("Display power ON");
        self.pwr.set_high();
        thread::sleep(Duration::from_millis(10));
    }

    /// Disable display power
    pub fn power_off(&mut self) {
        tracing::debug!("Display power OFF");
        self.pwr.set_low();
    }
}

impl Drop for GpioController {
    fn drop(&mut self) {
        // Ensure power is off when controller is dropped
        self.pwr.set_low();
        tracing::debug!("GPIO controller dropped, power disabled");
    }
}

