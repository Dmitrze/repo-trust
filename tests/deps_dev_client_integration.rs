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
            "/v3/projects/github.com/prometheus/prometheus/packages",
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_string(
                    r#"{"packages":[{"system":"GO","name":"github.com/prometheus/prometheus"}]}"#,
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
        .and(path("/v3/projects/github.com/ghost/ghost/packages"))
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
    // Upstream returns packages in arbitrary order: (NPM,b), (GO,a), (NPM,a).
    // Client must sort to: (GO,a), (NPM,a), (NPM,b).
    Mock::given(method("GET"))
        .and(path("/v3/projects/github.com/acme/multi/packages"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_string(
                    r#"{"packages":[
                        {"system":"NPM","name":"b"},
                        {"system":"GO","name":"a"},
                        {"system":"NPM","name":"a"}
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
                name: "a".to_string(),
            },
            PackageRef {
                system: "NPM".to_string(),
                name: "a".to_string(),
            },
            PackageRef {
                system: "NPM".to_string(),
                name: "b".to_string(),
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
            "/v3/projects/github.com/prometheus/prometheus/packages",
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
