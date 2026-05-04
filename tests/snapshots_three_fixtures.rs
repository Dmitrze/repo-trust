#![allow(
    clippy::unused_async,
    clippy::float_cmp,
    clippy::doc_lazy_continuation,
    clippy::unreadable_literal,
    clippy::too_many_lines,
    dead_code
)]

//! End-to-end JSON snapshot tests for the three canonical reference repos.
//!
//! Day 5 acceptance gate: spin up a `MockServer` per fixture, run the
//! compiled `repo-trust` binary in `--mode standard --modules
//! activity,maintainers,security,stars,adoption --format json` against it,
//! parse the produced report, redact the two non-deterministic fields
//! (`snapshot_at` + `runtime_seconds`) per ADR-0007, and assert the JSON
//! against an `insta` snapshot.
//!
//! The three fixtures cover three canonical shapes:
//!
//! - `octocat/Hello-World` — inactive baseline. Small / mostly-empty
//!   endpoints, low scores across every module, Scorecard not yet scored,
//!   no published packages.
//! - `prometheus/prometheus` — active popular Go project. High commits /
//!   contributors / stars, deps.dev returns one Go package with weekly
//!   downloads, Scorecard score 8.5, healthy doc presence.
//! - `rust-lang/cargo` — active popular Rust project. Same shape as
//!   prometheus but Rust ecosystem signals (one Cargo package, slightly
//!   different mix of contributors / releases) so the snapshots are
//!   distinct.
//!
//! All three fixtures are inline `const &str` bodies, mirroring the pattern
//! in `tests/all_five_modules_integration.rs`. Tests use `path_regex` to
//! catch-all unknown `/contents/*` paths so the doc-presence probes don't
//! need to be enumerated exhaustively per fixture.

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;
use wiremock::matchers::{method, path, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ─── Helpers ──────────────────────────────────────────────────────────────

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

/// Healthy 9-signal user profile body. Same shape across all three fixtures
/// so the per-stargazer mock is just a constant.
fn healthy_user(login: &str) -> String {
    format!(
        r#"{{"login":"{login}","created_at":"2018-03-15T00:00:00Z","followers":120,"following":35,"public_repos":40,"public_gists":3,"bio":"engineer","blog":"https://example.org","email":null,"type":"User"}}"#
    )
}

/// Low-activity user profile — used for the `octocat/Hello-World` fixture
/// to drive the "low-activity share is high" branch of the stars scorer.
fn low_activity_user(login: &str) -> String {
    format!(
        r#"{{"login":"{login}","created_at":"2024-09-01T00:00:00Z","followers":0,"following":0,"public_repos":1,"public_gists":0,"bio":null,"blog":null,"email":null,"type":"User"}}"#
    )
}

/// README payload with `Hello world` plus enough filler to land above the
/// adoption-module half-credit band. Base64-encoded inline so the fixture
/// stays self-contained.
const README_SMALL_B64: &str =
    "SGVsbG8gd29ybGQuIFRoaXMgaXMgYSB0aW55IFJFQURNRSB1c2VkIGFzIGFuIGluYWN0aXZlIGJhc2VsaW5lIGZpeHR1cmUgZm9yIHRoZSByZXBvLXRydXN0IHNuYXBzaG90IHRlc3RzLg==";

/// Pinned scan instant: `2026-05-03T12:00:00Z`. The fixture commit /
/// release / issue / PR dates in this file are all anchored to this
/// constant so evidence values like `days_since_last_commit` stay stable
/// across CI runs on different calendar days.
const PINNED_SNAPSHOT_AT: &str = "2026-05-03T12:00:00Z";

/// Invoke the compiled binary against `server_uri`, returning the parsed
/// JSON report as a `serde_json::Value`. Mirrors the invocation pattern in
/// `tests/aggregate_determinism.rs`, plus `--snapshot-at` to pin the scan's
/// "now" so evidence values derived from `now - commit_date` are stable.
fn run_scan(server_uri: &str, slug: &str) -> Value {
    let out = TempDir::new().unwrap();
    let cache_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("repo-trust").unwrap();
    cmd.args([
        "scan",
        slug,
        "--mode",
        "standard",
        "--modules",
        "activity,maintainers,security,stars,adoption",
        "--format",
        "json",
        "--quiet",
        "--output",
    ])
    .arg(out.path())
    .args(["--api-base-url"])
    .arg(server_uri)
    .args(["--snapshot-at", PINNED_SNAPSHOT_AT])
    .env("HOME", cache_dir.path())
    .env_remove("GITHUB_TOKEN")
    .env_remove("REPO_TRUST_WEIGHTS__ACTIVITY")
    .assert()
    .success();

    let safe = slug.replace('/', "_");
    let json_path = out.path().join(format!("{safe}.json"));
    let bytes = std::fs::read(&json_path).expect("report file present");
    serde_json::from_slice(&bytes).expect("valid JSON report")
}

/// Strip the two fields explicitly excluded from determinism per
/// `architecture.md` §9 (`snapshot_at`, `runtime_seconds`). Mirrors the
/// helper in `tests/aggregate_determinism.rs` so all snapshot consumers
/// follow the same redaction policy.
fn redact_for_snapshot(mut v: Value) -> Value {
    if let Some(obj) = v.as_object_mut() {
        obj.insert("snapshot_at".into(), Value::String("[REDACTED]".into()));
        obj.insert(
            "runtime_seconds".into(),
            Value::Number(serde_json::Number::from(0u64)),
        );
        if let Some(repo) = obj.get_mut("repository").and_then(|r| r.as_object_mut()) {
            repo.insert("snapshot_at".into(), Value::String("[REDACTED]".into()));
        }
    }
    v
}

// ─── octocat/Hello-World — inactive baseline ──────────────────────────────

const OCTOCAT_REPO: &str = r#"{
    "full_name":"octocat/Hello-World",
    "html_url":"https://github.com/octocat/Hello-World",
    "default_branch":"master",
    "language":null,
    "stargazers_count":80,
    "forks_count":0,
    "watchers_count":9,
    "open_issues_count":0,
    "archived":false,
    "has_issues":true,
    "created_at":"2011-01-26T19:01:12Z",
    "pushed_at":"2014-06-11T21:51:23Z"
}"#;

