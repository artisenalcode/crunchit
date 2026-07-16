use image::codecs::jpeg::JpegEncoder;
use image::{ImageBuffer, Rgba};
use std::fs;
use std::path::Path;
use std::process::Command;

fn run_crunchit(dir: &Path, lossy: bool) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_crunchit"));
    cmd.arg(dir);
    if lossy {
        cmd.arg("--lossy");
    }
    let status = cmd.status().expect("failed to run crunchit");
    assert!(status.success(), "crunchit exited with {status}");
}

fn gradient(w: u32, h: u32) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    ImageBuffer::from_fn(w, h, |x, y| {
        Rgba([(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8, 255])
    })
}

#[test]
fn png_shrinks_and_stays_valid() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.png");
    gradient(128, 128).save(&path).unwrap();
    let before = fs::metadata(&path).unwrap().len();

    run_crunchit(dir.path(), false);

    let after = fs::metadata(&path).unwrap().len();
    assert!(
        after < before,
        "expected png to shrink: {before} -> {after}"
    );
    let img = image::open(&path).unwrap();
    assert_eq!((img.width(), img.height()), (128, 128));
}

#[test]
fn jpeg_lossy_shrinks_and_stays_valid() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.jpg");
    let mut file = fs::File::create(&path).unwrap();
    let mut encoder = JpegEncoder::new_with_quality(&mut file, 100);
    encoder
        .encode_image(&image::DynamicImage::ImageRgba8(gradient(128, 128)))
        .unwrap();
    drop(file);
    let before = fs::metadata(&path).unwrap().len();

    run_crunchit(dir.path(), true);

    let after = fs::metadata(&path).unwrap().len();
    assert!(
        after < before,
        "expected jpeg to shrink: {before} -> {after}"
    );
    let img = image::open(&path).unwrap();
    assert_eq!((img.width(), img.height()), (128, 128));
}

#[test]
fn gif_survives_optimization() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.gif");
    let img = image::DynamicImage::ImageRgba8(gradient(64, 64));
    img.save_with_format(&path, image::ImageFormat::Gif)
        .unwrap();
    let before = fs::metadata(&path).unwrap().len();

    run_crunchit(dir.path(), false);

    let after = fs::metadata(&path).unwrap().len();
    assert!(after <= before, "gif grew: {before} -> {after}");
    let img = image::open(&path).unwrap();
    assert_eq!((img.width(), img.height()), (64, 64));
}

#[test]
fn svg_shrinks_and_stays_svg() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.svg");
    let verbose_svg = r##"<?xml version="1.0" encoding="UTF-8"?>
<!-- a comment that should be stripped by the optimiser -->
<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
    <!-- another comment -->
    <rect x="10"   y="10"   width="80"   height="80"   fill="#ff0000" />
</svg>
"##;
    fs::write(&path, verbose_svg).unwrap();
    let before = fs::metadata(&path).unwrap().len();

    run_crunchit(dir.path(), false);

    let after = fs::metadata(&path).unwrap().len();
    assert!(
        after < before,
        "expected svg to shrink: {before} -> {after}"
    );
    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains("<svg"));
    assert!(!content.contains("a comment"));
}

#[test]
fn unsupported_files_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("notes.txt");
    fs::write(&path, "do not touch").unwrap();

    run_crunchit(dir.path(), false);

    assert_eq!(fs::read_to_string(&path).unwrap(), "do not touch");
}
