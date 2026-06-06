# Image Converter

A Rust CLI tool for converting and optimizing images.

## Features

- Reads common image formats (PNG, JPEG, etc.)
- Optionally resizes images with configurable `--width`/`--height` (preserves aspect ratio, Lanczos3 filter)
- Encodes output as **WebP** or **AVIF** with adjustable quality
- Custom output filename via `--output-name` (useful for multiple compression variants)

## Usage

```bash
cargo run -- <INPUT> <OUTPUT> [OPTIONS]
```

- `INPUT` — path to the input image file
- `OUTPUT` — path to the output directory

### Options

| Flag | Description | Default |
|------|-------------|---------|
| `--format <webp\|avif>` | Output format | `webp` |
| `--quality <0–100>` | Encoding quality | `80` |
| `--width <px>` | Output width (height derived proportionally) | — |
| `--height <px>` | Output height (width derived proportionally) | — |
| `--output-name <name>` | Output filename stem (without extension) | input filename |

### Examples

```bash
# Convert to WebP (no resize)
cargo run -- input.png output/

# Convert with quality
cargo run -- input.png output/ --quality 90

# Convert to AVIF
cargo run -- input.png output/ --format avif

# Resize to width 400 (height proportional)
cargo run -- input.png output/ --width 400

# Custom output filename (useful for comparing compression levels)
cargo run -- input.png output/ --quality 80 --output-name img_q80
cargo run -- input.png output/ --quality 50 --output-name img_q50
```

The output file will be named `<input-stem>.<format>` inside the given directory,
or `<output-name>.<format>` when `--output-name` is provided.

## Development

```bash
make lint       # run clippy
make test       # run tests
make coverage   # run test coverage (requires cargo-llvm-cov)
make ci         # lint + test
```
