# Image Converter

A Rust CLI tool for converting and optimizing images.

## Features

- Reads common image formats (PNG, JPEG, etc.)
- Resizes images wider than 800 px down to 800 px (preserves aspect ratio, Lanczos3 filter)
- Encodes output as **WebP** or **AVIF** with adjustable quality

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

### Example

```bash
cargo run -- input.png output/
cargo run -- input.png output/ --quality 90
cargo run -- input.png output/ --format avif
```

The output file will be named `<input-stem>.<format>` inside the given directory.

## Development

```bash
make lint       # run clippy
make test       # run tests
make coverage   # run test coverage (requires cargo-llvm-cov)
make ci         # lint + test
```
