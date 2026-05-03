# Architecture — Repo Trust

> Companion document to `PRD.md`. Defines the module boundaries, data flow, contracts, and concrete technology choices that Claude Code should follow when implementing the project.

---

## 1. Architecture at a Glance

```
                               ┌───────────────────────────────────┐
                               │           CLI (clap + indicatif)   │
                               │  scan · batch · explain · serve    │
                               └────────────────┬───────────────────┘
                                                │
                                                ▼
                               ┌───────────────────────────────────┐
                               │      Orchestrator / Run Context    │
                               │  config · cache · clients · seed   │
                               └────────────────┬───────────────────┘
                                                │
        ┌───────────────────────────────────────┼─────────────────────────────┐
        ▼                                       ▼                             ▼
┌───────────────┐                    ┌──────────────────┐            ┌──────────────────┐
│   API Layer   │                    │ Collectors (per  │            │  Storage Layer    │
│ github_client │  ←  HTTP / cache → │     module)      │  ←────→    │  SQLite cache     │
│ deps_dev      │                    │  raw → normalized │            │  scorer outputs   │
│ scorecard     │                    └────────┬─────────┘            └──────────────────┘
│ osv           │                             │
└───────────────┘                             ▼
                                     ┌──────────────────┐
                                     │ Feature pipelines│
                                     │  per module      │
                                     └────────┬─────────┘
                                              │
                                              ▼
                                     ┌──────────────────┐
                                     │  Module scorers  │
                                     │  + explainers    │
                                     └────────┬─────────┘
                                              │
                                              ▼
                                     ┌──────────────────┐
                                     │   Aggregator     │
                                     │   + confidence    │
                                     └────────┬─────────┘
                                              │
                          ┌───────────────────┼───────────────────┐
                          ▼                   ▼                   ▼
                  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐
                  │  JSON report │    │  MD report   │    │  CSV report  │
                  └──────────────┘    └──────────────┘    └──────────────┘
                                              │
                                              ▼
                                     ┌──────────────────┐
                                     │  Optional axum    │
                                     │  localhost viewer │
                                     └──────────────────┘
```

### Layering rules (enforced via crate-level boundaries and `cargo deny` checks)
- `cli` may import from any layer.
- `modules` may import from `collectors`, `features`, `models`, `scoring`, `utils`.
- `collectors` may import from `api`, `storage`, `models`, `utils` — never from `modules` or `scoring`.
- `scoring` is pure (no I/O). It accepts feature structs and returns score structs.
- `reports` reads from `models` only — never re-runs collectors.
- `models` is the type vocabulary; it does **not** depend on `storage` or `api` for any leaf type. **Exception:** `RepositoryContext` carries runtime handles (`Cache`, `GithubClient`, `ScorecardClient`, `OsvClient`, `DepsDevClient`) for v1 — see [ADR-0012](adr/0012-repository-context-runtime-handles.md). Post-v1.0 forward path: split into `runtime::RunContext` if a second consumer of `models` ever appears.

---

## 2. Technology Stack

### Why Rust?

**Performance** — batch scanning 100+ repos is a real use case. Cold-cache p95 < 30s per repo with hundreds of HTTP requests means tight async scheduling and zero allocations on the hot path matter.

**Distribution** — the modern OSS CLI tool aesthetic in 2026 is single-binary distribution. Tools like `uv`, `ruff`, `biome`, `bun`, `ripgrep`, `fd`, `bat`, `gitui`, `hyperfine`, `tokei` set the standard. `cargo install repo-trust` and a downloadable binary from GitHub Releases are first-class. No Python runtime required on the user machine.

**Memory safety** — for a tool that ships in CI pipelines and parses untrusted JSON from third-party APIs, memory-safe by default is a meaningful security posture, especially for our application to the GitHub Secure Open Source Fund.

**Type system** — sum types (`enum`) + exhaustive matching make the Trust Module contract genuinely safe to extend. The five v1 modules and any future plugins get compile-time guarantees about completeness.

**Ecosystem** — `octocrab` (GitHub API), `reqwest` + `tokio` (async HTTP), `serde` (zero-cost JSON), `clap` v4 (CLI), `rusqlite` + `r2d2` (cache), `tracing` (structured logs), `insta` (snapshot tests). All actively maintained, all production-grade.

