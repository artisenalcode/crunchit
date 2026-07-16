use anyhow::Result;
use oxipng::{InFile, Options as OxiOptions, OutFile, StripChunks, optimize as oxi_optimize};
use std::fs;
use std::path::Path;

pub(crate) fn optimize(path: &Path, _lossy: bool) -> Result<u64> {
    let initial_size = fs::metadata(path)?.len();
    let mut options = OxiOptions::from_preset(6); // Max compression
    options.strip = StripChunks::Safe;

    let in_file = InFile::Path(path.to_path_buf());
    let out_file = OutFile::Path {
        path: Some(path.to_path_buf()),
        preserve_attrs: false,
    };

    oxi_optimize(&in_file, &out_file, &options)?;

    let final_size = fs::metadata(path)?.len();
    if initial_size > final_size {
        Ok(initial_size - final_size)
    } else {
        Ok(0)
    }
}
