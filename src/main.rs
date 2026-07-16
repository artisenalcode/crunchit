use anyhow::Result;
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use oxipng::{InFile, Options as OxiOptions, OutFile, StripChunks, optimize};
use rayon::prelude::*;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
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
}

#[derive(Debug, Default)]
struct OptimizationStats {
    processed: AtomicUsize,
    saved_bytes: AtomicU64,
    errors: AtomicUsize,
}

fn optimize_png(path: &Path, _lossy: bool) -> Result<u64> {
    let initial_size = fs::metadata(path)?.len();
    let mut options = OxiOptions::from_preset(6); // Max compression
    options.strip = StripChunks::Safe;

    let in_file = InFile::Path(path.to_path_buf());
    let out_file = OutFile::Path {
        path: Some(path.to_path_buf()),
        preserve_attrs: false,
    };

    optimize(&in_file, &out_file, &options)?;

    let final_size = fs::metadata(path)?.len();
    if initial_size > final_size {
        Ok(initial_size - final_size)
    } else {
        Ok(0)
    }
}

fn optimize_jpeg(path: &Path, lossy: bool) -> Result<u64> {
    let initial_size = fs::metadata(path)?.len();

    // Pure rust "lossless" is re-encoding at high quality
    let quality = if lossy { 85 } else { 100 };

    let img = image::open(path)?;

    let temp_path = path.with_extension("tmp.jpg");
    let mut file = fs::File::create(&temp_path)?;
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut file, quality);

    encoder.encode_image(&img)?;
    drop(file);

    let final_size = fs::metadata(&temp_path)?.len();
    if initial_size > final_size {
        fs::rename(&temp_path, path)?;
        Ok(initial_size - final_size)
    } else {
        fs::remove_file(&temp_path)?;
        Ok(0)
    }
}

fn optimize_gif(path: &Path, _lossy: bool) -> Result<u64> {
    let initial_size = fs::metadata(path)?.len();

    let file_in = fs::File::open(path)?;
    let mut decoder = gif::DecodeOptions::new();
    decoder.set_color_output(gif::ColorOutput::Indexed);
    let mut reader = decoder.read_info(file_in).map_err(|e| anyhow::anyhow!(e))?;

    let temp_path = path.with_extension("tmp.gif");
    let mut file_out = fs::File::create(&temp_path)?;

    {
        let mut encoder = gif::Encoder::new(
            &mut file_out,
            reader.width(),
            reader.height(),
            reader.global_palette().unwrap_or(&[]),
        )
        .map_err(|e| anyhow::anyhow!(e))?;
        encoder
            .set_repeat(reader.repeat())
            .map_err(|e| anyhow::anyhow!(e))?;

        while let Some(frame) = reader.read_next_frame().map_err(|e| anyhow::anyhow!(e))? {
            encoder.write_frame(frame).map_err(|e| anyhow::anyhow!(e))?;
        }
    }

    let final_size = fs::metadata(&temp_path)?.len();
    if initial_size > final_size {
        fs::rename(&temp_path, path)?;
        Ok(initial_size - final_size)
    } else {
        fs::remove_file(&temp_path)?;
        Ok(0)
    }
}

fn optimize_svg(path: &Path, _lossy: bool) -> Result<u64> {
    let initial_size = fs::metadata(path)?.len();
    let content = fs::read_to_string(path)?;

    use oxvg_ast::{parse::roxmltree::parse, serialize::Node as _, visitor::Info};
    use oxvg_optimiser::Jobs;

    let optimized_content = parse(&content, |dom, allocator| {
        let jobs = Jobs::default();
        jobs.run(dom, &Info::new(allocator)).unwrap();
        dom.serialize().unwrap()
    })
    .map_err(|e| anyhow::anyhow!("svg parse error: {}", e))?;

    if optimized_content.len() < initial_size as usize {
        fs::write(path, optimized_content)?;
        Ok(initial_size - fs::metadata(path)?.len())
    } else {
        Ok(0)
    }
}

fn process_file(path: &Path, lossy: bool) -> Result<u64> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    if ext.eq_ignore_ascii_case("png") {
        optimize_png(path, lossy)
    } else if ext.eq_ignore_ascii_case("jpg") || ext.eq_ignore_ascii_case("jpeg") {
        optimize_jpeg(path, lossy)
    } else if ext.eq_ignore_ascii_case("gif") {
        optimize_gif(path, lossy)
    } else if ext.eq_ignore_ascii_case("svg") {
        optimize_svg(path, lossy)
    } else {
        Ok(0)
    }
}

fn is_supported_image(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    ext.eq_ignore_ascii_case("png")
        || ext.eq_ignore_ascii_case("jpg")
        || ext.eq_ignore_ascii_case("jpeg")
        || ext.eq_ignore_ascii_case("gif")
        || ext.eq_ignore_ascii_case("svg")
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let start_time = Instant::now();

    if let Some(threads) = cli.threads {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build_global()?;
    }

    println!("Scanning directory: {}", cli.path);

    let mut files_to_process = Vec::new();
    for entry in WalkDir::new(&cli.path).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if path.is_file() && is_supported_image(path) {
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

        match process_file(file_path, cli.lossy) {
            Ok(saved) => {
                stats.processed.fetch_add(1, Ordering::Relaxed);
                stats.saved_bytes.fetch_add(saved, Ordering::Relaxed);
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
    if errors > 0 {
        println!("Errors/Skipped:  {} (Failed to process file)", errors);
    }
    println!("Time taken:      {:.2?}", start_time.elapsed());

    Ok(())
}