### Why not Go?
OpenSSF Scorecard and deps.dev are written in Go. We considered it. Go's `go install` distribution is excellent. We chose Rust because: (1) memory safety hard guarantee matters for a security-adjacent tool, (2) sum-type-driven module contracts are stronger than Go interfaces, (3) `cargo` ergonomics > Go modules for application crates, (4) we want the "modern dev-tool" perception that the 2026 Rust CLI generation has earned.

### Why not Python?
Python is slower per request, requires a runtime on user machines, has weaker static guarantees, and Python CLI tools like `pip`-installed binaries get a worse first-run experience than `cargo install` or a downloaded native binary. We are happy to consume Python OSS infrastructure (e.g. MemPalace as a developer tool) but the shipped binary is Rust.

### Core crates

| Concern | Crate | Why |
| --- | --- | --- |
| Edition | Rust 2021, MSRV `1.75` | Mature `async fn` in traits in stable since Dec 2023 |
| CLI parsing | `clap` v4 (`derive` feature) | De facto standard; superb `--help` formatting; subcommands |
| Terminal output | `indicatif` + `console` + `comfy-table` | Progress bars, colored output, tables |
| Async runtime | `tokio` (full features) | Required by `reqwest`, `axum`, most async libraries |
| HTTP client | `reqwest` (`json`, `gzip`, `rustls-tls` features) | Async, ergonomic, supports HTTP/2 and ETag |
| GitHub API | `octocrab` | Maintained Rust client for GitHub REST + GraphQL |
| Schemas / serialization | `serde`, `serde_json`, `serde_with` | Zero-cost serialization; JSON Schema generation via `schemars` |
| Error handling | `thiserror` (libraries) + `anyhow` (binaries) | Standard split; rich error context |
| Storage | `rusqlite` + `r2d2` + `r2d2_sqlite` | Embedded, zero-config; pooled connections for batch mode |
| Logging | `tracing` + `tracing-subscriber` | Structured logging with spans; opt-in JSON output |
| Time | `chrono` or `time` | Pick one; `time` is more modern |
| Config files | `figment` (TOML + env + CLI) | Layered config with provenance |
| Web (optional) | `axum` + `tower-http` + `askama` | Modern, composable; only loaded for `serve` |
| Graph analysis (deep mode) | `petgraph` | Star-cluster detection |
| RNG (deterministic) | `rand` + `rand_chacha` | Seeded ChaCha20 for reproducibility |
| Hashing | `blake3` | Fast content-addressable cache keys |

### Quality and dev tools

| Concern | Tool |
| --- | --- |
| Formatter | `rustfmt` (committed `rustfmt.toml`) |
| Linter | `clippy` (CI fails on warnings) |
| Tests | built-in `cargo test` |
| Property tests | `proptest` (for scoring functions) |
| HTTP mocking | `wiremock` (for collector integration tests) |
| Snapshot tests | `insta` (for golden-file determinism checks) |
| Coverage | `cargo-tarpaulin` (CI gate ≥ 85% on scoring + modules) |
| Audit | `cargo-deny` (license + advisory + supply-chain checks) |
| Pre-commit | `pre-commit` framework with rustfmt + clippy + commit-msg |
| Docs | `cargo doc` + `mdbook` (only when we ship a docs site) |
| Release | `cargo-release` + `git-cliff` for changelog generation |
| CI | GitHub Actions (matrix: ubuntu, macos, windows × stable, MSRV) |

---

## 3. Source Layout

