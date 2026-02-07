//! Remove Gemini watermark from a single image.
//!
//! Usage:
//! ```sh
//! cargo run --example remove_watermark -- input.jpg output.jpg
//! ```

use std::env;
use std::process;

use gemini_watermark_removal::{ProcessOptions, WatermarkEngine};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <input> <output>", args[0]);
        process::exit(1);
    }

    let input = &args[1];
    let output = &args[2];

    let engine = WatermarkEngine::new().expect("failed to initialize engine");
    let opts = ProcessOptions::default();
    let result = engine.process_file(input.as_ref(), output.as_ref(), &opts);

    if result.skipped {
        println!("Skipped: {}", result.message);
    } else if result.success {
        println!("Done: {}", result.message);
    } else {
        eprintln!("Error: {}", result.message);
        process::exit(1);
    }
}
