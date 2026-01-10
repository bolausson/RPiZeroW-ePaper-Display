//! Image processing module.
//!
//! Provides image download, transformation, and dithering for the e-paper display.

pub mod dither;
pub mod download;
pub mod transform;

pub use dither::dither_image;
pub use download::{download_image, DownloadError};
pub use transform::{transform_image, Rotation, TransformOptions};

use crate::config::Config;
use crate::display::DisplayController;
use thiserror::Error;

/// Image processing errors
#[derive(Error, Debug)]
pub enum ProcessingError {
    #[error("Download error: {0}")]
    Download(#[from] DownloadError),

    #[error("Display error: {0}")]
    Display(#[from] crate::display::DisplayError),

    #[error("No image URL configured")]
    NoImageUrl,
}

/// Image processor that handles the full pipeline
pub struct ImageProcessor {
    display: DisplayController,
}

impl ImageProcessor {
    /// Create a new image processor
    pub fn new(display: DisplayController) -> Self {
        Self { display }
    }

    /// Process and display an image from URL
    ///
    /// Full pipeline:
    /// 1. Download image from URL
    /// 2. Apply transformations (rotate, mirror, scale)
    /// 3. Dither to 7-color palette
    /// 4. Send to display
    ///
    /// Memory optimization: Explicitly drops intermediate buffers to free
    /// memory before the next allocation. This reduces peak memory usage
    /// on the Pi Zero W's constrained RAM.
    pub async fn process_and_display(&self, config: &Config) -> Result<(), ProcessingError> {
        if !config.has_image_url() {
            return Err(ProcessingError::NoImageUrl);
        }

        tracing::info!("Starting image processing pipeline");

        // Download image (~1.5MB for 800x480 RGBA)
        let img = download_image(&config.image_url).await?;

        // Apply transformations with configurable dimensions and transform order
        // `img` is consumed here, freeing the original ~1.5MB DynamicImage
        let options = TransformOptions {
            rotation: Rotation::from(config.rotation),
            mirror_h: config.mirror_h,
            mirror_v: config.mirror_v,
            scale_to_fit: config.scale_to_fit,
            rotate_first: config.rotate_first,
            target_width: config.display_width,
            target_height: config.display_height,
        };
        let rgb_image = transform_image(img, &options);
        // Note: `img` is now moved into transform_image and freed

        // Dither to 7-color palette (~192KB output for 800x480)
        // The dither function uses row-by-row processing (~19KB working memory)
        let buffer = dither_image(&rgb_image);

        // Explicitly drop rgb_image (~1.15MB) before display operation
        // This ensures we have freed as much memory as possible before
        // the display operation which may also need buffers
        drop(rgb_image);

        // Ensure display is initialized
        self.display.init().await?;

        // Send to display - only `buffer` (~192KB) is in memory now
        self.display.display(&buffer).await?;

        tracing::info!("Image processing complete");
        Ok(())
    }

    /// Show test pattern on display
    pub async fn show_test_pattern(&self) -> Result<(), ProcessingError> {
        self.display.test_pattern().await?;
        Ok(())
    }

    /// Clear display
    pub async fn clear_display(&self) -> Result<(), ProcessingError> {
        self.display.init().await?;
        self.display.clear().await?;
        Ok(())
    }

    /// Put display to sleep
    #[allow(dead_code)]
    pub async fn sleep_display(&self) -> Result<(), ProcessingError> {
        self.display.sleep().await?;
        Ok(())
    }
}

