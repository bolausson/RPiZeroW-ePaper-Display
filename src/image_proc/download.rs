//! Image download functionality.
//!
//! Handles fetching images from HTTP/HTTPS URLs with retry logic.

use image::DynamicImage;
use std::time::Duration;
use thiserror::Error;

/// Download errors
#[derive(Error, Debug)]
pub enum DownloadError {
    #[error("HTTP request failed: {0}")]
    RequestError(#[from] reqwest::Error),

    #[error("HTTP error: {status}")]
    HttpError { status: u16 },

    #[error("Image decode failed: {0}")]
    DecodeError(#[from] image::ImageError),

    #[error("Empty URL")]
    EmptyUrl,

    #[error("Download timeout")]
    Timeout,
}

/// Download configuration
#[derive(Debug, Clone)]
pub struct DownloadConfig {
    /// Request timeout
    pub timeout: Duration,
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Base delay between retries (doubled each attempt)
    pub retry_delay: Duration,
    /// Maximum image dimensions
    pub max_width: u32,
    pub max_height: u32,
}

impl Default for DownloadConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            max_retries: 3,
            retry_delay: Duration::from_secs(2),
            max_width: 4096,
            max_height: 4096,
        }
    }
}

/// Download an image from a URL
pub async fn download_image(url: &str) -> Result<DynamicImage, DownloadError> {
    download_image_with_config(url, &DownloadConfig::default()).await
}

/// Download an image from a URL with custom configuration
pub async fn download_image_with_config(
    url: &str,
    config: &DownloadConfig,
) -> Result<DynamicImage, DownloadError> {
    let url = url.trim();
    if url.is_empty() {
        return Err(DownloadError::EmptyUrl);
    }

    tracing::info!("Downloading image from: {}", url);

    let client = reqwest::Client::builder()
        .timeout(config.timeout)
        .build()?;

    let bytes: bytes::Bytes = download_with_retry(&client, url, config).await?;

    tracing::debug!("Downloaded {} bytes, decoding image...", bytes.len());

    // Decode image with size limits
    let reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| DownloadError::DecodeError(image::ImageError::IoError(e)))?;

    let img = reader.decode()?;

    // Check dimensions
    let (width, height) = (img.width(), img.height());
    tracing::info!("Image decoded: {}x{}", width, height);

    if width > config.max_width || height > config.max_height {
        tracing::warn!(
            "Image dimensions {}x{} exceed maximum {}x{}",
            width,
            height,
            config.max_width,
            config.max_height
        );
    }

    Ok(img)
}

/// Download with retry logic
async fn download_with_retry(
    client: &reqwest::Client,
    url: &str,
    config: &DownloadConfig,
) -> Result<bytes::Bytes, DownloadError> {
    let mut last_error = None;

    for attempt in 0..config.max_retries {
        if attempt > 0 {
            let delay = config.retry_delay * 2u32.pow(attempt - 1);
            tracing::debug!("Retry attempt {}/{}, waiting {:?}", attempt + 1, config.max_retries, delay);
            tokio::time::sleep(delay).await;
        }

        match client.get(url).send().await {
            Ok(response) => {
                let status = response.status();

                if status.is_success() {
                    match response.bytes().await {
                        Ok(bytes) => return Ok(bytes),
                        Err(e) => {
                            tracing::warn!("Failed to read response body: {}", e);
                            last_error = Some(DownloadError::RequestError(e));
                        }
                    }
                } else {
                    tracing::warn!("HTTP error: {} for {}", status, url);
                    last_error = Some(DownloadError::HttpError {
                        status: status.as_u16(),
                    });
                }
            }
            Err(e) => {
                tracing::warn!("Request failed: {} for {}", e, url);
                last_error = Some(DownloadError::RequestError(e));
            }
        }
    }

    Err(last_error.unwrap_or(DownloadError::Timeout))
}

