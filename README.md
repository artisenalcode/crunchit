# crunchit

[![CI](https://github.com/artisenalcode/crunchit/actions/workflows/ci.yml/badge.svg)](https://github.com/artisenalcode/crunchit/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

A fast, cross-platform, Rust-based CLI for optimizing images. It operates as a drop-in replacement for tools like ImageOptim but works straight from your terminal — one static binary, no external tools required (`jpegoptim`, `gifsicle`, `cwebp`, etc).

It natively optimizes the following formats completely in-process:
- **PNG**: Lossless optimization powered by `oxipng`
- **JPEG**: Pseudo-lossless (Quality 100) or lossy (Quality 85) optimization powered by the `image` crate
- **GIF**: Lossless palette and frame compression via the `gif` crate
- **SVG**: Vector graphics minification via the `oxvg` family
- **WebP**: Lossless re-encode via the `image` crate

It can also **generate next-gen web variants** alongside your originals — see
[`--convert`](#generating-next-gen-variants) below.

## Installation

Prebuilt binaries for Linux and macOS are attached to each
[GitHub release](https://github.com/artisenalcode/crunchit/releases).

Or compile from source and install to your local path:

```bash
git clone https://github.com/artisenalcode/crunchit
cd crunchit
cargo build --release
mkdir -p ~/.local/bin
cp target/release/crunchit ~/.local/bin/
```

Make sure `~/.local/bin` is in your `PATH` by adding it to your `~/.bashrc`, `~/.zshrc`, or `~/.profile`:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

## Usage

Run `crunchit` on any directory to recursively scan and optimize all supported images (`.png`, `.jpg`, `.jpeg`, `.gif`, `.svg`):

```bash
# Optimize all images in the current directory and subdirectories losslessly
crunchit .

# Or point it to a specific directory
crunchit ~/Pictures/ToOptimize
```

### Options

```bash
Usage: crunchit [OPTIONS] [PATH]

Arguments:
  [PATH]  Directory to scan for images [default: .]

Options:
  -t, --threads <THREADS>  Number of threads to use (default: number of logical cores)
      --lossy              Run in lossy mode (default is lossless)
  -h, --help               Print help
  -V, --version            Print version
```

### Examples

**Maximum performance**: Utilize 16 threads specifically:
```bash
crunchit -t 16 ./my_images
```

**Lossy Mode**: If you want to compress aggressively and are okay with dropping JPEG quality slightly (to 85):
```bash
crunchit --lossy ./web_assets
```

## Generating next-gen variants

By default crunchit only optimizes in place and never creates files. Opt in with
`--convert` to spawn modern-format siblings next to each original:

```bash
# photo.jpg → photo.webp, anim.gif → anim.webp (animated)
crunchit --convert webp ./web_assets

# Tune variant quality (default 80)
crunchit --convert webp --webp-quality 70 ./web_assets
```

Conversion rules:
- **PNG / JPEG → WebP** (lossy, quality `--webp-quality`, alpha preserved)
- **PNG / JPEG → AVIF** (`--convert avif`, quality `--avif-quality`, default 60)
- **Animated GIF → animated WebP** — typically a dramatic size reduction

AVIF encoding is CPU-heavy (~13s for a 3MB PNG with the pure-Rust encoder) but pays for
itself: the same screenshot that WebP takes to 125K, AVIF takes to **64K (97% smaller)**.

Re-runs are idempotent: a variant is only regenerated when its source is newer.
HEIC input is planned — see [ROADMAP.md](ROADMAP.md).

## Benchmarks

Lossless mode on a mixed PNG corpus — screenshots (`ss_*`, the common web-asset case)
and AI-generated photographic PNGs (already tightly compressed, a worst case for
lossless optimization):

| File | Before | After | Saved |
|---|---|---|---|
| ss_cat.png | 3.1M | 2.4M | 21.9% |
| ss_dog.png | 3.0M | 2.4M | 21.9% |
| russian_blue_cat.png | 2.1M | 2.0M | 6.5% |
| scottish_terrier.png | 2.1M | 2.0M | 7.5% |
| **Total** | 11M | 8.6M | **15.8%** |

Next-gen variant generation (`--convert webp`, default quality 80) on the same corpus:

| Source | Optimized | WebP variant | Smaller by |
|---|---|---|---|
| ss_cat.png | 2.4M | 125K | 94.8% |
| ss_dog.png | 2.4M | 126K | 94.7% |
| russian_blue_cat.png | 2.0M | 104K | 94.7% |
| scottish_terrier.png | 2.0M | 105K | 94.6% |

Reproduce with `bash bench/run.sh` after placing sample images in `bench/images/`
(the corpus itself is not committed).

## Development

```bash
cargo test            # smoke tests: every format optimizes and stays decodable
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

The optional `asm` feature (`cargo build --release --features asm`) uses hand-written
SIMD in the AVIF encoder for ~15% less CPU; it requires the `nasm` assembler at build
time. Prebuilt release binaries ship with it enabled; the default build needs no
assembler and compiles anywhere.

Tagging `v*` builds and attaches release binaries (Linux x86_64, macOS arm64/x86_64) via GitHub Actions.

## License

[MIT](LICENSE)