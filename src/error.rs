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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_messages() {
        let io_err = Error::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "gone"));
        assert!(io_err.to_string().contains("gone"));

        let unsupported = Error::UnsupportedFormat("tiff".to_string());
        assert!(unsupported.to_string().contains("tiff"));

        let too_small = Error::ImageTooSmall {
            width: 10,
            height: 20,
            wm_size: 48,
        };
        let msg = too_small.to_string();
        assert!(msg.contains("10x20"));
        assert!(msg.contains("48x48"));
    }
}
