//! HTTP handlers for the localhost web viewer.
//!
//! All handlers are read-only by default. `POST /scans` is gated behind
//! `--allow-scan` and returns `405 Method Not Allowed` otherwise to
//! mitigate browser-driven DNS-rebinding attacks against localhost
//! (`specs/web-viewer.md` §2).

use askama::Template;
use axum::extract::{Path, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Redirect, Response};
use time::format_description::well_known::Rfc3339;

use crate::models::{EvidenceItem, ModuleResult, RepositorySummary, TrustReport};

use super::routes::AppState;

/// One row in the cached-reports table.
#[derive(Debug, Clone)]
pub struct IndexRow {
    pub repo: String,
    pub mode: String,
    pub scoring_ver: String,
    pub computed_at: String,
    pub score: u8,
    pub category: &'static str,
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    rows: Vec<IndexRow>,
    allow_scan: bool,
}

#[derive(Template)]
#[template(path = "report.html")]
struct ReportTemplate {
    report: ReportView,
}

#[derive(Template)]
#[template(path = "not_found.html")]
struct NotFoundTemplate {
    message: String,
}

/// Display-friendly projection of [`TrustReport`] used by the askama template.
///
/// We translate enum variants to plain strings here so the template doesn't
/// need [`std::fmt::Display`] impls on the model types (we cannot modify
/// `src/models/`). All fields are `pub` so askama can read them.
#[derive(Debug)]
pub struct ReportView {
    pub schema_version: String,
    pub scoring_version: String,
    pub repository: RepositorySummary,
    pub overall_score: u8,
    pub overall_confidence: &'static str,
    pub category: &'static str,
    pub mode: &'static str,
    pub modules: Vec<ModuleView>,
    pub evidence: Vec<EvidenceView>,
    pub top_strengths: Vec<EvidenceView>,
    pub top_concerns: Vec<EvidenceView>,
    pub caveats: Vec<String>,
}

#[derive(Debug)]
pub struct ModuleView {
    pub module: String,
    pub score: u8,
    pub confidence: &'static str,
}

#[derive(Debug)]
pub struct EvidenceView {
    pub module: String,
    pub code: String,
    pub label: String,
    pub verdict: &'static str,
    pub rationale: String,
}

impl ReportView {
    fn from_report(r: TrustReport) -> Self {
        Self {
            schema_version: r.schema_version,
            scoring_version: r.scoring_version,
            repository: r.repository,
            overall_score: r.overall_score,
            overall_confidence: confidence_str(r.overall_confidence),
            category: category_str(r.category),
            mode: mode_str(r.mode),
            modules: r.modules.into_iter().map(ModuleView::from_module).collect(),
            evidence: r
                .evidence
                .into_iter()
                .map(EvidenceView::from_evidence)
                .collect(),
            top_strengths: r
                .top_strengths
                .into_iter()
                .map(EvidenceView::from_evidence)
                .collect(),
            top_concerns: r
                .top_concerns
                .into_iter()
                .map(EvidenceView::from_evidence)
                .collect(),
            caveats: r.caveats,
        }
    }
}

impl ModuleView {
    fn from_module(m: ModuleResult) -> Self {
        Self {
            module: m.module,
            score: m.score,
            confidence: confidence_str(m.confidence),
        }
    }
}

impl EvidenceView {
    fn from_evidence(e: EvidenceItem) -> Self {
        Self {
            module: e.module,
            code: e.code,
            label: e.label,
            verdict: verdict_str(e.verdict),
            rationale: e.rationale,
        }
    }
}

const fn confidence_str(c: crate::models::Confidence) -> &'static str {
    match c {
        crate::models::Confidence::Low => "Low",
        crate::models::Confidence::Medium => "Medium",
        crate::models::Confidence::High => "High",
    }
}

const fn category_str(c: crate::models::Category) -> &'static str {
    match c {
        crate::models::Category::Strong => "Strong",
        crate::models::Category::Good => "Good",
        crate::models::Category::Mixed => "Mixed",
        crate::models::Category::Weak => "Weak",
        crate::models::Category::HighRisk => "HighRisk",
    }
}

const fn mode_str(m: crate::models::Mode) -> &'static str {
    match m {
        crate::models::Mode::Quick => "quick",
        crate::models::Mode::Standard => "standard",
        crate::models::Mode::Deep => "deep",
    }
}

const fn verdict_str(v: crate::models::Verdict) -> &'static str {
    match v {
        crate::models::Verdict::Positive => "Positive",
        crate::models::Verdict::Neutral => "Neutral",
        crate::models::Verdict::Concerning => "Concerning",
        crate::models::Verdict::HighRisk => "HighRisk",
    }
}

// ─── Handlers ────────────────────────────────────────────────────────────

