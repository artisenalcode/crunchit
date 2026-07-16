use anyhow::Result;
use std::fs;
use std::path::Path;

pub(crate) fn optimize(path: &Path, _lossy: bool) -> Result<u64> {
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
