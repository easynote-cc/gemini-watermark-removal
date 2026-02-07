# gemini-watermark-removal

[![CI](https://github.com/easynote-cc/gemini-watermark-removal/actions/workflows/ci.yml/badge.svg)](https://github.com/easynote-cc/gemini-watermark-removal/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/gemini-watermark-removal.svg)](https://crates.io/crates/gemini-watermark-removal)
[![docs.rs](https://docs.rs/gemini-watermark-removal/badge.svg)](https://docs.rs/gemini-watermark-removal)
[![codecov](https://codecov.io/gh/easynote-cc/gemini-watermark-removal/graph/badge.svg)](https://codecov.io/gh/easynote-cc/gemini-watermark-removal)
[![License](https://img.shields.io/crates/l/gemini-watermark-removal.svg)](https://github.com/easynote-cc/gemini-watermark-removal#license)

Remove visible Gemini AI watermarks from images via reverse alpha blending.

## Overview

Gemini AI overlays a semi-transparent star/sparkle logo on generated images. This crate reverses the alpha-blending equation to recover the original pixels, using calibrated 48x48 and 96x96 alpha masks.

**Note:** This tool only removes the *visible* watermark. It cannot remove SynthID (Google's invisible watermark).

## Library Usage

```rust
use gemini_watermark_removal::{WatermarkEngine, ProcessOptions};

let engine = WatermarkEngine::new().expect("failed to init engine");

// Detect
let img = image::open("photo.jpg").unwrap().to_rgb8();
let opts = ProcessOptions::default();
let detection = engine.detect(&img, &opts);
println!("Confidence: {:.0}%", detection.confidence * 100.0);

// Remove
let mut img = image::open("photo.jpg").unwrap().to_rgb8();
engine.remove(&mut img, None);
img.save("cleaned.jpg").unwrap();
```

## CLI Usage

```bash
# Install
cargo install gemini-watermark-removal

# Single image
gemini-watermark photo.jpg -o cleaned.jpg

# Batch directory
gemini-watermark ./input/ -o ./output/

# Force removal (skip detection)
gemini-watermark photo.jpg -o cleaned.jpg --force

# Verbose output
gemini-watermark photo.jpg -o cleaned.jpg -v
```

## How It Works

**Forward (Gemini applies):**
```
watermarked = alpha * 255 + (1 - alpha) * original
```

**Reverse (we recover):**
```
original = (watermarked - alpha * 255) / (1 - alpha)
```

Detection uses a three-stage weighted ensemble:
- Spatial NCC (50%) - pattern correlation with alpha map
- Gradient NCC (30%) - edge signature matching
- Variance Analysis (20%) - texture dampening detection

## Minimum Supported Rust Version (MSRV)

The minimum supported Rust version is **1.85**.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

Licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.
