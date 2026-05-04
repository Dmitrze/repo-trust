#![allow(
    clippy::unused_async,
    clippy::float_cmp,
    clippy::doc_lazy_continuation,
    clippy::unreadable_literal,
    clippy::too_many_lines
)]

//! End-to-end integration tests for `AdoptionModule::run` against a
//! wiremock-backed federation of GitHub + deps.dev.
//!
//! Covers `tests/scenarios/adoption-signals-module.md`:
//! - S-001 (popular package + complete docs scores high)
//! - S-101 (no published package falls back to Medium + caveat)
//!
//! NOTE: this test covers the Adoption module end-to-end against the deps.dev
//! stub (`src/api/deps_dev.rs` STUB) — the stub returns `Ok(Vec::new())`
//! from `project_packages` regardless of input, so the happy-path test
//! invokes the scoring layer with no packages and asserts the no-packages
//! branch of the scorer. The same test fixture stays valid once the real
//! deps.dev client lands on `feat/api-deps-dev`; only the package-mock
//! arms need to be added on the rebase.

use repo_trust::api::deps_dev::Client as DepsDevClient;
use repo_trust::api::github::Client as GhClient;
use repo_trust::api::osv::Client as OsvClient;
use repo_trust::api::scorecard::Client as ScorecardClient;
use repo_trust::cli::scan::Mode;
use repo_trust::models::{Confidence, ModuleWeights, RepositoryContext, Verdict};
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

