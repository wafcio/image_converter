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

Each output file is named `<input-stem>.<format>` inside the given directory.

## Development

```bash
make lint       # run clippy
make test       # run tests
make coverage   # run test coverage (requires cargo-llvm-cov)
make ci         # lint + test
```
