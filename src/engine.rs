//! Core watermark removal engine.

use std::path::{Path, PathBuf};

use image::{DynamicImage, ImageFormat, RgbImage};

use crate::alpha_maps;
use crate::blending;
use crate::detection::{self, DetectionResult};
use crate::error::{Error, Result};

/// Watermark size classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatermarkSize {
    /// 48x48 watermark, 32px margin (images where either dimension <= 1024).
    Small,
    /// 96x96 watermark, 64px margin (images where both dimensions > 1024).
    Large,
}

/// Options controlling watermark processing behavior.
#[derive(Debug, Clone)]
pub struct ProcessOptions {
    /// Skip watermark detection, process unconditionally.
    pub force: bool,
    /// Detection confidence threshold (0.0-1.0).
    pub threshold: f32,
    /// Force a specific watermark size instead of auto-detecting.
    pub force_size: Option<WatermarkSize>,
    /// Enable verbose logging.
    pub verbose: bool,
    /// Suppress non-error output.
    pub quiet: bool,
}

impl Default for ProcessOptions {
    fn default() -> Self {
        Self {
            force: false,
            threshold: 0.25,
            force_size: None,
            verbose: false,
            quiet: false,
        }
    }
}

/// Result of processing a single image file.
#[derive(Debug)]
pub struct ProcessResult {
    /// Path of the processed file.
    pub path: PathBuf,
    /// Whether processing succeeded.
    pub success: bool,
    /// Whether the file was skipped (no watermark detected).
    pub skipped: bool,
    /// Detection confidence score.
    pub confidence: f32,
    /// Human-readable status message.
    pub message: String,
}

/// The watermark engine holding pre-computed alpha maps.
///
/// Create once with [`WatermarkEngine::new()`] and reuse for multiple images.
/// The engine decodes and caches the embedded alpha maps at initialization.
pub struct WatermarkEngine {
    alpha_map_small: Vec<f32>,
    alpha_map_large: Vec<f32>,
    logo_value: f32,
}

impl WatermarkEngine {
    /// Create a new engine from embedded PNG data.
    ///
    /// Decodes the 48x48 and 96x96 alpha maps and caches them for reuse.
    ///
    /// # Errors
    ///
    /// Returns [`Error::AlphaMapDecode`] if the embedded PNGs cannot be decoded.
    ///
    /// # Panics
    ///
    /// Panics if the embedded alpha map PNGs have unexpected dimensions (should never
    /// happen unless the binary data is corrupted).
    pub fn new() -> Result<Self> {
        let (alpha_small, w48, h48) = blending::calculate_alpha_map(alpha_maps::BG_48_PNG)?;
        assert_eq!(w48, 48, "Small alpha map must be 48x48");
        assert_eq!(h48, 48, "Small alpha map must be 48x48");

        let (alpha_large, w96, h96) = blending::calculate_alpha_map(alpha_maps::BG_96_PNG)?;
        assert_eq!(w96, 96, "Large alpha map must be 96x96");
        assert_eq!(h96, 96, "Large alpha map must be 96x96");

        Ok(Self {
            alpha_map_small: alpha_small,
            alpha_map_large: alpha_large,
            logo_value: 255.0,
        })
    }

    /// Determine watermark size based on image dimensions.
    ///
    /// - **Large** (96x96, 64px margin): both width AND height > 1024
    /// - **Small** (48x48, 32px margin): otherwise (including 1024x1024)
    #[must_use]
    #[allow(clippy::unused_self)] // method on `self` for API consistency
    pub fn watermark_size_for(&self, width: u32, height: u32) -> WatermarkSize {
        if width > 1024 && height > 1024 {
            WatermarkSize::Large
        } else {
            WatermarkSize::Small
        }
    }

    /// Get watermark config (size, margin, `alpha_map`) for given dimensions.
    fn config(
        &self,
        width: u32,
        height: u32,
        force_size: Option<WatermarkSize>,
    ) -> (u32, u32, &[f32]) {
        let size = force_size.unwrap_or_else(|| self.watermark_size_for(width, height));
        match size {
            WatermarkSize::Small => (48, 32, &self.alpha_map_small),
            WatermarkSize::Large => (96, 64, &self.alpha_map_large),
        }
    }

    /// Calculate watermark position (top-left corner of watermark region).
    #[allow(clippy::unused_self)]
    fn position(&self, img_w: u32, img_h: u32, wm_size: u32, margin: u32) -> (u32, u32) {
        let x = img_w.saturating_sub(wm_size + margin);
        let y = img_h.saturating_sub(wm_size + margin);
        (x, y)
    }

