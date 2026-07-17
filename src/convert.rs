use anyhow::{Result, anyhow};
use image::AnimationDecoder;
use image::codecs::gif::GifDecoder;
use std::fs;
use std::io::BufReader;
use std::path::Path;

pub struct ConvertOptions {
    pub webp: bool,
    pub webp_quality: f32,
    pub avif: bool,
    pub avif_quality: f32,
}

/// (variants created, bytes written) for `path`; (0, 0) if none were due.
pub fn convert_file(path: &Path, opts: &ConvertOptions) -> Result<(usize, u64)> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let mut created = 0;
    let mut bytes = 0;
    let mut record = |written: u64| {
        if written > 0 {
            created += 1;
            bytes += written;
        }
    };

    if opts.webp {
        match ext.as_str() {
            "png" | "jpg" | "jpeg" => record(still_to_webp(path, opts.webp_quality)?),
            "gif" => record(gif_to_animated_webp(path, opts.webp_quality)?),
            _ => {}
        }
    }
    if opts.avif && matches!(ext.as_str(), "png" | "jpg" | "jpeg") {
        record(still_to_avif(path, opts.avif_quality)?);
    }
    #[cfg(feature = "heic")]
    if matches!(ext.as_str(), "heic" | "heif") {
        let (jpeg_path, written) = heic_to_jpeg(path)?;
        record(written);
        if opts.webp {
            record(still_to_webp(&jpeg_path, opts.webp_quality)?);
        }
        if opts.avif {
            record(still_to_avif(&jpeg_path, opts.avif_quality)?);
        }
    }
    Ok((created, bytes))
}

#[cfg(feature = "heic")]
fn avif_via_libheif(source: &Path, target: &Path, quality: f32) -> Result<u64> {
    use libheif_rs::{
        Channel, ColorSpace, CompressionFormat, EncoderQuality, HeifContext, LibHeif, RgbChroma,
    };

    let img = image::open(source)?;
    let rgba = img.to_rgba8();
    let (width, height) = (img.width(), img.height());

    let mut heif_image = libheif_rs::Image::new(width, height, ColorSpace::Rgb(RgbChroma::Rgba))?;
    heif_image.create_plane(Channel::Interleaved, width, height, 8)?;
    let mut planes = heif_image.planes_mut();
    let plane = planes
        .interleaved
        .as_mut()
        .ok_or_else(|| anyhow!("libheif: no interleaved plane"))?;
    let row_bytes = width as usize * 4;
    let stride = plane.stride;
    for (y, chunk) in rgba.as_raw().chunks_exact(row_bytes).enumerate() {
        plane.data[y * stride..y * stride + row_bytes].copy_from_slice(chunk);
    }

    let lib_heif = LibHeif::new();
    let mut encoder = lib_heif.encoder_for_format(CompressionFormat::Av1)?;
    encoder.set_quality(EncoderQuality::Lossy(quality.clamp(0.0, 100.0) as u8))?;
    let mut ctx = HeifContext::new()?;
    ctx.encode_image(&heif_image, &mut encoder, None)?;
    ctx.write_to_file(target.to_str().ok_or_else(|| anyhow!("non-utf8 path"))?)?;
    let written = fs::metadata(target)?.len();
    Ok(written)
}

/// HEIC is input-only: decode via libheif into an optimized JPEG sibling, which then
/// feeds the standard JPEG conversion rules. The original is never touched.
#[cfg(feature = "heic")]
fn heic_to_jpeg(path: &Path) -> Result<(std::path::PathBuf, u64)> {
    let target = path.with_extension("jpg");
    if is_fresh(&target, path) {
        return Ok((target, 0));
    }

    let lib_heif = libheif_rs::LibHeif::new();
    let ctx = libheif_rs::HeifContext::read_from_file(
        path.to_str().ok_or_else(|| anyhow!("non-utf8 path"))?,
    )?;
    let handle = ctx.primary_image_handle()?;
    let decoded = lib_heif.decode(
        &handle,
        libheif_rs::ColorSpace::Rgb(libheif_rs::RgbChroma::Rgba),
        None,
    )?;
    let plane = decoded
        .planes()
        .interleaved
        .ok_or_else(|| anyhow!("heic: no interleaved plane"))?;

    // libheif rows are stride-padded; repack to tight RGBA.
    let (width, height) = (plane.width, plane.height);
    let row_bytes = width as usize * 4;
    let mut rgba = Vec::with_capacity(row_bytes * height as usize);
    for row in 0..height as usize {
        let start = row * plane.stride;
        rgba.extend_from_slice(&plane.data[start..start + row_bytes]);
    }
    let buffer = image::RgbaImage::from_raw(width, height, rgba)
        .ok_or_else(|| anyhow!("heic: decoded size mismatch"))?;

    let mut file = fs::File::create(&target)?;
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut file, 85);
    encoder.encode_image(&image::DynamicImage::ImageRgba8(buffer).to_rgb8())?;
    drop(file);
    let written = fs::metadata(&target)?.len();
    Ok((target, written))
}

