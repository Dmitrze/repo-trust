//! End-to-end integration tests for the `repo-trust` binary.
//!
//! These tests use `assert_cmd` to invoke the compiled binary and check
//! its stdout/stderr and exit code. They do NOT make real API calls;
//! collector tests with mocked HTTP live in their own files.

use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn version_subcommand_prints_version_and_scoring() {
    let mut cmd = Command::cargo_bin("repo-trust").unwrap();
    cmd.arg("version")
        .assert()
        .success()
        .stdout(contains("repo-trust"))
        .stdout(contains("scoring-model"));
}

#[test]
fn help_subcommand_lists_all_subcommands() {
    let mut cmd = Command::cargo_bin("repo-trust").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(contains("scan"))
        .stdout(contains("batch"))
        .stdout(contains("explain"))
        .stdout(contains("cache"))
        .stdout(contains("config"))
        .stdout(contains("completions"));
}

#[test]
fn scan_help_shows_mode_flag() {
    let mut cmd = Command::cargo_bin("repo-trust").unwrap();
    cmd.args(["scan", "--help"])
        .assert()
        .success()
        .stdout(contains("--mode"))
        .stdout(contains("quick"))
        .stdout(contains("standard"))
        .stdout(contains("deep"));
}

#[test]
fn invalid_repo_url_returns_error() {
    // The scan handler currently returns "not yet implemented" — once it's wired
    // up, this test should be tightened to expect specific error text and
    // exit code 2 (Repository not found / not accessible) per architecture.md §8.
    let mut cmd = Command::cargo_bin("repo-trust").unwrap();
    cmd.args(["scan", "https://gitlab.com/owner/repo"])
        .assert()
        .failure();
}

#[test]
fn completions_bash_outputs_completion_script() {
    let mut cmd = Command::cargo_bin("repo-trust").unwrap();
    let output = cmd.args(["completions", "bash"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("repo-trust"),
        "bash completion script should mention the binary name"
    );
}
