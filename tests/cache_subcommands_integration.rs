#![allow(
    clippy::unused_async,
    clippy::float_cmp,
    clippy::doc_lazy_continuation,
    clippy::unreadable_literal,
    clippy::too_many_lines,
    dead_code
)]

//! Integration tests for `repo-trust cache info|clear|prune`.

use assert_cmd::Command;
use predicates::str::contains;
use tempfile::TempDir;

#[test]
fn cache_info_prints_path_and_row_counts() {
    let cache_dir = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("repo-trust").unwrap();
    cmd.args(["cache", "info"])
        .env("HOME", cache_dir.path())
        .assert()
        .success()
        .stdout(contains("Cache:"))
        .stdout(contains("api_cache rows:"))
        .stdout(contains("features rows:"))
        .stdout(contains("reports rows:"))
        .stdout(contains("soft cap:"));
}

#[test]
fn cache_clear_on_empty_is_a_noop() {
    let cache_dir = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("repo-trust").unwrap();
    cmd.args(["cache", "clear"])
        .env("HOME", cache_dir.path())
        .assert()
        .success()
        .stdout(contains("Cleared 0 entries"));
}

#[test]
fn cache_prune_on_empty_is_a_noop() {
    let cache_dir = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("repo-trust").unwrap();
    cmd.args(["cache", "prune"])
        .env("HOME", cache_dir.path())
        .assert()
        .success()
        .stdout(contains("Pruned 0 expired entries"));
}

#[test]
fn cache_clear_with_repo_scopes_to_repo() {
    let cache_dir = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("repo-trust").unwrap();
    cmd.args(["cache", "clear", "--repo", "octocat/Hello-World"])
        .env("HOME", cache_dir.path())
        .assert()
        .success()
        .stdout(contains("octocat/Hello-World"));
}

#[test]
fn cache_clear_all_clears_every_table() {
    let cache_dir = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("repo-trust").unwrap();
    cmd.args(["cache", "clear", "--all"])
        .env("HOME", cache_dir.path())
        .assert()
        .success()
        .stdout(contains("api_cache="))
        .stdout(contains("features="))
        .stdout(contains("reports="));
}