fn still_to_avif(path: &Path, quality: f32) -> Result<u64> {
    let target = path.with_extension("avif");
    if is_fresh(&target, path) {
        return Ok(0);
    }

    // System libaom (via libheif) encodes ~20x faster than pure-Rust ravif;
    // prefer it when compiled in, fall back to ravif if the plugin is missing.
    #[cfg(feature = "heic")]
    if let Ok(written) = avif_via_libheif(path, &target, quality) {
        return Ok(written);
    }

    let img = image::open(path)?;
    let rgba = img.to_rgba8();
    use rgb::FromSlice;
    let pixels = ravif::Img::new(
        rgba.as_raw().as_rgba(),
        img.width() as usize,
        img.height() as usize,
    );
    let encoded = ravif::Encoder::new()
        .with_quality(quality)
        .with_alpha_quality(quality)
        .with_speed(6)
        .encode_rgba(pixels)
        .map_err(|e| anyhow!("avif encode: {e}"))?;
    fs::write(&target, &encoded.avif_file)?;
    Ok(encoded.avif_file.len() as u64)
}

/// A variant is fresh when it exists and is at least as new as its source.
fn is_fresh(target: &Path, source: &Path) -> bool {
    let target_mtime = fs::metadata(target).and_then(|m| m.modified());
    let source_mtime = fs::metadata(source).and_then(|m| m.modified());
    matches!((target_mtime, source_mtime), (Ok(t), Ok(s)) if t >= s)
}

fn still_to_webp(path: &Path, quality: f32) -> Result<u64> {
    let target = path.with_extension("webp");
    if is_fresh(&target, path) {
        return Ok(0);
    }

    let img = image::open(path)?;
    let rgba = img.to_rgba8();
    let encoded = webp::Encoder::from_rgba(&rgba, img.width(), img.height()).encode(quality);
    fs::write(&target, &*encoded)?;
    Ok(encoded.len() as u64)
}

fn gif_to_animated_webp(path: &Path, quality: f32) -> Result<u64> {
    let target = path.with_extension("webp");
    if is_fresh(&target, path) {
        return Ok(0);
    }

    let decoder = GifDecoder::new(BufReader::new(fs::File::open(path)?))?;
    let frames = decoder.into_frames().collect_frames()?;
    let first = frames.first().ok_or_else(|| anyhow!("gif has no frames"))?;
    let (width, height) = first.buffer().dimensions();

    let options = webp_animation::EncoderOptions {
        encoding_config: Some(webp_animation::EncodingConfig {
            quality,
            ..Default::default()
        }),
        ..Default::default()
    };
    let mut encoder = webp_animation::Encoder::new_with_options((width, height), options)
        .map_err(|e| anyhow!("webp encoder: {e:?}"))?;

    let mut timestamp_ms: i32 = 0;
    for frame in &frames {
        encoder
            .add_frame(frame.buffer().as_raw(), timestamp_ms)
            .map_err(|e| anyhow!("webp frame: {e:?}"))?;
        let (numer, denom) = frame.delay().numer_denom_ms();
        timestamp_ms += (numer / denom.max(1)) as i32;
    }

    let data = encoder
        .finalize(timestamp_ms)
        .map_err(|e| anyhow!("webp finalize: {e:?}"))?;
    fs::write(&target, &data)?;
    Ok(data.len() as u64)
}
