use gemini_watermark_removal::{ProcessOptions, WatermarkEngine, WatermarkSize};
use image::RgbImage;

#[test]
fn engine_initializes_successfully() {
    let engine = WatermarkEngine::new();
    assert!(engine.is_ok());
}

#[test]
fn detect_returns_low_confidence_for_blank_image() {
    let engine = WatermarkEngine::new().unwrap();
    let img = RgbImage::new(200, 200);
    let opts = ProcessOptions::default();
    let result = engine.detect(&img, &opts);

    assert!(!result.detected);
    assert!(result.confidence < 0.1);
}

#[test]
fn detect_returns_low_confidence_for_large_blank_image() {
    let engine = WatermarkEngine::new().unwrap();
    let img = RgbImage::new(2048, 2048);
    let opts = ProcessOptions::default();
    let result = engine.detect(&img, &opts);

    assert!(!result.detected);
}

#[test]
fn remove_does_not_crash_on_blank_image() {
    let engine = WatermarkEngine::new().unwrap();
    let mut img = RgbImage::new(200, 200);
    engine.remove(&mut img, None);
    // Should not panic
}

#[test]
fn remove_does_not_crash_on_large_blank_image() {
    let engine = WatermarkEngine::new().unwrap();
    let mut img = RgbImage::new(2048, 2048);
    engine.remove(&mut img, Some(WatermarkSize::Large));
}

#[test]
fn watermark_size_selection_matches_spec() {
    let engine = WatermarkEngine::new().unwrap();

    // Small when either dimension <= 1024
    assert_eq!(engine.watermark_size_for(800, 600), WatermarkSize::Small);
    assert_eq!(engine.watermark_size_for(1024, 1024), WatermarkSize::Small);
    assert_eq!(engine.watermark_size_for(2048, 512), WatermarkSize::Small);

    // Large when both > 1024
    assert_eq!(engine.watermark_size_for(1025, 1025), WatermarkSize::Large);
    assert_eq!(engine.watermark_size_for(4096, 4096), WatermarkSize::Large);
}

#[test]
fn force_size_overrides_detection() {
    let engine = WatermarkEngine::new().unwrap();
    let img = RgbImage::new(200, 200);

    let opts = ProcessOptions {
        force_size: Some(WatermarkSize::Small),
        ..ProcessOptions::default()
    };

    let result = engine.detect(&img, &opts);
    // Should run without error even though image is small
    assert!(!result.detected);
}
