mod gif;
mod jpeg;
mod png;
mod svg;
mod webp;

use anyhow::Result;
use std::path::Path;

fn ext(path: &Path) -> &str {
    path.extension().and_then(|e| e.to_str()).unwrap_or("")
}

pub fn process_file(path: &Path, lossy: bool) -> Result<u64> {
    let ext = ext(path);
    if ext.eq_ignore_ascii_case("png") {
        png::optimize(path, lossy)
    } else if ext.eq_ignore_ascii_case("jpg") || ext.eq_ignore_ascii_case("jpeg") {
        jpeg::optimize(path, lossy)
    } else if ext.eq_ignore_ascii_case("gif") {
        gif::optimize(path, lossy)
    } else if ext.eq_ignore_ascii_case("svg") {
        svg::optimize(path, lossy)
    } else if ext.eq_ignore_ascii_case("webp") {
        webp::optimize(path, lossy)
    } else {
        Ok(0)
    }
}

pub fn is_supported_image(path: &Path) -> bool {
    let ext = ext(path);
    #[cfg(feature = "heic")]
    if ext.eq_ignore_ascii_case("heic") || ext.eq_ignore_ascii_case("heif") {
        return true;
    }
    ext.eq_ignore_ascii_case("png")
        || ext.eq_ignore_ascii_case("jpg")
        || ext.eq_ignore_ascii_case("jpeg")
        || ext.eq_ignore_ascii_case("gif")
        || ext.eq_ignore_ascii_case("svg")
        || ext.eq_ignore_ascii_case("webp")
}
