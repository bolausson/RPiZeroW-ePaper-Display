//! Waveshare 7.3" E Ink Spectra 6 (EPD7IN3E) display driver.
//!
//! 7-color e-paper display: Black, White, Yellow, Red, Orange, Blue, Green
//! Resolution: 800 x 480 pixels
//! 4-bit color depth (2 pixels per byte)
//!
//! Based on official Waveshare Python driver:
//! https://github.com/waveshare/e-Paper/blob/master/RaspberryPi_JetsonNano/python/lib/waveshare_epd/epd7in3e.py

use super::gpio::{GpioController, GpioError};
use super::spi::{SpiDisplay, SpiError};
use std::thread;
use std::time::Duration;
use thiserror::Error;

/// Display dimensions
pub const WIDTH: u32 = 800;
pub const HEIGHT: u32 = 480;

/// Buffer size: 2 pixels per byte (4-bit color)
pub const BUFFER_SIZE: usize = (WIDTH as usize * HEIGHT as usize) / 2;

/// EPD commands (from official Waveshare driver)
#[allow(dead_code)]
mod cmd {
    pub const CMDH: u8 = 0xAA;              // Command header
    pub const POWER_SETTING: u8 = 0x01;
    pub const POWER_OFF: u8 = 0x02;
    pub const POWER_ON: u8 = 0x04;
    pub const DEEP_SLEEP: u8 = 0x07;
    pub const DATA_START: u8 = 0x10;
    pub const DISPLAY_REFRESH: u8 = 0x12;
    pub const PLL_CONTROL: u8 = 0x30;
    pub const VCOM_DATA_INTERVAL: u8 = 0x50;
    pub const TCON_SETTING: u8 = 0x60;
    pub const RESOLUTION_SETTING: u8 = 0x61;
    pub const POWER_SAVING: u8 = 0xE3;
    pub const PANEL_SETTING: u8 = 0x00;
    pub const INPUT_DATA: u8 = 0x03;
    pub const BOOSTER_SOFT_START1: u8 = 0x05;
    pub const BOOSTER_SOFT_START2: u8 = 0x06;
    pub const BOOSTER_SOFT_START3: u8 = 0x08;
    pub const UNKNOWN_84: u8 = 0x84;
}

/// 7-color palette indices
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Black = 0,
    White = 1,
    Yellow = 2,
    Red = 3,
    Orange = 4,
    Blue = 5,
    Green = 6,
}

