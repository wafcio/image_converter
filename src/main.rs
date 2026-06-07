mod config;
mod processor;

use clap::Parser;
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::process;

#[derive(Debug, Parser)]
#[command(name = "image-converter")]
#[command(about = "Converts and optimizes images")]
struct Cli {
    /// Path to the input directory containing images
    input: PathBuf,

    /// Path to the output directory
    output: PathBuf,

    /// Output format (webp, avif)
    #[arg(long, default_value_t = processor::OutputFormat::Webp)]
    format: processor::OutputFormat,

    /// Encoding quality (0–100)
    #[arg(long, default_value_t = 80.0)]
    quality: f32,

    /// Output width (optional; height is derived proportionally)
    #[arg(long)]
    width: Option<u32>,

    /// Output height (optional; width is derived proportionally)
    #[arg(long)]
    height: Option<u32>,
}

fn is_image_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| matches!(e.to_lowercase().as_str(), "png" | "jpg" | "jpeg" | "gif" | "bmp" | "ico" | "tiff" | "tif" | "webp" | "avif"))
}

fn run(cli: &Cli, cfg: Option<&config::Config>) {
    if !cli.output.exists()
        && let Err(e) = std::fs::create_dir_all(&cli.output)
    {
        eprintln!("Error: failed to create output directory: {e}");
        process::exit(1);
    }

    let entries: Vec<PathBuf> = match std::fs::read_dir(&cli.input) {
        Ok(rd) => rd
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.is_file() && is_image_file(p))
            .collect(),
        Err(e) => {
            eprintln!("Error: failed to read input directory: {e}");
            process::exit(1);
        }
    };

    if entries.is_empty() {
        eprintln!("Error: no image files found in input directory");
        process::exit(1);
    }

    let heuristics = cfg.and_then(|c| if c.heuristics.enabled { Some(&c.heuristics) } else { None });

    let results: Vec<Result<processor::ProcessResult, Box<dyn std::error::Error + Send + Sync>>> = entries
        .par_iter()
        .map(|path| {
            processor::process(
                path,
                &cli.output,
                cli.format,
                cli.quality,
                cli.width,
                cli.height,
                None,
                heuristics,
            )
        })
        .collect();

    let mut success = 0;
    let mut failed = 0;
    for result in results {
        match result {
            Ok(r) => {
                let in_name = r.input_path.file_name().unwrap_or_default().to_string_lossy();
                let out_name = r.output_path.file_name().unwrap_or_default().to_string_lossy();
                println!(
                    "OK:    {in_name} → {out_name}  ({}×{} → {}×{})",
                    r.original_width, r.original_height, r.final_width, r.final_height,
                );
                success += 1;
            }
            Err(e) => {
                eprintln!("FAIL:  {e}");
                failed += 1;
            }
        }
    }

    println!("\nProcessed: {success} successful, {failed} failed");
}

fn main() {
    let cfg = config::Config::load();
    run(&Cli::parse(), cfg.as_ref());
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::error::ErrorKind;

    #[test]
    fn test_parse_valid_args() {
        let cli = Cli::try_parse_from(["image-converter", "input_dir", "output/"])
            .expect("valid args should parse");
        assert_eq!(cli.input, PathBuf::from("input_dir"));
        assert_eq!(cli.output, PathBuf::from("output/"));
    }

    #[test]
    fn test_parse_missing_args() {
        let err = Cli::try_parse_from(["image-converter", "input_dir"])
            .expect_err("missing output should fail");
        assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn test_parse_no_args() {
        let err = Cli::try_parse_from(["image-converter"])
            .expect_err("no args should fail");
        assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
    }
}
