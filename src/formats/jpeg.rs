use anyhow::Result;
use std::fs;
use std::path::Path;

pub(crate) fn optimize(path: &Path, lossy: bool) -> Result<u64> {
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