/// Display driver errors
#[derive(Error, Debug)]
pub enum DisplayError {
    #[error("GPIO error: {0}")]
    Gpio(#[from] GpioError),

    #[error("SPI error: {0}")]
    Spi(#[from] SpiError),

    #[error("Display not initialized")]
    NotInitialized,

    #[error("Invalid buffer size: expected {expected}, got {actual}")]
    InvalidBufferSize { expected: usize, actual: usize },
}

/// EPD7IN3E display driver
pub struct Epd7in3e {
    gpio: GpioController,
    spi: SpiDisplay,
    initialized: bool,
}

impl Epd7in3e {
    /// Create a new display driver instance
    pub fn new() -> Result<Self, DisplayError> {
        let gpio = GpioController::new()?;
        let spi = SpiDisplay::new()?;

        Ok(Self {
            gpio,
            spi,
            initialized: false,
        })
    }

    /// Initialize the display hardware
    /// Based on official Waveshare epd7in3e.py init() sequence
    pub fn init(&mut self) -> Result<(), DisplayError> {
        tracing::info!("Initializing EPD7IN3E display ({}x{})", WIDTH, HEIGHT);

        // Power on and reset
        self.gpio.power_on();
        self.gpio.reset();
        self.gpio.wait_busy()?;
        thread::sleep(Duration::from_millis(30));

        // Command header (0xAA)
        self.send_command_data(cmd::CMDH, &[0x49, 0x55, 0x20, 0x08, 0x09, 0x18])?;

        // Power setting (0x01)
        self.send_command_data(cmd::POWER_SETTING, &[0x3F])?;

        // Panel setting (0x00)
        self.send_command_data(cmd::PANEL_SETTING, &[0x5F, 0x69])?;

        // Input data setting (0x03)
        self.send_command_data(cmd::INPUT_DATA, &[0x00, 0x54, 0x00, 0x44])?;

        // Booster soft start 1 (0x05)
        self.send_command_data(cmd::BOOSTER_SOFT_START1, &[0x40, 0x1F, 0x1F, 0x2C])?;

        // Booster soft start 2 (0x06)
        self.send_command_data(cmd::BOOSTER_SOFT_START2, &[0x6F, 0x1F, 0x17, 0x49])?;

        // Booster soft start 3 (0x08)
        self.send_command_data(cmd::BOOSTER_SOFT_START3, &[0x6F, 0x1F, 0x1F, 0x22])?;

        // PLL control (0x30)
        self.send_command_data(cmd::PLL_CONTROL, &[0x03])?;

        // VCOM and data interval (0x50)
        self.send_command_data(cmd::VCOM_DATA_INTERVAL, &[0x3F])?;

        // TCON setting (0x60)
        self.send_command_data(cmd::TCON_SETTING, &[0x02, 0x00])?;

        // Resolution setting (0x61) - 800 x 480 = 0x0320 x 0x01E0
        self.send_command_data(cmd::RESOLUTION_SETTING, &[0x03, 0x20, 0x01, 0xE0])?;

        // Unknown command 0x84
        self.send_command_data(cmd::UNKNOWN_84, &[0x01])?;

        // Power saving (0xE3)
        self.send_command_data(cmd::POWER_SAVING, &[0x2F])?;

        // Power on (0x04) and wait for ready
        self.send_command(cmd::POWER_ON)?;
        self.gpio.wait_busy()?;

        self.initialized = true;
        tracing::info!("Display initialized successfully");

        Ok(())
    }

    /// Display image data from buffer
    ///
    /// Buffer should contain packed 4-bit pixel data (2 pixels per byte)
    /// Based on official Waveshare display() and TurnOnDisplay() sequence
    ///
    /// Note: Buffer size should match the display dimensions (800x480 / 2 = 192000 bytes)
    /// but we allow flexibility for different configured dimensions as the display
    /// hardware will handle the data stream.
    pub fn display(&mut self, buffer: &[u8]) -> Result<(), DisplayError> {
        if !self.initialized {
            return Err(DisplayError::NotInitialized);
        }

        // Validate buffer size matches expected display size
        // The hardware expects exactly BUFFER_SIZE bytes for 800x480 display
        if buffer.len() != BUFFER_SIZE {
            tracing::warn!(
                "Buffer size mismatch: expected {} bytes for {}x{}, got {} bytes",
                BUFFER_SIZE, WIDTH, HEIGHT, buffer.len()
            );
            return Err(DisplayError::InvalidBufferSize {
                expected: BUFFER_SIZE,
                actual: buffer.len(),
            });
        }

        tracing::info!("Sending image data to display ({} bytes)", buffer.len());

        // Send image data (command 0x10)
        self.send_command(cmd::DATA_START)?;
        self.spi.write_data_bulk(&mut self.gpio, buffer)?;

        // TurnOnDisplay sequence from official driver
        self.turn_on_display()?;

        tracing::info!("Display refresh complete");
        Ok(())
    }

    /// Turn on display and refresh
    /// Based on official Waveshare TurnOnDisplay() sequence
    fn turn_on_display(&mut self) -> Result<(), DisplayError> {
        // Power on (0x04)
        self.send_command(cmd::POWER_ON)?;
        self.gpio.wait_busy()?;

        // Display refresh (0x12) with data byte 0x00
        self.send_command_data(cmd::DISPLAY_REFRESH, &[0x00])?;
        tracing::info!("Waiting for display refresh to complete...");
        self.gpio.wait_busy()?;

        // Power off (0x02) with data byte 0x00
        self.send_command_data(cmd::POWER_OFF, &[0x00])?;
        self.gpio.wait_busy()?;

        Ok(())
    }

    /// Clear display to a single color
    pub fn clear(&mut self, color: Color) -> Result<(), DisplayError> {
        if !self.initialized {
            self.init()?;
        }

        let pixel = (color as u8) << 4 | (color as u8);
        let buffer = vec![pixel; BUFFER_SIZE];

        tracing::info!("Clearing display to {:?}", color);
        self.display(&buffer)
    }

    /// Display test pattern showing all 7 colors
    pub fn test_pattern(&mut self) -> Result<(), DisplayError> {
        if !self.initialized {
            self.init()?;
        }

        tracing::info!("Displaying test pattern");

        let mut buffer = vec![0u8; BUFFER_SIZE];
        let stripe_height = HEIGHT / 7;

        for y in 0..HEIGHT {
            let color = match y / stripe_height {
                0 => Color::Black,
                1 => Color::White,
                2 => Color::Yellow,
                3 => Color::Red,
                4 => Color::Orange,
                5 => Color::Blue,
                _ => Color::Green,
            } as u8;

            let packed = (color << 4) | color;

            for x in (0..WIDTH).step_by(2) {
                let idx = ((y * WIDTH + x) / 2) as usize;
                buffer[idx] = packed;
            }
        }

        self.display(&buffer)
    }

    /// Put display into deep sleep mode
    pub fn sleep(&mut self) -> Result<(), DisplayError> {
        tracing::info!("Putting display to sleep");

        self.send_command(cmd::POWER_OFF)?;
        self.gpio.wait_busy()?;

        self.send_command_data(cmd::DEEP_SLEEP, &[0xA5])?;

        self.gpio.power_off();
        self.initialized = false;

        Ok(())
    }

    /// Wake display from sleep
    #[allow(dead_code)]
    pub fn wake(&mut self) -> Result<(), DisplayError> {
        if self.initialized {
            return Ok(());
        }
        self.init()
    }

    /// Send command to display
    fn send_command(&mut self, cmd: u8) -> Result<(), DisplayError> {
        self.spi.write_command(&mut self.gpio, cmd)?;
        Ok(())
    }

    /// Send command with data to display
    fn send_command_data(&mut self, cmd: u8, data: &[u8]) -> Result<(), DisplayError> {
        self.spi.write_command_data(&mut self.gpio, cmd, data)?;
        Ok(())
    }
}

impl Drop for Epd7in3e {
    fn drop(&mut self) {
        if self.initialized {
            let _ = self.sleep();
        }
    }
}

