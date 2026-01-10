//! SPI communication wrapper for e-paper display.
//!
//! Provides SPI interface for sending commands and data to the display.
//! Uses SPI0 with CE0 (Chip Enable 0) at 4 MHz.

use super::gpio::GpioController;
use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
use thiserror::Error;

/// SPI configuration
pub mod config {
    /// SPI clock speed in Hz (4 MHz)
    pub const CLOCK_SPEED: u32 = 4_000_000;
}

/// SPI-related errors
#[derive(Error, Debug)]
pub enum SpiError {
    #[error("SPI initialization failed: {0}")]
    InitError(#[from] rppal::spi::Error),

    #[error("SPI write failed: {0}")]
    WriteError(String),
}

/// SPI display interface
pub struct SpiDisplay {
    spi: Spi,
}

impl SpiDisplay {
    /// Initialize SPI for display communication
    ///
    /// Uses SPI0, CE0, Mode 0 (CPOL=0, CPHA=0), 4 MHz clock
    pub fn new() -> Result<Self, SpiError> {
        let spi = Spi::new(
            Bus::Spi0,
            SlaveSelect::Ss0,
            config::CLOCK_SPEED,
            Mode::Mode0,
        )?;

        tracing::debug!(
            "SPI initialized: Bus=SPI0, SS=CE0, Speed={}Hz, Mode=0",
            config::CLOCK_SPEED
        );

        Ok(Self { spi })
    }

    /// Send a command byte to the display
    ///
    /// Sets DC pin LOW before sending (command mode)
    pub fn write_command(&mut self, gpio: &mut GpioController, cmd: u8) -> Result<(), SpiError> {
        gpio.dc_low();
        self.spi
            .write(&[cmd])
            .map_err(|e| SpiError::WriteError(e.to_string()))?;
        Ok(())
    }

    /// Send a single data byte to the display
    ///
    /// Sets DC pin HIGH before sending (data mode)
    #[allow(dead_code)]
    pub fn write_data(&mut self, gpio: &mut GpioController, data: u8) -> Result<(), SpiError> {
        gpio.dc_high();
        self.spi
            .write(&[data])
            .map_err(|e| SpiError::WriteError(e.to_string()))?;
        Ok(())
    }

    /// Send multiple data bytes to the display
    ///
    /// Sets DC pin HIGH before sending (data mode)
    /// More efficient for bulk transfers (e.g., image data)
    pub fn write_data_bulk(
        &mut self,
        gpio: &mut GpioController,
        data: &[u8],
    ) -> Result<(), SpiError> {
        gpio.dc_high();

        // Write in chunks to avoid potential buffer issues
        const CHUNK_SIZE: usize = 4096;

        for chunk in data.chunks(CHUNK_SIZE) {
            self.spi
                .write(chunk)
                .map_err(|e| SpiError::WriteError(e.to_string()))?;
        }

        Ok(())
    }

    /// Send command followed by data bytes
    pub fn write_command_data(
        &mut self,
        gpio: &mut GpioController,
        cmd: u8,
        data: &[u8],
    ) -> Result<(), SpiError> {
        self.write_command(gpio, cmd)?;
        if !data.is_empty() {
            self.write_data_bulk(gpio, data)?;
        }
        Ok(())
    }
}