/// `GET /` — list of cached reports newest-first.
pub async fn index(State(state): State<AppState>) -> Response {
    let summaries = match state.cache.list_all_reports() {
        Ok(rs) => rs,
        Err(err) => return server_error(&err),
    };

    let mut rows: Vec<IndexRow> = Vec::with_capacity(summaries.len());
    for s in summaries {
        // Look up the latest body to extract score + category. We tolerate
        // a missing body (race with cache eviction) by skipping the row.
        let Some(body) = state
            .cache
            .latest_report(&s.repo, &s.mode, &s.scoring_ver)
            .ok()
            .flatten()
        else {
            continue;
        };
        let Ok(report) = serde_json::from_slice::<TrustReport>(&body) else {
            continue;
        };
        let computed_at = s.computed_at.format(&Rfc3339).unwrap_or_default();
        rows.push(IndexRow {
            repo: s.repo,
            mode: s.mode,
            scoring_ver: s.scoring_ver,
            computed_at,
            score: report.overall_score,
            category: category_str(report.category),
        });
    }

    let tmpl = IndexTemplate {
        rows,
        allow_scan: state.allow_scan,
    };
    render_html(&tmpl)
}

/// `GET /reports/{owner}/{name}` — single-report view.
pub async fn report(
    State(state): State<AppState>,
    Path((owner, name)): Path<(String, String)>,
) -> Response {
    let full_name = format!("{owner}/{name}");
    let Some((report, _)) = lookup_latest(&state, &full_name) else {
        return not_found_response(&format!("No cached report for `{full_name}`."));
    };
    let view = ReportView::from_report(report);
    let tmpl = ReportTemplate { report: view };
    render_html(&tmpl)
}

/// `GET /api/reports/{owner}/{name}` — raw cached JSON byte-for-byte.
pub async fn report_json(
    State(state): State<AppState>,
    Path((owner, name)): Path<(String, String)>,
) -> Response {
    let full_name = format!("{owner}/{name}");
    let Some((_, bytes)) = lookup_latest(&state, &full_name) else {
        return (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "application/json")],
            format!(r#"{{"error":"no cached report for {full_name}"}}"#),
        )
            .into_response();
    };
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        bytes,
    )
        .into_response()
}

/// `POST /scans` — trigger a re-scan when `--allow-scan` is set.
///
/// Body is `application/x-www-form-urlencoded` with `repo` + `mode`. On
/// success we 303-redirect to the rendered report. When `--allow-scan` is
/// off, this returns `405 Method Not Allowed` immediately and never reads
/// the body.
pub async fn scan(State(state): State<AppState>, headers: HeaderMap, body: String) -> Response {
    if !state.allow_scan {
        return method_not_allowed();
    }

    // Parse `repo=...&mode=...` from the form body. We do this manually to
    // avoid pulling axum::extract::Form (which would require us to enable
    // the `form` feature; it is enabled by default but staying explicit
    // keeps the surface tiny and easier to test).
    let _ct = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let mut repo = String::new();
    let mut mode = String::from("standard");
    for pair in body.split('&') {
        let Some((k, v)) = pair.split_once('=') else {
            continue;
        };
        let v = url_decode(v);
        match k {
            "repo" => repo = v,
            "mode" => mode = v,
            _ => {},
        }
    }

    if repo.is_empty() {
        return (StatusCode::BAD_REQUEST, "missing `repo` field").into_response();
    }

    let mode_enum = match mode.as_str() {
        "quick" => crate::cli::scan::Mode::Quick,
        "deep" => crate::cli::scan::Mode::Deep,
        _ => crate::cli::scan::Mode::Standard,
    };

    // Build the same arg shape the CLI uses. We delegate to
    // `cli::scan::execute` — the single source of truth for the scan
    // pipeline — rather than duplicating it here.
    let args = crate::cli::scan::ScanArgs {
        repo: repo.clone(),
        mode: mode_enum,
        modules: None,
        skip_modules: None,
        output: std::env::temp_dir().join("repo-trust-web-scans"),
        format: Vec::new(),
        weights: None,
        scoring_version: None,
        token: None,
        seed: None,
        refresh: false,
        refresh_module: None,
        debug: false,
        quiet: true,
        no_color: true,
        json: false,
        api_base_url: None,
        snapshot_at: None,
    };

    match crate::cli::scan::execute(args).await {
        Ok(_) => {
            // Normalize redirect target — the cache key uses the canonical
            // `owner/repo` form, which `repo` already is for the simple
            // case. If a URL was passed we'd need to canonicalize first,
            // but the form UI sends bare `owner/repo`.
            let target = format!("/reports/{repo}");
            Redirect::to(&target).into_response()
        },
        Err(e) => {
            tracing::error!(error = ?e, "scan failed via web form");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("scan failed: {e:#}"),
            )
                .into_response()
        },
    }
}

