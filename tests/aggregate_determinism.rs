//! End-to-end determinism check (ADR-0007).
//!
//! Runs the full 3-module scan twice against the same wiremock fixture
//! and asserts byte-identical JSON output modulo `snapshot_at` and
//! `runtime_seconds` (the only fields explicitly excluded from determinism
//! per `architecture.md` §9).

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const REPO_BODY: &str = r#"{
    "full_name":"acme/widget",
    "html_url":"https://github.com/acme/widget",
    "default_branch":"main",
    "language":"Rust",
    "stargazers_count":1234,
    "forks_count":56,
    "watchers_count":1234,
    "open_issues_count":3,
    "archived":false,
    "has_issues":true,
    "created_at":"2018-01-01T00:00:00Z",
    "pushed_at":"2026-04-30T00:00:00Z"
}"#;

const SCORECARD_BODY: &str = r#"{"date":"2026-04-25T00:00:00Z","repo":{"name":"github.com/acme/widget","commit":"abc"},"score":7.5,"checks":[{"name":"Pinned-Dependencies","score":7,"reason":"ok","documentation":{}}]}"#;

fn ok(body: &str) -> ResponseTemplate {
    ResponseTemplate::new(200)
        .insert_header("ETag", "\"x\"")
        .insert_header("X-RateLimit-Remaining", "4999")
        .insert_header("X-RateLimit-Reset", "9999999999")
        .set_body_string(body)
}

fn nf() -> ResponseTemplate {
    ResponseTemplate::new(404).set_body_string("{}")
}

async fn fixture_server() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/acme/widget"))
        .respond_with(ok(REPO_BODY))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/acme/widget/commits"))
        .respond_with(ok("[]"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/acme/widget/releases"))
        .respond_with(ok(r#"[{"tag_name":"v1.0.0","name":"v1.0.0","draft":false,"prerelease":false,"created_at":"2026-01-01T00:00:00Z"}]"#))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/acme/widget/issues"))
        .respond_with(ok("[]"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/acme/widget/pulls"))
        .respond_with(ok("[]"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/acme/widget/contributors"))
        .respond_with(ok(
            r#"[{"login":"alice","contributions":50,"type":"User"}]"#,
        ))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/projects/github.com/acme/widget"))
        .respond_with(ok(SCORECARD_BODY))
        .mount(&server)
        .await;
    // Doc presence: license + codeowners present, others absent.
    Mock::given(method("GET"))
        .and(path("/repos/acme/widget/contents/LICENSE"))
        .respond_with(ok(r#"{"name":"LICENSE","type":"file"}"#))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/acme/widget/contents/.github/CODEOWNERS"))
        .respond_with(ok(r#"{"name":".github/CODEOWNERS","type":"file"}"#))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/acme/widget/contents/.github/workflows"))
        .respond_with(ok(r#"{"type":"dir"}"#))
        .mount(&server)
        .await;
    server
}

async fn run_scan_once(server_uri: &str, run_id: &str) -> Value {
    let out = TempDir::new().unwrap();
    let cache_dir = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("repo-trust").unwrap();
    cmd.args([
        "scan",
        "acme/widget",
        "--mode",
        "standard",
        "--modules",
        "activity,maintainers,security",
        "--output",
    ])
    .arg(out.path())
    .args(["--api-base-url"])
    .arg(server_uri)
    .env("HOME", cache_dir.path())
    .env_remove("GITHUB_TOKEN")
    .env_remove("REPO_TRUST_WEIGHTS__ACTIVITY")
    .assert()
    .success();

    let json_path = out.path().join("acme_widget.json");
    let bytes = std::fs::read(&json_path).unwrap_or_else(|e| panic!("read report ({run_id}): {e}"));
    serde_json::from_slice(&bytes).unwrap_or_else(|e| panic!("parse report ({run_id}): {e}"))
}

/// Strip the two fields explicitly excluded from determinism per
/// `architecture.md` §9 (`snapshot_at`, `runtime_seconds`).
fn redact_for_compare(mut v: Value) -> Value {
    if let Some(obj) = v.as_object_mut() {
        obj.insert("snapshot_at".into(), Value::String("[REDACTED]".into()));
        obj.insert("runtime_seconds".into(), Value::Number(0.into()));
        if let Some(repo) = obj.get_mut("repository").and_then(|r| r.as_object_mut()) {
            repo.insert("snapshot_at".into(), Value::String("[REDACTED]".into()));
        }
    }
    v
}

#[tokio::test]
async fn two_scans_against_same_fixture_produce_identical_json() {
    let server = fixture_server().await;
    let r1 = redact_for_compare(run_scan_once(&server.uri(), "r1").await);
    let r2 = redact_for_compare(run_scan_once(&server.uri(), "r2").await);

    assert_eq!(
        serde_json::to_string_pretty(&r1).unwrap(),
        serde_json::to_string_pretty(&r2).unwrap(),
        "two scans against the same fixture must produce byte-identical JSON (ADR-0007)"
    );
}