```
repo-trust/
├── README.md
├── LICENSE                       # Apache-2.0
├── CONTRIBUTING.md
├── CODE_OF_CONDUCT.md
├── SECURITY.md
├── SUPPORT.md
├── CHANGELOG.md
├── Cargo.toml                    # workspace + binary crate manifest
├── Cargo.lock                    # committed (binary crate)
├── rust-toolchain.toml           # pin stable + components
├── rustfmt.toml
├── clippy.toml
├── deny.toml                     # cargo-deny policy
├── .editorconfig
├── .pre-commit-config.yaml
├── .github/
│   ├── FUNDING.yml               # GitHub Sponsors button
│   ├── dependabot.yml            # cargo + actions weekly
│   ├── PULL_REQUEST_TEMPLATE.md
│   ├── ISSUE_TEMPLATE/
│   │   ├── bug_report.yml
│   │   ├── feature_request.yml
│   │   └── methodology_question.yml
│   └── workflows/
│       ├── ci.yml                # fmt, clippy, test, tarpaulin
│       ├── release.yml           # tag → crates.io + GitHub Release + binaries
│       ├── scorecard.yml         # OSSF Scorecard on ourselves (dogfooding)
│       └── codeql.yml            # GH-native SAST
├── docs/
│   ├── PRD.md
│   ├── architecture.md           # ← this file
│   ├── methodology.md
│   ├── module-specs.md
│   ├── scoring-model.md
│   ├── benchmark-plan.md
│   ├── api-notes.md
│   ├── governance.md
│   └── adr/                      # Architecture Decision Records
│       ├── 0001-language-rust.md
│       ├── 0002-clap-cli.md
│       ├── 0003-sqlite-cache.md
│       ├── 0004-no-ml-in-v1.md
│       └── 0005-federate-not-replicate.md
├── src/
│   ├── main.rs                   # binary entry point
│   ├── lib.rs                    # library exports for plugins
│   ├── cli/
│   │   ├── mod.rs
│   │   ├── scan.rs
│   │   ├── batch.rs
│   │   ├── explain.rs
│   │   ├── serve.rs
│   │   ├── cache.rs
│   │   └── config.rs
│   ├── api/
│   │   ├── mod.rs
│   │   ├── client.rs             # shared reqwest client, retry, ETag
│   │   ├── github.rs             # GitHub REST + GraphQL via octocrab
│   │   ├── deps_dev.rs           # deps.dev API client
│   │   ├── scorecard.rs          # scorecard.dev API client
│   │   └── osv.rs                # OSV.dev API client
│   ├── collectors/
│   │   ├── mod.rs                # Collector trait
│   │   ├── repo.rs
│   │   ├── stars.rs
│   │   ├── activity.rs
│   │   ├── maintainers.rs
│   │   ├── adoption.rs
│   │   └── security.rs
│   ├── features/
│   │   ├── mod.rs
│   │   ├── stars.rs
│   │   ├── activity.rs
│   │   ├── maintainers.rs
│   │   ├── adoption.rs
│   │   └── security.rs
│   ├── modules/
│   │   ├── mod.rs                # TrustModule trait + registry
│   │   ├── stars.rs
│   │   ├── activity.rs
│   │   ├── maintainers.rs
│   │   ├── adoption.rs
│   │   └── security.rs
│   ├── scoring/
│   │   ├── mod.rs
│   │   ├── aggregate.rs
│   │   ├── thresholds.rs         # versioned thresholds; loaded from TOML
│   │   ├── confidence.rs
│   │   ├── explain.rs
│   │   └── weights.rs
│   ├── models/
│   │   ├── mod.rs
│   │   ├── repository.rs
│   │   ├── evidence.rs
│   │   ├── scores.rs
│   │   └── reports.rs
│   ├── reports/
│   │   ├── mod.rs                # Reporter trait
│   │   ├── terminal.rs           # comfy-table + console
│   │   ├── json_report.rs
│   │   ├── markdown_report.rs
│   │   ├── csv_report.rs
│   │   ├── sarif_report.rs       # v1.1
│   │   └── html_report.rs
│   ├── storage/
│   │   ├── mod.rs
│   │   ├── cache.rs              # SQLite-backed cache facade
│   │   ├── schema.sql
│   │   └── migrations/
│   ├── web/
│   │   ├── mod.rs                # axum app
│   │   ├── routes.rs
│   │   ├── templates/
│   │   └── static/
│   ├── config/
│   │   ├── mod.rs
│   │   ├── default.toml          # default thresholds and weights
│   │   ├── presets/
│   │   │   ├── strict.toml
│   │   │   ├── lenient.toml
│   │   │   └── security_first.toml
│   │   └── loader.rs
│   └── utils/
│       ├── mod.rs
│       ├── time.rs
│       ├── sampling.rs           # deterministic stargazer sampling
│       ├── normalization.rs
│       ├── ratelimit.rs
│       └── repo_url.rs
├── tests/
│   ├── integration_scan.rs       # end-to-end with wiremock
│   ├── golden_outputs.rs         # insta snapshot tests
│   └── fixtures/                 # cached API responses for tests
├── benches/                      # criterion benchmarks
│   └── scoring_bench.rs
├── examples/
│   ├── repos.txt
│   ├── sample-report.json
│   ├── sample-report.md
│   └── benchmark-set.csv
└── scripts/
    ├── run_benchmark.rs
    └── update_fixtures.rs
```

