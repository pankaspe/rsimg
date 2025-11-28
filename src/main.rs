// src/main.rs
//
// Main entry point for RSIMG — a Rust-powered parallel image optimizer.
// Handles argument parsing, validation, and orchestrates image processing.

mod processor;

use anyhow::{Context, Result};
use clap::Parser;
use indicatif::MultiProgress;
use owo_colors::OwoColorize;
use std::path::PathBuf;
use walkdir::WalkDir;

// CLI arguments structure using clap
#[derive(Parser)]
#[command(
    name = "rsimg",
    author = "Andrea JB <mez.tnt@gmail.com>",
    version,
    about = "Rust-powered parallel image optimizer",
    long_about = "Fast, parallel image optimizer that resizes and converts images to multiple formats.\nSupports JPG, PNG, WebP with real-time progress visualization.",
    after_help = "EXAMPLES:\n    \
                  rsimg photo.jpg\n    \
                  rsimg ./photos --output ./optimized --recursive\n    \
                  rsimg ./images --formats webp,jpg --scales 100,75,50 --quality 85\n    \
                  rsimg ./gallery --threads 4 -r\n\n\
                  For more information, visit: https://github.com/yourusername/rsimg"
)]
struct Args {
    /// File or folder to process
    #[arg(value_name = "INPUT", help = "Input file or directory")]
    input: PathBuf,

    /// Output formats (comma-separated: jpg,webp,png)
    #[arg(
        long,
        value_delimiter = ',',
        default_values_t = vec!["jpg".to_string(), "webp".to_string()],
        value_name = "FORMATS",
        help = "Output image formats"
    )]
    formats: Vec<String>,

    /// Scale percentages (comma-separated: 100,75,50,25)
    #[arg(
        long,
        value_delimiter = ',',
        default_values_t = vec![75, 50, 25],
        value_name = "SCALES",
        help = "Image scale percentages (10-100)"
    )]
    scales: Vec<u32>,

    /// Compression quality (0-100, higher is better)
    #[arg(
        long,
        default_value_t = 80,
        value_name = "QUALITY",
        help = "JPEG/WebP quality level"
    )]
    quality: u8,

    /// Process subdirectories recursively
    #[arg(
        short,
        long,
        default_value_t = false,
        help = "Scan directories recursively"
    )]
    recursive: bool,

    /// Number of parallel threads (default: auto-detect CPU cores)
    #[arg(short, long, value_name = "N", help = "Number of threads to use")]
    threads: Option<usize>,

    /// Output directory for optimized images (default: same as input)
    #[arg(short, long, value_name = "DIR", help = "Output directory path")]
    output: Option<PathBuf>,
}

fn main() -> Result<()> {
    // Parse CLI arguments
    let args = Args::parse();

    // Clear terminal screen
    print!("\x1B[2J\x1B[1;1H");

    // Configure Rayon thread pool if user specified a thread count
    if let Some(threads) = args.threads {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build_global()
            .context("Failed to configure thread pool")?;
    }

    // Print header with styling
    println!("{}", "\n=== RSIMG — Image Optimizer ===\n".bold().cyan());

    // Validate quality parameter
    if args.quality > 100 {
        anyhow::bail!("Quality must be between 0 and 100");
    }

    // Validate scale percentages
    for scale in &args.scales {
        if *scale < 10 || *scale > 100 {
            anyhow::bail!("Scales must be between 10 and 100 ({}% is invalid)", scale);
        }
    }

    // Collect all valid image files based on input path
    let files = collect_image_files(&args)?;

    if files.is_empty() {
        println!("{}", "No valid images found.".red());
        return Ok(());
    }

    // Create output directory if user specified one
    if let Some(ref output_dir) = args.output {
        std::fs::create_dir_all(output_dir).with_context(|| {
            format!(
                "Failed to create output directory: {}",
                output_dir.display()
            )
        })?;
    }

    let total_files = files.len(); // Save total number of files for later display

    // Print summary of files found
    println!(
        "  {} {} {}",
        "📁".bright_blue(),
        "Found".bright_white(),
        format!("{} images", total_files).bright_cyan().bold()
    );

    // Display output directory info if specified
    if let Some(ref output_dir) = args.output {
        println!(
            "  {} Output: {}/",
            "💾".bright_white(),
            output_dir.display().to_string().bright_yellow()
        );
    }

    // Display formats, scales, and quality settings
    println!(
        "  {} Formats: {} | Scales: {} | Quality: {}",
        "⚙️ ".bright_white(),
        args.formats.join(", ").bright_yellow(),
        args.scales
            .iter()
            .map(|s| format!("{}%", s))
            .collect::<Vec<_>>()
            .join(", ")
            .bright_yellow(),
        format!("{}%", args.quality).bright_yellow()
    );

    // Display number of threads in use
    let num_threads = rayon::current_num_threads();
    println!(
        "  {} Using {} {}",
        "🚀".bright_white(),
        num_threads.to_string().bright_green().bold(),
        if num_threads == 1 {
            "thread"
        } else {
            "threads"
        }
        .dimmed()
    );

    println!(); // Empty line for spacing

    // Create multi-progress bar for concurrent image processing
    let mp = create_multi_progress();

    // Process all images through processor module
    processor::process_all(
        files,
        &args.formats,
        &args.scales,
        args.quality,
        args.output.as_ref(),
        &mp,
    )?;

    // Print success message
    println!(
        "\n  {} {}",
        "✓".green().bold(),
        "Processing completed successfully!".green().bold()
    );

    println!(
        "  {} {} images optimized\n",
        "  ".dimmed(),
        total_files.to_string().bright_cyan()
    );

    Ok(())
}

// Collect all image files from input path
fn collect_image_files(args: &Args) -> Result<Vec<PathBuf>> {
    const VALID_EXTENSIONS: &[&str] = &[
        "jpg", "jpeg", "png", "gif", "webp", "bmp", "tiff", "tif", "ico",
    ];
    let mut files = Vec::new();

    if !args.input.exists() {
        anyhow::bail!("Path '{}' does not exist", args.input.display());
    }

    if args.input.is_file() {
        // Single file input
        validate_image_file(&args.input, VALID_EXTENSIONS)?;
        files.push(args.input.clone());
    } else if args.input.is_dir() {
        // Directory input (recursively if specified)
        let walker = if args.recursive {
            WalkDir::new(&args.input)
        } else {
            WalkDir::new(&args.input).max_depth(1)
        };

        for entry in walker.into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();

            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if VALID_EXTENSIONS.contains(&ext.to_lowercase().as_str()) {
                        files.push(path.to_path_buf());
                    }
                }
            }
        }
    } else {
        anyhow::bail!(
            "Path '{}' is not a valid file or directory",
            args.input.display()
        );
    }

    Ok(files)
}

// Validate that a file has a supported image extension
fn validate_image_file(path: &PathBuf, valid_ext: &[&str]) -> Result<()> {
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        if valid_ext.contains(&ext.to_lowercase().as_str()) {
            return Ok(());
        }
    }
    anyhow::bail!("File '{}' is not a supported image format", path.display());
}

// Create a MultiProgress object for concurrent progress bars
fn create_multi_progress() -> MultiProgress {
    MultiProgress::new()
}
