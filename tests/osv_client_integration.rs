#![allow(
    clippy::unused_async,
    clippy::float_cmp,
    clippy::doc_lazy_continuation,
    clippy::unreadable_literal,
    clippy::too_many_lines,
    dead_code
)]

//! Integration tests for `repo_trust::api::osv::Client`.
//!
//! Covers `tests/scenarios/osv-client.md`:
//! - S-001: empty response `{"vulns":[]}` → empty Vec
//! - S-002: populated response with one withdrawn + one not-withdrawn → Vec of length 1
//! - S-101: deterministic sort by `id`
//! - S-102: cache hit serves without network
//! - S-201: 503 returns `Err`

use repo_trust::api::osv::{Client, OsvError, PackageCoords};
use repo_trust::storage::Cache;
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const NPM: &str = "npm";

fn coords(name: &str, version: &str) -> PackageCoords {
    PackageCoords {
        name: name.to_string(),
        ecosystem: NPM.to_string(),
        version: version.to_string(),
    }
}

async fn fresh_client(server: &MockServer) -> (Client, TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let cache = Cache::open(dir.path().join("cache.db")).unwrap();
    let http = reqwest::Client::builder().build().unwrap();
    let client = Client::new(http, cache).with_base_url(server.uri());
    (client, dir)
}

#[tokio::test]
async fn s001_empty_response_returns_empty_vec() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/query"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"vulns":[]}"#))
        .expect(1)
        .mount(&server)
        .await;

    let (client, _dir) = fresh_client(&server).await;
    let advisories = client.query(&coords("safe-pkg", "1.0.0")).await.unwrap();
    assert!(
        advisories.is_empty(),
        "expected empty Vec, got {advisories:?}"
    );
}

#[tokio::test]
async fn s002_populated_response_filters_withdrawn() {
    let server = MockServer::start().await;
    let body = r#"{
        "vulns":[
            {
                "id":"GHSA-aaaa-bbbb-cccc",
                "summary":"open advisory",
                "modified":"2025-05-01T00:00:00Z"
            },
            {
                "id":"GHSA-zzzz-yyyy-xxxx",
                "summary":"withdrawn advisory",
                "modified":"2025-04-01T00:00:00Z",
                "withdrawn":"2025-06-01T00:00:00Z"
            }
        ]
    }"#;
    Mock::given(method("POST"))
        .and(path("/v1/query"))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
        .mount(&server)
        .await;

    let (client, _dir) = fresh_client(&server).await;
    let advisories = client.query(&coords("lodash", "4.17.20")).await.unwrap();
    assert_eq!(advisories.len(), 1, "withdrawn should be filtered out");
    assert_eq!(advisories[0].id, "GHSA-aaaa-bbbb-cccc");
}

#[tokio::test]
async fn s101_deterministic_sort_by_id() {
    let server = MockServer::start().await;
    // Input order: cccc, aaaa, bbbb — output must be sorted alphabetically.
    let body = r#"{
        "vulns":[
            {"id":"GHSA-cccc","summary":"c","modified":"2025-01-01T00:00:00Z"},
            {"id":"GHSA-aaaa","summary":"a","modified":"2025-01-01T00:00:00Z"},
            {"id":"GHSA-bbbb","summary":"b","modified":"2025-01-01T00:00:00Z"}
        ]
    }"#;
    Mock::given(method("POST"))
        .and(path("/v1/query"))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
        .mount(&server)
        .await;

    let (client, _dir) = fresh_client(&server).await;
    let advisories = client.query(&coords("foo", "0.1.0")).await.unwrap();
    let ids: Vec<&str> = advisories.iter().map(|a| a.id.as_str()).collect();
    assert_eq!(ids, vec!["GHSA-aaaa", "GHSA-bbbb", "GHSA-cccc"]);
}

#[tokio::test]
async fn s102_cache_hit_serves_without_network() {
    let server = MockServer::start().await;
    let body = r#"{
        "vulns":[
            {"id":"GHSA-cached","summary":"cached","modified":"2025-01-01T00:00:00Z"}
        ]
    }"#;
    // Mock should only be hit once: the second call must come from cache.
    Mock::given(method("POST"))
        .and(path("/v1/query"))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
        .expect(1)
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let cache = Cache::open(dir.path().join("cache.db")).unwrap();
    let http = reqwest::Client::builder().build().unwrap();
    let client = Client::new(http, cache).with_base_url(server.uri());

    let coords = coords("lodash", "4.17.20");
    let first = client.query(&coords).await.unwrap();
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].id, "GHSA-cached");

    // Second call within TTL — must serve from cache.
    let second = client.query(&coords).await.unwrap();
    assert_eq!(second.len(), 1);
    assert_eq!(second[0].id, "GHSA-cached");
    // `expect(1)` on the mock asserts at drop time that exactly one POST hit it.
}

#[tokio::test]
async fn s201_503_returns_err() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/query"))
        .respond_with(ResponseTemplate::new(503).set_body_string("upstream down"))
        .mount(&server)
        .await;

    let (client, _dir) = fresh_client(&server).await;
    let err = client.query(&coords("any", "0.0.0")).await.unwrap_err();
    let downcast = err.downcast_ref::<OsvError>().expect("typed OsvError");
    match downcast {
        OsvError::Other { status, body } => {
            assert_eq!(*status, 503);
            assert!(
                body.contains("upstream down"),
                "body should propagate, got: {body}"
            );
        },
    }
}