    /// Detect watermark in an image.
    ///
    /// Returns a [`DetectionResult`] with confidence scores from the
    /// three-stage detection algorithm.
    #[must_use]
    pub fn detect(&self, image: &RgbImage, opts: &ProcessOptions) -> DetectionResult {
        let (wm_size, margin, alpha_map) =
            self.config(image.width(), image.height(), opts.force_size);
        let (pos_x, pos_y) = self.position(image.width(), image.height(), wm_size, margin);

        detection::detect_watermark(
            image,
            alpha_map,
            wm_size,
            wm_size,
            pos_x,
            pos_y,
            opts.threshold,
        )
    }

    /// Remove watermark from an image in-place.
    ///
    /// Applies reverse alpha blending at the expected watermark position.
    /// The `force_size` parameter overrides automatic size detection.
    pub fn remove(&self, image: &mut RgbImage, force_size: Option<WatermarkSize>) {
        let (wm_size, margin, alpha_map) = self.config(image.width(), image.height(), force_size);
        let (pos_x, pos_y) = self.position(image.width(), image.height(), wm_size, margin);

        blending::remove_watermark_alpha_blend(
            image,
            alpha_map,
            wm_size,
            wm_size,
            pos_x,
            pos_y,
            self.logo_value,
        );
    }

    /// Process a single image file: load, detect, remove, save.
    ///
    /// Returns a [`ProcessResult`] indicating success, skip, or failure.
    #[must_use]
    pub fn process_file(
        &self,
        input: &Path,
        output: &Path,
        opts: &ProcessOptions,
    ) -> ProcessResult {
        let mut result = ProcessResult {
            path: input.to_path_buf(),
            success: false,
            skipped: false,
            confidence: 0.0,
            message: String::new(),
        };

        // Load image
        let dyn_img = match image::open(input) {
            Ok(img) => img,
            Err(e) => {
                result.message = format!("Failed to load: {e}");
                return result;
            }
        };

        let mut rgb_img = dyn_img.to_rgb8();
        let (w, h) = (rgb_img.width(), rgb_img.height());

        // Check image is large enough
        let (wm_size, margin, _) = self.config(w, h, opts.force_size);
        if w < wm_size + margin || h < wm_size + margin {
            result.skipped = true;
            result.success = true;
            result.message = format!("Image too small ({w}x{h}) for {wm_size}x{wm_size} watermark");
            return result;
        }

        // Detection (unless forced)
        if !opts.force {
            let detection = self.detect(&rgb_img, opts);
            result.confidence = detection.confidence;

            if !detection.detected && detection.confidence < opts.threshold {
                result.skipped = true;
                result.success = true;
                result.message = format!(
                    "No watermark detected ({:.0}% confidence, spatial={:.2}, grad={:.2}, var={:.2})",
                    detection.confidence * 100.0,
                    detection.spatial_score,
                    detection.gradient_score,
                    detection.variance_score,
                );
                return result;
            }
        }

        // Remove watermark
        self.remove(&mut rgb_img, opts.force_size);

        // Save output
        if let Some(parent) = output.parent() {
            if !parent.exists() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    result.message = format!("Failed to create output directory: {e}");
                    return result;
                }
            }
        }

        match save_image(&rgb_img, output) {
            Ok(()) => {
                result.success = true;
                result.message = "Watermark removed".to_string();
            }
            Err(e) => {
                result.message = format!("Failed to save: {e}");
            }
        }

        result
    }

    /// Process all supported images in a directory.
    ///
    /// Uses parallel iteration when the `cli` feature is enabled (via rayon).
    /// Returns a [`ProcessResult`] for each image found.
    ///
    /// # Panics
    ///
    /// Panics if any directory entry has no filename (should not happen for regular files).
    #[must_use]
    pub fn process_directory(
        &self,
        input_dir: &Path,
        output_dir: &Path,
        opts: &ProcessOptions,
    ) -> Vec<ProcessResult> {
        let entries: Vec<_> = match std::fs::read_dir(input_dir) {
            Ok(rd) => rd
                .filter_map(std::result::Result::ok)
                .filter(|e| e.file_type().map(|ft| ft.is_file()).unwrap_or(false))
                .filter(|e| is_supported_image(e.path().as_path()))
                .collect(),
            Err(e) => {
                return vec![ProcessResult {
                    path: input_dir.to_path_buf(),
                    success: false,
                    skipped: false,
                    confidence: 0.0,
                    message: format!("Failed to read directory: {e}"),
                }];
            }
        };

        // Create output directory
        if !output_dir.exists() {
            if let Err(e) = std::fs::create_dir_all(output_dir) {
                return vec![ProcessResult {
                    path: output_dir.to_path_buf(),
                    success: false,
                    skipped: false,
                    confidence: 0.0,
                    message: format!("Failed to create output directory: {e}"),
                }];
            }
        }

        #[cfg(feature = "cli")]
        {
            use rayon::prelude::*;
            entries
                .par_iter()
                .map(|entry| {
                    let input_path = entry.path();
                    let filename = input_path.file_name().unwrap();
                    let output_path = output_dir.join(filename);
                    self.process_file(&input_path, &output_path, opts)
                })
                .collect()
        }

        #[cfg(not(feature = "cli"))]
        {
            entries
                .iter()
                .map(|entry| {
                    let input_path = entry.path();
                    let filename = input_path.file_name().unwrap();
                    let output_path = output_dir.join(filename);
                    self.process_file(&input_path, &output_path, opts)
                })
                .collect()
        }
    }
}

