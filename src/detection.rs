//! Three-stage watermark detection algorithm.
//!
//! Detects the presence of a Gemini watermark using a weighted ensemble:
//! 1. **Spatial NCC** (50%): normalized cross-correlation with the alpha map
//! 2. **Gradient NCC** (30%): edge signature matching via Sobel operators
//! 3. **Variance Analysis** (20%): texture dampening detection

use image::RgbImage;

/// Detection weight: spatial NCC.
const SPATIAL_WEIGHT: f32 = 0.50;
/// Detection weight: gradient NCC.
const GRADIENT_WEIGHT: f32 = 0.30;
/// Detection weight: variance analysis.
const VARIANCE_WEIGHT: f32 = 0.20;
/// Circuit breaker: if spatial NCC < this, reject early.
const SPATIAL_CIRCUIT_BREAKER: f32 = 0.25;
/// Internal detection threshold for declaring "detected".
const DETECTION_THRESHOLD: f32 = 0.35;
/// Minimum reference region height for variance analysis.
const MIN_REF_HEIGHT: u32 = 8;
/// Minimum reference stddev to compute variance score (in normalized [0,1] space).
const MIN_REF_STDDEV: f32 = 5.0 / 255.0;

/// Result of watermark detection.
#[derive(Debug, Clone)]
pub struct DetectionResult {
    /// Whether a watermark was detected above the confidence threshold.
    pub detected: bool,
    /// Overall confidence score in `[0, 1]`.
    pub confidence: f32,
    /// Stage 1: spatial NCC score.
    pub spatial_score: f32,
    /// Stage 2: gradient NCC score.
    pub gradient_score: f32,
    /// Stage 3: variance analysis score.
    pub variance_score: f32,
}

impl Default for DetectionResult {
    fn default() -> Self {
        Self {
            detected: false,
            confidence: 0.0,
            spatial_score: 0.0,
            gradient_score: 0.0,
            variance_score: 0.0,
        }
    }
}

/// Convert an RGB image region to grayscale float values in `[0, 1]`.
///
/// Uses luminance formula: `0.299*R + 0.587*G + 0.114*B`.
fn region_to_grayscale(img: &RgbImage, x: u32, y: u32, w: u32, h: u32) -> Vec<f32> {
    let mut gray = Vec::with_capacity((w * h) as usize);
    for dy in 0..h {
        for dx in 0..w {
            let px = img.get_pixel(x + dx, y + dy);
            let lum =
                0.299 * f32::from(px[0]) + 0.587 * f32::from(px[1]) + 0.114 * f32::from(px[2]);
            gray.push(lum / 255.0);
        }
    }
    gray
}

/// Normalized Cross-Correlation between two equal-length float slices.
///
/// `NCC = sum((a-mean_a)*(b-mean_b)) / sqrt(sum((a-mean_a)^2) * sum((b-mean_b)^2))`
fn ncc(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len());
    #[allow(clippy::cast_precision_loss)]
    let n = a.len() as f32;
    if n < 1.0 {
        return 0.0;
    }

    let mean_a = a.iter().sum::<f32>() / n;
    let mean_b = b.iter().sum::<f32>() / n;

    let mut numerator = 0.0_f32;
    let mut denom_a = 0.0_f32;
    let mut denom_b = 0.0_f32;

    for (va, vb) in a.iter().zip(b.iter()) {
        let da = va - mean_a;
        let db = vb - mean_b;
        numerator += da * db;
        denom_a += da * da;
        denom_b += db * db;
    }

    let denom = (denom_a * denom_b).sqrt();
    if denom < 1e-10 {
        0.0
    } else {
        numerator / denom
    }
}

/// Compute Sobel gradient magnitude for a 2D float array.
///
/// Uses 3x3 Sobel kernels. Border pixels are set to 0.
fn sobel_magnitude(data: &[f32], width: usize, height: usize) -> Vec<f32> {
    let mut result = vec![0.0_f32; width * height];

    for y in 1..height - 1 {
        for x in 1..width - 1 {
            // Safety: y >= 1 and x >= 1, dy/dx in {-1, 0, 1}, so indices are always valid.
            #[allow(clippy::cast_sign_loss, clippy::cast_possible_wrap)]
            let idx = |dy: isize, dx: isize| -> f32 {
                data[((y as isize + dy) as usize) * width + (x as isize + dx) as usize]
            };

            let gx = -idx(-1, -1) + idx(-1, 1) - 2.0 * idx(0, -1) + 2.0 * idx(0, 1) - idx(1, -1)
                + idx(1, 1);

            let gy = -idx(-1, -1) - 2.0 * idx(-1, 0) - idx(-1, 1)
                + idx(1, -1)
                + 2.0 * idx(1, 0)
                + idx(1, 1);

            result[y * width + x] = (gx * gx + gy * gy).sqrt();
        }
    }

    result
}

