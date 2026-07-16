use anyhow::{Result, anyhow};
use image::AnimationDecoder;
use image::codecs::gif::GifDecoder;
use std::fs;
use std::io::BufReader;
use std::path::Path;

pub struct ConvertOptions {
    pub webp: bool,
    pub webp_quality: f32,
}

/// Bytes written for newly created variants of `path`, or 0 if none were due.
pub fn convert_file(path: &Path, opts: &ConvertOptions) -> Result<u64> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let mut bytes = 0;
    if opts.webp {
        match ext.as_str() {
            "png" | "jpg" | "jpeg" => bytes += still_to_webp(path, opts.webp_quality)?,
            "gif" => bytes += gif_to_animated_webp(path, opts.webp_quality)?,
            _ => {}
        }
    }
    Ok(bytes)
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
