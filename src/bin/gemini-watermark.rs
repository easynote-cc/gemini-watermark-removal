use std::path::{Path, PathBuf};
use std::process;

use clap::Parser;

use gemini_watermark_removal::{
    default_output_path, ProcessOptions, ProcessResult, WatermarkEngine, WatermarkSize,
};

#[derive(Parser)]
#[command(
    name = "gemini-watermark",
    about = "Remove visible Gemini AI watermarks via reverse alpha blending",
    version,
    after_help = "Simple usage: gemini-watermark <image>  (auto-detect and remove in-place)\n\n\
                  NOTE: This tool only removes the VISIBLE Gemini watermark (star/sparkle logo).\n\
                  It cannot remove SynthID (invisible watermark)."
)]
#[allow(clippy::struct_excessive_bools)]
struct Cli {
    /// Input image file or directory
    input: String,

    /// Output file or directory (default: {name}_cleaned.{ext})
    #[arg(short, long)]
    output: Option<String>,

    /// Skip watermark detection, process unconditionally
    #[arg(short, long)]
    force: bool,

    /// Detection confidence threshold (0.0-1.0)
    #[arg(short, long, default_value = "0.25")]
    threshold: f32,

    /// Force 48x48 watermark size (for images <= 1024px)
    #[arg(long)]
    force_small: bool,

    /// Force 96x96 watermark size (for images > 1024px)
    #[arg(long)]
    force_large: bool,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Suppress all non-error output
    #[arg(short, long)]
    quiet: bool,
}

fn main() {
    let cli = Cli::parse();

    if cli.force_small && cli.force_large {
        eprintln!("Error: Cannot specify both --force-small and --force-large");
        process::exit(1);
    }

    if !(0.0..=1.0).contains(&cli.threshold) {
        eprintln!("Error: Threshold must be between 0.0 and 1.0");
        process::exit(1);
    }

    let force_size = if cli.force_small {
        Some(WatermarkSize::Small)
    } else if cli.force_large {
        Some(WatermarkSize::Large)
    } else {
        None
    };

    let opts = ProcessOptions {
        force: cli.force,
        threshold: cli.threshold,
        force_size,
        verbose: cli.verbose,
        quiet: cli.quiet,
    };

    let engine = match WatermarkEngine::new() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Fatal: Failed to initialize engine: {e}");
            process::exit(1);
        }
    };

    let input_path = Path::new(&cli.input);
    if !input_path.exists() {
        eprintln!("Error: Input path does not exist: {}", cli.input);
        process::exit(1);
    }

    if !opts.quiet {
        if opts.force {
            eprintln!("WARNING: Force mode - processing ALL images without detection!");
        } else {
            eprintln!(
                "Auto-detection enabled (threshold: {:.0}%)",
                opts.threshold * 100.0
            );
        }
        eprintln!();
    }

    let results = if input_path.is_dir() {
        let output_dir = if let Some(o) = &cli.output {
            PathBuf::from(o)
        } else {
            eprintln!("Error: Output directory is required for batch processing");
            eprintln!("Usage: gemini-watermark <input_dir> -o <output_dir>");
            process::exit(1);
        };
        engine.process_directory(input_path, &output_dir, &opts)
    } else {
        let output_path = match &cli.output {
            Some(o) => PathBuf::from(o),
            None => default_output_path(input_path),
        };
        vec![engine.process_file(input_path, &output_path, &opts)]
    };

    let mut success_count = 0u32;
    let mut skip_count = 0u32;
    let mut fail_count = 0u32;

    for r in &results {
        print_result(r, &opts);
        if r.skipped {
            skip_count += 1;
        } else if r.success {
            success_count += 1;
        } else {
            fail_count += 1;
        }
    }

    if results.len() > 1 && !opts.quiet {
        eprintln!();
        eprint!("[Summary] Processed: {success_count}");
        if skip_count > 0 {
            eprint!(", Skipped: {skip_count}");
        }
        if fail_count > 0 {
            eprint!(", Failed: {fail_count}");
        }
        eprintln!(" (Total: {})", results.len());
    }

    if fail_count > 0 {
        process::exit(1);
    }
}

fn print_result(result: &ProcessResult, opts: &ProcessOptions) {
    if opts.quiet && result.success {
        return;
    }

    let filename = result.path.file_name().map_or_else(
        || result.path.display().to_string(),
        |f| f.to_string_lossy().to_string(),
    );

    if result.skipped {
        if !opts.quiet {
            eprintln!("[SKIP] {filename}: {}", result.message);
        }
    } else if result.success {
        if !opts.quiet {
            if result.confidence > 0.0 {
                eprintln!(
                    "[OK] {filename} ({:.0}% confidence)",
                    result.confidence * 100.0
                );
            } else {
                eprintln!("[OK] {filename}");
            }
        }
    } else {
        eprintln!("[FAIL] {filename}: {}", result.message);
    }

    if opts.verbose && !result.message.is_empty() {
        eprintln!("  -> {}", result.message);
    }
}
