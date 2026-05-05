#![allow(
    clippy::unused_async,
    clippy::float_cmp,
    clippy::doc_lazy_continuation,
    clippy::unreadable_literal,
    clippy::too_many_lines
)]

//! Integration tests for `repo_trust::api::deps_dev::Client`.
//!
//! Covers `tests/scenarios/deps-dev-client.md`:
//! - S-001: 200 OK on `project_packages` returns sorted Vec.
//! - S-002: 200 OK on `package` returns weekly downloads parsed from string.
//! - S-101: 404 on `project_packages` returns `Ok(Vec::new())`.
//! - S-102: deterministic sort by `(system, name)`.
//! - S-201: 5xx returns `Err`.

use repo_trust::api::deps_dev::{Client, DepsDevError, PackageRef};
use repo_trust::storage::Cache;
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn fresh_client(server: &MockServer) -> (Client, TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let cache = Cache::open(dir.path().join("cache.db")).unwrap();
    let http = reqwest::Client::builder().build().unwrap();
    let client = Client::new(http, cache).with_base_url(server.uri());
    (client, dir)
}

// ─── S-001 ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn s001_project_packages_returns_sorted_vec() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(
            "/v3alpha/projects/github.com%2Fprometheus%2Fprometheus:packageversions",
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                // Two versions with verified GO_ORIGIN provenance — clears
                // the first-party filter (≥2 versions per (system, name)
                // AND verified provenance OR name-match; this entry has
                // both).
                .set_body_string(
                    r#"{"versions":[
                        {"versionKey":{"system":"GO","name":"github.com/prometheus/prometheus","version":"v2.45.0"},"relationProvenance":"GO_ORIGIN"},
                        {"versionKey":{"system":"GO","name":"github.com/prometheus/prometheus","version":"v2.46.0"},"relationProvenance":"GO_ORIGIN"}
                    ]}"#,
                ),
        )
        .expect(1)
        .mount(&server)
        .await;

    let (client, _dir) = fresh_client(&server).await;
    let packages = client
        .project_packages("prometheus", "prometheus")
        .await
        .expect("call succeeds");

    assert_eq!(packages.len(), 1, "expected one package, got {packages:?}");
    assert_eq!(packages[0].system, "GO");
    assert_eq!(packages[0].name, "github.com/prometheus/prometheus");
}

#[tokio::test]
async fn s001b_project_packages_dedupes_versionkey_entries() {
    // Real-world :packageversions returns one entry per (system, name,
    // version) tuple. For a project with 50 releases of one package,
    // there are 50 entries with the same (system, name). The client
    // must dedupe to the unique (system, name) set, AND apply the
    // first-party filter — `tokio-util` has only 1 version here so
    // it falls below the version-count threshold and gets dropped.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(
            "/v3alpha/projects/github.com%2Ftokio-rs%2Ftokio:packageversions",
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_string(
                    r#"{"versions":[
                        {"versionKey":{"system":"CARGO","name":"tokio","version":"1.0.0"},"relationProvenance":"UNVERIFIED_METADATA"},
                        {"versionKey":{"system":"CARGO","name":"tokio","version":"1.1.0"},"relationProvenance":"UNVERIFIED_METADATA"},
                        {"versionKey":{"system":"CARGO","name":"tokio","version":"1.2.0"},"relationProvenance":"UNVERIFIED_METADATA"},
                        {"versionKey":{"system":"CARGO","name":"tokio-util","version":"0.7.0"},"relationProvenance":"UNVERIFIED_METADATA"}
                    ]}"#,
                ),
        )
        .expect(1)
        .mount(&server)
        .await;

    let (client, _dir) = fresh_client(&server).await;
    let packages = client.project_packages("tokio-rs", "tokio").await.unwrap();
    assert_eq!(
        packages,
        vec![PackageRef {
            system: "CARGO".into(),
            name: "tokio".into()
        }],
        "tokio (3 versions, name-match) survives the filter; tokio-util \
         (1 version) falls below MIN_FIRST_PARTY_VERSIONS",
    );
}

// ─── S-002 ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn s002_package_returns_weekly_downloads() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v3/systems/NPM/packages/lodash"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_string(
                    r#"{"system":"NPM","name":"lodash","weeklyDownloads":"50000000","latestVersion":"4.17.21"}"#,
                ),
        )
        .expect(1)
        .mount(&server)
        .await;

    let (client, _dir) = fresh_client(&server).await;
    let info = client
        .package("NPM", "lodash")
        .await
        .expect("call succeeds");

    assert_eq!(info.system, "NPM");
    assert_eq!(info.name, "lodash");
    assert_eq!(
        info.weekly_downloads,
        Some(50_000_000),
        "weeklyDownloads must parse from JSON string '50000000' to u64",
    );
    assert_eq!(info.latest_version.as_deref(), Some("4.17.21"));
}