/// Compute standard deviation of a float slice.
fn stddev(data: &[f32]) -> f32 {
    if data.is_empty() {
        return 0.0;
    }
    #[allow(clippy::cast_precision_loss)]
    let n = data.len() as f32;
    let mean = data.iter().sum::<f32>() / n;
    let variance = data.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / n;
    variance.sqrt()
}

/// Detect whether a Gemini watermark is present at the given position.
///
/// Uses a three-stage weighted ensemble:
/// 1. **Spatial NCC** (50%): correlation between region brightness and alpha map
/// 2. **Gradient NCC** (30%): edge signature matching via Sobel operators
/// 3. **Variance Analysis** (20%): texture dampening detection
///
/// # Arguments
///
/// * `image` - The image to analyze.
/// * `alpha_map` - Flat alpha map array of size `wm_width * wm_height`.
/// * `wm_width` - Width of the watermark region.
/// * `wm_height` - Height of the watermark region.
/// * `pos_x` - X coordinate of the watermark's top-left corner.
/// * `pos_y` - Y coordinate of the watermark's top-left corner.
/// * `user_threshold` - User-specified threshold for the spatial circuit breaker.
#[must_use]
pub fn detect_watermark(
    image: &RgbImage,
    alpha_map: &[f32],
    wm_width: u32,
    wm_height: u32,
    pos_x: u32,
    pos_y: u32,
    user_threshold: f32,
) -> DetectionResult {
    let mut result = DetectionResult::default();

    let img_w = image.width();
    let img_h = image.height();

    // Clip ROI to image bounds
    let x2 = (pos_x + wm_width).min(img_w);
    let y2 = (pos_y + wm_height).min(img_h);
    if pos_x >= x2 || pos_y >= y2 {
        return result;
    }

    let roi_w = x2 - pos_x;
    let roi_h = y2 - pos_y;

    // Extract grayscale region
    let gray_region = region_to_grayscale(image, pos_x, pos_y, roi_w, roi_h);

    // Get corresponding alpha sub-region (in case of clipping)
    let alpha_region: Vec<f32> = if roi_w == wm_width && roi_h == wm_height {
        alpha_map.to_vec()
    } else {
        let mut sub = Vec::with_capacity((roi_w * roi_h) as usize);
        for dy in 0..roi_h {
            for dx in 0..roi_w {
                sub.push(alpha_map[(dy * wm_width + dx) as usize]);
            }
        }
        sub
    };

    // Stage 1: Spatial NCC
    let spatial_score = ncc(&gray_region, &alpha_region).max(0.0);
    result.spatial_score = spatial_score;

    // Circuit breaker
    let breaker = user_threshold.min(SPATIAL_CIRCUIT_BREAKER);
    if spatial_score < breaker {
        result.confidence = spatial_score * 0.5;
        return result;
    }

    // Stage 2: Gradient NCC
    let w = roi_w as usize;
    let h = roi_h as usize;
    let img_grad = sobel_magnitude(&gray_region, w, h);
    let alpha_grad = sobel_magnitude(&alpha_region, w, h);
    let gradient_score = ncc(&img_grad, &alpha_grad).max(0.0);
    result.gradient_score = gradient_score;

    // Stage 3: Variance Analysis
    let mut variance_score = 0.0_f32;

    // Use region above watermark as reference
    let ref_h = pos_y.min(wm_height).min(img_h.saturating_sub(pos_y));
    if ref_h > MIN_REF_HEIGHT && pos_y >= ref_h {
        let ref_region = region_to_grayscale(image, pos_x, pos_y - ref_h, roi_w, ref_h);
        let wm_stddev = stddev(&gray_region);
        let ref_stddev = stddev(&ref_region);

        if ref_stddev > MIN_REF_STDDEV {
            variance_score = (1.0 - wm_stddev / ref_stddev).clamp(0.0, 1.0);
        }
    }
    result.variance_score = variance_score;

    // Weighted ensemble
    let confidence = SPATIAL_WEIGHT * spatial_score
        + GRADIENT_WEIGHT * gradient_score
        + VARIANCE_WEIGHT * variance_score;

    result.confidence = confidence.clamp(0.0, 1.0);
    result.detected = result.confidence >= DETECTION_THRESHOLD;

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ncc_returns_one_for_perfect_match() {
        let a = vec![0.1, 0.5, 0.9, 0.3, 0.7];
        let score = ncc(&a, &a);
        assert!(
            (score - 1.0).abs() < 1e-5,
            "Perfect match should give NCC ~1.0, got {score}"
        );
    }

    #[test]
    fn ncc_returns_negative_one_for_inverse() {
        let a = vec![0.1, 0.5, 0.9, 0.3, 0.7];
        let b: Vec<f32> = a.iter().map(|v| 1.0 - v).collect();
        let score = ncc(&a, &b);
        assert!(
            (score + 1.0).abs() < 1e-5,
            "Inverse should give NCC ~-1.0, got {score}"
        );
    }

    #[test]
    fn ncc_returns_low_for_uncorrelated() {
        let a = vec![1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0];
        let b = vec![1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0];
        let score = ncc(&a, &b);
        assert!(
            score.abs() < 0.5,
            "Weakly correlated signals should give low NCC, got {score}"
        );
    }

    #[test]
    fn ncc_of_empty_slices_is_zero() {
        let score = ncc(&[], &[]);
        assert!(
            score.abs() < 1e-6,
            "NCC of empty slices should be 0, got {score}"
        );
    }

    #[test]
    fn stddev_of_empty_slice_is_zero() {
        assert!(stddev(&[]).abs() < 1e-6);
    }

    #[test]
    fn stddev_of_constant_values_is_zero() {
        let data = vec![0.42; 100];
        assert!(
            stddev(&data).abs() < 1e-6,
            "Constant values should have stddev 0"
        );
    }

    #[test]
    fn stddev_of_known_values() {
        // stddev of [1, 2, 3, 4, 5] = sqrt(2.0) ≈ 1.4142
        let data: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let sd = stddev(&data);
        let expected = 2.0_f32.sqrt();
        assert!(
            (sd - expected).abs() < 1e-5,
            "Expected stddev ~{expected}, got {sd}"
        );
    }

    #[test]
    fn detect_returns_result_for_small_image_with_clipping() {
        // Image smaller than watermark region — should not panic, ROI gets clipped
        let img = RgbImage::new(20, 20);
        let alpha_map = vec![0.3; 48 * 48];
        // Watermark 48x48 placed at (0,0) on a 20x20 image — heavy clipping
        let result = detect_watermark(&img, &alpha_map, 48, 48, 0, 0, 0.25);
        // Should run without panic, confidence should be low for blank image
        assert!(!result.detected);
    }

    #[test]
    fn detect_circuit_breaker_rejects_low_spatial() {
        // Uniform image should have near-zero spatial NCC with any alpha pattern
        let img = RgbImage::new(100, 100); // all black
        #[allow(clippy::cast_precision_loss)]
        let alpha_map: Vec<f32> = (0..48 * 48).map(|i| (i % 5) as f32 / 10.0).collect();

        let result = detect_watermark(&img, &alpha_map, 48, 48, 20, 20, 0.25);

        // Circuit breaker should trigger: gradient and variance stay 0
        assert!(result.gradient_score.abs() < f32::EPSILON);
        assert!(result.variance_score.abs() < f32::EPSILON);
        assert!(!result.detected);
    }

    #[test]
    fn sobel_returns_zero_for_flat_image() {
        let data = vec![0.5_f32; 10 * 10];
        let grad = sobel_magnitude(&data, 10, 10);
        for &g in &grad {
            assert!(g.abs() < 1e-6, "Flat image should have zero gradient");
        }
    }

    #[test]
    fn sobel_detects_vertical_edge() {
        let mut data = vec![0.0_f32; 10 * 10];
        for y in 0..10 {
            for x in 5..10 {
                data[y * 10 + x] = 1.0;
            }
        }
        let grad = sobel_magnitude(&data, 10, 10);
        let center_grad = grad[5 * 10 + 5];
        assert!(
            center_grad > 0.1,
            "Edge should produce non-zero gradient, got {center_grad}"
        );
    }
}
