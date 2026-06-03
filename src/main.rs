mod processor;

use clap::Parser;
use std::path::PathBuf;
use std::process;

#[derive(Debug, Parser)]
#[command(name = "image-converter")]
#[command(about = "Converts and optimizes images")]
struct Cli {
    /// Path to the input image file
    input: PathBuf,

    /// Path to the output directory
    output: PathBuf,
}

fn run(cli: &Cli) {
    if !cli.output.exists()
        && let Err(e) = std::fs::create_dir_all(&cli.output)
    {
        eprintln!("Error: failed to create output directory: {e}");
        process::exit(1);
    }

    match processor::process(&cli.input, &cli.output) {
        Ok(result) => {
            println!("Input:  {}", result.input_path.display());
            println!("Output: {}", result.output_path.display());
            println!(
                "Size:   {}×{} → {}×{}",
                result.original_width,
                result.original_height,
                result.final_width,
                result.final_height,
            );
        }
        Err(e) => {
            eprintln!("Error: failed to process image: {e}");
            process::exit(1);
        }
    }
}

fn main() {
    run(&Cli::parse());
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::error::ErrorKind;

    #[test]
    fn test_parse_valid_args() {
        let cli = Cli::try_parse_from(["image-converter", "input.png", "output/"])
            .expect("valid args should parse");
        assert_eq!(cli.input, PathBuf::from("input.png"));
        assert_eq!(cli.output, PathBuf::from("output/"));
    }

    #[test]
    fn test_parse_missing_args() {
        let err = Cli::try_parse_from(["image-converter", "input.png"])
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
