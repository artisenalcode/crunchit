use image::codecs::jpeg::JpegEncoder;
use image::{ImageBuffer, Rgba};
use std::fs;
use std::path::Path;
use std::process::Command;

fn run_crunchit(dir: &Path, lossy: bool) {
    let mut args = vec![];
    if lossy {
        args.push("--lossy");
    }
    run_crunchit_args(dir, &args);
}

fn run_crunchit_args(dir: &Path, args: &[&str]) {
    let status = Command::new(env!("CARGO_BIN_EXE_crunchit"))
        .arg(dir)
        .args(args)
        .status()
        .expect("failed to run crunchit");
    assert!(status.success(), "crunchit exited with {status}");
}

fn assert_webp_magic(path: &Path) {
    let bytes = fs::read(path).unwrap();
    assert_eq!(&bytes[..4], b"RIFF", "missing RIFF header");
    assert_eq!(&bytes[8..12], b"WEBP", "missing WEBP fourcc");
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
fn webp_survives_optimization() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.webp");
    let img = image::DynamicImage::ImageRgba8(gradient(64, 64));
    img.save_with_format(&path, image::ImageFormat::WebP)
        .unwrap();
    let before = fs::metadata(&path).unwrap().len();

    run_crunchit(dir.path(), false);

    let after = fs::metadata(&path).unwrap().len();
    assert!(after <= before, "webp grew: {before} -> {after}");
    let img = image::open(&path).unwrap();
    assert_eq!((img.width(), img.height()), (64, 64));
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
fn convert_creates_webp_variant_once() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("photo.png");
    gradient(128, 128).save(&source).unwrap();

    run_crunchit_args(dir.path(), &["--convert", "webp"]);

    let variant = dir.path().join("photo.webp");
    assert!(variant.exists(), "webp variant not created");
    assert_webp_magic(&variant);
    let img = image::open(&variant).unwrap();
    assert_eq!((img.width(), img.height()), (128, 128));

    // Second run must be a no-op: the fresh variant is not regenerated.
    let mtime = fs::metadata(&variant).unwrap().modified().unwrap();
    run_crunchit_args(dir.path(), &["--convert", "webp"]);
    assert_eq!(
        mtime,
        fs::metadata(&variant).unwrap().modified().unwrap(),
        "variant was regenerated"
    );
}

#[test]
fn convert_creates_avif_variant() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("photo.png");
    gradient(64, 64).save(&source).unwrap();

    run_crunchit_args(dir.path(), &["--convert", "avif"]);

    let variant = dir.path().join("photo.avif");
    assert!(variant.exists(), "avif variant not created");
    let bytes = fs::read(&variant).unwrap();
    assert_eq!(&bytes[4..12], b"ftypavif", "missing avif brand");
}

#[test]
fn convert_animated_gif_to_animated_webp() {
    use image::codecs::gif::GifEncoder;
    use image::{Delay, Frame};

    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("anim.gif");
    let file = fs::File::create(&source).unwrap();
    let mut encoder = GifEncoder::new(file);
    let frames = (0..3u32).map(|i| {
        let buf = ImageBuffer::from_fn(64, 64, |x, y| {
            Rgba([(x + i * 40) as u8, y as u8, (i * 80) as u8, 255])
        });
        Frame::from_parts(buf, 0, 0, Delay::from_numer_denom_ms(100, 1))
    });
    encoder.encode_frames(frames).unwrap();
    drop(encoder);

    run_crunchit_args(dir.path(), &["--convert", "webp"]);

    let variant = dir.path().join("anim.webp");
    assert!(variant.exists(), "animated webp variant not created");
    assert_webp_magic(&variant);
}

#[test]
fn unsupported_files_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("notes.txt");
    fs::write(&path, "do not touch").unwrap();

    run_crunchit(dir.path(), false);

    assert_eq!(fs::read_to_string(&path).unwrap(), "do not touch");
}
