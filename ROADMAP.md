# Roadmap — modern web formats & conversion matrix

## Goal

Extend crunchit from an in-place optimizer for legacy formats (PNG/JPEG/GIF/SVG) into a
modern web-asset pipeline: optimize WebP natively, generate next-gen variants (WebP/AVIF)
from standard uploads, and accept HEIC as an input format — without breaking the existing
"point it at a directory, it optimizes in place" contract.

## Context

- Current implementation: single binary, `src/main.rs` (~240 lines). `process_file()`
  dispatches on extension to `optimize_{png,jpeg,gif,svg}()`; each optimizes **in place**
  and returns bytes saved. Rayon parallelism, `walkdir` scan, atomic stats.
- Tests: `tests/smoke.rs` drives the compiled binary per format.
- Constraint: README pitches "pure Rust, no external dependencies". Lossy WebP encoding,
  animated WebP, and HEIC decoding all require C libraries (statically linked — no
  *runtime* deps, but the "pure Rust" wording must soften to "no external tools required").
- Crate reality check (corrections to the suggestion list):
  - `image` crate: WebP **decode** is native, but **encode is lossless-only** — no lossy,
    no alpha-tuned quality, no animation. Fine for optimizing existing WebP, not for
    generating variants at web-typical sizes.
  - `webp` crate (libwebp bindings, statically built): lossy + alpha encode. Required for
    the conversion matrix. `webp-animation` (also libwebp) for animated WebP.
  - `ravif`: pure-Rust AVIF **encoder** (rav1e). Encode-only is exactly what variant
    generation needs; AVIF decode (dav1d) is not required.
  - `libheif-rs`: bindings to system libheif + HEVC decoder. Heavy, patent-encumbered,
    hard to cross-compile — must be an opt-in cargo feature, decode-only.

## Design decisions

- **In-place optimization stays the default.** The conversion matrix is opt-in:
  `crunchit --convert webp,avif <dir>` spawns sibling files (`photo.jpg` → `photo.webp`,
  `photo.avif`) next to the source.
- **Idempotency rule:** a file is skipped as a conversion *source* if it is itself a
  generated variant, or if the target variant already exists and is newer than the source.
- **Naming:** `foo.png` → `foo.webp` / `foo.avif` (extension swap, not suffix stacking).
  On basename collision (`foo.png` + `foo.jpg`), first writer wins; log the skip.
- **Matrix (as proposed):** JPEG/PNG → WebP + AVIF · animated GIF → animated WebP ·
  HEIC → JPEG (then JPEG rules apply). HEIC originals are never deleted.

## Steps

### Phase 0 — ship first (not this roadmap)
Release v0.1.0 as-is. Everything below lands behind new flags in v0.2+.

### Phase 1 (v0.2) — module split + WebP
1. Split `src/main.rs` into `src/formats/{png,jpeg,gif,svg}.rs`, `src/scan.rs`,
   `src/stats.rs`; `main.rs` keeps CLI + orchestration. Check: `cargo test` still green.
2. Add `.webp` to `is_supported_image()`; `optimize_webp()` = decode via `image`,
   re-encode lossless via `image` (pure Rust), keep-if-smaller like other formats.
   Check: new smoke test `webp_shrinks_or_stays`.
3. Add `webp` + `webp-animation` deps; new `src/convert.rs` with `--convert webp`:
   PNG/JPEG → lossy WebP (default q80, `--webp-quality` flag), animated GIF → animated
   WebP. Check: smoke tests assert sibling exists, decodes, and animated GIF variant is
   smaller than source; idempotency test (second run creates nothing).
4. README: soften "pure Rust" → "single static binary, no external tools"; document
   `--convert`; extend `bench/run.sh` with a variant-generation table.

### Phase 2 (v0.3) — AVIF
5. Add `ravif` (or `image` with `avif` feature); `--convert avif` generates `.avif`
   siblings (default quality ~60, `--avif-quality`). Warn in `--help` that AVIF encoding
   is CPU-heavy. Check: smoke test decodes header (magic bytes `ftypavif`), size sanity.
6. Bench: measure encode wall-time on the corpus; document in README (users must see the
   speed/size trade-off honestly).

### Phase 3 (v0.4) — HEIC input
7. Cargo feature `heic` (off by default, off in release binaries): `libheif-rs` decode →
   optimized JPEG sibling → existing JPEG conversion rules produce WebP/AVIF.
   Check: feature-gated test behind `#[cfg(feature = "heic")]`; CI job matrix adds one
   `--features heic` build on ubuntu (apt libheif-dev).
8. Document the feature build (`cargo install crunchit --features heic`) and why it's
   opt-in (system lib + patent-encumbered codecs).

## Validation (every phase)

- `cargo test` / `cargo clippy --all-targets -- -D warnings` / `cargo fmt --check` (CI).
- `bash bench/run.sh` on the local corpus; update README tables from real output only.
- Manual QA: run against a copy of a real asset directory (cedarandsage screenshots),
  verify in-place default still never creates files without `--convert`.

## Risks

- **Positioning drift:** libwebp/libheif erode the "pure Rust" pitch — mitigated by static
  linking and honest README wording; `ravif` keeps AVIF pure Rust.
- **AVIF encode time:** minutes for multi-MB images; must be opt-in, quality-capped, and
  documented, or the tool feels broken.
- **Release matrix:** libwebp cross-compiles fine; libheif does not — keep `heic` out of
  release binaries (hence the cargo feature).
- **Binary size:** rav1e adds several MB — acceptable for a CLI, note in README.
- **Semantics:** first release where crunchit *creates* files; idempotency + collision
  rules above are the guard against variant explosions on re-runs.

## Open questions

- Default conversion qualities (WebP 80 / AVIF 60 proposed — validate on the bench corpus).
- Should `--convert` imply optimizing the source too (current lean: yes, keep both).
- WordPress-style workflows may want `--convert heic:jpeg` only — is per-rule selection
  needed in v0.2, or is the fixed matrix enough until someone asks?
