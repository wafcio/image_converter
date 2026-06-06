use assert_cmd::Command;
use predicates::str::contains;
use std::path::PathBuf;

fn test_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("image-converter-test-{name}-{}", std::process::id()))
}

fn create_test_png(dir: &std::path::Path) -> PathBuf {
    let path = dir.join("test.png");
    let img = image::RgbaImage::new(100, 100);
    img.save(&path).unwrap();
    path
}

fn create_wide_test_png(dir: &std::path::Path) -> PathBuf {
    let path = dir.join("wide.png");
    let img = image::RgbaImage::new(400, 200);
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
        .stdout(contains("--format"))
        .stdout(contains("--quality"))
        .stdout(contains("--width"))
        .stdout(contains("--height"));
}

#[test]
fn test_process_webp() {
    let dir = test_dir("webp");
    let out_dir = dir.join("out");
    std::fs::create_dir_all(&dir).unwrap();
    create_test_png(&dir);

    let mut cmd = Command::cargo_bin("image-converter").unwrap();
    let assert = cmd
        .args([dir.to_str().unwrap(), out_dir.to_str().unwrap()])
        .assert();
    assert.success()
        .stdout(contains("OK:"))
        .stdout(contains("Processed:"));

    assert!(
        out_dir.join("test.webp").exists(),
        "webp output should exist"
    );
    assert!(
        out_dir.join("test.webp").metadata().unwrap().len() > 0,
        "webp output should not be empty"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_process_webp_with_quality() {
    let dir = test_dir("webp_q");
    let out_dir = dir.join("out_q");
    std::fs::create_dir_all(&dir).unwrap();
    create_test_png(&dir);

    let mut cmd = Command::cargo_bin("image-converter").unwrap();
    let assert = cmd
        .args([
            dir.to_str().unwrap(),
            out_dir.to_str().unwrap(),
            "--quality",
            "50",
        ])
        .assert();
    assert.success().stdout(contains("OK:"));

    assert!(
        out_dir.join("test.webp").exists(),
        "webp output should exist"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_process_avif() {
    let dir = test_dir("avif");
    let out_dir = dir.join("out_avif");
    std::fs::create_dir_all(&dir).unwrap();
    create_test_png(&dir);

    let mut cmd = Command::cargo_bin("image-converter").unwrap();
    let assert = cmd
        .args([
            dir.to_str().unwrap(),
            out_dir.to_str().unwrap(),
            "--format",
            "avif",
        ])
        .assert();
    assert.success().stdout(contains("OK:"));

    assert!(
        out_dir.join("test.avif").exists(),
        "avif output should exist"
    );
    assert!(
        out_dir.join("test.avif").metadata().unwrap().len() > 0,
        "avif output should not be empty"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_resize_width_only() {
    let dir = test_dir("resize_w");
    let out_dir = dir.join("out_w");
    std::fs::create_dir_all(&dir).unwrap();
    create_wide_test_png(&dir);

    let mut cmd = Command::cargo_bin("image-converter").unwrap();
    let assert = cmd
        .args([
            dir.to_str().unwrap(),
            out_dir.to_str().unwrap(),
            "--width",
            "200",
        ])
        .assert();
    assert.success().stdout(contains("200×100"));

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_resize_height_only() {
    let dir = test_dir("resize_h");
    let out_dir = dir.join("out_h");
    std::fs::create_dir_all(&dir).unwrap();
    create_wide_test_png(&dir);

    let mut cmd = Command::cargo_bin("image-converter").unwrap();
    let assert = cmd
        .args([
            dir.to_str().unwrap(),
            out_dir.to_str().unwrap(),
            "--height",
            "100",
        ])
        .assert();
    assert.success().stdout(contains("200×100"));

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_resize_both() {
    let dir = test_dir("resize_both");
    let out_dir = dir.join("out_both");
    std::fs::create_dir_all(&dir).unwrap();
    create_wide_test_png(&dir);

    let mut cmd = Command::cargo_bin("image-converter").unwrap();
    let assert = cmd
        .args([
            dir.to_str().unwrap(),
            out_dir.to_str().unwrap(),
            "--width",
            "100",
            "--height",
            "50",
        ])
        .assert();
    assert.success().stdout(contains("100×50"));

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_no_resize_by_default() {
    let dir = test_dir("noresize");
    let out_dir = dir.join("out_noresize");
    std::fs::create_dir_all(&dir).unwrap();
    create_wide_test_png(&dir);

    let mut cmd = Command::cargo_bin("image-converter").unwrap();
    let assert = cmd
        .args([dir.to_str().unwrap(), out_dir.to_str().unwrap()])
        .assert();
    // Without --width/--height, dimensions stay the same (400×200)
    assert.success().stdout(contains("400×200"));

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_missing_args_fails() {
    let mut cmd = Command::cargo_bin("image-converter").unwrap();
    let assert = cmd.arg("input_dir").assert();
    assert
        .failure()
        .stderr(contains("required arguments were not provided"));
}

#[test]
fn test_empty_dir_fails() {
    let dir = test_dir("empty");
    let out_dir = dir.join("out");
    std::fs::create_dir_all(&dir).unwrap();

    let mut cmd = Command::cargo_bin("image-converter").unwrap();
    let assert = cmd
        .args([dir.to_str().unwrap(), out_dir.to_str().unwrap()])
        .assert();
    assert
        .failure()
        .stderr(contains("no image files found"));

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_multiple_files() {
    let dir = test_dir("multi");
    let out_dir = dir.join("out_multi");
    std::fs::create_dir_all(&dir).unwrap();
    create_test_png(&dir);
    create_wide_test_png(&dir);

    let mut cmd = Command::cargo_bin("image-converter").unwrap();
    let assert = cmd
        .args([dir.to_str().unwrap(), out_dir.to_str().unwrap()])
        .assert();
    assert
        .success()
        .stdout(contains("Processed: 2 successful, 0 failed"));

    assert!(out_dir.join("test.webp").exists());
    assert!(out_dir.join("wide.webp").exists());

    std::fs::remove_dir_all(&dir).ok();
}
