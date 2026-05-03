//! End-to-end integration test for `ActivityModule::run` against
//! a wiremock-backed GitHub fixture.
//!
//! Covers `tests/scenarios/activity-health-module.md` S-001 (inactive
//! repo scores low) — the prometheus + rust-lang/cargo fixtures land on
//! Day 5 alongside the snapshot tests.

use std::sync::Arc;

use repo_trust::api::github::Client as GhClient;
use repo_trust::cli::scan::Mode;
use repo_trust::models::{ModuleWeights, RepositoryContext};
use repo_trust::modules::TrustModule as _;
use repo_trust::storage::Cache;
use repo_trust::utils::ratelimit::RateLimiter;
use semver::Version;
use time::OffsetDateTime;
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

#[tokio::test]
async fn s001_inactive_repo_scores_low() {
    let server = MockServer::start().await;
    let _ = Arc::new(()); // keep clippy happy
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
    Mock::given(method("GET"))
        .and(path("/repos/octocat/Hello-World/commits"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("ETag", "\"c\"")
                .insert_header("X-RateLimit-Remaining", "4998")
                .insert_header("X-RateLimit-Reset", "9999999999")
                .set_body_string("[]"),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/octocat/Hello-World/releases"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("ETag", "\"rl\"")
                .insert_header("X-RateLimit-Remaining", "4997")
                .insert_header("X-RateLimit-Reset", "9999999999")
                .set_body_string("[]"),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/octocat/Hello-World/issues"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("ETag", "\"i\"")
                .insert_header("X-RateLimit-Remaining", "4996")
                .insert_header("X-RateLimit-Reset", "9999999999")
                .set_body_string("[]"),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/repos/octocat/Hello-World/pulls$"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("ETag", "\"p\"")
                .insert_header("X-RateLimit-Remaining", "4995")
                .insert_header("X-RateLimit-Reset", "9999999999")
                .set_body_string("[]"),
        )
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let cache = Cache::open(dir.path().join("cache.db")).unwrap();
    let http = reqwest::Client::builder().build().unwrap();
    let github =
        GhClient::new(http, cache.clone(), RateLimiter::new(10), None).with_base_url(server.uri());

    let ctx = RepositoryContext {
        full_name: "octocat/Hello-World".into(),
        canonical_url: url::Url::parse("https://github.com/octocat/Hello-World").unwrap(),
        mode: Mode::Standard,
        scoring_version: Version::parse("1.0.0").unwrap(),
        weights: ModuleWeights::default(),
        rng_seed: 0,
        snapshot_at: OffsetDateTime::parse(
            "2026-05-03T00:00:00Z",
            &time::format_description::well_known::Iso8601::DEFAULT,
        )
        .unwrap(),
        cache,
        github,
    };

    let module = repo_trust::modules::activity::ActivityModule;
    let (result, evidence) = module.run(&ctx).await.expect("activity run");

    assert_eq!(result.module, "activity");
    assert!(
        result.score <= 10,
        "expected score ≤10 for inactive Hello-World, got {}",
        result.score
    );
    assert!(
        evidence.len() >= 3,
        "expected ≥3 evidence items, got {}",
        evidence.len()
    );
    // Confidence: not archived, repo created 2011 (very old), missing only
    // benign "no_releases" → High.
    assert_eq!(result.confidence, repo_trust::models::Confidence::High);
}