/// `GET /static/*path` — serve embedded asset by name.
pub async fn static_asset(Path(path): Path<String>) -> Response {
    let Some(file) = super::Assets::get(&path) else {
        return (StatusCode::NOT_FOUND, "asset not found").into_response();
    };
    let mime = mime_for(&path);
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, mime)],
        file.data.into_owned(),
    )
        .into_response()
}

/// Fallback handler for unknown routes.
pub async fn not_found() -> Response {
    not_found_response("That page does not exist.")
}

// ─── Helpers ─────────────────────────────────────────────────────────────

fn lookup_latest(state: &AppState, full_name: &str) -> Option<(TrustReport, Vec<u8>)> {
    let summaries = state.cache.list_all_reports().ok()?;
    let row = summaries.into_iter().find(|s| s.repo == full_name)?;
    let body = state
        .cache
        .latest_report(&row.repo, &row.mode, &row.scoring_ver)
        .ok()
        .flatten()?;
    let report = serde_json::from_slice::<TrustReport>(&body).ok()?;
    Some((report, body))
}

fn render_html<T: Template>(tmpl: &T) -> Response {
    match tmpl.render() {
        Ok(html) => Html(html).into_response(),
        Err(err) => {
            tracing::error!(error = ?err, "template render failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("template error: {err}"),
            )
                .into_response()
        },
    }
}

fn server_error(err: &anyhow::Error) -> Response {
    tracing::error!(error = ?err, "internal server error");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        format!("internal error: {err:#}"),
    )
        .into_response()
}

fn not_found_response(message: &str) -> Response {
    let tmpl = NotFoundTemplate {
        message: message.to_string(),
    };
    let body = tmpl
        .render()
        .unwrap_or_else(|_| format!("<h1>Not found</h1><p>{message}</p>"));
    (StatusCode::NOT_FOUND, Html(body)).into_response()
}

fn method_not_allowed() -> Response {
    (
        StatusCode::METHOD_NOT_ALLOWED,
        [(header::ALLOW, "GET")],
        "POST /scans disabled. Restart `repo-trust serve --allow-scan` to enable.",
    )
        .into_response()
}

fn mime_for(path: &str) -> &'static str {
    match path.rsplit_once('.').map(|(_, ext)| ext) {
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "application/javascript; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("ico") => "image/x-icon",
        Some("html") => "text/html; charset=utf-8",
        _ => "application/octet-stream",
    }
}

/// Minimal `application/x-www-form-urlencoded` value decoder. Handles `+`
/// → space and `%xx` percent-escapes; invalid escapes are passed through
/// verbatim.
fn url_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(' ');
                i += 1;
            },
            b'%' if i + 2 < bytes.len() => {
                let hex = &s[i + 1..i + 3];
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    out.push(char::from(byte));
                    i += 3;
                } else {
                    out.push('%');
                    i += 1;
                }
            },
            b => {
                out.push(char::from(b));
                i += 1;
            },
        }
    }
    out
}