---

## 4. Module Contract

Every scoring module implements the same object-safe trait so that adding a new module is mechanical and the registry can hold them as `Vec<Box<dyn TrustModule>>` without erasure overhead.

```rust
use async_trait::async_trait;

use crate::models::{ModuleResult, EvidenceItem, RepositoryContext};

/// A scoring module. Five live in v1; plugins can register more (post-v1.2; see ADR-0010).
#[async_trait]
pub trait TrustModule: Send + Sync {
    /// Stable identifier, e.g. "stars", "activity".
    fn name(&self) -> &'static str;

    /// SemVer of this module's scoring logic. Bumped on threshold or weight changes.
    fn version(&self) -> &'static str;

    /// Run the full collect → features → score → explain pipeline, returning the
    /// module's result and its evidence items.
    async fn run(
        &self,
        ctx: &RepositoryContext,
    ) -> anyhow::Result<(ModuleResult, Vec<EvidenceItem>)>;
}
```

The four pipeline stages (`collect`, `compute_features`, `score`, `explain`) live as plain functions in `src/collectors/<module>.rs`, `src/features/<module>.rs`, and `src/modules/<module>.rs`. Each stage is independently testable; the trait `run()` body is conventionally a four-line wiring of the stages. This shape is documented in `docs/adr/0011-module-trait-shipped-shape.md` along with the rationale for choosing the simpler object-safe trait over a GAT-split design.

### Module registry

`modules::registry()` builds a registry from:
1. Built-in modules (the five v1 modules, statically linked).
2. Plugins registered via `inventory` crate or a feature-flag-gated entry-point mechanism.

Plugins are reserved for v1.2+. We do not advertise the plugin API for v1.0 to avoid committing to a stable plugin interface too early.

---

## 5. Data Model (serde + JSON Schema)

Key entity types:

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Mode { Quick, Standard, Deep }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Confidence { Low, Medium, High }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Verdict { Positive, Neutral, Concerning, HighRisk }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Category { Strong, Good, Mixed, Weak, HighRisk }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleWeights {
    pub stars: f64,
    pub activity: f64,
    pub maintainers: f64,
    pub adoption: f64,
    pub security: f64,
}

