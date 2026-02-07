# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2025-02-07

### Added

- Initial release
- Reverse alpha blending watermark removal for Gemini AI images
- Three-stage watermark detection: Spatial NCC, Gradient NCC, Variance Analysis
- Embedded 48x48 and 96x96 alpha maps for watermark matching
- CLI with single-file and batch directory processing
- Cross-platform support (macOS, Linux, Windows)
- Library API (`WatermarkEngine`, `ProcessOptions`, `ProcessResult`)

[0.1.0]: https://github.com/easynote-cc/gemini-watermark-removal/releases/tag/v0.1.0
