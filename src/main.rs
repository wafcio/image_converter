use clap::Parser;

#[derive(Parser)]
#[command(name = "image-converter")]
#[command(about = "Converts and optimizes images")]
struct Cli {
    /// Path to the input image file
    input: String,

    /// Path to the output directory
    output: String,
}

fn main() {
    let cli = Cli::parse();

    println!("Input file:  {}", cli.input);
    println!("Output dir:  {}", cli.output);
}
