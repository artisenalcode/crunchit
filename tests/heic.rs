use image::{ImageBuffer, Rgba};
use std::fs;
use std::path::Path;
use std::process::Command;

/// Builds a HEIC fixture with the system `heif-enc` tool; skips the test (with a
/// message) when the tool or its HEVC encoder is unavailable.
fn make_heic_fixture(dir: &Path) -> Option<std::path::PathBuf> {
    let png = dir.join("fixture.png");
    let img = ImageBuffer::from_fn(64, 64, |x, y| {
        Rgba([(x * 4) as u8, (y * 4) as u8, 128u8, 255])
    });
    img.save(&png).unwrap();

    let heic = dir.join("photo.heic");
    // Prefer a true HEVC fixture; fall back to an AV1-compressed HEIF container
    // (same libheif decode path in crunchit) when no x265 encoder plugin exists.
    for extra_args in [&[][..], &["--avif"][..]] {
        let output = Command::new("heif-enc")
            .args(extra_args)
            .arg(&png)
            .arg("-o")
            .arg(&heic)
            .output();
        if matches!(output, Ok(ref out) if out.status.success()) && heic.exists() {
            fs::remove_file(&png).unwrap();
            return Some(heic);
        }
    }
    eprintln!("skipping: heif-enc unavailable or no usable encoder plugin");
    None
}

#[test]
fn heic_converts_to_jpeg_and_webp() {
    let dir = tempfile::tempdir().unwrap();
    let Some(heic) = make_heic_fixture(dir.path()) else {
        return;
    };
    let original = fs::read(&heic).unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_crunchit"))
        .arg(dir.path())
        .args(["--convert", "webp"])
        .status()
        .unwrap();
    assert!(status.success());

    let jpeg = dir.path().join("photo.jpg");
    assert!(jpeg.exists(), "jpeg sibling not created");
    let img = image::open(&jpeg).unwrap();
    assert_eq!((img.width(), img.height()), (64, 64));

    let webp = dir.path().join("photo.webp");
    assert!(webp.exists(), "webp sibling not created");
    let bytes = fs::read(&webp).unwrap();
    assert_eq!(&bytes[..4], b"RIFF");

    assert_eq!(fs::read(&heic).unwrap(), original, "heic original modified");
}
