use assert_cmd::Command;
use predicates::str::contains;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

const HEURISTIC_CONFIG: &str = r#"
[heuristics]
enabled = true

[heuristics.small]
max_width = 256
max_height = 256
lossless = true
quality = 100

[heuristics.large]
min_dimension = 1200
lossless = false
quality = 50

[heuristics.medium]
lossless = false
quality = 80
"#;

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
        .stdout(contains("--height"))
        .stdout(contains("--watch"));
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

fn create_small_png(dir: &Path) -> PathBuf {
    let path = dir.join("icon.png");
    let img = image::RgbaImage::new(64, 64);
    img.save(&path).unwrap();
    path
}

fn create_large_png(dir: &Path) -> PathBuf {
    let path = dir.join("photo.png");
    let img = image::RgbaImage::new(1920, 1080);
    img.save(&path).unwrap();
    path
}

#[test]
fn test_heuristic_with_config() {
    let workspace = test_dir("heuristic_on");
    let input = workspace.join("input");
    let output = workspace.join("output");
    std::fs::create_dir_all(&input).unwrap();

    create_small_png(&input);
    create_large_png(&input);

    std::fs::write(workspace.join("config.toml"), HEURISTIC_CONFIG).unwrap();

    let mut cmd = Command::cargo_bin("image-converter").unwrap();
    let assert = cmd
        .current_dir(&workspace)
        .args([
            input.to_str().unwrap(),
            output.to_str().unwrap(),
        ])
        .assert();
    assert.success().stdout(contains("Processed: 2 successful, 0 failed"));

    assert!(output.join("icon.webp").exists(), "icon output should exist");
    assert!(output.join("photo.webp").exists(), "photo output should exist");

    std::fs::remove_dir_all(&workspace).ok();
}

#[test]
fn test_heuristic_without_config() {
    let workspace = test_dir("heuristic_off");
    let input = workspace.join("input");
    let output = workspace.join("output");
    std::fs::create_dir_all(&input).unwrap();

    create_small_png(&input);
    create_large_png(&input);

    let mut cmd = Command::cargo_bin("image-converter").unwrap();
    let assert = cmd
        .current_dir(&workspace)
        .args([
            input.to_str().unwrap(),
            output.to_str().unwrap(),
        ])
        .assert();
    assert.success().stdout(contains("Processed: 2 successful, 0 failed"));

    assert!(output.join("icon.webp").exists(), "icon output should exist");
    assert!(output.join("photo.webp").exists(), "photo output should exist");

    std::fs::remove_dir_all(&workspace).ok();
}

#[test]
fn test_watch_processes_existing() {
    let dir = test_dir("watch_existing");
    let out_dir = dir.join("out");
    std::fs::create_dir_all(&dir).unwrap();
    create_test_png(&dir);

    let bin = assert_cmd::cargo::cargo_bin("image-converter");
    let mut child = std::process::Command::new(bin)
        .args([dir.to_str().unwrap(), out_dir.to_str().unwrap(), "--watch"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    std::thread::sleep(Duration::from_secs(4));

    let _ = child.kill();
    let _ = child.wait();

    assert!(
        out_dir.join("test.webp").exists(),
        "existing file should be processed in watch mode"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_watch_detects_new_file() {
    let dir = test_dir("watch_new");
    let out_dir = dir.join("out");
    std::fs::create_dir_all(&dir).unwrap();

    let bin = assert_cmd::cargo::cargo_bin("image-converter");
    let mut child = std::process::Command::new(bin)
        .args([dir.to_str().unwrap(), out_dir.to_str().unwrap(), "--watch"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    std::thread::sleep(Duration::from_secs(2));

    let img = image::RgbaImage::new(200, 150);
    img.save(dir.join("new.png")).unwrap();

    let deadline = Instant::now() + Duration::from_secs(8);
    let output_path = out_dir.join("new.webp");
    let mut found = false;
    while Instant::now() < deadline {
        if output_path.exists() {
            found = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(200));
    }

    let _ = child.kill();
    let _ = child.wait();

    assert!(found, "new file should be auto-processed in watch mode");

    std::fs::remove_dir_all(&dir).ok();
}

const QUALITY_SEARCH_CONFIG: &str = r"
[heuristics]
enabled = false

[quality_search]
enabled = true
";

#[test]
fn test_quality_search_with_config() {
    let workspace = test_dir("qsearch");
    let input = workspace.join("input");
    let output = workspace.join("output");
    std::fs::create_dir_all(&input).unwrap();

    create_test_png(&input);

    std::fs::write(workspace.join("config.toml"), QUALITY_SEARCH_CONFIG).unwrap();

    let mut cmd = Command::cargo_bin("image-converter").unwrap();
    let assert = cmd
        .current_dir(&workspace)
        .args([input.to_str().unwrap(), output.to_str().unwrap()])
        .assert();
    assert.success().stdout(contains("OK:"));

    assert!(
        output.join("test.webp").exists(),
        "webp output should exist with quality_search"
    );

    std::fs::remove_dir_all(&workspace).ok();
}
