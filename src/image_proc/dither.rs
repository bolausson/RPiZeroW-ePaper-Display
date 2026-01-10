//! Floyd-Steinberg dithering for 7-color e-paper display.
//!
//! Converts RGB images to the 7-color palette of the EPD7IN3E display
//! using error diffusion dithering.

use crate::display::Color;
use image::RgbImage;

/// RGB values for each display color
const PALETTE: [(i32, i32, i32); 7] = [
    (0, 0, 0),       // Black
    (255, 255, 255), // White
    (255, 255, 0),   // Yellow
    (255, 0, 0),     // Red
    (255, 128, 0),   // Orange
    (0, 0, 255),     // Blue
    (0, 255, 0),     // Green
];

/// Find the nearest palette color using Euclidean distance in RGB space
fn find_nearest_color(r: i32, g: i32, b: i32) -> usize {
    PALETTE
        .iter()
        .enumerate()
        .min_by_key(|(_, (pr, pg, pb))| {
            let dr = r - pr;
            let dg = g - pg;
            let db = b - pb;
            dr * dr + dg * dg + db * db
        })
        .map(|(i, _)| i)
        .unwrap_or(0)
}

/// Calculate buffer size for given dimensions (2 pixels per byte)
pub fn calculate_buffer_size(width: u32, height: u32) -> usize {
    (width as usize * height as usize) / 2
}

/// Apply Floyd-Steinberg dithering to an RGB image
///
/// Returns a buffer of packed 4-bit pixel data (2 pixels per byte)
/// suitable for sending to the EPD7IN3E display.
///
/// The image dimensions should match the expected target dimensions.
pub fn dither_image(img: &RgbImage) -> Vec<u8> {
    let (width, height) = img.dimensions();

    tracing::info!("Applying Floyd-Steinberg dithering ({}x{})", width, height);

    // Create working buffer with i32 to handle error diffusion overflow
    let mut buffer: Vec<(i32, i32, i32)> = img
        .pixels()
        .map(|p| (p[0] as i32, p[1] as i32, p[2] as i32))
        .collect();

    // Output buffer (packed 4-bit pixels) - calculated dynamically
    let buffer_size = calculate_buffer_size(width, height);
    let mut result = vec![0u8; buffer_size];

    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) as usize;
            let (r, g, b) = buffer[idx];

            // Clamp values to valid range
            let r = r.clamp(0, 255);
            let g = g.clamp(0, 255);
            let b = b.clamp(0, 255);

            // Find nearest palette color
            let color_idx = find_nearest_color(r, g, b);
            let (pr, pg, pb) = PALETTE[color_idx];

            // Calculate quantization error
            let err_r = r - pr;
            let err_g = g - pg;
            let err_b = b - pb;

            // Distribute error to neighboring pixels (Floyd-Steinberg pattern)
            // Right: 7/16
            if x + 1 < width {
                let i = idx + 1;
                buffer[i].0 += err_r * 7 / 16;
                buffer[i].1 += err_g * 7 / 16;
                buffer[i].2 += err_b * 7 / 16;
            }

            if y + 1 < height {
                // Bottom-left: 3/16
                if x > 0 {
                    let i = idx + width as usize - 1;
                    buffer[i].0 += err_r * 3 / 16;
                    buffer[i].1 += err_g * 3 / 16;
                    buffer[i].2 += err_b * 3 / 16;
                }

                // Bottom: 5/16
                let i = idx + width as usize;
                buffer[i].0 += err_r * 5 / 16;
                buffer[i].1 += err_g * 5 / 16;
                buffer[i].2 += err_b * 5 / 16;

                // Bottom-right: 1/16
                if x + 1 < width {
                    let i = idx + width as usize + 1;
                    buffer[i].0 += err_r / 16;
                    buffer[i].1 += err_g / 16;
                    buffer[i].2 += err_b / 16;
                }
            }

            // Pack two 4-bit pixels into one byte
            let byte_idx = idx / 2;
            if x % 2 == 0 {
                result[byte_idx] = (color_idx as u8) << 4;
            } else {
                result[byte_idx] |= color_idx as u8;
            }
        }
    }

    tracing::debug!("Dithering complete, output size: {} bytes", result.len());
    result
}

/// Get color name for debugging
#[allow(dead_code)]
pub fn color_name(color: Color) -> &'static str {
    match color {
        Color::Black => "Black",
        Color::White => "White",
        Color::Yellow => "Yellow",
        Color::Red => "Red",
        Color::Orange => "Orange",
        Color::Blue => "Blue",
        Color::Green => "Green",
    }
}