const OCTOCAT_STARGAZERS: &str = r#"[
    {"starred_at":"2024-09-01T00:00:00Z","user":{"login":"oct_u1","type":"User"}},
    {"starred_at":"2024-09-02T00:00:00Z","user":{"login":"oct_u2","type":"User"}}
]"#;

async fn mount_octocat(server: &MockServer) {
    Mock::given(method("GET"))
        .and(path("/repos/octocat/Hello-World"))
        .respond_with(ok(OCTOCAT_REPO))
        .mount(server)
        .await;

    // Activity / maintainers / security all share these endpoints.
    Mock::given(method("GET"))
        .and(path("/repos/octocat/Hello-World/commits"))
        .respond_with(ok("[]"))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/octocat/Hello-World/releases"))
        .respond_with(ok("[]"))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/octocat/Hello-World/issues"))
        .respond_with(ok("[]"))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/octocat/Hello-World/pulls"))
        .respond_with(ok("[]"))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/octocat/Hello-World/contributors"))
        .respond_with(ok(
            r#"[{"login":"octocat","contributions":1,"type":"User"}]"#,
        ))
        .mount(server)
        .await;

    // Stars: small sample, low-activity profiles.
    Mock::given(method("GET"))
        .and(path("/repos/octocat/Hello-World/stargazers"))
        .respond_with(ok(OCTOCAT_STARGAZERS))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/users/oct_u1"))
        .respond_with(ok(&low_activity_user("oct_u1")))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/users/oct_u2"))
        .respond_with(ok(&low_activity_user("oct_u2")))
        .mount(server)
        .await;

    // Adoption: tiny README, no docs / examples / packages.
    let readme_body = format!(
        r#"{{"name":"README.md","type":"file","encoding":"base64","content":"{README_SMALL_B64}"}}"#
    );
    Mock::given(method("GET"))
        .and(path("/repos/octocat/Hello-World/readme"))
        .respond_with(ok(&readme_body))
        .mount(server)
        .await;

    // Catch-all for /contents/* — every doc-presence probe falls through to
    // 404 unless we mount a specific 200 above it.
    Mock::given(method("GET"))
        .and(path_regex(r"^/repos/octocat/Hello-World/contents/.*$"))
        .respond_with(nf())
        .mount(server)
        .await;

    // Scorecard: not scored.
    Mock::given(method("GET"))
        .and(path("/projects/github.com/octocat/Hello-World"))
        .respond_with(nf())
        .mount(server)
        .await;

    // deps.dev: no packages mapped.
    Mock::given(method("GET"))
        .and(path(
            "/v3alpha/projects/github.com%2Foctocat%2FHello-World:packageversions",
        ))
        .respond_with(nf())
        .mount(server)
        .await;
}

