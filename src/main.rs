use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "image-converter")]
#[command(about = "Converts and optimizes images")]
struct Cli {
    /// Path to the input image file
    input: String,

    /// Path to the output directory
    output: String,
}

fn run(cli: &Cli) {
    println!("Input file:  {}", cli.input);
    println!("Output dir:  {}", cli.output);
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
        assert_eq!(cli.input, "input.png");
        assert_eq!(cli.output, "output/");
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
