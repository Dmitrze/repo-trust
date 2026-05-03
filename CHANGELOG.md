# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

The **scoring model** has its own SemVer separate from the CLI version. See [`docs/scoring-model.md`](docs/scoring-model.md) for the scoring change log.

## [Unreleased]

### Added
- Foundation documents: PRD, architecture, methodology plan
- Apache-2.0 license; methodology docs additionally under CC-BY-4.0
- Code of Conduct (Contributor Covenant 2.1)
- Contributing guide, security policy, support policy
- Repository structure for Rust CLI implementation
- Five-module trust framework definition: Star Authenticity, Activity Health, Maintainer Health, Adoption Signals, Security & Readiness
- Federation strategy with OpenSSF Scorecard, deps.dev, OSV (consume, do not replicate)
- ADR-0011: TrustModule trait shipped object-safe `run()` shape (locks the v1 trait surface).
- Spec-first scaffolding: `specs/{cache-layer,config-loader,github-api-client,activity-health-module}.md` with paired `tests/scenarios/`.
- Storage layer: r2d2-pooled SQLite cache (`src/storage/cache.rs`), schema migrations via `rusqlite_migration` (`src/storage/migrations/0001_initial.sql`), 0600 file perms on Unix, `Cache` handle threaded into `RepositoryContext`.
- Layered configuration via `figment`: embedded `src/config/default.toml` + user file (`~/.repo-trust/config.toml`) + project file (`./.repo-trust.toml`) + `REPO_TRUST_*` env + CLI overrides; `Config` typed structs with `WeightsConfig::into() → ModuleWeights`, tilde expansion in cache path, `GithubConfig::resolve_token()` reading the configured env var.
- GitHub REST client (`src/api/github.rs`): ETag-aware `fetch_json` lifecycle (cache hit → 304 revalidate → 200 store-new), per-endpoint TTLs from architecture §6.3, typed errors mapped to architecture §8 exit codes. Methods: `get_repo`, `list_commits` (windowed + paginated), `list_releases`, `list_issues_since` (filters out PRs), `list_pulls` (paginated with since-cutoff), `list_contributors`, `list_stargazers` (with `vnd.github.star+json` for star dates), `file_exists` (200/404 for doc-presence checks).
- `src/utils/ratelimit.rs`: `RateLimiter` with `tokio::sync::Semaphore` (default 10 permits) + `X-RateLimit-Remaining` / `X-RateLimit-Reset` tracking; warns at <100, pauses until reset at <10.
- Activity Health module (`src/modules/activity.rs`) end-to-end: collector pulls 18-month commit window plus releases, recent issues, recent PRs through the cached GitHub client; features layer normalizes to `ActivityFeatures` with `Option<u64>` for last-commit/last-release timestamps; scorer (`src/scoring/activity.rs`) emits ≥3 evidence items and arithmetic-mean sub-scores per `methodology.md` §Module 2. Threshold table in `src/scoring/thresholds.rs` (`ActivityThresholds::v1`) plus generic `linear_lower_better` / `linear_higher_better` helpers covered by 8 unit tests. 11 scorer tests + 3 features tests + 1 wiremock integration test against a minimal `octocat/Hello-World` fixture.
- `RepositoryContext` carries `github: GithubClient` alongside `cache: Cache`; new `owner_repo()` helper splits `full_name`.
- `cli::scan::execute` partial wire (Day 1): repo-URL parsing → config load → cache open → GitHub client construction → run modules → aggregate → write JSON report into `--output` dir + cache the report. Only Activity Health is wired end-to-end this day; remaining modules land Day 2/3. Hidden `--api-base-url` flag (also `REPO_TRUST_API_BASE_URL`) lets integration tests point at wiremock. End-to-end CLI test (`tests/scan_cli_integration.rs`) confirms the binary writes a parseable `TrustReport`.
- scorecard.dev REST client (`src/api/scorecard.rs`): ETag-aware `fetch_json` lifecycle reusing the existing `storage::Cache`, 7-day TTL per `architecture.md` §6.3, typed `ScorecardError::Other` for non-200/304/404 responses, and `Client::get` returning `Ok(None)` for the "not yet scored" 404 signal. DTOs (`ScorecardReport`, `ScorecardRepoRef`, `CheckResult`) cover the fields the Security & Readiness module needs. Re-exported as `repo_trust::api::ScorecardClient`. 5 wiremock integration tests (`tests/scorecard_client_integration.rs`) cover scenarios S-001/S-101/S-102/S-201/S-202 from `tests/scenarios/scorecard-client.md`.
- OSV.dev federated query client (`src/api/osv.rs`): `Client::query(PackageCoords)` over `POST /v1/query` with cache-key `osv:{ecosystem}:{name}:{version}` and 6h TTL per architecture §6.3; client-side filter for withdrawn advisories and stable sort by `id` for determinism. Typed `OsvError::Other { status, body }` for 5xx upstream failures. DTOs (`OsvAdvisory`, `Severity`, `Affected`, `PackageRef`) match the public OSV-schema fields the Security module needs. Five wiremock integration tests in `tests/osv_client_integration.rs` cover S-001 (empty response), S-002 (withdrawn-filter), S-101 (deterministic sort), S-102 (cache hit serves without network), S-201 (503 → Err). Wired into `api::OsvClient` re-export. Day 3 connects this client to the Security collector once Adoption supplies the repo→packages map.
- ADR-0012: documents the layering exception that puts runtime handles (`Cache`, `GithubClient`, future Scorecard / OSV / deps.dev clients) on `RepositoryContext`. `architecture.md` §1 patched accordingly. Forward path: split into `runtime::RunContext` if a second consumer of `models` ever appears.
- `chore`: removed unused `octocrab` dependency (-19 transitive crates) and patched `CLAUDE.md` §4 to reflect the direct-`reqwest` GitHub client + add the small leaf crates (`url`, `semver`, `dirs`).
- Maintainer Health module (`src/modules/maintainers.rs`) end-to-end: collector pulls 18-month commit window (cache-shared with Activity), contributors summary, and probes for `CODEOWNERS` (4 paths), `MAINTAINERS.md`, `GOVERNANCE.md` (3 paths) concurrently via `try_join_all`. Features layer: bot filter (`type=Bot` + `[bot]` suffix + `*-bot` + known names) → 365d commits-by-author map → Gini coefficient + bus-factor proxy + retention rate (cross-180d-window overlap) + top 5 human authors. Scorer (`src/scoring/maintainers.rs`): 4 sub-scores (`bus_factor_proxy`, `commit_concentration`, `contributor_retention`, `governance_docs`) per `methodology.md` §Module 3. Solo-maintainer is `Concerning` evidence, never `HighRisk` standalone (per `module-specs.md`). 11 scorer tests + 13 features tests + 6 proptest invariants on Gini/bus-factor + 1 wiremock integration test (bot-filter S-101 + solo-maintainer S-002).
- `MaintainerThresholds::v1()` added to `src/scoring/thresholds.rs` (bus-factor full credit at 5, Gini full-credit/zero at 0.40/0.85, retention full-credit/zero at 0.50/0.10).
- Security & Readiness module (`src/modules/security.rs`) end-to-end: collector federates Scorecard via `api::scorecard::Client`, runs doc-presence probes (SECURITY.md, CONTRIBUTING.md, CODE_OF_CONDUCT.md, LICENSE family, CODEOWNERS at 4 paths) and CI-workflow probes concurrently via `tokio::join!`/`try_join_all`. Features layer extracts `scorecard_score`, `scorecard_age_days`, `scorecard_checks_failed` (score < 5), and `semver_consistent` (every release tag must be `vX.Y.Z` or `X.Y.Z`). Scorer applies the federation policy from `methodology.md` §Module 5 (Scorecard ≤30d → weight 0.40 + High; 30-90d → 0.30 + Medium; >90d/absent → ignored + Low). 12 scorer tests + 4 features tests + 2 wiremock integration tests (S-001 fresh Scorecard + S-002 404 fallback). Day 2 keeps OSV deferred to Day 3 (zero advisories with `osv_deferred_to_phase_3` caveat).
- `RepositoryContext` carries `scorecard: ScorecardClient` + `osv: OsvClient` alongside the GitHub client (per ADR-0012).
- `cli::scan::execute` now wires the **3-module Day-2 default set** (Activity + Maintainers + Security); CLI tests pass `--modules` explicitly to scope wiremock fixture surface.
- Exit-code mapping wired in `cli::run`: `scan::execute` errors get downcast to `GithubError` and mapped per `architecture.md` §8 (404 → 2, 401 → 3, 403/rate-limit → 4); 2 integration tests (`tests/exit_codes.rs`) assert 401 → 3 and 403 → 4 against wiremock.
- `tests/aggregate_determinism.rs`: runs the full 3-module scan twice against the same wiremock fixture and asserts byte-identical JSON modulo `snapshot_at` + `runtime_seconds` (ADR-0007 enforcement at the integration level).
- deps.dev v3 federated client (`src/api/deps_dev.rs`): `Client::project_packages(owner, repo)` over `GET /v3/projects/github.com/{owner}/{repo}/packages` and `Client::package(system, name)` over `GET /v3/systems/{system}/packages/{name}`. ETag-aware `fetch_json` lifecycle on the existing `storage::Cache`, 24h TTL per architecture §6.3, cache keys `deps_dev:projects:{owner}/{repo}:packages` and `deps_dev:systems:{system}:{name}`. Typed `DepsDevError::{NotFound, Other}`; the project endpoint swallows 404 → `Ok(Vec::new())` (no packages mapped is a normal Adoption signal), the package endpoint propagates `NotFound`. DTOs: `PackageRef { system, name }` (sorts lexicographically for deterministic output) and `PackageInfo { system, name, weekly_downloads: Option<u64>, latest_version: Option<String> }` with a custom `deserialize_string_to_u64_option` deserializer for deps.dev's string-typed `weeklyDownloads`. Re-exported as `repo_trust::api::DepsDevClient`. Five wiremock integration tests in `tests/deps_dev_client_integration.rs` cover scenarios S-001 / S-002 / S-101 / S-102 / S-201 from `tests/scenarios/deps-dev-client.md`, plus six in-module unit tests for the custom deserializer and `PackageRef` ordering.
- Adoption Signals module (`src/modules/adoption.rs`) end-to-end: collector federates deps.dev for project→packages mapping and per-package weekly downloads (graceful 5xx handling — `deps_dev_error: true` rather than abort), GitHub `/readme` endpoint with base64-decoding and word-count, and `docs/` + `examples/` directory probes via `tokio::try_join!`. Features layer (`src/features/adoption.rs`) sums weekly downloads across packages (`Option<u64>` to drop the sub-score on missing data), de-duplicates package systems via `BTreeSet`, and computes a `documentation_maturity_score` in `[0.0, 1.0]` with per-component weights (README band 0.20–0.50, docs 0.30, examples 0.20). Scorer (`src/scoring/adoption.rs`) emits four sub-scores (`weekly_downloads` logarithmic banding 0/25/50/75/100, `documentation_maturity`, `package_systems_count`, `awesome_list_mentions`), arithmetic-mean aggregation, and federation caveats (`no_packages` → Medium, `deps_dev_unavailable` → Low, `archived` → Low — all Neutral verdicts per `methodology.md` §Module 4 conservative posture). Missing README is the *only* Concerning verdict the module emits. 16 scorer tests + 6 features tests + 2 wiremock integration tests (well-documented happy path + no-packages fallback covering S-101 + S-201).
- `AdoptionThresholds::v1()` added to `src/scoring/thresholds.rs` (download bands at 1k/10k/100k/1M; README word-count breakpoints at 100/500; High-confidence downloads floor at 10k).
- `src/api/github.rs::Client::get_readme` — fetches `/repos/{owner}/{repo}/readme` and base64-decodes the body via a small in-tree decoder (no new runtime crate dep). Returns `Ok(None)` on 404. 4 unit tests cover decode + whitespace + no-padding + invalid-char paths.
- `RepositoryContext` carries `deps_dev: DepsDevClient` alongside the other federated clients (per ADR-0012); `cli::scan::execute` constructs the client and wires `--api-base-url` overrides.

### Notes
- Pre-alpha. APIs and outputs will change before `v1.0.0`. Do not depend on this in production.

---

## [0.1.0] — TBD

The initial release will include:
- CLI skeleton (`scan`, `batch`, `explain`, `serve`, `cache`, `config`, `version`)
- Activity Health module
- Maintainer Health module
- Security & Readiness module (federating OpenSSF Scorecard)
- JSON and Markdown report writers
- SQLite-backed local cache with ETag-aware fetching
- Quick and Standard execution modes

The Star Authenticity, Adoption Signals, and Deep mode features are planned for `0.2.0` and beyond.

[Unreleased]: https://github.com/Dmitrze/repo-trust/compare/HEAD
[0.1.0]: https://github.com/Dmitrze/repo-trust/releases/tag/v0.1.0