/// Check if a file has a supported image extension.
#[must_use]
pub fn is_supported_image(path: &Path) -> bool {
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => matches!(
            ext.to_lowercase().as_str(),
            "jpg" | "jpeg" | "png" | "webp" | "bmp"
        ),
        None => false,
    }
}

/// Save an RGB image with format-specific quality settings.
///
/// # Errors
///
/// Returns an error if the format is unsupported or writing fails.
pub fn save_image(img: &RgbImage, path: &Path) -> Result<()> {
    let format =
        ImageFormat::from_path(path).map_err(|e| Error::UnsupportedFormat(e.to_string()))?;

    let dyn_img = DynamicImage::ImageRgb8(img.clone());

    match format {
        ImageFormat::Jpeg => {
            let file = std::fs::File::create(path)?;
            let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(file, 100);
            encoder.encode_image(&dyn_img)?;
        }
        ImageFormat::Png | ImageFormat::WebP | ImageFormat::Bmp => {
            dyn_img.save(path)?;
        }
        _ => {
            return Err(Error::UnsupportedFormat(format!("{format:?}")));
        }
    }

    Ok(())
}

/// Generate a default output path from an input path.
///
/// Example: `"photo.jpg"` becomes `"photo_cleaned.jpg"`.
#[must_use]
pub fn default_output_path(input: &Path) -> PathBuf {
    let stem = input.file_stem().unwrap_or_default().to_string_lossy();
    let ext = input.extension().unwrap_or_default().to_string_lossy();
    let parent = input.parent().unwrap_or(Path::new("."));
    parent.join(format!("{stem}_cleaned.{ext}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watermark_size_small_when_either_dim_lte_1024() {
        let engine = WatermarkEngine::new().unwrap();
        assert_eq!(engine.watermark_size_for(800, 600), WatermarkSize::Small);
        assert_eq!(engine.watermark_size_for(1024, 1024), WatermarkSize::Small);
        assert_eq!(engine.watermark_size_for(2048, 512), WatermarkSize::Small);
        assert_eq!(engine.watermark_size_for(512, 2048), WatermarkSize::Small);
    }

    #[test]
    fn watermark_size_large_when_both_dims_gt_1024() {
        let engine = WatermarkEngine::new().unwrap();
        assert_eq!(engine.watermark_size_for(1025, 1025), WatermarkSize::Large);
        assert_eq!(engine.watermark_size_for(2048, 2048), WatermarkSize::Large);
    }

    #[test]
    fn default_output_path_appends_cleaned_suffix() {
        let p = default_output_path(Path::new("/tmp/photo.jpg"));
        assert_eq!(p, PathBuf::from("/tmp/photo_cleaned.jpg"));

        let p = default_output_path(Path::new("image.png"));
        assert_eq!(
            p.file_name().unwrap().to_str().unwrap(),
            "image_cleaned.png"
        );
    }

    #[test]
    fn is_supported_image_accepts_common_formats() {
        assert!(is_supported_image(Path::new("photo.jpg")));
        assert!(is_supported_image(Path::new("photo.JPEG")));
        assert!(is_supported_image(Path::new("photo.png")));
        assert!(is_supported_image(Path::new("photo.webp")));
        assert!(is_supported_image(Path::new("photo.bmp")));
    }

    #[test]
    fn is_supported_image_rejects_unsupported_formats() {
        assert!(!is_supported_image(Path::new("photo.gif")));
        assert!(!is_supported_image(Path::new("photo.txt")));
        assert!(!is_supported_image(Path::new("photo")));
    }
}
