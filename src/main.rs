use clap::Parser;
use image_converter::{config, processor};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process;
use std::time::{Duration, Instant};

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

    /// Watch input directory for new files and process them automatically
    #[arg(long, short)]
    watch: bool,

    /// AVIF encoder speed (1–10, lower = slower/better compression, higher = faster)
    #[arg(long, default_value_t = 6)]
    speed: u8,
}

fn is_image_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| matches!(e.to_lowercase().as_str(), "png" | "jpg" | "jpeg" | "gif" | "bmp" | "ico" | "tiff" | "tif" | "webp" | "avif"))
}

#[allow(clippy::too_many_arguments)]
fn process_single(
    path: &Path,
    output: &Path,
    format: processor::OutputFormat,
    quality: f32,
    width: Option<u32>,
    height: Option<u32>,
    heuristics: Option<&config::HeuristicsConfig>,
    quality_search: Option<&config::QualitySearchConfig>,
    speed: u8,
) -> Result<processor::ProcessResult, String> {
    processor::process(path, output, format, quality, width, height, None, heuristics, quality_search, speed)
        .map_err(|e| format!("{}: {e}", path.file_name().unwrap_or_default().to_string_lossy()))
}

fn print_result(result: &Result<processor::ProcessResult, String>) {
    match result {
        Ok(r) => {
            let in_name = r.input_path.file_name().unwrap_or_default().to_string_lossy();
            let out_name = r.output_path.file_name().unwrap_or_default().to_string_lossy();
            println!(
                "OK:    {in_name} → {out_name}  ({}×{} → {}×{})",
                r.original_width, r.original_height, r.final_width, r.final_height,
            );
        }
        Err(e) => eprintln!("FAIL:  {e}"),
    }
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
    let quality_search = cfg.as_ref().and_then(|c| if c.quality_search.enabled { Some(&c.quality_search) } else { None });

    let results: Vec<Result<processor::ProcessResult, String>> = entries
        .par_iter()
        .map(|path| process_single(path, &cli.output, cli.format, cli.quality, cli.width, cli.height, heuristics, quality_search, cli.speed))
        .collect();

    let mut success = 0;
    let mut failed = 0;
    for result in &results {
        print_result(result);
        if result.is_ok() { success += 1; } else { failed += 1; }
    }

    println!("\nProcessed: {success} successful, {failed} failed");
}

fn run_watch(cli: &Cli, cfg: Option<&config::Config>) {
    // Ensure output directory exists
    if !cli.output.exists()
        && let Err(e) = std::fs::create_dir_all(&cli.output)
    {
        eprintln!("Error: failed to create output directory: {e}");
        process::exit(1);
    }

    let heuristics = cfg.and_then(|c| if c.heuristics.enabled { Some(&c.heuristics) } else { None });
    let quality_search = cfg.as_ref().and_then(|c| if c.quality_search.enabled { Some(&c.quality_search) } else { None });

    // Process existing files first
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

    if !entries.is_empty() {
        eprintln!("Processing existing files...");
            let results: Vec<Result<processor::ProcessResult, String>> = entries
                .par_iter()
                .map(|path| process_single(path, &cli.output, cli.format, cli.quality, cli.width, cli.height, heuristics, quality_search, cli.speed))
                .collect();

            let mut success = 0;
            let mut failed = 0;
            for result in &results {
                print_result(result);
                if result.is_ok() { success += 1; } else { failed += 1; }
            }
            println!("\nProcessed: {success} successful, {failed} failed");
        }

    // Set up file watcher
    let (tx, rx) = std::sync::mpsc::channel::<Result<Event, notify::Error>>();
    let mut watcher: RecommendedWatcher = RecommendedWatcher::new(
        move |res| {
            let _ = tx.send(res);
        },
        notify::Config::default(),
    )
    .unwrap_or_else(|e| {
        eprintln!("Error: failed to create file watcher: {e}");
        process::exit(1);
    });

    if let Err(e) = watcher.watch(&cli.input, RecursiveMode::NonRecursive) {
        eprintln!("Error: failed to watch directory: {e}");
        process::exit(1);
    }

    println!("\nWatching {} for new images... (Ctrl+C to stop)", cli.input.display());

    // Debounce tracking: file → last event time
    let mut pending: HashMap<PathBuf, Instant> = HashMap::new();
    let debounce_timeout = Duration::from_millis(1200);

    loop {
        // Check for pending files that are ready
        let now = Instant::now();
        let ready: Vec<PathBuf> = pending
            .iter()
            .filter(|(_, t)| now.duration_since(**t) >= debounce_timeout)
            .map(|(p, _)| p.clone())
            .collect();

        for path in &ready {
            pending.remove(path);

            if !path.exists() || !is_image_file(path) {
                continue;
            }

            match process_single(path, &cli.output, cli.format, cli.quality, cli.width, cli.height, heuristics, quality_search, cli.speed) {
                Ok(r) => print_result(&Ok(r)),
                Err(e) => eprintln!("FAIL:  {e}"),
            }
        }

        // Wait for new events with a short timeout so pending checks run
        match rx.recv_timeout(Duration::from_millis(500)) {
            Ok(Ok(event)) => {
                if matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_)) {
                    for path in event.paths {
                        if is_image_file(&path) {
                            pending.insert(path, Instant::now());
                        }
                    }
                }
            }
            Ok(Err(e)) => eprintln!("Watch error: {e}"),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => { /* normal */ }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                eprintln!("Watch channel disconnected");
                break;
            }
        }
    }
}

fn main() {
    let cli = Cli::parse();
    let cfg = config::Config::load();

    if cli.watch {
        run_watch(&cli, cfg.as_ref());
    } else {
        run(&cli, cfg.as_ref());
    }
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
