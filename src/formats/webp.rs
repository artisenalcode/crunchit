use anyhow::Result;
use image::codecs::webp::WebPEncoder;
use std::fs;
use std::path::Path;

pub(crate) fn optimize(path: &Path, _lossy: bool) -> Result<u64> {
    let initial_size = fs::metadata(path)?.len();
    let img = image::open(path)?;

    let temp_path = path.with_extension("tmp.webp");
    let file = fs::File::create(&temp_path)?;
    let encoder = WebPEncoder::new_lossless(file);
    img.write_with_encoder(encoder)?;

    let final_size = fs::metadata(&temp_path)?.len();
    if initial_size > final_size {
        fs::rename(&temp_path, path)?;
        Ok(initial_size - final_size)
    } else {
        fs::remove_file(&temp_path)?;
        Ok(0)
    }
}