// ─── prometheus/prometheus — active popular Go project ────────────────────

const PROMETHEUS_REPO: &str = r#"{
    "full_name":"prometheus/prometheus",
    "html_url":"https://github.com/prometheus/prometheus",
    "default_branch":"main",
    "language":"Go",
    "stargazers_count":56000,
    "forks_count":9000,
    "watchers_count":1200,
    "open_issues_count":700,
    "archived":false,
    "has_issues":true,
    "created_at":"2012-11-24T00:00:00Z",
    "pushed_at":"2026-04-30T00:00:00Z"
}"#;

const PROMETHEUS_RELEASES: &str = r#"[
    {"tag_name":"v2.51.0","name":"v2.51.0","draft":false,"prerelease":false,"created_at":"2026-04-15T00:00:00Z","published_at":"2026-04-15T00:00:00Z"},
    {"tag_name":"v2.50.0","name":"v2.50.0","draft":false,"prerelease":false,"created_at":"2026-03-01T00:00:00Z","published_at":"2026-03-01T00:00:00Z"},
    {"tag_name":"v2.49.0","name":"v2.49.0","draft":false,"prerelease":false,"created_at":"2026-01-20T00:00:00Z","published_at":"2026-01-20T00:00:00Z"}
]"#;

const PROMETHEUS_CONTRIBUTORS: &str = r#"[
    {"login":"alice","contributions":520,"type":"User"},
    {"login":"bob","contributions":410,"type":"User"},
    {"login":"carol","contributions":305,"type":"User"},
    {"login":"dave","contributions":250,"type":"User"},
    {"login":"erin","contributions":190,"type":"User"},
    {"login":"frank","contributions":140,"type":"User"},
    {"login":"grace","contributions":110,"type":"User"},
    {"login":"heidi","contributions":85,"type":"User"}
]"#;

const PROMETHEUS_SCORECARD: &str = r#"{
    "date":"2026-04-25T00:00:00Z",
    "repo":{"name":"github.com/prometheus/prometheus","commit":"abcdef0"},
    "score":8.5,
    "checks":[
        {"name":"Pinned-Dependencies","score":8,"reason":"ok","documentation":{}},
        {"name":"Branch-Protection","score":9,"reason":"ok","documentation":{}}
    ]
}"#;

const PROMETHEUS_DEPS_DEV_PACKAGES: &str = r#"{
    "packages":[{"system":"GO","name":"github.com/prometheus/prometheus"}]
}"#;

const PROMETHEUS_GO_PACKAGE: &str = r#"{
    "system":"GO",
    "name":"github.com/prometheus/prometheus",
    "weeklyDownloads":"100000",
    "latestVersion":"v2.51.0"
}"#;

