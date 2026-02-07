//! Alpha blending math for watermark removal.
//!
//! Gemini applies watermarks via forward alpha blending:
//! `watermarked = alpha * logo + (1 - alpha) * original`
//!
//! This module provides the reverse operation to recover original pixels.

use image::RgbImage;

use crate::error::{Error, Result};

/// Alpha threshold: ignore pixels with negligible watermark effect (noise).
const ALPHA_THRESHOLD: f32 = 0.002;

/// Maximum alpha: clamp to avoid division by near-zero in reverse blending.
const MAX_ALPHA: f32 = 0.99;

/// Calculate an alpha map from embedded PNG background capture data.
///
/// The PNG is a screenshot of the Gemini watermark rendered on a white background.
/// The alpha at each pixel is derived as: `alpha = max(R, G, B) / 255.0`.
///
/// Returns a flat `Vec<f32>` of length `width * height`, plus the `(width, height)`.
///
/// # Errors
///
/// Returns [`Error::AlphaMapDecode`] if the PNG data cannot be decoded.
pub fn calculate_alpha_map(png_bytes: &[u8]) -> Result<(Vec<f32>, u32, u32)> {
    let img = image::load_from_memory(png_bytes)
        .map_err(Error::AlphaMapDecode)?
        .to_rgb8();

    let width = img.width();
    let height = img.height();
    let mut alpha_map = Vec::with_capacity((width * height) as usize);

    for pixel in img.pixels() {
        let r = f32::from(pixel[0]);
        let g = f32::from(pixel[1]);
        let b = f32::from(pixel[2]);
        let max_val = r.max(g).max(b);
        alpha_map.push(max_val / 255.0);
    }

    Ok((alpha_map, width, height))
}

/// Remove watermark from an image using reverse alpha blending.
///
/// Applies the formula: `original = (watermarked - alpha * logo_value) / (1 - alpha)`
///
/// Operates in-place on the image at the specified position. Pixels with alpha
/// below the threshold (0.002) are left unchanged.
///
/// # Arguments
///
/// * `image` - The watermarked image to modify in-place.
/// * `alpha_map` - Flat array of alpha values, length `wm_width * wm_height`.
/// * `wm_width` - Width of the watermark region in pixels.
/// * `wm_height` - Height of the watermark region in pixels.
/// * `pos_x` - X coordinate of the watermark's top-left corner.
/// * `pos_y` - Y coordinate of the watermark's top-left corner.
/// * `logo_value` - The logo color value (255.0 for white).
pub fn remove_watermark_alpha_blend(
    image: &mut RgbImage,
    alpha_map: &[f32],
    wm_width: u32,
    wm_height: u32,
    pos_x: u32,
    pos_y: u32,
    logo_value: f32,
) {
    let img_w = image.width();
    let img_h = image.height();

    // Clip to image bounds
    let x2 = (pos_x + wm_width).min(img_w);
    let y2 = (pos_y + wm_height).min(img_h);

    if pos_x >= x2 || pos_y >= y2 {
        return;
    }

    for dy in 0..(y2 - pos_y) {
        for dx in 0..(x2 - pos_x) {
            let alpha_idx = (dy * wm_width + dx) as usize;
            let mut alpha = alpha_map[alpha_idx];

            // Skip pixels with negligible watermark effect
            if alpha < ALPHA_THRESHOLD {
                continue;
            }

            // Clamp alpha to avoid division instability
            alpha = alpha.min(MAX_ALPHA);
            let inv_alpha = 1.0 - alpha;

            let px = image.get_pixel_mut(pos_x + dx, pos_y + dy);
            for ch in 0..3 {
                let watermarked = f32::from(px[ch]);
                let original = (watermarked - alpha * logo_value) / inv_alpha;
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                {
                    px[ch] = original.clamp(0.0, 255.0) as u8;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alpha_maps;

    #[test]
    fn alpha_map_48_loads_with_correct_dimensions() {
        let (map, w, h) = calculate_alpha_map(alpha_maps::BG_48_PNG).unwrap();
        assert_eq!(w, 48);
        assert_eq!(h, 48);
        assert_eq!(map.len(), 48 * 48);
        for &a in &map {
            assert!((0.0..=1.0).contains(&a));
        }
    }

    #[test]
    fn alpha_map_96_loads_with_correct_dimensions() {
        let (map, w, h) = calculate_alpha_map(alpha_maps::BG_96_PNG).unwrap();
        assert_eq!(w, 96);
        assert_eq!(h, 96);
        assert_eq!(map.len(), 96 * 96);
    }

    #[test]
    fn reverse_blend_recovers_original_within_tolerance() {
        let mut original = RgbImage::new(100, 100);
        for px in original.pixels_mut() {
            *px = image::Rgb([128, 64, 200]);
        }
        let original_copy = original.clone();

        let size = 10u32;
        #[allow(clippy::cast_precision_loss)]
        let alpha_map: Vec<f32> = (0..size * size)
            .map(|i| (i as f32) / (size * size) as f32 * 0.5)
            .collect();

        let pos_x = 50u32;
        let pos_y = 50u32;
        let logo_value = 255.0f32;

        // Apply forward blend
        for dy in 0..size {
            for dx in 0..size {
                let alpha = alpha_map[(dy * size + dx) as usize];
                if alpha < ALPHA_THRESHOLD {
                    continue;
                }
                let px = original.get_pixel_mut(pos_x + dx, pos_y + dy);
                for ch in 0..3 {
                    let orig = f32::from(px[ch]);
                    let result = alpha * logo_value + (1.0 - alpha) * orig;
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    {
                        px[ch] = result.clamp(0.0, 255.0) as u8;
                    }
                }
            }
        }

        // Reverse
        remove_watermark_alpha_blend(
            &mut original,
            &alpha_map,
            size,
            size,
            pos_x,
            pos_y,
            logo_value,
        );

        // Verify within tolerance (+/- 2 due to double u8 rounding)
        for dy in 0..size {
            for dx in 0..size {
                let restored = original.get_pixel(pos_x + dx, pos_y + dy);
                let orig = original_copy.get_pixel(pos_x + dx, pos_y + dy);
                for ch in 0..3 {
                    let diff = (i32::from(restored[ch]) - i32::from(orig[ch])).abs();
                    assert!(
                        diff <= 2,
                        "Pixel ({dx},{dy}) ch {ch} diff {diff} (restored={}, orig={})",
                        restored[ch],
                        orig[ch]
                    );
                }
            }
        }
    }
}
