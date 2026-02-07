# Contributing to gemini-watermark-removal

Thank you for your interest in contributing! This document provides guidelines
and information for contributors.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/YOUR_USERNAME/gemini-watermark-removal.git`
3. Create a branch: `git checkout -b my-feature`
4. Make your changes
5. Submit a pull request

## Development Setup

```bash
# Ensure you have Rust 1.70+ installed
rustup update stable

# Build
cargo build --all-features

# Run tests
cargo test --all-features

# Run clippy lints
cargo clippy --all-targets --all-features -- -D warnings

# Format check
cargo fmt --all -- --check
```

## Pull Request Guidelines

- Keep PRs focused on a single change
- Add tests for new functionality
- Ensure all CI checks pass (clippy, fmt, tests)
- Update CHANGELOG.md for user-facing changes
- Follow existing code style and conventions

## Running Coverage Locally

```bash
cargo install cargo-llvm-cov
cargo llvm-cov --all-features --html
open target/llvm-cov/html/index.html
```

## Reporting Issues

- Use the GitHub issue templates when available
- Include Rust version (`rustc --version`) and OS information
- Provide a minimal reproduction case if possible

## License

By contributing, you agree that your contributions will be licensed under the
same dual license as the project: MIT OR Apache-2.0.
