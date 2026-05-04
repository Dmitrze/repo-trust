//! Verifies `--format json,md,csv` dispatches to all three writers and
//! produces all three files. Reuses the all-five-modules wiremock fixture
//! pattern but pruned to the activity module so the fixture stays minimal.

use assert_cmd::Command;
use tempfile::TempDir;
use wiremock::matchers::{method, path, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

const REPO_BODY: &str = r#"{
    "full_name":"acme/widget",
    "html_url":"https://github.com/acme/widget",
    "default_branch":"main",
    "language":"Rust",
    "stargazers_count":250,
    "forks_count":25,
    "watchers_count":5,
    "open_issues_count":3,
    "archived":false,
    "has_issues":true,
    "created_at":"2018-01-01T00:00:00Z",
    "pushed_at":"2026-04-30T00:00:00Z"
}"#;

fn ok(body: &str) -> ResponseTemplate {
    ResponseTemplate::new(200)
        .insert_header("ETag", "\"x\"")
        .insert_header("X-RateLimit-Remaining", "4999")
        .insert_header("X-RateLimit-Reset", "9999999999")
        .set_body_string(body)
}

async fn fixture_server() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/acme/widget"))
        .respond_with(ok(REPO_BODY))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/repos/acme/widget/.*"))
        .respond_with(ok("[]"))
        .mount(&server)
        .await;
    server
}

#[tokio::test]
async fn format_dispatch_json_md_csv_produces_all_three_files() {
    let server = fixture_server().await;
    let out = TempDir::new().unwrap();
    let cache_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("repo-trust").unwrap();
    cmd.args([
        "scan",
        "acme/widget",
        "--mode",
        "standard",
        "--modules",
        "activity",
        "--format",
        "json,md,csv",
        "--output",
    ])
    .arg(out.path())
    .args(["--api-base-url"])
    .arg(server.uri())
    .env("HOME", cache_dir.path())
    .env_remove("GITHUB_TOKEN")
    .assert()
    .success();

    let json_path = out.path().join("acme_widget.json");
    let md_path = out.path().join("acme_widget.md");
    let csv_path = out.path().join("acme_widget.csv");

    assert!(json_path.exists(), "JSON report should be written");
    assert!(md_path.exists(), "Markdown report should be written");
    assert!(csv_path.exists(), "CSV report should be written");

    let md = std::fs::read_to_string(&md_path).unwrap();
    assert!(
        md.contains("# Trust Report"),
        "markdown should have H1 header, got: {}",
        &md[..md.len().min(200)]
    );
    let csv = std::fs::read_to_string(&csv_path).unwrap();
    assert!(
        csv.contains("acme/widget"),
        "CSV should contain the repo name"
    );
    assert!(
        csv.contains("full_name,url,overall_score"),
        "CSV header row missing"
    );
}
