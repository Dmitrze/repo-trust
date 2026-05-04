# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

The **scoring model** has its own SemVer separate from the CLI version. See [`docs/scoring-model.md`](docs/scoring-model.md) for the scoring change log.

## [0.1.0] — 2026-05-04

Initial public release. Five-module trust framework end-to-end against the live GitHub API.

### Added

#### Five trust modules (full pipelines)
- **Star Authenticity** — Heuristic 1 (9-signal low-activity profile share, 6-band table) + Heuristic 2 (lockstep timing z-score, 28-day rolling baseline lagged 7d) + Heuristic 3 (fork/watcher ratios with ecosystem multipliers). Final formula: `0.55 × H1 + 0.30 × H2 + 0.15 × H3` per `methodology.md` §Module 1 v1.0. Verdict ceiling stays `Concerning` even when combined H1+H2 evidence is emitted — never `HighRisk` standalone. Probabilistic phrasing only (`fake` / `fraud` / `bot` forbidden in evidence rationale, test-enforced). 5pp leniency on low-activity threshold for repos < 6 months old. Below-floor short-circuit for repos < 50 stars.
- **Activity Health** — collector pulls 18-month commit window plus releases / recent issues / recent PRs through the cached GitHub client; arithmetic-mean of 5 sub-scores per `methodology.md` §Module 2 v1.0.
- **Maintainer Health** — Gini coefficient on commits-by-author + bus-factor proxy + retention rate (cross-180d-window overlap) + governance-doc presence (CODEOWNERS at 4 paths, MAINTAINERS.md, GOVERNANCE.md). Bot filter (`type=Bot` + `[bot]` suffix + `*-bot` + known names). Solo-maintainer is `Concerning` evidence, never `HighRisk` standalone (per `module-specs.md`).
- **Adoption Signals** — federates deps.dev for repo→packages mapping + per-package weekly downloads. GitHub README maturity (presence + word count) + `docs/` + `examples/` directory probes. Logarithmic download bands (1k→25, 10k→50, 100k→75, 1M→100). Conservative posture: no published package = `Medium` confidence + `no_packages` Neutral caveat (NEVER `Concerning`).
- **Security & Readiness** — federates Scorecard (Scorecard ≤30d → weight 0.40 + High; 30-90d → 0.30 + Medium; >90d/absent → 0.0 + Low) + concurrent doc-presence probes (SECURITY.md, CONTRIBUTING.md, CODE_OF_CONDUCT.md, LICENSE family, CODEOWNERS) + CI workflow probes + semver-tag consistency. OSV wired but not invoked in v0.1.0 — per-package OSV lands when v0.2.0 makes the deps.dev mapping authoritative.

#### CLI surface
- `repo-trust scan` — full 5-module scan + aggregate + per-format writers. Mode-derived sample budgets (Quick / Standard / Deep). `--mode` / `--modules` / `--skip-modules` / `--output` / `--format` / `--weights` / `--scoring-version` / `--token` (also `GITHUB_TOKEN`) / `--seed` / `--refresh` / `--refresh-module` / `--debug` / `--quiet` / `--no-color` / `--json` / `--api-base-url` (hidden — wiremock). `--snapshot-at` (hidden — pinned timestamp for snapshot tests).
- `repo-trust serve` (web feature) — axum app on `127.0.0.1:8765` (default). Routes: `GET /` (cached-reports index, newest-first), `GET /reports/{owner}/{name}` (askama-rendered module cards + evidence), `GET /api/reports/{owner}/{name}` (raw cached JSON byte-for-byte), `POST /scans` (gated behind `--allow-scan` for DNS-rebinding mitigation; 405 otherwise), `GET /static/*` (CSS embedded via `rust-embed`). Single-binary preserved.
- `repo-trust cache info|clear|prune` — cache file path + size + per-table row counts + soft cap. `clear` defaults to api_cache; `--repo` scopes; `--all` also clears features + reports. `prune` removes expired rows.
- `repo-trust completions <shell>` — generated shell completions.
- `repo-trust version` — version + scoring-model version.

#### Output formats
- **Terminal** (default unless `--quiet`) — comfy-table + console color (Strong=green, Good=cyan, Mixed=yellow, Weak=orange, HighRisk=red); ANSI suppression when piped.
- **JSON** — frozen schema (`REPORT_SCHEMA_VERSION = 1.0.0`). Deterministic per ADR-0007.
- **Markdown** — long-form GFM with module sections + evidence tables + methodology footer. No new runtime dep (plain `writeln!`).
- **CSV** — fixed 21-column row per repo for batch + spreadsheet import. RFC-4180-ish quoting.
- **SARIF** — placeholder (Format::Sarif arm warns + skips); v0.2.0 work.

