//! End-to-end integration tests for `SecurityModule::run` against a
//! wiremock-backed federation of GitHub + Scorecard.
//!
//! Covers `tests/scenarios/security-readiness-module.md`:
//! - S-001 (fresh Scorecard scores high)
//! - S-002 (no Scorecard falls back to Low confidence + doc-only signals)

use repo_trust::api::github::Client as GhClient;
use repo_trust::api::osv::Client as OsvClient;
use repo_trust::api::scorecard::Client as ScorecardClient;
use repo_trust::cli::scan::Mode;
use repo_trust::models::{Confidence, ModuleWeights, RepositoryContext};
use repo_trust::modules::TrustModule as _;
use repo_trust::storage::Cache;
use repo_trust::utils::ratelimit::RateLimiter;
use semver::Version;
use time::OffsetDateTime;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const REPO_BODY: &str = r#"{
    "full_name":"acme/widget",
    "html_url":"https://github.com/acme/widget",
    "default_branch":"main",
    "language":"Rust",
    "stargazers_count":4200,
    "forks_count":300,
    "watchers_count":4200,
    "open_issues_count":12,
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

fn nf() -> ResponseTemplate {
    ResponseTemplate::new(404).set_body_string("{}")
}

async fn shared_github_mocks(server: &MockServer, license_present: bool) {
    Mock::given(method("GET"))
        .and(path("/repos/acme/widget"))
        .respond_with(ok(REPO_BODY))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/acme/widget/releases"))
        .respond_with(ok(r#"[{"tag_name":"v1.0.0","name":"v1.0.0","draft":false,"prerelease":false,"created_at":"2026-01-01T00:00:00Z"}]"#))
        .mount(server)
        .await;

    // Doc presence — license toggleable per scenario.
    let security_md = ok(r#"{"name":"SECURITY.md","type":"file"}"#);
    let contributing = ok(r#"{"name":"CONTRIBUTING.md","type":"file"}"#);
    let coc = ok(r#"{"name":"CODE_OF_CONDUCT.md","type":"file"}"#);
    let codeowners = ok(r#"{"name":".github/CODEOWNERS","type":"file"}"#);
    let workflows = ok(r#"{"name":".github/workflows","type":"dir"}"#);

    Mock::given(method("GET"))
        .and(path("/repos/acme/widget/contents/SECURITY.md"))
        .respond_with(security_md)
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/acme/widget/contents/CONTRIBUTING.md"))
        .respond_with(contributing)
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/acme/widget/contents/CODE_OF_CONDUCT.md"))
        .respond_with(coc)
        .mount(server)
        .await;
    if license_present {
        Mock::given(method("GET"))
            .and(path("/repos/acme/widget/contents/LICENSE"))
            .respond_with(ok(r#"{"name":"LICENSE","type":"file"}"#))
            .mount(server)
            .await;
    } else {
        for p in [
            "/repos/acme/widget/contents/LICENSE",
            "/repos/acme/widget/contents/LICENSE.md",
            "/repos/acme/widget/contents/LICENSE.txt",
            "/repos/acme/widget/contents/COPYING",
        ] {
            Mock::given(method("GET"))
                .and(path(p))
                .respond_with(nf())
                .mount(server)
                .await;
        }
    }
    Mock::given(method("GET"))
        .and(path("/repos/acme/widget/contents/.github/CODEOWNERS"))
        .respond_with(codeowners)
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/acme/widget/contents/.github/workflows"))
        .respond_with(workflows)
        .mount(server)
        .await;
}

fn fresh_scorecard_body() -> String {
    let now = OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Iso8601::DEFAULT)
        .unwrap();
    format!(
        r#"{{"date":"{now}","repo":{{"name":"github.com/acme/widget","commit":"abc"}},"score":8.7,"checks":[{{"name":"Binary-Artifacts","score":10,"reason":"ok","documentation":{{}}}},{{"name":"Pinned-Dependencies","score":6,"reason":"ok","documentation":{{}}}}]}}"#
    )
}

async fn build_ctx(server_uri: String) -> (RepositoryContext, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let cache = Cache::open(dir.path().join("cache.db")).unwrap();
    let http = reqwest::Client::builder().build().unwrap();
    let github = GhClient::new(http.clone(), cache.clone(), RateLimiter::new(10), None)
        .with_base_url(&server_uri);
    let scorecard = ScorecardClient::new(http.clone(), cache.clone()).with_base_url(&server_uri);
    let osv = OsvClient::new(http, cache.clone()).with_base_url(&server_uri);
    let ctx = RepositoryContext {
        full_name: "acme/widget".into(),
        canonical_url: url::Url::parse("https://github.com/acme/widget").unwrap(),
        mode: Mode::Standard,
        scoring_version: Version::parse("1.0.0").unwrap(),
        weights: ModuleWeights::default(),
        rng_seed: 0,
        snapshot_at: OffsetDateTime::now_utc(),
        cache,
        github,
        scorecard,
        osv,
    };
    (ctx, dir)
}

#[tokio::test]
async fn s001_fresh_scorecard_scores_high() {
    let server = MockServer::start().await;
    shared_github_mocks(&server, true).await;
    Mock::given(method("GET"))
        .and(path("/projects/github.com/acme/widget"))
        .respond_with(ok(&fresh_scorecard_body()))
        .mount(&server)
        .await;

    let (ctx, _dir) = build_ctx(server.uri()).await;
    let module = repo_trust::modules::security::SecurityModule;
    let (result, evidence) = module.run(&ctx).await.expect("security run");

    assert_eq!(result.module, "security");
    assert!(
        result.score >= 75,
        "expected ≥75 with fresh Scorecard + all docs, got {}",
        result.score
    );
    assert_eq!(result.confidence, Confidence::High);
    assert!(evidence.iter().any(|e| e.code == "scorecard_score"));
    assert!(evidence.iter().any(|e| e.code == "documentation_presence"));
    assert!(evidence.iter().any(|e| e.code == "ci_workflow_present"));
}

#[tokio::test]
async fn s002_no_scorecard_falls_back_with_low_confidence() {
    let server = MockServer::start().await;
    shared_github_mocks(&server, false).await;
    // Scorecard 404.
    Mock::given(method("GET"))
        .and(path("/projects/github.com/acme/widget"))
        .respond_with(nf())
        .mount(&server)
        .await;

    let (ctx, _dir) = build_ctx(server.uri()).await;
    let module = repo_trust::modules::security::SecurityModule;
    let (result, evidence) = module.run(&ctx).await.expect("security run");

    assert_eq!(result.confidence, Confidence::Low);
    assert!(result.missing_data.iter().any(|m| m == "scorecard"));
    assert!(evidence.iter().any(|e| e.code == "scorecard_unavailable"));
    assert!(
        evidence.iter().any(|e| e.code == "has_license"
            && matches!(e.verdict, repo_trust::models::Verdict::Concerning)),
        "missing-LICENSE should surface as Concerning evidence"
    );
}