/// Shared GitHub mocks: the repo metadata, README endpoint, and the
/// `docs/` + `examples/` directory probes.
async fn shared_github_mocks(
    server: &MockServer,
    readme_present: bool,
    docs_present: bool,
    examples_present: bool,
) {
    Mock::given(method("GET"))
        .and(path("/repos/acme/widget"))
        .respond_with(ok(REPO_BODY))
        .mount(server)
        .await;

    if readme_present {
        // Base64 of "Widget — a sample repository for the Adoption module
        // integration test. " * ~30 → ~600 words once decoded.
        let plain = "Widget is a sample repository used by the repo-trust Adoption module integration test. ".repeat(30);
        let b64 = base64_encode(plain.as_bytes());
        let body = format!(r#"{{"name":"README.md","content":"{b64}","encoding":"base64"}}"#);
        Mock::given(method("GET"))
            .and(path("/repos/acme/widget/readme"))
            .respond_with(ok(&body))
            .mount(server)
            .await;
    } else {
        Mock::given(method("GET"))
            .and(path("/repos/acme/widget/readme"))
            .respond_with(nf())
            .mount(server)
            .await;
    }

    let docs_resp = if docs_present {
        ok(r#"[{"name":"index.md","type":"file"}]"#)
    } else {
        nf()
    };
    Mock::given(method("GET"))
        .and(path("/repos/acme/widget/contents/docs"))
        .respond_with(docs_resp)
        .mount(server)
        .await;

    let examples_resp = if examples_present {
        ok(r#"[{"name":"hello.rs","type":"file"}]"#)
    } else {
        nf()
    };
    Mock::given(method("GET"))
        .and(path("/repos/acme/widget/contents/examples"))
        .respond_with(examples_resp)
        .mount(server)
        .await;
}

async fn build_ctx(server_uri: String) -> (RepositoryContext, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let cache = Cache::open(dir.path().join("cache.db")).unwrap();
    let http = reqwest::Client::builder().build().unwrap();
    let github = GhClient::new(http.clone(), cache.clone(), RateLimiter::new(10), None)
        .with_base_url(&server_uri);
    let scorecard = ScorecardClient::new(http.clone(), cache.clone()).with_base_url(&server_uri);
    let osv = OsvClient::new(http.clone(), cache.clone()).with_base_url(&server_uri);
    let deps_dev = DepsDevClient::new(http, cache.clone()).with_base_url(&server_uri);
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
        deps_dev,
    };
    (ctx, dir)
}

/// Scenario S-101 + happy-path partial (README + docs/ + examples/ present;
/// deps.dev stub returns empty Vec — verifies the no-packages fallback path
/// end-to-end).
#[tokio::test]
async fn s001_well_documented_repo_runs_end_to_end() {
    let server = MockServer::start().await;
    shared_github_mocks(&server, true, true, true).await;

    let (ctx, _dir) = build_ctx(server.uri()).await;
    let module = repo_trust::modules::adoption::AdoptionModule;
    let (result, evidence) = module.run(&ctx).await.expect("adoption run");

    assert_eq!(result.module, "adoption");
    assert!(
        evidence.len() >= 3,
        "expected ≥3 evidence items, got {}",
        evidence.len()
    );
    // Documentation maturity should be present and Positive (README ≥500
    // words + docs/ + examples/ → 1.0 → 100).
    let dm = evidence
        .iter()
        .find(|e| e.code == "documentation_maturity")
        .expect("documentation_maturity evidence");
    assert!(matches!(dm.verdict, Verdict::Positive));
    // No README evidence (we provided one).
    assert!(!evidence.iter().any(|e| e.code == "no_readme"));
}

/// S-101: no published package falls back gracefully — Medium confidence,
/// `no_packages` caveat in `missing_data`, no `weekly_downloads` sub-score.
#[tokio::test]
async fn s101_no_packages_falls_back_to_medium() {
    let server = MockServer::start().await;
    // README absent for this scenario — exercises the S-201 path too.
    shared_github_mocks(&server, false, false, false).await;

    let (ctx, _dir) = build_ctx(server.uri()).await;
    let module = repo_trust::modules::adoption::AdoptionModule;
    let (result, evidence) = module.run(&ctx).await.expect("adoption run");

    assert_eq!(result.confidence, Confidence::Medium);
    assert!(
        result.missing_data.iter().any(|m| m == "no_packages"),
        "missing_data must contain `no_packages`; got {:?}",
        result.missing_data
    );
    let np = evidence
        .iter()
        .find(|e| e.code == "no_packages")
        .expect("no_packages evidence");
    assert!(matches!(np.verdict, Verdict::Neutral));
    // weekly_downloads sub-score is dropped when no packages are present.
    assert!(!result.sub_scores.contains_key("weekly_downloads"));
    // No README → Concerning evidence per S-201.
    let nr = evidence
        .iter()
        .find(|e| e.code == "no_readme")
        .expect("no_readme evidence");
    assert!(matches!(nr.verdict, Verdict::Concerning));
}

// ─── Tiny base64 encoder for the README fixture body ──────────────────────

fn base64_encode(input: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    let mut chunks = input.chunks_exact(3);
    for chunk in &mut chunks {
        let n = (u32::from(chunk[0]) << 16) | (u32::from(chunk[1]) << 8) | u32::from(chunk[2]);
        out.push(char::from(ALPHABET[((n >> 18) & 0x3F) as usize]));
        out.push(char::from(ALPHABET[((n >> 12) & 0x3F) as usize]));
        out.push(char::from(ALPHABET[((n >> 6) & 0x3F) as usize]));
        out.push(char::from(ALPHABET[(n & 0x3F) as usize]));
    }
    let rem = chunks.remainder();
    match rem.len() {
        1 => {
            let n = u32::from(rem[0]) << 16;
            out.push(char::from(ALPHABET[((n >> 18) & 0x3F) as usize]));
            out.push(char::from(ALPHABET[((n >> 12) & 0x3F) as usize]));
            out.push('=');
            out.push('=');
        },
        2 => {
            let n = (u32::from(rem[0]) << 16) | (u32::from(rem[1]) << 8);
            out.push(char::from(ALPHABET[((n >> 18) & 0x3F) as usize]));
            out.push(char::from(ALPHABET[((n >> 12) & 0x3F) as usize]));
            out.push(char::from(ALPHABET[((n >> 6) & 0x3F) as usize]));
            out.push('=');
        },
        _ => {},
    }
    out
}