const PROMETHEUS_STARGAZERS: &str = r#"[
    {"starred_at":"2024-06-01T00:00:00Z","user":{"login":"prom_u1","type":"User"}},
    {"starred_at":"2024-06-02T00:00:00Z","user":{"login":"prom_u2","type":"User"}},
    {"starred_at":"2024-06-03T00:00:00Z","user":{"login":"prom_u3","type":"User"}},
    {"starred_at":"2024-06-04T00:00:00Z","user":{"login":"prom_u4","type":"User"}},
    {"starred_at":"2024-06-05T00:00:00Z","user":{"login":"prom_u5","type":"User"}}
]"#;

/// Hand-crafted commit list — six commits across the 18-month window so
/// the activity scorer sees realistic values for the 30/90/365d slices.
const PROMETHEUS_COMMITS: &str = r#"[
    {"sha":"c1","commit":{"author":{"name":"alice","email":"alice@example.org","date":"2026-04-29T10:00:00Z"},"message":"feat: ingest improvements"},"author":{"login":"alice","type":"User"}},
    {"sha":"c2","commit":{"author":{"name":"bob","email":"bob@example.org","date":"2026-04-20T10:00:00Z"},"message":"fix: corner case"},"author":{"login":"bob","type":"User"}},
    {"sha":"c3","commit":{"author":{"name":"carol","email":"carol@example.org","date":"2026-03-05T10:00:00Z"},"message":"refactor: storage path"},"author":{"login":"carol","type":"User"}},
    {"sha":"c4","commit":{"author":{"name":"dave","email":"dave@example.org","date":"2026-01-15T10:00:00Z"},"message":"feat: tracing"},"author":{"login":"dave","type":"User"}},
    {"sha":"c5","commit":{"author":{"name":"erin","email":"erin@example.org","date":"2025-10-10T10:00:00Z"},"message":"chore: deps"},"author":{"login":"erin","type":"User"}},
    {"sha":"c6","commit":{"author":{"name":"frank","email":"frank@example.org","date":"2025-06-12T10:00:00Z"},"message":"docs: roadmap"},"author":{"login":"frank","type":"User"}}
]"#;

/// Two issues touched in the last 90 days — exercises the `issues_enabled`
/// branch of the activity scorer.
const PROMETHEUS_ISSUES: &str = r#"[
    {"number":101,"state":"closed","title":"slow query","user":{"login":"alice","type":"User"},"comments":4,"created_at":"2026-03-01T00:00:00Z","updated_at":"2026-04-10T00:00:00Z","closed_at":"2026-04-10T00:00:00Z"},
    {"number":102,"state":"open","title":"perf regression","user":{"login":"bob","type":"User"},"comments":2,"created_at":"2026-04-20T00:00:00Z","updated_at":"2026-04-29T00:00:00Z","closed_at":null}
]"#;

const PROMETHEUS_PULLS: &str = r#"[
    {"number":201,"state":"closed","title":"add label","user":{"login":"alice","type":"User"},"created_at":"2026-04-01T00:00:00Z","updated_at":"2026-04-15T00:00:00Z","closed_at":"2026-04-15T00:00:00Z","merged_at":"2026-04-15T00:00:00Z"},
    {"number":202,"state":"open","title":"refactor scrape","user":{"login":"carol","type":"User"},"created_at":"2026-04-20T00:00:00Z","updated_at":"2026-04-28T00:00:00Z","closed_at":null,"merged_at":null}
]"#;

/// Long README — base64 of ~600 words to clear the adoption-module
/// "well-documented" band.
fn long_readme_b64() -> String {
    let plain = "Prometheus is an open-source systems monitoring and alerting toolkit originally built at SoundCloud. ".repeat(40);
    base64_encode(plain.as_bytes())
}

