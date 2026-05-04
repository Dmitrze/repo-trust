#![allow(
    clippy::unused_async,
    clippy::float_cmp,
    clippy::doc_lazy_continuation,
    clippy::unreadable_literal,
    clippy::too_many_lines,
    dead_code
)]

//! End-to-end integration test running ALL 5 modules against a single
//! wiremock fixture. Validates the Day-3 acceptance gate: "all 5 modules
//! return real `ModuleResult`".

use assert_cmd::Command;
use repo_trust::models::TrustReport;
use std::collections::HashSet;
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

const SCORECARD_BODY: &str = r#"{"date":"2026-04-25T00:00:00Z","repo":{"name":"github.com/acme/widget","commit":"abc"},"score":7.5,"checks":[{"name":"Pinned-Dependencies","score":7,"reason":"ok","documentation":{}}]}"#;

const README_BODY: &str = r#"{"name":"README.md","type":"file","encoding":"base64","content":"SGVsbG8gd29ybGQuIFRoaXMgaXMgYSByZWFkbWUgZmlsZSB3aXRoIGVub3VnaCB3b3JkcyB0byBjcm9zcyB0aGUgaGFsZi1jcmVkaXQgYmFuZC4gT25lIHR3byB0aHJlZSBmb3VyIGZpdmUgc2l4IHNldmVuIGVpZ2h0IG5pbmUgdGVuLg=="}"#;

const STARGAZERS_BODY: &str = r#"[
    {"starred_at":"2024-06-01T00:00:00Z","user":{"login":"u1","type":"User"}},
    {"starred_at":"2024-06-01T00:00:00Z","user":{"login":"u2","type":"User"}}
]"#;

fn user_profile(login: &str, healthy: bool) -> String {
    if healthy {
        format!(
            r#"{{"login":"{login}","created_at":"2020-01-01T00:00:00Z","followers":50,"following":20,"public_repos":15,"public_gists":2,"bio":"hello","blog":null,"email":null,"type":"User"}}"#
        )
    } else {
        format!(
            r#"{{"login":"{login}","created_at":"2024-06-01T00:00:00Z","followers":0,"following":0,"public_repos":1,"public_gists":0,"bio":null,"blog":null,"email":null,"type":"User"}}"#
        )
    }
}

fn ok(body: &str) -> ResponseTemplate {
    ResponseTemplate::new(200)
        .insert_header("ETag", "\"x\"")
        .insert_header("X-RateLimit-Remaining", "4999")
        .insert_header("X-RateLimit-Reset", "9999999999")
        .set_body_string(body)
}

async fn fixture_server() -> MockServer {
    let server = MockServer::start().await;
    // Repo metadata.
    Mock::given(method("GET"))
        .and(path("/repos/acme/widget"))
        .respond_with(ok(REPO_BODY))
        .mount(&server)
        .await;
    // Activity collector endpoints.
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
    // Maintainers + Security need contributors.
    Mock::given(method("GET"))
        .and(path("/repos/acme/widget/contributors"))
        .respond_with(ok(
            r#"[{"login":"alice","contributions":50,"type":"User"}]"#,
        ))
        .mount(&server)
        .await;
    // Stars: stargazers + per-user profiles.
    Mock::given(method("GET"))
        .and(path("/repos/acme/widget/stargazers"))
        .respond_with(ok(STARGAZERS_BODY))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/users/u1"))
        .respond_with(ok(&user_profile("u1", true)))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/users/u2"))
        .respond_with(ok(&user_profile("u2", true)))
        .mount(&server)
        .await;
    // Adoption: README + docs/examples + deps.dev (no packages).
    Mock::given(method("GET"))
        .and(path("/repos/acme/widget/readme"))
        .respond_with(ok(README_BODY))
        .mount(&server)
        .await;
    // Wildcard: any /repos/acme/widget/contents/* path that we don't define
    // explicitly returns 404 (file_exists handles this as `false`).
    Mock::given(method("GET"))
        .and(path_regex(r"^/repos/acme/widget/contents/.*$"))
        .respond_with(ResponseTemplate::new(404).set_body_string("{}"))
        .mount(&server)
        .await;
    // Doc-presence: have LICENSE + .github/CODEOWNERS + .github/workflows.
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
    // Scorecard.
    Mock::given(method("GET"))
        .and(path("/projects/github.com/acme/widget"))
        .respond_with(ok(SCORECARD_BODY))
        .mount(&server)
        .await;
    // deps.dev: no packages mapped (404 → empty Vec).
    Mock::given(method("GET"))
        .and(path("/v3/projects/github.com/acme/widget/packages"))
        .respond_with(ResponseTemplate::new(404).set_body_string("{}"))
        .mount(&server)
        .await;
    server
}

#[tokio::test]
async fn all_five_modules_run_end_to_end_against_wiremock() {
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
        "activity,maintainers,security,stars,adoption",
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
    let bytes = std::fs::read(&json_path).expect("report file present");
    let report: TrustReport = serde_json::from_slice(&bytes).expect("valid TrustReport");

    let module_names: HashSet<&str> = report.modules.iter().map(|m| m.module.as_str()).collect();
    assert!(module_names.contains("activity"));
    assert!(module_names.contains("maintainers"));
    assert!(module_names.contains("security"));
    assert!(module_names.contains("stars"));
    assert!(module_names.contains("adoption"));
    assert_eq!(
        report.modules.len(),
        5,
        "expected exactly 5 module results, got {}",
        report.modules.len()
    );

    // Each module emitted at least one piece of evidence.
    for m in &report.modules {
        let count = report
            .evidence
            .iter()
            .filter(|e| e.module == m.module)
            .count();
        assert!(
            count >= 3,
            "module {} emitted {} evidence items, expected ≥3",
            m.module,
            count
        );
    }

    // Aggregate produced a value in [0, 100] and a category.
    assert!(report.overall_score <= 100);
    assert!(matches!(
        report.category,
        repo_trust::models::Category::Strong
            | repo_trust::models::Category::Good
            | repo_trust::models::Category::Mixed
            | repo_trust::models::Category::Weak
            | repo_trust::models::Category::HighRisk
    ));
}
