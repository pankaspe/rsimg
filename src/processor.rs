// src/processor.rs
//
use anyhow::{Context, Result};
use image::{DynamicImage, ImageFormat};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;
use rayon::prelude::*;
use std::path::{Path, PathBuf};

/// Processes all images in parallel, handling errors and progress display
pub fn process_all(
    files: Vec<PathBuf>,
    formats: &[String],
    scales: &[u32],
    quality: u8,
    output_dir: Option<&PathBuf>,
    mp: &MultiProgress,
) -> Result<()> {
    // Total operations per image (scales * formats)
    let operations_per_image = (formats.len() * scales.len()) as u64;

    // Parallel processing using Rayon
    let results: Vec<Result<()>> = files
        .par_iter()
        .map(|path| {
            // Create a progress bar for each file
            let pb = if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                let pb = mp.add(ProgressBar::new(operations_per_image));
                pb.set_style(
                    ProgressStyle::with_template(
                        "  {msg:40} [{bar:40.cyan/blue}] {pos:>2}/{len:2}",
                    )
                    .unwrap()
                    .progress_chars("━━╾─"),
                );

                // Truncate filename if too long for display
                let display_name = if name.len() > 35 {
                    format!("{}...{}", &name[..20], &name[name.len() - 12..])
                } else {
                    name.to_string()
                };

                pb.set_message(format!("📄 {}", display_name.bright_white()));
                Some(pb)
            } else {
                None
            };

            // Process the image with progress tracking
            let result = process_single_with_progress(
                path,
                formats,
                scales,
                quality,
                output_dir,
                pb.as_ref(),
            );

            // Finish progress bar with success/failure
            if let Some(pb) = &pb {
                if result.is_ok() {
                    let name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| {
                            if n.len() > 35 {
                                format!("{}...{}", &n[..20], &n[n.len() - 12..])
                            } else {
                                n.to_string()
                            }
                        })
                        .unwrap_or("unknown".to_string());

                    pb.finish_with_message(format!("  ✓ {}", name.green()));
                } else {
                    let name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| {
                            if n.len() > 35 {
                                format!("{}...{}", &n[..20], &n[n.len() - 12..])
                            } else {
                                n.to_string()
                            }
                        })
                        .unwrap_or("unknown".to_string());

                    pb.finish_with_message(format!("  ✗ {}", name.red()));
                }
            }

            result
        })
        .collect();

    // Collect all errors
    let errors: Vec<_> = results.into_iter().filter_map(|r| r.err()).collect();

    // Report any errors encountered during processing
    if !errors.is_empty() {
        eprintln!("\n{} Errors during processing:", "⚠️ ".yellow().bold());
        for (i, err) in errors.iter().enumerate() {
            eprintln!(
                "  {}. {}",
                (i + 1).to_string().red(),
                err.to_string().dimmed()
            );
        }
        eprintln!();
        anyhow::bail!("{} images were not processed correctly", errors.len());
    }

    Ok(())
}

/// Processes a single image, resizing and saving to all specified formats,
/// and updating the progress bar incrementally
fn process_single_with_progress(
    path: &Path,
    formats: &[String],
    scales: &[u32],
    quality: u8,
    output_dir: Option<&PathBuf>,
    pb: Option<&ProgressBar>,
) -> Result<()> {
    // Load the image from disk
    let img =
        image::open(path).with_context(|| format!("Failed to open image: {}", path.display()))?;

    // Extract filename without extension
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid filename: {}", path.display()))?;

    // Determine output directory (user-specified or same as input)
    let output_parent = if let Some(out_dir) = output_dir {
        out_dir.clone()
    } else {
        path.parent()
            .ok_or_else(|| anyhow::anyhow!("Cannot determine parent directory"))?
            .to_path_buf()
    };

    // Iterate over all scales and formats
    for &scale in scales {
        let resized = resize_image(&img, scale)?;

        for fmt in formats {
            let output_name = format!("{stem}_{scale}pct.{fmt}");
            let output_path = output_parent.join(output_name);

            // Save image to disk
            save_image(&resized, &output_path, fmt, quality)
                .with_context(|| format!("Error saving: {}", output_path.display()))?;

            // Increment progress bar
            if let Some(pb) = pb {
                pb.inc(1);
            }
        }
    }

    Ok(())
}

/// Resizes an image according to the given scale percentage
fn resize_image(img: &DynamicImage, scale: u32) -> Result<DynamicImage> {
    if scale == 100 {
        // Return original image if scale is 100%
        return Ok(img.clone());
    }

    let factor = scale as f32 / 100.0;
    let new_width = (img.width() as f32 * factor).round() as u32;
    let new_height = (img.height() as f32 * factor).round() as u32;

    // Prevent creating images with zero dimensions
    if new_width == 0 || new_height == 0 {
        anyhow::bail!(
            "Resulting dimensions too small: {}x{} (scale: {}%)",
            new_width,
            new_height,
            scale
        );
    }

    // Resize using high-quality Lanczos3 filter
    Ok(img.resize(new_width, new_height, image::imageops::FilterType::Lanczos3))
}

/// Saves an image to disk in the specified format and quality
fn save_image(img: &DynamicImage, path: &Path, format: &str, quality: u8) -> Result<()> {
    match format.to_lowercase().as_str() {
        "jpg" | "jpeg" => save_jpeg(img, path, quality),
        "webp" => save_webp(img, path, quality),
        "png" => save_png(img, path),
        _ => Err(anyhow::anyhow!("Unsupported format: {}", format)),
    }
}

/// Saves image as JPEG with the given quality
fn save_jpeg(img: &DynamicImage, path: &Path, quality: u8) -> Result<()> {
    let file = std::fs::File::create(path)
        .with_context(|| format!("Failed to create file: {}", path.display()))?;

    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(file, quality);
    encoder
        .encode_image(img)
        .with_context(|| "Error during JPEG encoding")?;

    Ok(())
}

/// Saves image as WebP with the given quality
fn save_webp(img: &DynamicImage, path: &Path, quality: u8) -> Result<()> {
    use webp::Encoder;

    // Convert to RGB8 for WebP encoder
    let rgb = img.to_rgb8();
    let encoder = Encoder::from_rgb(&rgb, rgb.width(), rgb.height());
    let webp_data = encoder.encode(quality as f32);

    // Write encoded WebP bytes to disk
    std::fs::write(path, &*webp_data)
        .with_context(|| format!("Failed to write WebP file: {}", path.display()))?;

    Ok(())
}

/// Saves image as PNG (lossless)
fn save_png(img: &DynamicImage, path: &Path) -> Result<()> {
    img.save_with_format(path, ImageFormat::Png)
        .with_context(|| format!("Failed to save PNG: {}", path.display()))?;

    Ok(())
}