async fn mount_prometheus(server: &MockServer) {
    Mock::given(method("GET"))
        .and(path("/repos/prometheus/prometheus"))
        .respond_with(ok(PROMETHEUS_REPO))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/prometheus/prometheus/commits"))
        .respond_with(ok(PROMETHEUS_COMMITS))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/prometheus/prometheus/releases"))
        .respond_with(ok(PROMETHEUS_RELEASES))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/prometheus/prometheus/issues"))
        .respond_with(ok(PROMETHEUS_ISSUES))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/prometheus/prometheus/pulls"))
        .respond_with(ok(PROMETHEUS_PULLS))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/prometheus/prometheus/contributors"))
        .respond_with(ok(PROMETHEUS_CONTRIBUTORS))
        .mount(server)
        .await;

    // Stars: 5 healthy profiles.
    Mock::given(method("GET"))
        .and(path("/repos/prometheus/prometheus/stargazers"))
        .respond_with(ok(PROMETHEUS_STARGAZERS))
        .mount(server)
        .await;
    for login in ["prom_u1", "prom_u2", "prom_u3", "prom_u4", "prom_u5"] {
        Mock::given(method("GET"))
            .and(path(format!("/users/{login}")))
            .respond_with(ok(&healthy_user(login)))
            .mount(server)
            .await;
    }

    // Adoption: long README + docs/ + examples/ present.
    let readme_body = format!(
        r#"{{"name":"README.md","type":"file","encoding":"base64","content":"{}"}}"#,
        long_readme_b64()
    );
    Mock::given(method("GET"))
        .and(path("/repos/prometheus/prometheus/readme"))
        .respond_with(ok(&readme_body))
        .mount(server)
        .await;

    // Catch-all 404 for unknown /contents/* probes; specific 200s below
    // override (wiremock: most-recently-registered wins).
    Mock::given(method("GET"))
        .and(path_regex(r"^/repos/prometheus/prometheus/contents/.*$"))
        .respond_with(nf())
        .mount(server)
        .await;
    for p in [
        "docs",
        "examples",
        "LICENSE",
        ".github/CODEOWNERS",
        ".github/workflows",
        "CONTRIBUTING.md",
        "CODE_OF_CONDUCT.md",
        "SECURITY.md",
    ] {
        Mock::given(method("GET"))
            .and(path(format!("/repos/prometheus/prometheus/contents/{p}")))
            .respond_with(ok(r#"{"name":"x","type":"file"}"#))
            .mount(server)
            .await;
    }

    // Scorecard: scored 8.5.
    Mock::given(method("GET"))
        .and(path("/projects/github.com/prometheus/prometheus"))
        .respond_with(ok(PROMETHEUS_SCORECARD))
        .mount(server)
        .await;

    // deps.dev: one Go package with weekly downloads.
    Mock::given(method("GET"))
        .and(path(
            "/v3alpha/projects/github.com%2Fprometheus%2Fprometheus:packageversions",
        ))
        .respond_with(ok(PROMETHEUS_DEPS_DEV_PACKAGES))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path(
            "/v3/systems/GO/packages/github.com%2Fprometheus%2Fprometheus",
        ))
        .respond_with(ok(PROMETHEUS_GO_PACKAGE))
        .mount(server)
        .await;
}

// ─── rust-lang/cargo — active popular Rust project ────────────────────────

const CARGO_REPO: &str = r#"{
    "full_name":"rust-lang/cargo",
    "html_url":"https://github.com/rust-lang/cargo",
    "default_branch":"master",
    "language":"Rust",
    "stargazers_count":13000,
    "forks_count":2400,
    "watchers_count":280,
    "open_issues_count":1500,
    "archived":false,
    "has_issues":true,
    "created_at":"2014-03-19T00:00:00Z",
    "pushed_at":"2026-05-01T00:00:00Z"
}"#;

const CARGO_RELEASES: &str = r#"[
    {"tag_name":"0.81.0","name":"0.81.0","draft":false,"prerelease":false,"created_at":"2026-04-20T00:00:00Z","published_at":"2026-04-20T00:00:00Z"},
    {"tag_name":"0.80.0","name":"0.80.0","draft":false,"prerelease":false,"created_at":"2026-02-25T00:00:00Z","published_at":"2026-02-25T00:00:00Z"}
]"#;