#### Architecture & rigor
- **Determinism** (ADR-0007) — same inputs + same upstream API state ⇒ byte-identical JSON modulo `snapshot_at` + `runtime_seconds`. Enforced by `tests/aggregate_determinism.rs` (full 3-module scan twice against same fixture) + 3 `tests/snapshots_three_fixtures.rs` snapshot tests (octocat/Hello-World, prometheus/prometheus, rust-lang/cargo).
- **Federation** (ADR-0005) — Scorecard, deps.dev, OSV consumed via thin clients with ETag-aware caching; no replication.
- **No `unsafe` code** (`#![deny(unsafe_code)]`).
- **Property-based tests** — 5 invariants on `scoring::aggregate` (bounded, deterministic, monotonic, confidence-demotion preserves bands) + Maintainer Gini bounds, 256 cases each (1 280+ total generated cases).
- **Snapshot tests** — 3 reference repos, deterministic via the new `--snapshot-at` flag.

#### Storage & infra
- r2d2-pooled SQLite cache (`api_cache` + `features` + `reports` tables) with ETag CRUD, 0600 perms on Unix.
- Layered `figment` config (defaults → `~/.repo-trust/config.toml` → `./.repo-trust.toml` → `REPO_TRUST_*` env → CLI flags).
- `RateLimiter` over `tokio::sync::Semaphore` with `X-RateLimit-Remaining` / `X-RateLimit-Reset` tracking; warns at <100, pauses until reset at <10.
- Architecture §8 exit-code mapping wired in `cli::run` (404→2, 401→3, 403→4).

#### Quality gates
- `cargo build --all-features --all-targets` clean (default + `--no-default-features` matrix entries).
- `cargo test --all-features` — 274 tests passing.
- `cargo fmt --all -- --check` clean.
- `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic` clean (cast_* family + ~15 other lints scoped-allowed in `src/lib.rs` with rationale).
- `cargo deny check` (license + bans + advisories + sources per `deny.toml`).
- `cargo audit` (RustSec advisory DB).
- `cargo tarpaulin` coverage ≥ 75%.
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` clean.
- OpenSSF Scorecard self-application (weekly + on-push).
- CodeQL static analysis (PR + weekly).
- Dependabot grouped updates for tokio / tracing / serde.

#### Documentation
- `README.md` with badges, install, quick-start, methodology link.
- `docs/PRD.md`, `docs/architecture.md`, `docs/methodology.md`, `docs/module-specs.md`, `docs/api-notes.md`, `docs/scoring-model.md`, `docs/governance.md`, `docs/benchmark-plan.md`.
- 12 ADRs (`docs/adr/0001` through `0012`) covering language choice, CLI framework, cache choice, no-ML-in-v1, federate-don't-replicate, five-modules, deterministic-output, confidence-separate-from-score, license, plugin-deferral, trait-shape, runtime-handles-on-context.
- `docs/benchmarks/v1.0.0.md` template + `scripts/run-benchmarks.sh` + `examples/benchmark-set.csv` (10 reference repos) — owner runs sweep post-launch with `$GITHUB_TOKEN`.

### Calibration

The following defaults are shipped as documented in `docs/methodology.md` v1.0; post-launch benchmark sweep (`scripts/run-benchmarks.sh`) informs whether v0.1.1 should recalibrate:

- **Security federation policy**: `0.40` / `0.30` interpreted as **absolute** weights against the fixed-pool of `docs (2.0) + ci (1.0) + semver (0.5) + osv (0.5)`. Scorecard fresh contributes ≈ 50% of the final module score.
- **Adoption download bands**: logarithmic (1k → 25, 10k → 50, 100k → 75, 1M → 100).
- **Stars lockstep z-score bands**: methodology v1 (`<3 → 100, 3-5 → 85, 5-8 → 60, 8-12 → 30, >12 → 10`). Combined H1+H2 condition (both ≥ 20% AND z ≥ 5) is the more reliable signal than H2 alone.

### Notes for v0.1.x follow-up

Tracked in GitHub issues post-launch:
- Maintainer / Security / Adoption / Stars / Web viewer wiremock test gap closures (currently covered by unit tests + 3 snapshot fixtures + at least 1 wiremock integration per module).
- Async `POST /scans` with job queue + polling (current synchronous behavior is fine for "developer-laptop" use case per `architecture.md` §12).
- True uniform random stargazer sampling (current Day-3/4 sample is recency-biased; deferred to Phase 2 deep mode).
- SARIF report writer.
- v1.1 LRU cache eviction; cache size cap enforcement.

---

[0.1.0]: https://github.com/Dmitrze/repo-trust/releases/tag/v0.1.0