#[derive(Debug, Clone)]
pub struct RepositoryContext {
    pub full_name: String,            // "octocat/Hello-World"
    pub canonical_url: url::Url,
    pub mode: Mode,
    pub scoring_version: semver::Version,
    pub weights: ModuleWeights,
    pub cache: CacheHandle,           // facade
    pub api: ApiClients,
    pub rng_seed: u64,
    pub snapshot_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceItem {
    pub module: String,
    pub code: String,                 // "low_activity_stargazer_share"
    pub label: String,
    pub value: serde_json::Value,
    pub threshold: Option<serde_json::Value>,
    pub verdict: Verdict,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleResult {
    pub module: String,
    pub score: u8,                    // 0–100
    pub confidence: Confidence,
    pub sub_scores: std::collections::BTreeMap<String, u8>,
    pub sample_size: Option<usize>,
    pub missing_data: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustReport {
    pub schema_version: String,
    pub repository: RepositorySummary,
    pub overall_score: u8,
    pub overall_confidence: Confidence,
    pub category: Category,
    pub modules: Vec<ModuleResult>,
    pub evidence: Vec<EvidenceItem>,
    pub top_strengths: Vec<EvidenceItem>,
    pub top_concerns: Vec<EvidenceItem>,
    pub caveats: Vec<String>,
    pub scoring_version: String,
    pub weights_used: ModuleWeights,
    pub snapshot_at: DateTime<Utc>,
    pub runtime_seconds: f64,
}
```

The `TrustReport` JSON schema is **frozen** per major version. Breaking changes are documented in `docs/scoring-model.md` change log and `CHANGELOG.md`. We auto-generate JSON Schema from `schemars` and commit it to `docs/schemas/trust-report-v1.json` for downstream consumers.

---

## 6. Caching and Rate Limits

### 6.1 Cache layout

SQLite at `~/.repo-trust/cache.db`:

```sql
-- Raw API responses
CREATE TABLE api_cache (
    cache_key   TEXT PRIMARY KEY,    -- "github:repos:{owner}/{name}:metadata"
    etag        TEXT,
    fetched_at  TEXT NOT NULL,       -- ISO 8601 UTC
    expires_at  TEXT,
    body_json   TEXT NOT NULL,
    schema_ver  TEXT NOT NULL
);

CREATE INDEX idx_api_cache_repo ON api_cache(cache_key);

-- Feature snapshots per (repo, module, scoring_version)
CREATE TABLE features (
    repo        TEXT NOT NULL,
    module      TEXT NOT NULL,
    scoring_ver TEXT NOT NULL,
    computed_at TEXT NOT NULL,
    body_json   TEXT NOT NULL,
    PRIMARY KEY (repo, module, scoring_ver)
);

-- Final reports
CREATE TABLE reports (
    repo        TEXT NOT NULL,
    mode        TEXT NOT NULL,
    scoring_ver TEXT NOT NULL,
    computed_at TEXT NOT NULL,
    body_json   TEXT NOT NULL,
    PRIMARY KEY (repo, mode, scoring_ver, computed_at)
);
```

Schema migrations live in `src/storage/migrations/` and run on first connection via `rusqlite_migration`.

### 6.2 ETag / conditional fetching

Every collector that talks to GitHub MUST:
1. Look up the cached `etag` for the request key.
2. Send `If-None-Match: <etag>` on the request.
3. On `304 Not Modified`, reuse cached body, only update `fetched_at`.
4. On `200`, store new body and new etag.

This is the single most important rate-limit mitigation. GitHub does not consume rate limit on `304` responses for authenticated requests.

### 6.3 Cache TTL by data class

| Data class | Default TTL | Rationale |
| --- | --- | --- |
| Repo metadata | 24h | Changes rarely |
| Stargazer page | 7d | Historical data, immutable on the past |
| Recent commits | 1h | Active data |
| Recent PRs/issues | 1h | Active data |
| Releases | 6h | Periodic |
| Contributors summary | 24h | Slowly changing |
| OSSF Scorecard | 7d | Updates weekly |
| deps.dev | 24h | |
| OSV advisories | 6h | |

`--refresh` invalidates everything for the repo. `--refresh-module stars` invalidates only that module's cache keys.

### 6.4 Rate-limit handling

`utils::ratelimit::RateLimiter` exposes a coordinator that:
- Inspects `X-RateLimit-Remaining` and `X-RateLimit-Reset` on every response.
- When remaining < threshold (default 100), pauses and emits a clear `tracing::warn!` event.
- Never silently fails; if rate limit cannot be honored, the run errors out with a recoverable exit code so CI can decide what to do.

For deep-mode stargazer sampling, the limiter coordinates concurrent requests via a `tokio::sync::Semaphore`.

---

## 7. Sampling Strategy (Star Authenticity)

The most expensive operation. Algorithm:

1. **Determine target sample size** from mode:
   - Quick: 0 (skip; or use repo-level heuristics only).
   - Standard: `min(200, total_stars)`.
   - Deep: `min(2000, total_stars)`.
2. **Sampling method:** Uniform random over GitHub's stargazer pagination, with `rng_seed` from context for determinism. Use `rand_chacha::ChaCha20Rng::seed_from_u64(seed)`.
3. **For each sampled stargazer:** fetch profile metadata in concurrent batches (semaphore-bounded, default 10 concurrent).
4. **Compute the low-activity profile share** using the StarScout / Dagster heuristic:
   ```
   account is "low-activity" if all of:
     - created_at after 2022-01-01
     - followers ≤ 1
     - following ≤ 1
     - public_gists == 0
     - public_repos ≤ 4
     - bio is empty
     - blog is empty
     - email is empty
     - star_date == account_created_date (when star date is available via GraphQL)
   ```
5. **Compute lockstep timing signal** (Standard+) via z-score of starring rate over a sliding window.
6. **Optional graph signal (Deep):** for sampled stargazers, fetch their recent star history and look for cluster overlap with known campaign signatures using `petgraph`. This module's deep variant should be guarded behind explicit `--deep` and a clear runtime warning.

We document this algorithm in detail in `docs/methodology.md` so users can audit it. We also publish the exact thresholds in `src/config/default.toml`.

---

## 8. Error Handling and Exit Codes

Application errors use `anyhow::Result<T>` with rich context. Library errors use `thiserror`-derived enum types per module.

| Exit code | Meaning |
| --- | --- |
| 0 | Success |
| 1 | Generic CLI error (bad arguments, etc.) |
| 2 | Repository not found / not accessible |
| 3 | GitHub API authentication failure |
| 4 | Rate limit exceeded and could not recover within timeout |
| 5 | Cache file is corrupted or unreadable |
| 6 | Scoring version mismatch (asked for an unsupported version) |
| 7 | Network error, all retries exhausted |
| 64 | Configuration error |

All errors print to stderr in a machine-parseable form when `--json` is set:

```json
{"error": {"code": "rate_limit", "message": "...", "retry_after": 1800}}
```

---

## 9. Determinism and Reproducibility

The CLI is **deterministic** in the strict sense:

- Same inputs (`repo`, `mode`, `scoring_version`, `weights`, `rng_seed`) + same upstream API state ⇒ byte-identical JSON report (excluding `snapshot_at` and `runtime_seconds`).
- All sampling uses `rand_chacha::ChaCha20Rng::seed_from_u64(seed)`; default seed is derived from `(repo, scoring_version)` via blake3 hash.
- All sorts use `BTreeMap` / `Vec::sort_by_key` with explicit keys; never rely on `HashMap` insertion order for output.
- Floats in feature computation are rounded to 6 decimals before JSON serialization via a `serde_with` custom serializer.

This determinism is enforced by snapshot tests with `insta`: a fixture set of cached API responses is replayed by `wiremock` and the JSON output is compared against committed snapshots. CI fails on snapshot drift; reviewing and accepting a change uses `cargo insta review`.

---

## 10. Testing Strategy

### 10.1 Test pyramid
- **Unit (≥ 80% of tests):** pure functions in `features/`, `scoring/`, `utils/`.
- **Integration (≈ 15%):** collectors with `wiremock`-mocked APIs.
- **Snapshot / golden (≈ 5%):** end-to-end against fixture repos with cached API responses; output JSON is diffed via `insta`.

### 10.2 Property-based tests (proptest)
The scoring functions are excellent candidates:
- Score is monotonic where it should be.
- Score is bounded `[0, 100]` (compile-time guarantee via `u8`, but bounded ranges checked).
- Confidence never increases when data completeness decreases.
- Aggregation is commutative under module reordering.

### 10.3 Benchmark tests
A separate `benches/` crate using `criterion` runs the tool against the curated benchmark set (50 repos) and asserts category-level stability. Run with `cargo bench`.

### 10.4 Coverage targets (cargo-tarpaulin)
- Overall: ≥ 80%
- `src/scoring/`: ≥ 95%
- `src/modules/`: ≥ 90%
- `src/cli/`: ≥ 70%

### 10.5 Mutation testing (stretch)
Use `cargo-mutants` against `src/scoring/` to validate test sensitivity. Target: ≥ 80% of mutants caught.

---

## 11. Configuration

Three sources, in priority order (later overrides earlier), via `figment`:

1. Built-in defaults (shipped in `config/default.toml`, embedded via `include_str!`).
2. User config file at `~/.repo-trust/config.toml`.
3. Project-local config file at `./.repo-trust.toml`.
4. Environment variables (`REPO_TRUST_*`).
5. CLI flags.

Example user config:

```toml
[github]
token_env = "GITHUB_TOKEN"

[scan]
default_mode = "standard"
default_modules = ["stars", "activity", "maintainers", "adoption", "security"]

[weights]
stars = 0.20
activity = 0.25
maintainers = 0.20
adoption = 0.20
security = 0.15

[stars]
sample_size_standard = 200
sample_size_deep = 2000

[output]
default_formats = ["terminal"]
```

---

## 12. Localhost Web Viewer

`repo-trust serve` starts an `axum` app on `localhost:8765` that:
- Lists all reports in the local cache.
- Shows a single report with module cards, evidence list, activity timeline, contributor concentration chart.
- Allows triggering a re-scan for a repo (calls into the same scan engine).
- Exposes the same JSON via `/api/reports/{owner}/{name}`.

The web layer **never** opens a port other than localhost by default. A `--bind 0.0.0.0` flag exists but is documented as risky.

Templates use `askama` (compile-time templating) for safety; static assets are embedded via `rust-embed` so the binary stays single-file.

---

## 13. Security Posture

- **No telemetry** by default. The CLI ships without any analytics SDK.
- **Token handling:** tokens are read from env or config file; never logged; never echoed except in `--debug` mode where they are partially redacted.
- **Cache hygiene:** cache file permissions are `0600` on Unix (set explicitly via `std::os::unix::fs::PermissionsExt`).
- **Dependency hygiene:**
  - `dependabot.yml` updates cargo deps weekly.
  - `cargo-deny` runs in CI to enforce license allowlist + advisory denylist + duplicate detection.
  - We run our own tool against ourselves in CI (dogfooding via `repo-trust scan Dmitrze/repo-trust`).
- **Supply chain:**
  - Sigstore signing for releases via `cosign`.
  - SLSA Level 2+ provenance via GitHub Actions reusable workflow.
  - crates.io publish via Trusted Publisher / OIDC (no long-lived API token).
  - SBOM (CycloneDX) generated and attached to each GitHub Release.

---

## 14. Architecture Decision Records (planned)

The following ADRs should exist in `docs/adr/` from day one:

| # | Title | Decision |
| --- | --- | --- |
| 0001 | Language choice: Rust | Rust 2021 edition, MSRV 1.75, for performance, distribution, and memory safety |
| 0002 | CLI framework: clap | clap v4 with derive macros for subcommands and ergonomic --help |
| 0003 | Cache: SQLite via rusqlite | Local SQLite for zero-dependency portability; r2d2 pool for batch |
| 0004 | No ML in v1 | Heuristic scoring is more transparent and defensible |
| 0005 | Federate, don't replicate | We consume OSSF Scorecard, deps.dev, OSV — we do not duplicate them |
| 0006 | Five modules | Star Authenticity + Activity + Maintainers + Adoption + Security |
| 0007 | Deterministic outputs | Same inputs ⇒ byte-identical JSON; ChaCha20Rng for sampling |
| 0008 | Confidence is separate from score | A high score with low confidence must look different from a high score with high confidence |
| 0009 | Apache-2.0 license | Permissive but with patent grant; suitable for enterprise CI use |
| 0010 | Plugin system via inventory crate | Reserved for v1.2; not exposed in v1.0 |

Each ADR follows the standard template (Context · Decision · Consequences).

---

## 15. Open Architectural Questions (to be resolved by ADRs)

- **GraphQL vs REST for stargazer pagination.** GraphQL is more efficient for some star queries but has different rate limits. Likely ADR 0011.
- **Should the cache be content-addressable (blake3-keyed)?** Currently keyed by request URL; could move to hash-keyed for deduplication across repos.
- **Plugin authentication:** if third-party modules can call APIs, who pays the rate-limit budget? Likely the user; we document.
- **Should `serve` ship in v1.0 or v1.2?** Trade-off between scope and adoption. Tentative answer: v1.0 ships a minimal axum viewer.
- **WASM compilation target?** A WASM build of the core scoring would allow browser-based viewers. Not a v1 priority.

---

*This document is versioned with the project. Major architectural changes require both an ADR and a minor-or-major version bump.*
