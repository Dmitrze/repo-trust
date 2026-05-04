# Day 1 — Foundation + Activity end-to-end (2026-05-03)

> **MemPalace fallback note** — MemPalace MCP was connected on Day 1 but the conversation didn't make use of it (the bootstrap focused on planning + audit + first implementations). This file backfills the diary entry per CLAUDE.md §17.

## What landed

6 atomic commits across 5 stacked feature branches, ~5 280 net new lines, 19 → 83 tests:

1. `docs/adr-0011-and-day1-specs` — ADR-0011 (TrustModule trait shipped object-safe `run()` shape; locks v1 contract; rejects GAT-split refactor on timeline grounds) + 4 specs (cache-layer / config-loader / github-api-client / activity-health-module) + paired scenarios + `architecture.md` §4 patch.
2. `feat/storage-cache` — r2d2-pooled SQLite cache (`api_cache` + `features` + `reports` tables per architecture §6.1), ETag CRUD, 0600 perms on Unix, 13 unit tests.
3. `feat/config-loader` — figment layering (defaults → user file → project file → env → CLI), `WeightsConfig` ↔ `ModuleWeights` conversion, tilde expansion, 11 hermetic tests via `figment::Jail`.
4. `feat/github-client` — direct `reqwest` GitHub client with ETag-aware `fetch_json` lifecycle (cache → 304 revalidate → 200 store-new), per-endpoint TTLs, typed `GithubError`. `RateLimiter` over `tokio::sync::Semaphore`. 8 wiremock integration tests.
5. `feat/module-activity` — first end-to-end module: collector → features → scorer pipeline. Threshold table in `src/scoring/thresholds.rs`. 11 scorer + 3 features + 1 wiremock integration tests.
6. `feat/scan-pipeline-day1` — `cli::scan::execute` partial wire (activity-only); hidden `--api-base-url` for wiremock-driven tests; 2 binary integration tests.

## Decisions

- **Trait shape locked** (ADR-0011): the shipped object-safe `TrustModule::run` signature is the v1 contract. GAT-split design from `architecture.md` was abandoned; refactor would cost 2 days on a 5-day budget for no observable benefit.
- **Cache layer foundation** for everything: ETag-aware caching keyed by request URL fragment, with explicit `is_stale()` marker for 304-conditional revalidation. Architecture §6 schema + TTL table shipped verbatim.
- **One file = one branch = one PR**: stacked PR strategy from Day 1 onward (each PR base = previous branch tip).

## Blockers / Friction

- `figment::Jail` test feature required adding `figment` to `[dev-dependencies]` (separate from runtime config dep) to enable the `test` feature.
- Cache layering exception (`Cache` field directly on `RepositoryContext` in `models`) noted but not yet documented — Day 2 ADR-0012 fixes this.

## What's deferred

- Maintainers / Security / Stars / Adoption modules — Day 2/3.
- Strict CI gates — Day 5 PM.
- 3 wiremock fixture set — Day 5 AM.

## Numbers

- 6 commits, +5 280 / -57 lines.
- 19 → 83 tests (+64).
- 6 PRs (#16-#21) opened end-of-day.
- All quality gates green locally; Day 1 CI ran on PR #16 only (others stacked off non-main bases).
