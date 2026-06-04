use assert_cmd::Command;
use predicates::str::contains;
use std::path::PathBuf;

fn test_dir() -> PathBuf {
    std::env::temp_dir().join(format!("image-converter-test-{}", std::process::id()))
}

fn create_test_png(dir: &std::path::Path) -> PathBuf {
    let path = dir.join("test.png");
    let img = image::RgbaImage::new(100, 100);
    img.save(&path).unwrap();
    path
}

#[test]
fn test_help_output() {
    let mut cmd = Command::cargo_bin("image-converter").unwrap();
    let assert = cmd.arg("--help").assert();
    assert
        .success()
        .stdout(contains("Converts and optimizes images"))
        .stdout(contains("<INPUT>"))
        .stdout(contains("<OUTPUT>"))
        .stdout(contains("--quality"));
}

#[test]
fn test_process_image() {
    let dir = test_dir();
    let out_dir = dir.join("out");
    std::fs::create_dir_all(&dir).unwrap();

    let input = create_test_png(&dir);

    let mut cmd = Command::cargo_bin("image-converter").unwrap();
    let assert = cmd
        .args([
            input.to_str().unwrap(),
            out_dir.to_str().unwrap(),
        ])
        .assert();
    assert.success().stdout(contains("Size:"));

    assert!(out_dir.join("test.webp").exists(), "output file should exist");
    assert!(out_dir.join("test.webp").metadata().unwrap().len() > 0, "output should not be empty");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_process_with_quality() {
    let dir = test_dir();
    let out_dir = dir.join("out_q");
    std::fs::create_dir_all(&dir).unwrap();

    let input = create_test_png(&dir);

    let mut cmd = Command::cargo_bin("image-converter").unwrap();
    let assert = cmd
        .args([
            input.to_str().unwrap(),
            out_dir.to_str().unwrap(),
            "--quality",
            "50",
        ])
        .assert();
    assert.success().stdout(contains("Size:"));

    assert!(out_dir.join("test.webp").exists(), "output file should exist");
    assert!(out_dir.join("test.webp").metadata().unwrap().len() > 0, "output should not be empty");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_missing_args_fails() {
    let mut cmd = Command::cargo_bin("image-converter").unwrap();
    let assert = cmd.arg("input.png").assert();
    assert
        .failure()
        .stderr(contains("required arguments were not provided"));
}