const CARGO_CONTRIBUTORS: &str = r#"[
    {"login":"ferris","contributions":880,"type":"User"},
    {"login":"crab","contributions":640,"type":"User"},
    {"login":"oxide","contributions":420,"type":"User"},
    {"login":"borrow","contributions":300,"type":"User"},
    {"login":"checker","contributions":210,"type":"User"},
    {"login":"trait","contributions":160,"type":"User"},
    {"login":"lifetime","contributions":120,"type":"User"}
]"#;

const CARGO_SCORECARD: &str = r#"{
    "date":"2026-04-26T00:00:00Z",
    "repo":{"name":"github.com/rust-lang/cargo","commit":"deadbeef"},
    "score":9.0,
    "checks":[
        {"name":"Pinned-Dependencies","score":9,"reason":"ok","documentation":{}},
        {"name":"Code-Review","score":10,"reason":"ok","documentation":{}}
    ]
}"#;

const CARGO_DEPS_DEV_PACKAGES: &str = r#"{
    "packages":[{"system":"CARGO","name":"cargo"}]
}"#;

const CARGO_PACKAGE: &str = r#"{
    "system":"CARGO",
    "name":"cargo",
    "weeklyDownloads":"75000",
    "latestVersion":"0.81.0"
}"#;

const CARGO_STARGAZERS: &str = r#"[
    {"starred_at":"2024-07-01T00:00:00Z","user":{"login":"cargo_u1","type":"User"}},
    {"starred_at":"2024-07-02T00:00:00Z","user":{"login":"cargo_u2","type":"User"}},
    {"starred_at":"2024-07-03T00:00:00Z","user":{"login":"cargo_u3","type":"User"}},
    {"starred_at":"2024-07-04T00:00:00Z","user":{"login":"cargo_u4","type":"User"}},
    {"starred_at":"2024-07-05T00:00:00Z","user":{"login":"cargo_u5","type":"User"}}
]"#;

const CARGO_COMMITS: &str = r#"[
    {"sha":"k1","commit":{"author":{"name":"ferris","email":"ferris@example.org","date":"2026-04-30T10:00:00Z"},"message":"feat: better resolver diagnostics"},"author":{"login":"ferris","type":"User"}},
    {"sha":"k2","commit":{"author":{"name":"crab","email":"crab@example.org","date":"2026-04-21T10:00:00Z"},"message":"fix: lockfile drift"},"author":{"login":"crab","type":"User"}},
    {"sha":"k3","commit":{"author":{"name":"oxide","email":"oxide@example.org","date":"2026-03-12T10:00:00Z"},"message":"refactor: build script env"},"author":{"login":"oxide","type":"User"}},
    {"sha":"k4","commit":{"author":{"name":"borrow","email":"borrow@example.org","date":"2026-01-30T10:00:00Z"},"message":"feat: workspace inheritance polish"},"author":{"login":"borrow","type":"User"}},
    {"sha":"k5","commit":{"author":{"name":"checker","email":"checker@example.org","date":"2025-11-20T10:00:00Z"},"message":"chore: bump deps"},"author":{"login":"checker","type":"User"}}
]"#;

const CARGO_ISSUES: &str = r#"[
    {"number":301,"state":"open","title":"resolver edge case","user":{"login":"ferris","type":"User"},"comments":7,"created_at":"2026-03-15T00:00:00Z","updated_at":"2026-04-25T00:00:00Z","closed_at":null}
]"#;

const CARGO_PULLS: &str = r#"[
    {"number":401,"state":"closed","title":"resolver fix","user":{"login":"crab","type":"User"},"created_at":"2026-04-05T00:00:00Z","updated_at":"2026-04-18T00:00:00Z","closed_at":"2026-04-18T00:00:00Z","merged_at":"2026-04-18T00:00:00Z"}
]"#;

