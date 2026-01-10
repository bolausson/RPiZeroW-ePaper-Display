//! Floyd-Steinberg dithering for 7-color e-paper display.
//!
//! Converts RGB images to the 7-color palette of the EPD7IN3E display
//! using error diffusion dithering.
//!
//! This implementation uses a memory-optimized row-by-row approach that only
//! keeps 2 rows in memory at a time, reducing memory usage from ~4.4MB to ~19KB
//! for an 800x480 image. This is critical for the Pi Zero W's limited RAM.

use crate::display::Color;
use image::RgbImage;

/// RGB values for each display color (using i16 for error diffusion arithmetic)
const PALETTE: [(i16, i16, i16); 7] = [
    (0, 0, 0),       // Black
    (255, 255, 255), // White
    (255, 255, 0),   // Yellow
    (255, 0, 0),     // Red
    (255, 128, 0),   // Orange
    (0, 0, 255),     // Blue
    (0, 255, 0),     // Green
];

/// Find the nearest palette color using Euclidean distance in RGB space
/// Uses i32 internally for distance calculation to avoid overflow
#[inline]
fn find_nearest_color(r: i16, g: i16, b: i16) -> usize {
    PALETTE
        .iter()
        .enumerate()
        .min_by_key(|(_, (pr, pg, pb))| {
            let dr = (r - pr) as i32;
            let dg = (g - pg) as i32;
            let db = (b - pb) as i32;
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
/// This implementation uses a memory-optimized row-by-row approach:
/// - Only keeps 2 rows of error accumulation in memory at a time
/// - Uses i16 instead of i32 (error range is -255 to +255, fits in i16)
/// - Memory usage: ~19KB for 2 rows vs ~4.4MB for full image buffer
///
/// The image dimensions should match the expected target dimensions.
pub fn dither_image(img: &RgbImage) -> Vec<u8> {
    let (width, height) = img.dimensions();
    let width_usize = width as usize;
    let height_usize = height as usize;

    tracing::info!(
        "Applying Floyd-Steinberg dithering ({}x{}) - memory optimized",
        width,
        height
    );

    // Only need 2 rows at a time: current and next
    // Using i16 instead of i32 (error range is -255 to +255, fits in i16)
    // Memory: 2 * width * 6 bytes = ~9.6KB for 800px width
    let mut curr_row: Vec<(i16, i16, i16)> = vec![(0, 0, 0); width_usize];
    let mut next_row: Vec<(i16, i16, i16)> = vec![(0, 0, 0); width_usize];

    // Output buffer (packed 4-bit pixels)
    let buffer_size = calculate_buffer_size(width, height);
    let mut result = vec![0u8; buffer_size];

    for y in 0..height_usize {
        // Load current row pixels and add accumulated error from previous row
        for x in 0..width_usize {
            let p = img.get_pixel(x as u32, y as u32);
            curr_row[x].0 += p[0] as i16;
            curr_row[x].1 += p[1] as i16;
            curr_row[x].2 += p[2] as i16;
        }

        for x in 0..width_usize {
            let (r, g, b) = curr_row[x];

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
            if x + 1 < width_usize {
                curr_row[x + 1].0 += err_r * 7 / 16;
                curr_row[x + 1].1 += err_g * 7 / 16;
                curr_row[x + 1].2 += err_b * 7 / 16;
            }

            if y + 1 < height_usize {
                // Bottom-left: 3/16
                if x > 0 {
                    next_row[x - 1].0 += err_r * 3 / 16;
                    next_row[x - 1].1 += err_g * 3 / 16;
                    next_row[x - 1].2 += err_b * 3 / 16;
                }

                // Bottom: 5/16
                next_row[x].0 += err_r * 5 / 16;
                next_row[x].1 += err_g * 5 / 16;
                next_row[x].2 += err_b * 5 / 16;

                // Bottom-right: 1/16
                if x + 1 < width_usize {
                    next_row[x + 1].0 += err_r / 16;
                    next_row[x + 1].1 += err_g / 16;
                    next_row[x + 1].2 += err_b / 16;
                }
            }

            // Pack two 4-bit pixels into one byte
            let byte_idx = (y * width_usize + x) / 2;
            if x % 2 == 0 {
                result[byte_idx] = (color_idx as u8) << 4;
            } else {
                result[byte_idx] |= color_idx as u8;
            }
        }

        // Swap rows: next becomes current, current is cleared for next iteration
        std::mem::swap(&mut curr_row, &mut next_row);
        // Clear the row that will accumulate errors for the row after next
        next_row.iter_mut().for_each(|p| *p = (0, 0, 0));
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

