use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn test_help_output() {
    let mut cmd = Command::cargo_bin("image-converter").unwrap();
    let assert = cmd.arg("--help").assert();
    assert
        .success()
        .stdout(contains("Converts and optimizes images"))
        .stdout(contains("<INPUT>"))
        .stdout(contains("<OUTPUT>"));
}

#[test]
fn test_valid_args_output() {
    let mut cmd = Command::cargo_bin("image-converter").unwrap();
    let assert = cmd.args(["input.png", "output/"]).assert();
    assert
        .success()
        .stdout(contains("Input file:  input.png"))
        .stdout(contains("Output dir:  output/"));
}

#[test]
fn test_missing_args_fails() {
    let mut cmd = Command::cargo_bin("image-converter").unwrap();
    let assert = cmd.arg("input.png").assert();
    assert
        .failure()
        .stderr(contains("required arguments were not provided"));
}
