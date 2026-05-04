# Day 2 — Maintainers + Security + ADR-0012 (2026-05-04)

## What landed

6 atomic commits, ~3 800 net new lines, 83 → 147 tests:

1. `docs/day-2-specs` — ADR-0012 (RepositoryContext layering exception: runtime handles like `Cache` and `GithubClient` allowed on the `models` struct for v1; refactor to `runtime::RunContext` deferred post-v1.0) + 4 specs (scorecard-client / osv-client / maintainer-health-module / security-readiness-module) + paired scenarios + `architecture.md` §1 patch.
2. `chore/octocrab-removal` — dropped 19 transitive crates; CLAUDE.md §4 patched to reflect direct-`reqwest` GitHub client.
3. `feat/api-scorecard` (parallel agent) — federated client over `api.scorecard.dev`. 5 wiremock tests (S-001/S-101/S-102/S-201/S-202).
4. `feat/api-osv` (parallel agent) — federated client over `api.osv.dev` `POST /v1/query`, withdrawn-advisory filter, deterministic sort. 5 wiremock tests.
5. `feat/module-maintainers` — Gini coefficient + bus-factor proxy + retention rate + bot filter + governance-doc presence. Solo-maintainer Concerning, never HighRisk. 11 scorer + 13 features + 6 proptest invariants + 1 wiremock integration.
6. `feat/module-security` (+ 3-module scan registry + exit-code wiring + ADR-0007 determinism integration test) — federation policy per `methodology.md` §Module 5; OSV deferred to Day 3 with `osv_deferred_to_phase_3` caveat.

## Decisions

- **ADR-0012**: pragmatic acceptance of layering exception. `RepositoryContext` is the single carrier through `TrustModule::run`; alternative wrapper-type or trait-object indirection adds ceremony for one binary consumer.
- **octocrab removal**: 19 transitive crates dropped; pre-strict-CI-gate hygiene. Re-add later if Phase 2 needs GraphQL.
- **3-module Day-2 default set**: `cli::scan::execute::select_modules()` defaults to `{activity, maintainers, security}`. Stars + Adoption land Day 3.
- **Aggregate determinism** (ADR-0007) — end-to-end integration test: run scan twice against same wiremock fixture, assert byte-identical JSON modulo `snapshot_at` + `runtime_seconds`.
- **Heavy mode parallelism**: 2 worktree subagents (scorecard + osv) ran concurrent (337s + 412s wall-clock) while I implemented Maintainers in main worktree. Stacked-rebase integration handled 4 expected conflicts cleanly.

## Blockers / Friction

- Stacked-rebase requires resolving CHANGELOG / `src/api/mod.rs` `pub use` / threshold-table conflicts on every chain (predictable, fast).
- MemPalace MCP **disconnected** during this session — diary entry deferred per Day 2 EOD Q3.

## Numbers

- 6 commits, +3 800 / -57 lines.
- 83 → 147 tests (+64).
- 6 PRs (#22-#27) opened end-of-day.