// ─── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{to_bytes, Body};
    use axum::http::Request;
    use tempfile::TempDir;
    use tower::ServiceExt;

    use crate::storage::Cache;

    /// Build a Cache + populated TempDir. Two reports for `acme/widget`
    /// (latest wins) and one for `octocat/Hello-World` (newest overall).
    fn populated_cache() -> (Cache, TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cache.db");
        let cache = Cache::open(&path).unwrap();

        let widget = sample_report("acme/widget", 73, crate::models::Category::Good);
        let widget_bytes = serde_json::to_vec(&widget).unwrap();
        cache
            .put_report("acme/widget", "standard", "1.0.0", &widget_bytes)
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let widget2 = sample_report("acme/widget", 75, crate::models::Category::Good);
        let widget2_bytes = serde_json::to_vec(&widget2).unwrap();
        cache
            .put_report("acme/widget", "standard", "1.0.0", &widget2_bytes)
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let hello = sample_report("octocat/Hello-World", 42, crate::models::Category::Weak);
        let hello_bytes = serde_json::to_vec(&hello).unwrap();
        cache
            .put_report("octocat/Hello-World", "standard", "1.0.0", &hello_bytes)
            .unwrap();

        (cache, dir)
    }

    fn sample_report(full_name: &str, score: u8, cat: crate::models::Category) -> TrustReport {
        use crate::models::{Confidence, ModuleResult, ModuleWeights, RepositorySummary, Verdict};
        use std::collections::BTreeMap;
        use time::OffsetDateTime;

        let modules = ["activity", "maintainers", "security", "stars", "adoption"]
            .iter()
            .map(|m| ModuleResult {
                module: (*m).to_string(),
                score: 50,
                confidence: Confidence::Medium,
                sub_scores: BTreeMap::new(),
                sample_size: None,
                missing_data: Vec::new(),
            })
            .collect();
        let evidence = (0..4)
            .map(|i| EvidenceItem {
                module: "activity".to_string(),
                code: format!("evidence_{i}"),
                label: format!("Evidence #{i}"),
                value: serde_json::json!(i),
                threshold: None,
                verdict: Verdict::Neutral,
                rationale: format!("This is evidence number {i}."),
            })
            .collect();

        TrustReport {
            schema_version: crate::REPORT_SCHEMA_VERSION.to_string(),
            repository: RepositorySummary {
                full_name: full_name.to_string(),
                url: format!("https://github.com/{full_name}"),
                default_branch: "main".to_string(),
                primary_language: Some("Rust".to_string()),
                stars: 100,
                snapshot_at: OffsetDateTime::UNIX_EPOCH,
            },
            overall_score: score,
            overall_confidence: Confidence::Medium,
            category: cat,
            mode: crate::models::Mode::Standard,
            modules,
            evidence,
            top_strengths: Vec::new(),
            top_concerns: Vec::new(),
            caveats: Vec::new(),
            scoring_version: crate::SCORING_VERSION.to_string(),
            weights_used: ModuleWeights::default(),
            snapshot_at: OffsetDateTime::UNIX_EPOCH,
            runtime_seconds: 1.234567,
        }
    }

    async fn body_string(resp: Response) -> String {
        let bytes = to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    #[tokio::test]
    async fn get_index_lists_all_cached_repos() {
        let (cache, _tmp) = populated_cache();
        let app = super::super::router(cache, false);
        let resp = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_string(resp).await;
        assert!(body.contains("acme/widget"));
        assert!(body.contains("octocat/Hello-World"));
        // Newest first: octocat written last, so should appear before acme.
        let pos_octocat = body.find("octocat/Hello-World").unwrap();
        let pos_acme = body.find("acme/widget").unwrap();
        assert!(
            pos_octocat < pos_acme,
            "octocat/Hello-World was written most recently and should sort first"
        );
    }

    #[tokio::test]
    async fn get_report_renders_score_and_module_names() {
        let (cache, _tmp) = populated_cache();
        let app = super::super::router(cache, false);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/reports/acme/widget")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_string(resp).await;
        assert!(body.contains("acme/widget"), "repo name in body");
        assert!(
            body.contains("Score 75") || body.contains(">75<"),
            "latest score (75) rendered; body snippet: {}",
            &body[..body.len().min(2000)],
        );
        for module in ["activity", "maintainers", "security", "stars", "adoption"] {
            assert!(body.contains(module), "module name `{module}` rendered");
        }
    }

    #[tokio::test]
    async fn get_api_report_returns_json_with_correct_content_type() {
        let (cache, _tmp) = populated_cache();
        let app = super::super::router(cache, false);
        let resp = app
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
        let body = body_string(resp).await;
        let parsed: TrustReport =
            serde_json::from_str(&body).expect("body is a parseable TrustReport");
        assert_eq!(parsed.repository.full_name, "acme/widget");
        // The JSON returned is the latest cached body — score 75, not 73.
        assert_eq!(parsed.overall_score, 75);
    }

    #[tokio::test]
    async fn unknown_report_returns_404() {
        let (cache, _tmp) = populated_cache();
        let app = super::super::router(cache, false);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/reports/ghost/ghost")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        let body = body_string(resp).await;
        assert!(
            body.contains("ghost/ghost"),
            "404 page mentions the missing repo for clarity"
        );
    }

    #[tokio::test]
    async fn post_scans_without_allow_scan_returns_405() {
        let (cache, _tmp) = populated_cache();
        let app = super::super::router(cache, false);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/scans")
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(Body::from("repo=acme/widget&mode=standard"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    #[tokio::test]
    async fn get_static_style_css_returns_200_text_css() {
        let (cache, _tmp) = populated_cache();
        let app = super::super::router(cache, false);
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
        assert!(ct.starts_with("text/css"), "got Content-Type {ct}");
        let body = body_string(resp).await;
        assert!(body.contains("--bg"), "CSS contains a custom property");
    }

    #[tokio::test]
    async fn empty_cache_renders_friendly_empty_state() {
        let dir = tempfile::tempdir().unwrap();
        let cache = Cache::open(dir.path().join("cache.db")).unwrap();
        let app = super::super::router(cache, false);
        let resp = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_string(resp).await;
        assert!(
            body.contains("No reports yet"),
            "empty-state message present per scenario S-101"
        );
    }

    #[test]
    fn url_decode_handles_percent_and_plus() {
        assert_eq!(url_decode("acme%2Fwidget"), "acme/widget");
        assert_eq!(url_decode("a+b"), "a b");
        assert_eq!(url_decode("plain"), "plain");
        // Invalid escape passed through verbatim.
        assert_eq!(url_decode("%ZZ"), "%ZZ");
    }
}
