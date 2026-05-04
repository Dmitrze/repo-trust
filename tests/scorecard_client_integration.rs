#![allow(
    clippy::unused_async,
    clippy::float_cmp,
    clippy::doc_lazy_continuation,
    clippy::unreadable_literal,
    clippy::too_many_lines,
    dead_code
)]

//! Integration tests for `repo_trust::api::scorecard::Client`.
//!
//! Covers `tests/scenarios/scorecard-client.md`:
//! - S-001: 200 OK returns parsed `ScorecardReport`.
//! - S-101: cache hit serves without network.
//! - S-102: 404 returns `Ok(None)`.
//! - S-201: 5xx returns `Err`.
//! - S-202: malformed JSON returns `Err` whose chain mentions parsing.

use repo_trust::api::scorecard::{Client, ScorecardError};
use repo_trust::storage::Cache;
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// A realistic Scorecard JSON body for `prometheus/prometheus`. Field shape
/// mirrors the live `https://api.scorecard.dev/projects/github.com/prometheus/prometheus`
/// response (truncated to 12 checks for readability — still ≥10 so S-001
/// length assertions hold).
const PROMETHEUS_BODY: &str = r#"{
    "date": "2026-04-21T07:18:34Z",
    "repo": {
        "name": "github.com/prometheus/prometheus",
        "commit": "abc1234567890abcdef1234567890abcdef12345"
    },
    "score": 8.7,
    "checks": [
        {"name":"Binary-Artifacts","score":10,"reason":"no binaries found in the repo","documentation":{"short":"Determines if the project has generated executable (binary) artifacts in the source repository.","url":"https://github.com/ossf/scorecard/blob/main/docs/checks.md#binary-artifacts"}},
        {"name":"Branch-Protection","score":8,"reason":"branch protection is partially enabled","documentation":{"short":"Determines if the default and release branches are protected with GitHub's branch protection settings.","url":"https://github.com/ossf/scorecard/blob/main/docs/checks.md#branch-protection"}},
        {"name":"CI-Tests","score":10,"reason":"all 30 merged PRs checked passed CI","documentation":{"short":"Determines if the project runs tests before pull requests are merged.","url":"https://github.com/ossf/scorecard/blob/main/docs/checks.md#ci-tests"}},
        {"name":"CII-Best-Practices","score":0,"reason":"no badge found","documentation":{"short":"Determines if the project has a CII Best Practices Badge.","url":"https://github.com/ossf/scorecard/blob/main/docs/checks.md#cii-best-practices"}},
        {"name":"Code-Review","score":9,"reason":"found 28/30 approved changesets -- score normalized to 9","documentation":{"short":"Determines if the project requires human code review before pull requests are merged.","url":"https://github.com/ossf/scorecard/blob/main/docs/checks.md#code-review"}},
        {"name":"Contributors","score":10,"reason":"42 different organizations found","documentation":{"short":"Determines if the project has a set of contributors from multiple organizations.","url":"https://github.com/ossf/scorecard/blob/main/docs/checks.md#contributors"}},
        {"name":"Dangerous-Workflow","score":10,"reason":"no dangerous workflow patterns detected","documentation":{"short":"Determines if a project's GitHub Action workflows avoid dangerous patterns.","url":"https://github.com/ossf/scorecard/blob/main/docs/checks.md#dangerous-workflow"}},
        {"name":"Dependency-Update-Tool","score":10,"reason":"update tool detected","documentation":{"short":"Determines if the project uses a dependency update tool.","url":"https://github.com/ossf/scorecard/blob/main/docs/checks.md#dependency-update-tool"}},
        {"name":"Fuzzing","score":10,"reason":"project is fuzzed","documentation":{"short":"Determines if the project uses fuzzing.","url":"https://github.com/ossf/scorecard/blob/main/docs/checks.md#fuzzing"}},
        {"name":"License","score":10,"reason":"license file detected","documentation":{"short":"Determines if the project has defined a license.","url":"https://github.com/ossf/scorecard/blob/main/docs/checks.md#license"}},
        {"name":"Maintained","score":10,"reason":"30 commit(s) and 25 issue activity found in the last 90 days","documentation":{"short":"Determines if the project is 'actively maintained'.","url":"https://github.com/ossf/scorecard/blob/main/docs/checks.md#maintained"}},
        {"name":"Pinned-Dependencies","score":7,"reason":"dependency not pinned by hash detected","documentation":{"short":"Determines if the project has declared and pinned the dependencies of its build process.","url":"https://github.com/ossf/scorecard/blob/main/docs/checks.md#pinned-dependencies"}}
    ]
}"#;

