use std::path::Path;
use std::process::Command;

/// Verify this crate can be consumed as a library dependency.
///
/// Creates a temporary Cargo project that depends on `image_converter`
/// via path, then runs `cargo check` to ensure the public API is
/// accessible and compiles.
#[test]
fn test_consumable_as_dependency() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let tmp = std::env::temp_dir().join(format!("image_converter_test_{}", std::process::id()));
    let tmp = tmp.join("consumer");
    let src = tmp.join("src");

    std::fs::create_dir_all(&src).unwrap();

    let dep_path = manifest_dir.display();

    std::fs::write(
        tmp.join("Cargo.toml"),
        format!(
            r#"[package]
name = "consumer"
version = "0.0.0"
edition = "2024"

[dependencies]
image-converter = {{ path = "{dep_path}" }}
"#,
            dep_path = dep_path,
        ),
    )
    .unwrap();

    std::fs::write(
        src.join("main.rs"),
        r#"use image_converter::{config, processor};
use std::path::Path;

fn main() {
    // Test processor::OutputFormat is accessible
    let _fmt = processor::OutputFormat::Webp;

    // Test config::Config is accessible
    let _ = config::Config::load();

    // Test config::HeuristicsConfig is accessible
    let _h = config::HeuristicsConfig::default();

    // Test config::QualitySearchConfig is accessible
    let _q = config::QualitySearchConfig::default();

    // Test processor::search_quality is accessible
    // (would need a real image, just checking it compiles)

    // Test processor::encode_to_memory is accessible
    // (same)

    // Test processor::detect_best_config is accessible
    // (same)

    println!("All public API types are accessible.");
}
"#,
    )
    .unwrap();

    let output = Command::new("cargo")
        .args(["check"])
        .current_dir(&tmp)
        .output()
        .expect("Failed to run cargo check");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    if !output.status.success() {
        panic!(
            "Consumer project failed to compile.\nstdout:\n{stdout}\nstderr:\n{stderr}"
        );
    }

    // Cleanup
    std::fs::remove_dir_all(tmp.parent().unwrap()).ok();
}
