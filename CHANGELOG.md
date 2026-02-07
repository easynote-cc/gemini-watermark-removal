# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1] - 2025-02-07

### Added

- 20 new unit tests (42 total), ~79% library line coverage
- Codecov coverage integration with cargo-llvm-cov
- README badges (CI, crates.io, docs.rs, codecov, license)
- MSRV (1.70) verification in CI
- cargo-deny supply chain security audit in CI
- Weekly scheduled CI builds
- Dependabot for automated dependency updates
- Issue templates (bug report, feature request) and PR template
- CONTRIBUTING.md, CHANGELOG.md, rustfmt.toml, deny.toml
- docs.rs metadata and Cargo.toml include list

## [0.1.0] - 2025-02-07

### Added

- Initial release
- Reverse alpha blending watermark removal for Gemini AI images
- Three-stage watermark detection: Spatial NCC, Gradient NCC, Variance Analysis
- Embedded 48x48 and 96x96 alpha maps for watermark matching
- CLI with single-file and batch directory processing
- Cross-platform support (macOS, Linux, Windows)
- Library API (`WatermarkEngine`, `ProcessOptions`, `ProcessResult`)

[0.1.1]: https://github.com/easynote-cc/gemini-watermark-removal/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/easynote-cc/gemini-watermark-removal/releases/tag/v0.1.0
