# Image Converter

A Rust CLI tool for batch converting and optimizing images using all CPU cores.

## Features

- Batch processes all images in a directory (parallel via rayon)
- Supports common formats: PNG, JPEG, GIF, BMP, TIFF, WebP, AVIF
- Optionally resizes images with configurable `--width`/`--height` (Lanczos3 filter)
- Encodes output as **WebP** or **AVIF** with adjustable quality

## Usage

```bash
cargo run -- <INPUT> <OUTPUT> [OPTIONS]
```

- `INPUT` — path to the input directory containing images
- `OUTPUT` — path to the output directory (created if missing)

### Options

| Flag | Description | Default |
|------|-------------|---------|
| `--format <webp\|avif>` | Output format | `webp` |
| `--quality <0–100>` | Encoding quality | `80` |
| `--width <px>` | Output width (height derived proportionally) | — |
| `--height <px>` | Output height (width derived proportionally) | — |
| `--watch` / `-w` | Watch input directory for new files and process them automatically | — |

### Examples

```bash
# Convert all images in a directory to WebP (no resize)
cargo run -- input_dir output/

# Convert to AVIF with quality 90
cargo run -- input_dir output/ --format avif --quality 90

# Resize all to width 400 (height proportional)
cargo run -- input_dir output/ --width 400

# Process 30 large photos on all CPU cores
cargo run -- photos/ compressed/
```

### Watch Mode

Start the program in the background — it processes all existing files, then watches the input directory for new images and converts them automatically:

```bash
cargo run -- input_dir output/ --watch

# Or with short flag:
cargo run -- input_dir output/ -w
```

While watching, drag or copy new images into the input directory — the converter picks them up within ~1.5 seconds. Press `Ctrl+C` to stop.

Each output file is named `<input-stem>.<format>` inside the given directory.

### Quality Search

Instead of using a fixed `--quality`, you can enable automatic quality search. The processor encodes each image in memory at quality levels 70, 80, and 90, then picks the level where stepping higher gives less than 15% size reduction (diminishing returns).

```toml
[quality_search]
enabled = true
```

When `quality_search` is enabled, it overrides `--quality` for lossy encodes.

### Auto-configuration

By default all images are encoded with the same `--quality`. To automatically tune parameters per image, place a `config.toml` in the working directory:

```toml
[heuristics]
enabled = true

[heuristics.small]
max_width = 256
max_height = 256
lossless = true
quality = 100

[heuristics.large]
min_dimension = 1200
min_file_size = 1048576   # 1 MB
lossless = false
quality = 75

[heuristics.medium]
lossless = false
quality = 80

[quality_search]
enabled = false
```

**Small** images (icons, thumbnails) use lossless WebP — no quality loss.
**Large** images (photos over 1200px or 1 MB) use aggressive lossy compression.
**Medium** images fall back to the default quality.

Without `config.toml`, heuristics and quality search are disabled, and `--quality` applies uniformly.

## Building

```bash
# Build a debug binary (target/debug/image-converter)
cargo build

# Build a release binary (target/release/image-converter) — optimized and recommended for use
cargo build --release
```

The resulting binary is self-contained and can be copied anywhere or linked into your `$PATH`.

## Development

```bash
make lint       # run clippy
make test       # run tests
make coverage   # run test coverage (requires cargo-llvm-cov)
make ci         # lint + test
```
