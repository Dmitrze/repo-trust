//! Integration test for the localhost web viewer.
//!
//! Drives the axum router via `oneshot` against a populated cache fixture
//! — no real port binding, no live HTTP. Covers the happy path defined in
//! `tests/scenarios/web-viewer.md` (S-001, S-002, S-103).

#![cfg(feature = "web")]

use std::collections::BTreeMap;

use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use repo_trust::models::{
    Category, Confidence, EvidenceItem, Mode, ModuleResult, ModuleWeights, RepositorySummary,
    TrustReport, Verdict,
};
use repo_trust::storage::Cache;
use repo_trust::web::router;
use tempfile::TempDir;
use time::OffsetDateTime;
use tower::ServiceExt;

fn fixture_report() -> TrustReport {
    let modules = ["activity", "maintainers", "security", "stars", "adoption"]
        .iter()
        .map(|m| ModuleResult {
            module: (*m).to_string(),
            score: 70,
            confidence: Confidence::High,
            sub_scores: BTreeMap::new(),
            sample_size: None,
            missing_data: Vec::new(),
        })
        .collect();
    let evidence = (0..5)
        .map(|i| EvidenceItem {
            module: "activity".to_string(),
            code: format!("evidence_{i}"),
            label: format!("Evidence #{i}"),
            value: serde_json::json!(i),
            threshold: None,
            verdict: if i < 2 {
                Verdict::Positive
            } else {
                Verdict::Neutral
            },
            rationale: format!("rationale {i}"),
        })
        .collect();

    TrustReport {
        schema_version: repo_trust::REPORT_SCHEMA_VERSION.to_string(),
        repository: RepositorySummary {
            full_name: "acme/widget".to_string(),
            url: "https://github.com/acme/widget".to_string(),
            default_branch: "main".to_string(),
            primary_language: Some("Rust".to_string()),
            stars: 1234,
            snapshot_at: OffsetDateTime::UNIX_EPOCH,
        },
        overall_score: 73,
        overall_confidence: Confidence::High,
        category: Category::Good,
        mode: Mode::Standard,
        modules,
        evidence,
        top_strengths: Vec::new(),
        top_concerns: Vec::new(),
        caveats: vec!["sample caveat for the integration test".to_string()],
        scoring_version: repo_trust::SCORING_VERSION.to_string(),
        weights_used: ModuleWeights::default(),
        snapshot_at: OffsetDateTime::UNIX_EPOCH,
        runtime_seconds: 1.234_567,
    }
}

fn fresh_cache() -> (Cache, TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let cache = Cache::open(dir.path().join("cache.db")).unwrap();
    (cache, dir)
}

#[tokio::test]
async fn happy_path_index_then_report_then_api() {
    let (cache, _tmp) = fresh_cache();
    let report = fixture_report();
    let bytes = serde_json::to_vec(&report).unwrap();
    cache
        .put_report(
            &report.repository.full_name,
            "standard",
            repo_trust::SCORING_VERSION,
            &bytes,
        )
        .unwrap();

    let app = router(cache, false);

    // ─── S-001: GET / lists the report ────────────────────────────────
    let resp = app
        .clone()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body(), 1 << 20).await.unwrap();
    let html = String::from_utf8(body.to_vec()).unwrap();
    assert!(html.contains("acme/widget"), "index lists the cached repo");
    assert!(
        html.contains("73"),
        "index shows the overall score 73; html: {}",
        &html[..html.len().min(500)],
    );

    // ─── S-002: GET /reports/{owner}/{name} renders module cards ──────
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/reports/acme/widget")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body(), 1 << 20).await.unwrap();
    let html = String::from_utf8(body.to_vec()).unwrap();
    assert!(html.contains("acme/widget"));
    assert!(html.contains("Score 73") || html.contains("73"));
    for module in ["activity", "maintainers", "security", "stars", "adoption"] {
        assert!(html.contains(module), "module name `{module}` rendered");
    }
    // ≥3 evidence items rendered.
    let evidence_count = html.matches("evidence_").count();
    assert!(
        evidence_count >= 3,
        "expected ≥3 evidence items, found {evidence_count}",
    );

    // ─── API: GET /api/reports/{owner}/{name} returns parseable JSON ──
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/reports/acme/widget")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()),
        Some("application/json"),
    );
    let body = to_bytes(resp.into_body(), 1 << 20).await.unwrap();
    let parsed: TrustReport = serde_json::from_slice(&body).unwrap();
    assert_eq!(parsed.repository.full_name, "acme/widget");
    assert_eq!(parsed.overall_score, 73);

    // ─── S-103: GET /static/style.css served from rust-embed ──────────
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/static/style.css")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let ct = resp
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(ct.starts_with("text/css"), "wrong Content-Type: {ct}");
}
