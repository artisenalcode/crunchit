# crunchit

[![CI](https://github.com/artisenalcode/crunchit/actions/workflows/ci.yml/badge.svg)](https://github.com/artisenalcode/crunchit/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

A fast, cross-platform CLI for optimizing images completely in pure Rust. It operates as a drop-in replacement for tools like ImageOptim but works straight from your terminal without requiring any external dependencies (`jpegoptim`, `gifsicle`, etc).

It natively optimizes the following formats completely in-process:
- **PNG**: Lossless optimization powered by `oxipng`
- **JPEG**: Pseudo-lossless (Quality 100) or lossy (Quality 85) optimization powered by the `image` crate
- **GIF**: Lossless palette and frame compression via the `gif` crate
- **SVG**: Vector graphics minification via the `oxvg` family

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

Reproduce with `bash bench/run.sh` after placing sample images in `bench/images/`
(the corpus itself is not committed).

## Development

```bash
cargo test            # smoke tests: every format optimizes and stays decodable
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

Tagging `v*` builds and attaches release binaries (Linux x86_64, macOS arm64/x86_64) via GitHub Actions.

## License

[MIT](LICENSE)