async fn mount_cargo(server: &MockServer) {
    Mock::given(method("GET"))
        .and(path("/repos/rust-lang/cargo"))
        .respond_with(ok(CARGO_REPO))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/rust-lang/cargo/commits"))
        .respond_with(ok(CARGO_COMMITS))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/rust-lang/cargo/releases"))
        .respond_with(ok(CARGO_RELEASES))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/rust-lang/cargo/issues"))
        .respond_with(ok(CARGO_ISSUES))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/rust-lang/cargo/pulls"))
        .respond_with(ok(CARGO_PULLS))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/rust-lang/cargo/contributors"))
        .respond_with(ok(CARGO_CONTRIBUTORS))
        .mount(server)
        .await;

    Mock::given(method("GET"))
        .and(path("/repos/rust-lang/cargo/stargazers"))
        .respond_with(ok(CARGO_STARGAZERS))
        .mount(server)
        .await;
    for login in ["cargo_u1", "cargo_u2", "cargo_u3", "cargo_u4", "cargo_u5"] {
        Mock::given(method("GET"))
            .and(path(format!("/users/{login}")))
            .respond_with(ok(&healthy_user(login)))
            .mount(server)
            .await;
    }

    let readme_body = format!(
        r#"{{"name":"README.md","type":"file","encoding":"base64","content":"{}"}}"#,
        long_readme_b64()
    );
    Mock::given(method("GET"))
        .and(path("/repos/rust-lang/cargo/readme"))
        .respond_with(ok(&readme_body))
        .mount(server)
        .await;

    Mock::given(method("GET"))
        .and(path_regex(r"^/repos/rust-lang/cargo/contents/.*$"))
        .respond_with(nf())
        .mount(server)
        .await;
    for p in [
        "docs",
        "examples",
        "LICENSE",
        ".github/CODEOWNERS",
        ".github/workflows",
        "CONTRIBUTING.md",
        "CODE_OF_CONDUCT.md",
        "SECURITY.md",
    ] {
        Mock::given(method("GET"))
            .and(path(format!("/repos/rust-lang/cargo/contents/{p}")))
            .respond_with(ok(r#"{"name":"x","type":"file"}"#))
            .mount(server)
            .await;
    }

    Mock::given(method("GET"))
        .and(path("/projects/github.com/rust-lang/cargo"))
        .respond_with(ok(CARGO_SCORECARD))
        .mount(server)
        .await;

    Mock::given(method("GET"))
        .and(path(
            "/v3alpha/projects/github.com%2Frust-lang%2Fcargo:packageversions",
        ))
        .respond_with(ok(CARGO_DEPS_DEV_PACKAGES))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v3/systems/CARGO/packages/cargo"))
        .respond_with(ok(CARGO_PACKAGE))
        .mount(server)
        .await;
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn octocat_hello_world_snapshot() {
    let server = MockServer::start().await;
    mount_octocat(&server).await;
    let report = redact_for_snapshot(run_scan(&server.uri(), "octocat/Hello-World"));
    insta::assert_json_snapshot!("octocat_hello_world", report);
}

#[tokio::test]
async fn prometheus_prometheus_snapshot() {
    let server = MockServer::start().await;
    mount_prometheus(&server).await;
    let report = redact_for_snapshot(run_scan(&server.uri(), "prometheus/prometheus"));
    insta::assert_json_snapshot!("prometheus_prometheus", report);
}

#[tokio::test]
async fn rust_lang_cargo_snapshot() {
    let server = MockServer::start().await;
    mount_cargo(&server).await;
    let report = redact_for_snapshot(run_scan(&server.uri(), "rust-lang/cargo"));
    insta::assert_json_snapshot!("rust_lang_cargo", report);
}

// ─── Tiny base64 encoder ──────────────────────────────────────────────────
//
// Inlined to keep the test file self-contained — same approach as
// `tests/adoption_module_integration.rs`. RFC 4648 standard alphabet,
// padded.

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
