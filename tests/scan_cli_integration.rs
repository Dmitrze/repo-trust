//! End-to-end scan integration: invokes the compiled binary against a
//! wiremock server pointed at via `--api-base-url`.
//!
//! Day 1: only Activity Health is wired through `cli::scan::execute`. The
//! report has one module entry; this test confirms the JSON file is
//! written and parses back to a valid `TrustReport`.

use assert_cmd::Command;
use repo_trust::models::TrustReport;
use serde_json;
use tempfile::TempDir;
use wiremock::matchers::{method, path, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

const REPO_INACTIVE: &str = r#"{
    "full_name":"octocat/Hello-World",
    "html_url":"https://github.com/octocat/Hello-World",
    "default_branch":"master",
    "language":"C",
    "stargazers_count":80,
    "forks_count":9,
    "watchers_count":80,
    "open_issues_count":0,
    "archived":false,
    "has_issues":true,
    "created_at":"2011-01-26T19:01:12Z",
    "pushed_at":"2024-01-26T19:14:43Z"
}"#;

async fn fixture_server() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/octocat/Hello-World"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("ETag", "\"r\"")
                .insert_header("X-RateLimit-Remaining", "4999")
                .insert_header("X-RateLimit-Reset", "9999999999")
                .set_body_string(REPO_INACTIVE),
        )
        .mount(&server)
        .await;
    for sub in [
        "/repos/octocat/Hello-World/commits",
        "/repos/octocat/Hello-World/releases",
        "/repos/octocat/Hello-World/issues",
    ] {
        Mock::given(method("GET"))
            .and(path(sub))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("ETag", "\"x\"")
                    .insert_header("X-RateLimit-Remaining", "4999")
                    .insert_header("X-RateLimit-Reset", "9999999999")
                    .set_body_string("[]"),
            )
            .mount(&server)
            .await;
    }
    Mock::given(method("GET"))
        .and(path_regex(r"^/repos/octocat/Hello-World/pulls$"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("ETag", "\"x\"")
                .insert_header("X-RateLimit-Remaining", "4999")
                .insert_header("X-RateLimit-Reset", "9999999999")
                .set_body_string("[]"),
        )
        .mount(&server)
        .await;
    server
}

#[tokio::test]
async fn scan_writes_json_report_against_wiremock() {
    let server = fixture_server().await;
    let out = TempDir::new().unwrap();
    let cache_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("repo-trust").unwrap();
    let assert = cmd
        .args([
            "scan",
            "octocat/Hello-World",
            "--mode",
            "standard",
            "--output",
        ])
        .arg(out.path())
        .args(["--api-base-url"])
        .arg(server.uri())
        .env("HOME", cache_dir.path()) // so cache lands inside the tempdir
        .env_remove("GITHUB_TOKEN")
        .env_remove("REPO_TRUST_WEIGHTS__ACTIVITY") // hermetic
        .assert()
        .success();
    let _ = assert;

    let json_path = out.path().join("octocat_Hello-World.json");
    let bytes = std::fs::read(&json_path).expect("report file present");
    let report: TrustReport = serde_json::from_slice(&bytes).expect("valid TrustReport");
    assert_eq!(report.repository.full_name, "octocat/Hello-World");
    assert_eq!(
        report.modules.len(),
        1,
        "Day 1 wires only the activity module"
    );
    assert_eq!(report.modules[0].module, "activity");
    assert!(report.modules[0].score <= 10);
    assert!(!report.evidence.is_empty(), "should have evidence items");
}

#[test]
fn scan_with_invalid_url_exits_with_error() {
    let mut cmd = Command::cargo_bin("repo-trust").unwrap();
    cmd.args(["scan", "https://gitlab.com/owner/repo"])
        .assert()
        .failure();
}
