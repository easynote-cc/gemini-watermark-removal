//! Remove visible Gemini AI watermarks via reverse alpha blending.
//!
//! Gemini AI overlays a semi-transparent star/sparkle logo on generated images.
//! This crate reverses the alpha-blending equation to recover the original pixels,
//! using calibrated 48x48 and 96x96 alpha masks embedded in the binary.
//!
//! # Quick Start
//!
//! ```no_run
//! use gemini_watermark_removal::{WatermarkEngine, ProcessOptions};
//!
//! let engine = WatermarkEngine::new().expect("failed to init engine");
//! let mut img = image::open("photo.jpg").unwrap().to_rgb8();
//! engine.remove(&mut img, None);
//! img.save("cleaned.jpg").unwrap();
//! ```
//!
//! # Detection
//!
//! Before removal, a three-stage detection algorithm checks whether a watermark
//! is present (spatial NCC, gradient NCC, variance analysis). Images without
//! detected watermarks can be automatically skipped to protect originals.
//!
//! ```no_run
//! use gemini_watermark_removal::{WatermarkEngine, ProcessOptions};
//!
//! let engine = WatermarkEngine::new().expect("failed to init engine");
//! let img = image::open("photo.jpg").unwrap().to_rgb8();
//! let opts = ProcessOptions::default();
//! let result = engine.detect(&img, &opts);
//! println!("Detected: {}, confidence: {:.0}%", result.detected, result.confidence * 100.0);
//! ```

#![deny(missing_docs)]

mod alpha_maps;
pub mod blending;
pub mod detection;
mod engine;
pub mod error;

pub use engine::{
    default_output_path, is_supported_image, save_image, ProcessOptions, ProcessResult,
    WatermarkEngine, WatermarkSize,
};
pub use error::{Error, Result};
