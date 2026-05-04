#![allow(
    clippy::unused_async,
    clippy::float_cmp,
    clippy::doc_lazy_continuation,
    clippy::unreadable_literal,
    clippy::too_many_lines
)]

//! Exit-code mapping tests per `docs/architecture.md` §8.
//!
//! - 401 from GitHub → exit 3 (authentication failure).
//! - 403 / rate-limit → exit 4 (rate-limit exceeded / forbidden).
//! These run the compiled binary against wiremock fixtures and assert the
//! process exit code, not just the error type.

use assert_cmd::Command;
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn unauthorized_repo_exits_with_code_3() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/private/repo"))
        .respond_with(
            ResponseTemplate::new(401).set_body_string("{\"message\":\"Bad credentials\"}"),
        )
        .mount(&server)
        .await;
    let cache_dir = TempDir::new().unwrap();
    let out = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("repo-trust").unwrap();
    cmd.args(["scan", "private/repo", "--modules", "activity", "--output"])
        .arg(out.path())
        .args(["--api-base-url"])
        .arg(server.uri())
        .env("HOME", cache_dir.path())
        .env_remove("GITHUB_TOKEN")
        .assert()
        .code(3);
}

#[tokio::test]
async fn rate_limit_exhausted_exits_with_code_4() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/heavy/repo"))
        .respond_with(
            ResponseTemplate::new(403).set_body_string("{\"message\":\"API rate limit exceeded\"}"),
        )
        .mount(&server)
        .await;
    let cache_dir = TempDir::new().unwrap();
    let out = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("repo-trust").unwrap();
    cmd.args(["scan", "heavy/repo", "--modules", "activity", "--output"])
        .arg(out.path())
        .args(["--api-base-url"])
        .arg(server.uri())
        .env("HOME", cache_dir.path())
        .env_remove("GITHUB_TOKEN")
        .assert()
        .code(4);
}
