//! Image transformation operations.
//!
//! Provides scaling, rotation, and mirroring for display preparation.

use image::{DynamicImage, GenericImageView, RgbImage};

/// Rotation angle in degrees
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rotation {
    None,
    Rotate90,
    Rotate180,
    Rotate270,
}

impl From<u16> for Rotation {
    fn from(degrees: u16) -> Self {
        match degrees {
            90 => Rotation::Rotate90,
            180 => Rotation::Rotate180,
            270 => Rotation::Rotate270,
            _ => Rotation::None,
        }
    }
}

/// Image transformation options
#[derive(Debug, Clone)]
pub struct TransformOptions {
    /// Rotation to apply
    pub rotation: Rotation,
    /// Mirror horizontally
    pub mirror_h: bool,
    /// Mirror vertically
    pub mirror_v: bool,
    /// Scale to fit display dimensions
    pub scale_to_fit: bool,
    /// Apply rotation before mirroring (true) or mirror before rotating (false)
    pub rotate_first: bool,
    /// Target display width
    pub target_width: u32,
    /// Target display height
    pub target_height: u32,
}

impl Default for TransformOptions {
    fn default() -> Self {
        Self {
            rotation: Rotation::None,
            mirror_h: false,
            mirror_v: false,
            scale_to_fit: true,
            rotate_first: true,
            target_width: 800,
            target_height: 480,
        }
    }
}

/// Transform an image for display
///
/// Applies the following operations based on rotate_first setting:
/// - If rotate_first: Rotation → Mirroring → Scaling
/// - If !rotate_first: Mirroring → Rotation → Scaling
pub fn transform_image(img: DynamicImage, options: &TransformOptions) -> RgbImage {
    let mut img = img;

    if options.rotate_first {
        // Rotate first, then mirror
        img = apply_rotation(img, options.rotation);
        img = apply_mirroring(img, options.mirror_h, options.mirror_v);
    } else {
        // Mirror first, then rotate
        img = apply_mirroring(img, options.mirror_h, options.mirror_v);
        img = apply_rotation(img, options.rotation);
    }

    // Scale to display size
    let (target_width, target_height) = (options.target_width, options.target_height);

    let scaled = if options.scale_to_fit {
        scale_to_fit(img, target_width, target_height)
    } else {
        scale_to_fill(img, target_width, target_height)
    };

    scaled.into_rgb8()
}

/// Apply rotation to image
fn apply_rotation(img: DynamicImage, rotation: Rotation) -> DynamicImage {
    match rotation {
        Rotation::None => img,
        Rotation::Rotate90 => img.rotate90(),
        Rotation::Rotate180 => img.rotate180(),
        Rotation::Rotate270 => img.rotate270(),
    }
}

/// Apply mirroring to image
fn apply_mirroring(mut img: DynamicImage, mirror_h: bool, mirror_v: bool) -> DynamicImage {
    if mirror_h {
        img = img.fliph();
    }
    if mirror_v {
        img = img.flipv();
    }
    img
}

/// Scale image to fit within dimensions (letterbox/pillarbox)
fn scale_to_fit(img: DynamicImage, max_width: u32, max_height: u32) -> DynamicImage {
    let (src_width, src_height) = img.dimensions();

    // Calculate scale factor to fit within bounds
    let scale_w = max_width as f32 / src_width as f32;
    let scale_h = max_height as f32 / src_height as f32;
    let scale = scale_w.min(scale_h);

    let new_width = (src_width as f32 * scale) as u32;
    let new_height = (src_height as f32 * scale) as u32;

    tracing::debug!(
        "Scaling {}x{} -> {}x{} (fit into {}x{})",
        src_width,
        src_height,
        new_width,
        new_height,
        max_width,
        max_height
    );

    // Resize the image
    let resized = img.resize(new_width, new_height, image::imageops::FilterType::Triangle);

    // Create canvas with white background and center the image
    let mut canvas = RgbImage::from_pixel(max_width, max_height, image::Rgb([255, 255, 255]));

    let offset_x = (max_width - new_width) / 2;
    let offset_y = (max_height - new_height) / 2;

    image::imageops::overlay(&mut canvas, &resized.to_rgb8(), offset_x as i64, offset_y as i64);

    DynamicImage::ImageRgb8(canvas)
}

/// Scale image to fill dimensions (crop overflow)
fn scale_to_fill(img: DynamicImage, target_width: u32, target_height: u32) -> DynamicImage {
    let (src_width, src_height) = img.dimensions();

    // Calculate scale factor to fill bounds
    let scale_w = target_width as f32 / src_width as f32;
    let scale_h = target_height as f32 / src_height as f32;
    let scale = scale_w.max(scale_h);

    let new_width = (src_width as f32 * scale) as u32;
    let new_height = (src_height as f32 * scale) as u32;

    tracing::debug!(
        "Scaling {}x{} -> {}x{} (fill {}x{})",
        src_width,
        src_height,
        new_width,
        new_height,
        target_width,
        target_height
    );

    // Resize the image
    let resized = img.resize_exact(new_width, new_height, image::imageops::FilterType::Triangle);

    // Crop to target size (center crop)
    let crop_x = (new_width - target_width) / 2;
    let crop_y = (new_height - target_height) / 2;

    resized.crop_imm(crop_x, crop_y, target_width, target_height)
}

