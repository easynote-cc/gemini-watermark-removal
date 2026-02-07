//! Error types for the gemini-watermark-removal crate.

/// Errors that can occur during watermark detection and removal.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Failed to decode an embedded alpha map PNG.
    #[error("failed to decode alpha map PNG: {0}")]
    AlphaMapDecode(image::ImageError),

    /// The image is too small to contain a watermark at the expected position.
    #[error("image too small ({width}x{height}) for {wm_size}x{wm_size} watermark")]
    ImageTooSmall {
        /// Image width in pixels.
        width: u32,
        /// Image height in pixels.
        height: u32,
        /// Expected watermark size in pixels.
        wm_size: u32,
    },

    /// An I/O error occurred while reading or writing files.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// The image format is not supported.
    #[error("unsupported image format: {0}")]
    UnsupportedFormat(String),

    /// An error occurred during image processing (load, save, encode).
    #[error("image processing error: {0}")]
    Image(#[from] image::ImageError),
}

/// A specialized `Result` type for this crate.
pub type Result<T> = std::result::Result<T, Error>;
