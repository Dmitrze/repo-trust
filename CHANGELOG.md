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
