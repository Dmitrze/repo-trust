//! End-to-end integration test for `MaintainersModule::run` against a
//! wiremock-backed GitHub fixture.
//!
//! Covers `tests/scenarios/maintainer-health-module.md`:
//! - S-002 (solo-maintainer flagged but not HighRisk)
//! - S-101 (bot commits excluded)
//! - the multi-maintainer healthy case (S-001) lands as a Day-5 fixture

use repo_trust::api::github::Client as GhClient;
use repo_trust::cli::scan::Mode;
use repo_trust::models::{ModuleWeights, RepositoryContext, Verdict};
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
    "stargazers_count":42,
    "forks_count":3,
    "watchers_count":42,
    "open_issues_count":1,
    "archived":false,
    "has_issues":true,
    "created_at":"2020-01-01T00:00:00Z",
    "pushed_at":"2026-04-30T00:00:00Z"
}"#;

/// 100 commits — 80 from `dependabot[bot]`, 20 from `alice`. Verifies the
/// bot-filter (S-101) and the solo-maintainer evidence path (S-002).
fn solo_with_bot_commits_body() -> String {
    let mut commits = Vec::new();
    // 80 bot commits (filtered out)
    for i in 0..80 {
        commits.push(format!(
            r#"{{"sha":"bot{i:03}","commit":{{"author":{{"name":"dependabot","date":"2026-01-01T00:00:00Z"}},"message":"deps"}},"author":{{"login":"dependabot[bot]","type":"Bot"}}}}"#
        ));
    }
    // 20 alice commits (the only human author)
    for i in 0..20 {
        commits.push(format!(
            r#"{{"sha":"a{i:03}","commit":{{"author":{{"name":"Alice","date":"2026-01-01T00:00:00Z"}},"message":"feat"}},"author":{{"login":"alice","type":"User"}}}}"#
        ));
    }
    format!("[{}]", commits.join(","))
}

const CONTRIBUTORS_BODY: &str = r#"[
    {"login":"alice","contributions":20,"type":"User"},
    {"login":"dependabot[bot]","contributions":80,"type":"Bot"}
]"#;

#[tokio::test]
async fn s002_solo_maintainer_flagged_but_not_highrisk() {
    let server = MockServer::start().await;
    let body = solo_with_bot_commits_body();
    Mock::given(method("GET"))
        .and(path("/repos/acme/widget"))
        .respond_with(rate_limited_200(REPO_BODY))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/acme/widget/commits"))
        .respond_with(rate_limited_200(&body))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/acme/widget/contributors"))
        .respond_with(rate_limited_200(CONTRIBUTORS_BODY))
        .mount(&server)
        .await;
    // All doc-presence probes 404 — no governance docs at all.
    for p in [
        "/repos/acme/widget/contents/CODEOWNERS",
        "/repos/acme/widget/contents/.github/CODEOWNERS",
        "/repos/acme/widget/contents/docs/CODEOWNERS",
        "/repos/acme/widget/contents/.gitlab/CODEOWNERS",
        "/repos/acme/widget/contents/MAINTAINERS.md",
        "/repos/acme/widget/contents/GOVERNANCE.md",
        "/repos/acme/widget/contents/GOVERNANCE",
        "/repos/acme/widget/contents/docs/GOVERNANCE.md",
    ] {
        Mock::given(method("GET"))
            .and(path(p))
            .respond_with(ResponseTemplate::new(404).set_body_string("{}"))
            .mount(&server)
            .await;
    }

    let dir = tempfile::tempdir().unwrap();
    let cache = Cache::open(dir.path().join("cache.db")).unwrap();
    let http = reqwest::Client::builder().build().unwrap();
    let github = GhClient::new(http.clone(), cache.clone(), RateLimiter::new(10), None)
        .with_base_url(server.uri());
    let scorecard = repo_trust::api::scorecard::Client::new(http.clone(), cache.clone())
        .with_base_url(server.uri());
    let osv = repo_trust::api::osv::Client::new(http, cache.clone()).with_base_url(server.uri());

    let ctx = RepositoryContext {
        full_name: "acme/widget".into(),
        canonical_url: url::Url::parse("https://github.com/acme/widget").unwrap(),
        mode: Mode::Standard,
        scoring_version: Version::parse("1.0.0").unwrap(),
        weights: ModuleWeights::default(),
        rng_seed: 0,
        snapshot_at: OffsetDateTime::parse(
            "2026-05-04T00:00:00Z",
            &time::format_description::well_known::Iso8601::DEFAULT,
        )
        .unwrap(),
        cache,
        github,
        scorecard,
        osv,
    };

    let module = repo_trust::modules::maintainers::MaintainersModule;
    let (result, evidence) = module.run(&ctx).await.expect("maintainers run");

    assert_eq!(result.module, "maintainers");
    assert!(
        evidence.len() >= 3,
        "expected ≥3 evidence items, got {}",
        evidence.len()
    );
    // Solo-maintainer surfaced as Concerning, NOT HighRisk.
    let solo = evidence
        .iter()
        .find(|e| e.code == "solo_maintainer")
        .expect("solo_maintainer evidence");
    assert!(matches!(solo.verdict, Verdict::Concerning));
    // Top authors evidence excludes the bot.
    let top = evidence
        .iter()
        .find(|e| e.code == "top_authors")
        .expect("top_authors evidence");
    let s = serde_json::to_string(&top.value).unwrap();
    assert!(s.contains("alice"), "top_authors should contain alice");
    assert!(
        !s.contains("dependabot"),
        "top_authors should not contain bot commits"
    );
}

fn rate_limited_200(body: &str) -> ResponseTemplate {
    ResponseTemplate::new(200)
        .insert_header("ETag", "\"x\"")
        .insert_header("X-RateLimit-Remaining", "4999")
        .insert_header("X-RateLimit-Reset", "9999999999")
        .set_body_string(body)
}
