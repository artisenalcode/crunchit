mod convert;
mod formats;
mod stats;

use anyhow::Result;
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use stats::OptimizationStats;
use std::sync::atomic::Ordering;
use std::time::Instant;
use walkdir::WalkDir;

/// A Rust-based CLI for ImageOptim-like image optimization
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Directory to scan for images
    #[arg(default_value = ".")]
    path: String,

    /// Number of threads to use (default: number of logical cores)
    #[arg(short, long)]
    threads: Option<usize>,

    /// Run in lossy mode (default is lossless)
    #[arg(long)]
    lossy: bool,

    /// Generate next-gen variants next to originals (comma-separated; currently: webp)
    #[arg(long, value_delimiter = ',')]
    convert: Vec<String>,

    /// Quality for generated WebP variants (0-100)
    #[arg(long, default_value_t = 80.0)]
    webp_quality: f32,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let start_time = Instant::now();

    for format in &cli.convert {
        if format != "webp" {
            anyhow::bail!("unsupported --convert format: {format} (supported: webp)");
        }
    }
    let convert_opts = convert::ConvertOptions {
        webp: cli.convert.iter().any(|f| f == "webp"),
        webp_quality: cli.webp_quality,
    };

    if let Some(threads) = cli.threads {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build_global()?;
    }

    println!("Scanning directory: {}", cli.path);

    let mut files_to_process = Vec::new();
    for entry in WalkDir::new(&cli.path).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if path.is_file() && formats::is_supported_image(path) {
            files_to_process.push(path.to_path_buf());
        }
    }

    let total_files = files_to_process.len();
    if total_files == 0 {
        println!("No supported images found in directory.");
        return Ok(());
    }

    println!("Found {} images to optimize.", total_files);

    let pb = ProgressBar::new(total_files as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) - {msg}")?
            .progress_chars("=>-"),
    );

    let stats = OptimizationStats::default();

    files_to_process.par_iter().for_each(|file_path| {
        let file_name = file_path.file_name().unwrap_or_default().to_string_lossy();
        pb.set_message(file_name.to_string());

        match formats::process_file(file_path, cli.lossy) {
            Ok(saved) => {
                stats.processed.fetch_add(1, Ordering::Relaxed);
                stats.saved_bytes.fetch_add(saved, Ordering::Relaxed);
            }
            Err(_e) => {
                stats.errors.fetch_add(1, Ordering::Relaxed);
            }
        }

        match convert::convert_file(file_path, &convert_opts) {
            Ok(0) => {}
            Ok(bytes) => {
                stats.variants.fetch_add(1, Ordering::Relaxed);
                stats.variant_bytes.fetch_add(bytes, Ordering::Relaxed);
            }
            Err(_e) => {
                stats.errors.fetch_add(1, Ordering::Relaxed);
            }
        }

        pb.inc(1);
    });

    pb.finish_with_message("Done");

    let processed = stats.processed.load(Ordering::Relaxed);
    let saved_bytes = stats.saved_bytes.load(Ordering::Relaxed);
    let errors = stats.errors.load(Ordering::Relaxed);

    let saved_human = human_bytes::human_bytes(saved_bytes as f64);

    println!("\n--- Optimization Summary ---");
    println!("Processed files: {}", processed);
    println!("Space saved:     {}", saved_human);
    let variants = stats.variants.load(Ordering::Relaxed);
    if variants > 0 {
        let variant_bytes = stats.variant_bytes.load(Ordering::Relaxed);
        println!(
            "Variants:        {} created ({})",
            variants,
            human_bytes::human_bytes(variant_bytes as f64)
        );
    }
    if errors > 0 {
        println!("Errors/Skipped:  {} (Failed to process file)", errors);
    }
    println!("Time taken:      {:.2?}", start_time.elapsed());

    Ok(())
}