// ─── S-101 ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn s101_project_endpoint_404_returns_empty_vec() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(
            "/v3alpha/projects/github.com%2Fghost%2Fghost:packageversions",
        ))
        .respond_with(ResponseTemplate::new(404).set_body_string("not found"))
        .mount(&server)
        .await;

    let (client, _dir) = fresh_client(&server).await;
    let result = client
        .project_packages("ghost", "ghost")
        .await
        .expect("404 on project endpoint must surface as Ok(Vec::new())");

    assert!(
        result.is_empty(),
        "404 on project endpoint must yield an empty Vec, got {result:?}",
    );
}

// ─── S-102 ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn s102_packages_sorted_by_system_then_name() {
    let server = MockServer::start().await;
    // Upstream returns packages in arbitrary order across (system, name)
    // tuples; we mock 2 versions per package so each clears
    // MIN_FIRST_PARTY_VERSIONS and the filter accepts them all (the
    // package names match the repo name "multi" via the "a" / "b" /
    // "multi" sub-cases below). Provenance is GO_ORIGIN for the GO
    // entry (verified) and UNVERIFIED_METADATA + name-match for the
    // NPM entries.
    //
    // Owner-aware name-match means we use the repo identifier "multi"
    // for the lookup and a synthetic "a" / "b" name won't match.
    // To keep the original sort intent, switch the test names to the
    // repo identifier "multi" plus "multi-extra" and exercise the
    // sort across (GO, multi), (NPM, multi), (NPM, multi-extra).
    Mock::given(method("GET"))
        .and(path(
            "/v3alpha/projects/github.com%2Facme%2Fmulti:packageversions",
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_string(
                    r#"{"versions":[
                        {"versionKey":{"system":"NPM","name":"multi","version":"1.0.0"},"relationProvenance":"UNVERIFIED_METADATA"},
                        {"versionKey":{"system":"NPM","name":"multi","version":"1.1.0"},"relationProvenance":"UNVERIFIED_METADATA"},
                        {"versionKey":{"system":"GO","name":"github.com/acme/multi","version":"v1.0.0"},"relationProvenance":"GO_ORIGIN"},
                        {"versionKey":{"system":"GO","name":"github.com/acme/multi","version":"v1.1.0"},"relationProvenance":"GO_ORIGIN"},
                        {"versionKey":{"system":"NPM","name":"@acme/multi","version":"1.0.0"},"relationProvenance":"UNVERIFIED_METADATA"},
                        {"versionKey":{"system":"NPM","name":"@acme/multi","version":"1.1.0"},"relationProvenance":"UNVERIFIED_METADATA"}
                    ]}"#,
                ),
        )
        .expect(1)
        .mount(&server)
        .await;

    let (client, _dir) = fresh_client(&server).await;
    let packages = client
        .project_packages("acme", "multi")
        .await
        .expect("call succeeds");

    assert_eq!(
        packages,
        vec![
            PackageRef {
                system: "GO".to_string(),
                name: "github.com/acme/multi".to_string(),
            },
            PackageRef {
                system: "NPM".to_string(),
                name: "@acme/multi".to_string(),
            },
            PackageRef {
                system: "NPM".to_string(),
                name: "multi".to_string(),
            },
        ],
        "packages must be sorted by (system, name) for deterministic output",
    );
}

// ─── S-201 ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn s201_5xx_returns_err() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(
            "/v3alpha/projects/github.com%2Fprometheus%2Fprometheus:packageversions",
        ))
        .respond_with(ResponseTemplate::new(503).set_body_string("upstream broke"))
        .mount(&server)
        .await;

    let (client, _dir) = fresh_client(&server).await;
    let err = client
        .project_packages("prometheus", "prometheus")
        .await
        .expect_err("5xx must be an error");

    let typed = err
        .downcast_ref::<DepsDevError>()
        .expect("typed DepsDevError");
    match typed {
        DepsDevError::Other { status, .. } => assert_eq!(*status, 503),
        DepsDevError::NotFound => {
            panic!("5xx must surface as DepsDevError::Other, got NotFound")
        },
    }
}