async fn fresh_client(server: &MockServer) -> (Client, TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let cache = Cache::open(dir.path().join("cache.db")).unwrap();
    let http = reqwest::Client::builder().build().unwrap();
    let client = Client::new(http, cache).with_base_url(server.uri());
    (client, dir)
}

// ─── S-001 ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn s001_200_returns_parsed_report() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/projects/github.com/prometheus/prometheus"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("ETag", "\"x\"")
                .insert_header("Content-Type", "application/json")
                .set_body_string(PROMETHEUS_BODY),
        )
        .expect(1)
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let cache = Cache::open(dir.path().join("cache.db")).unwrap();
    let http = reqwest::Client::builder().build().unwrap();
    let client = Client::new(http, cache.clone()).with_base_url(server.uri());

    let report = client
        .get("prometheus", "prometheus")
        .await
        .expect("call succeeds")
        .expect("Some(report)");

    assert!(
        (report.score - 8.7).abs() < f64::EPSILON,
        "score should round-trip exactly, got {}",
        report.score
    );
    assert!(
        report.checks.len() >= 10,
        "expected at least 10 check results, got {}",
        report.checks.len()
    );
    assert_eq!(report.repo.name, "github.com/prometheus/prometheus");

    // Cache must contain an entry under the spec'd key.
    let entry = cache
        .get("scorecard:projects/github.com/prometheus/prometheus")
        .expect("cache get OK")
        .expect("cache entry present");
    assert_eq!(entry.etag.as_deref(), Some("\"x\""));
    assert!(!entry.body.is_empty(), "cached body should be the raw JSON");
}

// ─── S-101 ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn s101_cache_hit_serves_without_network() {
    let server = MockServer::start().await;
    // .expect(1) → fails on drop if the mock is hit a second time.
    Mock::given(method("GET"))
        .and(path("/projects/github.com/prometheus/prometheus"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("ETag", "\"x\"")
                .set_body_string(PROMETHEUS_BODY),
        )
        .expect(1)
        .mount(&server)
        .await;

    let (client, _dir) = fresh_client(&server).await;

    // First call: populates cache (1 request to mock).
    let first = client
        .get("prometheus", "prometheus")
        .await
        .unwrap()
        .expect("Some");
    // Second call: must NOT touch the network — entry is fresh under 7-day TTL.
    let second = client
        .get("prometheus", "prometheus")
        .await
        .unwrap()
        .expect("Some");

    assert!((first.score - second.score).abs() < f64::EPSILON);
    assert_eq!(first.checks.len(), second.checks.len());
}

// ─── S-102 ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn s102_404_returns_ok_none() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/projects/github.com/octocat/Hello-World"))
        .respond_with(ResponseTemplate::new(404).set_body_string("not found"))
        .mount(&server)
        .await;

    let (client, _dir) = fresh_client(&server).await;
    let result = client
        .get("octocat", "Hello-World")
        .await
        .expect("404 is not an error");
    assert!(
        result.is_none(),
        "404 must surface as Ok(None), got Some({result:?})"
    );
}

// ─── S-201 ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn s201_5xx_returns_err() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/projects/github.com/prometheus/prometheus"))
        .respond_with(ResponseTemplate::new(500).set_body_string("upstream broke"))
        .mount(&server)
        .await;

    let (client, _dir) = fresh_client(&server).await;
    let err = client
        .get("prometheus", "prometheus")
        .await
        .expect_err("5xx must be an error");
    let typed = err
        .downcast_ref::<ScorecardError>()
        .expect("typed ScorecardError");
    match typed {
        ScorecardError::Other { status, .. } => assert_eq!(*status, 500),
    }
}

// ─── S-202 ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn s202_malformed_json_returns_parse_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/projects/github.com/prometheus/prometheus"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("ETag", "\"x\"")
                .set_body_string("not valid json {"),
        )
        .mount(&server)
        .await;

    let (client, _dir) = fresh_client(&server).await;
    let err = client
        .get("prometheus", "prometheus")
        .await
        .expect_err("malformed JSON must be an error");

    // The error chain must mention the parse step so callers can disambiguate
    // transport vs. parse failures.
    let chain = format!("{err:#}");
    assert!(
        chain.contains("parse ScorecardReport"),
        "error chain should mention 'parse ScorecardReport', got: {chain}"
    );
}
