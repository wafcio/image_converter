# Image Converter

A Rust CLI tool for converting and optimizing images.

## Usage

```bash
cargo run -- <INPUT> <OUTPUT>
```

- `INPUT` — path to the input image file
- `OUTPUT` — path to the output directory

### Example

```bash
cargo run -- input.png output/
```

## Development

```bash
make lint       # run clippy
make test       # run tests
make coverage   # run test coverage (requires cargo-llvm-cov)
make ci         # lint + test
```
